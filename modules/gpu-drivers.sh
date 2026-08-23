#!/bin/bash
# desc: GPU drivers — Mesa/Vulkan for AMD, nvidia-utils for NVIDIA (auto-detected)
set -euo pipefail

PACMAN_PKGS_BASE=(mesa vulkan-radeon libva-mesa-driver mesa-vdpau)
PACMAN_PKGS_NVIDIA=(nvidia-utils nvidia-prime)
PKGS_32=(lib32-mesa lib32-vulkan-radeon lib32-nvidia-utils)

have_gpu() {
  case "$1" in
    amd) lspci -nn | grep -qi 'vga.*\[1002:\]\|display.*\[1002:\]' ;;
    nvidia) lspci -nn | grep -qi 'vga.*\[10de:\]\|display.*\[10de:\]' ;;
  esac
}

module_apply() {
  if have_gpu amd; then
    echo "AMD GPU detected: installing Mesa + RADV stack"
    sudo pacman -S --needed --noconfirm "${PACMAN_PKGS_BASE[@]}" "${PKGS_32[@]}"
  elif have_gpu nvidia; then
    echo "NVIDIA GPU detected: installing nvidia-utils"
    sudo pacman -S --needed --noconfirm "${PACMAN_PKGS_NVIDIA[@]}" "${PKGS_32[@]}"
  else
    echo "No discrete AMD/NVIDIA GPU found via lspci; installing base Mesa only"
    sudo pacman -S --needed --noconfirm mesa
  fi
}

module_undo() {
  echo "gpu-drivers: undo skipped intentionally — removing GPU drivers would break graphics."
  echo "(Uninstall manually if you really mean it.)"
}
