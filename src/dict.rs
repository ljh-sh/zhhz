//! Dictionary representation and longest-prefix matching.
//!
//! The hot path for long-text conversion is `longest_prefix`, called once
//! per input position per dictionary in the conversion group. This module
//! optimises for that path with two changes vs the previous HashMap+Box
//! implementation:
//!
//! * **Arena** of nodes (`Vec<Node>`, one allocation for the whole trie)
//!   instead of `Box<Node>` recursion. Eliminates ~250k heap allocations
//!   per process.
//! * **Sorted `Vec<(char, u32)>` children** per node, looked up by
//!   binary search, instead of `HashMap<char, Node>`. Cache-friendly
//!   contiguous layout; no hash overhead per character.
//!
//! Build is two-phase so the root (which has ~3000 children) does not pay
//! O(n) per insertion:
//! 1. Insert with a `HashMap<char, u32>` child map (O(1) amortised insert)
//!    into the arena node.
//! 2. `finalize()` drains each `HashMap` into a sorted `Vec<(char, u32)>`.
//! The query then never touches the build-time HashMap.

use std::collections::HashMap;

#[derive(Default)]
struct Node {
    /// Build-phase child map. Empty in the final (post-`finalize`) trie.
    children_map: HashMap<char, u32>,
    /// Frozen child list (sorted by `char`). Populated by `finalize()`.
    children: Vec<(char, u32)>,
    /// Default (first) candidate value — used by the hot path
    /// (`longest_prefix`) for back-compat and zero-overhead lookup.
    value: Option<String>,
    /// All candidate values (default first). Populated only when the
    /// dictionary line had multiple whitespace-separated values; used by
    /// `longest_prefix_multi` for n-gram disambig.
    candidates: Option<Vec<String>>,
}

pub struct Dict {
    nodes: Vec<Node>,
}

impl Dict {
    /// Build a dictionary from raw OpenCC `.txt` dictionary text.
    ///
    /// Lines are `key<TAB>value` or `key<TAB>value1 value2 ...`. The first
    /// value is the default (used by the hot path). All values are kept for
    /// `longest_prefix_multi` callers (n-gram disambig).
    pub fn from_text(raw: &str) -> Self {
        let mut d = Dict {
            nodes: vec![Node::default()],
        };
        for line in raw.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, vals)) = line.split_once('\t') else {
                continue;
            };
            let mut iter = vals.split_whitespace();
            let Some(first) = iter.next() else {
                continue;
            };
            // Rest of the candidates (may be empty).
            let rest: Vec<String> = iter.map(|s| s.to_string()).collect();
            d.insert(key, first, if rest.is_empty() { None } else { Some(rest) });
        }
        d.finalize();
        d
    }

    /// Build a dictionary from explicit `(key, value)` entries (e.g. user
    /// custom words; later entries win on duplicate keys).
    pub fn from_entries(entries: &[(String, String)]) -> Self {
        let mut d = Dict {
            nodes: vec![Node::default()],
        };
        for (k, v) in entries {
            d.insert(k, v, None);
        }
        d.finalize();
        d
    }

    fn insert(&mut self, key: &str, value: &str, extra: Option<Vec<String>>) {
        let mut cur: u32 = 0;
        for ch in key.chars() {
            // Immutable lookup first; the borrow ends before the create
            // path's `push`, so the borrow checker is happy (no two
            // simultaneous `&mut self` borrows).
            let existing = self.nodes[cur as usize].children_map.get(&ch).copied();
            let child = match existing {
                Some(idx) => idx,
                None => {
                    let idx = self.nodes.len() as u32;
                    self.nodes.push(Node::default());
                    self.nodes[cur as usize].children_map.insert(ch, idx);
                    idx
                }
            };
            cur = child;
        }
        self.nodes[cur as usize].value = Some(value.to_string());
        if let Some(more) = extra {
            let mut all = Vec::with_capacity(1 + more.len());
            all.push(value.to_string());
            all.extend(more);
            self.nodes[cur as usize].candidates = Some(all);
        }
    }

    /// Move each node's `children_map` into a sorted `Vec` and clear the
    /// map. Called once at the end of construction. Cost: O(N) traversal
    /// + O(sum n log n) sorts; negligible vs the O(total_chars) build.
    fn finalize(&mut self) {
        for n in &mut self.nodes {
            let mut v: Vec<(char, u32)> = n.children_map.drain().collect();
            v.sort_unstable_by_key(|(c, _)| *c);
            n.children = v;
        }
    }

    /// Longest key that is a prefix of `text`. Returns the matched byte
    /// length and the default candidate value, or `None` if no key
    /// prefixes `text`.
    pub fn longest_prefix(&self, text: &str) -> Option<(usize, &str)> {
        let mut node: u32 = 0;
        let mut best: Option<(usize, &str)> = None;
        for (i, ch) in text.char_indices() {
            let n = &self.nodes[node as usize];
            // Binary search in the frozen, sorted child slice.
            match n.children.binary_search_by_key(&ch, |(c, _)| *c) {
                Ok(pos) => {
                    let child = n.children[pos].1;
                    if let Some(v) = &self.nodes[child as usize].value {
                        best = Some((i + ch.len_utf8(), v.as_str()));
                    }
                    node = child;
                }
                Err(_) => break,
            }
        }
        best
    }

    /// Longest-prefix match returning **all** candidate values, ordered
    /// default-first. Returns `None` if no key prefixes `text`; returns
    /// `Some((len, single))` for single-candidate matches; returns
    /// `Some((len, all))` for multi-candidate matches where `all.len() > 1`.
    ///
    /// `single` and `all` borrow from the dict; the caller can either
    /// emit the first value (fast path) or call into a language model to
    /// disambiguate.
    pub fn longest_prefix_multi<'a>(
        &'a self,
        text: &str,
    ) -> Option<(usize, Vec<&'a str>)> {
        let mut node: u32 = 0;
        let mut best: Option<(usize, Vec<&'a str>)> = None;
        for (i, ch) in text.char_indices() {
            let n = &self.nodes[node as usize];
            match n.children.binary_search_by_key(&ch, |(c, _)| *c) {
                Ok(pos) => {
                    let child = n.children[pos].1;
                    let cn = &self.nodes[child as usize];
                    if let Some(cands) = &cn.candidates {
                        // Multi-value: collect all.
                        let v: Vec<&str> = cands.iter().map(String::as_str).collect();
                        best = Some((i + ch.len_utf8(), v));
                    } else if let Some(v) = &cn.value {
                        best = Some((i + ch.len_utf8(), vec![v.as_str()]));
                    }
                    node = child;
                }
                Err(_) => break,
            }
        }
        best
    }
}

