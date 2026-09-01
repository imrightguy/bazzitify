#!/usr/bin/env bash
# Verifies the openSUSE Tumbleweed smoke-test job required by BZ-22.
set -euo pipefail

workflow=".github/workflows/build.yml"
readme="README.md"

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

[[ -f "$workflow" ]] || fail "missing $workflow"
[[ -f "$readme" ]] || fail "missing $readme"

grep -Eq '^  opensuse-check:$' "$workflow" \
    || fail "build workflow is missing the opensuse-check job"
grep -Eq '^      image: opensuse/tumbleweed:[^[:space:]]+$' "$workflow" \
    || fail "opensuse-check must use an openSUSE Tumbleweed container image"
grep -Fq 'bash tests/test_distro.sh' "$workflow" \
    || fail "opensuse-check must run the distro/package resolution smoke test"
grep -Fq 'bash bin/bazzitify --list' "$workflow" \
    || fail "opensuse-check must run module discovery"
grep -Fq 'bash bin/bazzitify --dry-run gaming-packages' "$workflow" \
    || fail "opensuse-check must run a non-mutating module dry-run"

grep -Fqx '| **openSUSE Tumbleweed** | 🟢 supported (zypper) |' "$readme" \
    || fail "README must show openSUSE Tumbleweed as supported in the distro table"

source modules/lib/distro.sh
detect_package_manager() { echo zypper; }
package_removal_command MangoHud gamescope \
    | grep -Fqx 'sudo zypper --non-interactive remove MangoHud gamescope' \
    || fail "zypper removal guidance must be non-interactive and package-specific"

for module in modules/codecs.sh modules/hdr-vrr.sh; do
    grep -Fq 'package_removal_command' "$module" \
        || fail "$module must use package-manager-aware removal guidance"
done

printf 'PASS: openSUSE CI smoke-test job, documentation, and removal guidance are configured\n'
