#!/bin/bash

# TTRPG Assistant - Rust/Tauri Build Script
# 100% Rust Architecture with Leptos Frontend

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
    if command_exists rustup; then
        if ! rustup target list | grep -q "wasm32-unknown-unknown (installed)"; then
            print_warning "WASM target not installed. Installing..."
            rustup target add wasm32-unknown-unknown || { print_error "Failed to install wasm target"; exit 1; }
        fi
        print_success "WASM target installed via rustup"
    elif [ -f "/usr/lib/rustlib/wasm32-unknown-unknown/lib/libstd-*.rlib" ] || ls /usr/lib/rustlib/wasm32-unknown-unknown/lib/libstd-*.rlib >/dev/null 2>&1; then
         print_success "WASM target installed (system package)"
    else
         print_warning "rustup not found and WASM target checks failed. Assuming WASM is installed via system package manager."
    fi

    if ! command_exists trunk; then
        print_warning "Trunk not found. Installing..."
        cargo install trunk || { print_error "Failed to install trunk"; exit 1; }
    fi
    print_success "Trunk installed: $(trunk --version 2>/dev/null || echo 'unknown')"

    # Check for Tauri CLI
    if ! command_exists cargo-tauri; then
        print_warning "Tauri CLI (cargo-tauri) not found. Installing..."
        cargo install tauri-cli || { print_error "Failed to install tauri-cli"; exit 1; }
    fi
    print_success "Tauri CLI installed: $(cargo tauri --version 2>/dev/null || echo 'unknown')"
}

detect_linux_distro() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        echo "$ID"
    elif command_exists lsb_release; then
        lsb_release -si | tr '[:upper:]' '[:lower:]'
    else
        echo "unknown"
    fi
}

install_linux_deps() {
    local distro=$(detect_linux_distro)
    print_info "Detected Linux distribution: $distro"

    case "$distro" in
        ubuntu|debian|pop|linuxmint|elementary)
            print_info "Installing dependencies for Debian/Ubuntu-based system..."
            sudo apt-get update
            sudo apt-get install -y \
                libwebkit2gtk-4.1-dev \
                libgtk-3-dev \
                libayatana-appindicator3-dev \
                librsvg2-dev \
                patchelf \
                libasound2-dev \
                libssl-dev \
                curl \
                wget \
                file
            ;;
        fedora|rhel|centos|rocky|almalinux)
            print_info "Installing dependencies for Fedora/RHEL-based system..."
            sudo dnf install -y \
                webkit2gtk4.1-devel \
                gtk3-devel \
                libappindicator-gtk3-devel \
                librsvg2-devel \
                patchelf \
                alsa-lib-devel \
                openssl-devel \
                curl \
                wget \
                file
            ;;
        arch|manjaro|endeavouros)
            print_info "Installing dependencies for Arch-based system..."
            sudo pacman -S --needed --noconfirm \
                webkit2gtk-4.1 \
                gtk3 \
                libappindicator-gtk3 \
                librsvg \
                patchelf \
                alsa-lib \
                openssl \
                curl \
                wget \
                file
            ;;
        opensuse*|sles)
            print_info "Installing dependencies for openSUSE/SLES..."
            sudo zypper install -y \
                webkit2gtk3-devel \
                gtk3-devel \
                libappindicator3-devel \
                librsvg-devel \
                patchelf \
                alsa-devel \
                libopenssl-devel \
                curl \
                wget \
                file
            ;;
        *)
            print_warning "Unknown distribution: $distro"
            print_warning "Please manually install: webkit2gtk, gtk3, libappindicator, librsvg, alsa-lib, openssl"
            return 1
            ;;
    esac
}

