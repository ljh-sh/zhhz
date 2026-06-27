# v0.8 implementation overhead measurement

Date: 2026-06-27
Hardware: Apple Silicon (M-series)
Corpus: 10 MB Chinese sample (mixed 齣/出 / STPhrases / unambiguous)
Build: `cargo build --release --example bench`
Runs: 3, averaged

## Throughput matrix

| mode    | report | MB/s   | Δ vs no-report |
| ------- | ------ | -----: | -------------: |
| fast    | off    | 29.32  | —              |
| fast    | on     | 17.02  | **-42%**       |
| bigram  | off    | 21.20  | —              |
| bigram  | on     | 14.21  | -33%           |
| trigram | off    | 21.39  | —              |
| trigram | on     | 13.96  | -35%           |
| opencc  | —      | 10.68  | baseline       |

## Implementation overhead (v0.8 vs v0.7 naked `convert()`)

| mode    | v0.7 MB/s | v0.8 MB/s | Δ      |
| ------- | --------: | --------: | -----: |
| fast    |     28.5  |     29.3  | +2.8%  |
| bigram  |     21.7  |     21.2  | -2.3%  |
| trigram |     21.5  |     21.4  | -0.5%  |

All deltas within noise. **Adding report infrastructure cost 0% on the
naked `convert()` path.** This was the design goal: report is opt-in
via `convert_with_report` / `--report` and does not tax the default
`convert()`.

## Report-path cost (when `--report` is on)

Report writes one TSV row per multi-value decision. For the 10 MB
sample there are **330 638 multi-value decisions** (33 064 / MB —
artificially high because the bench corpus deliberately maximizes
齣/出 / 了一类 ambiguous chars). The actual write cost is dominated
by:

1. The second FMM walk over `text` (`scan_ambiguous`)
2. Per-decision String allocations (6 fields × 33K = 200K allocs)
3. `fs::write` of the TSV

33-42% overhead is consistent with that workload.

For real-world text (~0.1-0.5% multi-value density) the report
would only add ~1-5% overhead.

## Caveats

- Bench corpus is worst-case (high multi-value density); real
  Chinese text has ~10-100× fewer ambiguous chars.
- Single-machine numbers; not normalized for clock speed.
- Compare against the **same machine** v0.7 baseline (28.5 MB/s fast)
  rather than the v0.6 cross-hardware 50 MB/s figure.

Refs: zhhz#15 (v0.7/v0.8 split), zhhz#14 (fast-path delta).