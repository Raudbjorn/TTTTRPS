#!/bin/bash

# TTTTRPS - AI-Powered TTRPG Assistant (TUI) Build Script
# Pure Rust with ratatui terminal interface

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

print_header() {
    echo -e "\n${PURPLE}╔═══════════════════════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${PURPLE}║                    TTTTRPS Build System (TUI Edition)                         ║${NC}"
    echo -e "${PURPLE}╚═══════════════════════════════════════════════════════════════════════════════╝${NC}"

    # Show git/GitHub status warnings
    check_git_status
}

# ============================================================================
# Git & GitHub Status
# ============================================================================

check_git_status() {
    if ! git rev-parse --git-dir > /dev/null 2>&1; then
        return 0
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

    if [[ "$current_branch" == "main" || "$current_branch" == "master" ]]; then
        return 0
    fi

    local default_branch=""
    if git show-ref --verify --quiet refs/heads/main; then
        default_branch="main"
    elif git show-ref --verify --quiet refs/heads/master; then
        default_branch="master"
    else
        return 0
    fi

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

    local github_repo=""
    github_repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null) || github_repo=""
    if [ -z "$github_repo" ]; then
        return 0
    fi

    local pr_count=0
    pr_count=$(gh pr list --state open --json number 2>/dev/null | jq length 2>/dev/null) || pr_count=0

    if [ "$pr_count" -gt 0 ]; then
        eval "$warnings_array_name+=('🔀 There are $pr_count open pull request(s) in $github_repo')"
    fi

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

# ============================================================================
# Output Helpers
# ============================================================================

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

# ============================================================================
# Dev Build Optimizations
# ============================================================================

# Dev-mode build optimizations: fast compile over fast runtime
# Called for dev/check/test/lint - NOT for build --release
# Idempotent: uses _DEV_OPTS_APPLIED marker to avoid reprocessing on repeated calls
setup_dev_optimizations() {
    if [ "${_DEV_OPTS_APPLIED:-}" = true ]; then
        return 0
    fi
    _DEV_OPTS_APPLIED=true

    print_section "Dev Build Optimizations"

    # Ensure sccache is wrapping rustc
    if command_exists sccache; then
        export RUSTC_WRAPPER=sccache
        sccache --start-server 2>/dev/null || true
        local cache_loc
        cache_loc=$(sccache --show-stats 2>&1 | grep 'Cache location' | sed 's/.*Cache location[[:space:]]*//' | xargs) || cache_loc="unknown"
        print_success "sccache: $cache_loc"
    else
        print_warning "sccache not found - compilation caching disabled"
    fi

    # Strip opt-level from RUSTFLAGS so cargo profiles control it
    # (dev profile = O0 for fastest compile; release profile = O3)
    if [[ "${RUSTFLAGS:-}" == *"opt-level"* ]]; then
        RUSTFLAGS=$(echo "$RUSTFLAGS" | sed 's/-C opt-level=[0-9sz]*//g' | tr -s ' ')
        export RUSTFLAGS
        print_info "Stripped opt-level from RUSTFLAGS (dev=O0 for fast compile)"
    fi

    # Verify mold linker; strip -fuse-ld=mold from RUSTFLAGS if missing
    if command_exists mold; then
        print_success "Linker: mold $(mold --version 2>/dev/null | cut -d' ' -f2)"
    else
        print_warning "mold linker not found - falling back to default linker"
        if [[ "${RUSTFLAGS:-}" == *"fuse-ld=mold"* ]]; then
            RUSTFLAGS=$(echo "$RUSTFLAGS" | sed 's/-C link-arg=-fuse-ld=mold//g' | tr -s ' ')
            export RUSTFLAGS
        fi
        if [[ "${LDFLAGS:-}" == *"fuse-ld=mold"* ]]; then
            LDFLAGS=$(echo "$LDFLAGS" | sed 's/-fuse-ld=mold//g' | tr -s ' ')
            export LDFLAGS
        fi
    fi

    print_info "RUSTC_WRAPPER=${RUSTC_WRAPPER:-unset}"
    print_info "RUSTFLAGS=${RUSTFLAGS:-unset}"
}

# ============================================================================
# Environment Checks
# ============================================================================

check_rust_env() {
    print_section "Checking Rust Environment"

    if ! command_exists cargo; then
        print_error "Rust/Cargo is not installed. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    print_success "Rust installed: $(cargo --version)"

    if command_exists rustup; then
        local toolchain
        toolchain=$(rustup show active-toolchain 2>/dev/null | cut -d' ' -f1) || toolchain="unknown"
        print_success "Active toolchain: $toolchain"
    fi
}