check_linux_deps() {
    print_section "Checking Linux Dependencies"

    local missing_deps=()
    local needs_install=false

    # Check for webkit2gtk
    if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null && ! pkg-config --exists webkit2gtk-4.0 2>/dev/null; then
        missing_deps+=("webkit2gtk")
        needs_install=true
    fi

    # Check for GTK3
    if ! pkg-config --exists gtk+-3.0 2>/dev/null; then
        missing_deps+=("gtk3")
        needs_install=true
    fi

    # Check for ALSA
    if ! pkg-config --exists alsa 2>/dev/null; then
        missing_deps+=("alsa")
        needs_install=true
    fi

    # Check for OpenSSL
    if ! pkg-config --exists openssl 2>/dev/null; then
        missing_deps+=("openssl")
        needs_install=true
    fi

    if [ "$needs_install" = true ]; then
        print_warning "Missing dependencies: ${missing_deps[*]}"

        if [ "$AUTO_INSTALL_DEPS" = true ]; then
            print_info "Auto-installing dependencies..."
            if install_linux_deps; then
                print_success "Dependencies installed successfully"
            else
                print_error "Failed to install dependencies automatically"
                exit 1
            fi
        else
            echo -e "\n${YELLOW}Would you like to install missing dependencies automatically? (y/n)${NC}"
            read -r response
            if [[ "$response" =~ ^[Yy]$ ]]; then
                if install_linux_deps; then
                    print_success "Dependencies installed successfully"
                else
                    print_error "Failed to install dependencies"
                    exit 1
                fi
            else
                print_warning "Proceeding without installing dependencies. Build might fail."
            fi
        fi
    else
        print_success "Linux dependencies check passed"
    fi
}

install_macos_deps() {
    print_section "Checking macOS Dependencies"

    if ! command_exists brew; then
        print_warning "Homebrew not found. Installing Homebrew..."
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
    fi

    print_info "Installing dependencies via Homebrew..."
    brew install curl wget

    print_success "macOS dependencies installed"
}

check_windows_deps() {
    print_section "Windows Dependencies"

    # Check if running in WSL
    if grep -qi microsoft /proc/version 2>/dev/null; then
        print_info "Running in WSL - using Linux dependency installation"
        check_linux_deps
        return
    fi

    print_info "For Windows native builds, ensure you have:"
    print_info "  - Microsoft Visual Studio C++ Build Tools"
    print_info "  - WebView2 Runtime (usually pre-installed on Windows 10/11)"
    print_info "  - Rust installed via rustup-init.exe"

    if ! command_exists choco; then
        print_warning "Chocolatey not found. Consider installing it for easier dependency management:"
        print_info "https://chocolatey.org/install"
    else
        print_info "You can install build tools via Chocolatey:"
        print_info "  choco install visualstudio2022buildtools visualstudio2022-workload-vctools"
    fi
}

install_frontend_tools() {
    print_section "Installing Frontend Build Tools"

    # Install Trunk if missing
    if ! command_exists trunk; then
        print_info "Installing Trunk..."
        cargo install trunk --locked || { print_error "Failed to install trunk"; return 1; }
        print_success "Trunk installed"
    else
        print_success "Trunk already installed"
    fi

    # Install/Update Tailwind CSS CLI
    cd "$FRONTEND_DIR"

    local tailwind_version="4.1.18"
    local needs_install=false

    if [ ! -f "tailwindcss" ]; then
        needs_install=true
    else
        local current_version=$(./tailwindcss --help 2>&1 | head -1 | grep -oP 'v\K[0-9.]+' || echo "0.0.0")
        if [ "$current_version" != "$tailwind_version" ]; then
            print_warning "Tailwind CSS version mismatch (current: $current_version, expected: $tailwind_version)"
            needs_install=true
        fi
    fi

    if [ "$needs_install" = true ]; then
        print_info "Installing Tailwind CSS CLI v$tailwind_version..."

        # Detect platform
        local platform=""
        local arch=""

        case "$OSTYPE" in
            linux*)
                platform="linux"
                ;;
            darwin*)
                platform="macos"
                ;;
            msys*|cygwin*|win32)
                platform="windows"
                ;;
        esac

        # Detect architecture
        case "$(uname -m)" in
            x86_64|amd64)
                arch="x64"
                ;;
            aarch64|arm64)
                arch="arm64"
                ;;
            armv7*)
                arch="armv7"
                ;;
        esac

        if [ -n "$platform" ] && [ -n "$arch" ]; then
            local binary_name="tailwindcss-${platform}-${arch}"
            [ "$platform" = "windows" ] && binary_name="${binary_name}.exe"

            local download_url="https://github.com/tailwindlabs/tailwindcss/releases/download/v${tailwind_version}/${binary_name}"

            print_info "Downloading from: $download_url"

            if curl -sL "$download_url" -o tailwindcss.tmp; then
                chmod +x tailwindcss.tmp
                mv tailwindcss.tmp tailwindcss
                print_success "Tailwind CSS CLI v$tailwind_version installed"
            else
                print_error "Failed to download Tailwind CSS CLI"
                rm -f tailwindcss.tmp
                return 1
            fi
        else
            print_error "Unsupported platform: $OSTYPE $(uname -m)"
            return 1
        fi
    else
        print_success "Tailwind CSS CLI already installed (v$tailwind_version)"
    fi

    cd "$PROJECT_ROOT"
}

