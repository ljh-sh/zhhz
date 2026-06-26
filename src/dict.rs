//! Dictionary representation and longest-prefix matching.
//!
//! An OpenCC dictionary is a flat `key -> value(s)` table. Conversion only ever
//! emits the *first* candidate, so a [`Dict`] stores `key -> default_value`.
//! Keys are indexed in a per-character trie so that a longest-prefix query is a
//! single left-to-right descent.
//!
//! The matching semantics are identical to OpenCC's `PrefixMatch`: within a
//! group, the highest-priority dictionary that has *any* prefix of the input
//! wins, and its longest prefix is returned (priority dominates length across
//! dictionaries; length dominates only within a single dictionary). See
//! [`group_longest_prefix`].

use std::collections::HashMap;

#[derive(Default)]
struct Node {
    children: HashMap<char, Node>,
    /// Default candidate (first value) emitted on a match at this node.
    value: Option<String>,
}

/// A conversion/segmentation dictionary supporting longest-prefix lookup.
pub struct Dict {
    root: Node,
}

impl Dict {
    /// Build a dictionary from raw OpenCC `.txt` dictionary text.
    pub fn from_text(raw: &str) -> Self {
        Self::from_text_with(raw, |key, first| Some((key.to_string(), first.to_string())))
    }

    fn from_text_with(
        raw: &str,
        mut keep: impl FnMut(&str, &str) -> Option<(String, String)>,
    ) -> Self {
        let mut root = Node::default();
        for line in raw.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, vals) = match line.split_once('\t') {
                Some(kv) => kv,
                None => continue,
            };
            let Some(first) = vals.split_whitespace().next() else {
                continue;
            };
            let Some((key, first)) = keep(key, first) else {
                continue;
            };
            let mut node = &mut root;
            for ch in key.chars() {
                node = node.children.entry(ch).or_default();
            }
            node.value = Some(first);
        }
        Dict { root }
    }

    /// Build a dictionary from explicit `(key, value)` entries (e.g. user
    /// custom words or inline config entries). Later duplicate keys win.
    pub fn from_entries(entries: &[(String, String)]) -> Self {
        let mut root = Node::default();
        for (key, value) in entries {
            if key.is_empty() {
                continue;
            }
            let mut node = &mut root;
            for ch in key.chars() {
                node = node.children.entry(ch).or_default();
            }
            node.value = Some(value.clone());
        }
        Dict { root }
    }

    /// Longest key that is a prefix of `text`. Returns the matched byte length
    /// and the default value, or `None` if no key prefixes `text`.
    pub fn longest_prefix(&self, text: &str) -> Option<(usize, &str)> {
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

/// Group longest-prefix: return the first (highest-priority) dictionary that has
/// any prefix of `text`, with that dictionary's longest prefix. Lower-priority
/// longer matches never override a higher-priority shorter match.
pub fn group_longest_prefix<'a>(group: &'a [Dict], text: &str) -> Option<(usize, &'a str)> {
    for dict in group {
        if let Some(m) = dict.longest_prefix(text) {
            return Some(m);
        }
    }
    None
}
