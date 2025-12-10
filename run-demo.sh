#!/bin/bash
# Run forge-e2e: Download latest binaries from GitHub releases and run
# Usage: ./run-demo.sh [--all]
#
# Uses curl -z for conditional GET (only downloads if remote is newer)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$SCRIPT_DIR/bin"
FORGE_E2E="$BIN_DIR/forge-e2e"
FORGE_DEMO="$BIN_DIR/forge-demo"

# Detect platform
detect_platform() {
    case "$(uname -s)" in
        Linux*)
            case "$(uname -m)" in
                aarch64|arm64) echo "aarch64-unknown-linux-gnu" ;;
                *)             echo "x86_64-unknown-linux-gnu" ;;
            esac
            ;;
        Darwin*)
            case "$(uname -m)" in
                arm64) echo "aarch64-apple-darwin" ;;
                *)     echo "x86_64-apple-darwin" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*) echo "x86_64-pc-windows-msvc" ;;
        *) echo "unknown" ;;
    esac
}

PLATFORM=$(detect_platform)

if [ "$PLATFORM" = "unknown" ]; then
    echo "Error: Unsupported platform"
    exit 1
fi

# Create bin directory
mkdir -p "$BIN_DIR"

# Download function with conditional GET
download_binary() {
    local name="$1"
    local url="$2"
    local dest="$3"
    local archive="$4"
    local archive_path="$BIN_DIR/$archive"

    echo "Checking $name..."

    # Use curl -z for conditional GET (only download if newer)
    if curl -fsSL -z "$dest" -o "$archive_path" "$url"; then
        # Check if we got a new file (archive exists and is newer than dest)
        if [ -f "$archive_path" ] && [ "$archive_path" -nt "$dest" ]; then
            echo "  Updating $name..."
            # Extract
            if [[ "$archive" == *.tar.gz ]]; then
                tar -xzf "$archive_path" -C "$BIN_DIR"
            elif [[ "$archive" == *.zip ]]; then
                unzip -oq "$archive_path" -d "$BIN_DIR"
            fi
            rm -f "$archive_path"
            chmod +x "$dest" 2>/dev/null || true
            echo "  Downloaded $name"
        else
            rm -f "$archive_path" 2>/dev/null || true
            echo "  $name is up to date"
        fi
    else
        if [ ! -f "$dest" ]; then
            echo "Error: Failed to download $name"
            return 1
        fi
        echo "  Using cached $name"
    fi
}

# Map platform to archive names
case "$PLATFORM" in
    aarch64-apple-darwin)
        # forge-demo uses simple names (no archive)
        DEMO_ASSET="forge-demo-macos-arm64"
        E2E_ARCHIVE="forge-e2e-aarch64-apple-darwin.tar.gz"
        ;;
    x86_64-apple-darwin)
        DEMO_ASSET="forge-demo-macos-x86_64"
        E2E_ARCHIVE="forge-e2e-x86_64-apple-darwin.tar.gz"
        ;;
    x86_64-unknown-linux-gnu)
        DEMO_ASSET="forge-demo-linux-x86_64"
        E2E_ARCHIVE="forge-e2e-x86_64-unknown-linux-gnu.tar.gz"
        ;;
    aarch64-unknown-linux-gnu)
        DEMO_ASSET="forge-demo-linux-arm64"
        E2E_ARCHIVE="forge-e2e-aarch64-unknown-linux-gnu.tar.gz"
        ;;
    x86_64-pc-windows-msvc)
        DEMO_ASSET="forge-demo-windows.exe"
        E2E_ARCHIVE="forge-e2e-x86_64-pc-windows-msvc.zip"
        ;;
esac

# Download forge-demo from royalbit/forge-demo
echo "Checking forge-demo..."
DEMO_URL="https://github.com/royalbit/forge-demo/releases/latest/download/$DEMO_ASSET"
if curl -fsSL -z "$FORGE_DEMO" -o "$FORGE_DEMO.tmp" "$DEMO_URL"; then
    if [ -f "$FORGE_DEMO.tmp" ] && [ -s "$FORGE_DEMO.tmp" ]; then
        mv "$FORGE_DEMO.tmp" "$FORGE_DEMO"
        chmod +x "$FORGE_DEMO"
        echo "  Downloaded forge-demo"
    else
        rm -f "$FORGE_DEMO.tmp"
        echo "  forge-demo is up to date"
    fi
else
    rm -f "$FORGE_DEMO.tmp"
    if [ ! -f "$FORGE_DEMO" ]; then
        echo "Error: Failed to download forge-demo"
        exit 1
    fi
    echo "  Using cached forge-demo"
fi

# Download forge-e2e from royalbit/forge-demo
E2E_URL="https://github.com/royalbit/forge-demo/releases/latest/download/$E2E_ARCHIVE"
download_binary "forge-e2e" "$E2E_URL" "$FORGE_E2E" "$E2E_ARCHIVE" || {
    echo "Note: forge-e2e not released yet. Building locally..."
    if command -v cargo &> /dev/null; then
        cargo build --release --manifest-path "$SCRIPT_DIR/Cargo.toml"
        cp "$SCRIPT_DIR/target/release/forge-e2e" "$FORGE_E2E"
        chmod +x "$FORGE_E2E"
    else
        echo "Error: cargo not found. Install Rust or wait for forge-e2e release."
        exit 1
    fi
}

echo ""
exec "$FORGE_E2E" "$@"
