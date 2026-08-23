#!/bin/bash
# desc: Input peripherals — Xbox (xone), Razer, and tablet driver support
# long: Installs controller and peripheral drivers Bazzite ships by default:
# long: • xone — modern Xbox One/Series wireless dongle driver (DKMS)
# long: • OpenRazer — Razer peripheral support
# long: • input-remapper — per-device key/button remapping (preinstalled+enabled on Bazzite)
# long: • OpenTabletDriver — drawing tablet driver suite
# long: • libratbag — gaming-mouse DPI/button configuration
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
        echo "  skip $pkg (not in repos; install an AUR helper like paru)"
        return 1
    fi
}

module_apply() {
    local distro
    distro=$(get_distro)
    echo "  Installing input peripherals packages for $distro"

    warn_if_unknown_distro || true

    local packages=(
        "xone-dkms"
        "openrazer-driver-dkms" "openrazer-daemon"
        "libratbag"
        "input-remapper" "input-remapper-gtk"
        "opentabletdriver"
    )
    local pm
    pm=$(detect_package_manager)

    case "$pm" in
        pacman)
            for pkg in "${packages[@]}"; do
                install_pkg_with_aur_fallback "$pkg"
            done
            # input-remapper service (Bazzite enables it by default)
            sudo systemctl enable --now input-remapper 2>/dev/null || true
            # udev rules reload
            sudo systemctl restart systemd-udevd 2>/dev/null || true
            sudo udevadm control --reload-rules 2>/dev/null || true
            ;;
        apt|zypper|dnf)
            for pkg in "${packages[@]}"; do
                if pkg_available "$pkg"; then
                    pkg_install "$pkg"
                else
                    echo "  skip $pkg (not available in $pm repositories)"
                fi
            done
            # input-remapper service
            sudo systemctl enable --now input-remapper 2>/dev/null || true
            # udev rules reload
            sudo systemctl restart systemd-udevd 2>/dev/null || true
            sudo udevadm control --reload-rules 2>/dev/null || true
            ;;
        *)
            echo "unsupported package manager; pacman with AUR helpers supported" >&2
            return 1
            ;;
    esac
}

module_undo() {
    echo "note: kernel modules are left installed; removing them can break paired devices."
    local pm
    pm=$(detect_package_manager)
    local packages=(
        "xone-dkms"
        "openrazer-driver-dkms" "openrazer-daemon"
        "libratbag"
        "input-remapper" "input-remapper-gtk"
        "opentabletdriver"
    )
    case "$pm" in
        pacman)
            echo "to remove manually: sudo pacman -Rns ${packages[*]}"
            ;;
        apt)
            echo "to remove manually: sudo apt-get remove ${packages[*]}"
            ;;
        zypper)
            echo "to remove manually: sudo zypper remove ${packages[*]}"
            ;;
        dnf)
            echo "to remove manually: sudo dnf remove ${packages[*]}"
            ;;
    esac
}