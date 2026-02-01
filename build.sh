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

# Project paths
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FRONTEND_DIR="$PROJECT_ROOT/frontend"
BACKEND_DIR="$PROJECT_ROOT/src-tauri"

# Default configuration
: "${LLM_PROXY_PORT:=18787}"
export LLM_PROXY_PORT

print_header() {
    echo -e "\n${PURPLE}╔═══════════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${PURPLE}║              TTRPG Assistant (Sidecar DM) Build System                        ║${NC}"
    echo -e "${PURPLE}╚═══════════════════════════════════════════════════════════════════════════════╝${NC}"

    # Show git/GitHub status warnings
    check_git_status
}

# Git repository status check
check_git_status() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        return 0  # Not a git repo, skip checks
    fi

    local warnings=()

    # Check for uncommitted changes
    local uncommitted=0
    uncommitted=$(git status --porcelain 2>/dev/null | wc -l) || uncommitted=0
    if [ "$uncommitted" -gt 20 ]; then
        warnings+=("🔄 You have $uncommitted uncommitted changes - consider committing or stashing")
    elif [ "$uncommitted" -gt 5 ]; then
        warnings+=("📝 You have $uncommitted uncommitted changes")
    fi

    # Check for unpushed commits and branch divergence
    local current_branch=""
    current_branch=$(git branch --show-current 2>/dev/null) || current_branch=""
    if [ -n "$current_branch" ]; then
        local unpushed=0
        unpushed=$(git rev-list --count '@{u}..HEAD' 2>/dev/null) || unpushed=0
        if [ "$unpushed" -gt 0 ]; then
            warnings+=("📤 You have $unpushed unpushed commits on branch '$current_branch'")
        fi

        # Check if branch is behind main/master
        check_branch_divergence warnings "$current_branch"
    fi

    # Check for GitHub status (if gh CLI is available)
    if command_exists gh; then
        check_github_status warnings
    fi

    # Display warnings if any
    if [ ${#warnings[@]} -gt 0 ]; then
        echo -e "\n${YELLOW}${WARNING} Git Status Notifications:${NC}"
        for warning in "${warnings[@]}"; do
            echo -e "  ${YELLOW}$warning${NC}"
        done
        echo ""
    fi
}

# Check branch divergence from main/master
# Note: Uses eval-based array manipulation to allow multiple functions to append
# warnings to a shared array. While bash 4.3+ supports nameref (local -n), this
# approach maintains compatibility with bash 4.0+ and macOS default bash.
# The array name is always a controlled internal variable ("warnings"), not user input.
check_branch_divergence() {
    # shellcheck disable=SC2178  # Intentionally used for array manipulation via eval
    local warnings_array_name=$1
    local current_branch=$2

    # Skip if we're on main/master
    if [[ "$current_branch" == "main" || "$current_branch" == "master" ]]; then
        return 0
    fi

    # Find the default branch
    local default_branch=""
    if git show-ref --verify --quiet refs/heads/main; then
        default_branch="main"
    elif git show-ref --verify --quiet refs/heads/master; then
        default_branch="master"
    else
        return 0
    fi

    # Check how far behind we are
    local behind=0
    behind=$(git rev-list --count HEAD.."$default_branch" 2>/dev/null) || behind=0

    if [ "$behind" -gt 20 ]; then
        eval "$warnings_array_name+=('📉 Branch '\''$current_branch'\'' is $behind commits behind '\''$default_branch'\'' - consider rebasing')"
    elif [ "$behind" -gt 5 ]; then
        eval "$warnings_array_name+=('📋 Branch '\''$current_branch'\'' is $behind commits behind '\''$default_branch'\''')"
    fi
}

# GitHub CLI integration for PR checks
# See check_branch_divergence() for rationale on eval-based array manipulation
check_github_status() {
    # shellcheck disable=SC2178  # Intentionally used for array manipulation via eval
    local warnings_array_name=$1

    # Check if we're in a GitHub repo
    local github_repo=""
    github_repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null) || github_repo=""
    if [ -z "$github_repo" ]; then
        return 0
    fi

    # Check for open pull requests
    local pr_count=0
    pr_count=$(gh pr list --state open --json number 2>/dev/null | jq length 2>/dev/null) || pr_count=0

    if [ "$pr_count" -gt 0 ]; then
        eval "$warnings_array_name+=('🔀 There are $pr_count open pull request(s) in $github_repo')"
    fi

    # Check for failed CI/CD runs on current branch
    local current_branch=""
    current_branch=$(git branch --show-current 2>/dev/null) || current_branch=""
    if [ -n "$current_branch" ]; then
        local failed_runs=0
        failed_runs=$(gh run list --branch "$current_branch" --status failure --limit 3 --json conclusion 2>/dev/null | jq length 2>/dev/null) || failed_runs=0
        if [ "$failed_runs" -gt 0 ]; then
            eval "$warnings_array_name+=('❌ Recent CI/CD failures on branch '\''$current_branch'\''')"
        fi
    fi
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
        # shellcheck source=/dev/null  # File exists only at runtime
        . /etc/os-release
        echo "$ID"
    elif command_exists lsb_release; then
        lsb_release -si | tr '[:upper:]' '[:lower:]'
    else
        echo "unknown"
    fi
}

