# Roadmap

## v0.1 — scaffolding (current)
- [x] Modular CLI (`bin/bazzitify`) with list/dry-run/undo
- [x] `gaming-packages` module (Arch + apt)
- [x] `sysctl` module
- [ ] `gpu-drivers` module (AMD/NVIDIA/Intel detection)
- [ ] `kernel-params` module
- [ ] `services` module (disable unneeded services, enable gamemode-related)
- [ ] `filesystems` module (TRIM, mount hygiene)
- [ ] `flatpak` module

## v0.2 — distro coverage
- [ ] openSUSE Tumbleweed (zypper) support
- [ ] Distro detection + per-distro package maps

## v0.3 — Bazzite parity audit
- [ ] Diff Bazzite's actual image config against our modules; port anything missing
- [ ] HDR/VRR enablement helpers
- [ ] Wayland gaming session defaults

## Later
- [ ] TUI (fzf picker)
- [ ] Per-game profiles
