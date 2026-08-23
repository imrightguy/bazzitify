# desc: Install gaming packages (Steam, Lutris, MangoHud, gamescope, gamemode)

module_apply() {
  echo "  Installing gaming packages for $(cat /etc/os-release | grep -oP '(?<=^ID=).+' | tr -d '"')"
  if command -v pacman >/dev/null; then
    sudo pacman -S --needed --noconfirm steam lutris mangohud gamescope gamemode lib32-mangohud lib32-gamemode
  elif command -v apt >/dev/null; then
    sudo apt-get update
    sudo apt-get install -y steam lutris mangohud gamescope gamemode
  else
    echo "  Unsupported package manager" >&2; return 1
  fi
}

module_undo() {
  echo "  (Package removal intentionally not automated — remove manually if desired)"
}
