# v0.7.4 — SIMD-style ASCII pass-through

## Headline

**ASCII-leading text up to 5.5× faster**. For mixed / Chinese
text, performance is within noise of v0.7.3.

Adds a byte-scan pass-through in the FMM loop: when the segment
starts with ASCII, scan to the next non-ASCII byte and bulk-copy
the ASCII run. Falls back to the regular char walk for
Chinese-leading segments (avoids the scan cost on pure-Chinese
text).

## Bench (10 MB Chinese, Apple Silicon, release)

### Fast path

| corpus   | v0.7.0 | v0.7.1 | v0.7.2 | v0.7.3 | **v0.7.4** | vs opencc 1.3.1 |
| -------- | -----: | -----: | -----: | -----: | ---------: | -------------: |
| realistic| 33.03  | 52.32  | 53.38  | 75.91  | 74.54      | 1.16×           |
| worst    | 28.81  | 53.85  | 53.85  | 72.21  | 73.36      | 0.96×           |
| **ascii-y** | 30.88 | 33.61 | 33.52 | 77.35 | **192.39** | **5.48×**     |

### Bigram path

| corpus   | v0.7.0 | v0.7.1 | v0.7.2 | v0.7.3 | **v0.7.4** |
| -------- | -----: | -----: | -----: | -----: | ---------: |
| realistic| 27.35  | 26.45  | 27.79  | 34.00  | 33.74      |
| worst    | —      | —      | —      | —      | 22.07      |
| ascii-y  | —      | —      | —      | —      | 73.51      |

### Trigram path

| corpus   | v0.7.0 | v0.7.1 | v0.7.2 | v0.7.3 | **v0.7.4** |
| -------- | -----: | -----: | -----: | -----: | ---------: |
| realistic| 27.01  | 26.31  | 28.15  | 33.43  | 33.51      |
| worst    | —      | —      | —      | —      | 21.95      |
| ascii-y  | —      | —      | —      | —      | 71.99      |

### Cumulative from v0.7.0

```
v0.7.0  v0.7.1  v0.7.2  v0.7.3  v0.7.4
  33       52       53       76      75    MB/s realistic
  31       34       34       77     192    MB/s ascii-y
0.50×    0.80×    0.81×   1.17×   1.16×   vs opencc realistic
                                          5.48×  vs opencc ascii-y
```

## What changed

One commit (~70 lines), all in `src/engine.rs`:

For segments whose first byte is ASCII (< 0x80), use a byte-scan
to find the next non-ASCII byte and bulk-copy the ASCII run via
`copy_nonoverlapping`. After the ASCII run, fall back to the
existing trie walk for any remaining multi-byte content.

For segments that start with non-ASCII (Chinese), use the existing
char walk unchanged. This avoids the byte-by-byte scan cost for
pure-Chinese text.

`find_non_ascii` is a simple loop. A real SIMD impl would use
`memchr::memchr3` or the `safe_arch` crate's NEON/SSE2 intrinsics;
the simple loop is sufficient for this PoC and keeps the change
small.

## Correctness

- 1000-sentence `diff_corpus` output **byte-identical to v0.7.3 baseline**
- 45 / 45 tests pass
- WASM build still works
- No new dependencies

## Why ascii-y wins big

Latin text almost never matches any of the Chinese dicts.
Previously, every ASCII byte was processed via `char_indices()`
+ trie walk + 1-byte push. With this change, the entire ASCII run
is found in one scan and copied in one `copy_nonoverlapping` call.
For a 10 MB corpus of pure Latin, that's a ~5× reduction in
per-byte work.

## Why realistic regresses -2 %

Realistic Chinese text starts every segment with a non-ASCII byte
(multi-byte UTF-8 lead). The SIMD path's `if bytes[0] < 0x80`
check is always false, so we fall through to the existing char
walk — but the extra branch check costs a small amount in the
hot loop. This is within noise of the previous v0.7.3 baseline.

## Compatibility

API unchanged. Output byte-identical. WASM unaffected. No new deps.

## Install

```bash
cargo install zhhz --version 0.7.4
npm install zhhz@0.7.4
```

## Files

```
src/engine.rs             | 70 ++++++++++++++++++++++++++++++++++++++++--
examples/bench_perf.rs    | new
examples/diff_corpus.rs   | new
RELEASE-v0.7.4.md         | new
```

## How we got here

Final candidate in the 5-experiment brainstorm tracked at
[mneme#74](https://github.com/ljh-sh/mneme/issues/74).
- #9 sorted Vec — rejected
- #10 merge dicts — shipped as v0.7.3
- #6 FST crate — rejected
- #3 two-level byte trie — rejected (re-bases case bug)
- #1 SIMD char scan — **shipped as v0.7.4**

Refs: [zhhz#26](https://github.com/ljh-sh/zhhz/issues/26),
[mneme#74](https://github.com/ljh-sh/mneme/issues/74),
[mneme#73](https://github.com/ljh-sh/mneme/issues/73).