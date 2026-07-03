---
layout: default
title: Home
---

<div class="hero">
  <h1>zhhz</h1>
  <p>Self-contained Simplified/Traditional Chinese converter — pure-Rust reimplementation of OpenCC. CLI, Rust library, npm (WASM), and a native Python binding — all backed by the same engine and the same embedded OpenCC dictionaries.</p>
  <div class="cta">
    <a class="btn primary" href="{{ '/install' | relative_url }}">Install</a>
    <a class="btn secondary" href="{{ '/demo' | relative_url }}">Live demo</a>
    <a class="btn secondary" href="{{ '/npm' | relative_url }}">npm API</a>
    <a class="btn secondary" href="{{ '/python' | relative_url }}">Python API</a>
  </div>
</div>

<div class="badges">
  <a href="https://www.npmjs.com/package/zhhz" title="npm package"><img alt="npm" src="https://img.shields.io/npm/v/zhhz?color=cb3837&amp;logo=npm&amp;logoColor=white"></a>
  <a href="https://pypi.org/project/zhhz/" title="PyPI package"><img alt="PyPI" src="https://img.shields.io/pypi/v/zhhz?color=3776ab&amp;logo=python&amp;logoColor=white"></a>
  <a href="https://crates.io/crates/zhhz" title="Rust crate"><img alt="crates.io" src="https://img.shields.io/crates/v/zhhz?color=fc8d62&amp;logo=rust&amp;logoColor=white"></a>
  <a href="https://jsr.io/@ljh-sh/zhhz" title="JSR package"><img alt="JSR" src="https://img.shields.io/jsr/v/@ljh-sh/zhhz?logo=deno&amp;logoColor=white"></a>
  <a href="https://github.com/ljh-sh/zhhz/blob/main/LICENSE" title="Apache 2.0"><img alt="License" src="https://img.shields.io/badge/license-Apache_2.0-blue.svg"></a>
  <a href="https://github.com/ljh-sh/zhhz/actions/workflows/wasm.yml" title="CI"><img alt="Build status" src="https://img.shields.io/github/actions/workflow-status/ljh-sh/zhhz/wasm.yml?branch=main&amp;logo=github-actions&amp;logoColor=white"></a>
</div>

## What is zhhz?

**zhhz** (zh hanzi — 转换汉字, "convert Chinese characters", a palindrome) is a pure-Rust reimplementation of [OpenCC](https://github.com/BYVoid/OpenCC). It converts between Simplified and Traditional Chinese across the same six variants OpenCC supports (Mainland, Taiwan, Hong Kong, Japan old/new), and identifies which variant a piece of text is written in.

All OpenCC dictionaries are embedded at compile time via `include_str!`, so a single **~1.86 MB static binary** (**588 KB xz-compressed** / 803 KB gzip / 663 KB zstd) carries everything it needs — no runtime data fetch, no separate dictionary directory.

## At a glance

```sh
# CLI
echo '汉字' | zhhz                # 漢字 (default s2t)
echo '信息' | zhhz -c s2twp       # 資訊 (Taiwan phrases)
echo '鼠标' | zhhz -c s2twp       # 滑鼠

# Rust library
use zhhz::{Config, Converter};
let c = Converter::new(Config::S2t);
assert_eq!(c.convert("汉字"), "漢字");

# Node.js
import { convert, detect, Converter } from "zhhz";
console.log(convert("汉字", "s2t"));            // 漢字
console.log(detect("他去了西維珍尼亞州"));      // { region: "cn-hk", ... }

# Python
import zhhz
print(zhhz.convert("信息", "s2twp"))   # 資訊
print(zhhz.detect("汉字计算机软件"))  # Detection(region='cn-s', confidence=57)
```

## Four distribution channels, one engine

| Channel | Audience | Install | s2t | t2s |
|---|---|---|---:|---:|
| **CLI** (native) | Shell, AI agents, scripts | `cargo install zhhz` | **88 MB/s** | 88 MB/s |
| **Rust library** | Rust consumers | `[dependencies] zhhz = "0.7"` | same as CLI | same as CLI |
| **npm** (WASM) | Node.js / browsers | `npm install zhhz` | 63 MB/s | **104 MB/s** |
| **Deno** (WASM) | Deno users | `npm:zhhz@…` (live) | 58 MB/s | **108 MB/s** |
| **Python** (PyO3) | Python consumers | `pip install zhhz` | same as Rust | same as Rust |

Two numbers worth highlighting:

- **`t2s` via npm/Deno beats the native CLI** (~120-125 %): the WASM blob loads the OpenCC dictionaries once and runs in-process — no fork + exec + pipe overhead.
- **Deno and Node.js are within 5-10 % of each other** for the same WASM blob — V8 is V8, whether it's running in Node or Deno.

All five share the same Rust conversion core. Conversion output is byte-identical to the OpenCC reference CLI on all 538 supported-config cases. Full perf table: [Benchmarks]({{ '/benchmarks' | relative_url }}).

## Designed for AI agents

`zhhz` is built first for AI agents (Claude, Cursor, custom LLM pipelines, batch jobs). The CLI is deliberately minimal:

- No TUI, no progress bars, no spinners. Plain text on stdout, errors on stderr.
- stdin / stdout friendly. `-` means stdin; positional args are files.
- Stable, predictable, safe. Same input → byte-identical output every time.
- No network, no filesystem writes unless asked (`--in-place`), no temp files.
- Single self-contained binary — drop it in a container and it just works.

## What makes zhhz different

- **Embedded dictionaries**: no `data/` directory, no runtime fetch, no marisa-trie to load. Build with `include_str!` and ship one binary.
- **One engine, four channels**: the same Rust core produces the CLI binary, the npm WebAssembly artifact, the Python `pip install` wheel, and the Rust library. No behavioral drift.
- **Strict superset of `opencc-js`**: same npm install path, same `Converter({from,to})` factory, same custom-words API — plus script-variant detection, introspection (`listConfigs` / `listLocales`), and semantic region flags (`Converter.forRegion("cn-s", "cn-tw")`). See [npm API]({{ '/npm' | relative_url }}) for the full comparison.
- **Memory-safe by construction** — pure Rust, no `unsafe` in the conversion core.
- **APLv2-licensed**, vendored dictionaries from upstream OpenCC.

## Where to go next

- [Install zhhz]({{ '/install' | relative_url }}) — Cargo, npm, pip, direct binary, or build from source
- [Live demo]({{ '/demo' | relative_url }}) — try it in your browser, no install needed
- [CLI reference]({{ '/cli' | relative_url }}) — every flag, every config, examples
- [Rust library]({{ '/library' | relative_url }}) — embedding zhhz in a Rust project
- [Node.js / npm]({{ '/npm' | relative_url }}) — `npm install zhhz`, full API reference
- [Python integration]({{ '/python' | relative_url }}) — `pip install zhhz`, threading, async-wrap recipe
- [Deno integration]({{ '/deno' | relative_url }}) — `npm:zhhz@…` (live) + `jsr:@ljh-sh/zhhz`
- [Benchmarks]({{ '/benchmarks' | relative_url }}) — 4-channel perf table (CLI / npm / Deno / native)
- [Why zhhz]({{ '/why' | relative_url }}) — design goals, scope, what zhhz is NOT
- [FAQ]({{ '/faq' | relative_url }}) — common questions