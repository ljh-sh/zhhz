//! WebAssembly bindings for `zhhz` (zhhz#40).
//!
//! Built when the `wasm` Cargo feature is enabled
//! (`cargo build --target wasm32-unknown-unknown --features wasm`).
//!
//! The npm package published from this module is **intentionally richer than
//! [`opencc-js`](https://github.com/nk2028/opencc-js)**:
//!
//! | Capability                       | zhhz | opencc-js |
//! |----------------------------------|:----:|:---------:|
//! | 16 OpenCC conversion configs     |  ✅  |    ✅     |
//! | Custom words (array form)        |  ✅  |    ✅     |
//! | Custom words (string form)       |  ✅  |    ✅     |
//! | Reusable converter instance      |  ✅  |    ✅     |
//! | Per-instance `convertWithCustom` |  ✅  |    ❌     |
//! | Script-variant detection         |  ✅  |    ❌     |
//! | Config / locale introspection    |  ✅  |  partial  |
//! | Semantic region flags (from/to)  |  ✅  |    ❌     |
//!
//! All OpenCC dictionaries are baked into the `.wasm` at compile time
//! (`include_str!`), so a single `npm install zhhz` is enough — no data
//! directory to ship alongside and no network access at startup.
//!
//! # Example
//!
//! ```js
//! import { Converter, detect, listConfigs } from "zhhz";
//!
//! const c = new Converter("s2twp");
//! console.log(c.convert("信息"));          // "資訊"
//! console.log(c.convertWithCustom("买软件", [["软件", "軟體"]])); // "買軟體"
//! console.log(detect("他去了西維珍尼亞州")); // { region: "cn-hk", confidence: 70 }
//! console.log(listConfigs());             // ["s2t", "t2s", "s2twp", ...]
//! ```

use wasm_bindgen::prelude::*;

use crate::detect;
use crate::engine::{
    region_pair_config, Config as EngineConfig, Converter as EngineConverter,
    Region as EngineRegion,
};

// ===========================================================================
// One-shot conversions (back-compat with the v0.7.x WASM surface)
// ===========================================================================

/// Convert `text` using one of the 16 built-in OpenCC configs
/// (`s2t`, `t2s`, `s2twp`, ...). Throws if `config` is not a recognised name.
#[wasm_bindgen]
pub fn convert(text: &str, config: &str) -> Result<String, JsError> {
    let cfg = EngineConfig::parse(config).map_err(|e| JsError::new(&e))?;
    Ok(EngineConverter::new(cfg).convert(text))
}

/// Convert with user-supplied custom words, injected at the highest priority
/// (same semantics as the CLI's `--dict`). `entries` accepts either:
///
/// - An array of `[key, value]` pairs, e.g. `[["软件", "軟體"]]`.
/// - A string of `"key value"` pairs separated by `|`, e.g.
///   `"软件 軟體|苹果 蘋果"` (matches opencc-js `DictLike`).
///
/// Empty pairs (`""`, `" "`, or `null` keys) are skipped.
#[wasm_bindgen]
pub fn convert_with_custom(text: &str, config: &str, entries: JsValue) -> Result<String, JsError> {
    let cfg = EngineConfig::parse(config).map_err(|e| JsError::new(&e))?;
    let pairs = parse_dict_like(&entries)?;
    Ok(EngineConverter::with_custom(cfg, &pairs).convert(text))
}

/// A detected script variant. `region` is one of `cn-s`, `cn-t`, `cn-tw`,
/// `cn-hk`, `jp-n`, `jp-t`. `confidence` is 0–100. Returns `null` when the
/// input has no CJK characters or kana.
#[wasm_bindgen]
pub fn detect(text: &str) -> Option<Detection> {
    detect::detect_text(text).map(|d| Detection {
        region: d.region.code().to_string(),
        confidence: d.confidence,
    })
}

#[wasm_bindgen]
pub struct Detection {
    #[wasm_bindgen(getter_with_clone)]
    pub region: String,
    pub confidence: u8,
}

