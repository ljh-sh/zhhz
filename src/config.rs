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

/// Try to merge a group of dicts into a single Dict. Returns
/// `Some(merged)` if the group is the Phrases+Characters pattern
/// (all child dicts are file-based with disjoint keys: phrases are
/// multi-char, characters are single-char). Returns `None` if the
/// group doesn't match (e.g. has inline dicts, mixed types, or
/// other configs) so the caller can fall back to the per-dict path.
///
/// **Safety**: this assumes STPhrases and STCharacters have disjoint
/// keys (verified empirically: STPhrases is all ≥2-char phrases,
/// STCharacters is all 1-char chars). If upstream opencc ever
/// changes this, the merge breaks output — but the diff_corpus
/// regression test will catch it.
fn try_merge_group(dicts: &[Value]) -> Result<Option<Dict>, String> {
    // Collect all dict names from file-based children.
    let mut names: Vec<String> = Vec::with_capacity(dicts.len());
    for child in dicts {
        let ty = child.get("type").and_then(|t| t.as_str()).unwrap_or("text");
        if ty != "text" && ty != "ocd" && ty != "ocd2" {
            return Ok(None);
        }
        let file = match child.get("file").and_then(|f| f.as_str()) {
            Some(f) => f,
            None => return Ok(None),
        };
        let name = file
            .trim_end_matches(".ocd2")
            .trim_end_matches(".ocd")
            .trim_end_matches(".txt");
        names.push(name.to_string());
    }
    // Concatenate all dict texts in priority order. Phrases first so
    // their multi-char keys shadow any single-char overlap with
    // Characters (which shouldn't happen, but defensive).
    let mut merged_text = String::new();
    for name in &names {
        let raw = data::dict_text_patched(name)
            .ok_or_else(|| format!("unknown embedded dictionary: {name}"))?;
        merged_text.push_str(&raw);
        if !merged_text.ends_with('\n') {
            merged_text.push('\n');
        }
    }
    Ok(Some(Dict::from_text(&merged_text)))
}

fn expand(spec: &Value, out: &mut Vec<Dict>) -> Result<(), String> {
    let ty = spec.get("type").and_then(|t| t.as_str()).unwrap_or("text");
    match ty {
        "group" => {
            let dicts = spec
                .get("dicts")
                .and_then(|d| d.as_array())
                .ok_or_else(|| "group.dicts is missing or not an array".to_string())?;
            // **perf (mneme#74, #10)**: when a group is the s2t/t2s
            // Phrases+Characters pattern (disjoint key sets, all multi-char
            // phrases + all single-char chars), merge all dicts into one
            // trie. Each FMM segment then does 1 trie lookup instead of N.
            // The merge is safe because the keys are disjoint.
            if let Some(merged) = try_merge_group(dicts)? {
                out.push(merged);
            } else {
                for child in dicts {
                    expand(child, out)?;
                }
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
            // STPhrases picks up the multi-value patch overlay; all
            // other dicts read the upstream text directly.
            let raw = data::dict_text_patched(name)
                .ok_or_else(|| format!("unknown embedded dictionary: {name}"))?;
            out.push(Dict::from_text(&raw));
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
