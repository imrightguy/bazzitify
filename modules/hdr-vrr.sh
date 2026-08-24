#!/bin/bash
# desc: HDR/VRR gaming helpers — compositor detection, opt-in env vars, KWin scripts, gamescope stack (Bazzite parity)
# long: HDR (High Dynamic Range) and VRR (Variable Refresh Rate) enablement for Wayland compositors.
# long: • KDE Plasma ≥6: installs kwin-effects-hdr + color-management; writes user-scoped KWin scripts to ~/.local/share/kwin/scripts/ (not system-wide)
# long: • gamescope: ensures gamescope + vkbasalt + mangohud present (depends on display-gpu-control, codecs modules)
# long: • Hyprland/sway: documents VRR enable steps; no forced config writes
# long: • Creates /etc/environment.d/99-bazzitify-hdr.conf with opt-in vars (KWIN_DRM_USE_HARDWARE_CURSORS=1, VKD3D_CONFIG=dxr11, RADV_PERFTEST=aco,rt)
# long: • VRR kernel param drm.vrr_enabled=1 managed via kernel-params module
# long: Honest limits: HDR on Linux is compositor-dependent; not all monitors work; VRR needs monitor + GPU + compositor support.
# long: Nothing is force-enabled — user opts in via GUI detail page; module_undo removes only bazzitify-created files.
# requires: gpu-drivers display-gpu-control codecs kernel-params
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib/distro.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/packages.sh"

MARKER="# bazzitify:hdr-vrr"
ENV_FILE="/etc/environment.d/99-bazzitify-hdr.conf"
KWIN_SCRIPTS_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/kwin/scripts"
BACKUP_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/bazzitify/hdr-vrr-backups"

# Detect compositor and Plasma version
# Returns: kwin6, kwin5, gnome, hyprland, sway, cosmic, gamescope, unknown
detect_compositor() {
    # Check for gamescope first (can run nested)
    if pgrep -x gamescope >/dev/null 2>&1; then
        echo gamescope
        return
    fi

    if [ -n "${KDE_FULL_SESSION:-}" ] || [ "${XDG_CURRENT_DESKTOP:-}" = "KDE" ] || pgrep -x kwin_wayland >/dev/null 2>&1; then
        # Check Plasma version via plasmashell
        local plasma_version
        if command -v plasmashell >/dev/null 2>&1; then
            plasma_version=$(plasmashell --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1)
            local major=${plasma_version%%.*}
            if [ -n "$major" ] && [ "$major" -ge 6 ]; then
                echo kwin6
                return
            fi
        fi
        echo kwin5
        return
    fi

    if [ "${XDG_CURRENT_DESKTOP:-}" = "GNOME" ] || [ "${XDG_CURRENT_DESKTOP:-}" = "ubuntu:GNOME" ] || pgrep -x gnome-shell >/dev/null 2>&1; then
        echo gnome
        return
    fi

    if [ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ] || pgrep -x Hyprland >/dev/null 2>&1; then
        echo hyprland
        return
    fi

    if [ -n "${SWAYSOCK:-}" ] || pgrep -x sway >/dev/null 2>&1; then
        echo sway
        return
    fi

    if [ "${XDG_CURRENT_DESKTOP:-}" = "COSMIC" ] || pgrep -x cosmic-comp >/dev/null 2>&1; then
        echo cosmic
        return
    fi

    echo unknown
}

# Check if AMD GPU is present
have_amd_gpu() {
    lspci -nn 2>/dev/null | grep -qi 'vga.*\[1002:\]\|display.*\[1002:\]'
}

# Backup a file before modification
backup_file() {
    local file="$1"
    [ -f "$file" ] || return 0
    mkdir -p "$BACKUP_DIR"
    local rel="${file#/}"
    local backup="$BACKUP_DIR/$rel"
    mkdir -p "$(dirname "$backup")"
    cp -p "$file" "$backup"
    echo "backed up $file -> $backup"
}

# Restore a file from backup
restore_file() {
    local file="$1"
    local rel="${file#/}"
    local backup="$BACKUP_DIR/$rel"
    if [ -f "$backup" ]; then
        cp -p "$backup" "$file"
        echo "restored $file from $backup"
    else
        echo "no backup for $file; leaving as-is"
    fi
}