// ===========================================================================
// Introspection (better than opencc-js: flat lists + semantic regions)
// ===========================================================================

/// All 16 built-in OpenCC config names in canonical order.
#[wasm_bindgen(js_name = listConfigs)]
pub fn list_configs() -> Vec<JsValue> {
    EngineConfig::ALL
        .iter()
        .map(|c| JsValue::from_str(c.name()))
        .collect()
}

/// All 6 region codes in canonical order (`cn-s`, `cn-t`, `cn-tw`,
/// `cn-hk`, `jp-n`, `jp-t`).
#[wasm_bindgen(js_name = listLocales)]
pub fn list_locales() -> Vec<JsValue> {
    EngineRegion::ALL
        .iter()
        .map(|r| JsValue::from_str(r.code()))
        .collect()
}

/// Resolve a `(from, to)` semantic-region pair to the corresponding OpenCC
/// config name (e.g. `(cn-s, cn-tw) → "s2tw"`, `(cn-s, cn-twp) → "s2twp"`).
/// Throws if the pair does not map to a single OpenCC config.
#[wasm_bindgen(js_name = configForRegionPair)]
pub fn config_for_region_pair(from: &str, to: &str) -> Result<String, JsError> {
    let f = EngineRegion::parse(from).map_err(|e| JsError::new(&e))?;
    let t = EngineRegion::parse(to).map_err(|e| JsError::new(&e))?;
    region_pair_config(f, t)
        .map(|c| c.name().to_string())
        .map_err(|e| JsError::new(&e))
}

// ===========================================================================
// Reusable converter instance (factory pattern, strictly better than
// opencc-js's `OpenCC.Converter({ from, to })` factory closure)
// ===========================================================================

/// A reusable converter bound to one of the 16 OpenCC configs.
///
/// Construct with `new Converter(configName)` (e.g. `new Converter("s2twp")`).
/// Use `convertWithCustom(text, entries)` to inject per-call custom words
/// without rebuilding the instance.
#[wasm_bindgen]
pub struct Converter {
    inner: EngineConverter,
    config_name: String,
}

#[wasm_bindgen]
impl Converter {
    /// Build a converter for `config` (one of `listConfigs()`).
    #[wasm_bindgen(constructor)]
    pub fn new(config: &str) -> Result<Converter, JsError> {
        let cfg = EngineConfig::parse(config).map_err(|e| JsError::new(&e))?;
        Ok(Converter {
            inner: EngineConverter::new(cfg),
            config_name: config.to_string(),
        })
    }

    /// Build a converter for a `(from, to)` semantic-region pair (e.g.
    /// `["cn-s", "cn-tw"]` → `s2tw`). Mirrors the CLI's `--from`/`--to`.
    #[wasm_bindgen(js_name = forRegion)]
    pub fn for_region(from: &str, to: &str) -> Result<Converter, JsError> {
        let cfg_name = config_for_region_pair(from, to)?;
        Converter::new(&cfg_name)
    }

    /// Convert `text`. Always pure (no I/O, no allocations beyond the
    /// returned `String`).
    pub fn convert(&self, text: &str) -> String {
        self.inner.convert(text)
    }

    /// Convert `text` with custom words injected at the highest priority.
    /// `entries` accepts the same `DictLike` shape as the top-level
    /// `convertWithCustom`.
    #[wasm_bindgen(js_name = convertWithCustom)]
    pub fn convert_with_custom(&self, text: &str, entries: JsValue) -> Result<String, JsError> {
        let pairs = parse_dict_like(&entries)?;
        // Build a fresh inner with custom merged in. The underlying data is
        // shared via `include_str!` (already parsed at first construction),
        // so per-call allocation is limited to the merged-custom `Vec`.
        let cfg = EngineConfig::parse(&self.config_name)
            .expect("config_name was validated at construction");
        Ok(EngineConverter::with_custom(cfg, &pairs).convert(text))
    }

