#!/usr/bin/env bash
# Verifies the Fedora smoke-test job required by BZ-23.
set -euo pipefail

workflow=".github/workflows/build.yml"

fail() {
    printf 'FAIL: %s\n' "$1" >&2
    exit 1
}

[[ -f "$workflow" ]] || fail "missing $workflow"

grep -Eq '^  fedora-check:$' "$workflow" \
    || fail "build workflow is missing the fedora-check job"
grep -Eq '^      image: fedora:[^[:space:]]+$' "$workflow" \
    || fail "fedora-check must use a Fedora container image"
grep -Fq 'bash tests/test_distro.sh' "$workflow" \
    || fail "fedora-check must run the distro/package resolution smoke test"
grep -Fq 'bash bin/bazzitify --list' "$workflow" \
    || fail "fedora-check must run module discovery"
grep -Fq 'bash bin/bazzitify --dry-run gaming-packages' "$workflow" \
    || fail "fedora-check must run a non-mutating module dry-run"

printf 'PASS: Fedora CI smoke-test job is configured\n'
