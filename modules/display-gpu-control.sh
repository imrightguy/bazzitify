#!/bin/bash
# desc: Display & GPU control — LACT, CoreCtrl, gamescope session tools
# long: Bazzite-parity display/GPU management:
# long: • LACT — GPU control daemon + GUI (clocks, power limit, fan curves)
# long: • CoreCtrl — AMD/Intel CPU+GPU profile control
# long: • gamescope — Valve's micro-compositor (Steam Gaming Mode engine)
# long: LACT's systemd service is enabled so settings persist across reboots.
set -euo pipefail

PACMAN_PKGS=(lact corectrl gamescope)

module_apply() {
  if command -v pacman >/dev/null 2>&1; then
    for pkg in "${PACMAN_PKGS[@]}"; do
      if pacman -Si "$pkg" >/dev/null 2>&1; then
        sudo pacman -S --needed --noconfirm "$pkg"
      elif command -v paru >/dev/null 2>&1; then
        paru -S --needed --noconfirm "$pkg"
      elif command -v yay >/dev/null 2>&1; then
        yay -S --needed --noconfirm "$pkg"
      else
        echo "skip $pkg (not in repos; install an AUR helper)"
      fi
    done
    # Persist LACT settings across reboots (Bazzite does this out of the box)
    sudo systemctl enable --now lactd 2>/dev/null \
      || echo "lactd service not present; run 'sudo lact' once to generate config"
    # Allow CoreCtrl to run without password prompt hint
    echo "note: for CoreCtrl sudo-free control see its wiki (polkit rules)"
    echo "gamescope usage: gamescope -e -- %command% or via Steam 'gamescope %command%'"
  else
    echo "unsupported package manager; Arch-family only right now" >&2
    return 1
  fi
}

module_undo() {
  sudo systemctl disable --now lactd 2>/dev/null || true
  echo "packages left installed; to remove:"
  echo "  sudo pacman -Rns ${PACMAN_PKGS[*]}"
}
