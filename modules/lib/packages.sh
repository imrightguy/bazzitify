#!/bin/bash
# desc: Per-distro package name maps for logical package names
# long: Maps logical package names (e.g., "steam", "mangohud") to
# long: distro-specific package names for Arch/pacman, Debian/apt,
# long: openSUSE/zypper, and Fedora/dnf.
# long: Source this file along with modules/lib/distro.sh

set -euo pipefail

# Resolve a logical package name to the distro-specific package name
# Usage: resolve_package <logical_name>
# Returns the package name for the CURRENT distro, or empty if not mapped
resolve_package() {
    local logical="${1:-}"
    [[ -z "$logical" ]] && return 1

    local distro
    distro=$(get_distro)
    resolve_package_for_distro "$distro" "$logical"
}

# Resolve a logical package name for a SPECIFIC distro
# Usage: resolve_package_for_distro <canonical_distro> <logical_name>
# Returns the package name, or empty if not mapped for that distro
resolve_package_for_distro() {
    local distro="${1:-}"
    local logical="${2:-}"
    [[ -z "$distro" || -z "$logical" ]] && return 1

    case "$distro" in
        arch|cachyos)
            _resolve_arch "$logical"
            ;;
        debian)
            _resolve_debian "$logical"
            ;;
        ubuntu)
            _resolve_ubuntu "$logical"
            ;;
        opensuse)
            _resolve_opensuse "$logical"
            ;;
        fedora)
            _resolve_fedora "$logical"
            ;;
        *)
            return 1
            ;;
    esac
}

