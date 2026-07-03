---
layout: default
title: Install
---

# Install zhhz

`zhhz` ships four install paths, one per distribution channel. Pick the one that matches how you'll use it.

## Cargo (Rust library + CLI)

If you have a Rust toolchain, this is the simplest path — gives you both the `zhhz` CLI binary and the Rust library.

```sh
cargo install zhhz
```

Requires Rust 1.74+. Pins the toolchain to a dated stable channel (`stable-2024-11-28` = Rust 1.83) at build time for reproducible release builds.

## Direct binary

For shell use without a Rust toolchain:

```sh
curl -L https://github.com/ljh-sh/zhhz/releases/latest/download/zhhz-x86_64-unknown-linux-musl.tar.xz | tar xJ -
sudo mv zhhz-x86_64-unknown-linux-musl/bin/zhhz /usr/local/bin/
```

The musl build is fully static and works on any Linux amd64. For macOS / arm64 / Windows, see the [release page](https://github.com/ljh-sh/zhhz/releases/latest) for the matching artifact.

### Binary size

The release tarball contains a single ~1.86 MB static binary (xz-compressed, the tarball is ~600 KB). Compression ratios for the binary alone:

| format | size | download impact |
|---|---:|---|
| uncompressed (musl x86_64) | ~1.86 MB | n/a |
| xz -9 (the release tarball) | ~588 KB | smallest |
| gzip -9 | ~803 KB | universal (no xz on Windows by default) |
| zstd -19 | ~663 KB | fast decompression |

The OpenCC dictionaries themselves are ~1.3 MiB of UTF-8 text, embedded into the binary via `include_str!`.

## Node.js / npm

```sh
npm install zhhz
```

Zero native deps. The OpenCC dictionaries are baked into the `.wasm`. Works in Node.js ≥ 18 and modern bundlers (webpack, vite, rollup). See the [npm reference]({{ '/npm' | relative_url }}) for the full API.

## Build from source

```sh
git clone https://github.com/ljh-sh/zhhz
cd zhhz
cargo build --release
# binary at target/release/zhhz
```

To produce the npm WebAssembly artifact locally (requires `wasm-pack`):

```sh
cargo install wasm-pack          # if you don't have it
wasm-pack build --target bundler --features wasm --out-dir pkg
```

The resulting `pkg/` is what gets published to npm. See `examples/node-usage/` in the repo for a runnable smoke against the npm package.

## Python

A native `pip install zhhz` PyO3 binding is on the [roadmap]({{ '/python' | relative_url }}). Until then, the simplest Python integration is to call the `zhhz` CLI via `subprocess`. See [Python integration]({{ '/python' | relative_url }}) for a stdlib-only example.

## Verifying the install

```sh
$ zhhz --version
zhhz 0.7.8

$ echo '汉字' | zhhz
漢字

$ echo '信息' | zhhz -c s2twp
資訊

$ echo '他去了西維珍尼亞州' | zhhz detect
cn-hk    70    -
```

## Updating

```sh
cargo install zhhz --force      # if installed via Cargo
npm update zhhz                 # if installed via npm
```

For the binary, re-download the latest artifact and replace the installed file.