---
layout: default
title: FAQ
permalink: /faq/
---

# FAQ

## General

### What does the name mean?

`zhhz` is a palindrome: **zh** hanzi (汉字, "Chinese characters") and **z**huan **h**uan **h**an **z**i (转换汉字, "convert Chinese characters"). Pronounce it however you like.

### Is `zhhz` a drop-in for OpenCC?

For all 16 OpenCC configs, yes — same input produces byte-identical output. The vendored dictionary data is pinned to a specific OpenCC commit (`data/UPSTREAM` in the repo). For other OpenCC features (C API, custom-config file format, jieba integration), no — zhhz covers the conversion core, not the entire OpenCC surface area.

### Why Rust?

Three reasons: (1) memory safety by construction (no `unsafe` in the conversion core), (2) one static binary with embedded data — no native dependency hell, (3) reuse the same engine in Rust, WebAssembly (npm), and (eventually) Python without re-implementing the trie walk in three languages.

### What's the bundle / binary size?

| Channel | Size |
|---|---|
| Static CLI binary (musl, ~1.7 MB) | One file, no dependencies. |
| npm package tarball | ~616 kB compressed, ~1.5 MB unpacked. |
| Rust library, debug build | TBD per release; small enough to ignore. |

## Conversion

### Why does `zhhz --from cn-s --to cn-tw` use `s2twp` and not `s2tw`?

Because the phrase-aware variant (`s2twp`) is strictly more capable — it does everything `s2tw` does, plus Taiwan phrase projection (`鼠标 → 滑鼠`, `信息 → 資訊`). When you ask for "Simplified → Taiwan", you almost always want the phrase projection. The CLI picks `s2twp` by design.

If you specifically want character-only conversion without phrase projection, pass `-c s2tw` directly.

### What about Japanese?

`jp2t` and `t2jp` are supported — Japanese Kyūjitai (old-form) ↔ Japanese Shinjitai (new-form) via OpenCC's standard Traditional intermediate form. OpenCC marks these configs as "experimental"; so does zhhz.

### Why does `A型肝炎` become `甲型肝炎` after `tw2sp`?

Because in Taiwan, "A型" is written "甲型" by convention. The Taiwan phrase dictionary includes this normalization as part of the phrase projection step. Same input via `t2s` keeps the `A`.

## Custom words

### How do I add terminology?

Either via the CLI (`--dict path/to/words.tsv`) or the library/API (`Converter::with_custom(...)` / `convert_with_custom(text, config, entries)`). The TSV file format is one `key<TAB>value` per line; lines starting with `#` are ignored. Entries override the built-in tables at the highest priority.

### Why "highest priority"?

Same semantics as OpenCC. Your word wins over any built-in entry that would otherwise match. This is what you want for domain vocabulary and branding — the cost is that a misspelled custom entry could mask a correct built-in one, so validate against your corpus.

## Performance

### Is zhhz faster than OpenCC?

It depends on the input. From the v0.7.x benchmark story ([mneme/perf-stories/](https://github.com/ljh-sh/mneme/tree/main/perf-stories)):

- **Warm cache realistic corpora**: zhhz is ~5–15 % faster than OpenCC.
- **Trigram-style inputs**: zhhz is ~25–35 % faster.
- **Cold cache / shuffled corpora**: closer to even.
- **Huge inputs (100 MB+)**: zhhz's arena output buffer keeps wall time flat; OpenCC's per-segment allocation pressure shows up at this scale.

The conversion output is byte-identical, so the difference is purely throughput.

### Why does zhhz sometimes appear slower than OpenCC on small inputs?

Process startup dominates. For a single short string, `fork() + exec() + parse dictionaries` is more than the actual conversion work. The fix is to build a `Converter` once (Rust) / `Converter` instance once (npm) and reuse it across many calls.

## Distribution

### Why is `zhhz` published to npm as WebAssembly?

Three reasons: (1) zero native dependencies for Node.js consumers — `npm install` and you're done; (2) the OpenCC dictionaries are baked into the `.wasm`, no runtime data fetch; (3) the same `.wasm` runs in Node.js, browsers, Deno, Bun, and edge runtimes.

The cost vs a native binding (napi-rs) is some WASM string-boundary overhead — typically 10–20 % on small-string workloads, less on large inputs. Revisit napi-rs if benchmarks show this is a real bottleneck.

### When will `pip install zhhz` work?

On the [Python integration roadmap]({{ '/python' | relative_url }}). Native PyO3 binding is planned after the npm ship. Until then, the stdlib subprocess helper documented on that page is the supported path.

## Where to go next

- [Install]({{ '/install' | relative_url }})
- [CLI reference]({{ '/cli' | relative_url }})
- [Rust library]({{ '/library' | relative_url }})
- [Node.js / npm]({{ '/npm' | relative_url }})
- [Python integration]({{ '/python' | relative_url }})