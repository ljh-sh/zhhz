# Contributing

Thanks for your interest in `zhhz`! This project follows the conventions of the
[`ljh-sh`](https://github.com/ljh-sh) Rust tools (`roff`, `fmeta`, `chardet`).

## Development

```bash
git clone https://github.com/ljh-sh/zhhz
cd zhhz
cargo test --all-features
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo fmt -- --check
```

The toolchain is pinned in `rust-toolchain.toml` (`stable-2024-11-28`) for
reproducible builds.

## Adding / changing dictionaries

Dictionary data lives under `data/dictionary/` and `data/config/` and is a **pure
mirror** of [BYVoid/OpenCC](https://github.com/BYVoid/OpenCC). Do not hand-edit
these files. To pull the latest upstream data:

```bash
scripts/sync-opencc.sh          # master HEAD
scripts/sync-opencc.sh 1.3.1    # a tag or commit
```

The five build-time-generated dictionaries (reversed variant tables, the
tofu-risk subset, the regional-phrase projection) are produced by `build.rs` from
the vendored source data — never commit generated files.

## Pull requests

- Branch from `main` (`feat/...`, `fix/...`); keep linear history (squash or
  rebase merges only).
- `cargo fmt`, `cargo clippy -D warnings`, and `cargo test` must pass.
- Conversion-correctness changes should include a test pinning the expected
  output.
