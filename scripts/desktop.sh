#!/usr/bin/env bash
set -euo pipefail

# Nix graphics libraries need an EGL driver adapter on non-NixOS Linux.
# Keep Wayland native; do not disable WebKit compositing or GPU rendering.
cheki_graphics=()
if [[ "$(uname -s)" == Linux ]]; then
  cheki_graphics=(nixGLIntel)
  if [[ -n "${WAYLAND_DISPLAY:-}" && -z "${GDK_BACKEND:-}" ]]; then
    export GDK_BACKEND=wayland
  fi
fi

case "${1:-dev}" in
  dev) exec "${cheki_graphics[@]}" npm run tauri dev ;;
  release) exec "${cheki_graphics[@]}" "${CARGO_TARGET_DIR:-.cache/cargo-target}/release/kanzencheki" ;;
  *) echo 'Usage: desktop.sh [dev|release]' >&2; exit 2 ;;
esac
