#!/bin/bash
# desc: Filesystem — weekly SSD TRIM timer + zram swap config
# depends: sysctl
set -euo pipefail

ZRAM_CONF=/etc/systemd/zram-generator.conf
ZRAM_CONTENT="[zram0]
zram-size = min(ram, 16384)
compression-algorithm = zstd
swap-priority = 100
fs-type = swap
"

module_apply() {
  # TRIM timer (harmless on non-SSD; systemd just won't discard anything useful)
  sudo systemctl enable --now fstrim.timer && echo "fstrim.timer enabled (weekly TRIM)"

  # zram
  if command -v pkg_install >/dev/null 2>&1; then
    source "$(dirname "${BASH_SOURCE[0]}")/lib/distro.sh"
    source "$(dirname "${BASH_SOURCE[0]}")/lib/packages.sh"
    local zram_pkgs
    zram_pkgs=$(resolve_package_list zram-generator)
    if [ -n "$zram_pkgs" ]; then
      pkg_install $zram_pkgs
    else
      echo "zram-generator unavailable for this distro; skipping zram"
      return 0
    fi
    if [ ! -f "$ZRAM_CONF" ]; then
      echo "$ZRAM_CONTENT" | sudo tee "$ZRAM_CONF" >/dev/null
      sudo systemctl daemon-reload
      sudo systemctl start systemd-zram-setup@zram0.service 2>/dev/null || true
      echo "zram configured: min(ram,16G) zstd, priority 100"
    else
      echo "zram conf already present at $ZRAM_CONF; leaving untouched"
    fi
  else
    echo "zram-generator unavailable; skipping zram"
  fi

  # Swappiness for gaming (less aggressive swap-out)
  printf 'vm.swappiness=10\n' | sudo tee /etc/sysctl.d/80-bazzitify-zram.conf >/dev/null
  sudo sysctl -p /etc/sysctl.d/80-bazzitify-zram.conf
}

module_undo() {
  sudo systemctl disable --now fstrim.timer 2>/dev/null && echo "fstrim.timer disabled"
  if [ -f "$ZRAM_CONF" ] && grep -q "bazzitify\|min(ram" "$ZRAM_CONF" 2>/dev/null; then
    sudo rm "$ZRAM_CONF" && echo "removed $ZRAM_CONF"
    sudo systemctl daemon-reload
  fi
  sudo rm -f /etc/sysctl.d/80-bazzitify-zram.conf
}
