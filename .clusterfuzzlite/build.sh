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
# image's `compile` wrapper, which `bash -eux $SRC/build.sh` runs after `cd`-ing
# into the project root.
#
# Critical: the env `RUSTFLAGS=--cfg fuzzing -Zsanitizer=<s> ...` that
# ClusterFuzzLite sets is *incompatible* with cargo-fuzz's internal
# `-Zsanitizer=address` for coverage. To avoid the two-sanitizers-at-once
# build error, route the sanitizer through `cargo fuzz --sanitizer=<s>`
# (which sets the sanitizer for the fuzz targets only) rather than via the
# env RUSTFLAGS — but the env RUSTFLAGS is set by the wrapper and we cannot
# unset it without breaking the cluster's `--cfg fuzzing` configuration.
#
# The trick: pass `--sanitizer=<s>` to cargo-fuzz so it picks up the right
# sanitizer from its own profile, and let the env RUSTFLAGS do its thing
# for `--cfg fuzzing`. cargo-fuzz has logic to coordinate these.
#
# Output:
#   $OUT/zhhz-fuzz-fmm_segment        (libFuzzer binary)
#   $OUT/zhhz-fuzz-convert_roundtrip  (libFuzzer binary)
################################################################################

# ClusterFuzzLite's compile wrapper invokes `bash -eux $SRC/build.sh`
# after `cd`-ing into the project root. The mounted project root path
# varies across CFLite versions (we've seen /src/zhhz, /github/workspace/...).
# The action log reports `repo_dir: ...` near the bottom — hard-code the
# path that build_fuzzers@v1 actually mounts at in our setup.
cd /github/workspace/storage/zhhz/fuzz

# Pass the sanitizer through `cargo fuzz --sanitizer=<s>`. cargo-fuzz
# internally translates this into the right rustc flags. The env
# RUSTFLAGS provides `--cfg fuzzing` for our conditional compilation.
cargo +nightly fuzz build \
    --sanitizer="${SANITIZER:-address}" \
    --release

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
