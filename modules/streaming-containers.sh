#!/bin/bash
# desc: Streaming & containers — Sunshine stream host, distrobox, waydroid
# long: Bazzite-parity streaming and container support:
# long: • Sunshine — self-hosted game stream server (pairs with Moonlight clients)
# long: • distrobox — immutable-friendly container tooling for any distro userland
# long: • waydroid — Android in a container (optional; big download)
# long: Sunshine gets firewall ports opened (TCP/UDP 47984-48010 range).
set -euo pipefail

PACMAN_PKGS=(sunshine distrobox)
OPT_PKGS=(waydroid)

module_apply() {
  if ! command -v pacman >/dev/null 2>&1; then
    echo "unsupported package manager; Arch-family only right now" >&2
    return 1
  fi
  local helper=""
  command -v paru >/dev/null 2>&1 && helper=paru
  command -v yay  >/dev/null 2>&1 && [ -z "$helper" ] && helper=yay
  for pkg in "${PACMAN_PKGS[@]}"; do
    if pacman -Si "$pkg" >/dev/null 2>&1; then
      sudo pacman -S --needed --noconfirm "$pkg"
    elif [ -n "$helper" ]; then
      "$helper" -S --needed --noconfirm "$pkg"
    else
      echo "skip $pkg (not in repos; install an AUR helper)"
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
  echo "waydroid optional: install later with: $helper -S waydroid (or via flatpak)"
}

module_undo() {
  systemctl --user disable --now sunshine 2>/dev/null || true
  echo "to remove: sudo pacman -Rns ${PACMAN_PKGS[*]}"
}
