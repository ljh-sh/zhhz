#!/usr/bin/env bash
# Build the authoritative opencc reference binary from the same commit as the
# vendored dictionary data (data/UPSTREAM). This gives a parity gate against
# opencc that uses *identical* data, so divergences are real engine bugs rather
# than data-version noise.
#
# Usage:
#   scripts/build-reference-opencc.sh [install-prefix]
#   default prefix: ./.reference-opencc
#
# After it finishes, point the parity harness at it:
#   OPENCC_BIN=./.reference-opencc/bin/opencc \
#   OPENCC_DATA_DIR=./.reference-opencc/share/opencc \
#     cargo run --example parity
#
# Requirements: cmake, a C++ compiler (clang/g++), python3 (for the upstream
# build scripts that generate derived dictionaries).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PREFIX="${1:-$ROOT/.reference-opencc}"

# Read the pinned commit from data/UPSTREAM.
COMMIT="$(awk '/^commit:[[:space:]]/{print $2; exit}' "$ROOT/data/UPSTREAM")"
if [ -z "$COMMIT" ]; then
    echo "error: could not find commit in data/UPSTREAM" >&2
    exit 1
fi
echo ">> building opencc reference @ ${COMMIT}"
echo ">> install prefix: ${PREFIX}"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

TARBALL="$TMP/opencc.tar.gz"
echo ">> fetching source tarball"
URLS=(
    "https://github.com/BYVoid/OpenCC/archive/${COMMIT}.tar.gz"
    "https://p.ljh.sh/https://github.com/BYVoid/OpenCC/archive/${COMMIT}.tar.gz"
)
ok=0
for url in "${URLS[@]}"; do
    if curl -fsSL --max-time 240 -o "$TARBALL" "$url"; then ok=1; break; fi
done
[ "$ok" -eq 1 ] || { echo "error: fetch failed" >&2; exit 1; }

tar -xzf "$TARBALL" -C "$TMP"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'OpenCC-*' | head -1)"

mkdir -p "$SRC/build"
cd "$SRC/build"
echo ">> cmake configure"
cmake .. \
    -DCMAKE_BUILD_TYPE=Release \
    -DENABLE_GTEST=OFF \
    -DCMAKE_INSTALL_PREFIX="$PREFIX" \
    -DCMAKE_INSTALL_RPATH="@loader_path/../lib" \
    -DCMAKE_INSTALL_RPATH_USE_LINK_PATH=ON \
    >/dev/null

echo ">> build (dictionaries + opencc)"
make -j"$(sysctl -n hw.ncpu 2>/dev/null || nproc)" >/dev/null

echo ">> install to ${PREFIX}"
make install >/dev/null

BIN="$PREFIX/bin/opencc"
DATA="$PREFIX/share/opencc"
echo ""
echo ">> done. Reference ready:"
echo "   export OPENCC_BIN='$BIN'"
echo "   export OPENCC_DATA_DIR='$DATA'"
echo ""
echo ">> run parity:"
echo "   cargo run --example parity"