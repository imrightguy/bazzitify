# desc: Install gaming packages (Steam, Lutris, MangoHud, gamescope, gamemode)

source "$(dirname "${BASH_SOURCE[0]}")/lib/distro.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/packages.sh"

module_apply() {
    local distro
    distro=$(get_distro)
    echo "  Installing gaming packages for $distro"

    # Warn if unknown distro
    warn_if_unknown_distro || true

    # Resolve packages for current distro
    local packages
    packages=$(resolve_package_list steam lutris mangohud gamescope gamemode lib32-mangohud lib32-gamemode)
    
    if [[ -z "$packages" ]]; then
        echo "  No packages to install for $distro" >&2
        return 1
    fi

    pkg_install $packages
}

module_undo() {
    echo "  (Package removal intentionally not automated — remove manually if desired)"
}