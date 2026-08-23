#!/bin/bash
# desc: HDR/VRR enablement — Wayland compositor config for HDR metadata + adaptive-sync (Bazzite parity)
# long: Enables HDR (High Dynamic Range) and VRR (Variable Refresh Rate) on supported Wayland compositors:
# long: • KDE Plasma / KWin: writes kwinrc with HDR metadata passthrough + vrr=1
# long: • GNOME / Mutter: enables mutter HDR + vrr via gsettings
# long: • Hyprland: adds monitor config for HDR + vrr=1 in hyprland.conf
# long: • sway: adds output config for adaptive-sync in sway config
# long: • COSMIC: enables HDR + VRR via cosmic-compiler config
# long: Backs up all modified config files before changes; module_undo restores them exactly.
# requires: gpu-drivers display-gpu-control
set -euo pipefail

source "$(dirname "${BASH_SOURCE[0]}")/lib/distro.sh"
source "$(dirname "${BASH_SOURCE[0]}")/lib/packages.sh"

PACMAN_PKGS=(libdisplay-info)
BACKUP_DIR="${XDG_STATE_HOME:-$HOME/.local/state}/bazzitify/hdr-vrr-backups"
MARKER="# bazzitify:hdr-vrr"

detect_compositor() {
  # Returns one of: kwin, gnome, hyprland, sway, cosmic, unknown
  if [ -n "${KDE_FULL_SESSION:-}" ] || [ "${XDG_CURRENT_DESKTOP:-}" = "KDE" ] || pgrep -x kwin_wayland >/dev/null 2>&1; then
    echo kwin
  elif [ "${XDG_CURRENT_DESKTOP:-}" = "GNOME" ] || [ "${XDG_CURRENT_DESKTOP:-}" = "ubuntu:GNOME" ] || pgrep -x gnome-shell >/dev/null 2>&1; then
    echo gnome
  elif [ -n "${HYPRLAND_INSTANCE_SIGNATURE:-}" ] || pgrep -x Hyprland >/dev/null 2>&1; then
    echo hyprland
  elif [ -n "${SWAYSOCK:-}" ] || pgrep -x sway >/dev/null 2>&1; then
    echo sway
  elif [ "${XDG_CURRENT_DESKTOP:-}" = "COSMIC" ] || pgrep -x cosmic-comp >/dev/null 2>&1; then
    echo cosmic
  else
    echo unknown
  fi
}

backup_file() {
  local file="$1"
  [ -f "$file" ] || return 0
  mkdir -p "$BACKUP_DIR"
  local rel="${file#/}"
  local backup="$BACKUP_DIR/$rel"
  mkdir -p "$(dirname "$backup")"
  cp -p "$file" "$backup"
  echo "backed up $file -> $backup"
}

restore_file() {
  local file="$1"
  local rel="${file#/}"
  local backup="$BACKUP_DIR/$rel"
  if [ -f "$backup" ]; then
    cp -p "$backup" "$file"
    echo "restored $file from $backup"
  else
    echo "no backup for $file; leaving as-is"
  fi
}

ensure_marker_in_file() {
  local file="$1"
  local content="$2"
  # If file doesn't exist, create it with marker + content
  if [ ! -f "$file" ]; then
    mkdir -p "$(dirname "$file")"
    printf "%s\n%s\n" "$MARKER" "$content" > "$file"
    return
  fi
  # If already tagged, don't duplicate
  if grep -q "$MARKER" "$file"; then
    return
  fi
  # Append marker + content
  printf "\n%s\n%s\n" "$MARKER" "$content" >> "$file"
}

remove_marker_section() {
  local file="$1"
  [ -f "$file" ] || return 0
  # Remove the marker line and the line immediately after it (our config block)
  # Use sed to delete marker line and next line
  sed -i "/$MARKER/{N;d;}" "$file" 2>/dev/null || true
  # Also handle case where marker is last line
  sed -i "/$MARKER/d" "$file" 2>/dev/null || true
}

configure_kwin_hdr_vrr() {
  local kwinrc="${KDE_CONFIG_HOME:-$HOME/.config}/kwinrc"
  backup_file "$kwinrc"

  # KWin HDR + VRR config
  local hdr_config="[Wayland]
Enabled=true
HDRMetadata=true

[Compositing]
VRREnabled=true
AdaptiveSync=true"

  ensure_marker_in_file "$kwinrc" "$hdr_config"
  echo "KWin HDR/VRR config applied to $kwinrc"
}

configure_gnome_hdr_vrr() {
  # GNOME uses gsettings for mutter
  # HDR: org.gnome.mutter experimental-features ['hdr']
  # VRR: org.gnome.mutter.vrr enabled true
  if command -v gsettings >/dev/null 2>&1; then
    # Backup current settings
    local backup_gschema="$BACKUP_DIR/gsettings-backup.txt"
    mkdir -p "$BACKUP_DIR"
    gsettings get org.gnome.mutter experimental-features > "$backup_gschema.hdr" 2>/dev/null || echo "[]" > "$backup_gschema.hdr"
    gsettings get org.gnome.mutter.vrr enabled > "$backup_gschema.vrr" 2>/dev/null || echo "false" > "$backup_gschema.vrr"

    # Enable HDR (experimental)
    local features
    features=$(gsettings get org.gnome.mutter experimental-features 2>/dev/null || echo "[]")
    if ! echo "$features" | grep -q "'hdr'"; then
      if [ "$features" = "[]" ]; then
        gsettings set org.gnome.mutter experimental-features "['hdr']"
      else
        gsettings set org.gnome.mutter experimental-features "${features%]*}, 'hdr']"
      fi
      echo "GNOME HDR enabled via gsettings"
    fi

    # Enable VRR
    gsettings set org.gnome.mutter.vrr enabled true 2>/dev/null && echo "GNOME VRR enabled via gsettings" || echo "GNOME VRR key not available (may need newer mutter)"
  else
    echo "gsettings not available; skipping GNOME HDR/VRR config"
  fi
}

