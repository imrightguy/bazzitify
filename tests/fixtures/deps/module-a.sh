#!/bin/bash
# desc: Test fixture module A (no dependencies)
# requires:
set -euo pipefail

MARKER_FILE="/tmp/bazzitify-dep-test-a.marker"

module_apply() {
    echo "Applying module-a"
    echo "$(date --iso-8601=seconds): module-a applied" > "$MARKER_FILE"
}

module_undo() {
    echo "Undoing module-a"
    rm -f "$MARKER_FILE"
}