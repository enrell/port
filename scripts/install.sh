#!/usr/bin/env sh
#
# install.sh - Install port binary from GitHub releases
# Usage: curl -LsSf https://raw.githubusercontent.com/enrell/port/main/scripts/install.sh | sh
#
set -e

ARCH="$(uname -m)"
REPO="enrell/port"
INSTALL_DIR="${HOME}/.local/bin"
INSTALL_PATH="${INSTALL_DIR}/port"

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "Missing required command: $1" >&2
        exit 1
    fi
}

need_cmd curl
need_cmd tar
need_cmd install

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

LATEST_URL="$(curl -fsSL -o /dev/null -w '%{url_effective}' "https://github.com/$REPO/releases/latest")"
LATEST_TAG="${LATEST_URL##*/}"
LATEST_VERSION="${LATEST_TAG#v}"

if [ -z "$LATEST_TAG" ] || [ "$LATEST_TAG" = "latest" ]; then
    echo "Failed to resolve the latest release version" >&2
    exit 1
fi

INSTALLED_VERSION=""
if [ -x "$INSTALL_PATH" ]; then
    INSTALLED_VERSION="$("$INSTALL_PATH" --version 2>/dev/null | awk '{print $2}')"
fi

if [ -n "$INSTALLED_VERSION" ] && [ "$INSTALLED_VERSION" = "$LATEST_VERSION" ]; then
    echo "port ${INSTALLED_VERSION} is already installed at ${INSTALL_PATH}"
    echo "No update available. Skipping download."
    exit 0
fi

DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$ASSET"
if [ -n "$INSTALLED_VERSION" ]; then
    echo "Updating port from ${INSTALLED_VERSION} to ${LATEST_VERSION}..."
else
    echo "Installing port ${LATEST_VERSION}..."
fi
echo "Downloading $ASSET..."
curl -fsSL "$DOWNLOAD_URL" -o "$TMPDIR/port.tar.gz"

# Extract to temp dir
tar -xzf "$TMPDIR/port.tar.gz" -C "$TMPDIR"

# Detect extracted binary name
EXTRACTED_BIN=$(ls "$TMPDIR" | grep -E '^port-(x86_64|aarch64)$' | head -n1)
if [ -z "$EXTRACTED_BIN" ]; then
    echo "Failed to find extracted binary" >&2
    exit 1
fi

# Install to ~/.local/bin
mkdir -p "$INSTALL_DIR"
install -Dm755 "$TMPDIR/$EXTRACTED_BIN" "$INSTALL_PATH"

echo "Installed port ${LATEST_VERSION} to ${INSTALL_PATH}"
echo "Make sure ${INSTALL_DIR} is in your PATH."
