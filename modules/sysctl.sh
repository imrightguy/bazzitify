# desc: sysctl latency/VM tweaks for gaming

SYSCTL_FILE=/etc/sysctl.d/99-bazzitify-gaming.conf

module_apply() {
  echo "  Writing $SYSCTL_FILE"
  sudo tee "$SYSCTL_FILE" >/dev/null <<'EOF'
# bazzitify gaming sysctl
vm.swappiness = 10
vm.vfs_cache_pressure = 50
vm.max_map_count = 2147483642
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
EOF
  sudo sysctl --system >/dev/null
}

module_undo() {
  echo "  Removing $SYSCTL_FILE"
  sudo rm -f "$SYSCTL_FILE"
  sudo sysctl --system >/dev/null
}