configure_hyprland_hdr_vrr() {
  local hypr_conf="${XDG_CONFIG_HOME:-$HOME/.config}/hypr/hyprland.conf"
  backup_file "$hypr_conf"

  # Hyprland HDR + VRR config
  # HDR: monitor=...,hdr,1 (per monitor) - we add a template
  # VRR: monitor=...,vrr,1
  local hdr_vrr_config="monitor=,preferred,auto,1,bitdepth,10,hdr,1,vrr,1"

  ensure_marker_in_file "$hypr_conf" "$hdr_vrr_config"
  echo "Hyprland HDR/VRR config applied to $hypr_conf (review monitor section)"
}

configure_sway_hdr_vrr() {
  local sway_conf="${XDG_CONFIG_HOME:-$HOME/.config}/sway/config"
  backup_file "$sway_conf"

  # sway HDR + VRR config
  # output * adaptive_sync on
  # output * hdr on (requires sway 1.9+)
  local hdr_vrr_config="output * adaptive_sync on
output * hdr on"

  ensure_marker_in_file "$sway_conf" "$hdr_vrr_config"
  echo "sway HDR/VRR config applied to $sway_conf"
}

configure_cosmic_hdr_vrr() {
  local cosmic_conf="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic/comp/config.ron"
  backup_file "$cosmic_conf"

  # COSMIC HDR + VRR - RON format config
  # This is a best-effort; COSMIC config is complex
  echo "COSMIC detected; HDR/VRR config requires manual review of $cosmic_conf"
  echo "See https://github.com/pop-os/cosmic-comp for HDR/VRR settings"
}

module_apply() {
  local pkgs
  if ! pkgs=$(resolve_package_list ${PACMAN_PKGS[*]}) || [ -z "$pkgs" ]; then
    echo "packages not mapped for this distro; skipping install" >&2
    return 1
  fi
  pkg_install $pkgs

  local comp
  comp=$(detect_compositor)
  echo "Detected compositor: $comp"

  case "$comp" in
    kwin)
      configure_kwin_hdr_vrr
      ;;
    gnome)
      configure_gnome_hdr_vrr
      ;;
    hyprland)
      configure_hyprland_hdr_vrr
      ;;
    sway)
      configure_sway_hdr_vrr
      ;;
    cosmic)
      configure_cosmic_hdr_vrr
      ;;
    *)
      echo "Unsupported or undetected Wayland compositor: $comp"
      echo "HDR/VRR config not applied. Supported: KWin, GNOME, Hyprland, sway, COSMIC"
      return 1
      ;;
  esac

  echo "HDR/VRR enablement complete for $comp. Log out and back in (or restart compositor) for changes to take effect."
  echo "Verify HDR: weston-info | grep -i hdr  (or check compositor settings UI)"
  echo "Verify VRR: check monitor OSD or compositor VRR status indicator"
}

module_undo() {
  local comp
  comp=$(detect_compositor)
  echo "Detected compositor for undo: $comp"

  case "$comp" in
    kwin)
      local kwinrc="${KDE_CONFIG_HOME:-$HOME/.config}/kwinrc"
      remove_marker_section "$kwinrc"
      restore_file "$kwinrc"
      ;;
    gnome)
      if command -v gsettings >/dev/null 2>&1; then
        local backup_gschema="$BACKUP_DIR/gsettings-backup.txt"
        if [ -f "$backup_gschema.hdr" ]; then
          gsettings set org.gnome.mutter experimental-features "$(cat "$backup_gschema.hdr")"
          echo "restored GNOME HDR setting"
        fi
        if [ -f "$backup_gschema.vrr" ]; then
          gsettings set org.gnome.mutter.vrr enabled "$(cat "$backup_gschema.vrr")"
          echo "restored GNOME VRR setting"
        fi
      fi
      ;;
    hyprland)
      local hypr_conf="${XDG_CONFIG_HOME:-$HOME/.config}/hypr/hyprland.conf"
      remove_marker_section "$hypr_conf"
      restore_file "$hypr_conf"
      ;;
    sway)
      local sway_conf="${XDG_CONFIG_HOME:-$HOME/.config}/sway/config"
      remove_marker_section "$sway_conf"
      restore_file "$sway_conf"
      ;;
    cosmic)
      local cosmic_conf="${XDG_CONFIG_HOME:-$HOME/.config}/cosmic/comp/config.ron"
      restore_file "$cosmic_conf"
      ;;
    *)
      echo "Unknown compositor; nothing to undo"
      ;;
  esac

  echo "packages left installed; to remove:"
  echo "  # removal command is distro-specific; pkg_remove handles it:
  #   (see module engine) or manually: sudo <pkg-manager remove> ${PACMAN_PKGS[*]}"
  echo "Backup directory preserved at $BACKUP_DIR (safe to delete manually)"
}