# ROADMAP — bazzitify

Machine-referable backlog. The **bazzitify-feature-planner** cron reads this file,
diffs it against the repo state and open GitHub issues, and opens one issue per
unclaimed `[ ]` item with a matching ID (`BZ-xx`). Issues reference these IDs;
the **bazzitify-implementer** cron picks issues oldest-first.

Status legend: `[ ]` todo · `[~]` in progress · `[x]` done
Each item carries: **ID** · priority (P0 highest) · scope tag · acceptance criteria.

## Done (for the planner's diff)
- [x] BZ-01 [module] gaming-packages (Steam, Proton-ge, GameMode) — modules/gaming-packages.sh
- [x] BZ-02 [engine] sysctl tuning — modules/sysctl.sh
- [x] BZ-03 [module] gpu-drivers w/ AMD/NVIDIA detect — modules/gpu-drivers.sh
- [x] BZ-04 [module] kernel-params (nowatchdog, split_lock, bootloader-aware) — modules/kernel-params.sh
- [x] BZ-05 [module] services (gamemode enablement) — modules/services.sh
- [x] BZ-06 [module] filesystems (TRIM, zram, swappiness) — modules/filesystems.sh
- [x] BZ-07 [module] flatpak + Flathub — modules/flatpak.sh
- [x] BZ-08 [module] codecs (VAAPI, MangoHud, vkBasalt, obs-vkcapture) — modules/codecs.sh
- [x] BZ-09 [module] input-peripherals (xone, OpenRazer, libratbag, input-remapper, OpenTabletDriver) — modules/input-peripherals.sh
- [x] BZ-10 [gui] Rust+Slint GUI, COSMIC-Tweaks layout, per-module detail page — src/, ui/app.slint
- [x] BZ-11 [engine] dry-run mode (runner RunOpts + GUI toggle) — src/runner.rs
- [x] BZ-12 [module] display-gpu-control (LACT/CoreCtrl/gamescope) — modules/display-gpu-control.sh
- [x] BZ-13 [module] streaming-containers (Sunshine/distrobox) — modules/streaming-containers.sh

## Backlog

### P1 — engine maturity
- [x] BZ-14 [engine] Applied-state persistence: record applied modules in ~/.local/state/bazzitify/applied.json; GUI shows "applied ✓" on launch; implementer must add tests.
- [ ] BZ-15 [gui] Per-module checkbox selection for batch apply (currently single or select-all); status column already exists in ui/app.slint.
- [ ] BZ-16 [engine] Module dependency ordering: optional `# requires:` header; topological sort before batch apply; test with a fixture pair.
- [ ] BZ-17 [cli] bin/bazzitify gains --json output for scripting/cron consumption.

### P2 — Bazzite parity remainder
- [ ] BZ-18 [module] HDR/VRR helpers: detect compositor (KDE Plasma ≥6 gamescope/HDR paths), install + guide; never force-enable hardware state; document limits honestly.
- [ ] BZ-19 [module] power-profiles: power-profiles-daemon + tuned profiles for gaming vs battery (laptop-aware).
- [ ] BZ-20 [gui] First-run wizard: distro detect → suggested module set → review in dry-run → apply.
- [ ] BZ-21 [docs] README module table auto-check against modules/ dir in CI (gh workflow).

### P3 — reach
- [ ] BZ-22 [distro] openSUSE Tumbleweed (zypper) package maps for all existing modules.
- [ ] BZ-23 [distro] Fedora (dnf) support behind detection guard.
- [ ] BZ-24 [gui] i18n-ready strings (externalize to a map).

## Explicitly out of scope (do not open issues)
- Steam Gaming Mode session (Deck-only), ujust/rpm-ostree machinery (Fedora Atomic),
  KDE cosmetics/themes, secure-boot enrollment, Deck BIOS services.
