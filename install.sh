#!/bin/bash
# Quick install script for todo CLI
# Usage: curl -sSL https://raw.githubusercontent.com/JhihJian/SUMM-Todo/main/install.sh | bash

set -e

REPO="JhihJian/SUMM-Todo"
VERSION="${TODO_VERSION:-latest}"

# Detect OS and architecture
detect_platform() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"

    case "$OS" in
        Darwin) OS="apple-darwin" ;;
        Linux)  OS="unknown-linux-gnu" ;;
        MINGW*|MSYS*|CYGWIN*) OS="pc-windows-msvc" ;;
        *) echo "Unsupported OS: $OS" >&2; exit 1 ;;
    esac

    case "$ARCH" in
        x86_64|amd64) ARCH="x86_64" ;;
        arm64|aarch64) ARCH="aarch64" ;;
        *) echo "Unsupported architecture: $ARCH" >&2; exit 1 ;;
    esac

    TARGET="${ARCH}-${OS}"
}

# Download and install
install() {
    local tmp_dir=$(mktemp -d)
    trap "rm -rf $tmp_dir" EXIT

    echo "Installing todo CLI..."

    # Determine version
    if [ "$VERSION" = "latest" ]; then
        DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/todo-${TARGET}.tar.gz"
    else
        DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/todo-${TARGET}.tar.gz"
    fi

    echo "Downloading from $DOWNLOAD_URL"
    curl -sSL "$DOWNLOAD_URL" | tar -xzf - -C "$tmp_dir"

    # Install binary
    local install_dir="${TODO_INSTALL_DIR:-/usr/local/bin}"
    if [ ! -w "$install_dir" ]; then
        echo "sudo required for $install_dir"
        sudo mv "$tmp_dir/todo" "$install_dir/todo"
        sudo chmod +x "$install_dir/todo"
    else
        mv "$tmp_dir/todo" "$install_dir/todo"
        chmod +x "$install_dir/todo"
    fi

    echo "✓ todo installed to $install_dir/todo"
    echo "  Run 'todo --help' to get started"
}

detect_platform
install
