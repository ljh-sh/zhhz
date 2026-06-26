//! WebAssembly bindings for `zhhz`.
//!
//! Built when the `wasm` Cargo feature is enabled
//! (`cargo build --target wasm32-unknown-unknown --features wasm`).
//! Exposes the conversion core and `detect` to JavaScript / Node.js /
//! browsers as plain functions. The OpenCC dictionaries are baked into
//! the `.wasm` at compile time (via `include_str!`), so a single
//! `import { convert } from './zhhz.js'` is enough — no data directory
//! to ship alongside.

use wasm_bindgen::prelude::*;

use crate::detect;
use crate::engine::{Config, Converter};

/// Convert `text` from a script variant to another using one of the
/// built-in OpenCC configs (`s2t`, `t2s`, `s2twp`, ...). See the
/// full list with `zhhz --list` or the README.
#[wasm_bindgen]
pub fn convert(text: &str, config: &str) -> Result<String, JsError> {
    let cfg = Config::parse(config).map_err(|e| JsError::new(&e))?;
    Ok(Converter::new(cfg).convert(text))
}

/// Convert with user-supplied custom words, injected at the highest
/// priority (same semantics as the CLI's `--dict`). `entries` is a JS
/// array of `[key, value]` pairs (e.g. `[["软件", "軟體"]]`).
#[wasm_bindgen]
pub fn convert_with_custom(
    text: &str,
    config: &str,
    entries: Vec<JsValue>,
) -> Result<String, JsError> {
    let cfg = Config::parse(config).map_err(|e| JsError::new(&e))?;
    let mut pairs: Vec<(String, String)> = Vec::with_capacity(entries.len());
    for e in entries {
        let pair: js_sys::Array = e
            .dyn_into()
            .map_err(|_| JsError::new("expected [key, value] pair"))?;
        let key = pair
            .get(0)
            .as_string()
            .ok_or_else(|| JsError::new("key must be a string"))?;
        let value = pair
            .get(1)
            .as_string()
            .ok_or_else(|| JsError::new("value must be a string"))?;
        pairs.push((key, value));
    }
    Ok(Converter::with_custom(cfg, &pairs).convert(text))
}

/// A detected script variant. `region` is one of the codes listed in
/// `zhhz --list` (`cn-s`, `cn-t`, `cn-tw`, `cn-hk`, `jp-n`, `jp-t`).
/// `confidence` is 0–100.
#[wasm_bindgen]
pub struct Detection {
    /// `getter_with_clone` so the JS-side `detection.region` getter clones
    /// the `String` out (required because `String` is not `Copy`).
    #[wasm_bindgen(getter_with_clone)]
    pub region: String,
    pub confidence: u8,
}

/// Identify the script variant of `text`. Returns `null` when there are
/// no CJK characters or kana.
#[wasm_bindgen]
pub fn detect(text: &str) -> Option<Detection> {
    detect::detect_text(text).map(|d| Detection {
        region: d.region.code().to_string(),
        confidence: d.confidence,
    })
}
