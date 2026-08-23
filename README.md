# bazzitify

Bazzite-style gaming optimizations — **without the immutable base**.

[Bazzite](https://bazzite.gg/) ships a great out-of-the-box gaming experience, but it locks you into a Fedora Atomic image. `bazzitify` extracts what actually makes Bazzite good (kernel params, services, drivers, tooling) and applies it as an opt-in set of scripts on the distro you already run:

- **Arch / CachyOS** (primary)
- Ubuntu-based (planned)
- openSUSE Tumbleweed (planned)

## Philosophy

- **Mutable by choice** — your distro, your package manager, no image rebasing.
- **Modular** — every tweak is its own module; apply only what you want.
- **Idempotent & reversible** — safe to re-run; every change is logged for undo (`--undo`).
- **Transparent** — nothing runs silently; each module prints what it changes.

## Usage

```bash
./bin/bazzitify --list            # show available modules
./bin/bazzitify --dry-run         # preview all changes
./bin/bazzitify                   # interactive module picker
./bin/bazzitify --all             # apply everything
./bin/bazzitify undo              # revert applied modules
```

## Modules

| Module | What it does |
|---|---|
| `gaming-packages` | Steam, Lutris, Heroic, ProtonUp-Qt, MangoHud, gamescope, gamemode |
| `gpu-drivers` | Mesa/NVK/NVIDIA driver setup per vendor |
| `kernel-params` | Gaming-oriented kernel cmdline tuning |
| `sysctl` | VM/swappiness/network latency tweaks |
| `services` | Disable/enable services for lower latency |
| `filesystems` | TRIM timers, ext4/xfs/btrfs mount hygiene |
| `flatpak` | Flatpak runtime + gaming apps |

## Status

Early scaffolding. See [docs/ROADMAP.md](docs/ROADMAP.md).
