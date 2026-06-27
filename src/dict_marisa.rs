//! Dictionary backed by `rsmarisa::Trie` (pure-Rust marisa-trie port).
//!
//! Mirrors `dict::Dict`'s public API so the engine can swap implementations.
//! The trie stores **keys only** (UTF-8 bytes, no NUL). Values are kept in a
//! parallel `Vec<Vec<String>>` indexed by `agent.key().id()` — same design
//! OpenCC uses for its `.ocd2` format (see OpenCC `MarisaDict`).
//!
//! Build cost is paid once at process startup (a few hundred ms for the
//! ~50k-key s2t dict). The hot-path `longest_prefix` is one
//! `common_prefix_search` walk — a single byte-by-byte trie descent, no
//! per-node heap allocation, no binary search.

use rsmarisa::{Agent, Keyset, Trie};

/// Same value layout as `dict::Dict` — single `String` for the default,
/// `Vec<String>` of all candidates (default first) when the line had
/// multi-value form (used by n-gram disambig).
struct Values {
    default: String,
    /// Empty vec == single-value path; non-empty == multi-value path.
    candidates: Vec<String>,
}

pub struct MarisaDict {
    trie: Trie,
    values: Vec<Values>,
}

impl MarisaDict {
    /// Build from raw OpenCC `.txt` dictionary text.
    pub fn from_text(raw: &str) -> Self {
        // First pass: collect (key, default, candidates) so we can build the
        // keyset (needs keys) and the parallel values vec (needs ids).
        let mut entries: Vec<(String, String, Vec<String>)> = Vec::new();
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
            let candidates: Vec<String> = iter.map(|s| s.to_string()).collect();
            entries.push((key.to_string(), first.to_string(), candidates));
        }
        // Build the keyset (only keys go into the trie).
        let mut keyset = Keyset::new();
        for (k, _, _) in &entries {
            keyset.push_back_bytes(k.as_bytes(), 1.0).expect("key too long");
        }
        let mut trie = Trie::new();
        trie.build(&mut keyset, 0);
        // predictive_search walks the trie in marisa's canonical order and
        // assigns each key its `id()`. We use that id to look up the matching
        // (key, value) entry by string match and build the parallel values vec
        // indexed by id.
        let mut values: Vec<Option<Values>> = (0..entries.len()).map(|_| None).collect();
        // Build a quick lookup map key → index in `entries`. Keys are unique
        // because the dict format doesn't allow duplicates.
        let mut by_key: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::with_capacity(entries.len());
        for (i, (k, _, _)) in entries.iter().enumerate() {
            by_key.insert(k.as_str(), i);
        }
        let mut agent = Agent::new();
        agent.init_state().expect("agent state init");
        agent.set_query_bytes(b"");
        while trie.predictive_search(&mut agent) {
            let id = agent.key().id();
            let key_bytes = agent.key().as_bytes();
            let key_str = std::str::from_utf8(key_bytes)
                .expect("marisa keys are UTF-8 (we inserted UTF-8)");
            let idx = *by_key.get(key_str).expect("every marisa key came from entries");
            let (_, default, candidates) = entries[idx].clone();
            values[id] = Some(Values { default, candidates });
        }
        let values: Vec<Values> = values.into_iter().map(|v| v.unwrap()).collect();
        MarisaDict { trie, values }
    }

    pub fn from_entries(entries: &[(String, String)]) -> Self {
        // Wrap each entry as a single-line .txt.
        let mut s = String::new();
        for (k, v) in entries {
            s.push_str(k);
            s.push('\t');
            s.push_str(v);
            s.push('\n');
        }
        Self::from_text(&s)
    }

    /// Longest key that is a prefix of `text`. Returns `(byte_len, default_value)`.
    /// Mirrors `Dict::longest_prefix`.
    pub fn longest_prefix(&self, text: &str) -> Option<(usize, &str)> {
        let mut agent = Agent::new();
        agent.init_state().expect("agent state init");
        agent.set_query_bytes(text.as_bytes());
        let mut best: Option<(usize, &str)> = None;
        while self.trie.common_prefix_search(&mut agent) {
            let id = agent.key().id();
            let v = &self.values[id];
            let qpos = agent.state().unwrap().query_pos();
            best = Some((qpos, v.default.as_str()));
        }
        best
    }

    /// Multi-value longest prefix — for n-gram disambig. Returns all
    /// candidate values (default first), or `vec![default]` if single-value.
    /// Mirrors `Dict::longest_prefix_multi`.
    pub fn longest_prefix_multi<'a>(&'a self, text: &str) -> Option<(usize, Vec<&'a str>)> {
        let mut agent = Agent::new();
        agent.init_state().expect("agent state init");
        agent.set_query_bytes(text.as_bytes());
        let mut best: Option<(usize, Vec<&'a str>)> = None;
        while self.trie.common_prefix_search(&mut agent) {
            let id = agent.key().id();
            let v = &self.values[id];
            let qpos = agent.state().unwrap().query_pos();
            let cands: Vec<&str> = if v.candidates.is_empty() {
                vec![v.default.as_str()]
            } else {
                let mut out = Vec::with_capacity(1 + v.candidates.len());
                out.push(v.default.as_str());
                out.extend(v.candidates.iter().map(String::as_str));
                out
            };
            best = Some((qpos, cands));
        }
        best
    }

    /// Stats for debugging / logging.
    pub fn stats(&self) -> (usize, usize, usize) {
        (self.trie.num_keys(), self.trie.num_nodes(), self.trie.io_size())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_prefix_basic() {
        let d = MarisaDict::from_entries(&[
            ("ab".to_string(), "AB".to_string()),
            ("abc".to_string(), "ABC".to_string()),
        ]);
        assert_eq!(d.longest_prefix("abcd"), Some((3, "ABC")));
        assert_eq!(d.longest_prefix("abx"), Some((2, "AB")));
        assert_eq!(d.longest_prefix("xb"), None);
    }

    #[test]
    fn chinese_keys() {
        let raw = "中\t中\n国\t國\n中国\t中國\n";
        let d = MarisaDict::from_text(raw);
        // Longest match in "中国话" is "中国" (2 chars = 6 bytes)
        assert_eq!(d.longest_prefix("中国话"), Some((6, "中國")));
        // Longest match in "中国人" is also "中国"
        assert_eq!(d.longest_prefix("中国人"), Some((6, "中國")));
        // Single-char "中" in "中午" → just "中" (3 bytes)
        assert_eq!(d.longest_prefix("中午"), Some((3, "中")));
    }

    #[test]
    fn multi_value_exposed() {
        let raw = "出\t出 齣\n";
        let d = MarisaDict::from_text(raw);
        let (len, cands) = d.longest_prefix_multi("出门").unwrap();
        assert_eq!(len, 3);
        assert_eq!(cands, vec!["出", "齣"]);
        let (len, first) = d.longest_prefix("出门").unwrap();
        assert_eq!((len, first), (3, "出"));
    }

    #[test]
    fn single_value_no_multi() {
        let raw = "中\t中\n国\t國\n";
        let d = MarisaDict::from_text(raw);
        let (len, cands) = d.longest_prefix_multi("中国").unwrap();
        assert_eq!(len, 3);
        assert_eq!(cands, vec!["中"]);
    }
}