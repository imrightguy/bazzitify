#!/bin/bash
# desc: Validate README.md Modules table against modules/ directory
# long: Checks that every module in modules/ with a # desc: header appears in the README table,
# long: and that every entry in the README table corresponds to an actual module file.
# long: Used by CI workflow to keep documentation in sync with implementation.

set -euo pipefail

readonly README="README.md"
readonly MODULES_DIR="modules"
readonly SCRIPT_NAME="$(basename "$0")"

# Extract module names from modules/ directory that have a # desc: header
# Output: sorted list of module names (basename without .sh), one per line
extract_fs_modules() {
    local modules=()
    for module_file in "$MODULES_DIR"/*.sh; do
        [[ -f "$module_file" ]] || continue
        # Check if file has a # desc: header
        if grep -q '^# desc:' "$module_file"; then
            local basename_module
            basename_module="$(basename "$module_file" .sh)"
            modules+=("$basename_module")
        fi
    done
    printf '%s\n' "${modules[@]}" | sort
}

# Extract module names from README.md Modules table
# Parses the table between "## Modules" header and the next "## " header
# Output: sorted list of module names, one per line
extract_readme_modules() {
    local in_modules_section=false
    local in_table=false
    local modules=()

    while IFS= read -r line; do
        # Detect start of Modules section (handles emoji in header)
        if [[ "$line" =~ ^##[[:space:]]+.*Modules[[:space:]]*$ ]]; then
            in_modules_section=true
            continue
        fi

        # Detect end of Modules section (next ## header)
        if [[ "$in_modules_section" == true && "$line" =~ ^##[[:space:]] ]]; then
            break
        fi

        # Detect start of table (header row) - allow leading whitespace
        if [[ "$in_modules_section" == true && "$line" =~ ^[[:space:]]*\|[[:space:]]*Module[[:space:]]*\| ]]; then
            in_table=true
            continue
        fi

        # Skip separator row
        if [[ "$in_table" == true && "$line" =~ ^[[:space:]]*\|[[:space:]]*- ]]; then
            continue
        fi

        # End of table (non-table line while in table)
        if [[ "$in_table" == true && ! "$line" =~ ^[[:space:]]*\| ]]; then
            break
        fi

        # Parse table row
        if [[ "$in_table" == true && "$line" =~ ^[[:space:]]*\| ]]; then
            # Split by | and get column 2 (module name)
            local parts=()
            IFS='|' read -ra parts <<< "$line"
            if [[ ${#parts[@]} -ge 3 ]]; then
                local module="${parts[1]}"
                # Trim whitespace and markdown bold
                module="$(echo "$module" | sed -E 's/^[[:space:]]*//; s/[[:space:]]*$//; s/^\*\*//; s/\*\*$//')"
                if [[ -n "$module" && "$module" != "Module" ]]; then
                    modules+=("$module")
                fi
            fi
        fi
    done < "$README"

    printf '%s\n' "${modules[@]}" | sort
}

# Print error with file:line reference for missing modules
report_missing() {
    local missing_modules=("$@")
    for module in "${missing_modules[@]}"; do
        # Find the module file to get line number of # desc:
        local module_file="$MODULES_DIR/$module.sh"
        local line_num=1
        if [[ -f "$module_file" ]]; then
            line_num=$(grep -n '^# desc:' "$module_file" | head -1 | cut -d: -f1)
        fi
        echo "::error file=$module_file,line=$line_num::Missing from README Modules table: $module"
    done
}

# Print error with file:line reference for stale modules
report_stale() {
    local stale_modules=("$@")
    for module in "${stale_modules[@]}"; do
        # Find the line in README where this module appears
        local line_num
        line_num=$(grep -n "^\| \*\*$module\*\* \|" "$README" | head -1 | cut -d: -f1)
        if [[ -z "$line_num" ]]; then
            line_num=1
        fi
        echo "::error file=$README,line=$line_num::Stale entry in README Modules table (module no longer exists): $module"
    done
}

main() {
    echo "=== $SCRIPT_NAME ==="
    echo "Validating $README Modules table against $MODULES_DIR/ directory..."

    # Extract module lists
    local fs_modules readme_modules
    fs_modules=$(extract_fs_modules)
    readme_modules=$(extract_readme_modules)

    local fs_count readme_count
    fs_count=$(echo "$fs_modules" | grep -c '^' || echo 0)
    readme_count=$(echo "$readme_modules" | grep -c '^' || echo 0)
    # Adjust for empty output
    [[ -z "$fs_modules" ]] && fs_count=0
    [[ -z "$readme_modules" ]] && readme_count=0

    echo "Modules in $MODULES_DIR/ (with # desc:): $fs_count"
    echo "$fs_modules" | sed 's/^/  /'
    echo "Modules in README table: $readme_count"
    echo "$readme_modules" | sed 's/^/  /'

    # Compare using comm (requires sorted input)
    local missing stale
    missing=$(comm -13 <(echo "$readme_modules") <(echo "$fs_modules") || true)
    stale=$(comm -23 <(echo "$readme_modules") <(echo "$fs_modules") || true)

    local has_errors=false

    if [[ -n "$missing" ]]; then
        echo ""
        echo "❌ Missing from README Modules table:"
        report_missing $missing
        has_errors=true
    else
        echo ""
        echo "✅ No modules missing from README"
    fi

    if [[ -n "$stale" ]]; then
        echo ""
        echo "❌ Stale entries in README Modules table:"
        report_stale $stale
        has_errors=true
    else
        echo ""
        echo "✅ No stale entries in README"
    fi

    if [[ "$has_errors" == true ]]; then
        echo ""
        echo "❌ README Modules table is out of sync with $MODULES_DIR/ directory"
        echo "Fix the README.md table to match the modules/ directory."
        exit 1
    else
        echo ""
        echo "✅ README Modules table is in sync with $MODULES_DIR/ directory"
        exit 0
    fi
}

main "$@"