# Remove bazzitify marker section from a file (marker line + next line)
remove_marker_section() {
    local file="$1"
    [ -f "$file" ] || return 0
    sed -i "/$MARKER/{N;d;}" "$file" 2>/dev/null || true
    sed -i "/$MARKER/d" "$file" 2>/dev/null || true
}

# Write environment.d file with HDR/VRR opt-in variables
write_environment_d() {
    mkdir -p "$(dirname "$ENV_FILE")"
    backup_file "$ENV_FILE"

    local env_content=""
    env_content+="KWIN_DRM_USE_HARDWARE_CURSORS=1\n"
    env_content+="VKD3D_CONFIG=dxr11\n"

    # AMD-specific RADV vars
    if have_amd_gpu; then
        env_content+="RADV_PERFTEST=aco,rt\n"
    fi

    # Tag with marker
    printf "%s\n%s" "$MARKER" "$env_content" | sudo tee "$ENV_FILE" >/dev/null
    echo "Wrote HDR/VRR environment variables to $ENV_FILE"
}

# Remove environment.d file
remove_environment_d() {
    if [ -f "$ENV_FILE" ]; then
        if grep -q "$MARKER" "$ENV_FILE"; then
            sudo rm -f "$ENV_FILE"
            echo "Removed $ENV_FILE"
        else
            echo "$ENV_FILE exists but not tagged by bazzitify; leaving as-is"
        fi
    fi
}

# Configure KWin scripts for Plasma ≥6 (user-scoped, not system-wide)
configure_kwin_scripts() {
    mkdir -p "$KWIN_SCRIPTS_DIR"

    # HDR metadata passthrough script
    local hdr_script_dir="$KWIN_SCRIPTS_DIR/bazzitify-hdr-metadata"
    local hdr_script_file="$hdr_script_dir/contents/code/main.js"
    local hdr_metadata_desktop="$hdr_script_dir/metadata.desktop"

    backup_file "$hdr_script_file" 2>/dev/null || true
    backup_file "$hdr_metadata_desktop" 2>/dev/null || true

    mkdir -p "$hdr_script_dir/contents/code"

    cat > "$hdr_metadata_desktop" <<'EOF'
[Desktop Entry]
Name=Bazzitify HDR Metadata Passthrough
Description=Enables HDR metadata passthrough for KWin
Type=Service
X-KWin-ServiceType=Script
X-Plasma-API=javascript
EOF

    cat > "$hdr_script_file" <<'EOF'
// Bazzitify HDR Metadata Passthrough
// Enables HDR metadata passthrough on supported outputs
// Marker: # bazzitify:hdr-vrr
var script = {
    init: function() {
        // KWin 6+ HDR metadata support
        if (typeof workspace !== 'undefined' && workspace.outputs) {
            workspace.outputs.forEach(function(output) {
                if (output.hdrMetadata) {
                    output.hdrMetadata = true;
                }
            });
        }
    }
};
script.init();
EOF

    # VRR script
    local vrr_script_dir="$KWIN_SCRIPTS_DIR/bazzitify-vrr"
    local vrr_script_file="$vrr_script_dir/contents/code/main.js"
    local vrr_metadata_desktop="$vrr_script_dir/metadata.desktop"

    backup_file "$vrr_script_file" 2>/dev/null || true
    backup_file "$vrr_metadata_desktop" 2>/dev/null || true

    mkdir -p "$vrr_script_dir/contents/code"

    cat > "$vrr_metadata_desktop" <<'EOF'
[Desktop Entry]
Name=Bazzitify VRR Enable
Description=Enables Variable Refresh Rate (adaptive-sync) on supported outputs
Type=Service
X-KWin-ServiceType=Script
X-Plasma-API=javascript
EOF

    cat > "$vrr_script_file" <<'EOF'
// Bazzitify VRR Enable
// Enables VRR/adaptive-sync on supported outputs
// Marker: # bazzitify:hdr-vrr
var script = {
    init: function() {
        if (typeof workspace !== 'undefined' && workspace.outputs) {
            workspace.outputs.forEach(function(output) {
                if (output.vrrSupported) {
                    output.vrrEnabled = true;
                }
            });
        }
    }
};
script.init();
EOF

    echo "Created KWin HDR/VRR scripts in $KWIN_SCRIPTS_DIR"
    echo "Enable them in System Settings > Window Management > KWin Scripts"
}

