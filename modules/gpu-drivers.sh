#!/bin/bash
# desc: GPU drivers — Mesa/Vulkan for AMD, nvidia-utils for NVIDIA (auto-detected)
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib/distro.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/packages.sh"

have_gpu() {
    case "$1" in
        amd) lspci -nn | grep -qi 'vga.*\[1002:\]\|display.*\[1002:\]' ;;
        nvidia) lspci -nn | grep -qi 'vga.*\[10de:\]\|display.*\[10de:\]' ;;
    esac
}

module_apply() {
    local distro
    distro=$(get_distro)
    echo "  Installing GPU drivers for $distro"

    warn_if_unknown_distro || true

    if have_gpu amd; then
        echo "AMD GPU detected: installing Mesa + RADV stack"
        local packages
        packages=$(resolve_package_list amd-driver opencl-amd vulkan-tools radeontop)
        pkg_install $packages
    elif have_gpu nvidia; then
        echo "NVIDIA GPU detected: installing nvidia-utils"
        local packages
        packages=$(resolve_package_list nvidia-driver opencl-nvidia vulkan-tools nvtop)
        pkg_install $packages
    else
        echo "No discrete AMD/NVIDIA GPU found via lspci; installing base Mesa only"
        local packages
        packages=$(resolve_package_list amd-driver)
        pkg_install $packages
    fi
}

module_undo() {
    echo "gpu-drivers: undo skipped intentionally — removing GPU drivers would break graphics."
    echo "(Uninstall manually if you really mean it.)"
}