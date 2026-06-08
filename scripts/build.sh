#!/usr/bin/env bash
set -euo pipefail

# Cross-platform build script for ReticulumChat
# Usage: ./scripts/build.sh [target]
# If no target is specified, builds for the current platform

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
TARGET_DIR="${PROJECT_ROOT}/target/release"
BINARY_NAME="reticulumchat"

echo "=== ReticulumChat Cross-Platform Build ==="
echo ""

# Determine target
detect_target() {
    local os arch
    os=$(uname -s)
    arch=$(uname -m)
    
    case "$os" in
        Linux)
            case "$arch" in
                x86_64) echo "x86_64-unknown-linux-gnu" ;;
                aarch64) echo "aarch64-unknown-linux-gnu" ;;
                armv7l) echo "armv7-unknown-linux-gnueabihf" ;;
                *) echo "unknown" ;;
            esac
            ;;
        Darwin)
            case "$arch" in
                x86_64) echo "x86_64-apple-darwin" ;;
                arm64) echo "aarch64-apple-darwin" ;;
                *) echo "unknown" ;;
            esac
            ;;
        MINGW*|MSYS*|CYGWIN*)
            echo "x86_64-pc-windows-msvc"
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

TARGET="${1:-$(detect_target)}"

if [ "$TARGET" = "unknown" ]; then
    echo "Error: Could not detect target platform"
    exit 1
fi

echo "Building for target: $TARGET"
echo ""

# Check if target is installed
if ! rustup target list --installed | grep -q "$TARGET"; then
    echo "Target $TARGET not installed. Installing..."
    rustup target add "$TARGET"
fi

# Install cross if needed for cross-compilation
if [ "$TARGET" != "$(detect_target)" ]; then
    if ! command -v cross &> /dev/null; then
        echo "Installing cross for cross-compilation..."
        cargo install cross --git https://github.com/cross-rs/cross
    fi
    BUILDER="cross"
else
    BUILDER="cargo"
fi

# Build
echo "Building release binary..."
cd "$PROJECT_ROOT"
$BUILDER build --release --target "$TARGET"

# Package
OUTPUT_DIR="${PROJECT_ROOT}/dist"
mkdir -p "$OUTPUT_DIR"

if [[ "$TARGET" == *windows* ]]; then
    EXT=".exe"
else
    EXT=""
fi

BINARY_PATH="${TARGET_DIR}/${TARGET}/release/${BINARY_NAME}${EXT}"
PACKAGE_NAME="${BINARY_NAME}-v$(grep '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')-${TARGET}"

if [ -f "$BINARY_PATH" ]; then
    if [[ "$TARGET" == *windows* ]]; then
        cp "$BINARY_PATH" "${OUTPUT_DIR}/${PACKAGE_NAME}.exe"
        echo ""
        echo "Built: ${OUTPUT_DIR}/${PACKAGE_NAME}.exe"
    else
        cp "$BINARY_PATH" "${OUTPUT_DIR}/${BINARY_NAME}"
        tar -czf "${OUTPUT_DIR}/${PACKAGE_NAME}.tar.gz" -C "$OUTPUT_DIR" "$BINARY_NAME"
        rm "${OUTPUT_DIR}/${BINARY_NAME}"
        echo ""
        echo "Built: ${OUTPUT_DIR}/${PACKAGE_NAME}.tar.gz"
    fi
else
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

echo ""
echo "Build complete!"
