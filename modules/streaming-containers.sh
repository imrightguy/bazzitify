#!/bin/bash
# desc: Streaming & containers — Sunshine stream host, distrobox, waydroid
# long: Bazzite-parity streaming and container support:
# long: • Sunshine — self-hosted game stream server (pairs with Moonlight clients)
# long: • distrobox — immutable-friendly container tooling for any distro userland
# long: • waydroid — Android in a container (optional; big download)
# long: Sunshine gets firewall ports opened (TCP/UDP 47984-48010 range).
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
    echo "  Installing streaming & containers packages for $distro"

    warn_if_unknown_distro || true

    local packages=("sunshine" "distrobox")
    local opt_packages=("waydroid")
    local pm
    pm=$(detect_package_manager)

    case "$pm" in
        pacman)
            for pkg in "${packages[@]}"; do
                install_pkg_with_aur_fallback "$pkg"
            done
            for pkg in "${opt_packages[@]}"; do
                if pkg_available "$pkg"; then
                    pkg_install "$pkg"
                else
                    echo "waydroid optional: install later with: paru/yay -S waydroid (or via flatpak)"
                fi
            done
            # Sunshine service so it survives reboot like Bazzite's setup-sunshine
            systemctl --user enable --now sunshine 2>/dev/null \
                || echo "sunshine user service not found; start it manually first time: sunshine"
            # Firewall (only if ufw is active — never enable a firewall that isn't there)
            if command -v ufw >/dev/null 2>&1 && sudo ufw status | grep -q "Status: active"; then
                for port in 47984/tcp 47989/tcp 47990/tcp 48010/tcp 47998/udp 47999/udp 48000/udp; do
                    sudo ufw allow "$port"
                done
            fi
            ;;
        apt|zypper|dnf)
            for pkg in "${packages[@]}"; do
                if pkg_available "$pkg"; then
                    pkg_install "$pkg"
                else
                    echo "  skip $pkg (not available in $pm repositories)"
                fi
            done
            for pkg in "${opt_packages[@]}"; do
                if pkg_available "$pkg"; then
                    pkg_install "$pkg"
                else
                    echo "waydroid optional: not available in $pm; try flatpak"
                fi
            done
            # Sunshine service
            systemctl --user enable --now sunshine 2>/dev/null \
                || echo "sunshine user service not found; start it manually first time: sunshine"
            ;;
        *)
            echo "unsupported package manager; pacman with AUR helpers supported" >&2
            return 1
            ;;
    esac
}

module_undo() {
    systemctl --user disable --now sunshine 2>/dev/null || true
    echo "to remove:"
    local pm
    pm=$(detect_package_manager)
    local packages=("sunshine" "distrobox")
    case "$pm" in
        pacman)
            echo "  sudo pacman -Rns ${packages[*]}"
            ;;
        apt)
            echo "  sudo apt-get remove ${packages[*]}"
            ;;
        zypper)
            echo "  sudo zypper remove ${packages[*]}"
            ;;
        dnf)
            echo "  sudo dnf remove ${packages[*]}"
            ;;
    esac
}