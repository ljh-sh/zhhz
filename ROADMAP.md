# Roadmap

## Done

- **v0.1.0** — Pure-Rust engine reproducing OpenCC's FMM segmentation +
  conversion-chain pipeline; all 16 OpenCC configs; data embedded via
  `include_str!`; `build.rs` reproducing the 5 generated dictionaries; custom
  dictionaries at highest priority; static ~1.6 MB binary.

## Next

### Correctness confidence

- **Differential-fuzz harness.** Generate random Chinese text, convert with both
  `zhhz` and the reference `opencc` CLI, and assert byte-identical output.
  Publish a parity report. This is the main differentiator over other Rust ports,
  none of which prove parity.

### Platforms / bindings

- **WASM + npm.** `wasm32-unknown-unknown` + `wasm-bindgen`; ship a small npm
  package so browser/Node consumers get OpenCC conversion with no native deps and
  no runtime data fetch.
- **Python native extension.** PyO3 + `maturin`; expose `convert(text, config=...)`
  with wheels for CPython.

### Performance / size

- **Compact dictionary representation.** Replace the per-node `HashMap` trie with
  an FST or double-array trie to cut binary size and startup parse cost. The data
  is ~1.3 MiB of UTF-8 text; a compacted form should land well under 1 MiB baked.
- **Ideographic Description Sequence handling.** Match OpenCC's
  `NextIdeographicDescriptionSequenceLength` for full parity on IDS input
  (vanishingly rare in practice).

### Distribution

- Publish to crates.io; add Homebrew formula and an `x-cmd` install entry once
  the first release is tagged.
