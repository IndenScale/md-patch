#!/bin/bash
set -e

# mdp (md-patch) installer script
# Usage: curl -fsSL https://raw.githubusercontent.com/IndenScale/md-patch/main/install.sh | sh

REPO="IndenScale/md-patch"
BINARY_NAME="mdp"

# Detect OS and architecture
detect_platform() {
    local _os=""
    local _arch=""
    local _suffix=""

    case "$(uname -s)" in
        Linux*)     _os="unknown-linux-gnu";;
        Darwin*)    _os="apple-darwin";;
        CYGWIN*|MINGW*|MSYS*) _os="pc-windows-msvc"; _suffix=".exe";;
        *)          echo "Unsupported OS: $(uname -s)"; exit 1;;
    esac

    case "$(uname -m)" in
        x86_64)     _arch="x86_64";;
        amd64)      _arch="x86_64";;
        arm64)      _arch="aarch64";;
        aarch64)    _arch="aarch64";;
        *)          echo "Unsupported architecture: $(uname -m)"; exit 1;;
    esac

    echo "${_arch}-${_os}${_suffix}"
}

# Get latest release version
get_latest_version() {
    curl -s "https://api.github.com/repos/${REPO}/releases/latest" | grep '"tag_name":' | sed -E 's/.*"v?([^"]+)".*/\1/'
}

# Download and install
main() {
    echo "Installing ${BINARY_NAME}..."

    # Detect platform
    PLATFORM=$(detect_platform)
    echo "Detected platform: ${PLATFORM}"

    # Get latest version
    VERSION=$(get_latest_version)
    if [ -z "$VERSION" ]; then
        echo "Error: Could not determine latest version"
        exit 1
    fi
    echo "Latest version: ${VERSION}"

    # Determine download URL and extraction method
    if echo "$PLATFORM" | grep -q "windows"; then
        ARCHIVE_NAME="${BINARY_NAME}-${VERSION}-${PLATFORM%.exe}.zip"
        IS_ZIP=true
    else
        ARCHIVE_NAME="${BINARY_NAME}-${VERSION}-${PLATFORM}.tar.gz"
        IS_ZIP=false
    fi

    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ARCHIVE_NAME}"
    
    # Create temp directory
    TEMP_DIR=$(mktemp -d)
    trap "rm -rf ${TEMP_DIR}" EXIT

    # Download
    echo "Downloading from: ${DOWNLOAD_URL}"
    if ! curl -fsSL "$DOWNLOAD_URL" -o "${TEMP_DIR}/${ARCHIVE_NAME}"; then
        echo "Error: Failed to download ${ARCHIVE_NAME}"
        echo "Your platform may not have a pre-built binary."
        echo "Consider building from source with: cargo install md-patch"
        exit 1
    fi

    # Extract
    cd "$TEMP_DIR"
    if [ "$IS_ZIP" = true ]; then
        unzip -q "$ARCHIVE_NAME"
    else
        tar xzf "$ARCHIVE_NAME"
    fi

    # Find binary
    EXTRACTED_DIR="${BINARY_NAME}-${VERSION}-${PLATFORM%.exe}"
    BINARY_PATH="${EXTRACTED_DIR}/${BINARY_NAME}"
    if echo "$PLATFORM" | grep -q "windows"; then
        BINARY_PATH="${BINARY_PATH}.exe"
    fi

    # Install
    INSTALL_DIR="${INSTALL_DIR:-${HOME}/.local/bin}"
    mkdir -p "$INSTALL_DIR"

    echo "Installing to: ${INSTALL_DIR}/${BINARY_NAME}"
    cp "$BINARY_PATH" "${INSTALL_DIR}/${BINARY_NAME}"
    chmod +x "${INSTALL_DIR}/${BINARY_NAME}"

    # Check if install directory is in PATH
    if ! echo "$PATH" | grep -q "${INSTALL_DIR}"; then
        echo ""
        echo "⚠️  Warning: ${INSTALL_DIR} is not in your PATH"
        echo "Add the following to your shell profile:"
        echo "  export PATH=\"\$PATH:${INSTALL_DIR}\""
    fi

    echo ""
    echo "✅ Successfully installed ${BINARY_NAME} v${VERSION}"
    echo ""
    echo "Run '${BINARY_NAME} --help' to get started"
}

main "$@"