check_optional_deps() {
    print_section "Checking Optional Dependencies"

    # Tesseract OCR (for scanned PDF extraction)
    if command_exists tesseract; then
        local tess_ver
        tess_ver=$(tesseract --version 2>&1 | head -1) || tess_ver="unknown"
        print_success "Tesseract OCR: $tess_ver"
    else
        print_info "Tesseract OCR: not found (scanned PDF extraction unavailable)"
    fi

    # pdfinfo (for PDF page count estimation)
    if command_exists pdfinfo; then
        print_success "pdfinfo: available"
    else
        print_info "pdfinfo: not found (PDF page counting unavailable)"
    fi

    # pdftoppm (for PDF to image conversion, used by OCR fallback)
    if command_exists pdftoppm; then
        print_success "pdftoppm: available"
    else
        print_info "pdftoppm: not found (OCR fallback for PDFs unavailable)"
    fi

    # OpenSSL (required by many Rust crates)
    if pkg-config --exists openssl 2>/dev/null; then
        print_success "OpenSSL: $(pkg-config --modversion openssl 2>/dev/null || echo 'available')"
    else
        print_warning "OpenSSL dev headers not found (may cause build errors)"
    fi
}

# ============================================================================
# Build Commands
# ============================================================================

run_dev() {
    print_section "Starting Development Mode"

    print_info "Running cargo run..."
    cd "$PROJECT_ROOT"

    RUST_LOG="${RUST_LOG:-info}" cargo run
}

run_build() {
    print_section "Building TTTTRPS"
    cd "$PROJECT_ROOT"

    if [ "$RELEASE" = true ]; then
        print_info "Building release binary (optimized)..."
        cargo build --release

        local binary="target/release/ttttrps"
        if [ -f "$binary" ]; then
            local size
            size=$(du -h "$binary" | cut -f1) || size="unknown"
            print_success "Release binary: $binary ($size)"
        else
            print_error "Release binary not found at $binary"
            exit 1
        fi
    else
        print_info "Building debug binary..."
        cargo build

        print_success "Debug binary: target/debug/ttttrps"
    fi
}

# ============================================================================
# Quality Commands
# ============================================================================

run_tests() {
    print_section "Running Tests"
    cd "$PROJECT_ROOT"

    print_info "Running unit tests..."
    cargo test --lib

    if [ "$RUN_INTEGRATION" = true ]; then
        print_info "Running integration tests..."
        cargo test -- --ignored
    else
        print_info "Skipping integration tests (use --integration to run)"
    fi

    if [ "$RUN_ALL_TESTS" = true ]; then
        print_info "Running all tests (including doc tests)..."
        cargo test
    fi

    print_success "Tests passed"
}

run_check() {
    print_section "Running Type Check"
    cd "$PROJECT_ROOT"

    print_info "Running cargo check..."
    cargo check

    print_success "Type check passed"
}

run_lint() {
    print_section "Running Clippy Lints"
    cd "$PROJECT_ROOT"

    print_info "Running clippy..."
    cargo clippy -- -D warnings

    print_success "Linting passed"
}

run_format() {
    print_section "Formatting Code"
    cd "$PROJECT_ROOT"

    print_info "Running cargo fmt..."
    cargo fmt

    print_success "Code formatted"
}

run_format_check() {
    print_section "Checking Code Formatting"
    cd "$PROJECT_ROOT"

    print_info "Running cargo fmt --check..."
    cargo fmt --check

    print_success "Formatting check passed"
}

# ============================================================================
# Utility Commands
# ============================================================================

