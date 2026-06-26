//! OpenCC config parsing and resolution.
//!
//! A config describes a pipeline:
//!
//! ```text
//! text --[segmentation (FMM)]--> segments --[conversion_chain stage 1]--> ...
//!      ... --[stage N]--> output
//! ```
//!
//! Each stage and the segmentation step use a *dictionary group* — an ordered
//! list of dictionaries where earlier members have higher priority. Custom user
//! words are injected as the highest-priority member of every group so they
//! override the built-in tables.

use serde_json::Value;

use crate::data;
use crate::dict::Dict;

/// A fully resolved conversion config: an optional segmentation group plus an
/// ordered list of conversion stages (each a dictionary group).
pub struct ResolvedConfig {
    pub segmentation: Option<Vec<Dict>>,
    pub chain: Vec<Vec<Dict>>,
}

/// Resolve an OpenCC config JSON string into dictionaries, injecting `custom`
/// words as the highest-priority member of every group.
pub fn resolve(json: &str, custom: &[(String, String)]) -> Result<ResolvedConfig, String> {
    let root: Value =
        serde_json::from_str(json).map_err(|e| format!("invalid config JSON: {e}"))?;

    let segmentation = match root.get("segmentation") {
        Some(seg) => {
            let dict = seg
                .get("dict")
                .ok_or_else(|| "segmentation.dict is missing".to_string())?;
            Some(resolve_group(dict, custom)?)
        }
        None => None,
    };

    let chain = root
        .get("conversion_chain")
        .and_then(|c| c.as_array())
        .ok_or_else(|| "conversion_chain is missing or not an array".to_string())?;
    let mut stages = Vec::with_capacity(chain.len());
    for stage in chain {
        let dict = stage
            .get("dict")
            .ok_or_else(|| "a conversion_chain stage is missing its dict".to_string())?;
        stages.push(resolve_group(dict, custom)?);
    }

    Ok(ResolvedConfig {
        segmentation,
        chain: stages,
    })
}

/// Resolve a dictionary spec (single dict, group, or inline) into a priority
/// group, with `custom` words prepended.
fn resolve_group(spec: &Value, custom: &[(String, String)]) -> Result<Vec<Dict>, String> {
    let mut group: Vec<Dict> = Vec::new();
    if !custom.is_empty() {
        group.push(Dict::from_entries(custom));
    }
    expand(spec, &mut group)?;
    Ok(group)
}

fn expand(spec: &Value, out: &mut Vec<Dict>) -> Result<(), String> {
    let ty = spec.get("type").and_then(|t| t.as_str()).unwrap_or("text");
    match ty {
        "group" => {
            let dicts = spec
                .get("dicts")
                .and_then(|d| d.as_array())
                .ok_or_else(|| "group.dicts is missing or not an array".to_string())?;
            for child in dicts {
                expand(child, out)?;
            }
        }
        // OpenCC ships `.ocd2` (marisa-trie) binaries; zhhz embeds the `.txt`
        // sources instead, so all file types resolve to embedded text.
        "text" | "ocd" | "ocd2" => {
            let file = spec
                .get("file")
                .and_then(|f| f.as_str())
                .ok_or_else(|| "file-based dict is missing its file name".to_string())?;
            let name = file
                .trim_end_matches(".ocd2")
                .trim_end_matches(".ocd")
                .trim_end_matches(".txt");
            let raw = data::dict_text(name)
                .ok_or_else(|| format!("unknown embedded dictionary: {name}"))?;
            out.push(Dict::from_text(raw));
        }
        "inline" => {
            let entries = spec
                .get("entries")
                .and_then(|e| e.as_object())
                .ok_or_else(|| "inline.entries is missing or not an object".to_string())?;
            let mut pairs = Vec::with_capacity(entries.len());
            for (key, val) in entries {
                let first = val
                    .as_str()
                    .unwrap_or("")
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .to_string();
                pairs.push((key.clone(), first));
            }
            out.push(Dict::from_entries(&pairs));
        }
        other => return Err(format!("unsupported dictionary type: {other}")),
    }
    Ok(())
}
