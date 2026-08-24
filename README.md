<div align="center">
  <img src="https://img.shields.io/badge/🎮_bazzitify-gaming%20tweaks%2C%20no%20immutability-cba6f7?style=for-the-badge&labelColor=1e1e2e" alt="bazzitify"/>
</div>

<div align="center">

[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Linux-orange?style=flat-square&logo=linux&logoColor=white)](#-getting-started)
[![Distro](https://img.shields.io/badge/Arch%20%7C%20CachyOS-supported-1793D1?style=flat-square&logo=archlinux&logoColor=white)](#-supported-distros)
[![Built with](https://img.shields.io/badge/Rust%20%2B%20Slint-native%20GUI-dea584?style=flat-square&logo=rust&logoColor=white)](#-architecture)

**Bazzite's proven gaming optimizations, packaged as opt-in reversible modules
for the mutable distro you already run.**

[Why](#-why) • [Modules](#-modules) • [Getting Started](#-getting-started) • [Writing Modules](#-writing-a-module) • [Roadmap](#-roadmap)

</div>

---

# Table of Contents

- [Why](#-why)
- [Modules](#-modules)
  - [Module safety model](#module-safety-model)
- [Getting Started](#-getting-started)
  - [GUI](#gui)
  - [CLI](#cli)
- [Writing a Module](#-writing-a-module)
- [Architecture](#-architecture)
- [Supported Distros](#-supported-distros)
- [Roadmap](#-roadmap)
- [Special Thanks](#-special-thanks)

---

## 🧭 Why

[Bazzite](https://bazzite.gg/) ships arguably the best out-of-the-box Linux gaming experience — but it's an **immutable** Fedora Atomic image. If you run Arch, CachyOS, or any traditional distro, your choices have been: switch everything, or hand-roll scattered tweaks from wiki pages and forum posts.

**bazzitify is the third option.** Each of Bazzite's proven optimizations becomes a small, readable bash module that:

- ✅ **Opt-in** — nothing changes until *you* click Apply
- ✅ **Explained** — every module describes exactly what it does, before it does it, right in the GUI
- ✅ **Reversible** — modules ship `undo`; applied state is logged to `~/.local/state/bazzitify/applied.log`
- ✅ **Readable** — each module is a short bash script you can open and audit in one screen

> [!IMPORTANT]
> Modules touch system configuration (`sysctl`, bootloader params, packages).
> Read a module's detail page in the GUI before applying it. Nothing runs automatically.

## 📦 Modules

| Module | What it does | Undo |
|---|---|---|
| **gaming-packages** | Steam, Lutris, MangoHud, gamescope, gamemode via pacman/apt | partial¹ |
| **sysctl** | VM & latency tuning: swappiness, `vm.max_map_count` for Proton, fq + BBR congestion control (`/etc/sysctl.d/99-bazzitify-gaming.conf`) | ✅ full |
| **gpu-drivers** | Hardware auto-detected via `lspci`: Mesa + RADV/VAAPI (AMD) or nvidia-utils + 32-bit libs (NVIDIA) | by design² |
| **kernel-params** | Bootloader-aware (systemd-boot & GRUB): `nowatchdog`, `nmi_watchdog=0`, `split_lock_detect=off` — tagged entries, idempotent | ✅ idempotent |
| **services** | GameMode daemon install + user service enablement | ✅ full |
| **filesystems** | Weekly `fstrim.timer`, zram swap (zstd, capped), lower swappiness for zram setups | ✅ full |
| **flatpak** | Flathub remote + ProtonUp-Qt and protontricks GUI | partial |
| **hdr-vrr** | HDR/VRR gaming helpers: compositor detection (KWin ≥6, gamescope, Hyprland, sway), KWin scripts, env vars, VRR kernel param | ✅ full¹ |

<sub>¹ Package *removal* on undo is deliberately conservative — it never removes anything you might have installed yourself before running bazzitify.<br/>
² Uninstalling GPU drivers would leave you at a black screen; the undo refuses on purpose.</sub>

### Module safety model

Every module follows the same contract:

```text
module_apply()   → makes the change, safe to re-run (idempotent where possible)
module_undo()    → restores prior state; refuses when restoring would be harmful
# desc:          → one-line summary shown in the sidebar list
# long:          → detail lines shown on the module page BEFORE you apply
```

## 🚀 Getting Started

### Requirements

- Linux with bash
- Rust stable (to build): `rustup default stable`
- A distro using pacman (Arch/CachyOS) or apt (Debian/Ubuntu) for package modules

### GUI

```bash
git clone https://github.com/imrightguy/bazzitify
cd bazzitify
cargo run --release
```

A native Slint window opens with COSMIC-Tweaks-style navigation:
pick a module in the left sidebar, read its detail page, then Apply or Undo.
Output streams to the log pane live while modules run — the UI never freezes.

### CLI

Same engine, no window:

```bash
bin/bazzitify --list              # discover modules
bin/bazzitify --dry-run           # preview what apply would do
bin/bazzitify apply sysctl        # apply one module
bin/bazzitify undo sysctl         # revert it
```

Applied state is logged to `~/.local/state/bazzitify/applied.log`.

## 🧩 Writing a Module

Drop a bash file into `modules/`:

```bash
#!/bin/bash
# desc: One-line summary shown in the sidebar
# long: Detailed explanation shown on the module page in the GUI.
# long: Add as many "# long:" lines as you need.

module_apply() {
  # your tweaks here
}

module_undo() {
  # restore previous state
}
```

That's the whole API. The GUI auto-discovers it, renders its description,
and wires up Apply / Undo buttons. No registration, no manifest.

## 🏗️ Architecture

```text
┌────────────────────────────────────────────────┐
│                ui/app.slint                    │
│     sidebar nav · detail pages · log pane      │
└──────────────────┬─────────────────────────────┘
                   │ event channel (mpsc)
┌──────────────────▼─────────────────────────────┐
│  src/main.rs   UI state · worker threads       │
│  src/module.rs parse + discover modules        │
│  src/runner.rs execute module_apply/_undo      │
└──────────────────┬─────────────────────────────┘
                   │ bash
┌──────────────────▼─────────────────────────────┐
│            modules/*.sh                        │
│   plain scripts · # desc / # long headers      │
└────────────────────────────────────────────────┘
```

- **Rust + Slint** native frontend — no Electron, no Python runtime
- Modules run on background threads; results stream back over a channel so the UI stays responsive
- The same modules power both GUI and CLI — one source of truth

## 🐧 Supported Distros

| Distro | Status |
|---|---|
| **Arch / CachyOS** | ✅ primary target |
| Debian / Ubuntu bases | 🟡 `gaming-packages` has an apt branch |
| openSUSE / Fedora | 🔜 planned |

## 🗺️ Roadmap

- [ ] Dry-run toggle inside the GUI
- [ ] Module dependency ordering (e.g. gpu-drivers → kernel-params)
- [ ] HDR / VRR parity audit against upstream Bazzite
- [ ] openSUSE support (zypper branch)
- [ ] Export/import a "profile" of selected modules

## 💜 Special Thanks

- [Bazzite](https://bazzite.gg/) & [Universal Blue](https://universal-blue.org/) — the inspiration and reference implementations
- [COSMIC Tweaks](https://github.com/cosmic-utils/tweaks) — layout inspiration
- [Slint](https://slint.dev/) — the GUI toolkit

<div align="center">
<sub>Not affiliated with Bazzite or Universal Blue — just inspired by them.<br/>
Licensed under <a href="LICENSE">MIT</a>.</sub>
</div>
