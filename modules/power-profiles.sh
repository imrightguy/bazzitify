#!/bin/bash
# desc: CPU power profiles / governor tuning for gaming vs battery (laptop-aware, power-profiles-daemon + tuned)
# long: Configures CPU frequency scaling for gaming workloads, matching Bazzite's out-of-the-box behavior:
# long: • Detects CPU vendor (AMD/Intel) and applies vendor-specific kernel parameters
# long: • AMD: amd_pstate=active (or passive with shared_mem=1), energy_performance_preference=performance
# long: • Intel: intel_pstate=active with no_hwp if needed, energy_performance_preference=performance
# long: • Sets CPU governor to 'performance' via cpupower and kernel cmdline (cpufreq.default_governor=performance)
# long: • Installs/enables power-profiles-daemon with 'performance' profile as user default
# long: • Creates /etc/tuned/bazzitify-gaming for tuned users as alternative
# long: • On laptops: also creates bazzitify-battery tuned profile (powersave governor, balanced GPU/disk)
# long: • Laptop detection via systemd-detect-virt, upower, or /sys/class/power_supply/BAT* (best-effort)
# long: • Integrates with powerprofilesctl for runtime profile switching (performance/balanced/power-saver)
# long: Requires tuned package on some distros for custom profiles; laptop detection is best-effort
# depends: services
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib/distro.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/packages.sh"

MARKER="bazzitify:power-profiles"
GOVERNOR_MARKER_FILE="/etc/default/bazzitify-governor-backup"