build_frontend() {
    print_section "Building Frontend (Leptos WASM)"
    cd "$FRONTEND_DIR"

    # Ensure node_modules exists (trunk fails on missing watch ignore paths)
    mkdir -p node_modules

    print_info "Compiling Leptos frontend with Trunk..."

    if [ "$RELEASE" = true ]; then
        trunk build --release
    else
        trunk build
    fi

    if [ -d "dist" ]; then
        print_success "Frontend built successfully in frontend/dist"
    else
        print_error "Frontend build failed or dist directory not found"
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

    if [ "$RELEASE" = true ]; then
        cargo tauri build
    else
        cargo tauri build --debug
    fi

    if [ $? -eq 0 ]; then
        print_success "Desktop app built successfully"
    else
        print_error "Desktop build failed"
        exit 1
    fi

    cd "$PROJECT_ROOT"
}

run_dev() {
    print_section "Starting Development Server"
    cd "$BACKEND_DIR"

    # Ensure node_modules exists (trunk fails on missing watch ignore paths)
    mkdir -p "$FRONTEND_DIR/node_modules"

    print_info "Running cargo tauri dev..."

    # Check and clean up ports
    # Check and clean up ports
    for port in 3030 1420; do
        if lsof -i :$port > /dev/null 2>&1; then
            print_warning "Port $port is in use. Attempting to cleanup..."
            lsof -t -i:$port | xargs kill -9 > /dev/null 2>&1 || true
            sleep 1
        fi
    done

    cargo tauri dev

    cd "$PROJECT_ROOT"
}

run_tests() {
    print_section "Running Tests"

    print_info "Testing backend (lib)..."
    cd "$BACKEND_DIR"
    cargo test --lib

    print_info "Testing backend (integration, requires services)..."
    if [ "$RUN_INTEGRATION" = true ]; then
        cargo test -- --ignored
    else
        print_warning "Skipping integration tests (use --integration to run)"
    fi

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

    print_info "Checking frontend..."
    cd "$FRONTEND_DIR"
    cargo check

    cd "$PROJECT_ROOT"
    print_success "Checks completed"
}

clean_artifacts() {
    print_section "Cleaning Build Artifacts"

    cd "$FRONTEND_DIR"
    rm -rf dist
    cargo clean

    cd "$BACKEND_DIR"
    cargo clean

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
    echo "  check       Run cargo check"
    echo "  clean       Remove all build artifacts"
    echo "  setup       Install all required dependencies"
    echo "  help        Show this help message"
    echo ""
    echo -e "${CYAN}Options:${NC}"
    echo "  --release      Build in release mode (optimized)"
    echo "  --integration  Run integration tests (requires Meilisearch)"
    echo "  --auto-deps    Automatically install dependencies without prompting"
}

# Parse arguments
RELEASE=false
RUN_INTEGRATION=false
AUTO_INSTALL_DEPS=false
COMMAND="build"

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            RELEASE=true
            shift
            ;;
        --integration)
            RUN_INTEGRATION=true
            shift
            ;;
        --auto-deps)
            AUTO_INSTALL_DEPS=true
            shift
            ;;
        dev|build|frontend|backend|test|check|clean|setup|help)
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
    setup)
        check_rust_env
        case "$OSTYPE" in
            linux*)
                check_linux_deps
                ;;
            darwin*)
                install_macos_deps
                ;;
            msys*|cygwin*|win32)
                check_windows_deps
                ;;
        esac
        install_frontend_tools
        print_success "Setup complete! You can now run './build.sh dev' or './build.sh build'"
        ;;
    dev)
        if command_exists cargo; then
             :
        else
             check_rust_env
        fi
        case "$OSTYPE" in
            linux*)
                check_linux_deps
                ;;
            darwin*)
                # macOS doesn't need special system deps for Tauri
                :
                ;;
            msys*|cygwin*|win32)
                check_windows_deps
                ;;
        esac
        install_frontend_tools
        run_dev
        ;;
    build)
        check_rust_env
        case "$OSTYPE" in
            linux*)
                check_linux_deps
                ;;
            darwin*)
                :
                ;;
            msys*|cygwin*|win32)
                check_windows_deps
                ;;
        esac
        install_frontend_tools
        build_frontend
        build_desktop
        ;;
    frontend)
        check_rust_env
        install_frontend_tools
        build_frontend
        ;;
    backend)
        check_rust_env
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
