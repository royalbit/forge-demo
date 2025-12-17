#!/bin/bash
# Run forge-e2e: E2E validation tool for forge-demo
# Usage: ./run-demo.sh [--all]
#
# Downloads both binaries from royalbit/forge-demo GitHub releases:
#   - forge-demo (raw binary, from forge-demo release tag)
#   - forge-e2e (tar.gz archive, from forge-e2e release tag)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$SCRIPT_DIR/bin"
FORGE_E2E="$BIN_DIR/forge-e2e"
FORGE_DEMO="$BIN_DIR/forge-demo"
REPO="royalbit/forge-demo"

# Detect platform
detect_platform() {
    case "$(uname -s)" in
        Linux*)
            case "$(uname -m)" in
                aarch64|arm64) echo "aarch64-unknown-linux-musl" ;;
                *)             echo "x86_64-unknown-linux-musl" ;;
            esac
            ;;
        Darwin*)
            case "$(uname -m)" in
                arm64) echo "aarch64-apple-darwin" ;;
                *)     echo "x86_64-apple-darwin" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*) echo "x86_64-pc-windows-gnu" ;;
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

# Get latest release tag for a pattern (matches against tag column)
get_latest_release() {
    local pattern="$1"
    gh release list --repo "$REPO" --limit 20 --json tagName --jq '.[].tagName' 2>/dev/null | \
        grep -E "$pattern" | head -1
}

# Download raw binary
download_raw_binary() {
    local name="$1"
    local tag="$2"
    local asset="$3"
    local dest="$4"

    echo "Checking $name ($tag)..."

    local url="https://github.com/$REPO/releases/download/$tag/$asset"

    if curl -fsSL -o "$dest.tmp" "$url" 2>/dev/null; then
        mv "$dest.tmp" "$dest"
        chmod +x "$dest"
        echo "  Downloaded $name"
    else
        rm -f "$dest.tmp"
        if [ -f "$dest" ]; then
            echo "  Using cached $name"
        else
            return 1
        fi
    fi
}

# Download and extract tar.gz
download_archive() {
    local name="$1"
    local tag="$2"
    local archive="$3"
    local dest="$4"

    echo "Checking $name ($tag)..."

    local url="https://github.com/$REPO/releases/download/$tag/$archive"
    local archive_path="$BIN_DIR/$archive"

    if curl -fsSL -o "$archive_path" "$url" 2>/dev/null; then
        if [[ "$archive" == *.tar.gz ]]; then
            tar -xzf "$archive_path" -C "$BIN_DIR"
        elif [[ "$archive" == *.zip ]]; then
            unzip -oq "$archive_path" -d "$BIN_DIR"
        fi
        rm -f "$archive_path"
        chmod +x "$dest" 2>/dev/null || true
        echo "  Downloaded $name"
    else
        rm -f "$archive_path"
        if [ -f "$dest" ]; then
            echo "  Using cached $name"
        else
            return 1
        fi
    fi
}

# Map platform to asset names
# forge-demo uses: forge-demo-<version>-<os>-<arch>
# forge-e2e uses: forge-e2e-<target>.tar.gz
case "$PLATFORM" in
    aarch64-apple-darwin)
        DEMO_ASSET_PATTERN="forge-demo-.*-darwin-arm64"
        E2E_ARCHIVE="forge-e2e-aarch64-apple-darwin.tar.gz"
        ;;
    x86_64-apple-darwin)
        DEMO_ASSET_PATTERN="forge-demo-.*-darwin-x86_64"
        E2E_ARCHIVE="forge-e2e-x86_64-apple-darwin.tar.gz"
        ;;
    x86_64-unknown-linux-musl)
        DEMO_ASSET_PATTERN="forge-demo-.*-linux-x86_64"
        E2E_ARCHIVE="forge-e2e-x86_64-unknown-linux-gnu.tar.gz"
        ;;
    aarch64-unknown-linux-musl)
        DEMO_ASSET_PATTERN="forge-demo-.*-linux-arm64"
        E2E_ARCHIVE="forge-e2e-aarch64-unknown-linux-gnu.tar.gz"
        ;;
    x86_64-pc-windows-gnu)
        DEMO_ASSET_PATTERN="forge-demo-.*-windows-x86_64.exe"
        E2E_ARCHIVE="forge-e2e-x86_64-pc-windows-msvc.zip"
        ;;
esac

# Find latest forge-demo release (v9.x.x tag)
DEMO_TAG=$(gh release list --repo "$REPO" --limit 20 --json tagName --jq '.[].tagName' 2>/dev/null | grep -E "^v9\.[0-9]+\.[0-9]+$" | head -1)
if [ -z "$DEMO_TAG" ]; then
    echo "Error: No forge-demo release found"
    exit 1
fi

# Find actual asset name matching pattern
DEMO_ASSET=$(gh release view "$DEMO_TAG" --repo "$REPO" --json assets --jq ".assets[].name" 2>/dev/null | grep -E "$DEMO_ASSET_PATTERN" | head -1)
if [ -z "$DEMO_ASSET" ]; then
    echo "Error: No forge-demo asset found matching $DEMO_ASSET_PATTERN"
    echo "Release: $DEMO_TAG"
    exit 1
fi

# Download forge-demo (raw binary)
download_raw_binary "forge-demo" "$DEMO_TAG" "$DEMO_ASSET" "$FORGE_DEMO" || {
    echo ""
    echo "Error: Failed to download forge-demo"
    echo "Release: $DEMO_TAG"
    echo "Asset: $DEMO_ASSET"
    exit 1
}

# Find latest forge-e2e release (v2.x.x tags)
E2E_TAG=$(get_latest_release "^v2\.")
if [ -z "$E2E_TAG" ]; then
    E2E_TAG="v2.1.0"  # Fallback
fi

# Download forge-e2e (tar.gz archive)
download_archive "forge-e2e" "$E2E_TAG" "$E2E_ARCHIVE" "$FORGE_E2E" || {
    echo ""
    echo "Error: Failed to download forge-e2e"
    echo "Release: $E2E_TAG"
    echo "Archive: $E2E_ARCHIVE"
    exit 1
}

echo ""
exec "$FORGE_E2E" "$@"
