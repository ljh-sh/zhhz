#!/bin/bash -eu
# Copyright 2026 ljh-sh
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#      http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
################################################################################
# ClusterFuzzLite build script.
#
# Invoked by `google/clusterfuzzlite/actions/build_fuzzers@v1` inside the
# base-builder-rust image. $SRC, $WORK, $OUT are set by ClusterFuzzLite.
#
# Outputs:
#   $OUT/zhhz-fuzz-fmm_segment        (libFuzzer binary)
#   $OUT/zhhz-fuzz-convert_roundtrip  (libFuzzer binary)
################################################################################

# ClusterFuzzLite provides the source via $SRC. Copy the fuzz project
# into $SRC if it's not already there.
if [ ! -d "$SRC/zhhz" ]; then
    cp -R /workspace/zhhz "$SRC/zhhz"
fi

cd "$SRC/zhhz/fuzz"

# cargo-fuzz uses its own bundled nightly toolchain, not the one
# exposed by $CC / $CXX (those are for C/C++ fuzzers). The sanitizer
# choice is wired through the workflow (matrix.sanitizer).
# cargo fuzz build reads it from $SANITIZER env var? No — pass it explicitly.
cargo +nightly fuzz build \
    --sanitizer="${SANITIZER:-address}" \
    --release \
    -- \
    || { echo "cargo fuzz build failed"; exit 1; }

# ClusterFuzzLite convention: binary names must be alphanumeric / `_` / `-`,
# no extension. We prefix with `zhhz-fuzz-` to avoid colliding with other
# binaries in $OUT (e.g. upstream `zhhz` CLI from later additions).
BIN_DIR="target/x86_64-unknown-linux-gnu/release"
for target in fmm_segment convert_roundtrip; do
    if [ -f "$BIN_DIR/$target" ]; then
        cp "$BIN_DIR/$target" "$OUT/zhhz-fuzz-$target"
    else
        echo "::error::expected fuzz binary $BIN_DIR/$target not found"
        exit 1
    fi
done

echo "ClusterFuzzLite build OK: $(ls -1 "$OUT"/zhhz-fuzz-* 2>/dev/null)"
