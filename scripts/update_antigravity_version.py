#!/usr/bin/env python3
"""
Fetch latest Antigravity version and update constants.rs.

Usage:
    ./scripts/update_antigravity_version.py [--check] [--dry-run]
    ./scripts/update_antigravity_version.py --version 1.15.8
    ./scripts/update_antigravity_version.py --url 'https://...Antigravity.tar.gz'

Options:
    --check       Only check version, don't update (exit 1 if outdated)
    --dry-run     Show what would be updated without writing
    --version V   Set version directly (skip fetching)
    --url URL     Extract version from download URL

The script can fetch the download page using playwright (JS rendering),
or you can provide the version/URL directly.

Install playwright (optional, for auto-fetch):
    uv pip install playwright
    playwright install chromium

Manual version check:
    1. Open https://antigravity.google/download/linux in browser
    2. Run in console: $('a[href$="Antigravity.tar.gz"]').href
    3. Pass the URL: ./update_antigravity_version.py --url '<url>'
"""

import argparse
import re
import subprocess
import sys
from pathlib import Path

DOWNLOAD_URL = "https://antigravity.google/download/linux"
CONSTANTS_PATH = Path(__file__).parent.parent / "src-tauri/src/oauth/gemini/constants.rs"
VERSION_PATTERN = re.compile(r'antigravity/(\d+\.\d+\.\d+)')
TARBALL_VERSION_PATTERN = re.compile(r'/(\d+\.\d+\.\d+)-\d+/')


def fetch_latest_version() -> str | None:
    """Fetch latest version from Antigravity download page."""
    try:
        from playwright.sync_api import sync_playwright
    except ImportError:
        print("playwright not installed. Install with:", file=sys.stderr)
        print("  uv pip install playwright && playwright install chromium", file=sys.stderr)
        return None

    try:
        with sync_playwright() as p:
            browser = p.chromium.launch(headless=True)
            page = browser.new_page()
            page.goto(DOWNLOAD_URL, wait_until="networkidle")

            # Find the tarball download link
            href = page.evaluate('''() => {
                const link = document.querySelector('a[href$="Antigravity.tar.gz"]');
                return link ? link.href : null;
            }''')

            browser.close()

            if not href:
                print("Could not find tarball download link", file=sys.stderr)
                return None

            # Extract version from URL like:
            # .../stable/1.15.8-5724687216017408/linux-x64/Antigravity.tar.gz
            match = TARBALL_VERSION_PATTERN.search(href)
            if match:
                return match.group(1)

            print(f"Could not parse version from: {href}", file=sys.stderr)
            return None

    except Exception as e:
        print(f"Error fetching version: {e}", file=sys.stderr)
        return None


def get_current_version() -> str | None:
    """Get current version from constants.rs."""
    if not CONSTANTS_PATH.exists():
        print(f"Constants file not found: {CONSTANTS_PATH}", file=sys.stderr)
        return None

    content = CONSTANTS_PATH.read_text()
    match = VERSION_PATTERN.search(content)
    if match:
        return match.group(1)

    print("Could not find USER_AGENT version in constants.rs", file=sys.stderr)
    return None


def update_version(new_version: str, dry_run: bool = False) -> bool:
    """Update version in constants.rs."""
    if not CONSTANTS_PATH.exists():
        return False

    content = CONSTANTS_PATH.read_text()

    # Replace version in USER_AGENT string
    new_content = VERSION_PATTERN.sub(f'antigravity/{new_version}', content)

    if content == new_content:
        print("No changes needed")
        return True

    if dry_run:
        print(f"Would update USER_AGENT to: antigravity/{new_version}")
        return True

    CONSTANTS_PATH.write_text(new_content)
    print(f"Updated USER_AGENT to: antigravity/{new_version}")
    return True


def extract_version_from_url(url: str) -> str | None:
    """Extract version from a download URL."""
    match = TARBALL_VERSION_PATTERN.search(url)
    if match:
        return match.group(1)
    print(f"Could not parse version from URL: {url}", file=sys.stderr)
    return None


def main():
    parser = argparse.ArgumentParser(description="Update Antigravity version")
    parser.add_argument("--check", action="store_true",
                        help="Only check if update needed (exit 1 if outdated)")
    parser.add_argument("--dry-run", action="store_true",
                        help="Show changes without writing")
    parser.add_argument("--version", metavar="V",
                        help="Set version directly (skip fetching)")
    parser.add_argument("--url", metavar="URL",
                        help="Extract version from download URL")
    args = parser.parse_args()

    current = get_current_version()
    if not current:
        sys.exit(1)

    print(f"Current version: {current}")

    # Determine latest version
    if args.version:
        latest = args.version
    elif args.url:
        latest = extract_version_from_url(args.url)
        if not latest:
            sys.exit(1)
    else:
        latest = fetch_latest_version()
        if not latest:
            sys.exit(1)

    print(f"Latest version:  {latest}")

    if current == latest:
        print("Already up to date")
        sys.exit(0)

    if args.check:
        print(f"Update available: {current} -> {latest}")
        sys.exit(1)

    if update_version(latest, dry_run=args.dry_run):
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == "__main__":
    main()