install_linux_deps() {
    local distro="unknown"
    distro=$(detect_linux_distro) || distro="unknown"
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
        local current_version="0.0.0"
        # Use POSIX-compatible version extraction (grep -P not available on macOS)
        current_version=$(./tailwindcss --help 2>&1 | head -1 | sed -n 's/.*v\([0-9][0-9.]*\).*/\1/p') || current_version="0.0.0"
        # Fallback to 0.0.0 if extraction failed
        [ -z "$current_version" ] && current_version="0.0.0"
        if [ "$current_version" != "$tailwind_version" ]; then
            print_warning "Tailwind CSS version mismatch (current: $current_version, expected: $tailwind_version)"
            needs_install=true
        fi
    fi

    if [ "$needs_install" = true ]; then
        print_info "Installing Tailwind CSS CLI v$tailwind_version..."

        # Detect platform
        local platform
        local arch
        platform=""
        arch=""

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

    # Check for broken gstreamer dependencies (common on Arch/rolling release)
    check_gstreamer_deps

    # Patch linuxdeploy for Arch Linux compatibility (fixes .relr.dyn strip errors)
    patch_linuxdeploy_strip

    print_info "Creating application bundle..."

    local build_args=()
    if [ "$RELEASE" != true ]; then
        build_args+=(--debug)
    fi

    if cargo tauri build "${build_args[@]}"; then
        print_success "Desktop app built successfully"
    else
        print_error "Desktop build failed"
        exit 1
    fi

    cd "$PROJECT_ROOT"
}

