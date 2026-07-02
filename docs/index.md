---
layout: default
title: Home
---

<div class="hero">
  <h1>zhhz</h1>
  <p>Self-contained Simplified/Traditional Chinese converter — pure-Rust reimplementation of OpenCC. CLI, Rust library, npm (WASM), and a planned Python binding.</p>
  <div class="cta">
    <a class="btn primary" href="{{ '/install' | relative_url }}">Install</a>
    <a class="btn secondary" href="{{ '/cli' | relative_url }}">CLI usage</a>
    <a class="btn secondary" href="{{ '/npm' | relative_url }}">Node.js</a>
    <a class="btn secondary" href="https://github.com/ljh-sh/zhhz" target="_blank" rel="noopener">GitHub</a>
  </div>
</div>

## What is zhhz?

**zhhz** (zh hanzi — 转换汉字, "convert Chinese characters", a palindrome) is a pure-Rust reimplementation of [OpenCC](https://github.com/BYVoid/OpenCC). It converts between Simplified and Traditional Chinese across the same six variants OpenCC supports (Mainland, Taiwan, Hong Kong, Japan old/new), and identifies which variant a piece of text is written in.

All OpenCC dictionaries are embedded at compile time via `include_str!`, so a single ~1.7 MB static binary carries everything it needs — no runtime data fetch, no separate dictionary directory.

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
```

## Distribution channels

| Channel | Audience | Status |
|---|---|---|
| CLI (`zhhz` binary) | Shell, AI agents, scripts | Ships today (v0.7.x) |
| Rust library (`cargo install zhhz`) | Rust consumers | Ships today (crates.io) |
| npm (`npm install zhhz`) | Node.js / browsers | Ships today (v0.7.8) |
| Python (`pip install zhhz`) | Python consumers | [Roadmap]({{ '/python' | relative_url }}); use the CLI via subprocess today |

## For AI agents

`zhhz` is built first for AI agents (Claude, Cursor, custom LLM pipelines, batch jobs). The CLI is deliberately minimal:

- No TUI, no progress bars, no spinners. Plain text on stdout, errors on stderr.
- stdin / stdout friendly. `-` means stdin; positional args are files.
- Stable, predictable, safe. Same input → byte-identical output every time.
- No network, no filesystem writes unless asked (`--in-place`), no temp files.

## Where to go next

- [Install zhhz]({{ '/install' | relative_url }}) — Cargo, npm, direct binary, or build from source
- [CLI reference]({{ '/cli' | relative_url }}) — every flag, every config, examples
- [Rust library]({{ '/library' | relative_url }}) — embedding zhhz in a Rust project
- [Node.js / npm]({{ '/npm' | relative_url }}) — `npm install zhhz`, full API reference
- [Python integration]({{ '/python' | relative_url }}) — today's subprocess path + planned PyO3 binding
- [Why zhhz]({{ '/why' | relative_url }}) — design goals, scope, and what zhhz doesn't try to be
- [FAQ]({{ '/faq' | relative_url }}) — common questions