clean_artifacts() {
    print_section "Cleaning Build Artifacts"
    cd "$PROJECT_ROOT"

    print_info "Running cargo clean..."
    cargo clean 2>/dev/null || true

    # Clean generated output files
    local extra_files=(
        "check_output.txt"
        "cargo_check_output.txt"
    )
    for f in "${extra_files[@]}"; do
        if [ -f "$f" ]; then
            rm -f "$f"
            print_info "Removed $f"
        fi
    done
    # Glob patterns
    rm -f check_output_*.txt 2>/dev/null

    # Verify cleanup
    local remaining_artifacts=()
    if [ -d "target" ]; then
        remaining_artifacts+=("target")
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

    # Build info
    echo -e "\n${BLUE}Build Info:${NC}"
    if [ -f "target/release/ttttrps" ]; then
        local rel_size rel_time
        rel_size=$(du -h "target/release/ttttrps" 2>/dev/null | cut -f1) || rel_size="unknown"
        rel_time=$(stat -c '%y' "target/release/ttttrps" 2>/dev/null | cut -d'.' -f1) || rel_time="unknown"
        echo -e "  Release binary: ${CYAN}$rel_size${NC} (built $rel_time)"
    else
        echo -e "  Release binary: ${YELLOW}not built${NC}"
    fi
    if [ -f "target/debug/ttttrps" ]; then
        local dbg_size dbg_time
        dbg_size=$(du -h "target/debug/ttttrps" 2>/dev/null | cut -f1) || dbg_size="unknown"
        dbg_time=$(stat -c '%y' "target/debug/ttttrps" 2>/dev/null | cut -d'.' -f1) || dbg_time="unknown"
        echo -e "  Debug binary: ${CYAN}$dbg_size${NC} (built $dbg_time)"
    else
        echo -e "  Debug binary: ${YELLOW}not built${NC}"
    fi

    # GitHub status if available
    if command_exists gh; then
        local github_repo=""
        github_repo=$(gh repo view --json nameWithOwner --jq .nameWithOwner 2>/dev/null) || github_repo=""
        if [ -n "$github_repo" ]; then
            echo -e "\n${BLUE}GitHub Status (${CYAN}$github_repo${BLUE}):${NC}"

            local open_prs="[]"
            open_prs=$(gh pr list --state open --json number,title,author 2>/dev/null) || open_prs="[]"
            local pr_count=0
            pr_count=$(echo "$open_prs" | jq length 2>/dev/null) || pr_count=0
            echo -e "  Open pull requests: ${CYAN}$pr_count${NC}"

            if [ "$pr_count" -gt 0 ] && [ "$pr_count" -le 5 ]; then
                echo "$open_prs" | jq -r '.[] | "    • #\(.number): \(.title) (@\(.author.login))"' 2>/dev/null | head -5
            fi

            local open_issues=0
            open_issues=$(gh issue list --state open --json number 2>/dev/null | jq length 2>/dev/null) || open_issues=0
            echo -e "  Open issues: ${CYAN}$open_issues${NC}"

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
    echo "  dev           Run in development mode (cargo run)"
    echo "  build         Build the binary (debug by default)"
    echo ""
    echo -e "${YELLOW}Quality Commands:${NC}"
    echo "  test          Run unit tests (cargo test --lib)"
    echo "  check         Run type check (cargo check)"
    echo "  lint          Run clippy lints"
    echo "  format        Format all code with rustfmt"
    echo "  format-check  Check formatting without modifying"
    echo ""
    echo -e "${YELLOW}Utility Commands:${NC}"
    echo "  status        Show git, build, and GitHub repository status"
    echo "  clean         Remove build artifacts (prompts for remaining)"
    echo "  clean --all   Force remove all artifacts without prompting"
    echo "  setup         Check environment and optional dependencies"
    echo "  help          Show this help message"
    echo ""
    echo -e "${YELLOW}Options:${NC}"
    echo "  --release      Build in release mode (optimized)"
    echo "  --integration  Include integration tests (requires services)"
    echo "  --all-tests    Run all tests including doc tests"
    echo "  --all, --force Remove all remaining artifacts without prompting (clean only)"
    echo ""
    echo -e "${YELLOW}Detected Tools:${NC}"
    echo -e "  Rust/Cargo:  ${CYAN}$(command_exists cargo && cargo --version 2>/dev/null || echo "not found")${NC}"
    echo -e "  sccache:     ${CYAN}$(command_exists sccache && echo "available" || echo "not found")${NC}"
    echo -e "  mold:        ${CYAN}$(command_exists mold && echo "$(mold --version 2>/dev/null)" || echo "not found")${NC}"
    echo -e "  Tesseract:   ${CYAN}$(command_exists tesseract && echo "available" || echo "not found")${NC}"
    echo -e "  GitHub CLI:  ${CYAN}$(command_exists gh && echo "available" || echo "not found")${NC}"
    echo ""
    echo -e "${YELLOW}Examples:${NC}"
    echo -e "  ${CYAN}$0 dev${NC}                    # Run in development mode"
    echo -e "  ${CYAN}$0 build --release${NC}        # Optimized release build"
    echo -e "  ${CYAN}$0 test${NC}                   # Run unit tests"
    echo -e "  ${CYAN}$0 test --integration${NC}     # Run unit + integration tests"
    echo -e "  ${CYAN}$0 lint && $0 test${NC}        # Lint then test"
    echo -e "  ${CYAN}$0 clean --all${NC}            # Force clean all artifacts"
    echo -e "  ${CYAN}$0 status${NC}                 # Check repo & build status"
}

# ============================================================================
# Argument Parsing
# ============================================================================

RELEASE=false
RUN_INTEGRATION=false
RUN_ALL_TESTS=false
FORCE_CLEAN=false
COMMAND="help"

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
        --all-tests)
            RUN_ALL_TESTS=true
            shift
            ;;
        --all|--force)
            FORCE_CLEAN=true
            shift
            ;;
        dev|build|test|check|clean|setup|help|lint|format|format-check|status)
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

# ============================================================================
# Main Execution
# ============================================================================

print_header

case $COMMAND in
    setup)
        check_rust_env
        check_optional_deps
        print_success "Setup check complete! Run './build.sh dev' to start or './build.sh build --release' for a release build."
        ;;
    dev)
        setup_dev_optimizations
        if ! command_exists cargo; then
            check_rust_env
        fi
        run_dev
        ;;
    build)
        if [ "$RELEASE" != true ]; then
            setup_dev_optimizations
        fi
        if ! command_exists cargo; then
            check_rust_env
        fi
        run_build
        ;;
    test)
        setup_dev_optimizations
        run_tests
        ;;
    check)
        setup_dev_optimizations
        run_check
        ;;
    clean)
        clean_artifacts
        ;;
    lint)
        setup_dev_optimizations
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
