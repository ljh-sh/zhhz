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
# ClusterFuzzLite Dockerfile.
#
# ClusterFuzzLite's base-builder `compile` wrapper runs:
#     BUILD_CMD="bash -eux $SRC/build.sh"
# so build.sh must live at `$SRC/build.sh` (i.e. /src/build.sh inside the
# container), not inside the project subdir. We COPY it there explicitly.
#
# zhhz's fuzz crate uses `cargo fuzz build --release` which compiles against
# the Rust standard library via `-Z build-std`. That requires the `rust-src`
# component to be installed in the pinned nightly toolchain (the base image
# doesn't ship it).
################################################################################
FROM gcr.io/oss-fuzz-base/base-builder-rust

# Install rust-src for the pinned nightly so `cargo +nightly fuzz build`
# (which uses -Z build-std) can rebuild std under the active sanitizer.
RUN rustup component add rust-src --toolchain nightly

# Copy build.sh to the location the compile wrapper invokes.
COPY .clusterfuzzlite/build.sh /src/build.sh
RUN chmod +x /src/build.sh