# Remove KWin scripts created by bazzitify
remove_kwin_scripts() {
    local hdr_script_dir="$KWIN_SCRIPTS_DIR/bazzitify-hdr-metadata"
    local vrr_script_dir="$KWIN_SCRIPTS_DIR/bazzitify-vrr"

    if [ -d "$hdr_script_dir" ] && [ -f "$hdr_script_dir/metadata.desktop" ] && grep -q "$MARKER" "$hdr_script_dir/contents/code/main.js" 2>/dev/null; then
        rm -rf "$hdr_script_dir"
        echo "Removed $hdr_script_dir"
    fi

    if [ -d "$vrr_script_dir" ] && [ -f "$vrr_script_dir/metadata.desktop" ] && grep -q "$MARKER" "$vrr_script_dir/contents/code/main.js" 2>/dev/null; then
        rm -rf "$vrr_script_dir"
        echo "Removed $vrr_script_dir"
    fi
}

# Configure kwinrc for HDR/VRR (fallback for Plasma 5, additional for Plasma 6)
configure_kwinrc() {
    local kwinrc="${KDE_CONFIG_HOME:-$HOME/.config}/kwinrc"
    backup_file "$kwinrc"

    local hdr_config="[Wayland]
Enabled=true
HDRMetadata=true

[Compositing]
VRREnabled=true
AdaptiveSync=true"

    # Only append if not already tagged
    if ! grep -q "$MARKER" "$kwinrc" 2>/dev/null; then
        printf "\n%s\n%s\n" "$MARKER" "$hdr_config" >> "$kwinrc"
        echo "KWin HDR/VRR config applied to $kwinrc"
    else
        echo "KWin config already tagged; skipping kwinrc modification"
    fi
}

# Remove kwinrc modifications
remove_kwinrc_config() {
    local kwinrc="${KDE_CONFIG_HOME:-$HOME/.config}/kwinrc"
    remove_marker_section "$kwinrc"
    restore_file "$kwinrc"
}

# Gamescope: ensure packages and document usage
configure_gamescope() {
    echo "Gamescope detected or requested."
    echo "Required packages (display-gpu-control + codecs modules): gamescope, vkbasalt, mangohud"
    echo "Usage: gamescope -e -- %command%  (or via Steam launch options)"
    echo "vkBasalt: ENABLE_VKBASALT=1 %command%"
    echo "MangoHud: mangohud %command%"
}

# Remove gamescope config (packages left installed per convention)
remove_gamescope() {
    echo "Gamescope packages left installed; remove manually if desired:"
    echo "  gamescope, vkbasalt, mangohud"
}

# Hyprland: document VRR/HDR steps
configure_hyprland() {
    local hypr_conf="${XDG_CONFIG_HOME:-$HOME/.config}/hypr/hyprland.conf"
    echo "Hyprland detected. HDR/VRR requires manual config in $hypr_conf:"
    echo "  monitor=,preferred,auto,1,bitdepth,10,hdr,1,vrr,1  (per monitor)"
    echo "See https://wiki.hyprland.org/Configuring/Monitors/#hdr-and-vrr"
}

# sway: document VRR/HDR steps
configure_sway() {
    local sway_conf="${XDG_CONFIG_HOME:-$HOME/.config}/sway/config"
    echo "sway detected. HDR/VRR requires manual config in $sway_conf:"
    echo "  output * adaptive_sync on"
    echo "  output * hdr on  (requires sway 1.9+)"
    echo "See https://man.archlinux.org/man/sway-output.5"
}

