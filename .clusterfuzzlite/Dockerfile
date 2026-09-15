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
# ClusterFuzzLite's `compile` script invokes `/bin/bash <project_root>/build.sh`
# (where <project_root> is wherever the project source ends up under the
# base-builder's mount). For our layout, that's `/src/zhhz/build.sh`.
# We COPY build.sh there before compile runs.
#
# zhhz has no system-level build deps for the fuzz crate: cargo-fuzz handles
# everything through its own bundled nightly toolchain. We only need a build
# shell with bash + curl + git (already in the base image).
################################################################################
FROM gcr.io/oss-fuzz-base/base-builder-rust

# Copy build.sh to the project root inside the container.
COPY .clusterfuzzlite/build.sh /src/zhhz/build.sh
RUN chmod +x /src/zhhz/build.sh
