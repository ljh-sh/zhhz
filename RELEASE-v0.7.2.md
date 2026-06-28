# v0.7.2 — Small perf win on top of v0.7.1

## Headline

**zhhz fast path: 52.32 → 53.38 MB/s on realistic (+2.0 %).** Cumulative vs v0.7.0: **+62 %**. vs OpenCC 1.3.1: **0.81×**.

One small change: `unsafe_push_str_ascii` helper that uses `copy_nonoverlapping` when the matched dict value is ASCII. Skips `String::push_str`'s UTF-8 validation pass.

## Bench (10 MB Chinese, Apple Silicon, release)

| mode    | corpus   | v0.7.0 | v0.7.1 | v0.7.2 | Δ vs v0.7.1 |
| ------- | -------- | -----: | -----: | -----: | ----------: |
| fast    | realistic| 33.03  | 52.32  | **53.38** | +2.0 % |
| fast    | worst    | 28.81  | 53.85  | **55.18** | +2.5 % |
| fast    | ascii-y  | 30.88  | 33.61  | 33.52  | -0.3 % |
| bigram  | realistic| 27.35  | 26.45  | 27.79  | +5.1 % |
| trigram | realistic| 27.01  | 27.12  | 28.15  | +3.8 % |
| opencc 1.3.1 | realistic | 65.45 | 65.45 | 65.68 | baseline |

vs OpenCC 1.3.1 (realistic fast):

| version | MB/s | × OpenCC |
| ------- | ---: | -------: |
| v0.7.0  | 33.03 | 0.50×    |
| v0.7.1  | 52.32 | 0.80×    |
| v0.7.2  | 53.38 | **0.81×**|

## Why only +2 %?

Earlier experiment **C** in [zhhz#18](https://github.com/ljh-sh/zhhz/issues/18) tried
unsafe on the **multi-byte** path and lost 2.6 %. Restricting
unsafe to the **ASCII-only** case (where `value.is_ascii()`
proves validity at runtime) is the right pattern, and gives a
small but consistent win on top of v0.7.1.

The bulk of the OpenCC gap (52 → 65 MB/s) is structural — see
[mneme#73](https://github.com/ljh-sh/mneme/issues/73) and
[zhhz#24](https://github.com/ljh-sh/zhhz/issues/24) for the
analysis. Closing it requires a byte-based trie (v0.9) or
marisa-trie integration (v0.10), not micro-opts.

## What changed

One commit (~38 lines), all in `src/engine.rs`:

```rust
// New helper:
fn unsafe_push_str_ascii(out: &mut String, value: &str) {
    debug_assert!(value.is_ascii());
    let bytes = value.as_bytes();
    let old_len = out.len();
    let new_len = old_len + bytes.len();
    if new_len > out.capacity() {
        out.reserve(new_len - old_len);
    }
    unsafe {
        let dst = out.as_mut_vec().as_mut_ptr().add(old_len);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len());
        out.as_mut_vec().set_len(new_len);
    }
}

// In fast path:
if value.is_ascii() {
    unsafe_push_str_ascii(out, value);
} else {
    out.push_str(value);
}
```

Safe by construction: `value.is_ascii()` proves every byte is
< 0x80, so the output buffer remains valid UTF-8.

## Test results

- 27 unit tests pass
- 10 context tests pass
- 7 conversion tests pass
- 1 doc test
- **45 total, 0 failed**

## Compatibility

API unchanged, output byte-identical, no new deps.

## Files

```
src/engine.rs | 39 +++++++++++++++++++++++++++++++++++++-
1 file changed, 38 insertions(+), 1 deletion(-)
```

Plus `RELEASE-v0.7.2.md` (this file).

## Install / upgrade

```bash
cargo install zhhz --version 0.7.2
# or
npm install zhhz@0.7.2
```

## Acknowledgements

zhhz#24 — detailed analysis with attempt log (L, P, N, S2).
macOS `sample` continues to be the right tool for finding what
to optimise next.