# Building `zhhz` — notes for an AI-agent tool

`zhhz` is a pure-Rust reimplementation of [OpenCC](https://github.com/BYVoid/OpenCC),
the Chinese conversion library. The interesting design constraint is not the
language model or the conversion algorithm — it is the **consumer**: the
primary user is an AI agent shelling out to a CLI or loading a library, not a
human typing into a terminal. This article is the experience of designing for
that consumer.

The project lives at <https://github.com/ljh-sh/zhhz>; the design and
specification HQ is the private `ljh-sh/mneme` repo.

## 1. Why a self-contained binary matters for agents

An agent invoking a CLI tool has to (a) locate the binary, (b) provide the
right inputs, (c) parse the output, (d) clean up afterwards. Every
external dependency on the runtime — a separate data directory, an
`opencc` shared library, a Python virtualenv, a network call to a model API —
is one more thing the agent's harness has to set up correctly before the tool
can do anything. Most of those failures are silent: `opencc` exits 0 with
empty output when it can't find `s2t.json` in the data directory, and the
agent gets a zero-byte string and a hard-to-diagnose downstream error.

`zhhz` is one binary. The OpenCC dictionaries (~1.3 MiB of `.txt`) are
embedded via `include_str!` at compile time. No data directory, no
`LD_LIBRARY_PATH`, no environment variable to set, no `--path` flag to
remember. Drop the binary in a container, on a sandbox VM, on a CI runner,
in a scratch `tar` — it works the same way every time. The release artifacts
are signed (`cosign`) and the project carries an OpenSSF Scorecard.

The cost is binary size: the unstripped, unoptimized binary is ~1.6 MiB; the
release build (lto + strip + abort-on-panic + codegen-units=1) is 1.7 MiB on
macOS arm64, ~1.8 MiB on Windows GNU. That is fine. An agent is not going
to notice a 1.7 MiB download; a human running `cargo install zhhz` is not
either. The trade-off — size for operational simplicity — is the right one.

## 2. The conversion algorithm: be opencc, not better-than-opencc

OpenCC has a known class of bugs in its segmentation (the most-cited is
[#475](https://github.com/BYVoid/OpenCC/issues/475): the FMM segmenter cannot
backtrack, so phrases whose prefix is itself a phrase get split). A
"better-than-opencc" Rust port could fix these. We chose not to.

The reason is correctness definition. For Chinese character conversion, we
do not yet have a principled notion of "correct" that diverges from OpenCC.
The dictionaries are the convention; the algorithm is what the convention
ships. Diverging from OpenCC means deciding what the convention *should* be,
which is a research problem, not a porting problem. So `zhhz` reproduces
OpenCC's pipeline exactly — FMM segmentation, ordered conversion chain,
first-candidate emission, dict-group semantics where the highest-priority
member wins — and the 538/538 byte-for-byte match against `opencc 1.2.0` on
the parity corpus is the correctness contract.

The future `FMM DP` segmentation fix is held behind an opt-in
`--segmentation dp` flag with the default staying as `Fmm`. The rationale is
written into the commit message and the changelog: "intentional divergence on
a known opencc defect, documented as such." When we add it, the parity
harness will move from "must match opencc" to "must match opencc on default,
must improve on the 33 documented #475 cases on `--segmentation dp`".

## 3. The build system reproduces OpenCC's generated dictionaries

OpenCC ships five dictionaries that are generated at build time from the
source `.txt` files (reverse-direction variant tables, a tofu-risk subset of
`TSCharacters`, and a regional-phrase projection from `HKPhrases`+`TWPhrases`
into a Simplified-shape lookup for `STPhrases`). The reference builds these
via Python scripts invoked from CMake.

`zhhz`'s `build.rs` reimplements all five generation rules in pure Rust.
The source `.txt` files are committed under `data/` and form a clean mirror
of upstream; the generated files are written into `OUT_DIR` and `include_str!`'d
into the binary. This keeps `data/` trivially auditable (`diff` it against
upstream), keeps the build hermetic (no Python or CMake at build time), and
keeps the platform surface small — the only Rust toolchain is needed to
build the binary.

The tradeoff is that the build.rs reimplements ~150 lines of logic that
upstream expresses in Python. The benefit — the user only ever runs `cargo`
— is worth it.

## 4. `zhhz detect`: a small feature that paid for itself

Half a day of work, ~200 lines: a classifier that reads a piece of Chinese
text and tells you whether it is Simplified, Traditional (OpenCC standard),
Taiwan Traditional, Hong Kong Traditional, Japanese Shinjitai, or Japanese
Kyūjitai, with a 0–100 confidence.

The trick is that the per-region signature character sets are derived from
the vendored OpenCC dictionaries at runtime — no new dependencies, no
hand-curated lists, automatically in sync with the data. The classifier
splits on Hiragana/Katakana presence (→ JP branch) versus not (→ Chinese
branch), then for the Chinese branch counts which region-exclusive
characters appear and picks the winner with a TW/HK upgrade heuristic.

Why this is worth shipping: an agent that has a chunk of Chinese text and
wants to *do* something with it often needs to know what kind of Chinese
it is first. The detection call is a 1.7 MiB-binary subprocess that
returns `<region>\t<confidence>\t<path>`, deterministic, no state, no
network. The agent pipes in `path/to/corpus.txt` and pipes out a tab
line. That is the right shape.

## 5. Designed for AI agents, not humans

The CLI deliberately avoids the things humans expect and agents do not:

- **No TUI, no progress bars, no spinners.** Output is plain text on
  stdout, errors on stderr. An agent captures both with `subprocess.run`,
  splits on `\n`, and proceeds.
- **stdin / stdout are first-class.** No files required. Pipe in text, get
  text out.
- **Same input → byte-identical output.** Every time. This is what makes
  caching, diffing, and testing tractable. There is no clock-dependence,
  no locale-dependence, no random tie-breaking in the conversion code.
- **chardet-style batch input.** `zhhz detect <files>...` /
  `--files-from <PATH|->` / `-0` / `--null` / recursive directory walking
  — the same pattern `chardet` uses, so an agent that already knows how to
  drive `chardet` knows how to drive `zhhz`. (v0.4 extends this to
  `zhhz convert`.)
- **No filesystem writes unless asked.** `zhhz input.txt` reads, converts,
  writes to stdout. `zhhz -i input.txt` rewrites in place. The agent
  decides.
- **Plain exit codes.** 0 on success, 1 on any read error. The agent
  branches on `$?`.

The README documents all of this up front ("Designed for AI agents"), so a
human evaluating the project knows not to expect a wizard.

## 6. Packaging: size, signing, supply-chain hygiene

Release engineering for an AI-agent tool has a different shape than for a
human-facing app:

- **Release tarballs, not installers.** The agent downloads a single
  `.tar.xz` containing one binary. No `.msi`, no `.dmg`, no `.deb`. The
  tarball is signed (`cosign sign-blob`) and the signature is attached
  alongside (`*.sigstore.json`).
- **`crates.io` publish, not npm.** The conversion core is Rust; Python
  and JS bindings ship later as separate packages. The library is on
  crates.io as of v0.3.0.
- **Cross-compiled for all 7 targets** (linux gnu/musl × x86_64/aarch64,
  macOS × x86_64/aarch64, windows gnu × x86_64) via `cargo-zigbuild`, so
  the agent's harness can grab the right artifact for its runtime without
  a build step.
- **OpenSSF Scorecard + CodeQL + cargo-deny + dependabot.** Standard
  supply-chain hygiene. The repo is public, so Scorecard publishes to
  the public security tab. `cargo-deny` blocks unknown-license transitive
  deps. `dependabot` keeps GitHub Actions and Cargo deps current.

## 7. The parity harness as the correctness gate

For v0.2 the project shipped a differential parity harness
(`examples/parity.rs`) that compares `zhhz` against the `opencc` CLI
byte-for-byte across all 16 built-in configs and a corpus of real phrases
plus edge cases. The harness classifies results into:

- **pass** — both sides produce the same string;
- **unsupported** — the reference `opencc` does not implement the config
  (`s2hkp` and `hk2sp` are missing from `opencc 1.2.0`; `zhhz` covers
  them, a real feature lead);
- **mismatch** — both ran, output differs.

The initial run against system opencc 1.2.0 was 538 / 538 supported-config
passes, 78 unsupported, and 8 mismatches in `s2twp` / `tw2sp`. All 8 were
data-version differences — `zhhz` vendors `cf0e4b6` (OpenCC master), which is
newer than 1.2.0 and includes fixes for `B超 → 超音波`, `密歇根州 →
密西根州`, etc. that 1.2.0 lacks. No engine bug.

To close the "data version" gap and make the parity harness an
authoritative gate, `scripts/build-reference-opencc.sh` builds the
reference opencc binary from the same commit (`data/UPSTREAM`) and same
data that `zhhz` vendors. The CI workflow installs cmake + g++, runs the
build script, points `OPENCC_BIN` + `OPENCC_DATA_DIR` at it, and runs the
parity harness. When CI passes, the contract is: `zhhz` is byte-identical
to opencc on every config both implement.

## 8. What we have learned

A few things that surprised us, in order of how much they shaped the
project:

1. **The conversion engine was the easy part.** OpenCC's C++ source is
   small and well-organized; translating it to Rust with the right test
   harness took a week. The data — vendoring, regenerating, building
   into the binary — took longer than the algorithm.

2. **The release pipeline took longer than the engine.** Tagging the
   release, cross-compiling 7 targets with `cargo-zigbuild`, attaching
   cosign signatures, uploading to crates.io, getting the GitHub Release
   to appear — all of this is operational work that does not feel like
   "writing a converter" but is what makes the tool actually usable by
   someone else.

3. **`Cargo.toml` version bumps interact with `cargo publish --locked`
   in a way that bites if you forget to regenerate the lockfile.** The
   first `cargo publish` attempt for v0.3.0 failed because the bump from
   `0.1.0` to `0.3.0` invalidated `Cargo.lock` against `--locked`.
   Regenerating the lockfile and committing it alongside the bump fixed it.

4. **The OpenSSF Scorecard badge is what makes the supply-chain story
   visible.** Without it, the public repo looks like any other hobby
   project. With it, the signed releases + denied-license + CI matrix
   become legible at a glance.

## 9. What is next

The roadmap is split by compatibility risk:

- **Safe (no risk of breaking parity with opencc)**:
  detect TW/HK classifier tuning, auto-detect-and-convert with a default
  target of Simplified, chardet-style batch input for the convert
  subcommand, WASM + npm bindings, Python (PyO3) bindings, IDS handling
  verification, more aggressive parity fuzzing, a compact-trie /
  pre-serialized dictionary format (see
  [mneme#64](https://github.com/ljh-sh/mneme/issues/64)).

- **Diverges from opencc (only when we can prove it is strictly
  better)**:
  the FMM DP segmentation fix
  ([mneme#62](https://github.com/ljh-sh/mneme/issues/62); upstream
  [#475](https://github.com/BYVoid/OpenCC/issues/475)), held behind the
  opt-in `--segmentation dp` flag with the default staying as `Fmm`.

The compact-trie work is a real win for the wasm and Python bindings
where every cold start pays the trie-build cost. The decision of *how* to
serialize (custom binary, `fst`, OpenCC's own `.ocd2` via a pure-Rust
reader) is non-trivial and lives in its own issue so we weigh it
deliberately.

---

*This article accompanies the v0.3.0 release. The implementation is
in the `ljh-sh/zhhz` repo; design discussions and the open defects
(FMM #475, compact trie) live in the private `ljh-sh/mneme` repo.*
