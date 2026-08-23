#!/bin/bash
# desc: Codecs & capture — full hw codec support, MangoHud overlay, OBS vkcapture, vkBasalt
# long: Matches Bazzite's out-of-the-box media/gaming stack:
# long: • Full hardware-accelerated H264 decode (libva + mesa drivers)
# long: • MangoHud — performance overlay (Vulkan/OpenGL)
# long: • OBS VkCapture — hardware-accelerated game capture for OBS
# long: • vkBasalt — Vulkan post-processing layer (CAS sharpening etc.)
# long: • GStreamer vaapi plugins for desktop video players
set -euo pipefail

PACMAN_PKGS=(
  libva-mesa-driver   # VAAPI for Mesa/AMD
  intel-media-driver  # VAAPI for Intel
  gstreamer-vaapi
  mangohud lib32-mangohud
  obs-vkcapture lib32-obs-vkcapture
  vkbasalt lib32-vkbasalt
)
APT_PKGS=(mesa-va-drivers gstreamer1.0-vaapi mangohud)

have() { pacman -Qi "$1" >/dev/null 2>&1 || dpkg -s "$1" >/dev/null 2>&1; }

module_apply() {
  if command -v pacman >/dev/null 2>&1; then
    local missing=()
    for p in "${PACMAN_PKGS[@]}"; do have "$p" || missing+=("$p"); done
    (( ${#missing[@]} )) && sudo pacman -S --needed --noconfirm "${missing[@]}"
    echo "MangoHud usage: mangohud %command% in Steam launch options"
    echo "vkBasalt usage:  ENABLE_VKBASALT=1 %command%"
    echo "OBS capture:     run game through 'obs-vkcapture steam' or set in Lutris"
  elif command -v apt-get >/dev/null 2>&1; then
    sudo apt-get install -y "${APT_PKGS[@]}"
  else
    echo "unsupported package manager" >&2; return 1
  fi
}

module_undo() {
  echo "codecs/overlay packages left installed — they're inert without per-game opt-in."
  echo "to remove manually on Arch:"
  echo "  sudo pacman -Rns ${PACMAN_PKGS[*]}"
}