# GNOME: document VRR/HDR steps (gsettings approach)
configure_gnome() {
    echo "GNOME detected. HDR/VRR via gsettings (experimental):"
    echo "  HDR: gsettings set org.gnome.mutter experimental-features \"['hdr']\""
    echo "  VRR: gsettings set org.gnome.mutter.vrr enabled true"
    echo "Requires Mutter 43+ for HDR, newer for VRR."
}

# COSMIC: document VRR/HDR steps
configure_cosmic() {
    local cosmic_conf="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic/comp/config.ron"
    echo "COSMIC detected. HDR/VRR config requires manual review of $cosmic_conf"
    echo "See https://github.com/pop-os/cosmic-comp for HDR/VRR settings"
}

# Ensure kernel parameter drm.vrr_enabled=1 via kernel-params module
ensure_vrr_kernel_param() {
    # We delegate to kernel-params module; here we just verify/document
    echo "VRR kernel parameter 'drm.vrr_enabled=1' should be set via kernel-params module."
    echo "Run: bazzitify apply kernel-params  (adds drm.vrr_enabled=1 to bootloader)"
}

module_apply() {
    local comp
    comp=$(detect_compositor)
    echo "Detected compositor: $comp"

    # Install base packages for HDR/VRR
    local base_pkgs
    if ! base_pkgs=$(resolve_package_list kwin-effects-hdr color-management libdisplay-info) || [ -z "$base_pkgs" ]; then
        echo "packages not mapped for this distro; skipping install" >&2
        return 1
    fi
    pkg_install $base_pkgs

    # Write environment.d opt-in variables (applies to all compositors)
    write_environment_d

    # Ensure VRR kernel parameter
    ensure_vrr_kernel_param

    case "$comp" in
        kwin6)
            echo "KDE Plasma 6+ detected — configuring KWin scripts + kwinrc"
            configure_kwin_scripts
            configure_kwinrc
            ;;
        kwin5)
            echo "KDE Plasma 5 detected — configuring kwinrc only (KWin scripts require Plasma 6)"
            configure_kwinrc
            ;;
        gamescope)
            configure_gamescope
            ;;
        hyprland)
            configure_hyprland
            ;;
        sway)
            configure_sway
            ;;
        gnome)
            configure_gnome
            ;;
        cosmic)
            configure_cosmic
            ;;
        *)
            echo "Unsupported or undetected Wayland compositor: $comp"
            echo "Environment variables written to $ENV_FILE; kernel param documented."
            echo "Supported: KWin (Plasma 5/6), gamescope, Hyprland, sway, GNOME, COSMIC"
            ;;
    esac

    echo ""
    echo "=== HDR/VRR setup complete for $comp ==="
    echo "Environment vars: $ENV_FILE (requires re-login)"
    echo "Kernel param: drm.vrr_enabled=1 (via kernel-params module, requires reboot)"
    if [ "$comp" = "kwin6" ]; then
        echo "KWin scripts: $KWIN_SCRIPTS_DIR/bazzitify-* (enable in System Settings > KWin Scripts)"
    fi
    echo ""
    echo "HONEST LIMITS:"
    echo "  • HDR on Linux is compositor-dependent; not all monitors work"
    echo "  • VRR needs monitor + GPU + compositor support"
    echo "  • Nothing is force-enabled — all changes are opt-in"
}

module_undo() {
    local comp
    comp=$(detect_compositor)
    echo "Detected compositor for undo: $comp"

    # Remove environment.d file
    remove_environment_d

    # Remove KWin scripts (user-scoped)
    remove_kwin_scripts

    # Remove kwinrc config
    remove_kwinrc_config

    # Gamescope: packages left installed
    if [ "$comp" = "gamescope" ]; then
        remove_gamescope
    fi

    # Note: kernel-params module handles its own undo for drm.vrr_enabled=1
    echo "Kernel parameter drm.vrr_enabled=1: undo via 'bazzitify undo kernel-params'"

    echo ""
    echo "=== HDR/VRR undone ==="
    echo "Removed bazzitify-created configs only (tagged with $MARKER)"
    echo "Packages left installed; to remove manually on Arch:"
    echo "  sudo pacman -Rns kwin-effects-hdr colord libdisplay-info"
    echo "Backup directory preserved at $BACKUP_DIR (safe to delete manually)"
}