#!/bin/bash
# desc: Wayland gaming session — gamescope session entry + Wayland gaming env vars (opt-in, reversible)
# long: Creates a gamescope Wayland session entry for login managers and an environment.d snippet
# long: with gaming-optimized Wayland environment variables. Requires gamescope and GPU driver stack.
# depends: display-gpu-control gpu-drivers
set -euo pipefail

MARKER="bazzitify-wayland-gaming"
SESSION_FILE="/usr/share/wayland-sessions/gamescope.desktop"
SESSION_FILE_X11="/usr/share/xsessions/gamescope.desktop"
ENV_FILE="/etc/environment.d/99-bazzitify-gaming.conf"

# Wayland gaming environment variables
ENV_VARS=(
    "SDL_VIDEODRIVER=wayland"
    "MOZ_ENABLE_WAYLAND=1"
    "QT_QPA_PLATFORM=wayland"
    "WINE_D3D_CONFIG=dxvk"
    "DXVK_ASYNC=1"
    "VKD3D_CONFIG=dxr11,multi_queue"
    "RADV_PERFTEST=aco,rt"
)

# Check if we're running with sufficient privileges for system paths
check_root() {
    if [[ $EUID -ne 0 ]]; then
        echo "warning: module requires root for system paths; re-run with sudo or as root" >&2
        return 1
    fi
    return 0
}

# Determine session directory (prefer wayland-sessions, fallback to xsessions)
detect_session_dir() {
    if [[ -d /usr/share/wayland-sessions ]]; then
        echo "/usr/share/wayland-sessions"
    elif [[ -d /usr/share/xsessions ]]; then
        echo "/usr/share/xsessions"
    else
        echo "/usr/share/wayland-sessions"  # default, will be created
    fi
}

# Create the gamescope desktop session file
create_session_file() {
    local session_dir
    session_dir=$(detect_session_dir)
    local target_file="${session_dir}/gamescope.desktop"
    
    echo "  Creating session file: $target_file"
    
    sudo mkdir -p "$session_dir"
    
    # Create session file with marker for idempotency
    cat <<EOF | sudo tee "$target_file" >/dev/null
[Desktop Entry]
Name=Gamescope (Wayland)
Comment=Steam gaming session via Gamescope on Wayland
Exec=gamescope -e -- steam -gamepadui
Type=Application
DesktopNames=gamescope
# ${MARKER}: managed by bazzitify wayland-gaming-session module
EOF

    echo "  Created $target_file"
}

# Create the environment.d snippet with Wayland gaming variables
create_env_file() {
    echo "  Creating environment.d snippet: $ENV_FILE"
    
    sudo mkdir -p "$(dirname "$ENV_FILE")"
    
    # Build the environment file content
    local env_content="# ${MARKER}: managed by bazzitify wayland-gaming-session module
# Wayland gaming environment variables
# Applies to new login sessions; no runtime injection

"
    
    for var in "${ENV_VARS[@]}"; do
        env_content+="${var}
"
    done
    
    echo "$env_content" | sudo tee "$ENV_FILE" >/dev/null
    
    echo "  Created $ENV_FILE"
}

module_apply() {
    echo "Applying Wayland gaming session module..."
    
    check_root || return 1
    
    # Install gamescope if not present (via display-gpu-control dependency)
    # The dependency ensures gamescope is available
    
    create_session_file
    create_env_file
    
    echo "Wayland gaming session module applied."
    echo "  - Session: $(detect_session_dir)/gamescope.desktop"
    echo "  - Environment: $ENV_FILE"
    echo "  Select 'Gamescope (Wayland)' at login to use."
}

module_undo() {
    echo "Removing Wayland gaming session module..."
    
    check_root || return 1
    
    local session_dir
    session_dir=$(detect_session_dir)
    local target_file="${session_dir}/gamescope.desktop"
    
    # Remove session file if it has our marker
    if [[ -f "$target_file" ]] && grep -q "$MARKER" "$target_file"; then
        echo "  Removing session file: $target_file"
        sudo rm -f "$target_file"
    else
        echo "  Session file not managed by bazzitify or already removed: $target_file"
    fi
    
    # Also check X11 fallback location
    if [[ -f "$SESSION_FILE_X11" ]] && grep -q "$MARKER" "$SESSION_FILE_X11"; then
        echo "  Removing X11 session file: $SESSION_FILE_X11"
        sudo rm -f "$SESSION_FILE_X11"
    fi
    
    # Remove environment.d snippet if it has our marker
    if [[ -f "$ENV_FILE" ]] && grep -q "$MARKER" "$ENV_FILE"; then
        echo "  Removing environment.d snippet: $ENV_FILE"
        sudo rm -f "$ENV_FILE"
    else
        echo "  Environment file not managed by bazzitify or already removed: $ENV_FILE"
    fi
    
    echo "Wayland gaming session module removed."
}