#!/bin/bash

# TTRPG Assistant - Rust/Tauri Build Script
# 100% Rust Architecture with Dioxus Frontend

set -e  # Exit on error

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Emojis for better UX
ROCKET="🚀"
CHECK="✅"
CROSS="❌"
WARNING="⚠️"
GEAR="⚙️"
PACKAGE="📦"
TEST="🧪"
BUILD="🔨"

# Project paths
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$PROJECT_ROOT/frontend"
BACKEND_DIR="$PROJECT_ROOT/src-tauri"
DIST_DIR="$PROJECT_ROOT/dist"

print_header() {
    echo -e "\n${PURPLE}╔═══════════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${PURPLE}║              TTRPG Assistant (Sidecar DM) Build System                        ║${NC}"
    echo -e "${PURPLE}╚═══════════════════════════════════════════════════════════════════════════════╝${NC}"
}

print_section() {
    echo -e "\n${CYAN}${GEAR} $1${NC}"
    echo -e "${CYAN}$(printf '%.0s─' {1..80})${NC}"
}

print_success() {
    echo -e "${GREEN}${CHECK} $1${NC}"
}

print_error() {
    echo -e "${RED}${CROSS} $1${NC}" >&2
}

print_warning() {
    echo -e "${YELLOW}${WARNING} $1${NC}"
}

print_info() {
    echo -e "${BLUE}${GEAR} $1${NC}"
}

command_exists() {
    command -v "$1" >/dev/null 2>&1
}

check_rust_env() {
    print_section "Checking Rust Environment"

    if ! command_exists cargo; then
        print_error "Rust/Cargo is not installed. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    print_success "Rust installed: $(cargo --version)"

    # Check wasm target
    if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
        print_warning "WASM target not installed. Installing..."
        rustup target add wasm32-unknown-unknown || { print_error "Failed to install wasm target"; exit 1; }
    fi
    print_success "WASM target installed"

    if ! command_exists dx; then
        print_warning "Dioxus CLI (dx) not found. Installing..."
        cargo install dioxus-cli || { print_error "Failed to install dioxus-cli"; exit 1; }
    fi
    print_success "Dioxus CLI installed: $(dx --version 2>/dev/null || echo 'unknown')"

    # Check for Tauri CLI
    if ! cargo tauri --version >/dev/null 2>&1; then
        print_warning "Tauri CLI not found. Installing..."
        cargo install tauri-cli || { print_error "Failed to install tauri-cli"; exit 1; }
    fi
    print_success "Tauri CLI installed: $(cargo tauri --version 2>/dev/null || echo 'unknown')"
}

