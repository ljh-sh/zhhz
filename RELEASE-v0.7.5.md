# v0.7.5 — kill per-match allocs in trigram disambig

## Headline

**Trigram mode 3-5 % faster.** Fast path unchanged. Same output.
Default `Converter::new()` mode gains a real but modest bump.

Single commit on top of v0.7.4, ~160 lines, no new deps.

## Bench (apples-to-apples, in-process 20 inner + 3 warmup discarded)

Median of 3 runs. OpenCC 1.3.1 included for context on the same harness
(`scripts/bench-fast.sh` lives on the `v0.7.4-marisa-trie` branch).

| corpus       | v0.7.5 fast | v0.7.4 fast | v0.7.5 trigram | v0.7.4 trigram | opencc fast |
|--|--:|--:|--:|--:|--:|
| realistic-repeat | 75.17 | 75.45 | — | — | 71.43 |
| classical-shigong | 41.22 | 40.92 | — | — | 37.35 |
| jinghuayuan | 41.99 | 41.35 | — | — | 38.19 |
| shuotang | 41.83 | 40.44 | — | — | 38.89 |
| mudanting | 45.25 | 44.72 | — | — | 40.31 |
| liezi | 41.63 | 41.33 | — | — | 38.75 |
| yangjiajiang | 40.22 | 38.95 | — | — | 37.18 |
| realistic-var | — | — | — | — | 41.96 |

| corpus | v0.7.5 trigram | v0.7.4 trigram | Δ |
|--|--:|--:|--:|
| realistic | 35.22 | 33.77 | **+4.3 %** |
| classical | 35.01 | 33.19 | **+5.5 %** |
| code | 49.84 | 48.29 | **+3.2 %** |

## What changed

The trigram path's disambig branch was paying 6 small allocations per
multi-value match (combined String, first_chars Vec, seen HashSet,
rest_of_first String, prev_owned Option<String>, plus disambiguate()'s
2N clones). Multi-value matches are ~0.2 % of FMM hits — but each paid
6 allocs.

`src/engine.rs` rewrites that branch to:

- **T1.1** — borrowed `&str` for prev context via new `last_n_chars()`
  helper. No combined String alloc, no reverse-iter char collection.
- **T1.2** — stack `[String; 4]` + linear dedup for first_chars. No Vec,
  no HashSet.
- **T1.3** — emit pick + tail of cands[0] directly. No rest_of_first
  String.
- **T1.4** — skip ngram entirely when both prev_emit and out are empty
  (start of input — no signal anyway).

## Correctness

- 45 / 45 tests pass (27 lib + 10 context_cases + 7 convert + 1 doctest)
- `diff_corpus` output byte-identical to v0.7.4 baseline (24 550 bytes)
- WASM build unchanged (no new code paths)

## What didn't ship

- **zhhz#29 (byte-trie self-impl)**: not implemented in this release.
  Spec lives on [zhhz#29](https://github.com/ljh-sh/zhhz/issues/29).
  Multi-week rewrite, cache-friendly node layout, expected +20-35 %
  on cache-cold corpora.
- **zhhz#30 (marisa-sys FFI)**: rejected. Implementation kept on
  `v0.7.4-marisa-trie` branch (pushed to origin) as reference; gives
  no measurable advantage on zhhz's data shape.

## Sample-based profile (zhhz#32)

A separate sample-based profile (1 ms interval, macOS `sample`)
confirms zhhz is **CPU-faster than opencc but wall-similar** because
zhhz has 2× more idle time per convert. zhhz CPU throughput on
`realistic-var` is **133 MB/s vs opencc's 67 MB/s**. The wall gap is
cache + allocator behavior, not CPU work.

`realistic-repeat`'s high zhhz stddev (2.83 MB/s) is explained by
cache-warm vs cache-cold variance — CPU time is more stable than wall.

## Install

```bash
cargo install zhhz --version 0.7.5
npm install zhhz@0.7.5
```

## Files

- `src/engine.rs` — `convert_segment` disambig branch + new
  `last_n_chars()` / `tail_n_chars()` helpers.
- `Cargo.toml` — version bumped to 0.7.5.

Refs: [zhhz#32](https://github.com/ljh-sh/zhhz/issues/32),
[zhhz#31](https://github.com/ljh-sh/zhhz/issues/31),
[mneme#74](https://github.com/ljh-sh/mneme/issues/74).