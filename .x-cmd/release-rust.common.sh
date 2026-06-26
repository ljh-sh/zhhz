
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

    BIN_NAME="${BIN_NAME:-app}"
    BUILD_MODE="${BUILD_MODE:-zigbuild-first}"
}

get_targets() {
    echo "x86_64-pc-windows-gnu:win:x64"
    echo "aarch64-pc-windows-gnullvm:win:arm64"
    echo "x86_64-apple-darwin:darwin:x64"
    echo "aarch64-apple-darwin:darwin:arm64"
    echo "x86_64-unknown-linux-musl:linux:x64"
    echo "aarch64-unknown-linux-musl:linux:arm64"
    echo "x86_64-unknown-freebsd:freebsd:x64"
    echo "i686-unknown-freebsd:freebsd:i686"
}

install_target() {
    local target="$1"
    if ! rustup target list --installed 2>/dev/null | grep -q "^${target}$"; then
        echo "  Installing target $target..."
        rustup target add "$target" 2>/dev/null || true
    fi
}

is_native() {
    local os="$1"
    case "$(uname)" in
        Darwin)
            [ "$os" = "darwin" ] && return 0
            ;;
        Linux)
            [ "$os" = "linux" ] && return 0
            ;;
    esac
    return 1
}

build_with_zigbuild() {
    local target="$1"
    if command -v cargo-zigbuild >/dev/null 2>&1; then
        cargo zigbuild --release --target "$target"
        return 0
    fi
    return 1
}

build_with_cargo() {
    local target="$1"
    cargo build --release --target "$target"
}

do_build() {
    local target="$1"
    local os="$2"
    local arch="$3"
    local bin_name="${BIN_NAME:-roff}"
    local out_subdir="${os}-${arch}/bin"
    local ext=""

    [ "$os" = "win" ] && ext=".exe"

    echo "Building for $os-$arch ($target)..."

    install_target "$target"

    cd "$PROJECT_DIR"

    if [ "$BUILD_MODE" = "zigbuild-first" ]; then
        if build_with_zigbuild "$target"; then
            :
        elif build_with_cargo "$target"; then
            :
        else
            echo "  Error: build failed"
            return 1
        fi
    elif [ "$BUILD_MODE" = "native-first" ]; then
        if is_native "$os"; then
            build_with_cargo "$target"
        elif build_with_zigbuild "$target"; then
            :
        else
            echo "  Error: zigbuild not available"
            return 1
        fi
    else
        echo "  Error: unknown BUILD_MODE: $BUILD_MODE"
        return 1
    fi

    local src="target/$target/release/${bin_name}${ext}"
    local dst="$OUT_DIR/$out_subdir/${bin_name}${ext}"

    if [ ! -f "$src" ]; then
        echo "  Warning: $src not found, skipping"
        return 0
    fi

    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
    echo "  -> $dst"
}

build_all_targets() {
    rm -rf "$OUT_DIR"
    mkdir -p "$OUT_DIR"

    while IFS=: read -r target os arch; do
        do_build "$target" "$os" "$arch" || true
    done <<EOF
$(get_targets)
EOF

    echo ""
    echo "Done! Artifacts in: $OUT_DIR"
    ls -la "$OUT_DIR"/*/ 2>/dev/null || true
}

show_usage() {
    echo "Usage: $0 [command] [options]"
    echo ""
    echo "Environment variables:"
    echo "  PROJECT_DIR       Project directory (default: script parent dir)"
    echo "  OUT_DIR          Output directory (default: PROJECT_DIR/.release-artifacts)"
    echo "  BIN_NAME         Binary name (default: app)"
    echo "  BUILD_MODE       Build mode: native-first or zigbuild-first"
    echo ""
    echo "Commands:"
    echo "  all              Build all targets"
    echo "  <target>         Build specific target (see below)"
    echo ""
    echo "Options:"
    echo "  --zigbuild-first    Use zigbuild for all targets (default)"
    echo "  --native-first     Use native cargo for current platform only"
    echo ""
    echo "Targets:"
    echo "  win-x64          x86_64-pc-windows-gnu"
    echo "  win-arm64        aarch64-pc-windows-gnullvm"
    echo "  darwin-x64       x86_64-apple-darwin"
    echo "  darwin-arm64     aarch64-apple-darwin"
    echo "  linux-x64        x86_64-unknown-linux-musl"
    echo "  linux-arm64      aarch64-unknown-linux-musl"
    echo "  freebsd-x64      x86_64-unknown-freebsd"
    echo "  freebsd-i686     i686-unknown-freebsd"
}
