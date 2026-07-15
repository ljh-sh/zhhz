# Roadmap

## Done

- **v0.8.2** — `zhhz info <CONFIG>` introspection subcommand. Prints
  config name, description, source/target region, and a small
  input/output example for any of the 16 built-in OpenCC configs.
  Pure UI; no conversion change. Bundled in v0.8.2 release: also covers
  v0.8.0 (TW/HK classifier tuning — `滑鼠` → `cn-tw`) and v0.8.1
  (`--auto --target <REGION>`). All 43 tests pass; clippy clean.
  GitHub Release + crates.io published.
- **v0.7.8** — WebAssembly bindings shipped as `npm install zhhz`;
  `convert` / `convert_with_custom` / `detect` / `listConfigs` /
  `listLocales` / `configForRegionPair` / `Converter` (factory class)
  exposed to Node.js with no native dependencies. API surface is
  strictly richer than [`opencc-js`](https://github.com/nk2028/opencc-js)
  (adds detection, introspection, semantic-region flags, per-instance
  custom-word injection). OpenCC dictionaries embedded in the `.wasm`.
  Closes [zhhz#40](https://github.com/ljh-sh/zhhz/issues/40).
- **v0.3.0** — `zhhz detect` subcommand (mirrors `chardet`'s CLI): file /
  stdin / `--files-from` / `-0 --null` / recursive dir walk; output
  `<region>\t<confidence>\t<path>`. Library API: `detect_text`,
  `detect_bytes`. Published on crates.io.
- **v0.2.0** — `--from` / `--to` semantic region CLI for multi-region 简繁
  (`cn-s` / `cn-t` / `cn-tw` / `cn-hk` / `jp-t` / `jp-n`). Differential
  parity harness against opencc (`examples/parity.rs`); CI builds opencc from
  `data/UPSTREAM` for an authoritative same-data gate. Parity vs opencc 1.2.0:
  538/538 supported-config cases byte-for-byte.
- **v0.1.0** — Pure-Rust engine reproducing OpenCC's FMM segmentation +
  conversion-chain pipeline; all 16 OpenCC configs; data embedded via
  `include_str!`; `build.rs` reproducing the 5 generated dictionaries; custom
  dictionaries at highest priority; static ~1.6 MB binary.

## Next

Features are split by compatibility risk with opencc.

### Safe (no risk of diverging from opencc)

These add new functionality or polish without touching the conversion core's
parity contract.

- **Streaming / batch.** Recursive directory conversion with progress
  reporting; large-file streaming reader for x-cmd pipelines. Conversion
  output is byte-identical to the existing `convert()` path. **Scope
  pending user clarification** — see
  [mneme#111](https://github.com/ljh-sh/mneme/issues/111).
- **Char-level CJK diff.** Implemented as a standalone tool rather than
  a `zhhz` subcommand — `zhhz` is for conversion, not general-purpose
  text comparison. Tracked at
  [mneme#110](https://github.com/ljh-sh/mneme/issues/110): multi-mode
  (char / hanzi / word / line), multi-algo (lcs / patience / myers /
  fmm), `--agent`-friendly JSON output. FMM match is the hard part
  (O(N²) on a pre-segmented word list, O(N) extra memory).
- **Python native extension.** PyO3 + `maturin`; wheels for CPython; the
  same `Converter` API exposed as a Python module. Public demand:
  [zhhz#41](https://github.com/ljh-sh/zhhz/issues/41).
- **IDS handling verification.** Most likely already byte-identical to
  opencc on Ideographic Description Sequence input (vanishingly rare in
  practice). Add a parity corpus case to confirm; if it diverges, add a
  single-line `is_ids_prefix()` advance in segment + convert to restore
  parity.
- **More parity fuzzing.** Expand the corpus (longer texts, mixed scripts,
  adversarial Unicode) and run the differential harness on every PR.
- **Compact dictionary representation (FST).** Replace the per-node
  `HashMap` trie with a finite-state transducer or double-array trie to cut
  binary size and startup parse cost. **Conditional**: the conversion
  output must remain byte-identical, verified by the parity harness.
  Memory isn't a primary concern, so this is a nice-to-have, not a
  priority.

### Diverges from opencc (only when we can prove it is strictly better)

These change behavior relative to opencc and so need strong justification
and a documented opt-in.

- **FMM DP segmentation** ([mneme#62](https://github.com/ljh-sh/mneme/issues/62);
  upstream [#475](https://github.com/BYVoid/OpenCC/issues/475)). Default
  remains `Fmm` (preserves opencc parity); an opt-in
  `--segmentation dp` enumerates all valid segmentations and picks the best
  by a small scoring function (fewest segments; tie-broken by total covered
  length). Fixes 33 documented opencc cases (e.g.
  `正则表达式 → 正規表示式` in `s2twp`). Ship behind the flag with a
  before/after parity report.