#!/bin/bash

# TTRPG Assistant - Rust Migration Build Script
# 100% Rust/Tauri Architecture

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

print_header() {
    echo -e "\n${PURPLE}╔═══════════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${PURPLE}║                TTRPG Assistant (Rust Edition) Build System                    ║${NC}"
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

    if ! command_exists dx; then
        print_warning "Dioxus CLI (dx) not found. Installing..."
        cargo install dioxus-cli || { print_error "Failed to install dioxus-cli"; exit 1; }
    fi
    print_success "Dioxus CLI installed: $(dx --version)"

    # Check for Tauri CLI
    # We use cargo-tauri usually, check if we can run it
    if ! cargo tauri --version >/dev/null 2>&1; then
        print_warning "Tauri CLI not found. Installing..."
        cargo install tauri-cli || { print_error "Failed to install tauri-cli"; exit 1; }
    fi
    print_success "Tauri CLI installed: $(cargo tauri --version)"
}

build_frontend() {
    print_section "Building Frontend (Dioxus)"
    cd frontend

    # Check dependencies logic? Dioxus manages this mostly.

    print_info "Building Dioxus App (Release)..."
    dx build --release

    # Copy artifacts to dist for Tauri
    # Dioxus 0.6+ outputs to target/dx/.../public
    mkdir -p dist
    cp -r target/dx/ttrpg-assistant-frontend/release/web/public/* dist/

    if [ $? -eq 0 ]; then
        print_success "Frontend built successfully"
    else
        print_error "Frontend build failed"
        exit 1
    fi
    cd ..
}

build_desktop() {
    print_section "Building Desktop App (Tauri)"

    cd src-tauri

    print_info "Building Tauri App..."
    cargo tauri build

    if [ $? -eq 0 ]; then
        print_success "Desktop app built successfully"
        echo -e "\n${GREEN}${ROCKET} Artifacts located in src-tauri/target/release/bundle/${NC}"
    else
        print_error "Desktop build failed"
        exit 1
    fi
    cd ..
}

run_dev() {
    print_section "Starting Development Server"
    cd src-tauri
    cargo tauri dev
}

# Main execution
print_header
check_rust_env

if [ "$1" == "dev" ]; then
    run_dev
elif [ "$1" == "clean" ]; then
    print_section "Cleaning Build Artifacts"
    cd frontend && cargo clean
    cd ../src-tauri && cargo clean
    print_success "Cleaned"
else
    build_frontend
    build_desktop
fi