detect_laptop() {
    # Returns 0 (true) if running on a laptop, 1 (false) otherwise
    # Best-effort detection via multiple methods
    
    # Method 1: systemd-detect-virt (checks for container/VM, but also chassis type)
    if command -v systemd-detect-virt >/dev/null 2>&1; then
        local chassis
        chassis=$(systemd-detect-virt --chassis 2>/dev/null || true)
        case "$chassis" in
            laptop|portable|notebook)
                return 0
                ;;
            desktop|server|vm|container)
                # Explicitly not a laptop, but continue checking other methods
                ;;
        esac
    fi
    
    # Method 2: upower (check for battery device)
    if command -v upower >/dev/null 2>&1; then
        if upower -e 2>/dev/null | grep -q '/battery_\|/BAT'; then
            return 0
        fi
    fi
    
    # Method 3: Check /sys/class/power_supply for battery
    if [ -d /sys/class/power_supply ]; then
        for bat in /sys/class/power_supply/BAT*; do
            [ -e "$bat" ] && return 0
        done
        # Some systems use different naming
        for bat in /sys/class/power_supply/*; do
            [ -e "$bat" ] && [ -f "$bat/type" ] && grep -qi "battery" "$bat/type" 2>/dev/null && return 0
        done
    fi
    
    # Method 4: Check for lid switch (laptop indicator)
    if [ -d /proc/acpi/button/lid ] && [ -n "$(ls -A /proc/acpi/button/lid 2>/dev/null)" ]; then
        return 0
    fi
    
    return 1
}

detect_cpu_vendor() {
    # Returns: amd, intel, or unknown
    if lscpu 2>/dev/null | grep -qi "vendor_id.*authenticamd"; then
        echo amd
    elif lscpu 2>/dev/null | grep -qi "vendor_id.*genuineintel"; then
        echo intel
    elif grep -qi "authenticamd" /proc/cpuinfo 2>/dev/null; then
        echo amd
    elif grep -qi "genuineintel" /proc/cpuinfo 2>/dev/null; then
        echo intel
    else
        echo unknown
    fi
}

detect_bootloader() {
    if [ -d /boot/loader/entries ] || [ -e /efi/loader/loader.conf ] || bootctl status >/dev/null 2>&1; then
        echo systemd-boot
    elif [ -f /etc/default/grub ]; then
        echo grub
    else
        echo unknown
    fi
}

get_current_governor() {
    # Returns the current CPU governor (first CPU core)
    if [ -f /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ]; then
        cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor 2>/dev/null || echo "unknown"
    else
        echo "unknown"
    fi
}

save_governor_backup() {
    local governor
    governor=$(get_current_governor)
    echo "$governor" | sudo tee "$GOVERNOR_MARKER_FILE" >/dev/null
    echo "saved current governor ($governor) to $GOVERNOR_MARKER_FILE"
}

restore_governor() {
    if [ -f "$GOVERNOR_MARKER_FILE" ]; then
        local governor
        governor=$(cat "$GOVERNOR_MARKER_FILE")
        if command -v cpupower >/dev/null 2>&1; then
            sudo cpupower frequency-set -g "$governor" >/dev/null 2>&1 && echo "restored governor to $governor via cpupower" || echo "cpupower restore failed"
        fi
        # Also restore via sysfs as fallback
        for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
            [ -f "$cpu" ] && echo "$governor" | sudo tee "$cpu" >/dev/null 2>&1 || true
        done
        rm -f "$GOVERNOR_MARKER_FILE"
    else
        echo "no governor backup found; skipping restore"
    fi
}

set_performance_governor() {
    if command -v cpupower >/dev/null 2>&1; then
        sudo cpupower frequency-set -g performance >/dev/null 2>&1 && echo "set governor to performance via cpupower" || echo "cpupower set failed"
    fi
    # Also set via sysfs as fallback (immediate, no reboot needed)
    for cpu in /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor; do
        [ -f "$cpu" ] && echo "performance" | sudo tee "$cpu" >/dev/null 2>&1 || true
    done
}

build_kernel_params() {
    local vendor="$1"
    local params=""

    case "$vendor" in
        amd)
            params="amd_pstate=active amd_pstate.shared_mem=1"
            ;;
        intel)
            params="intel_pstate=active intel_pstate=no_hwp"
            ;;
        *)
            params=""
            ;;
    esac

    # Common: performance governor at boot + energy performance preference
    if [ -n "$params" ]; then
        params="$params cpufreq.default_governor=performance"
    else
        params="cpufreq.default_governor=performance"
    fi

    echo "$params"
}

apply_kernel_params() {
    local vendor="$1"
    local params
    params=$(build_kernel_params "$vendor")

    if [ -z "$params" ]; then
        echo "unknown CPU vendor; skipping kernel params"
        return 1
    fi

    local bl
    bl=$(detect_bootloader)
    case "$bl" in
        systemd-boot)
            echo "systemd-boot detected: appending params to entries (idempotent)"
            for entry in /boot/loader/entries/*.conf /efi/loader/entries/*.conf; do
                [ -f "$entry" ] || continue
                if grep -q "$MARKER" "$entry"; then
                    echo "  $entry already tagged; skipping"
                    continue
                fi
                sudo sed -i "s|^\\(options .*\\)$|\\1 $params # $MARKER|" "$entry"
                echo "  updated $entry"
            done
            ;;
        grub)
            echo "GRUB detected: adding to GRUB_CMDLINE_LINUX_DEFAULT"
            if grep -q "$MARKER" /etc/default/grub; then
                echo "  already tagged; skipping"
            else
                sudo sed -i.bak-bazzitify-power "s|^\\(GRUB_CMDLINE_LINUX_DEFAULT=\\\"\\)\\(.*\\)\\\"|\\1\\2 $params\\\" # $MARKER|" /etc/default/grub
                if command -v grub-mkconfig >/dev/null; then
                    sudo grub-mkconfig -o /boot/grub/grub.cfg
                fi
                echo "GRUB regenerated (backup at /etc/default/grub.bak-bazzitify-power)"
            fi
            ;;
        *)
            echo "Unknown bootloader — cannot apply kernel params safely. Aborting."
            return 1
            ;;
    esac
}

remove_kernel_params() {
    local bl
    bl=$(detect_bootloader)
    case "$bl" in
        systemd-boot)
            for entry in /boot/loader/entries/*.conf /efi/loader/entries/*.conf; do
                [ -f "$entry" ] || continue
                if grep -q "$MARKER" "$entry"; then
                    sudo sed -i "s| $MARKER||; s|# $MARKER||; s| $params # $MARKER||" "$entry" 2>/dev/null || true
                    # More robust cleanup: remove the specific params we added
                    sudo sed -i "s/ amd_pstate=active//g; s/ amd_pstate.shared_mem=1//g; s/ intel_pstate=active//g; s/ intel_pstate=no_hwp//g; s/ cpufreq.default_governor=performance//g" "$entry" 2>/dev/null || true
                    sudo sed -i "s/  */ /g; s/ # $MARKER//g" "$entry" 2>/dev/null || true
                    echo "  cleaned $entry"
                fi
            done
            ;;
        grub)
            if [ -f /etc/default/grub.bak-bazzitify-power ]; then
                sudo mv /etc/default/grub.bak-bazzitify-power /etc/default/grub
                if command -v grub-mkconfig >/dev/null; then
                    sudo grub-mkconfig -o /boot/grub/grub.cfg
                fi
                echo "restored GRUB default from backup"
            else
                # Fallback: try to remove our params
                sudo sed -i "s/ amd_pstate=active//g; s/ amd_pstate.shared_mem=1//g; s/ intel_pstate=active//g; s/ intel_pstate=no_hwp//g; s/ cpufreq.default_governor=performance//g" /etc/default/grub 2>/dev/null || true
                sudo sed -i "s/  */ /g; s/ # $MARKER//g" /etc/default/grub 2>/dev/null || true
                if command -v grub-mkconfig >/dev/null; then
                    sudo grub-mkconfig -o /boot/grub/grub.cfg
                fi
                echo "cleaned GRUB params (no backup found)"
            fi
            ;;
        *)
            echo "unknown bootloader; nothing to undo for kernel params"
            ;;
    esac
}

