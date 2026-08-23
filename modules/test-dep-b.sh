#!/bin/bash
# desc: Test dependency module B (requires A)
# requires: test-dep-a
set -euo pipefail

module_apply() {
    echo "Applying test-dep-b"
}

module_undo() {
    echo "Undoing test-dep-b"
}