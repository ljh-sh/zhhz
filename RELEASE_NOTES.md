# Release v0.8.2 — bundled release notes

This release contains three sequential feature releases:

## v0.8.0 — detect TW/HK classifier tuning
- `zhhz detect` TW/HK classifier now handles shared regional phrases
  (`滑鼠`, `伊利諾` are used in both regions). Default to TW when
  only shared regional-traditional phrases are present.

## v0.8.1 — `--auto --target <REGION>`
- New `--target` flag for `--auto` mode. Configurable destination
  region; default remains cn-s. Clear errors for unsupported combos.

## v0.8.2 — `zhhz info <CONFIG>`
- New introspection subcommand. Prints config name, description,
  source/target region, and a small input/output example.
