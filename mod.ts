// JSR (@ljh-sh/zhhz) re-exports of zhhz.
//
// zhhz's conversion core is a WebAssembly module published to npm.
// JSR doesn't host native WASM blobs the way npm does, so the
// canonical home for the .wasm is npm:zhhz@<version>. This JSR
// package is a thin facade that re-exports the npm API, so Deno
// users can `import { ... } from "jsr:@ljh-sh/zhhz"` without
// a deno.json "imports" map.
//
// The exports here mirror src/wasm.rs in the npm package; if you
// add a new Rust-side function there, add a corresponding re-export
// here in the same release.

export {
  convert,
  convert_with_custom,
  detect,
  listConfigs,
  listLocales,
  configForRegionPair,
  Converter,
  Detection,
} from "npm:zhhz@0.7.9";

// Re-export the version so consumers can pin programmatically.
export const VERSION = "0.7.9";