/// Group longest-prefix: the first (highest-priority) dictionary that has
/// any prefix of `text` wins, returning that dictionary's longest match.
/// Lower-priority longer matches never override a higher-priority shorter
/// match — the priority ordering dominates.
pub fn group_longest_prefix<'a>(group: &'a [Dict], text: &str) -> Option<(usize, &'a str)> {
    for dict in group {
        if let Some(m) = dict.longest_prefix(text) {
            return Some(m);
        }
    }
    None
}

/// Group longest-prefix with multi-value candidates (for n-gram disambig).
pub fn group_longest_prefix_multi<'a>(
    group: &'a [Dict],
    text: &str,
) -> Option<(usize, Vec<&'a str>)> {
    for dict in group {
        if let Some(m) = dict.longest_prefix_multi(text) {
            return Some(m);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_prefix_basic() {
        let d = Dict::from_entries(&[
            ("ab".to_string(), "AB".to_string()),
            ("abc".to_string(), "ABC".to_string()),
        ]);
        assert_eq!(d.longest_prefix("abcd"), Some((3, "ABC")));
        assert_eq!(d.longest_prefix("abx"), Some((2, "AB")));
        assert_eq!(d.longest_prefix("xb"), None);
    }

    #[test]
    fn arena_no_box() {
        // Smoke: a 2k-entry dict still works (and the absence of `Box` in
        // the source guarantees no per-node heap allocations).
        let entries: Vec<(String, String)> = (0..2000)
            .map(|i| (format!("汉字{i}"), format!("漢字{i}")))
            .collect();
        let d = Dict::from_entries(&entries);
        assert_eq!(d.longest_prefix("汉字0尾"), Some((7, "漢字0")));
    }

    #[test]
    fn multi_value_exposed() {
        // 出 -> 出 齣  (multi-value, default first)
        let raw = "出\t出 齣\n";
        let d = Dict::from_text(raw);
        let (len, cands) = d.longest_prefix_multi("出门").unwrap();
        assert_eq!(len, 3);
        assert_eq!(cands, vec!["出", "齣"]);
        // Default path still returns just first
        let (len, first) = d.longest_prefix("出门").unwrap();
        assert_eq!((len, first), (3, "出"));
    }

    #[test]
    fn single_value_no_multi() {
        // Single-value entries should still work and have cands.len() == 1
        let raw = "中\t中\n国\t國\n";
        let d = Dict::from_text(raw);
        let (len, cands) = d.longest_prefix_multi("中国").unwrap();
        assert_eq!(len, 3);
        assert_eq!(cands, vec!["中"]);
    }
}
