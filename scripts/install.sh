#!/bin/sh
# Kannaka Constellation Installer
# Usage: curl -sSf https://install.ninja-portal.com/kannaka | sh
#
# Detects OS and architecture, downloads the kannaka binary from GitHub
# Releases, installs it to ~/.local/bin, and runs `kannaka init`.

set -e

REPO="NickFlach/kannaka-memory"
BINARY_NAME="kannaka"
INSTALL_DIR="${KANNAKA_INSTALL_DIR:-$HOME/.local/bin}"
VERSION="${KANNAKA_VERSION:-latest}"

# --- Platform detection ---

detect_os() {
    case "$(uname -s)" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        CYGWIN*|MINGW*|MSYS*) echo "windows" ;;
        *)
            echo "Error: unsupported operating system: $(uname -s)" >&2
            exit 1
            ;;
    esac
}

detect_arch() {
    case "$(uname -m)" in
        x86_64|amd64)  echo "x86_64" ;;
        aarch64|arm64) echo "aarch64" ;;
        *)
            echo "Error: unsupported architecture: $(uname -m)" >&2
            exit 1
            ;;
    esac
}

OS="$(detect_os)"
ARCH="$(detect_arch)"

# Binary extension
EXT=""
if [ "$OS" = "windows" ]; then
    EXT=".exe"
fi

echo "Kannaka Constellation Installer"
echo "================================"
echo "  Platform: ${OS} ${ARCH}"
echo ""

# --- Resolve download URL ---

if [ "$VERSION" = "latest" ]; then
    DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${BINARY_NAME}-${OS}-${ARCH}${EXT}"
else
    DOWNLOAD_URL="https://github.com/${REPO}/releases/download/v${VERSION}/${BINARY_NAME}-${VERSION}-${OS}-${ARCH}${EXT}"
fi

echo "  Downloading from: ${DOWNLOAD_URL}"

# --- Download ---

TMPDIR="${TMPDIR:-/tmp}"
TMP_FILE="${TMPDIR}/${BINARY_NAME}${EXT}"

if command -v curl > /dev/null 2>&1; then
    curl -fSL --progress-bar -o "${TMP_FILE}" "${DOWNLOAD_URL}"
elif command -v wget > /dev/null 2>&1; then
    wget -q --show-progress -O "${TMP_FILE}" "${DOWNLOAD_URL}"
else
    echo "Error: neither curl nor wget found. Install one and try again." >&2
    exit 1
fi

echo "  Downloaded to: ${TMP_FILE}"

# --- Install ---

mkdir -p "${INSTALL_DIR}"
mv "${TMP_FILE}" "${INSTALL_DIR}/${BINARY_NAME}${EXT}"
chmod +x "${INSTALL_DIR}/${BINARY_NAME}${EXT}"

echo "  Installed to: ${INSTALL_DIR}/${BINARY_NAME}${EXT}"

# --- PATH check ---

case ":${PATH}:" in
    *":${INSTALL_DIR}:"*)
        # Already in PATH
        ;;
    *)
        echo ""
        echo "  NOTE: ${INSTALL_DIR} is not in your PATH."
        echo "  Add it by running:"
        echo ""
        SHELL_NAME="$(basename "${SHELL:-/bin/sh}")"
        case "$SHELL_NAME" in
            zsh)
                echo "    echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
                echo "    source ~/.zshrc"
                ;;
            fish)
                echo "    fish_add_path ~/.local/bin"
                ;;
            *)
                echo "    echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.bashrc"
                echo "    source ~/.bashrc"
                ;;
        esac
        echo ""
        ;;
esac

# --- First-run initialization ---

KANNAKA_BIN="${INSTALL_DIR}/${BINARY_NAME}${EXT}"
CONFIG_FILE="${HOME}/.kannaka/config.toml"

if [ ! -f "${CONFIG_FILE}" ]; then
    echo ""
    echo "  First install detected. Running setup..."
    echo ""
    # kannaka init will be available once ADR-0025 Task 2 is implemented.
    # For now, just verify the binary works.
    if "${KANNAKA_BIN}" status > /dev/null 2>&1 || "${KANNAKA_BIN}" 2>&1 | head -1 | grep -q "Usage"; then
        echo "  Binary verified. Run 'kannaka init' to configure your agent."
    else
        echo "  Warning: binary downloaded but could not execute. Check permissions."
    fi
else
    echo ""
    echo "  Existing config found at ${CONFIG_FILE}"
    echo "  Upgraded kannaka binary in place."
fi

echo ""
echo "  Installation complete."
echo ""
echo "  Next steps:"
echo "    kannaka init          # configure your agent (first time)"
echo "    kannaka status        # check system health"
echo "    kannaka swarm status  # see the constellation"
echo ""
