# v0.8 implementation overhead measurement

Date: 2026-06-27
Hardware: Apple Silicon (M-series)
Corpus: 10 MB Chinese sample (mixed 齣/出 / STPhrases / unambiguous)
Build: `cargo build --release --example bench`
Runs: 3, averaged

## Throughput matrix

| mode    | report | MB/s   | Δ vs no-report |
| ------- | ------ | -----: | -------------: |
| fast    | off    | 28.56  | —              |
| fast    | on     | 16.81  | **-41%**       |
| bigram  | off    | 21.76  | —              |
| bigram  | on     | 14.28  | -34%           |
| trigram | off    | 21.33  | —              |
| trigram | on     | 13.75  | -36%           |
| opencc 1.3.1 | —  | 70.00  | baseline       |

## Implementation overhead (v0.8 vs v0.7 naked `convert()`)

| mode    | v0.7 MB/s | v0.8 MB/s | Δ      |
| ------- | --------: | --------: | -----: |
| fast    |     28.48 |     28.56 | +0.3%  |
| bigram  |     21.70 |     21.76 | +0.3%  |
| trigram |     21.68 |     21.33 | -1.6%  |

All deltas within noise. **Adding report infrastructure cost 0% on
the naked `convert()` path.** This was the design goal: report is
opt-in via `convert_with_report` / `--report` and does not tax the
default `convert()`.

## vs opencc

| engine      | version | MB/s   |
| ----------- | ------- | -----: |
| opencc      | 1.3.1   | 70.00  |
| zhhz fast   | 0.8.0   | 28.56  |
| zhhz bigram | 0.8.0   | 21.76  |
| zhhz trigram| 0.8.0   | 21.33  |

zhhz fast is **0.41× opencc 1.3.1 throughput**. This is a reversal of
the v0.6 claim that zhhz beats opencc — that benchmark was run
against opencc 1.2.0, which is **6.6× slower** than 1.3.1 (10.68 vs
70.00 MB/s on the same hardware + corpus).

What opencc 1.3.1 does not give you that zhhz does:

- Multi-platform static binary (`cargo build --release` → one file,
  no C++ runtime, no marisa dep, no Python binding).
- WebAssembly build (`zhhz` npm package, see v0.5.0).
- Multi-value dict exposure + n-gram disambig (10/10 on 齣/出 test
  cases where opencc gives the wrong answer in default mode).
- Patch overlay (data-driven corrections without forking upstream).
- `--report` sidecar for LLM / encoder post-process.

Use zhhz for **correctness** and **portability**. Use opencc 1.3.1
for **raw throughput on x86 / Linux server bulk conversion**.

## Report-path cost (when `--report` is on)

Report writes one TSV row per multi-value decision. For the 10 MB
sample there are **330 638 multi-value decisions** (33 064 / MB —
artificially high because the bench corpus deliberately maximizes
齣/出 / 了一类 ambiguous chars). The actual write cost is dominated
by:

1. The second FMM walk over `text` (`scan_ambiguous`)
2. Per-decision String allocations (6 fields × 33K = 200K allocs)
3. `fs::write` of the TSV

34-41% overhead is consistent with that workload.

For real-world text (~0.1-0.5% multi-value density) the report
would only add ~1-5% overhead.

## Caveats

- Bench corpus is worst-case (high multi-value density); real
  Chinese text has ~10-100× fewer ambiguous chars.
- Single-machine numbers; not normalized for clock speed.
- opencc 1.3.1 is the current Homebrew `opencc` (verified
  2026-06-27).
- Compare against the **same machine** v0.7 baseline (28.5 MB/s
  fast) rather than the v0.6 cross-hardware 50 MB/s figure.

Refs: zhhz#15 (v0.7/v0.8 split), zhhz#14 (fast-path delta),
mneme#70 (direction survey).