# Resolve multiple logical packages to a space-separated list
# Usage: resolve_package_list <logical_name>...
# Returns space-separated package names for current distro
resolve_package_list() {
    local logical_pkgs=("$@")
    [[ ${#logical_pkgs[@]} -eq 0 ]] && return 0

    local resolved=()
    for pkg in "${logical_pkgs[@]}"; do
        local r
        r=$(resolve_package "$pkg")
        if [[ -n "$r" ]]; then
            resolved+=("$r")
        else
            echo "warning: no package mapping for '$pkg' on $(get_distro)" >&2
        fi
    done
    echo "${resolved[@]}"
}

# === Arch / CachyOS package maps ===
_resolve_arch() {
    local logical="$1"
    case "$logical" in
        # Gaming platforms
        steam) echo "steam" ;;
        lutris) echo "lutris" ;;
        heroic) echo "heroic-games-launcher" ;;
        bottles) echo "bottles" ;;
        
        # Gaming tools & overlays
        mangohud) echo "mangohud" ;;
        gamemode) echo "gamemode" ;;
        gamescope) echo "gamescope" ;;
        goverlay) echo "goverlay" ;;
        protontricks) echo "protontricks" ;;
        protonup-qt) echo "protonup-qt" ;;
        
        # Graphics drivers & tools
        nvidia-driver) echo "nvidia nvidia-utils lib32-nvidia-utils" ;;
        nvidia-dkms) echo "nvidia-dkms nvidia-utils lib32-nvidia-utils" ;;
        amd-driver) echo "mesa lib32-mesa vulkan-radeon lib32-vulkan-radeon" ;;
        intel-driver) echo "mesa lib32-mesa vulkan-intel lib32-vulkan-intel" ;;
        opencl-amd) echo "opencl-mesa clinfo" ;;
        opencl-nvidia) echo "opencl-nvidia lib32-opencl-nvidia" ;;
        rocm) echo "rocm-opencl-runtime rocm-hip-runtime" ;;
        
        # Vulkan & GPU tools
        vulkan-tools) echo "vulkan-tools vulkan-icd-loader lib32-vulkan-icd-loader" ;;
        vkd3d) echo "vkd3d lib32-vkd3d" ;;
        dxvk) echo "dxvk" ;;
        gpu-viewer) echo "gpu-viewer" ;;
        radeontop) echo "radeontop" ;;
        nvtop) echo "nvtop" ;;
        intel-gpu-tools) echo "intel-gpu-tools" ;;
        
        # Input & peripherals
        input-remapper) echo "input-remapper" ;;
        piper) echo "piper" ;;
        ratbagctl) echo "ratbagctl" ;;
        sole) echo "sole" ;;
        openrazer) echo "openrazer-daemon openrazer-driver-dkms" ;;
        logitech) echo "piper ratbagctl" ;;
        
        # Streaming & containers
        sunshine) echo "sunshine" ;;
        moonlight) echo "moonlight-qt" ;;
        obs-studio) echo "obs-studio" ;;
        obs-vkcapture) echo "obs-vkcapture" ;;
        v4l2loopback) echo "v4l2loopback-dkms" ;;
        
        # Audio
        pipewire) echo "pipewire wireplumber pipewire-pulse pipewire-alsa pipewire-jack lib32-pipewire lib32-wireplumber" ;;
        easyeffects) echo "easyeffects" ;;
        helvum) echo "helvum" ;;
        
        # Codecs & media
        ffmpeg) echo "ffmpeg" ;;
        gstreamer) echo "gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav" ;;
        codecs) echo "ffmpeg gst-plugins-base gst-plugins-good gst-plugins-bad gst-plugins-ugly gst-libav" ;;
        vaapi) echo "libva libva-mesa-driver libva-intel-driver libva-vdpau-driver-vp9" ;;
        vdpau) echo "vdpauinfo libvdpau-va-gl" ;;
        
        # Filesystems & storage
        libdisplay-info) echo "libdisplay-info" ;;
        edid-decode) echo "edid-decode" ;;
        zram-generator) echo "zram-generator" ;;
        btrfs) echo "btrfs-progs" ;;
        fstrim) echo "util-linux" ;;
        
        # Power & performance
        power-profiles-daemon) echo "power-profiles-daemon" ;;
        tuned) echo "tuned" ;;
        cpupower) echo "cpupower" ;;
        thermald) echo "thermald" ;;
        auto-cpufreq) echo "auto-cpufreq" ;;
        amd-pstate) echo "linux" ;;  # kernel parameter, not a package
        intel-pstate) echo "linux" ;;

        # HDR/VRR
        kwin-effects-hdr) echo "kwin" ;;
        color-management) echo "colord" ;;
        vkbasalt) echo "vkbasalt" ;;
        
        # System tuning
        sysctl) echo "procps-ng" ;;
        kernel-params) echo "grub" ;;  # or systemd-boot, handled specially
        
        # Services
        gamemoded) echo "gamemode" ;;
        lib32-gamemode) echo "lib32-gamemode" ;;
        
        # Flatpak
        flatpak) echo "flatpak" ;;
        flathub) echo "flatpak" ;;  # remote, not package
        
        # AUR helpers (not in official repos)
        paru) echo "paru" ;;
        yay) echo "yay" ;;
        
        # Misc
        htop) echo "htop" ;;
        btop) echo "btop" ;;
        neofetch) echo "neofetch" ;;
        fastfetch) echo "fastfetch" ;;
        *) return 1 ;;
    esac
}

