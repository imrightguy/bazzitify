#!/bin/bash
# desc: Shared library for distro detection and package manager abstraction
# long: Provides canonical distro IDs, package manager detection, and
# long: package management abstraction (install, check, remove) that works
# long: across Arch/pacman, Debian/apt, openSUSE/zypper, and Fedora/dnf.
# long: Unknown distros fall back gracefully with clear warnings.

set -euo pipefail

# Canonical distro IDs we support
# arch, cachyos, debian, ubuntu, opensuse, fedora
# Unknown distros return "unknown"

# Detect the running distro from /etc/os-release
# Returns a raw distro ID (e.g., arch, cachyos, debian, ubuntu, opensuse-tumbleweed, fedora)
detect_distro() {
    if [[ ! -f /etc/os-release ]]; then
        echo "unknown"
        return 0
    fi

    # shellcheck disable=SC1091
    source /etc/os-release
    echo "${ID:-unknown}"
}

# Map raw distro ID to canonical distro ID
# Supported: arch, cachyos, debian, ubuntu, opensuse, fedora
# Returns "unknown" for unsupported distros
canonical_distro_id() {
    local raw_id="${1:-}"
    case "$raw_id" in
        arch|cachyos)
            echo "$raw_id"
            ;;
        debian|ubuntu)
            echo "$raw_id"
            ;;
        opensuse*|suse*)
            echo "opensuse"
            ;;
        fedora|rhel|centos|rocky|alma)
            echo "fedora"
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

# Get the canonical distro ID for the current system
get_distro() {
    local raw
    raw=$(detect_distro)
    canonical_distro_id "$raw"
}

# Detect the package manager for the current distro
# Returns: pacman, apt, zypper, dnf, or unknown
detect_package_manager() {
    local distro
    distro=$(get_distro)
    case "$distro" in
        arch|cachyos)
            echo "pacman"
            ;;
        debian|ubuntu)
            echo "apt"
            ;;
        opensuse)
            echo "zypper"
            ;;
        fedora)
            echo "dnf"
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

# Check if a package is available in repositories
# Usage: pkg_available <package_name>
pkg_available() {
    local pkg="${1:-}"
    [[ -z "$pkg" ]] && return 1

    local pm
    pm=$(detect_package_manager)
    case "$pm" in
        pacman)
            pacman -Si "$pkg" >/dev/null 2>&1
            ;;
        apt)
            apt-cache show "$pkg" >/dev/null 2>&1
            ;;
        zypper)
            zypper se --match-exact "$pkg" >/dev/null 2>&1
            ;;
        dnf)
            dnf list available "$pkg" >/dev/null 2>&1
            ;;
        *)
            echo "warn: unknown package manager, cannot check availability of $pkg" >&2
            return 1
            ;;
    esac
}

# Check if a package is installed
# Usage: pkg_installed <package_name>
pkg_installed() {
    local pkg="${1:-}"
    [[ -z "$pkg" ]] && return 1

    local pm
    pm=$(detect_package_manager)
    case "$pm" in
        pacman)
            pacman -Qi "$pkg" >/dev/null 2>&1
            ;;
        apt)
            dpkg -s "$pkg" >/dev/null 2>&1
            ;;
        zypper)
            rpm -q "$pkg" >/dev/null 2>&1
            ;;
        dnf)
            rpm -q "$pkg" >/dev/null 2>&1
            ;;
        *)
            echo "warn: unknown package manager, cannot check if $pkg is installed" >&2
            return 1
            ;;
    esac
}

# Install packages
# Usage: pkg_install <package_name>...
# Returns 0 on success, non-zero on failure
pkg_install() {
    local pkgs=("$@")
    [[ ${#pkgs[@]} -eq 0 ]] && return 0

    local pm
    pm=$(detect_package_manager)
    case "$pm" in
        pacman)
            sudo pacman -S --needed --noconfirm "${pkgs[@]}"
            ;;
        apt)
            sudo apt-get update && sudo apt-get install -y "${pkgs[@]}"
            ;;
        zypper)
            sudo zypper --non-interactive install "${pkgs[@]}"
            ;;
        dnf)
            sudo dnf install -y "${pkgs[@]}"
            ;;
        *)
            echo "error: unknown package manager, cannot install packages: ${pkgs[*]}" >&2
            return 1
            ;;
    esac
}

# Remove packages
# Usage: pkg_remove <package_name>...
# Returns 0 on success, non-zero on failure
pkg_remove() {
    local pkgs=("$@")
    [[ ${#pkgs[@]} -eq 0 ]] && return 0

    local pm
    pm=$(detect_package_manager)
    case "$pm" in
        pacman)
            sudo pacman -Rns --noconfirm "${pkgs[@]}"
            ;;
        apt)
            sudo apt-get remove -y "${pkgs[@]}"
            ;;
        zypper)
            sudo zypper --non-interactive remove "${pkgs[@]}"
            ;;
        dnf)
            sudo dnf remove -y "${pkgs[@]}"
            ;;
        *)
            echo "error: unknown package manager, cannot remove packages: ${pkgs[*]}" >&2
            return 1
            ;;
    esac
}

# Print the conservative, package-manager-specific command for a user to
# remove packages manually. Modules intentionally do not remove packages on
# undo because they cannot distinguish packages installed by bazzitify from
# packages the user installed independently.
# Usage: package_removal_command <package>...
package_removal_command() {
    local pkgs=("$@")
    [[ ${#pkgs[@]} -eq 0 ]] && return 0

    case "$(detect_package_manager)" in
        pacman)
            printf 'sudo pacman -Rns %s\n' "${pkgs[*]}"
            ;;
        apt)
            printf 'sudo apt-get remove %s\n' "${pkgs[*]}"
            ;;
        zypper)
            printf 'sudo zypper --non-interactive remove %s\n' "${pkgs[*]}"
            ;;
        dnf)
            printf 'sudo dnf remove %s\n' "${pkgs[*]}"
            ;;
        *)
            printf 'Remove manually with your package manager: %s\n' "${pkgs[*]}"
            ;;
    esac
}

# Get the package manager name for display
get_package_manager_name() {
    detect_package_manager
}

# Warn if distro is unknown
warn_if_unknown_distro() {
    local distro
    distro=$(get_distro)
    if [[ "$distro" == "unknown" ]]; then
        local raw
        raw=$(detect_distro)
        echo "warning: running on unsupported distro '$raw' — package management will not work" >&2
        return 1
    fi
    return 0
}