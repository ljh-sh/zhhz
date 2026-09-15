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
# ClusterFuzzLite's `compile` script expects build.sh at `$SRC/build.sh`
# (the project root inside the container), not at `.clusterfuzzlite/build.sh`.
# We copy it into place here so the Rust-specific build steps run from the
# canonical location ClusterFuzzLite looks at.
#
# zhhz has no system-level build deps for the fuzz crate: cargo-fuzz handles
# everything through its own bundled nightly toolchain. We only need a build
# shell with bash + curl + git (already in the base image).
################################################################################
FROM gcr.io/oss-fuzz-base/base-builder-rust

# Copy build.sh to the canonical ClusterFuzzLite location ($SRC/build.sh).
# ClusterFuzzLite's compile wrapper invokes `/bin/bash -c compile` from the
# base image, which in turn sources $SRC/build.sh.
COPY .clusterfuzzlite/build.sh $SRC/build.sh
RUN chmod +x $SRC/build.sh
