# v0.7.8 — Node.js / npm package (zhhz#40)

## Headline

`zhhz` is now installable as an npm package. The same conversion engine,
compiled to WebAssembly, with the OpenCC dictionaries baked into the
`.wasm` — no native dependencies, no data directory, no network fetch at
runtime.

```sh
npm install zhhz
```

The npm API is **strictly richer than
[`opencc-js`](https://github.com/nk2028/opencc-js)**: adds script-variant
detection (`detect()`), introspection (`listConfigs` / `listLocales`),
a `Converter` factory class with per-call custom-word injection, and
semantic-region flags (`Converter.forRegion("cn-s", "cn-tw")`). Full
matrix in `docs/npm.md`.

The native CLI and Rust library API are unchanged. The wasm feature is
opt-in (`--features wasm`) and adds zero overhead to the native path:
`cargo test --release --locked` wall-time is unchanged from v0.7.7.

## What's in the package

- `convert(text, config)` — one-shot conversion across all 16 OpenCC configs.
- `convert_with_custom(text, config, entries)` — custom words, **either**
  array form (`[["软件","軟體"]]`) **or** string form (`"软件 軟體|..."`,
  opencc-js DictLike compat).
- `detect(text)` — script-variant detection (cn-s / cn-t / cn-tw / cn-hk /
  jp-t / jp-n), returning `{ region, confidence }` (0–100). **Unique to
  zhhz** — opencc-js has no equivalent.
- `listConfigs()` / `listLocales()` — introspection.
- `configForRegionPair(from, to)` — resolve a semantic region pair to
  the right OpenCC config name (e.g. `(cn-s, cn-tw) → "s2twp"`).
- `new Converter(config)` — reusable factory instance with `.convert` /
  `.convertWithCustom` / `.withCustom` / `.config`. Strictly better than
  opencc-js's closure factory: exposes config, supports per-call
  custom-word injection without rebuilding.
- `Converter.forRegion(from, to)` — build a Converter from semantic
  region flags (mirrors the CLI's `--from` / `--to`).

Output is byte-identical to the CLI: same FMM segmentation, same
conversion-chain pipeline, same dictionary data (`data/UPSTREAM`
pinned to the same commit as v0.7.7).

## Bundle size

~1.5 MiB unzipped (the `.wasm` blob holds the OpenCC dictionary data).
Gzip-compressed transfer is ~500 KiB. Same order of magnitude as
opencc-js.

## Install

```sh
npm install zhhz
```

Or from a clean checkout, the runnable example:

```sh
git clone https://github.com/ljh-sh/zhhz
cd zhhz/examples/node-usage
npm install
npm start   # prints "24 passed, 0 failed"
```

The CLI is unchanged:

```bash
cargo install zhhz --version 0.7.8
# or
curl -L https://github.com/ljh-sh/zhhz/releases/latest/download/zhhz-x86_64-unknown-linux-musl.tar.xz | tar xJ -
```

## Compatibility

- The native CLI binary is unchanged: same args, same output, same
  dictionary data.
- The Rust library API is unchanged: `Converter::new(Config)` /
  `Converter::with_custom(Config, &[...])` / `convert(text) -> String`
  / `detect_text(text)`.
- The npm package is additive to `src/wasm.rs`; the conversion core in
  `src/engine.rs` is untouched.

## What didn't ship

- **Async / streaming API**: WASM string boundary doesn't stream cleanly;
  would require careful `WebAssembly.Memory` design. Revisit if benchmarks
  show the sync path is a bottleneck.
- **napi-rs native binding**: would let Node.js users skip the WASM
  startup cost. Revisit only if benchmarks demand it.
- **Per-platform prebuilds** (`.node` binaries per Node ABI): same blocker
  as above.
- **Replacing `convert_with_custom`'s `Array<[string, string]>` shape**:
  deferred; the current shape works and matches what wasm-bindgen auto-
  generates TypeScript types for.

## Refs

- [zhhz#40](https://github.com/ljh-sh/zhhz/issues/40) — Provide nodejs library
- [nk2028/opencc-js](https://github.com/nk2028/opencc-js) — reference
  JS OpenCC port we strictly exceed in API surface
- `docs/npm.md` — full npm reference
- `examples/node-usage/` — runnable smoke (24 assertions)

## History

- [zhhz#32](https://github.com/ljh-sh/zhhz/issues/32) — sample-based profile
- [zhhz#33](https://github.com/ljh-sh/zhhz/issues/33) — final trie decision
- [zhhz#34](https://github.com/ljh-sh/zhhz/issues/34) — v0.7.x retrospective
- [zhhz#35](https://github.com/ljh-sh/zhhz/issues/35) — arena + cache-line work
- [mneme#74](https://github.com/ljh-sh/mneme/issues/74) — master perf tracking