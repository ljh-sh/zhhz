//! The conversion engine: FMM segmentation + ordered conversion chain.
//!
//! This mirrors OpenCC's `MaxMatchSegmentation` + `ConversionChain` pipeline
//! exactly:
//!
//! 1. **Segment** the input with forward maximum matching against the
//!    segmentation group. Matched phrases become individual segments; runs of
//!    unmatched characters coalesce into a single segment each.
//! 2. **Convert** each segment through every stage in order (stage *n*'s output
//!    feeds stage *n+1*). Each stage re-walks its segment with longest-prefix
//!    matching against its own group, emitting the first candidate on a match
//!    or copying one character through on a miss.
//!
//! If a config has no segmentation (e.g. `t2jp`), the whole input is treated as
//! a single segment.

use crate::config::{self, ResolvedConfig};
use crate::data;
use crate::dict::{group_longest_prefix, Dict};

/// A built-in OpenCC conversion configuration.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Config {
    S2t,
    T2s,
    S2tw,
    Tw2s,
    S2hk,
    Hk2s,
    S2twp,
    Tw2sp,
    S2hkp,
    Hk2sp,
    T2tw,
    Tw2t,
    T2hk,
    Hk2t,
    T2jp,
    Jp2t,
}

impl Config {
    /// All built-in configs, in canonical order.
    pub const ALL: [Config; 16] = [
        Config::S2t,
        Config::T2s,
        Config::S2tw,
        Config::Tw2s,
        Config::S2hk,
        Config::Hk2s,
        Config::S2twp,
        Config::Tw2sp,
        Config::S2hkp,
        Config::Hk2sp,
        Config::T2tw,
        Config::Tw2t,
        Config::T2hk,
        Config::Hk2t,
        Config::T2jp,
        Config::Jp2t,
    ];

    /// The config name as used on the CLI and in OpenCC (`s2t`, `tw2sp`, ...).
    pub fn name(&self) -> &'static str {
        match self {
            Config::S2t => "s2t",
            Config::T2s => "t2s",
            Config::S2tw => "s2tw",
            Config::Tw2s => "tw2s",
            Config::S2hk => "s2hk",
            Config::Hk2s => "hk2s",
            Config::S2twp => "s2twp",
            Config::Tw2sp => "tw2sp",
            Config::S2hkp => "s2hkp",
            Config::Hk2sp => "hk2sp",
            Config::T2tw => "t2tw",
            Config::Tw2t => "tw2t",
            Config::T2hk => "t2hk",
            Config::Hk2t => "hk2t",
            Config::T2jp => "t2jp",
            Config::Jp2t => "jp2t",
        }
    }

    /// A short human-readable description of the conversion direction.
    pub fn description(&self) -> &'static str {
        match self {
            Config::S2t => "Simplified to Traditional (OpenCC standard)",
            Config::T2s => "Traditional (OpenCC standard) to Simplified",
            Config::S2tw => "Simplified to Traditional (Taiwan)",
            Config::Tw2s => "Traditional (Taiwan) to Simplified",
            Config::S2hk => "Simplified to Traditional (Hong Kong)",
            Config::Hk2s => "Traditional (Hong Kong) to Simplified",
            Config::S2twp => "Simplified to Traditional (Taiwan, with phrases)",
            Config::Tw2sp => "Traditional (Taiwan) to Simplified (with phrases)",
            Config::S2hkp => "Simplified to Traditional (Hong Kong, with phrases)",
            Config::Hk2sp => "Traditional (Hong Kong) to Simplified (with phrases)",
            Config::T2tw => "Traditional (OpenCC standard) to Traditional (Taiwan)",
            Config::Tw2t => "Traditional (Taiwan) to Traditional (OpenCC standard)",
            Config::T2hk => "Traditional (OpenCC standard) to Traditional (Hong Kong)",
            Config::Hk2t => "Traditional (Hong Kong) to Traditional (OpenCC standard)",
            Config::T2jp => "Japanese Kyūjitai (old) to Shinjitai (new)",
            Config::Jp2t => "Japanese Shinjitai (new) to Kyūjitai (old)",
        }
    }

    /// Parse a config name; returns `Err` with the list of valid names on miss.
    pub fn parse(name: &str) -> Result<Config, String> {
        for cfg in Config::ALL {
            if cfg.name() == name {
                return Ok(cfg);
            }
        }
        let names: Vec<&str> = Config::ALL.iter().map(|c| c.name()).collect();
        Err(format!(
            "unknown config '{name}'. Valid: {}",
            names.join(", ")
        ))
    }
}

/// A configured converter. Cheap to clone is *not* supported; build one per
/// pipeline and reuse it for many inputs.
pub struct Converter {
    cfg: ResolvedConfig,
}

impl Converter {
    /// Build a converter for a built-in config with no custom words.
    pub fn new(config: Config) -> Converter {
        Converter::with_custom(config, &[])
    }

    /// Build a converter, injecting `custom` words as the highest-priority
    /// dictionary in both segmentation and every conversion stage.
    pub fn with_custom(config: Config, custom: &[(String, String)]) -> Converter {
        let json = data::config_text(config.name())
            .unwrap_or_else(|| panic!("zhhz: missing embedded config '{}'", config.name()));
        let cfg = config::resolve(json, custom)
            .unwrap_or_else(|e| panic!("zhhz: failed to resolve config '{}': {e}", config.name()));
        Converter { cfg }
    }

