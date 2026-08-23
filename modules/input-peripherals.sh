#!/bin/bash
# desc: Input peripherals — Xbox (xone), Razer, and tablet driver support
# long: Installs controller and peripheral drivers Bazzite ships by default:
# long: • xone — modern Xbox One/Series wireless dongle driver (DKMS)
# long: • OpenRazer — Razer peripheral support
# long: • input-remapper — per-device key/button remapping (preinstalled+enabled on Bazzite)
# long: • OpenTabletDriver — drawing tablet driver suite
# long: • libratbag — gaming-mouse DPI/button configuration
set -euo pipefail

PACMAN_PKGS=(xone-dkms-git openrazer-driver-dkms openrazer-daemon libratbag input-remapper input-remapper-gtk opentabletdriver)

module_apply() {
  if command -v pacman >/dev/null 2>&1; then
    for pkg in "${PACMAN_PKGS[@]}"; do
      # AUR packages need an AUR helper; try paru then yay, else skip with notice.
      if pacman -Si "$pkg" >/dev/null 2>&1; then
        sudo pacman -S --needed --noconfirm "$pkg"
      elif command -v paru >/dev/null 2>&1; then
        paru -S --needed --noconfirm "$pkg"
      elif command -v yay >/dev/null 2>&1; then
        yay -S --needed --noconfirm "$pkg"
      else
        echo "skip $pkg (not in repos; install an AUR helper like paru)"
      fi
    done
    # input-remapper service (Bazzite enables it by default)
    sudo systemctl enable --now input-remapper 2>/dev/null || true
    # udev rules reload
    sudo systemctl restart systemd-udevd 2>/dev/null || true
    sudo udevadm control --reload-rules 2>/dev/null || true
  else
    echo "unsupported package manager; Arch-family only right now" >&2
    return 1
  fi
}

module_undo() {
  if command -v pacman >/dev/null 2>&1; then
    echo "note: kernel modules are left installed; removing them can break paired devices."
    echo "to remove manually: sudo pacman -Rns ${PACMAN_PKGS[*]}"
  fi
}
