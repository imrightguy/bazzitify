#!/bin/bash
# desc: Flatpak — Flathub remote + gaming apps (ProtonPlus, Bottles optional)
set -euo pipefail

FLATPAK_APPS=(
  com.github.Matoking.protontricks   # protontricks GUI wrapper
  net.davidotek.pupgui2              # ProtonUp-Qt: manage Proton-GE
)

flathub_present() {
  flatpak remotes --columns=name 2>/dev/null | grep -qx flathub
}

module_apply() {
  if ! command -v flatpak >/dev/null 2>&1; then
    sudo pacman -S --needed --noconfirm flatpak
  fi
  if ! flathub_present; then
    sudo flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo
    echo "Flathub added"
  fi
  for app in "${FLATPAK_APPS[@]}"; do
    flatpak install -y --noninteractive flathub "$app" && echo "installed: $app"
  done
}

module_undo() {
  for app in "${FLATPAK_APPS[@]}"; do
    flatpak uninstall -y --noninteractive "$app" 2>/dev/null && echo "uninstalled: $app"
  done
  echo "Note: Flathub remote left in place."
}
