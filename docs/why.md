---
layout: default
title: Why zhhz
permalink: /why/
---

# Why zhhz

OpenCC is the de-facto Chinese-conversion library. `zhhz` is a from-scratch Rust reimplementation built around the same dictionaries, designed to be:

- **One self-contained binary.** Data is embedded via `include_str!`; nothing is fetched or installed alongside it.
- **Memory-safe by construction** — pure Rust in the conversion core.
- **Friendly to custom conversion words** at the highest priority, for terminology, branding, or domain vocabulary.
- **Tracked against upstream data** via a pinned, reproducible sync script (`scripts/sync-opencc.sh`).

## What zhhz is for

- **AI agents** (Claude, Cursor, custom LLM pipelines, batch jobs) that need to call Chinese conversion from a shell pipe or a Node.js process without runtime setup.
- **Server-side text processing** where one static binary that always produces byte-identical output is more valuable than a 12-component build chain.
- **Embedders** who want the same conversion engine reachable from Rust, Node.js (today), and Python (planned) without re-licensing the dictionary data.

## What zhhz is not

- **Not a wrapper around the OpenCC C++ library.** zhhz re-implements the conversion pipeline in pure Rust from scratch (FMM segmentation, ordered dictionary-group chain). The shared asset is the dictionary data, which is vendored from upstream.
- **Not a general-purpose NLP toolkit.** No tokenisation, no part-of-speech tagging, no segmentation for jieba-style use. Conversion only.
- **Not a research platform for new segmentation models.** The default is OpenCC's FMM (preserves upstream parity). An optional DP segmentation path is on the roadmap but opt-in and explicit.

## What zhhz doesn't try to be

- **Not a replacement for OpenCC** — zhhz covers all 16 OpenCC configs and tracks upstream data, but zhhz does not promise to be a drop-in C ABI for OpenCC. If you need that, use the C++ library directly.
- **Not the fastest possible implementation.** zhhz is competitive with OpenCC on warm corpora (within 5–15 % on most benchmarks, sometimes faster) and significantly faster on trigram-style inputs. For absolute peak throughput the path forward is a compact FST/double-array trie representation — on the roadmap, but not the priority.
- **Not a UI.** No TUI, no progress bars, no animated anything. Plain text on stdout, errors on stderr.

## Scope decisions

A few non-obvious choices worth knowing:

| Decision | Why |
|---|---|
| Hand-rolled CLI arg parser, no `clap` | The toolchain is pinned to `stable-2024-11-28` (= Rust 1.83) for reproducible release builds; `clap 4.6+` needs edition2024 / 1.85, so a hand-rolled parser keeps the static binary small. |
| `build.rs` regenerates 5 build-time dictionaries | OpenCC's build system generates reversed variant tables, a tofu-risk subset, and regional-phrase projections at build time; zhhz reproduces all five deterministically from the vendored source data so `data/` stays a pure upstream mirror. |
| Custom dicts go to the highest priority | Same semantics as OpenCC's `--dict` — your terminology, branding, or domain vocabulary wins over the built-in tables. |

## Where the design lives

Design notes, retrospective write-ups, and benchmark data live in the private [`mneme`](https://github.com/ljh-sh/mneme) repo (specifically `mneme/zhhz-design/` and `mneme/perf-stories/`). They are not part of the public `zhhz` repo by design — see the `mneme` repo's `README` for the convention.

## Related

- [FAQ]({{ '/faq' | relative_url }})
- [Install]({{ '/install' | relative_url }})
- [CLI reference]({{ '/cli' | relative_url }})