configure_energy_performance_preference() {
    # Set energy_performance_preference to performance for all CPUs
    for cpu in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do
        [ -f "$cpu" ] && echo "performance" | sudo tee "$cpu" >/dev/null 2>&1 || true
    done
    # Also via cpupower if available
    if command -v cpupower >/dev/null 2>&1; then
        sudo cpupower set -b 0 2>/dev/null || true  # performance bias
    fi
}

restore_energy_performance_preference() {
    # Restore to balanced/default
    for cpu in /sys/devices/system/cpu/cpu*/cpufreq/energy_performance_preference; do
        [ -f "$cpu" ] && echo "balance_performance" | sudo tee "$cpu" >/dev/null 2>&1 || true
    done
    if command -v cpupower >/dev/null 2>&1; then
        sudo cpupower set -b 4 2>/dev/null || true  # balanced bias
    fi
}

setup_power_profiles_daemon() {
    local _pkgs
    if ! _pkgs=$(resolve_package_list power-profiles-daemon) || [ -z "$_pkgs" ]; then
        echo "packages not mapped for this distro; skipping install" >&2
        return 1
    fi
    pkg_install $_pkgs

    # Enable and start the system service
    sudo systemctl enable --now power-profiles-daemon.service 2>/dev/null || echo "power-profiles-daemon service enable failed (may already be running)"

    # Set performance profile as default for current user session
    if command -v powerprofilesctl >/dev/null 2>&1; then
        powerprofilesctl set performance 2>/dev/null && echo "set power-profiles-daemon profile to performance" || echo "powerprofilesctl set performance failed"
    fi
}

disable_power_profiles_daemon() {
    # Restore to balanced profile before stopping the service.
    if command -v powerprofilesctl >/dev/null 2>&1; then
        powerprofilesctl set balanced 2>/dev/null && echo "restored power-profiles-daemon profile to balanced" || echo "powerprofilesctl set balanced failed"
    fi

    # The module enabled this service during apply; undo must stop and disable it.
    # Packages remain installed so no user-installed software is removed.
    sudo systemctl disable --now power-profiles-daemon.service 2>/dev/null \
        && echo "stopped and disabled power-profiles-daemon" \
        || echo "power-profiles-daemon service disable failed"
}

