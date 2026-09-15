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
# So build.sh must live at `$SRC/build.sh` (i.e. `/src/build.sh` inside the
# container), NOT inside the project subdir. We COPY it there explicitly.
#
# zhhz has no system-level build deps for the fuzz crate: cargo-fuzz handles
# everything through its own bundled nightly toolchain. We only need a build
# shell with bash + curl + git (already in the base image).
################################################################################
FROM gcr.io/oss-fuzz-base/base-builder-rust

# Copy build.sh to the location the compile wrapper invokes.
COPY .clusterfuzzlite/build.sh /src/build.sh
RUN chmod +x /src/build.sh
