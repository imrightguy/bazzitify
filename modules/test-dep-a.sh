#!/bin/bash
# desc: Test dependency module A (no dependencies)
# requires:
set -euo pipefail

module_apply() {
    echo "Applying test-dep-a"
}

module_undo() {
    echo "Undoing test-dep-a"
}