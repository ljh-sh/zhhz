---
layout: default
title: Rust library
permalink: /library/
---

# Rust library

`zhhz` is a regular Rust crate. The CLI is just one binary that calls the library; everything in `src/` is also exposed as public API.

## Quick start

Add to `Cargo.toml`:

```toml
[dependencies]
zhhz = "0.7"
```

Convert some text:

```rust
use zhhz::{Config, Converter};

let c = Converter::new(Config::S2t);
assert_eq!(c.convert("汉字"), "漢字");
```

Custom words override the built-in tables at the highest priority:

```rust
let c = Converter::with_custom(
    Config::S2t,
    &[("软件".into(), "軟體".into())],
);
assert_eq!(c.convert("买一台打印机"), "買一台印表機");
```

## Conversion

```rust
pub enum Config {
    S2t, T2s,
    S2tw, Tw2s,
    S2hk, Hk2s,
    S2twp, Tw2sp,
    S2hkp, Hk2sp,
    T2tw, Tw2t,
    T2hk, Hk2t,
    T2jp, Jp2t,
}

pub struct Converter { /* ... */ }

impl Converter {
    pub fn new(config: Config) -> Self;
    pub fn with_custom(config: Config, custom: &[(String, String)]) -> Self;
    pub fn convert(&self, text: &str) -> String;
}
```

All 16 OpenCC configs are variants of `Config`. Use `Config::parse(name)` if you have a string from a CLI flag or config file:

```rust
let cfg = Config::parse("s2twp").expect("known config");
let converter = Converter::new(cfg);
```

## Semantic region flags

For UI code, prefer region codes over config names:

```rust
use zhhz::{Config, Region};

let cfg = Region::parse("cn-s")
    .and_then(|from| region_pair_config(from, Region::CnTw))
    .expect("supported pair");
let converter = Converter::new(cfg);
```

`Region::ALL` is `[CnS, CnT, CnTw, CnHk, JpT, JpN]`. `region_pair_config(from, to)` returns the `Config` that performs the conversion (preferring phrase-aware variants when they exist).

## Detection

```rust
use zhhz::{detect_text, Detection};

let d: Option<Detection> = detect_text("他去了西維珍尼亞州");
// Some(Detection { region: "cn-hk", confidence: 70 })

if let Some(d) = d {
    println!("region = {}, confidence = {}", d.region, d.confidence);
}
```

`Detection::region` is one of `"cn-s"` / `"cn-t"` / `"cn-tw"` / `"cn-hk"` / `"jp-n"` / `"jp-t"`. `confidence` is 0–100 (share of signature characters in the input). Returns `None` when there are no CJK characters or kana.

For raw bytes, use `detect_bytes(&[u8])` (rejects invalid UTF-8).

## Reuse and cost

`Converter` holds parsed dictionaries; building one is the expensive step, calling `convert()` is cheap. Build a `Converter` once at startup and reuse it across many inputs:

```rust
let converter = Converter::new(Config::S2twp);

for line in stdin.lines() {
    let converted = converter.convert(&line?);
    println!("{}", converted);
}
```

Conversion is pure (no I/O, no network, no filesystem access). It's safe to call from any thread; you can also build separate `Converter` instances per thread.

## Feature flags

```toml
[dependencies]
zhhz = { version = "0.7", default-features = false }   # just the core
zhhz = { version = "0.7", features = ["wasm"] }       # + WebAssembly bindings
```

The default feature set is empty. The `wasm` feature pulls in `wasm-bindgen` + `js-sys` and exposes the same conversion / detection API to JavaScript via the `wasm` module (gated by `#[cfg(feature = "wasm")]`). The native path is unaffected by this flag — building without `--features wasm` produces the same `Converter::convert` output.

## Threading

`Converter::convert` takes `&self`. The conversion engine is pure and thread-safe; you can share a single `Converter` across threads (it's `Sync`). For maximum throughput, build one per worker thread and avoid contention.

## Where to go next

- [Node.js / npm]({{ '/npm' | relative_url }}) — same API exposed via WebAssembly
- [Python integration]({{ '/python' | relative_url }}) — subprocess today, PyO3 planned
- [CLI reference]({{ '/cli' | relative_url }}) — if you only need the binary