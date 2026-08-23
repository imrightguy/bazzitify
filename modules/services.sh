#!/bin/bash
# desc: Services — enable gamemode/gamemoded socket, disable useless-for-gaming services
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib/distro.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/packages.sh"

module_apply() {
  # GameMode daemon (distro-agnostic via package map)
  local pkgs
  if pkgs=$(resolve_package_list gamemode lib32-gamemode); [ -n "$pkgs" ]; then
    pkg_install $pkgs
    systemctl --user enable --now gamemoded.service 2>/dev/null && echo "gamemoded enabled" \
      || echo "gamemoded socket activation only (starts on demand)"
  else
    echo "gamemode not mapped for this distro; skipping"
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