setup_tuned_profile() {
    local tuned_dir="/etc/tuned/bazzitify-gaming"
    local tuned_file="$tuned_dir/tuned.conf"

    local _pkgs
    if ! _pkgs=$(resolve_package_list tuned) || [ -z "$_pkgs" ]; then
        echo "packages not mapped for this distro; skipping install" >&2
        return 1
    fi
    pkg_install $_pkgs

    # Create the tuned profile
    sudo mkdir -p "$tuned_dir"
    sudo tee "$tuned_file" >/dev/null <<'EOF'
# bazzitify gaming tuned profile
[main]
summary=Bazzitify gaming performance profile
include=throughput-performance

[cpu]
governor=performance
energy_performance_preference=performance
min_perf_pct=100

[sysctl]
vm.swappiness=10
vm.vfs_cache_pressure=50
net.core.default_qdisc=fq
net.ipv4.tcp_congestion_control=bbr
EOF

    echo "created tuned profile at $tuned_file"

    # Enable tuned service and apply profile
    sudo systemctl enable --now tuned.service 2>/dev/null || echo "tuned service enable failed"
    if command -v tuned-adm >/dev/null 2>&1; then
        sudo tuned-adm profile bazzitify-gaming 2>/dev/null && echo "applied tuned profile bazzitify-gaming" || echo "tuned-adm profile apply failed"
    fi
}

remove_tuned_profile() {
    local tuned_dir="/etc/tuned/bazzitify-gaming"

    # Revert to default tuned profile
    if command -v tuned-adm >/dev/null 2>&1; then
        sudo tuned-adm profile balanced 2>/dev/null && echo "reverted tuned profile to balanced" || echo "tuned-adm revert failed"
    fi

    # Remove our custom profile
    if [ -d "$tuned_dir" ]; then
        sudo rm -rf "$tuned_dir"
        echo "removed tuned profile directory $tuned_dir"
    fi

    # Note: we don't disable tuned service as other profiles may use it
}

setup_battery_tuned_profile() {
    local tuned_dir="/etc/tuned/bazzitify-battery"
    local tuned_file="$tuned_dir/tuned.conf"

    local _pkgs
    if ! _pkgs=$(resolve_package_list tuned) || [ -z "$_pkgs" ]; then
        echo "packages not mapped for this distro; skipping install" >&2
        return 1
    fi
    pkg_install $_pkgs

    # Create the tuned battery profile
    sudo mkdir -p "$tuned_dir"
    sudo tee "$tuned_file" >/dev/null <<'EOF'
# bazzitify battery tuned profile
[main]
summary=Bazzitify battery saving profile
include=powersave

[cpu]
governor=powersave
energy_performance_preference=balance_power
min_perf_pct=0

[sysctl]
vm.swappiness=60
vm.vfs_cache_pressure=100
EOF

    echo "created battery tuned profile at $tuned_file"
}

remove_battery_tuned_profile() {
    local tuned_dir="/etc/tuned/bazzitify-battery"

    # Revert to default tuned profile if we're currently using battery profile
    if command -v tuned-adm >/dev/null 2>&1; then
        local current_profile
        current_profile=$(tuned-adm active 2>/dev/null | awk -F': ' '/Current active profile/ {print $2}' || echo "")
        if [ "$current_profile" = "bazzitify-battery" ]; then
            sudo tuned-adm profile balanced 2>/dev/null && echo "reverted tuned profile from battery to balanced" || echo "tuned-adm revert from battery failed"
        fi
    fi

    # Remove our custom battery profile
    if [ -d "$tuned_dir" ]; then
        sudo rm -rf "$tuned_dir"
        echo "removed battery tuned profile directory $tuned_dir"
    fi
}