check_linux_deps() {
    print_section "Checking Linux Dependencies"

    # Check for required libraries
    local missing_deps=()

    # WebKitGTK check
    if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
        missing_deps+=("webkit2gtk-4.1")
    fi

    # GTK check
    if ! pkg-config --exists gtk+-3.0 2>/dev/null; then
        missing_deps+=("gtk+-3.0")
    fi

    # libappindicator for system tray
    if ! pkg-config --exists appindicator3-0.1 2>/dev/null; then
        print_warning "libappindicator not found - system tray may not work"
    fi

    if [ ${#missing_deps[@]} -gt 0 ]; then
        print_error "Missing dependencies: ${missing_deps[*]}"
        echo -e "${YELLOW}Install with: paru -S webkit2gtk-4.1 gtk3 libappindicator-gtk3${NC}"
        exit 1
    fi

    print_success "All Linux dependencies installed"
}

build_frontend() {
    print_section "Building Frontend (Dioxus WASM)"
    cd "$FRONTEND_DIR"

    print_info "Compiling Dioxus frontend..."

    # Build with dx
    if [ "$RELEASE" = true ]; then
        dx build --release
        FRONTEND_OUTPUT="target/dx/ttrpg-assistant-frontend/release/web/public"
    else
        dx build
        FRONTEND_OUTPUT="target/dx/ttrpg-assistant-frontend/debug/web/public"
    fi

    # Copy artifacts to dist for Tauri
    mkdir -p "$DIST_DIR"
    if [ -d "$FRONTEND_OUTPUT" ]; then
        cp -r "$FRONTEND_OUTPUT"/* "$DIST_DIR/"
        print_success "Frontend built successfully"
        print_info "Output: $DIST_DIR"
    else
        print_error "Frontend output directory not found: $FRONTEND_OUTPUT"
        exit 1
    fi

    cd "$PROJECT_ROOT"
}

build_backend() {
    print_section "Building Backend (Tauri)"
    cd "$BACKEND_DIR"

    print_info "Compiling Tauri backend..."

    if [ "$RELEASE" = true ]; then
        cargo build --release
    else
        cargo build
    fi

    print_success "Backend built successfully"
    cd "$PROJECT_ROOT"
}

build_desktop() {
    print_section "Building Desktop App (Tauri Bundle)"
    cd "$BACKEND_DIR"

    print_info "Creating application bundle..."

    # Build with Tauri
    if [ "$RELEASE" = true ]; then
        cargo tauri build
    else
        cargo tauri build --debug
    fi

    if [ $? -eq 0 ]; then
        print_success "Desktop app built successfully"

        # Find and list the output bundles
        echo -e "\n${GREEN}${PACKAGE} Build artifacts:${NC}"

        if [ -d "target/release/bundle" ]; then
            find target/release/bundle -type f \( -name "*.deb" -o -name "*.rpm" -o -name "*.AppImage" -o -name "*.dmg" -o -name "*.exe" -o -name "*.msi" \) 2>/dev/null | while read -r file; do
                size=$(du -h "$file" | cut -f1)
                echo -e "  ${CYAN}$file${NC} (${size})"
            done
        fi
    else
        print_error "Desktop build failed"
        exit 1
    fi

    cd "$PROJECT_ROOT"
}

run_dev() {
    print_section "Starting Development Server"
    cd "$BACKEND_DIR"

    # Run Tauri dev mode (includes frontend hot-reload)
    cargo tauri dev

    cd "$PROJECT_ROOT"
}

run_tests() {
    print_section "Running Tests"

    print_info "Testing backend..."
    cd "$BACKEND_DIR"
    cargo test

    print_info "Testing frontend..."
    cd "$FRONTEND_DIR"
    cargo test

    cd "$PROJECT_ROOT"
    print_success "All tests passed"
}

run_check() {
    print_section "Running Checks"

    print_info "Checking backend..."
    cd "$BACKEND_DIR"
    cargo check
    cargo clippy -- -D warnings 2>/dev/null || print_warning "Clippy has warnings"

    print_info "Checking frontend..."
    cd "$FRONTEND_DIR"
    cargo check

    cd "$PROJECT_ROOT"
    print_success "Checks completed"
}

clean_artifacts() {
    print_section "Cleaning Build Artifacts"

    cd "$FRONTEND_DIR"
    cargo clean
    rm -rf target/dx

    cd "$BACKEND_DIR"
    cargo clean

    rm -rf "$DIST_DIR"

    cd "$PROJECT_ROOT"
    print_success "Cleaned all build artifacts"
}

show_help() {
    echo -e "${CYAN}Usage:${NC} $0 [command] [options]"
    echo ""
    echo -e "${CYAN}Commands:${NC}"
    echo "  dev         Start development server with hot-reload"
    echo "  build       Build everything (frontend + desktop bundle)"
    echo "  frontend    Build only the frontend"
    echo "  backend     Build only the backend"
    echo "  test        Run all tests"
    echo "  check       Run cargo check and clippy"
    echo "  clean       Remove all build artifacts"
    echo "  help        Show this help message"
    echo ""
    echo -e "${CYAN}Options:${NC}"
    echo "  --release   Build in release mode (optimized)"
    echo ""
    echo -e "${CYAN}Examples:${NC}"
    echo "  $0 dev              # Start dev server"
    echo "  $0 build --release  # Build optimized release"
    echo "  $0 test             # Run all tests"
}

# Parse arguments
RELEASE=false
COMMAND="build"

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            RELEASE=true
            shift
            ;;
        dev|build|frontend|backend|test|check|clean|help)
            COMMAND=$1
            shift
            ;;
        *)
            print_warning "Unknown option: $1"
            show_help
            exit 1
            ;;
    esac
done

# Main execution
print_header

case $COMMAND in
    dev)
        check_rust_env
        [[ "$OSTYPE" == "linux-gnu"* ]] && check_linux_deps
        run_dev
        ;;
    build)
        check_rust_env
        [[ "$OSTYPE" == "linux-gnu"* ]] && check_linux_deps
        build_frontend
        build_desktop
        ;;
    frontend)
        check_rust_env
        build_frontend
        ;;
    backend)
        check_rust_env
        [[ "$OSTYPE" == "linux-gnu"* ]] && check_linux_deps
        build_backend
        ;;
    test)
        run_tests
        ;;
    check)
        run_check
        ;;
    clean)
        clean_artifacts
        ;;
    help|*)
        show_help
        ;;
esac

echo -e "\n${GREEN}${ROCKET} Done!${NC}"