    /// Convert a piece of text.
    pub fn convert(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len() + text.len() / 5);
        match &self.cfg.segmentation {
            Some(seg_group) => {
                for segment in SegmentIter::new(text, seg_group) {
                    out.push_str(&convert_through_chain(segment, &self.cfg.chain));
                }
            }
            None => out.push_str(&convert_through_chain(text, &self.cfg.chain)),
        }
        out
    }
}

/// Iterate the segments produced by FMM segmentation of `text`, yielding slices
/// into the original text (no allocation per segment).
struct SegmentIter<'a> {
    text: &'a str,
    seg_group: &'a [Dict],
    pos: usize,
    buf_start: usize,
    buf_len: usize,
    done: bool,
}

impl<'a> SegmentIter<'a> {
    fn new(text: &'a str, seg_group: &'a [Dict]) -> Self {
        SegmentIter {
            text,
            seg_group,
            pos: 0,
            buf_start: 0,
            buf_len: 0,
            done: false,
        }
    }
}

impl<'a> Iterator for SegmentIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        if self.done {
            return None;
        }
        while self.pos < self.text.len() {
            let rest = &self.text[self.pos..];
            if let Some((key_len, _value)) = group_longest_prefix(self.seg_group, rest) {
                // Flush any accumulated unmatched run first. Do NOT advance past
                // the match here — the next call re-finds it (OpenCC emits the
                // flushed buffer and the matched key as two separate segments).
                if self.buf_len > 0 {
                    let slice = &self.text[self.buf_start..self.buf_start + self.buf_len];
                    self.buf_len = 0;
                    return Some(slice);
                }
                let slice = &self.text[self.pos..self.pos + key_len];
                self.pos += key_len;
                self.buf_start = self.pos;
                return Some(slice);
            }
            // No prefix match: accumulate one character into the unmatched run.
            let ch_len = rest
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or_else(|| rest.len().min(4));
            if self.buf_len == 0 {
                self.buf_start = self.pos;
            }
            self.buf_len += ch_len;
            self.pos += ch_len;
        }
        // Flush the trailing unmatched run.
        if self.buf_len > 0 {
            let slice = &self.text[self.buf_start..self.buf_start + self.buf_len];
            self.buf_len = 0;
            self.done = true;
            return Some(slice);
        }
        self.done = true;
        None
    }
}

/// Run a single segment through the whole conversion chain.
fn convert_through_chain(segment: &str, chain: &[Vec<Dict>]) -> String {
    if chain.is_empty() {
        return segment.to_string();
    }
    let mut current = segment.to_string();
    for stage in chain {
        current = convert_segment(&current, stage);
    }
    current
}

/// One conversion stage: longest-prefix per position. On a match, emit the
/// default candidate and advance by the key length; on a miss, copy one
/// character through and advance by one character.
fn convert_segment(segment: &str, group: &[Dict]) -> String {
    let mut out = String::with_capacity(segment.len() + segment.len() / 5);
    let mut pos = 0;
    while pos < segment.len() {
        let rest = &segment[pos..];
        if let Some((key_len, value)) = group_longest_prefix(group, rest) {
            out.push_str(value);
            pos += key_len;
        } else {
            let ch_len = rest
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or_else(|| rest.len().min(4));
            out.push_str(&rest[..ch_len]);
            pos += ch_len;
        }
    }
    out
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
    fn group_priority_dominates_length() {
        // Higher-priority dict has a shorter prefix; it must win.
        let high = Dict::from_entries(&[("ab".to_string(), "HI".to_string())]);
        let low = Dict::from_entries(&[("abc".to_string(), "LO".to_string())]);
        let group = vec![high, low];
        assert_eq!(group_longest_prefix(&group, "abcd"), Some((2, "HI")));
    }

    #[test]
    fn group_falls_through() {
        let high = Dict::from_entries(&[("zz".to_string(), "Z".to_string())]);
        let low = Dict::from_entries(&[("abc".to_string(), "LO".to_string())]);
        let group = vec![high, low];
        assert_eq!(group_longest_prefix(&group, "abcd"), Some((3, "LO")));
    }

    #[test]
    fn s2t_basic() {
        let c = Converter::new(Config::S2t);
        // "计算" is an STPhrases key; "机" converts via STCharacters.
        assert_eq!(c.convert("计算机"), "計算機");
        assert_eq!(c.convert("汉字"), "漢字");
        assert_eq!(c.convert("汉字计算机软件繁体"), "漢字計算機軟件繁體");
    }

    #[test]
    fn t2s_basic() {
        let c = Converter::new(Config::T2s);
        assert_eq!(c.convert("漢字"), "汉字");
        assert_eq!(c.convert("計算機"), "计算机");
    }

    #[test]
    fn s2twp_regional_phrase() {
        // "信息" is a TWPhrases key (Simplified -> Taiwan phrase); applied by s2twp.
        let c = Converter::new(Config::S2twp);
        assert_eq!(c.convert("信息"), "資訊");
    }

    #[test]
    fn japanese_shinjitai() {
        // jp2t: new -> old; t2jp: old -> new (generated reverse).
        assert_eq!(Converter::new(Config::Jp2t).convert("万与"), "萬與");
        assert_eq!(Converter::new(Config::T2jp).convert("萬與"), "万与");
    }

    #[test]
    fn custom_words_override() {
        // s2t("软件") is "軟件" (char-by-char). A custom word forces "軟體".
        assert_eq!(Converter::new(Config::S2t).convert("软件"), "軟件");
        let c = Converter::with_custom(Config::S2t, &[("软件".to_string(), "軟體".to_string())]);
        assert_eq!(c.convert("软件"), "軟體");
        assert_eq!(c.convert("买软件"), "買軟體");
    }
}
