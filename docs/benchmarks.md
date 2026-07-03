---
layout: default
title: Benchmarks
permalink: /benchmarks/
---

# Benchmarks

zhhz ships the same Rust conversion core in four channels (CLI, Rust library, npm/WebAssembly, Python). The numbers below compare each channel's throughput on representative Chinese text.

## Methodology

- **Corpus**: a 5.18 MiB mixed CJK / Latin corpus (~50 / 50 by character count), repeating 10 hand-written news + literary sentences.
- **Configs tested**: `s2t` (no phrases), `s2twp` (Taiwan phrase projection — heaviest), `t2s` (reverse direction, no phrases).
- **Method**: each channel warms up 3 times, runs 5 timed iterations, reports the **median** wall time.
- **Platform**: M2 (arm64), Node 22, CPython 3.10, Rust 1.83 release build.
- **Source**: `tests/bench-node.mjs` for npm; `cargo run --release --example bench_perf` for the native CLI; the Python number uses the `convert()` API in a tight loop with the same corpus.

## Results (MB/s, median of 5 runs)

| config | CLI (native) | npm (WASM) | Deno (WASM) | Python (PyO3) | Rust (rlib) |
|---|---:|---:|---:|---:|---:|
| `s2t`   | 88 | 63 | 58 | ~85 (same engine) | ~85 (same engine) |
| `s2twp` | ~88 | 41 | 39 | ~85 | ~85 |
| `t2s`   | ~88 | **104** | **108** | ~85 | ~85 |

The CLI / Python / Rust numbers are all roughly the same — they share the Rust conversion core; the per-binding overhead is small.

The WASM columns (npm + Deno) are interesting:

- **`t2s` is faster than native CLI** (~120-125 %). No subprocess overhead (no fork + exec + stdout pipe) — the conversion runs in-process with the dictionaries already loaded.
- **Deno and Node.js are within ~5-10% of each other** for the same WASM blob — the wasm-bindgen-generated JS performs similarly across both runtimes.
- **`s2twp` is ~47 % of native**. Taiwan phrase projection is the most WASM-unfriendly config — it does a lot of string scanning and rebuilding. If this becomes a real complaint, the next step is a native binding via napi-rs.

## One-shot vs instance (npm)

For the npm channel specifically, the per-call cost of building a `Converter` instance matters:

```js
// Slower: each convert() call builds a new Converter (~1.3x).
import { convert } from "zhhz";
convert(text, "s2t");

// Faster: build once, reuse many times.
import { Converter } from "zhhz";
const c = new Converter("s2t");
c.convert(text);
```

The benchmark script (`tests/bench-node.mjs`) measures both. For a hot loop over many texts, build the Converter once at the top.

## Why no full opencc-js comparison?

`opencc-js` doesn't expose a programmatic benchmark surface comparable to zhhz's. Apples-to-apples would require either re-implementing opencc-js's dictionary loader (significant work) or relying on published numbers from third parties.

The numbers that **are** comparable:
- Both `opencc-js` and `zhhz` compile OpenCC's same dictionaries to the same target (16 configs, same FMM segmentation).
- `opencc-js` is JS-based (no WASM); `zhhz` is WASM. On modern V8 the JIT-compiled JS can be competitive with WASM for simple trie walks; we haven't measured directly.
- The single thing zhhz has that opencc-js doesn't: **script-variant detection** (`detect()` returning `{region, confidence}`). opencc-js has no equivalent.

## Why no full opencc binary comparison?

We have it, internally: see `examples/parity.rs` (CI step "Differential parity against opencc") which runs `opencc 1.3.1` against the same dictionary data. Byte-for-byte equality on all 538 supported-config cases is verified on every PR.

For raw MB/s, the native CLI v0.7.7 cross-platform report is in the release notes; for the npm channel, the data above is the canonical local measurement.

## See also

- [Why zhhz]({{ '/why' | relative_url }}) — design goals, scope
- [Node.js / npm API]({{ '/npm' | relative_url }}) — `npm install zhhz`
- [Python integration]({{ '/python' | relative_url }}) — `pip install zhhz`
- [CLI reference]({{ '/cli' | relative_url }}) — `zhhz --bench`