# === Debian package maps ===
_resolve_debian() {
    local logical="$1"
    case "$logical" in
        steam) echo "steam" ;;
        lutris) echo "lutris" ;;
        heroic) echo "heroic-games-launcher" ;;
        bottles) echo "bottles" ;;
        
        mangohud) echo "mangohud" ;;
        gamemode) echo "gamemode libgamemode0" ;;
        gamescope) echo "gamescope" ;;
        goverlay) echo "goverlay" ;;
        protontricks) echo "protontricks" ;;
        protonup-qt) echo "protonup-qt" ;;
        
        nvidia-driver) echo "nvidia-driver nvidia-driver-libs-i386" ;;
        nvidia-dkms) echo "nvidia-dkms nvidia-driver-libs-i386" ;;
        amd-driver) echo "mesa-vulkan-drivers mesa-vulkan-drivers:i386 libgl1-mesa-dri libgl1-mesa-dri:i386" ;;
        intel-driver) echo "mesa-vulkan-drivers mesa-vulkan-drivers:i386 libgl1-mesa-dri libgl1-mesa-dri:i386 intel-media-va-driver:i386" ;;
        opencl-amd) echo "mesa-opencl-icd clinfo" ;;
        opencl-nvidia) echo "nvidia-opencl-icd nvidia-opencl-icd:i386" ;;
        rocm) echo "rocm-opencl-runtime rocm-hip-runtime" ;;
        
        vulkan-tools) echo "vulkan-tools vulkan-validationlayers-dev" ;;
        vkd3d) echo "libvkd3d1 libvkd3d1:i386" ;;
        dxvk) echo "dxvk" ;;
        gpu-viewer) echo "gpu-viewer" ;;
        radeontop) echo "radeontop" ;;
        nvtop) echo "nvtop" ;;
        intel-gpu-tools) echo "intel-gpu-tools" ;;
        
        input-remapper) echo "input-remapper" ;;
        piper) echo "piper" ;;
        ratbagctl) echo "ratbagctl" ;;
        sole) echo "sole" ;;
        openrazer) echo "openrazer-daemon openrazer-driver-dkms" ;;
        logitech) echo "piper ratbagctl" ;;
        
        sunshine) echo "sunshine" ;;
        moonlight) echo "moonlight-qt" ;;
        obs-studio) echo "obs-studio" ;;
        obs-vkcapture) echo "obs-vkcapture" ;;
        v4l2loopback) echo "v4l2loopback-dkms" ;;
        
        pipewire) echo "pipewire wireplumber pipewire-audio-client-libraries libpipewire-0.3-0 libspa-0.2-bluetooth libspa-0.2-jack" ;;
        easyeffects) echo "easyeffects" ;;
        helvum) echo "helvum" ;;
        
        ffmpeg) echo "ffmpeg" ;;
        gstreamer) echo "gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav" ;;
        codecs) echo "ffmpeg gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-plugins-bad gstreamer1.0-plugins-ugly gstreamer1.0-libav" ;;
        vaapi) echo "intel-media-va-driver va-driver-all vainfo" ;;
        vdpau) echo "vdpauinfo libvdpau-va-gl1" ;;
        
        zram-generator) echo "zram-generator" ;;
        btrfs) echo "btrfs-progs" ;;
        fstrim) echo "util-linux" ;;
        
        power-profiles-daemon) echo "power-profiles-daemon" ;;
        tuned) echo "tuned" ;;
        cpupower) echo "linux-cpupower" ;;
        thermald) echo "thermald" ;;
        auto-cpufreq) echo "auto-cpufreq" ;;

        # HDR/VRR
        kwin-effects-hdr) echo "kwin" ;;
        color-management) echo "colord" ;;
        vkbasalt) echo "vkbasalt" ;;

        sysctl) echo "procps" ;;
        kernel-params) echo "grub2" ;;
        
        gamemoded) echo "gamemode" ;;
        lib32-gamemode) echo "libgamemode0:i386" ;;
        
        flatpak) echo "flatpak" ;;
        flathub) echo "flatpak" ;;
        
        htop) echo "htop" ;;
        btop) echo "btop" ;;
        neofetch) echo "neofetch" ;;
        fastfetch) echo "fastfetch" ;;
        *) return 1 ;;
    esac
}

# === Ubuntu package maps (extends Debian, adds PPA-specific) ===
_resolve_ubuntu() {
    local logical="$1"
    # First try Debian mapping
    local debian_result
    debian_result=$(_resolve_debian "$logical")
    if [[ -n "$debian_result" ]]; then
        echo "$debian_result"
        return 0
    fi
    # Ubuntu-specific overrides/additions
    case "$logical" in
        nvidia-driver) echo "nvidia-driver-550 nvidia-driver-550-libs-i386" ;;
        nvidia-dkms) echo "nvidia-dkms-550 nvidia-driver-550-libs-i386" ;;
        *) return 1 ;;
    esac
}

