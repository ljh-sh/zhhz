//! Build script: derive OpenCC's build-time-generated dictionaries from the
//! vendored source data and emit them into `OUT_DIR`.
//!
//! `data/dictionary/*.txt` is a pure upstream mirror of BYVoid/OpenCC. Five
//! files are *generated* by OpenCC's build system (see `data/scripts/` and
//! `data/CMakeLists.txt`) and are reconstructed here so that every shipped
//! config resolves, with no Python/CMake at build time:
//!
//! | generated                              | from                       | rule            |
//! |----------------------------------------|----------------------------|-----------------|
//! | `TSCharactersExt.txt`                  | `TSCharacters.txt`         | extract_tofu_risk |
//! | `TWVariantsRev.txt`                    | `TWVariants.txt`           | reverse         |
//! | `HKVariantsRev.txt`                    | `HKVariants.txt`           | reverse         |
//! | `JPShinjitaiCharactersRev.txt`         | `JPShinjitaiCharacters.txt`| reverse         |
//! | `STPhrases_GeneratedFromRegionalPhrases.txt` | `HKPhrases.txt` + `TWPhrases.txt` keys | t2s projection |
//!
//! The reverse and tofu-risk transforms are deterministic pure functions
//! (mirror `data/scripts/common.py::reverse_items` and `extract_tofu_risk.py`).
//! The ST-phrases projection needs a t2s conversion; since `t2s.json` is a
//! single-stage chain, projecting a key equals one `convert_segment` pass over
//! the t2s group `[TSPhrases, TSCharactersExt, TSCharacters]` (priority order),
//! which is reproduced below with a minimal trie.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let dict_dir = manifest.join("data").join("dictionary");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let tscharacters = read(&dict_dir.join("TSCharacters.txt"));

    // 1. TSCharactersExt <- extract_tofu_risk(TSCharacters.txt)
    let tscharacters_ext = extract_tofu_risk(&tscharacters);
    write(&out_dir.join("TSCharactersExt.txt"), &tscharacters_ext);

    // 2. reverse variant dictionaries
    for (src, dst) in [
        ("TWVariants", "TWVariantsRev"),
        ("HKVariants", "HKVariantsRev"),
        ("JPShinjitaiCharacters", "JPShinjitaiCharactersRev"),
    ] {
        let text = read(&dict_dir.join(format!("{src}.txt")));
        write(&out_dir.join(format!("{dst}.txt")), &reverse_dict(&text));
    }

    // 3. STPhrases_GeneratedFromRegionalPhrases <- t2s(HKPhrases + TWPhrases keys)
    let tsphrases = build_trie(&parse_simple(&read(&dict_dir.join("TSPhrases.txt"))));
    let tschars_ext_trie = build_trie(&parse_simple(&tscharacters_ext));
    let tschars_trie = build_trie(&parse_simple(&tscharacters));
    // t2s.json chain = group[TSPhrases, TSCharactersExt, TSCharacters] (priority order)
    let t2s_group: [&Trie; 3] = [&tsphrases, &tschars_ext_trie, &tschars_trie];
    let st_gen = generate_st_phrases(&dict_dir, &t2s_group);
    write(
        &out_dir.join("STPhrases_GeneratedFromRegionalPhrases.txt"),
        &st_gen,
    );

    // Re-run if any source dictionary changes.
    println!("cargo:rerun-if-changed=build.rs");
    if let Ok(entries) = fs::read_dir(&dict_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("txt") {
                println!("cargo:rerun-if-changed={}", p.display());
            }
        }
    }
}

fn read(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("build.rs: failed to read {}: {e}", path.display()))
}

fn write(path: &Path, contents: &str) {
    fs::write(path, contents)
        .unwrap_or_else(|e| panic!("build.rs: failed to write {}: {e}", path.display()));
}

/// Parse a `.txt` dictionary into `(key, first_value)` pairs.
/// Comments (`#`) and blank lines are skipped; values beyond the first
/// candidate are dropped (conversion only ever emits the first candidate).
fn parse_simple(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, vals) = line.split_once('\t').unwrap_or((line, ""));
        let first = vals.split_whitespace().next();
        if let Some(v) = first {
            out.push((key.to_string(), v.to_string()));
        }
    }
    out
}

