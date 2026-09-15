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
# Invoked by `google/clusterfuzzlite/actions/build_fuzzers@v1` via the base
# image's `compile` wrapper, which `cd`s into the project source dir and then
# runs `/src/build.sh`. ClusterFuzzLite sets these env vars before invoking
# `compile`:
#   $SRC      — the project source root (varies across CFLite versions:
#               sometimes `/src/<project>`, sometimes `/src`)
#   $OUT      — destination directory for fuzz binaries
#   $WORK     — scratch space for intermediate artifacts
#   $RUSTFLAGS — already includes `-Zsanitizer=<s>` and `--cfg fuzzing`
#                for the active sanitizer; we MUST NOT pass `--sanitizer=`
#                to `cargo fuzz` on top (would activate two sanitizers at
#                once and the build would fail).
#   $SANITIZER, $FUZZING_ENGINE, $FUZZING_LANGUAGE, $ARCHITECTURE
#
# Output:
#   $OUT/zhhz-fuzz-fmm_segment        (libFuzzer binary)
#   $OUT/zhhz-fuzz-convert_roundtrip  (libFuzzer binary)
################################################################################

# ClusterFuzzLite's compile wrapper invokes `bash -eux $SRC/build.sh`
# after `cd $SRC/<project>`. The mounted project root depends on the CFLite
# version (some put it at /src/<project>, some at /github/workspace/...).
# Use $SRC so the build works regardless of mount path.
cd "$SRC/fuzz"

# `cargo fuzz build --release` matches what ClusterFuzzLite's Rust helper
# expects (binary at target/<triple>/release/<name>). No `--sanitizer` flag
# — that would override RUSTFLAGS and trip the "two sanitizers at once"
# build error we hit when wiring RUSTFLAGS ourselves in GH Actions.
cargo +nightly fuzz build --release

# ClusterFuzzLite binary-name convention: alphanumeric / `_` / `-`, no
# extension. Prefix `zhhz-fuzz-` so the fuzzer names don't collide with
# other artifacts in $OUT (e.g. the upstream `zhhz` CLI binary if a
# future change adds one to the same $OUT).
BIN_DIR="target/x86_64-unknown-linux-gnu/release"
for target in fmm_segment convert_roundtrip; do
    if [ ! -f "$BIN_DIR/$target" ]; then
        echo "::error::expected fuzz binary $BIN_DIR/$target not found"
        exit 1
    fi
    cp "$BIN_DIR/$target" "$OUT/zhhz-fuzz-$target"
done

echo "ClusterFuzzLite build OK:"
ls -la "$OUT"/zhhz-fuzz-*
