#!/usr/bin/env bash
#
# launch-claude-debug.sh - Launch Claude Desktop with CDP debugging enabled
#
# Usage:
#   ./launch-claude-debug.sh [port]
#
# Default port is 9222

set -euo pipefail

PORT="${1:-9222}"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${CYAN}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[OK]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Find Claude Desktop binary
find_claude_binary() {
    local candidates=(
        "/opt/Claude/claude"
        "/opt/claude-desktop/claude"
        "/usr/bin/claude"
        "/usr/local/bin/claude"
        "$HOME/.local/bin/claude"
        "/Applications/Claude.app/Contents/MacOS/Claude"
    )
    
    # Check PATH first
    if command -v claude &> /dev/null; then
        echo "claude"
        return 0
    fi
    
    # Check common locations
    for path in "${candidates[@]}"; do
        if [[ -x "$path" ]]; then
            echo "$path"
            return 0
        fi
    done
    
    # Try to find it
    local found
    found=$(find /opt /usr -name "claude" -type f -executable 2>/dev/null | head -n1)
    if [[ -n "$found" ]]; then
        echo "$found"
        return 0
    fi
    
    return 1
}

# Check if port is already in use
check_port() {
    if command -v ss &> /dev/null; then
        ss -tlnp 2>/dev/null | grep -q ":$PORT " && return 1
    elif command -v netstat &> /dev/null; then
        netstat -tlnp 2>/dev/null | grep -q ":$PORT " && return 1
    elif command -v lsof &> /dev/null; then
        lsof -i :"$PORT" &> /dev/null && return 1
    fi
    return 0
}

# Main
main() {
    log_info "Claude Desktop CDP Launcher"
    log_info "Port: $PORT"
    echo
    
    # Check if port is available
    if ! check_port; then
        log_warn "Port $PORT appears to be in use"
        log_info "This might mean Claude Desktop is already running with debugging enabled"
        log_info "Try connecting to ws://127.0.0.1:$PORT"
        exit 0
    fi
    
    # Find Claude binary
    log_info "Looking for Claude Desktop..."
    CLAUDE_BIN=$(find_claude_binary) || {
        log_error "Could not find Claude Desktop binary"
        log_info "Please install Claude Desktop or specify the path manually"
        exit 1
    }
    log_success "Found: $CLAUDE_BIN"
    
    # Launch with debugging
    log_info "Launching Claude Desktop with CDP on port $PORT..."
    echo
    echo -e "${CYAN}───────────────────────────────────────────────────────${NC}"
    echo -e "  CDP WebSocket URL: ${GREEN}ws://127.0.0.1:$PORT${NC}"
    echo -e "  DevTools URL:      ${GREEN}http://127.0.0.1:$PORT${NC}"
    echo -e "${CYAN}───────────────────────────────────────────────────────${NC}"
    echo
    
    # Launch and detach
    exec "$CLAUDE_BIN" --remote-debugging-port="$PORT" "$@"
}

main "$@"