# === openSUSE package maps ===
_resolve_opensuse() {
    local logical="$1"
    case "$logical" in
        steam) echo "steam" ;;
        lutris) echo "lutris" ;;
        heroic) echo "heroic-games-launcher" ;;
        bottles) echo "bottles" ;;
        
        mangohud) echo "MangoHud" ;;
        gamemode) echo "gamemode gamemode-32bit" ;;
        gamescope) echo "gamescope" ;;
        goverlay) echo "goverlay" ;;
        protontricks) echo "protontricks" ;;
        protonup-qt) echo "protonup-qt" ;;
        
        nvidia-driver) echo "nvidia-computeG06 nvidia-gfxG06-kmp-default nvidia-glG06 nvidia-glG06-32bit" ;;
        amd-driver) echo "Mesa Mesa-32bit Mesa-dri Mesa-dri-32bit vulkan-tools" ;;
        intel-driver) echo "Mesa Mesa-32bit Mesa-dri Mesa-dri-32bit intel-media-driver intel-media-driver-32bit" ;;
        opencl-amd) echo "ocl-icd opencl-headers clinfo" ;;
        opencl-nvidia) echo "nvidia-computeG06 nvidia-computeG06-32bit" ;;
        rocm) echo "rocm-opencl-runtime rocm-hip-runtime" ;;
        
        vulkan-tools) echo "vulkan-tools vulkan-validationlayers" ;;
        vkd3d) echo "vkd3d vkd3d-32bit" ;;
        dxvk) echo "dxvk" ;;
        gpu-viewer) echo "gpu-viewer" ;;
        radeontop) echo "radeontop" ;;
        nvtop) echo "nvtop" ;;
        intel-gpu-tools) echo "intel-gpu-tools" ;;
        
        input-remapper) echo "input-remapper" ;;
        piper) echo "piper" ;;
        ratbagctl) echo "ratbagctl" ;;
        sole) echo "sole" ;;
        openrazer) echo "openrazer-daemon openrazer-driver-dkms" ;;
        logitech) echo "piper ratbagctl" ;;
        
        sunshine) echo "sunshine" ;;
        moonlight) echo "moonlight-qt" ;;
        obs-studio) echo "obs-studio" ;;
        obs-vkcapture) echo "obs-vkcapture" ;;
        v4l2loopback) echo "v4l2loopback-kmp-default" ;;
        
        pipewire) echo "pipewire wireplumber pipewire-pulseaudio pipewire-alsa pipewire-jack pipewire-32bit wireplumber-32bit" ;;
        easyeffects) echo "easyeffects" ;;
        helvum) echo "helvum" ;;
        
        ffmpeg) echo "ffmpeg-6" ;;
        gstreamer) echo "gstreamer-plugins-base gstreamer-plugins-good gstreamer-plugins-bad gstreamer-plugins-ugly gstreamer-plugins-libav" ;;
        codecs) echo "ffmpeg-6 gstreamer-plugins-base gstreamer-plugins-good gstreamer-plugins-bad gstreamer-plugins-ugly gstreamer-plugins-libav" ;;
        vaapi) echo "intel-media-driver libva-utils vainfo" ;;
        vdpau) echo "vdpauinfo libvdpau_va_gl1" ;;
        
        libdisplay-info) echo "libdisplay-info" ;;
        zram-generator) echo "zram-generator" ;;
        btrfs) echo "btrfsprogs" ;;
        fstrim) echo "util-linux" ;;
        
        power-profiles-daemon) echo "power-profiles-daemon" ;;
        tuned) echo "tuned" ;;
        cpupower) echo "kernel-tools" ;;
        thermald) echo "thermald" ;;
        auto-cpufreq) echo "auto-cpufreq" ;;

        # HDR/VRR
        kwin-effects-hdr) echo "kwin" ;;
        color-management) echo "colord" ;;
        vkbasalt) echo "vkbasalt" ;;

        sysctl) echo "procps" ;;
        kernel-params) echo "grub2" ;;
        
        gamemoded) echo "gamemode" ;;
        lib32-gamemode) echo "gamemode-32bit" ;;
        
        flatpak) echo "flatpak" ;;
                flathub) echo "flatpak" ;;

                # Input peripherals (added for openSUSE support)
                lact) echo "lact" ;;
                corectrl) echo "corectrl" ;;
                libratbag) echo "libratbag" ;;
                input-remapper-gtk) echo "input-remapper-gtk" ;;
                opentabletdriver) echo "opentabletdriver" ;;
                xone-dkms) echo "xone" ;;
                distrobox) echo "distrobox" ;;
                waydroid) echo "waydroid" ;;

                # VA-API / codecs
                libva-mesa-driver) echo "Mesa-libva" ;;
                gstreamer-vaapi) echo "gstreamer-plugins-vaapi" ;;

                # 32-bit packages (some may require additional repos)
                lib32-mangohud) echo "mangohud-32bit" ;;
                lib32-obs-vkcapture) echo "obs-vkcapture-32bit" ;;
                lib32-vkbasalt) echo "vkbasalt-32bit" ;;
                lib32-gamemode) echo "libgamemode0-32bit" ;;

                htop) echo "htop" ;;
                btop) echo "btop" ;;
                neofetch) echo "neofetch" ;;
                fastfetch) echo "fastfetch" ;;

                nvidia-dkms) echo "nvidia-computeG06 nvidia-gfxG06-kmp-default nvidia-glG06 nvidia-glG06-32bit" ;;
                amd-pstate) echo "" ;;
                intel-pstate) echo "" ;;
                *) return 1 ;;
    esac
}