/// Reproduce `data/scripts/extract_tofu_risk.py`: every line starting with
/// `# @tofu-risk:` is followed by a mapping `key\tv1 v2 ...`; emit
/// `key\tv2 ...` (drop the identity first candidate when it equals the key).
fn extract_tofu_risk(text: &str) -> String {
    const PREFIX: &str = "# @tofu-risk:";
    let lines: Vec<&str> = text.lines().collect();
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        if !line.starts_with(PREFIX) {
            continue;
        }
        let mapping = lines.get(i + 1).copied().unwrap_or_else(|| {
            panic!(
                "build.rs: missing mapping after @tofu-risk at line {}",
                i + 1
            )
        });
        let mapping = mapping.trim_end_matches('\r');
        if mapping.trim().is_empty() || mapping.starts_with('#') {
            panic!(
                "build.rs: expected mapping after @tofu-risk at line {}",
                i + 1
            );
        }
        let (key, vals) = mapping.split_once('\t').unwrap_or((mapping, ""));
        let mut values: Vec<&str> = vals.split_whitespace().collect();
        if values.first() == Some(&key) {
            values.remove(0);
        }
        if values.is_empty() {
            panic!(
                "build.rs: empty extension mapping after @tofu-risk at line {}",
                i + 1
            );
        }
        out.push_str(key);
        out.push('\t');
        out.push_str(&values.join(" "));
        out.push('\n');
    }
    out
}

/// Reproduce `data/scripts/common.py::reverse_items` (Dict.swap + sort + dump):
/// build `value -> [keys]` in encounter order, apply `# @reverse-prefer:`
/// annotations, sort by the (reversed) key, and emit `key\tv1 v2 ...\n`.
fn reverse_dict(text: &str) -> String {
    const PREF: &str = "# @reverse-prefer:";
    let mut entries: Vec<(String, Vec<String>)> = Vec::new();
    let mut prefs: HashMap<String, String> = HashMap::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix(PREF) {
            let fields: Vec<&str> = rest.split_whitespace().collect();
            if !fields.is_empty() {
                prefs.insert(fields[0].to_string(), fields[fields.len() - 1].to_string());
            }
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, vals) = line.split_once('\t').unwrap_or((line, ""));
        let values: Vec<String> = vals.split_whitespace().map(String::from).collect();
        if !values.is_empty() {
            entries.push((key.to_string(), values));
        }
    }

    // value -> Vec<keys> preserving first-seen order of values
    let mut dic: Vec<(String, Vec<String>)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for (key, values) in &entries {
        for value in values {
            if let Some(&i) = index.get(value) {
                dic[i].1.push(key.clone());
            } else {
                let i = dic.len();
                index.insert(value.clone(), i);
                dic.push((value.clone(), vec![key.clone()]));
            }
        }
    }

    // apply reverse preferences: move the preferred original key to the front
    for (rev_key, preferred) in &prefs {
        if let Some(&i) = index.get(rev_key) {
            let vals = &mut dic[i].1;
            if let Some(pos) = vals.iter().position(|v| v == preferred) {
                let item = vals.remove(pos);
                vals.insert(0, item);
            }
        }
    }

    // sort by reversed key (str byte order == Unicode codepoint order)
    dic.sort_by(|a, b| a.0.cmp(&b.0));

    let mut out = String::new();
    out.push_str("# Open Chinese Convert (OpenCC) Dictionary (generated, reversed)\n");
    out.push_str("# Format: key\tvalue(s) (values separated by spaces)\n");
    out.push_str("# License: Apache-2.0 (see LICENSE)\n");
    out.push_str("# Source: generated by zhhz build.rs from the corresponding variants file\n");
    out.push('\n');
    for (key, values) in &dic {
        out.push_str(key);
        out.push('\t');
        out.push_str(&values.join(" "));
        out.push('\n');
    }
    out
}

