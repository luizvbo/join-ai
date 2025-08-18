#!/bin/sh
#
# join-ai installer script
#
# This script is designed to be run via curl:
#   sh -c "$(curl -fsSL https://raw.githubusercontent.com/luizvbo/join-ai/main/install.sh)"

set -e

# Define repository and binary name
REPO="luizvbo/join-ai"
BINARY_NAME="join-ai"

# Determine the Operating System and Architecture
get_os_arch() {
    OS_TYPE=$(uname -s | tr '[:upper:]' '[:lower:]')
    MACHINE_ARCH=$(uname -m)

    TARGET=""

    case "$OS_TYPE" in
        linux)
            case "$MACHINE_ARCH" in
                x86_64) TARGET="x86_64-unknown-linux-gnu" ;;
                aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
                *) echo "Error: Unsupported architecture ($MACHINE_ARCH) for Linux."; exit 1 ;;
            esac
            ;;
        darwin)
            case "$MACHINE_ARCH" in
                x86_64) TARGET="x86_64-apple-darwin" ;;
                arm64) TARGET="aarch64-apple-darwin" ;;
                *) echo "Error: Unsupported architecture ($MACHINE_ARCH) for macOS."; exit 1 ;;
            esac
            ;;
        *)
            echo "Error: Unsupported operating system ($OS_TYPE). Only Linux and macOS are supported by this script."
            exit 1
            ;;
    esac
    echo "$TARGET"
}

# Get the target triple
TARGET=$(get_os_arch)
echo "Detected target: $TARGET"

# Get the latest release tag from GitHub API
LATEST_TAG=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')
if [ -z "$LATEST_TAG" ]; then
    echo "Error: Could not determine the latest release version."
    exit 1
fi
echo "Latest version: $LATEST_TAG"

# Construct the download URL
DOWNLOAD_URL="https://github.com/$REPO/releases/download/$LATEST_TAG/$BINARY_NAME-$TARGET"

# Define the installation directory
INSTALL_DIR="/usr/local/bin"

# Download and install the binary
echo "Downloading from: $DOWNLOAD_URL"
# The -L flag for curl is important to follow redirects
curl -L --progress-bar "$DOWNLOAD_URL" -o "$BINARY_NAME"

# Make the binary executable
chmod +x "$BINARY_NAME"

# Move the binary to the installation directory (requires sudo)
echo "Installing to $INSTALL_DIR..."
if [ -w "$INSTALL_DIR" ]; then
    mv "$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    echo "✅ $BINARY_NAME has been installed successfully to $INSTALL_DIR"
    echo "You can now run '$BINARY_NAME --help'"
else
    echo "Attempting to install with sudo..."
    sudo mv "$BINARY_NAME" "$INSTALL_DIR/$BINARY_NAME"
    echo "✅ $BINARY_NAME has been installed successfully to $INSTALL_DIR"
    echo "You can now run '$BINARY_NAME --help'"
fi
