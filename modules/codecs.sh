#!/bin/bash
# desc: Codecs & capture — full hw codec support, MangoHud overlay, OBS vkcapture, vkBasalt
# long: Matches Bazzite's out-of-the-box media/gaming stack:
# long: • Full hardware-accelerated H264 decode (libva + mesa drivers)
# long: • MangoHud — performance overlay (Vulkan/OpenGL)
# long: • OBS VkCapture — hardware-accelerated game capture for OBS
# long: • vkBasalt — Vulkan post-processing layer (CAS sharpening etc.)
# long: • GStreamer vaapi plugins for desktop video players
# requires: gpu-drivers
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib/distro.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/packages.sh"

module_apply() {
    local distro
    distro=$(get_distro)
    echo "  Installing codecs & capture packages for $distro"

    warn_if_unknown_distro || true

    # Resolve packages for current distro
    local packages
    packages=$(resolve_package_list \
        libva-mesa-driver intel-media-driver gstreamer-vaapi \
        mangohud lib32-mangohud \
        obs-vkcapture lib32-obs-vkcapture \
        vkbasalt lib32-vkbasalt \
        ffmpeg gstreamer codecs vaapi vdpau)
    
    if [[ -z "$packages" ]]; then
        echo "  No packages to install for $distro" >&2
        return 1
    fi

    pkg_install $packages

    echo "MangoHud usage: mangohud %command% in Steam launch options"
    echo "vkBasalt usage:  ENABLE_VKBASALT=1 %command%"
    echo "OBS capture:     run game through 'obs-vkcapture steam' or set in Lutris"
}

module_undo() {
    echo "codecs/overlay packages left installed — they're inert without per-game opt-in."
    echo "to remove manually:"
    local packages
    packages=$(resolve_package_list \
        libva-mesa-driver intel-media-driver gstreamer-vaapi \
        mangohud lib32-mangohud \
        obs-vkcapture lib32-obs-vkcapture \
        vkbasalt lib32-vkbasalt \
        ffmpeg gstreamer codecs vaapi vdpau)
    package_removal_command $packages
}