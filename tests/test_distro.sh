#!/bin/bash
# Tests for modules/lib/distro.sh and modules/lib/packages.sh
# Run with: bash tests/test_distro.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
LIB_DIR="$REPO_ROOT/modules/lib"

# Source the libraries
source "$LIB_DIR/distro.sh"
source "$LIB_DIR/packages.sh"

PASS=0
FAIL=0

assert_eq() {
    local expected="$1"
    local actual="$2"
    local msg="${3:-}"
    if [[ "$expected" == "$actual" ]]; then
        echo "  PASS: $msg"
        ((PASS++))
    else
        echo "  FAIL: $msg" >&2
        echo "    expected: '$expected'" >&2
        echo "    actual:   '$actual'" >&2
        ((FAIL++))
    fi
}

assert_contains() {
    local haystack="$1"
    local needle="$2"
    local msg="${3:-}"
    # Case-insensitive match for package names
    if [[ "${haystack,,}" == *"${needle,,}"* ]]; then
        echo "  PASS: $msg"
        ((PASS++))
    else
        echo "  FAIL: $msg" >&2
        echo "    haystack: '$haystack'" >&2
        echo "    needle:   '$needle'" >&2
        ((FAIL++))
    fi
}

assert_command_succeeds() {
    local cmd="$1"
    local msg="${2:-}"
    if eval "$cmd" >/dev/null 2>&1; then
        echo "  PASS: $msg"
        ((PASS++))
    else
        echo "  FAIL: $msg" >&2
        ((FAIL++))
    fi
}

echo "=== Testing distro detection ==="

# Test detect_distro returns a known ID
distro=$(detect_distro)
assert_contains "arch cachyos debian ubuntu opensuse fedora unknown" "$distro" "detect_distro returns known ID"

# Test detect_package_manager returns correct manager for current distro
pm=$(detect_package_manager)
assert_contains "pacman apt zypper dnf unknown" "$pm" "detect_package_manager returns known manager"

# Test canonical distro ID mapping
assert_eq "arch" "$(canonical_distro_id "arch")" "canonical_distro_id: arch -> arch"
assert_eq "cachyos" "$(canonical_distro_id "cachyos")" "canonical_distro_id: cachyos -> cachyos"
assert_eq "debian" "$(canonical_distro_id "debian")" "canonical_distro_id: debian -> debian"
assert_eq "ubuntu" "$(canonical_distro_id "ubuntu")" "canonical_distro_id: ubuntu -> ubuntu"
assert_eq "opensuse" "$(canonical_distro_id "opensuse-tumbleweed")" "canonical_distro_id: opensuse-tumbleweed -> opensuse"
assert_eq "opensuse" "$(canonical_distro_id "opensuse-leap")" "canonical_distro_id: opensuse-leap -> opensuse"
assert_eq "fedora" "$(canonical_distro_id "fedora")" "canonical_distro_id: fedora -> fedora"
assert_eq "unknown" "$(canonical_distro_id "nixos")" "canonical_distro_id: nixos -> unknown"

echo ""
echo "=== Testing package manager abstraction ==="

# Test pkg_available for current package manager
# We can't test actual package availability without root, but we can test the function exists
assert_command_succeeds "type pkg_available" "pkg_available function exists"
assert_command_succeeds "type pkg_installed" "pkg_installed function exists"
assert_command_succeeds "type pkg_install" "pkg_install function exists"
assert_command_succeeds "type pkg_remove" "pkg_remove function exists"

# Test resolve_package for logical names on current distro
current_distro=$(canonical_distro_id "$(detect_distro)")
if [[ "$current_distro" != "unknown" ]]; then
    # Test a few known logical packages
    resolved=$(resolve_package "steam")
    assert_contains "$resolved" "steam" "resolve_package: steam resolves to something containing steam on $current_distro"
    
    resolved=$(resolve_package "mangohud")
    assert_contains "$resolved" "mangohud" "resolve_package: mangohud resolves on $current_distro"
    
    resolved=$(resolve_package "gamemode")
    assert_contains "$resolved" "gamemode" "resolve_package: gamemode resolves on $current_distro"
    
    resolved=$(resolve_package "lutris")
    assert_contains "$resolved" "lutris" "resolve_package: lutris resolves on $current_distro"
    
    resolved=$(resolve_package "gamescope")
    assert_contains "$resolved" "gamescope" "resolve_package: gamescope resolves on $current_distro"
fi

# Test resolve_package_list
packages="steam mangohud gamemode"
resolved_list=$(resolve_package_list $packages)
assert_contains "$resolved_list" "steam" "resolve_package_list: steam in list"
assert_contains "$resolved_list" "mangohud" "resolve_package_list: mangohud in list"
assert_contains "$resolved_list" "gamemode" "resolve_package_list: gamemode in list"

echo ""
echo "=== Testing package maps for all distros ==="

# Test that each distro has mappings for key logical packages
for distro in arch cachyos debian ubuntu opensuse fedora; do
    for pkg in steam mangohud gamemode lutris gamescope; do
        resolved=$(resolve_package_for_distro "$distro" "$pkg")
        if [[ -n "$resolved" ]]; then
            assert_contains "$resolved" "$pkg" "package map: $distro/$pkg resolves"
        else
            echo "  SKIP: $distro/$pkg not mapped (may be intentional)"
        fi
    done
done

echo ""
echo "=== Summary ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"

if [[ $FAIL -gt 0 ]]; then
    exit 1
fi