module_apply() {
    echo "=== bazzitify power-profiles: gaming CPU optimization ==="

    # Install required packages
    local _pkgs
    if ! _pkgs=$(resolve_package_list cpupower power-profiles-daemon tuned) || [ -z "$_pkgs" ]; then
        echo "packages not mapped for this distro; skipping install" >&2
        return 1
    fi
    pkg_install $_pkgs

    # Detect CPU vendor
    local vendor
    vendor=$(detect_cpu_vendor)
    echo "Detected CPU vendor: $vendor"

    if [ "$vendor" = "unknown" ]; then
        echo "Warning: Could not detect CPU vendor (AMD/Intel). Some optimizations may not apply."
    fi

    # Detect laptop
    local is_laptop=false
    if detect_laptop; then
        is_laptop=true
        echo "Laptop detected: will create battery profile"
    else
        echo "Desktop system detected: gaming profile only"
    fi

    # Save current governor for undo
    save_governor_backup

    # Set performance governor immediately
    set_performance_governor

    # Configure energy performance preference
    configure_energy_performance_preference

    # Apply kernel parameters (bootloader-aware)
    if [ "$vendor" != "unknown" ]; then
        apply_kernel_params "$vendor"
    fi

    # Setup power-profiles-daemon
    setup_power_profiles_daemon

    # Setup tuned profile as alternative
    setup_tuned_profile

    # Setup battery profile on laptops
    if [ "$is_laptop" = true ]; then
        setup_battery_tuned_profile
        echo "Battery profile (bazzitify-battery) created for laptop power saving"
    fi

    echo ""
    echo "=== power-profiles applied ==="
    echo "CPU vendor: $vendor"
    echo "System type: $([ "$is_laptop" = true ] && echo "laptop" || echo "desktop")"
    echo "Governor: performance (immediate + kernel cmdline)"
    echo "Energy performance preference: performance"
    echo "power-profiles-daemon: performance profile active"
    echo "tuned profile: bazzitify-gaming created and applied"
    if [ "$is_laptop" = true ]; then
        echo "tuned battery profile: bazzitify-battery created (switch via powerprofilesctl or tuned-adm)"
    fi
    echo ""
    echo "Note: Kernel parameters require reboot to take full effect."
    if [ "$is_laptop" = true ]; then
        echo "Laptop: use 'powerprofilesctl set power-saver' or 'tuned-adm profile bazzitify-battery' for battery saving."
    else
        echo "Laptop warning: performance governor reduces battery life."
    fi
}

module_undo() {
    echo "=== bazzitify power-profiles: undoing gaming CPU optimization ==="

    # Restore governor
    restore_governor

    # Restore energy performance preference
    restore_energy_performance_preference

    # Remove kernel parameters
    remove_kernel_params

    # Disable power-profiles-daemon (restore to balanced)
    disable_power_profiles_daemon

    # Remove tuned gaming profile
    remove_tuned_profile

    # Remove battery tuned profile if it exists
    remove_battery_tuned_profile

    echo ""
    echo "=== power-profiles undone ==="
    echo "Governor restored, kernel params cleaned, power-profiles-daemon set to balanced, tuned profiles removed."
    echo "Packages left installed; to remove:"
    local pm
    pm=$(detect_package_manager)
    case "$pm" in
        pacman) echo "  sudo pacman -Rns cpupower power-profiles-daemon tuned" ;;
        zypper) echo "  sudo zypper --non-interactive remove cpupower power-profiles-daemon tuned" ;;
        dnf)    echo "  sudo dnf remove cpupower power-profiles-daemon tuned" ;;
        apt)    echo "  sudo apt-get remove cpupower power-profiles-daemon tuned" ;;
        *)      echo "  (remove cpupower power-profiles-daemon tuned with your package manager)" ;;
    esac
    echo "Reboot recommended to fully revert kernel parameters."
}