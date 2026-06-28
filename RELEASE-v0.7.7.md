# v0.7.7 — arena output buffer (allocator pressure kill)

## Headline

**+12 % wall MB/s on cache-warm realistic corpora; +22 % on trigram disambig.**
Same output as v0.7.5 (byte-identical via `diff_corpus`). Same API.
Drop-in replacement.

Single commit on top of v0.7.5: a reusable `Vec<u8>` scratch buffer
owned by `Converter`, swapped out per `convert()` to materialize the
final `String`. Eliminates the 350K per-segment `String::with_capacity`
allocations per 10 MB input.

## Bench (apples-to-apples, in-process 20 inner + 3 warmup discarded)

### macOS Apple M2

| corpus | v0.7.7 fast | v0.7.5 fast | Δ | opencc 1.3.1 fast |
|--|--:|--:|--:|--:|
| realistic-repeat | 87.92 | 77.87 | **+12.9 %** | 71.98 |
| realistic-norepeat | 77.25 | 68.45 | +12.9 % | 66.67 |
| realistic-var | 43.24 | 42.83 | +1.0 % | 41.88 |
| jinghuayuan | 42.53 | 41.42 | +2.7 % | 38.65 |
| shuotang | 42.52 | 42.35 | +0.4 % | 39.19 |
| mudanting | 46.15 | 45.12 | +2.3 % | 40.16 |
| liezi | 41.94 | 41.82 | +0.3 % | 38.75 |
| yangjiajiang | 40.38 | 40.72 | -0.8 % | 37.03 |
| classical | 42.02 | 41.14 | +2.1 % | 37.52 |
| code | 71.93 | 70.05 | +2.7 % | 28.56 |
| news | 40.00 | 34.65 | +15.4 % | 30.47 |

Trigram (v0.7.7 vs v0.7.5):
- realistic-repeat: 46.66 vs 36.62 (**+27.4 %**)
- news: 23.60 vs 19.47 (+21.2 %)

### Linux x86_64 (Docker Desktop, gcc-14)

| corpus | v0.7.7 fast | opencc 1.3.1 fast | ratio |
|--|--:|--:|--:|
| realistic-repeat | 73.73 | 70.40 | 1.05 |
| realistic-norepeat | 66.43 | 65.10 | 1.02 |
| jinghuayuan | 40.86 | 36.95 | 1.11 |
| code | 63.82 | 26.65 | 2.39 |
| news | 33.12 | 28.50 | 1.16 |

## Why

Sample-based profile (zhhz#32) showed zhhz is CPU-faster than opencc
on every corpus but loses 2× in wall time. Root cause: **allocator
pressure** — 350K `malloc`/`free` per 10 MB input pollutes L2 cache with
metadata churn. Arena buffer kills it cleanly with **zero algorithmic
change** and **zero risk of regression** (diff_corpus byte-identical).

v0.7.6 was planned to include cache-line Node layout (zhhz#35 A) but
that regressed warm cache (`-8 %` on realistic-repeat). Reverted; see
[zhhz#35](https://github.com/ljh-sh/zhhz/issues/35) for full data.
v0.7.7 ships only the safe win.

## Bug fix

`tail_n_chars(&str, n)` returned `&s[s.len()..]` when fewer than `n`
chars present, panicking on news corpus trigram path (where `prev_emit`
could be 1 char in a trigram context). Added early-return guard.

## Compatibility

- Drop-in replacement for v0.7.5
- Same `Dict::from_text`, `Dict::from_entries`, `convert(text) -> String`
- 27/27 unit tests pass, doctest passes
- `diff_corpus` byte-identical vs v0.7.5
- New `RefCell<Vec<u8>>` field on `Converter` (zero impact on public API)

## What's next

[mneme#78](https://github.com/ljh-sh/mneme/issues/78) — v0.7.6/7/8/9
stabilisation releases; **v0.8.0 is the perf baseline + translation
control protocol release** (JSON / XML / line / region overrides, all
with zero-cost abstraction in the no-custom path).

Cache-cold corpus gap (jinghuayuan 42.53 → target 60; realistic-var
43.24 → target 65) requires **inline short values** in the Node — design
captured in zhhz#35, planned for v0.8.0.

## Pre-built binaries

This release ships two binaries:

- `zhhz-0.7.7-aarch64-apple-darwin.tar.xz` — macOS Apple Silicon
- `zhhz-0.7.7-x86_64-unknown-linux-gnu.tar.xz` — Linux x86_64

Verified on:
- macOS 15.x, Apple M2, rustc 1.83 (pinned stable-2024-11-28)
- Debian trixie, gcc-14, rustc 1.83 (rustup default)

## History

- [zhhz#32](https://github.com/ljh-sh/zhhz/issues/32) — sample-based profile
- [zhhz#33](https://github.com/ljh-sh/zhhz/issues/33) — final trie decision
- [zhhz#34](https://github.com/ljh-sh/zhhz/issues/34) — v0.7.x retrospective
- [zhhz#35](https://github.com/ljh-sh/zhhz/issues/35) — arena + cache-line work, A reverted
- [mneme#74](https://github.com/ljh-sh/mneme/issues/74) — master perf tracking
- [mneme#77](https://github.com/ljh-sh/mneme/issues/77) — perf concepts
- [mneme#78](https://github.com/ljh-sh/mneme/issues/78) — v0.7.6–v0.8.0 plan