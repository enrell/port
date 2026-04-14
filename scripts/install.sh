#!/usr/bin/env sh
#
# install.sh - Install port binary from GitHub releases
# Usage: curl -LsSf https://raw.githubusercontent.com/enrell/port/main/scripts/install.sh | sh
#
set -e

ARCH="$(uname -m)"
REPO="enrell/port"

# Detect asset name based on architecture
case "$ARCH" in
    x86_64)
        ASSET="port-x86_64-unknown-linux-gnu.tar.gz"
        ;;
    aarch64|arm64)
        ASSET="port-aarch64-unknown-linux-gnu.tar.gz"
        ;;
    *)
        echo "Unsupported architecture: $ARCH" >&2
        exit 1
        ;;
esac

TMPDIR=$(mktemp -d)
trap 'rm -rf "$TMPDIR"' EXIT INT TERM

# Download the latest release asset
DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/$ASSET"
echo "Downloading $ASSET..."
curl -sL "$DOWNLOAD_URL" -o "$TMPDIR/port.tar.gz"

# Extract to temp dir
tar -xzf "$TMPDIR/port.tar.gz" -C "$TMPDIR"

# Detect extracted binary name
EXTRACTED_BIN=$(ls "$TMPDIR" | grep -E '^port-(x86_64|aarch64)$' | head -n1)
if [ -z "$EXTRACTED_BIN" ]; then
    echo "Failed to find extracted binary" >&2
    exit 1
fi

# Install to ~/.local/bin
mkdir -p "$HOME/.local/bin"
install -Dm755 "$TMPDIR/$EXTRACTED_BIN" "$HOME/.local/bin/port"

echo "Installed port to ~/.local/bin/port"
echo "Make sure ~/.local/bin is in your PATH."