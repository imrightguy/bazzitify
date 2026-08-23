#!/bin/bash
# desc: Kernel params — nowatchdog, split_lock_detect=off, amdgpu overrides (bootloader-aware)
set -euo pipefail

MARKER="bazzitify"
PARAMS="nowatchdog split_lock_detect=off nmi_watchdog=0"

detect_bootloader() {
  if [ -d /boot/loader/entries ] || [ -e /efi/loader/loader.conf ] || bootctl status >/dev/null 2>&1; then
    echo systemd-boot
  elif [ -f /etc/default/grub ]; then
    echo grub
  else
    echo unknown
  fi
}

module_apply() {
  local bl
  bl=$(detect_bootloader)
  case "$bl" in
    systemd-boot)
      echo "systemd-boot detected: appending params to entries (idempotent)"
      for entry in /boot/loader/entries/*.conf /efi/loader/entries/*.conf; do
        [ -f "$entry" ] || continue
        if grep -q "$MARKER" "$entry"; then
          echo "  $entry already tagged; skipping"
          continue
        fi
        sudo sed -i "s|^\(options .*\)$|\1 $PARAMS # $MARKER|" "$entry"
        echo "  updated $entry"
      done
      ;;
    grub)
      echo "GRUB detected: adding to GRUB_CMDLINE_LINUX_DEFAULT"
      if grep -q "$MARKER" /etc/default/grub; then
        echo "  already tagged; skipping"
      else
        sudo sed -i.bak-bazzitify "s|^\(GRUB_CMDLINE_LINUX_DEFAULT=\"\)\(.*\)\"|\1\2 $PARAMS\" # $MARKER|" /etc/default/grub
        if command -v grub-mkconfig >/dev/null; then
          sudo grub-mkconfig -o /boot/grub/grub.cfg
        fi
        echo "GRUB regenerated (backup at /etc/default/grub.bak-bazzitify)"
      fi
      ;;
    *)
      echo "Unknown bootloader — cannot apply kernel params safely. Aborting."
      return 1
      ;;
  esac
}

module_undo() {
  local bl
  bl=$(detect_bootloader)
  case "$bl" in
    systemd-boot)
      for entry in /boot/loader/entries/*.conf /efi/loader/entries/*.conf; do
        [ -f "$entry" ] || continue
        if grep -q "$MARKER" "$entry"; then
          sudo sed -i "s| $PARAMS # $MARKER||; s|$PARAMS # $MARKER||" "$entry"
          echo "  cleaned $entry"
        fi
      done
      ;;
    grub)
      if [ -f /etc/default/grub.bak-bazzitify ]; then
        sudo mv /etc/default/grub.bak-bazzitify /etc/default/grub
        sudo grub-mkconfig -o /boot/grub/grub.cfg
        echo "restored GRUB default from backup"
      fi
      ;;
    *) echo "unknown bootloader; nothing to undo" ;;
  esac
}
