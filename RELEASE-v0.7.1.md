# v0.7.1 — Perf patch release

## Headline

**zhhz fast path: 33 → 52 MB/s on realistic Chinese corpus (+58 %).** vs OpenCC 1.3.1: **0.50× → 0.80×**.

Three small allocator-traffic fixes. No API changes, no new features, byte-identical output to v0.7.0.

## Bench (10 MB Chinese, Apple Silicon, release)

| mode    | corpus   | v0.7.0 | v0.7.1 | Δ      |
| ------- | -------- | -----: | -----: | -----: |
| fast    | realistic| 33.03  | **52.32** | **+58 %** |
| fast    | worst    | 28.81  | **53.85** | **+87 %** |
| fast    | ascii-y  | 30.88  | 33.61  | +8.8 % |
| bigram  | realistic| 27.35  | 26.45  | -3.3 % |
| trigram | realistic| 27.01  | 27.12  | +0.4 % |
| opencc 1.3.1 | realistic | 65.45 | 65.45 | baseline |
| opencc 1.3.1 | worst     | 76.00 | 76.00 | baseline |

vs OpenCC 1.3.1:

| corpus    | v0.7.0 | v0.7.1 | gap remaining |
| --------- | -----: | -----: | ------------: |
| realistic | 0.50×  | **0.80×** | 6 pp to 0.85× |
| worst     | 0.38×  | **0.71×** | 9 pp to 0.80× |
| ascii-y   | 0.89×  | **0.96×** | effectively closed |

worst corpus = artificial high-multi-value density (`一出戏`, `出了`, `了` repeated); realistic = mixed natural Chinese; ascii-y = Latin text without Chinese.

## What changed

Three commits, ~70 lines of diff, all in `src/engine.rs`:

### I — `tail_2_chars` helper

Replaced `out.chars().rev().take(2).collect::<Vec<char>>().into_iter().rev().collect::<String>()` (two allocations per FMM segment) with a single byte-slice walk that allocates only the result `String`.

### J — in-place stage buffer

`convert_segment` now takes `&mut String out` and writes into a caller-provided buffer. The chain loop reuses one buffer across all stages. `chain.len() == 1` (e.g. `t2jp`) skips the loop entirely.

### K — skip `prev_emit` alloc in fast path

The fast path (`ngram.is_none()`) never reads the `keep` value. Return `String::new()` without computing `tail_2_chars` for it.

## How we found it

`cargo install flamegraph` + macOS `sample` showed the hot spot was `Vec<char>::from_iter` running twice per segment — about 700 K unnecessary allocations per 10 MB input.

Six prior micro-opt experiments on the trie structure had all regressed on the realistic corpus. See [zhhz#18](https://github.com/ljh-sh/zhhz/issues/18) for the full list (linear scan, side HashMap, `Vec<u8>` accumulator, `#[inline(always)]`, etc.).

The lesson: **profile first, then optimise.** Source-level guesses about what is hot were wrong every time.

Full story in [mneme#73](https://github.com/ljh-sh/mneme/issues/73).

## What is NOT in this release

- **L**: skip `out.reserve` when capacity already enough → deferred to v0.7.2
- **M**: byte-based trie (would close the remaining 21 % gap) → v0.9 effort
- **N**: unsafe byte copy for `push_str` (C failed; revisit with right helper) → v0.7.2
- **O**: `marisa-trie` integration → v0.10 effort, requires new dep

## Test results

- 27 unit tests pass
- 10 context tests pass (4 ngram + 2 known-limitation + 4 v0.6 baseline)
- 7 conversion tests pass
- 1 doc test
- **45 total, 0 failed**

## Compatibility

- **API unchanged** — `Converter::convert(&str) -> String` signature identical
- **Output byte-identical** to v0.7.0 — all 45 tests pass without modification
- No new dependencies
- WASM build unaffected
- Same `Cargo.toml` profile settings (`lto = true`, `codegen-units = 1`, `strip = true`, `panic = "abort"`)

## Install / upgrade

```bash
# WASM (unchanged)
npm install zhhz@0.7.1

# From source
cargo install zhhz --version 0.7.1

# Or use the static binary attached to this release.
```

## Try it

```bash
echo '汉字计算机软件' | zhhz --fast
# 漢字計算機軟件

echo '汉字计算机软件' | zhhz --trigram --ngram /path/to/3gram.arpa
# uses ngram disambiguation; download from ljh-sh/ngram-exp
```

## Files changed

```
src/engine.rs | 65 ++++++++++++++++++++++++++++++++++----------------------
1 file changed, 39 insertions(+), 26 deletions(-)
```

Plus `examples/bench_perf.rs` (new — rigorous bench harness, 3 corpora, best-of-5, 2 warmup runs).

## Acknowledgements

- macOS `sample` and `cargo-flamegraph` made this work possible
- OpenCC 1.3.1 baseline numbers from Homebrew (`brew install opencc`)
- The 6-failure-then-3-win arc is documented in detail at [mneme#73](https://github.com/ljh-sh/mneme/issues/73)