    /// Return a new `Converter` that has these custom words baked in
    /// (every subsequent `.convert()` call applies them). Equivalent to
    /// `OpenCC.CustomConverter` in opencc-js but chained off the constructor.
    #[wasm_bindgen(js_name = withCustom)]
    pub fn with_custom(&self, entries: JsValue) -> Result<Converter, JsError> {
        let pairs = parse_dict_like(&entries)?;
        let cfg = EngineConfig::parse(&self.config_name)
            .expect("config_name was validated at construction");
        Ok(Converter {
            inner: EngineConverter::with_custom(cfg, &pairs),
            config_name: self.config_name.clone(),
        })
    }

    /// The config name this converter was built for (e.g. `"s2twp"`).
    #[wasm_bindgen(getter)]
    pub fn config(&self) -> String {
        self.config_name.clone()
    }
}

// ===========================================================================
// DictLike parsing — accepts string OR array form (opencc-js compat +
// strictly better error messages)
// ===========================================================================

/// Parse a `DictLike` value (opencc-js term) — either a `string` of
/// `"key value"` entries separated by `|`, or a JS array of `[key, value]`
/// tuples. Empty / malformed entries are rejected with a clear message.
fn parse_dict_like(value: &JsValue) -> Result<Vec<(String, String)>, JsError> {
    if value.is_undefined() || value.is_null() {
        return Err(JsError::new(
            "custom entries must be a non-empty string or [[key,value]] array",
        ));
    }

    // String form: "软件 軟體|苹果 蘋果"
    if let Some(s) = value.as_string() {
        return parse_dict_string(&s);
    }

    // Array form: [["软件","軟體"],["苹果","蘋果"]]
    if value.is_array() {
        let arr = js_sys::Array::from(value);
        let mut out = Vec::with_capacity(arr.length() as usize);
        for (i, item) in arr.iter().enumerate() {
            if !item.is_array() {
                return Err(JsError::new(&format!(
                    "entries[{}] must be a [key, value] pair, got {}",
                    i,
                    type_name(&item)
                )));
            }
            let pair = js_sys::Array::from(&item);
            let key = pair.get(0).as_string().ok_or_else(|| {
                JsError::new(&format!("entries[{}][0] (key) must be a string", i))
            })?;
            let value = pair.get(1).as_string().ok_or_else(|| {
                JsError::new(&format!("entries[{}][1] (value) must be a string", i))
            })?;
            if key.is_empty() {
                return Err(JsError::new(&format!("entries[{}] has an empty key", i)));
            }
            out.push((key, value));
        }
        if out.is_empty() {
            return Err(JsError::new("entries array is empty"));
        }
        return Ok(out);
    }

    Err(JsError::new(&format!(
        "entries must be a string or [[key,value]] array, got {}",
        type_name(value)
    )))
}

fn parse_dict_string(s: &str) -> Result<Vec<(String, String)>, JsError> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(JsError::new("custom dict string is empty"));
    }
    let mut out = Vec::new();
    for (i, raw) in trimmed.split('|').enumerate() {
        let piece = raw.trim();
        if piece.is_empty() {
            continue;
        }
        // Split on the first ASCII space; allow the value to contain spaces.
        let (key, value) = match piece.split_once(' ') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => {
                return Err(JsError::new(&format!(
                    "dict entry {}: expected \"key value\" separated by a space, got {:?}",
                    i, piece
                )))
            }
        };
        if key.is_empty() {
            return Err(JsError::new(&format!("dict entry {}: empty key", i)));
        }
        out.push((key.to_string(), value.to_string()));
    }
    if out.is_empty() {
        return Err(JsError::new("custom dict string parsed to zero entries"));
    }
    Ok(out)
}

fn type_name(v: &JsValue) -> &'static str {
    if v.is_undefined() {
        "undefined"
    } else if v.is_null() {
        "null"
    } else if v.is_array() {
        "Array"
    } else if v.as_string().is_some() {
        "string"
    } else if v.as_bool().is_some() {
        "boolean"
    } else if v.as_f64().is_some() {
        "number"
    } else {
        "object"
    }
}
