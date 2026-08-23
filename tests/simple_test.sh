#!/bin/bash
# Simple test script
set -euo pipefail

source /run/media/rightguy/data/dev/projects/bazzitify/modules/lib/distro.sh
source /run/media/rightguy/data/dev/projects/bazzitify/modules/lib/packages.sh

echo "Test 1: detect_distro"
distro=$(detect_distro)
echo "  distro=$distro"

echo "Test 2: detect_package_manager"
pm=$(detect_package_manager)
echo "  pm=$pm"

echo "Test 3: canonical_distro_id"
echo "  arch: $(canonical_distro_id arch)"
echo "  cachyos: $(canonical_distro_id cachyos)"
echo "  debian: $(canonical_distro_id debian)"
echo "  ubuntu: $(canonical_distro_id ubuntu)"
echo "  opensuse-tumbleweed: $(canonical_distro_id opensuse-tumbleweed)"
echo "  opensuse-leap: $(canonical_distro_id opensuse-leap)"
echo "  fedora: $(canonical_distro_id fedora)"
echo "  nixos: $(canonical_distro_id nixos)"

echo "Test 4: function existence"
type pkg_available && echo "  pkg_available: OK"
type pkg_installed && echo "  pkg_installed: OK"
type pkg_install && echo "  pkg_install: OK"
type pkg_remove && echo "  pkg_remove: OK"

echo "Test 5: resolve_package"
current_distro=$(canonical_distro_id "$(detect_distro)")
echo "  current_distro=$current_distro"
if [[ "$current_distro" != "unknown" ]]; then
    echo "  steam: $(resolve_package steam)"
    echo "  mangohud: $(resolve_package mangohud)"
    echo "  gamemode: $(resolve_package gamemode)"
    echo "  lutris: $(resolve_package lutris)"
    echo "  gamescope: $(resolve_package gamescope)"
fi

echo "Test 6: resolve_package_list"
packages="steam mangohud gamemode"
resolved_list=$(resolve_package_list $packages)
echo "  resolved_list=$resolved_list"

echo "Test 7: resolve_package_for_distro"
for distro in arch cachyos debian ubuntu opensuse fedora; do
    for pkg in steam mangohud gamemode lutris gamescope; do
        resolved=$(resolve_package_for_distro "$distro" "$pkg")
        echo "  $distro/$pkg=$resolved"
    done
done

echo "ALL TESTS PASSED"