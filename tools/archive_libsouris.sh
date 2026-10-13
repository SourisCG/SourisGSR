#!/usr/bin/env bash
# Archive libsouris_gsr.a from a core meson build for the Rust SDK.
# Usage: ./tools/archive_libsouris.sh [build-dir] [out-dir]
# Example:
#   ./tools/archive_libsouris.sh /tmp/sgsr-build /tmp/sgsr-native
#   SOURIS_GSR_LIB_DIR=/tmp/sgsr-native cargo test --features ffi -p souris-gsr-sdk
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_DIR="${1:-/tmp/sgsr-build}"
OUT_DIR="${2:-/tmp/sgsr-native}"

if [ ! -f "$ROOT/core/meson.build" ]; then
  echo "run from repo root, core/meson.build not found" >&2
  exit 1
fi

if [ ! -d "$BUILD_DIR" ]; then
  echo "== meson setup $BUILD_DIR (core, no capabilities/systemd) =="
  meson setup "$BUILD_DIR" "$ROOT/core" -Dcapabilities=false -Dsystemd=false
fi
echo "== meson compile $BUILD_DIR =="
meson compile -C "$BUILD_DIR"

mkdir -p "$OUT_DIR"
# Prefer the meson-built static lib; fall back to archiving objects.
if ls "$BUILD_DIR"/libsouris_gsr.a >/dev/null 2>&1; then
  cp "$BUILD_DIR"/libsouris_gsr.a "$OUT_DIR"/
  echo "copied $BUILD_DIR/libsouris_gsr.a -> $OUT_DIR/"
else
  echo "libsouris_gsr.a not found, archiving objects manually" >&2
  OBJ=$(find "$BUILD_DIR" -name 'souris_gsr.c.o' | head -1)
  if [ -z "$OBJ" ]; then
    echo "no souris_gsr objects found in $BUILD_DIR" >&2
    exit 1
  fi
  LIBDIR=$(dirname "$OBJ")/../..
  find "$BUILD_DIR" -name '*.o' ! -name 'main.c.o' -print0 | xargs -0 ar rcs "$OUT_DIR/libsouris_gsr.a"
  echo "archived $OUT_DIR/libsouris_gsr.a (libdir hint $LIBDIR)"
fi
ls -lh "$OUT_DIR"/libsouris_gsr.a
