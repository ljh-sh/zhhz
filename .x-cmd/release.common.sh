#!/usr/bin/env sh

set -e

init_common() {
    SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

    if [ -n "$PROJECT_DIR" ]; then
        PROJECT_DIR="$(cd "$PROJECT_DIR" && pwd)"
    else
        PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
    fi

    if [ -n "$OUT_DIR" ]; then
        OUT_DIR="$(cd "$OUT_DIR" && pwd)"
    else
        OUT_DIR="$PROJECT_DIR/.release-artifacts"
    fi

    BIN_NAME="${BIN_NAME:-roff}"
}

get_targets() {
    echo "x86_64-unknown-linux-musl:linux:x86_64"
    echo "aarch64-unknown-linux-musl:linux:aarch64"
    echo "x86_64-pc-windows-gnu:windows:x86_64"
    echo "x86_64-apple-darwin:darwin:x86_64"
    echo "aarch64-apple-darwin:darwin:aarch64"
}

get_current_target() {
    rustc -vV | sed -n 's|host: ||p'
}

build_native() {
    local target
    target="$(get_current_target)"
    rm -rf "$OUT_DIR"
    mkdir -p "$OUT_DIR"
    do_build "$target"
    write_build_info
}

build_all_targets() {
    rm -rf "$OUT_DIR"
    mkdir -p "$OUT_DIR"

    while IFS=: read -r target os arch; do
        if command -v cross >/dev/null 2>&1; then
            do_cross_build "$target" "$os" "$arch" || true
        else
            echo "  cross not installed; run 'cargo install cross' to build all targets"
            break
        fi
    done <<EOF
$(get_targets)
EOF

    write_build_info

    echo ""
    echo "Done! Artifacts in: $OUT_DIR"
    echo ""
    ls -la "$OUT_DIR"/*.tar.xz 2>/dev/null || true
}

do_build() {
    local target="$1"
    local bin_name="${BIN_NAME:-roff}"
    local sysroot
    sysroot="$(rustc --print sysroot)"

    echo "Building for $target..."

    cd "$PROJECT_DIR"

    SOURCE_DATE_EPOCH=1700000000 \
    TZ=UTC \
    LC_ALL=C \
    RUSTFLAGS="--remap-path-prefix=$PROJECT_DIR=. --remap-path-prefix=$sysroot=/rust --remap-path-prefix=$HOME/.cargo=/cargo ${RUSTFLAGS:-}" \
    cargo build --release --target "$target"

    local src="target/$target/release/$bin_name"
    if [ ! -f "$src" ]; then
        echo "  Warning: $src not found, skipping"
        return 0
    fi

    mkdir -p "$OUT_DIR/bin"
    cp "$src" "$OUT_DIR/bin/"
    chmod +x "$OUT_DIR/bin/$bin_name"
    strip "$OUT_DIR/bin/$bin_name" 2>/dev/null || true

    local size
    size=$(stat -f%z "$OUT_DIR/bin/$bin_name" 2>/dev/null || stat -c%s "$OUT_DIR/bin/$bin_name" 2>/dev/null || echo "unknown")
    echo "  -> $OUT_DIR/bin/$bin_name ($size bytes)"
}

do_cross_build() {
    local target="$1"
    local os="$2"
    local arch="$3"
    local bin_name="${BIN_NAME:-roff}"
    local pkg_name="roff-${os}-${arch}"
    local pkg_dir="$OUT_DIR/$pkg_name"
    local sysroot
    sysroot="$(rustc --print sysroot)"

    echo "Cross-building for $target..."

    cd "$PROJECT_DIR"

    SOURCE_DATE_EPOCH=1700000000 \
    TZ=UTC \
    LC_ALL=C \
    RUSTFLAGS="--remap-path-prefix=$PROJECT_DIR=. --remap-path-prefix=$sysroot=/rust --remap-path-prefix=$HOME/.cargo=/cargo ${RUSTFLAGS:-}" \
    cross build --release --target "$target"

    local src="target/$target/release/$bin_name"
    if [ "$os" = "windows" ]; then
        src="${src}.exe"
    fi

    if [ ! -f "$src" ]; then
        echo "  Warning: $src not found, skipping"
        return 0
    fi

    rm -rf "$pkg_dir"
    mkdir -p "$pkg_dir/bin"
    cp "$src" "$pkg_dir/bin/"
    chmod +x "$pkg_dir/bin/"* 2>/dev/null || true
    strip "$pkg_dir/bin/"* 2>/dev/null || true

    # Normalize mtimes for reproducible tarballs
    local mdate="202311140000.00"
    find "$pkg_dir" -exec touch -m -t "$mdate" {} + 2>/dev/null || true

    local tarball="$OUT_DIR/${pkg_name}.tar.xz"
    (cd "$pkg_dir" && tar -cJf "$tarball" --format ustar bin)
    rm -rf "$pkg_dir"

    local size
    size=$(stat -f%z "$tarball" 2>/dev/null || stat -c%s "$tarball" 2>/dev/null || echo "unknown")
    echo "  -> $tarball ($size bytes)"
}

write_build_info() {
    local info_file="$OUT_DIR/BUILD_INFO.txt"
    {
        echo "# roff-cli build info"
        echo "built_at_utc=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        echo "rustc_version=$(rustc --version 2>&1)"
        echo "cargo_version=$(cargo --version 2>&1)"
        echo "source_date_epoch=1700000000"
    } > "$info_file"
    echo "  -> $info_file"
}

package_all_targets() {
    cd "$OUT_DIR" && shasum -a 256 roff-*.tar.xz BUILD_INFO.txt > SHA256SUMS 2>/dev/null || true
    echo "  -> $OUT_DIR/SHA256SUMS"
}

verify_reproducibility() {
    local target="${1:-$(get_current_target)}"
    local bin_name="${BIN_NAME:-roff}"

    echo "Verifying reproducibility for $target..."

    do_build "$target"
    shasum -a 256 "$OUT_DIR/bin/$bin_name" > /tmp/roff-verify-h1.txt

    do_build "$target"
    shasum -a 256 "$OUT_DIR/bin/$bin_name" > /tmp/roff-verify-h2.txt

    if diff /tmp/roff-verify-h1.txt /tmp/roff-verify-h2.txt >/dev/null; then
        echo "✓ PASS: same sha256 across two consecutive builds"
        cat /tmp/roff-verify-h1.txt
    else
        echo "✗ FAIL: sha256 differs"
        diff /tmp/roff-verify-h1.txt /tmp/roff-verify-h2.txt
        exit 1
    fi

    if strings "$OUT_DIR/bin/$bin_name" | grep -qE "^/Users/|^/home/"; then
        echo "✗ FAIL: user path leaked in binary"
        exit 1
    fi
    echo "✓ PASS: no user path in binary"
}

show_usage() {
    echo "Usage: $0 [command] [options]"
    echo ""
    echo "Environment variables:"
    echo "  PROJECT_DIR       Project directory (default: script parent dir)"
    echo "  OUT_DIR           Output directory (default: PROJECT_DIR/.release-artifacts)"
    echo "  BIN_NAME          Binary name (default: roff)"
    echo ""
    echo "Commands:"
    echo "  all               Build all targets (requires cross)"
    echo "  native            Build for current host target"
    echo "  release           Build all + package + sha256sums (requires cross)"
    echo "  verify [target]   Build twice + compare sha256 + check no leaks"
    echo ""
    echo "Examples:"
    echo "  $0 native              # Build for current host"
    echo "  $0 all                 # Build all targets"
    echo "  $0 release             # Full release flow"
    echo "  $0 verify              # Verify current target reproducibility"
}
