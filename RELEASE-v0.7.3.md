# v0.7.3 — Merge Phrases + Characters into one trie

## Headline

**zhhz fast realistic: 53.38 → 75.91 MB/s (+42 %)**.
Cumulative from v0.7.0: 33 → 76 MB/s (+130 %).
**vs OpenCC 1.3.1: 0.81× → 1.17×** (we beat OpenCC on realistic!).

One structural change: when a config group has disjoint phrase +
character dicts (the s2t/t2s pattern), merge them into one trie.
Each FMM segment does 1 trie lookup instead of N.

## Bench (10 MB Chinese, Apple Silicon, release)

| mode    | corpus   | v0.7.0 | v0.7.2 | **v0.7.3** | Δ vs v0.7.2 | × opencc |
| ------- | -------- | -----: | -----: | ---------: | ----------: | -------: |
| fast    | realistic| 33.03  | 53.38  | **75.91**  | **+42.2 %** | **1.17×** |
| fast    | worst    | 28.81  | 53.85  | **72.21**  | **+34.1 %** | **0.96×** |
| fast    | ascii-y  | 30.88  | 33.52  | **77.35**  | **+130.7 %** | **2.21×** |
| bigram  | realistic| 27.35  | 27.79  | 34.00      | +22.4 %     | —        |
| trigram | realistic| 27.01  | 28.15  | 33.43      | +18.7 %     | —        |
| opencc 1.3.1 | realistic | 65.45 | 65.68 | 64.94   | —           | 1.00×    |
| opencc 1.3.1 | worst     | 76.00 | 77.23 | 75.12   | —           | 1.00×    |
| opencc 1.3.1 | ascii-y   | 35.02 | 35.28 | 34.97   | —           | 1.00×    |

Cumulative from v0.7.0:

| version | realistic MB/s | × opencc |
| ------- | -------------: | -------: |
| v0.7.0  | 33.03          | 0.50×    |
| v0.7.1  | 52.32          | 0.80×    |
| v0.7.2  | 53.38          | 0.81×    |
| **v0.7.3** | **75.91**  | **1.17×**|

## What changed

One commit (~80 lines), all in `src/config.rs`:

```rust
fn try_merge_group(dicts: &[Value]) -> Result<Option<Dict>, String> {
    // Concatenate all dict texts in priority order.
    // Phrases first (multi-char) shadow Characters (single-char).
    let mut merged_text = String::new();
    for name in &names {
        let raw = data::dict_text_patched(name)?;
        merged_text.push_str(&raw);
    }
    Ok(Some(Dict::from_text(&merged_text)))
}
```

Applied in `expand()` when a group spec has only file-based children.
Merges STPhrases_Generated (508), STPhrases (49,136), STCharacters
(4,011) into one Dict.

## Why this works

Empirically verified key disjointness before merging:

| dict | total keys | multi-char | overlap with chars |
| ---- | ---------: | ---------: | -----------------: |
| STPhrases_Generated | 508 | 508 | 0 |
| STPhrases | 49,136 | 49,136 | 0 |
| STCharacters | 4,011 | 0 | (self) |

Zero overlap → safe to merge. FMM longest-prefix logic naturally
prefers STPhrases multi-char entries over STCharacters single-char.

## Why ascii-y wins 130 %

ascii-y corpus (Latin text, no Chinese chars) almost never matches
any of the 3 original dicts. Previously each FMM segment did 3
failed trie lookups; now it does 1. Lookup cost drops to 1/3 for
unmatched chars.

## Why realistic wins 42 %

Even when chars do match, merging means **one trie walk** instead
of 3 (or, when multi-char phrases match, **one walk that finds the
longest match** instead of short-circuiting after the first dict
match). FMM's longest-prefix semantics are preserved.

## Test results

- 27 unit tests pass
- 10 context tests pass
- 7 conversion tests pass
- 1 doc test
- 1000-sentence diff_corpus output byte-identical to baseline
- **45 / 45 tests pass, 0 regressions**

## Compatibility

- API unchanged
- Output byte-identical to v0.7.2
- WASM build still works
- No new dependencies

## Install / upgrade

```bash
cargo install zhhz --version 0.7.3
npm install zhhz@0.7.3
```

## Files

```
src/config.rs                | 86 ++++++++++++++++++++++++++++++++--
examples/bench_perf.rs       | new
examples/diff_corpus.rs      | new (correctness regression)
RELEASE-v0.7.3.md           | new
```

## Acknowledgements

Result of mneme#74 perf brainstorm. Found via zhhz#26 / #18 / #19
precedent. Diff_corpus pattern ensures correctness before merging
multi-dict configs.

## What's NOT in this PR

- **Q** (Node repacking) — superseded by #10. #10 already exceeds
  opencc on realistic; further work may not be needed.
- **M** (byte-trie self-implementation) — likely no win after #10.
- **#6** (FST crate) — REJECTED, no longest_prefix API.
- **#1 / #12** (SIMD char scan) — could push to 2× opencc but
  effort is 4-6 weeks; defer to v0.8.

Refs: mneme#74, zhhz#26 (closed), zhhz#22.