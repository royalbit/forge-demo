#!/bin/bash
# Run forge-e2e: E2E validation tool for forge-demo
# Usage: ./run-demo.sh [--all]
#
# Downloads forge-e2e from GitHub releases
# Requires forge-demo binary in bin/ (build from main forge repo)

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
            return 1
        fi
        echo "  Using cached $name"
    fi
}

# Map platform to archive names
case "$PLATFORM" in
    aarch64-apple-darwin)
        E2E_ARCHIVE="forge-e2e-aarch64-apple-darwin.tar.gz"
        ;;
    x86_64-apple-darwin)
        E2E_ARCHIVE="forge-e2e-x86_64-apple-darwin.tar.gz"
        ;;
    x86_64-unknown-linux-gnu)
        E2E_ARCHIVE="forge-e2e-x86_64-unknown-linux-gnu.tar.gz"
        ;;
    aarch64-unknown-linux-gnu)
        E2E_ARCHIVE="forge-e2e-aarch64-unknown-linux-gnu.tar.gz"
        ;;
    x86_64-pc-windows-msvc)
        E2E_ARCHIVE="forge-e2e-x86_64-pc-windows-msvc.zip"
        ;;
esac

# Check for forge-demo binary
if [ ! -f "$FORGE_DEMO" ]; then
    echo "forge-demo binary not found at $FORGE_DEMO"
    echo ""
    echo "To build forge-demo from the main forge repo:"
    echo "  cd /path/to/forge"
    echo "  cargo build --release --bin forge-demo"
    echo "  cp target/release/forge-demo $BIN_DIR/"
    echo ""

    # Try to build from parent directory if forge repo exists
    FORGE_REPO="$SCRIPT_DIR/../forge"
    if [ -f "$FORGE_REPO/Cargo.toml" ]; then
        echo "Found forge repo at $FORGE_REPO, building..."
        cargo build --release --bin forge-demo --manifest-path "$FORGE_REPO/Cargo.toml"
        cp "$FORGE_REPO/target/release/forge-demo" "$FORGE_DEMO"
        chmod +x "$FORGE_DEMO"
        echo "Built forge-demo successfully"
    else
        echo "Error: forge repo not found. Please build forge-demo manually."
        exit 1
    fi
fi

# Download forge-e2e from royalbit/forge-demo releases
E2E_URL="https://github.com/royalbit/forge-demo/releases/latest/download/$E2E_ARCHIVE"
download_binary "forge-e2e" "$E2E_URL" "$FORGE_E2E" "$E2E_ARCHIVE" || {
    echo ""
    echo "Error: Failed to download forge-e2e"
    echo "No release found at: $E2E_URL"
    echo ""
    echo "To build and publish releases, run:"
    echo "  make publish"
    echo ""
    exit 1
}

echo ""
exec "$FORGE_E2E" "$@"
