#!/bin/bash
# desc: Test fixture module B (requires module-a)
# requires: module-a
set -euo pipefail

MARKER_FILE="/tmp/bazzitify-dep-test-b.marker"

module_apply() {
    echo "Applying module-b"
    # Verify module-a was applied first by checking its marker
    if [ ! -f "/tmp/bazzitify-dep-test-a.marker" ]; then
        echo "ERROR: module-a marker not found! Dependency ordering failed." >&2
        return 1
    fi
    echo "$(date --iso-8601=seconds): module-b applied" > "$MARKER_FILE"
}

module_undo() {
    echo "Undoing module-b"
    rm -f "$MARKER_FILE"
}