/// Reproduce `data/scripts/generate_st_phrases_from_regional_phrases.py`:
/// project each regional phrase key through t2s, and for projections of
/// length >= 3 chars emit `simplified_key -> regional_key` (first wins,
/// conflicting projections abort the build).
fn generate_st_phrases(dict_dir: &Path, t2s_group: &[&Trie]) -> String {
    let mut map: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    for input in ["HKPhrases.txt", "TWPhrases.txt"] {
        for (key, _value) in parse_simple(&read(&dict_dir.join(input))) {
            let converted = convert_segment(t2s_group, &key);
            if converted.chars().count() < 3 {
                continue;
            }
            if let Some(existing) = map.get(&converted) {
                assert_eq!(
                    existing, &key,
                    "build.rs: conflicting ST phrase projection for {converted}: {existing} vs {key}"
                );
            } else {
                map.insert(converted, key);
            }
        }
    }

    let mut out = String::new();
    out.push_str("# Open Chinese Convert (OpenCC) Dictionary\n");
    out.push_str("# File: STPhrases_GeneratedFromRegionalPhrases.txt\n");
    out.push_str("# Format: key\tvalue(s) (values separated by spaces)\n");
    out.push_str("# License: Apache-2.0 (see LICENSE)\n");
    out.push_str("# Source: generated from HKPhrases.txt, TWPhrases.txt keys via t2s.json\n");
    out.push_str("# Used in configs: s2t.json, s2hk.json, s2hkp.json, s2tw.json, s2twp.json\n");
    out.push_str("#\n");
    out.push_str("# This generated ST phrase dictionary preserves Simplified-input spans\n");
    out.push_str("# before applying regional phrase vocabulary.\n");
    out.push('\n');
    for (converted, original) in &map {
        out.push_str(converted);
        out.push('\t');
        out.push_str(original);
        out.push('\n');
    }
    out
}

// --- minimal trie + conversion (mirrors src/dict.rs semantics) -------------

#[derive(Default)]
struct Node {
    children: HashMap<char, Node>,
    value: Option<String>,
}

struct Trie {
    root: Node,
}

fn build_trie(entries: &[(String, String)]) -> Trie {
    let mut root = Node::default();
    for (key, value) in entries {
        let mut node = &mut root;
        for ch in key.chars() {
            node = node.children.entry(ch).or_default();
        }
        node.value = Some(value.clone());
    }
    Trie { root }
}

impl Trie {
    /// Longest key that is a prefix of `text`; returns `(matched byte len, value)`.
    fn longest_prefix(&self, text: &str) -> Option<(usize, &str)> {
        let mut node = &self.root;
        let mut best: Option<(usize, &str)> = None;
        for (i, ch) in text.char_indices() {
            match node.children.get(&ch) {
                Some(child) => {
                    node = child;
                    if let Some(v) = &node.value {
                        best = Some((i + ch.len_utf8(), v.as_str()));
                    }
                }
                None => break,
            }
        }
        best
    }
}

/// Group longest-prefix: the first (highest-priority) trie with any prefix
/// match wins, returning that trie's longest prefix. Equivalent to OpenCC's
/// flattened `PrefixMatch` table (min dictOrder, then max length).
fn group_longest_prefix<'a>(group: &[&'a Trie], text: &str) -> Option<(usize, &'a str)> {
    for trie in group {
        if let Some(m) = trie.longest_prefix(text) {
            return Some(m);
        }
    }
    None
}

/// One conversion stage over a segment: longest-prefix per position, emitting
/// the first candidate on match or copying one char through on miss.
fn convert_segment(group: &[&Trie], seg: &str) -> String {
    let mut out = String::with_capacity(seg.len() + seg.len() / 5);
    let mut pos = 0;
    while pos < seg.len() {
        let rest = &seg[pos..];
        if let Some((key_len, value)) = group_longest_prefix(group, rest) {
            out.push_str(value);
            pos += key_len;
        } else {
            let ch_len = rest
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or(rest.len().min(4));
            out.push_str(&rest[..ch_len]);
            pos += ch_len;
        }
    }
    out
}
