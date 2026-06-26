# zhhz

[![CI](https://github.com/ljh-sh/zhhz/actions/workflows/ci.yml/badge.svg)](https://github.com/ljh-sh/zhhz/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> Self-contained Simplified/Traditional Chinese converter — a pure-Rust, data-embedded reimplementation of [OpenCC](https://github.com/BYVoid/OpenCC).

`zhhz` converts between Simplified and Traditional Chinese (plus Taiwan, Hong Kong, and Japanese-shinjitai variants) using the OpenCC dictionaries. **All dictionaries are embedded in the binary at compile time** — one ~1.6 MB static binary, no runtime download, no separate data directory.

The name is a palindrome: **zh** hanzi, and **z**huan **h**uan **h**an **z**i (转换汉字, "convert Chinese characters").

## Why

OpenCC is the de-facto Chinese-conversion library, but its reference implementation is a C++ library with a CMake build, marisa-trie binaries loaded at runtime, and a [memory-safety bug history](https://github.com/BYVoid/OpenCC/issues/997). `zhhz` is a from-scratch Rust port that:

- **Is one self-contained binary.** Data is embedded via `include_str!`; nothing is fetched or installed alongside it.
- **Is memory-safe by construction** — no C++, no `unsafe` in the conversion core.
- **Supports custom conversion words** at the highest priority, for terminology, branding, or domain vocabulary.
- **Tracks upstream data** via a pinned, reproducible sync script (`scripts/sync-opencc.sh`).

## Install

### Cargo

```bash
cargo install zhhz
```

### Direct binary

```bash
curl -L https://github.com/ljh-sh/zhhz/releases/latest/download/zhhz-x86_64-unknown-linux-musl.tar.xz | tar xJ -
sudo mv zhhz-x86_64-unknown-linux-musl/bin/zhhz /usr/local/bin/
```

### Build from source

Requires Rust 1.74+.

```bash
git clone https://github.com/ljh-sh/zhhz
cd zhhz
cargo build --release   # binary at target/release/zhhz
```

## Usage

```bash
echo '汉字' | zhhz                       # default s2t:  漢字
echo '漢字' | zhhz -c t2s                # t2s:          汉字
echo '信息' | zhhz -c s2twp              # s2twp:        資訊
zhhz -c s2t input.txt                   # convert a file
zhhz -c s2t -i input.txt                # rewrite in place
zhhz --list                             # list all configs
```

Configs (mirrors OpenCC):

| config | direction |
|--------|-----------|
| `s2t` / `t2s` | Simplified ↔ Traditional (OpenCC standard) |
| `s2tw` / `tw2s` | Simplified ↔ Traditional (Taiwan) |
| `s2twp` / `tw2sp` | …with Taiwan phrases |
| `s2hk` / `hk2s` | Simplified ↔ Traditional (Hong Kong) |
| `s2hkp` / `hk2sp` | …with Hong Kong phrases |
| `t2tw` / `tw2t` | Traditional (standard) ↔ Taiwan |
| `t2hk` / `hk2t` | Traditional (standard) ↔ Hong Kong |
| `t2jp` / `jp2t` | Japanese Kyūjitai ↔ Shinjitai |

### Custom dictionaries

A custom dictionary is a TSV file (`key<TAB>value`); `#` lines are ignored.
Entries override the built-in tables at the highest priority:

```bash
# mywords.txt
# key	value
软件	軟體
独家	獨家

echo '买软件吃独家' | zhhz -c s2t --dict mywords.txt   # 買軟體喫獨家
```

## Library

```rust
use zhhz::{Config, Converter};

let c = Converter::new(Config::S2t);
assert_eq!(c.convert("汉字"), "漢字");

// Custom words override the built-in tables.
let c = Converter::with_custom(Config::S2t, &[("软件".into(), "軟體".into())]);
assert_eq!(c.convert("买软件"), "買軟體");
```

The engine is pure Rust with a tiny dependency tree (`serde_json`, `anyhow`) and no
filesystem or network access, so it is straightforward to bind from WASM and Python
(both are on the roadmap).

## How it works

`zhhz` reproduces OpenCC's pipeline exactly:

1. **Segment** the input with forward maximum matching (FMM) against the
   segmentation dictionary group.
2. **Convert** each segment through an ordered chain of dictionary groups; each
   stage re-walks its segment with longest-prefix matching, emitting the first
   candidate on a match.

The group match semantics match OpenCC's `PrefixMatch`: the highest-priority
dictionary with any prefix wins (priority dominates length across dictionaries;
length dominates only within one dictionary).

The OpenCC build system generates five dictionaries at build time (reversed
variant tables, a tofu-risk subset, and a regional-phrase projection). `build.rs`
reproduces all five deterministically from the vendored source data, so `data/`
stays a pure mirror of upstream.

## Data and licensing

Dictionary data is vendored from [BYVoid/OpenCC](https://github.com/BYVoid/OpenCC)
(see [`data/UPSTREAM`](data/UPSTREAM) for the pinned commit) and is
**Apache-2.0**, same as the source code. Re-vendor the latest upstream data with:

```bash
scripts/sync-opencc.sh            # master HEAD
scripts/sync-opencc.sh 1.3.1      # a specific tag/commit
```

## Roadmap

- [x] Pure-Rust engine, all 16 OpenCC configs, embedded data, custom words
- [ ] Differential-fuzz harness proving output parity vs the `opencc` CLI
- [ ] WASM build + npm package (`wasm32-unknown-unknown`)
- [ ] Python native extension (PyO3 / `maturin`)
- [ ] Compact dictionary representation (FST / double-array) for smaller binaries

See [ROADMAP.md](ROADMAP.md).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Issues and PRs are welcome.

## Security

See [SECURITY.md](SECURITY.md). For vulnerabilities, email
[lijunhao@x-cmd.com](mailto:lijunhao@x-cmd.com) rather than opening a public issue.

## License

Apache 2.0 — see [LICENSE](LICENSE). Dictionary data is Apache-2.0, vendored from
OpenCC.
