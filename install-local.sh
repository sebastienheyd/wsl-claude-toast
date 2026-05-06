#!/usr/bin/env bash
# Local-build installer for development.
#
# Runs `cargo build --release`, then installs the freshly built binary
# under ~/.claude/bin/wsl-claude-toast and refreshes the Claude Code
# hook + personal AppID — same flow as install.sh, but without the
# GitHub release download.
#
# Usage:
#   ./install-local.sh
#
# Optional environment variables:
#   WCT_INSTALL_DIR    Target directory. Default: $HOME/.claude/bin.
#   WCT_NO_HOOK=1      Skip the uninstall/install hook cycle.

set -euo pipefail

readonly BIN_NAME="wsl-claude-toast"

WCT_INSTALL_DIR="${WCT_INSTALL_DIR:-$HOME/.claude/bin}"

err() { printf 'error: %s\n' "$*" >&2; exit 1; }
log() { printf '==> %s\n' "$*"; }

[ -n "${BASH_VERSION:-}" ] || err "this installer requires bash"

case "$(uname -s)" in
    Linux) ;;
    *) err "this installer only supports Linux/WSL (got $(uname -s))" ;;
esac

command -v cargo >/dev/null 2>&1 || err "cargo not found in PATH"
command -v powershell.exe >/dev/null 2>&1 \
    || err "powershell.exe not found in PATH; this tool requires WSL with Windows interop"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

log "Building release binary with cargo..."
cargo build --release

SRC="${SCRIPT_DIR}/target/release/${BIN_NAME}"
[ -x "$SRC" ] || err "build artifact missing: ${SRC}"

mkdir -p "${WCT_INSTALL_DIR}"
TARGET="${WCT_INSTALL_DIR}/${BIN_NAME}"

IS_UPDATE=0
if [ -x "$TARGET" ]; then
    IS_UPDATE=1
    log "Existing installation detected at ${TARGET} — running update."
fi

if [ "$IS_UPDATE" = "1" ] && [ "${WCT_NO_HOOK:-0}" != "1" ]; then
    log "Cleaning previous hook with existing binary..."
    "$TARGET" --uninstall-hook \
        || log "Warning: previous --uninstall-hook failed (continuing)."
fi

cp -f "$SRC" "$TARGET"
chmod +x "$TARGET"
log "Binary installed at ${TARGET}"

if [ "${WCT_NO_HOOK:-0}" = "1" ]; then
    log "WCT_NO_HOOK=1 — skipping --install-hook"
    exit 0
fi

if [ "$IS_UPDATE" = "1" ]; then
    log "Re-registering Claude Code hook and personal AppID with new binary..."
else
    log "Registering Claude Code hook and personal AppID..."
fi
"$TARGET" --install-hook
log "Done."
