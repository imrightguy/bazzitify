#!/bin/bash
# desc: Display & GPU control — LACT, CoreCtrl, gamescope session tools
# long: Bazzite-parity display/GPU management:
# long: • LACT — GPU control daemon + GUI (clocks, power limit, fan curves)
# long: • CoreCtrl — AMD/Intel CPU+GPU profile control
# long: • gamescope — Valve's micro-compositor (Steam Gaming Mode engine)
# long: LACT's systemd service is enabled so settings persist across reboots.
# depends: gpu-drivers
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib/distro.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/packages.sh"

# Try to install package via official repos, then AUR helpers
install_pkg_with_aur_fallback() {
    local pkg="$1"
    if pkg_available "$pkg"; then
        pkg_install "$pkg"
    elif command -v paru >/dev/null 2>&1; then
        paru -S --needed --noconfirm "$pkg"
    elif command -v yay >/dev/null 2>&1; then
        yay -S --needed --noconfirm "$pkg"
    else
        echo "  skip $pkg (not in repos; install an AUR helper)"
        return 1
    fi
}

module_apply() {
    local distro
    distro=$(get_distro)
    echo "  Installing display & GPU control packages for $distro"

    warn_if_unknown_distro || true

    local packages=("lact" "corectrl" "gamescope")
    local pm
    pm=$(detect_package_manager)

    case "$pm" in
        pacman)
            for pkg in "${packages[@]}"; do
                install_pkg_with_aur_fallback "$pkg"
            done
            # Persist LACT settings across reboots (Bazzite does this out of the box)
            sudo systemctl enable --now lactd 2>/dev/null \
                || echo "lactd service not present; run 'sudo lact' once to generate config"
            # Allow CoreCtrl to run without password prompt hint
            echo "note: for CoreCtrl sudo-free control see its wiki (polkit rules)"
            echo "gamescope usage: gamescope -e -- %command% or via Steam 'gamescope %command%'"
            ;;
        apt|zypper|dnf)
            for pkg in "${packages[@]}"; do
                if pkg_available "$pkg"; then
                    pkg_install "$pkg"
                else
                    echo "  skip $pkg (not available in $pm repositories)"
                fi
            done
            ;;
        *)
            echo "unsupported package manager; pacman with AUR helpers supported" >&2
            return 1
            ;;
    esac
}

module_undo() {
    sudo systemctl disable --now lactd 2>/dev/null || true
    echo "packages left installed; to remove:"
    local packages
    packages=$(resolve_package_list lact corectrl gamescope)
    local pm
    pm=$(detect_package_manager)
    case "$pm" in
        pacman)
            echo "  sudo pacman -Rns $packages"
            ;;
        apt)
            echo "  sudo apt-get remove $packages"
            ;;
        zypper)
            echo "  sudo zypper remove $packages"
            ;;
        dnf)
            echo "  sudo dnf remove $packages"
            ;;
    esac
}