# === Fedora package maps ===
_resolve_fedora() {
    local logical="$1"
    case "$logical" in
        steam) echo "steam" ;;
        lutris) echo "lutris" ;;
        heroic) echo "heroic-games-launcher" ;;
        bottles) echo "bottles" ;;
        
        mangohud) echo "mangohud" ;;
        gamemode) echo "gamemode gamemode.i686" ;;
        gamescope) echo "gamescope" ;;
        goverlay) echo "goverlay" ;;
        protontricks) echo "protontricks" ;;
        protonup-qt) echo "protonup-qt" ;;
        
        nvidia-driver) echo "akmod-nvidia xorg-x11-drv-nvidia xorg-x11-drv-nvidia-libs.i686" ;;
        nvidia-dkms) echo "akmod-nvidia xorg-x11-drv-nvidia xorg-x11-drv-nvidia-libs.i686" ;;
        amd-driver) echo "mesa-vulkan-drivers mesa-vulkan-drivers.i686 mesa-dri-drivers mesa-dri-drivers.i686" ;;
        intel-driver) echo "mesa-vulkan-drivers mesa-vulkan-drivers.i686 mesa-dri-drivers mesa-dri-drivers.i686 intel-media-driver intel-media-driver.i686" ;;
        intel-media-driver) echo "intel-media-driver intel-media-driver.i686" ;;
        opencl-amd) echo "mesa-libOpenCL clinfo" ;;
        opencl-nvidia) echo "xorg-x11-drv-nvidia-libs xorg-x11-drv-nvidia-libs.i686" ;;
        rocm) echo "rocm-opencl-runtime rocm-hip-runtime" ;;
        
        vulkan-tools) echo "vulkan-tools vulkan-validation-layers" ;;
        vkd3d) echo "vkd3d vkd3d.i686" ;;
        dxvk) echo "dxvk" ;;
        gpu-viewer) echo "gpu-viewer" ;;
        radeontop) echo "radeontop" ;;
        nvtop) echo "nvtop" ;;
        intel-gpu-tools) echo "intel-gpu-tools" ;;
        
        input-remapper) echo "input-remapper" ;;
        piper) echo "piper" ;;
        ratbagctl) echo "ratbagctl" ;;
        sole) echo "sole" ;;
        openrazer) echo "openrazer-daemon openrazer-driver-dkms" ;;
        logitech) echo "piper ratbagctl" ;;
        
        sunshine) echo "sunshine" ;;
        moonlight) echo "moonlight-qt" ;;
        obs-studio) echo "obs-studio" ;;
        obs-vkcapture) echo "obs-vkcapture" ;;
        v4l2loopback) echo "v4l2loopback-kmod" ;;
        
        pipewire) echo "pipewire wireplumber pipewire-pulseaudio pipewire-alsa pipewire-jack pipewire-libs.i686 wireplumber-libs.i686" ;;
        easyeffects) echo "easyeffects" ;;
        helvum) echo "helvum" ;;
        
        ffmpeg) echo "ffmpeg ffmpeg-free" ;;
        gstreamer) echo "gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-plugins-ugly-free gstreamer1-plugins-bad-freeworld gstreamer1-libav" ;;
        codecs) echo "ffmpeg ffmpeg-free gstreamer1-plugins-base gstreamer1-plugins-good gstreamer1-plugins-bad-free gstreamer1-plugins-ugly-free gstreamer1-plugins-bad-freeworld gstreamer1-libav" ;;
        vaapi) echo "intel-media-driver libva-utils vainfo" ;;
        vdpau) echo "vdpauinfo libvdpau-va-gl" ;;
        
        zram-generator) echo "zram-generator" ;;
        btrfs) echo "btrfs-progs" ;;
        fstrim) echo "util-linux" ;;
        
        power-profiles-daemon) echo "power-profiles-daemon" ;;
        tuned) echo "tuned" ;;
        cpupower) echo "kernel-tools" ;;
        thermald) echo "thermald" ;;
        auto-cpufreq) echo "auto-cpufreq" ;;

        # HDR/VRR
                kwin-effects-hdr) echo "kwin" ;;
                color-management) echo "colord" ;;
                libdisplay-info) echo "libdisplay-info" ;;
                vkbasalt) echo "vkbasalt" ;;
                lib32-vkbasalt) echo "vkbasalt.i686" ;;

                sysctl) echo "procps-ng" ;;
                kernel-params) echo "grub2" ;;

                gamemoded) echo "gamemode" ;;
                lib32-gamemode) echo "gamemode.i686" ;;
                lib32-mangohud) echo "mangohud.i686" ;;

                # Codecs & media (additional)
                libva-mesa-driver) echo "mesa-libva" ;;
                gstreamer-vaapi) echo "gstreamer1-vaapi" ;;
                lib32-obs-vkcapture) echo "obs-vkcapture.i686" ;;

                # Display & GPU control
                lact) echo "lact" ;;
                corectrl) echo "corectrl" ;;
        
        flatpak) echo "flatpak" ;;
        flathub) echo "flatpak" ;;
        
        htop) echo "htop" ;;
        btop) echo "btop" ;;
        neofetch) echo "neofetch" ;;
        fastfetch) echo "fastfetch" ;;
        *) return 1 ;;
    esac
}

# List all logical package names that have mappings
list_logical_packages() {
    echo "steam lutris heroic bottles mangohud gamemode gamescope goverlay protontricks protonup-qt"
    echo "nvidia-driver nvidia-dkms amd-driver intel-driver opencl-amd opencl-nvidia rocm"
    echo "vulkan-tools vkd3d dxvk gpu-viewer radeontop nvtop intel-gpu-tools"
    echo "input-remapper piper ratbagctl sole openrazer logitech"
    echo "sunshine moonlight obs-studio obs-vkcapture v4l2loopback"
    echo "pipewire easyeffects helvum"
    echo "ffmpeg gstreamer codecs vaapi vdpau"
    echo "zram-generator btrfs fstrim"
    echo "power-profiles-daemon tuned cpupower thermald auto-cpufreq"
    echo "sysctl kernel-params"
    echo "gamemoded lib32-gamemode"
    echo "flatpak flathub"
    echo "htop btop neofetch fastfetch"
}

# Check if a logical package has a mapping for current distro
has_mapping() {
    local logical="${1:-}"
    [[ -z "$logical" ]] && return 1
    resolve_package "$logical" >/dev/null 2>&1
}