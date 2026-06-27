//! zhhz — a self-contained Simplified/Traditional Chinese converter.
//!
//! `zhhz` is a pure-Rust reimplementation of [OpenCC](https://github.com/BYVoid/OpenCC).
//! All OpenCC dictionaries and configs are embedded in the binary at compile
//! time, so there is no runtime download and no separate data directory: one
//! static binary, one conversion call.
//!
//! The name is a palindrome: **zh** hanzi / **z**huan **h**uan **h**an **z**i.
//!
//! # Example
//!
//! ```no_run
//! use zhhz::{Config, Converter};
//!
//! let c = Converter::new(Config::S2t);
//! assert_eq!(c.convert("汉字"), "漢字");
//!
//! // Custom words override the built-in tables at the highest priority.
//! let c = Converter::with_custom(Config::S2t, &[("打印机".into(), "印表機".into())]);
//! assert_eq!(c.convert("买一台打印机"), "買一台印表機");
//! ```

pub mod cli;
pub mod config;
pub mod data;
pub mod detect;
pub mod dict;
pub mod engine;
pub mod ngram;

#[cfg(all(feature = "marisa", not(feature = "wasm")))]
pub mod dict_marisa;

#[cfg(feature = "wasm")]
pub mod wasm;

// When the `marisa` feature is on, the public `Dict` is the rsmarisa-backed
// implementation. Engine + config reference `Dict` and are unaware of the
// swap. WASM always uses the in-house trie (rsmarisa would link extra code
// into the WASM blob; we keep WASM lean).
#[cfg(all(feature = "marisa", not(feature = "wasm")))]
pub use dict_marisa::MarisaDict as Dict;
#[cfg(not(all(feature = "marisa", not(feature = "wasm"))))]
pub use dict::Dict;

pub use detect::{detect_bytes, detect_text, Detection};
pub use engine::{Config, Converter, NgramMode, Region};
pub use ngram::NgramModel;

/// The default conversion config used when `--config` is omitted.
pub const DEFAULT_CONFIG: &str = "s2t";

/// Run the CLI and return the top-level result.
pub fn run() -> anyhow::Result<()> {
    cli::run()
}
