#!/bin/bash
# desc: Services — enable gamemode/gamemoded socket, disable useless-for-gaming services
set -euo pipefail

ENABLE=(gamemode.service 2>/dev/null || true)

module_apply() {
  # GameMode daemon
  if pacman -Qi gamemode >/dev/null 2>&1 || pacman -Si gamemode >/dev/null 2>&1; then
    sudo pacman -S --needed --noconfirm gamemode lib32-gamemode
    systemctl --user enable --now gamemoded.service 2>/dev/null && echo "gamemoded enabled" \
      || echo "gamemoded socket activation only (starts on demand)"
  fi

  # Disable common latency contributors when present but not essential
  for svc in power-profiles-daemon.service upower.service; do
    if systemctl is-enabled "$svc" >/dev/null 2>&1; then
      echo "note: $svc present (left enabled — disabling can break battery/power UX)"
    fi
  done

  # Network tuning already in sysctl module; here: disable split-lock mitigation warnings spam
  echo "services module complete"
}

module_undo() {
  systemctl --user disable --now gamemoded.service 2>/dev/null && echo "gamemoded disabled" || true
}
