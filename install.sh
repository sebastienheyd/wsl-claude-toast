#!/usr/bin/env bash
# wsl-claude-toast installer.
#
# Downloads the latest pre-built binary for your CPU architecture, places it
# under ~/.claude/bin/wsl-claude-toast, and registers the Claude Code hook
# plus the personal AppID via --install-hook.
#
# Usage (one-liner):
#   wget -qO- https://raw.githubusercontent.com/sebastienheyd/wsl-claude-toast/main/install.sh | bash
#   curl -fsSL https://raw.githubusercontent.com/sebastienheyd/wsl-claude-toast/main/install.sh | bash
#
# Optional environment variables:
#   WCT_VERSION        Pin a specific tag (e.g. v0.1.0). Default: latest.
#   WCT_INSTALL_DIR    Target directory. Default: $HOME/.claude/bin.
#   WCT_NO_HOOK=1      Skip --install-hook (binary is only downloaded).

set -euo pipefail

readonly REPO="sebastienheyd/wsl-claude-toast"
readonly BIN_NAME="wsl-claude-toast"

WCT_VERSION="${WCT_VERSION:-latest}"
WCT_INSTALL_DIR="${WCT_INSTALL_DIR:-$HOME/.claude/bin}"

err() { printf 'error: %s\n' "$*" >&2; exit 1; }
log() { printf '==> %s\n' "$*"; }

[ -n "${BASH_VERSION:-}" ] || err "this installer requires bash"

case "$(uname -s)" in
    Linux) ;;
    *) err "this installer only supports Linux/WSL (got $(uname -s))" ;;
esac

command -v powershell.exe >/dev/null 2>&1 \
    || err "powershell.exe not found in PATH; this tool requires WSL with Windows interop"

if command -v curl >/dev/null 2>&1; then
    DL="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
    DL="wget -qO-"
else
    err "neither curl nor wget is available"
fi

case "$(uname -m)" in
    x86_64|amd64) ASSET="${BIN_NAME}-linux-x86_64" ;;
    aarch64|arm64) ASSET="${BIN_NAME}-linux-aarch64" ;;
    *) err "unsupported architecture: $(uname -m)" ;;
esac

if [ "$WCT_VERSION" = "latest" ]; then
    URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
    URL="https://github.com/${REPO}/releases/download/${WCT_VERSION}/${ASSET}"
fi

log "Architecture detected : $(uname -m) → ${ASSET}"
log "Version              : ${WCT_VERSION}"
log "Install dir          : ${WCT_INSTALL_DIR}"
log "Source URL           : ${URL}"

mkdir -p "${WCT_INSTALL_DIR}"
TARGET="${WCT_INSTALL_DIR}/${BIN_NAME}"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

log "Downloading binary..."
$DL "$URL" > "$TMP"

if [ ! -s "$TMP" ]; then
    err "downloaded file is empty (release asset missing for ${ASSET}?)"
fi

mv "$TMP" "$TARGET"
chmod +x "$TARGET"
log "Binary installed at ${TARGET}"

if [ "${WCT_NO_HOOK:-0}" = "1" ]; then
    log "WCT_NO_HOOK=1 — skipping --install-hook"
    exit 0
fi

log "Registering Claude Code hook and personal AppID..."
"$TARGET" --install-hook
log "Done."