check_gstreamer_deps() {
    # Check for broken gstreamer library dependencies (common on rolling release distros)
    # linuxdeploy's gstreamer plugin will fail if dependencies are missing

    if [ "$OSTYPE" != "linux-gnu" ] && [[ "$OSTYPE" != linux* ]]; then
        return 0
    fi

    local missing_deps=()

    # Check for common broken deps in gstreamer plugins
    if [ -f /usr/lib/gstreamer-1.0/libgstlibav.so ]; then
        local broken
        broken=$(ldd /usr/lib/gstreamer-1.0/libgstlibav.so 2>&1 | grep "not found" | head -5)
        if [ -n "$broken" ]; then
            missing_deps+=("$broken")
        fi
    fi

    if [ ${#missing_deps[@]} -gt 0 ]; then
        print_warning "Broken gstreamer dependencies detected (common on rolling release distros):"
        for dep in "${missing_deps[@]}"; do
            echo -e "  ${YELLOW}$dep${NC}"
        done

        # Try to fix common version mismatches with symlinks
        # Example: libvvenc.so.1.13 missing but libvvenc.so.1.14 exists
        for dep in "${missing_deps[@]}"; do
            local libname
            libname=$(echo "$dep" | awk '{print $1}')
            if [[ "$libname" =~ \.so(\.[0-9]+)+$ ]]; then
                local base_name="${libname%%.so*}.so"
                local newer_lib
                newer_lib=$(ldconfig -p 2>/dev/null | grep "$base_name" | head -1 | awk '{print $NF}')
                if [ -n "$newer_lib" ] && [ -f "$newer_lib" ]; then
                    print_info "Found potential fix: symlink $libname -> $newer_lib"
                    if [ "$AUTO_INSTALL_DEPS" = true ]; then
                        sudo ln -sf "$newer_lib" "/usr/lib/$libname" && \
                            print_success "Created compatibility symlink"
                    else
                        echo -e "${YELLOW}Run: sudo ln -sf $newer_lib /usr/lib/$libname${NC}"
                    fi
                fi
            fi
        done

        print_info "AppImage bundling may fail. Options:"
        print_info "  1. Create symlinks as shown above"
        print_info "  2. Rebuild gstreamer packages: paru -S gst-libav gst-plugins-bad"
        print_info "  3. Skip AppImage: cargo tauri build --bundles deb,rpm"
        echo ""
    fi
}

patch_linuxdeploy_strip() {
    # Fix for modern Linux: linuxdeploy's bundled strip may not recognize .relr.dyn sections
    # This newer ELF relocation format is used by modern distros (Arch, Fedora 38+, etc.)
    # Solution: Extract the linuxdeploy AppImage and replace bundled strip with system's
    #
    # This fix is needed when:
    # - System linuxdeploy is not installed (Tauri downloads AppImage)
    # - The AppImage's bundled binutils are outdated
    #
    # Not needed when:
    # - System linuxdeploy is installed (uses system strip via PATH)

    if [ "$OSTYPE" != "linux-gnu" ] && [[ "$OSTYPE" != linux* ]]; then
        return 0  # Only needed on Linux
    fi

    # If system linuxdeploy is installed, it will use system strip - no patching needed
    if command_exists linuxdeploy; then
        local ld_version=""
        ld_version=$(linuxdeploy --version 2>&1 | head -1) || ld_version="unknown"
        print_info "Using system linuxdeploy ($ld_version)"
        return 0
    fi

    # No system linuxdeploy - Tauri will download an AppImage
    # We need to download and patch it before Tauri runs
    local tauri_cache="${XDG_CACHE_HOME:-$HOME/.cache}/tauri"
    local appimage_url="https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage"
    local appimage_path="$tauri_cache/linuxdeploy-x86_64.AppImage"
    local patch_dir="/tmp/linuxdeploy-patched"
    local patched_strip="$patch_dir/squashfs-root/usr/bin/strip"

    # Check if already patched in this session
    if [ -L "$patched_strip" ] && [ "$(readlink -f "$patched_strip" 2>/dev/null)" = "/usr/bin/strip" ]; then
        print_info "Using patched linuxdeploy (system strip for .relr.dyn compatibility)"
        export LINUXDEPLOY="$patch_dir/squashfs-root/AppRun"
        return 0
    fi

    # Download AppImage if not present
    mkdir -p "$tauri_cache"
    if [ ! -f "$appimage_path" ]; then
        print_info "Downloading linuxdeploy AppImage..."
        if ! curl -fsSL "$appimage_url" -o "$appimage_path"; then
            print_warning "Failed to download linuxdeploy - Tauri will try during build"
            return 0
        fi
        chmod +x "$appimage_path"
    fi

    # Verify it's an actual AppImage (not a wrapper)
    local file_size
    file_size=$(stat -c%s "$appimage_path" 2>/dev/null) || file_size=0
    if [ "$file_size" -lt 1000000 ]; then
        # Less than 1MB - likely a wrapper, not full AppImage
        print_info "linuxdeploy appears to be a wrapper binary, skipping patch"
        return 0
    fi

    print_info "Patching linuxdeploy to use system strip (fixes .relr.dyn section handling)..."

    # Extract the AppImage
    rm -rf "$patch_dir"
    mkdir -p "$patch_dir"
    cd "$patch_dir" || return 1

    # Try extraction with APPIMAGE_EXTRACT_AND_RUN to avoid FUSE issues
    if ! env APPIMAGE_EXTRACT_AND_RUN=1 "$appimage_path" --appimage-extract >/dev/null 2>&1; then
        # Fallback: try extracting with unsquashfs if available
        if command_exists unsquashfs; then
            local offset
            offset=$(grep -aob 'hsqs' "$appimage_path" 2>/dev/null | head -1 | cut -d: -f1)
            if [ -n "$offset" ]; then
                dd if="$appimage_path" bs=1M iflag=skip_bytes,count_bytes skip="$offset" 2>/dev/null | unsquashfs -d squashfs-root -f /dev/stdin >/dev/null 2>&1
            fi
        fi
    fi

    # Check if extraction succeeded
    if [ ! -d "$patch_dir/squashfs-root" ]; then
        print_warning "Could not extract linuxdeploy AppImage - build may fail on AppImage step"
        print_info "Install linuxdeploy via package manager to avoid this: paru -S linuxdeploy"
        cd "$PROJECT_ROOT"
        return 1
    fi

    # Replace bundled strip with system strip (if bundled strip exists)
    if [ -f "$patched_strip" ] || [ -L "$patched_strip" ]; then
        rm -f "$patched_strip"
        ln -s /usr/bin/strip "$patched_strip"
        print_success "Patched linuxdeploy: replaced bundled strip with /usr/bin/strip"
    else
        # Some versions might not bundle strip - create the symlink anyway
        mkdir -p "$(dirname "$patched_strip")"
        ln -s /usr/bin/strip "$patched_strip"
        print_success "Added system strip symlink to linuxdeploy"
    fi

    # Export the patched linuxdeploy path for Tauri to use
    if [ -f "$patch_dir/squashfs-root/AppRun" ]; then
        chmod +x "$patch_dir/squashfs-root/AppRun"
        export LINUXDEPLOY="$patch_dir/squashfs-root/AppRun"
        print_success "Set LINUXDEPLOY=$LINUXDEPLOY"
    fi

    cd "$PROJECT_ROOT" || return 1
    return 0
}

setup_enchant_backend() {
    # Detect available enchant spell-checking backends to avoid libenchant warnings
    # WebKitGTK uses libenchant for spell checking, which may warn about missing backends

    if [ -n "$ENCHANT_BACKEND" ]; then
        return 0  # User already set a preference
    fi

    # Check for available backends in order of preference
    local backends=("hunspell" "aspell" "nuspell" "ispell")
    local enchant_lib_dirs=("/usr/lib/enchant-2" "/usr/lib64/enchant-2" "/usr/local/lib/enchant-2")

    for backend in "${backends[@]}"; do
        for lib_dir in "${enchant_lib_dirs[@]}"; do
            if [ -f "${lib_dir}/lib${backend}.so" ] || [ -f "${lib_dir}/${backend}.so" ]; then
                export ENCHANT_BACKEND="$backend"
                print_info "Set ENCHANT_BACKEND=$backend (spell-check)"
                return 0
            fi
        done
        # Also check if the backend command exists
        if command_exists "$backend"; then
            export ENCHANT_BACKEND="$backend"
            print_info "Set ENCHANT_BACKEND=$backend (spell-check)"
            return 0
        fi
    done

    # No backend found, but that's okay - just means no spell checking
    return 0
}

setup_display_environment() {
    # Detect Wayland and configure display environment for WebKitGTK compatibility
    local is_wayland=false

    if [ "$XDG_SESSION_TYPE" = "wayland" ] || [ -n "$WAYLAND_DISPLAY" ]; then
        is_wayland=true
    fi

    if [ "$is_wayland" = true ]; then
        print_warning "Wayland session detected - configuring display environment for WebKitGTK"

        # Force X11 backend via XWayland to avoid Wayland protocol errors
        if [ -z "$GDK_BACKEND" ]; then
            export GDK_BACKEND=x11
            print_info "Set GDK_BACKEND=x11 (XWayland mode)"
        fi

        # Disable GPU compositing to avoid GBM buffer creation failures
        if [ -z "$WEBKIT_DISABLE_COMPOSITING_MODE" ]; then
            export WEBKIT_DISABLE_COMPOSITING_MODE=1
            print_info "Set WEBKIT_DISABLE_COMPOSITING_MODE=1 (software rendering)"
        fi

        echo -e "${CYAN}  Tip: These workarounds are needed due to WebKitGTK/Wayland compatibility issues${NC}"
    fi

    # Setup spell-checking backend
    setup_enchant_backend
}

run_dev() {
    print_section "Starting Development Server"
    cd "$BACKEND_DIR"

    # Ensure node_modules exists (trunk fails on missing watch ignore paths)
    mkdir -p "$FRONTEND_DIR/node_modules"

    # Setup display environment (Wayland workarounds)
    setup_display_environment

    # Kill any existing instances of the app binary
    if [ "$SEIZE_PORT" = true ]; then
        print_info "Killing old instances of ttrpg-assistant..."
        pkill -f "target/debug/ttrpg-assistant" || true
    fi

    # Check for port conflicts (3030 is trunk dev server, 1420 is Tauri, LLM_PROXY_PORT is proxy)
    for port in 3030 1420 "$LLM_PROXY_PORT"; do
        if ! check_port_usage "$port" "$SEIZE_PORT"; then
            exit 1
        fi
    done

    print_info "Running cargo tauri dev..."
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

    local remaining_artifacts=()

    # Clean frontend
    cd "$FRONTEND_DIR" || { print_error "Failed to cd to frontend directory"; return 1; }

    print_info "Cleaning frontend dist..."
    rm -rf dist

    print_info "Running cargo clean in frontend..."
    cargo clean 2>/dev/null || true

    # Clean backend
    cd "$BACKEND_DIR" || { print_error "Failed to cd to backend directory"; return 1; }

    print_info "Running cargo clean in backend..."
    cargo clean 2>/dev/null || true

    cd "$PROJECT_ROOT" || { print_error "Failed to cd to project root"; return 1; }

    # Trust but verify - check for remaining artifacts
    print_info "Verifying cleanup..."

    if [ -d "$FRONTEND_DIR/target" ]; then
        remaining_artifacts+=("$FRONTEND_DIR/target")
    fi
    if [ -d "$FRONTEND_DIR/dist" ]; then
        remaining_artifacts+=("$FRONTEND_DIR/dist")
    fi
    if [ -d "$BACKEND_DIR/target" ]; then
        remaining_artifacts+=("$BACKEND_DIR/target")
    fi

    if [ ${#remaining_artifacts[@]} -gt 0 ]; then
        print_warning "Found ${#remaining_artifacts[@]} remaining artifact(s) after cargo clean:"
        for artifact in "${remaining_artifacts[@]}"; do
            local size
            size=$(du -sh "$artifact" 2>/dev/null | cut -f1) || size="unknown"
            echo -e "  ${YELLOW}→${NC} $artifact ($size)"
        done

        if [ "$FORCE_CLEAN" = true ]; then
            print_info "Force clean enabled, removing all remaining artifacts..."
            for artifact in "${remaining_artifacts[@]}"; do
                rm -rf "$artifact"
                print_success "Removed: $artifact"
            done
        else
            echo ""
            read -rp "Remove these remaining artifacts? [y/N] " response
            case "$response" in
                [yY][eE][sS]|[yY])
                    for artifact in "${remaining_artifacts[@]}"; do
                        rm -rf "$artifact"
                        print_success "Removed: $artifact"
                    done
                    ;;
                *)
                    print_info "Keeping remaining artifacts. Use './build.sh clean --all' to force removal."
                    ;;
            esac
        fi
    fi

    print_success "Clean completed"
}

run_lint() {
    print_section "Running Clippy Lints"

    print_info "Linting backend..."
    cd "$BACKEND_DIR"
    cargo clippy -- -D warnings

    print_info "Linting frontend..."
    cd "$FRONTEND_DIR"
    cargo clippy -- -D warnings

    cd "$PROJECT_ROOT"
    print_success "Linting passed"
}

run_format() {
    print_section "Formatting Code"

    print_info "Formatting backend..."
    cd "$BACKEND_DIR"
    cargo fmt

    print_info "Formatting frontend..."
    cd "$FRONTEND_DIR"
    cargo fmt

    cd "$PROJECT_ROOT"
    print_success "Code formatted"
}

run_format_check() {
    print_section "Checking Code Formatting"

    print_info "Checking backend formatting..."
    cd "$BACKEND_DIR"
    cargo fmt --check

    print_info "Checking frontend formatting..."
    cd "$FRONTEND_DIR"
    cargo fmt --check

    cd "$PROJECT_ROOT"
    print_success "Formatting check passed"
}

show_status() {
    print_section "Repository Status"

    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        print_warning "Not in a git repository"
        return 0
    fi

    # Basic git status
    echo -e "${BLUE}Git Status:${NC}"
    local current_branch="detached"
    current_branch=$(git branch --show-current 2>/dev/null) || current_branch="detached"
    local uncommitted=0
    uncommitted=$(git status --porcelain 2>/dev/null | wc -l) || uncommitted=0
    local unpushed="unknown"
    unpushed=$(git rev-list --count '@{u}..HEAD' 2>/dev/null) || unpushed="unknown"

    echo -e "  Branch: ${CYAN}$current_branch${NC}"
    echo -e "  Uncommitted changes: ${CYAN}$uncommitted${NC}"
    echo -e "  Unpushed commits: ${CYAN}$unpushed${NC}"

    # GitHub status if available
    if command_exists gh; then
        local github_repo=""
        github_repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null) || github_repo=""
        if [ -n "$github_repo" ]; then
            echo -e "\n${BLUE}GitHub Status (${CYAN}$github_repo${BLUE}):${NC}"

            # Pull requests
            local open_prs="[]"
            open_prs=$(gh pr list --state open --json number,title,author 2>/dev/null) || open_prs="[]"
            local pr_count=0
            pr_count=$(echo "$open_prs" | jq length 2>/dev/null) || pr_count=0
            echo -e "  Open pull requests: ${CYAN}$pr_count${NC}"

            if [ "$pr_count" -gt 0 ] && [ "$pr_count" -le 5 ]; then
                echo "$open_prs" | jq -r '.[] | "    • #\(.number): \(.title) (@\(.author.login))"' 2>/dev/null | head -5
            fi

            # Issues
            local open_issues=0
            open_issues=$(gh issue list --state open --json number 2>/dev/null | jq length 2>/dev/null) || open_issues=0
            echo -e "  Open issues: ${CYAN}$open_issues${NC}"

            # Dependabot alerts
            local vuln_count=0
            vuln_count=$(gh api repos/:owner/:repo/dependabot/alerts --jq '[.[] | select(.state == "open")] | length' 2>/dev/null) || vuln_count=0
            if [ "$vuln_count" -gt 0 ]; then
                echo -e "  ${YELLOW}Security vulnerabilities: $vuln_count${NC}"
            fi
        else
            echo -e "\n${YELLOW}  Not authenticated with GitHub CLI or not a GitHub repo${NC}"
        fi
    else
        echo -e "\n${YELLOW}  GitHub CLI (gh) not available for enhanced status${NC}"
    fi
}

show_help() {
    echo -e "${CYAN}Usage:${NC} $0 [command] [options]"
    echo ""
    echo -e "${YELLOW}Build Commands:${NC}"
    echo "  dev           Start development server with hot-reload"
    echo "  build         Build everything (frontend + desktop bundle)"
    echo "  frontend      Build only the frontend"
    echo "  backend       Build only the backend"
    echo ""
    echo -e "${YELLOW}Quality Commands:${NC}"
    echo "  test          Run all tests"
    echo "  check         Run cargo check"
    echo "  lint          Run clippy lints on all code"
    echo "  format        Format all code with rustfmt"
    echo "  format-check  Check formatting without modifying"
    echo ""
    echo -e "${YELLOW}Utility Commands:${NC}"
    echo "  status        Show git and GitHub repository status"
    echo "  clean         Remove build artifacts (prompts for remaining)"
    echo "  clean --all   Force remove all artifacts without prompting"
    echo "  setup         Install all required dependencies"
    echo "  help          Show this help message"
    echo ""
    echo -e "${YELLOW}Options:${NC}"
    echo "  --release      Build in release mode (optimized)"
    echo "  --integration  Run integration tests (requires Meilisearch)"
    echo "  --auto-deps    Automatically install dependencies without prompting"
    echo "  --seize-port   Automatically kill processes using required ports (3030, 1420)"
    echo "  --all, --force Remove all remaining artifacts without prompting (clean only)"
    echo ""
    echo -e "${YELLOW}Detected Tools:${NC}"
    echo -e "  Rust/Cargo: ${CYAN}$(command_exists cargo && cargo --version 2>/dev/null || echo "not found")${NC}"
    echo -e "  Trunk: ${CYAN}$(command_exists trunk && trunk --version 2>/dev/null || echo "not found")${NC}"
    echo -e "  Tauri CLI: ${CYAN}$(command_exists cargo-tauri && cargo tauri --version 2>/dev/null || echo "not found")${NC}"
    echo -e "  GitHub CLI: ${CYAN}$(command_exists gh && echo "available" || echo "not found")${NC}"
    echo ""
    echo -e "${YELLOW}Examples:${NC}"
    echo -e "  ${CYAN}$0 dev${NC}                    # Start development server"
    echo -e "  ${CYAN}$0 build --release${NC}        # Production build"
    echo -e "  ${CYAN}$0 lint && $0 test${NC}        # Lint then test"
    echo -e "  ${CYAN}$0 clean --all${NC}            # Force clean all artifacts"
    echo -e "  ${CYAN}$0 status${NC}                 # Check repo status"
}

# Check if port is in use and get process info
check_port_usage() {
    local port=$1
    local seize=$2

    # Check if port is in use
    local pid=""
    pid=$(lsof -t -i:"$port" 2>/dev/null | head -1) || pid=""

    if [ -z "$pid" ]; then
        return 0  # Port is free
    fi

    # Get process info
    local proc_name="unknown"
    proc_name=$(ps -p "$pid" -o comm= 2>/dev/null) || proc_name="unknown"
    local proc_cmd="unknown"
    proc_cmd=$(ps -p "$pid" -o args= 2>/dev/null) || proc_cmd="unknown"
    local proc_user="unknown"
    proc_user=$(ps -p "$pid" -o user= 2>/dev/null) || proc_user="unknown"
    local proc_start="unknown"
    proc_start=$(ps -p "$pid" -o lstart= 2>/dev/null) || proc_start="unknown"

    echo -e "\n${YELLOW}${WARNING} Port $port is already in use${NC}"
    echo -e "${BLUE}Process Information:${NC}"
    echo -e "  PID:     ${CYAN}$pid${NC}"
    echo -e "  Name:    ${CYAN}$proc_name${NC}"
    echo -e "  User:    ${CYAN}$proc_user${NC}"
    echo -e "  Command: ${CYAN}$proc_cmd${NC}"
    echo -e "  Started: ${CYAN}$proc_start${NC}"

    if [ "$seize" = true ]; then
        print_warning "Killing process $pid (--seize-port specified)..."
        kill -9 "$pid" 2>/dev/null
        sleep 1
        # Verify it's dead
        if lsof -t -i:"$port" > /dev/null 2>&1; then
            print_error "Failed to kill process on port $port"
            return 1
        fi
        print_success "Port $port is now free"
        return 0
    fi

    # Interactive prompt
    echo -e "\n${YELLOW}Would you like to kill this process? (y/n)${NC}"
    read -r response
    if [[ "$response" =~ ^[Yy]$ ]]; then
        print_info "Killing process $pid..."
        kill -9 "$pid" 2>/dev/null
        sleep 1
        if lsof -t -i:"$port" > /dev/null 2>&1; then
            print_error "Failed to kill process on port $port"
            return 1
        fi
        print_success "Port $port is now free"
        return 0
    else
        print_error "Cannot start dev server while port $port is in use"
        print_info "You can also use --seize-port to automatically kill conflicting processes"
        return 1
    fi
}

# Parse arguments
RELEASE=false
RUN_INTEGRATION=false
AUTO_INSTALL_DEPS=false
SEIZE_PORT=false
FORCE_CLEAN=false
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
        --seize-port)
            SEIZE_PORT=true
            shift
            ;;
        --all|--force)
            FORCE_CLEAN=true
            shift
            ;;
        dev|build|frontend|backend|test|check|clean|setup|help|lint|format|format-check|status)
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
    lint)
        run_lint
        ;;
    format)
        run_format
        ;;
    format-check)
        run_format_check
        ;;
    status)
        show_status
        ;;
    help|*)
        show_help
        ;;
esac

echo -e "\n${GREEN}${ROCKET} Done!${NC}"
