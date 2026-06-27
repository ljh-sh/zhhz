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
use crate::dict::{group_longest_prefix, group_longest_prefix_multi, Dict};
use crate::ngram::NgramModel;

/// N-gram disambiguation mode.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NgramMode {
    /// No n-gram disambiguation — fast path, equivalent to v0.6.0.
    Off,
    /// Use bigram (P(c2 | c1)) for disambig.
    Bigram,
    /// Use trigram (P(c3 | c1, c2)) for disambig, falls back to bigram.
    Trigram,
}

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

/// A script variant that `zhhz` can convert to/from. Codes are stable,
/// short identifiers for the CLI (`--from cn-s --to cn-tw`).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Region {
    /// Simplified Chinese (mainland).
    CnS,
    /// Traditional Chinese, OpenCC standard.
    CnT,
    /// Traditional Chinese (Taiwan standard).
    CnTw,
    /// Traditional Chinese (Hong Kong standard).
    CnHk,
    /// Japanese Kyūjitai (old-form kanji).
    JpT,
    /// Japanese Shinjitai (new-form kanji).
    JpN,
}

impl Region {
    /// All regions in canonical order.
    pub const ALL: [Region; 6] = [
        Region::CnS,
        Region::CnT,
        Region::CnTw,
        Region::CnHk,
        Region::JpT,
        Region::JpN,
    ];

    /// The short, stable region code used on the CLI (`cn-s`, `cn-tw`, ...).
    pub fn code(&self) -> &'static str {
        match self {
            Region::CnS => "cn-s",
            Region::CnT => "cn-t",
            Region::CnTw => "cn-tw",
            Region::CnHk => "cn-hk",
            Region::JpT => "jp-t",
            Region::JpN => "jp-n",
        }
    }

    /// A human-readable description for `--list` and error messages.
    pub fn description(&self) -> &'static str {
        match self {
            Region::CnS => "Simplified Chinese (mainland)",
            Region::CnT => "Traditional Chinese (OpenCC standard)",
            Region::CnTw => "Traditional Chinese (Taiwan standard)",
            Region::CnHk => "Traditional Chinese (Hong Kong standard)",
            Region::JpT => "Japanese Kyūjitai (old-form)",
            Region::JpN => "Japanese Shinjitai (new-form)",
        }
    }

    /// Parse a region code (`cn-s`, `cn-tw`, ...); case-sensitive, returns an
    /// error listing valid codes on miss.
    pub fn parse(s: &str) -> Result<Region, String> {
        for r in Region::ALL {
            if r.code() == s {
                return Ok(r);
            }
        }
        let codes: Vec<&str> = Region::ALL.iter().map(|r| r.code()).collect();
        Err(format!("unknown region '{s}'. Valid: {}", codes.join(", ")))
    }
}

/// Resolve a `(from, to)` region pair to the OpenCC config that performs it.
/// Returns an error for identity pairs and for pairs without a direct config.
pub fn region_pair_config(from: Region, to: Region) -> Result<Config, String> {
    use Region::*;
    Ok(match (from, to) {
        (CnS, CnT) => Config::S2t,
        (CnT, CnS) => Config::T2s,
        (CnS, CnTw) => Config::S2twp,
        (CnTw, CnS) => Config::Tw2sp,
        (CnS, CnHk) => Config::S2hkp,
        (CnHk, CnS) => Config::Hk2sp,
        (CnT, CnTw) => Config::T2tw,
        (CnTw, CnT) => Config::Tw2t,
        (CnT, CnHk) => Config::T2hk,
        (CnHk, CnT) => Config::Hk2t,
        (JpN, JpT) => Config::Jp2t,
        (JpT, JpN) => Config::T2jp,
        (JpN, CnT) => Config::Jp2t,
        (CnT, JpN) => Config::T2jp,
        (f, t) if f == t => {
            return Err(format!(
                "no conversion needed: {} -> {}",
                f.code(),
                t.code()
            ))
        }
        (f, t) => {
            return Err(format!(
                "no direct conversion from '{}' to '{}'; try an intermediate (e.g. {} -> cn-t -> {})",
                f.code(),
                t.code(),
                f.code(),
                t.code()
            ));
        }
    })
}

/// A configured converter. Cheap to clone is *not* supported; build one per
/// pipeline and reuse it for many inputs.
pub struct Converter {
    cfg: ResolvedConfig,
    ngram: Option<NgramModel>,
    ngram_mode: NgramMode,
}

impl Converter {
    /// Build a converter for a built-in config with no custom words.
    /// No n-gram disambig (fast path; v0.6.0 behaviour).
    pub fn new(config: Config) -> Converter {
        Converter::with_custom(config, &[])
    }

    /// Build a converter, injecting `custom` words as the highest-priority
    /// dictionary in both segmentation and every conversion stage.
    /// No n-gram disambig.
    pub fn with_custom(config: Config, custom: &[(String, String)]) -> Converter {
        let json = data::config_text(config.name())
            .unwrap_or_else(|| panic!("zhhz: missing embedded config '{}'", config.name()));
        let cfg = config::resolve(json, custom)
            .unwrap_or_else(|e| panic!("zhhz: failed to resolve config '{}': {e}", config.name()));
        Converter {
            cfg,
            ngram: None,
            ngram_mode: NgramMode::Off,
        }
    }

    /// Enable n-gram disambiguation with the given model and mode.
    /// The mode must not be `Off` if a model is supplied.
    pub fn with_ngram(mut self, model: NgramModel, mode: NgramMode) -> Self {
        debug_assert!(mode != NgramMode::Off);
        self.ngram = Some(model);
        self.ngram_mode = mode;
        self
    }

    /// Build a converter with everything: config, custom words, and
    /// optional n-gram model.
    pub fn with_ngram_custom(
        config: Config,
        custom: &[(String, String)],
        model: Option<(NgramModel, NgramMode)>,
    ) -> Converter {
        let mut c = Self::with_custom(config, custom);
        if let Some((m, mode)) = model {
            c = c.with_ngram(m, mode);
        }
        c
    }

    /// Convert a piece of text.
    pub fn convert(&self, text: &str) -> String {
        let mut out = String::with_capacity(text.len() + text.len() / 5);
        let mut prev_emit = String::new();
        match &self.cfg.segmentation {
            Some(seg_group) => {
                for segment in SegmentIter::new(text, seg_group) {
                    let (new_seg, new_prev) = convert_through_chain(
                        segment,
                        &self.cfg.chain,
                        self.ngram.as_ref(),
                        self.ngram_mode,
                        &prev_emit,
                    );
                    out.push_str(&new_seg);
                    prev_emit = new_prev;
                }
            }
            None => {
                let (new_seg, _) = convert_through_chain(
                    text,
                    &self.cfg.chain,
                    self.ngram.as_ref(),
                    self.ngram_mode,
                    &prev_emit,
                );
                out.push_str(&new_seg);
            }
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
///
/// `prev_emit` is the running emitted text from earlier segments in the
/// same input (or empty for the first segment). It is the source of the
/// n-gram `prev` context used to disambiguate multi-value phrase matches
/// near the start of this segment — without it, a multi-value match at
/// position 0 of the segment would have no left context and would fall
/// back to the dict's first candidate.
fn convert_through_chain(
    segment: &str,
    chain: &[Vec<Dict>],
    ngram: Option<&NgramModel>,
    mode: NgramMode,
    prev_emit: &str,
) -> (String, String) {
    if chain.is_empty() {
        return (segment.to_string(), prev_emit.to_string());
    }
    // **perf (zhhz#21, J)**: if the chain has exactly one stage (e.g.
    // t2jp), skip the loop and the redundant intermediate String.
    // Saves 1 String alloc per segment (350K / 10 MB).
    if chain.len() == 1 && ngram.is_none() {
        let mut out = String::with_capacity(segment.len() + segment.len() / 5);
        let keep = convert_segment(segment, &chain[0], ngram, mode, prev_emit, &mut out);
        return (out, keep);
    }
    // Reuse a single `String` across stages to avoid per-stage
    // alloc-then-drop.
    let mut current = String::with_capacity(segment.len() + segment.len() / 5);
    current.push_str(segment);
    let mut stage_prev = prev_emit.to_string();
    let mut final_prev = prev_emit.to_string();
    for stage in chain {
        // Move `current` into `prev` so we can borrow `prev` as input
        // AND use `current`'s allocation as the output buffer for the
        // next stage. After convert_segment, `prev` is dropped and
        // `current` holds the new output, reusing its capacity.
        let mut prev = std::mem::replace(
            &mut current,
            String::with_capacity(0),
        );
        let new_prev = convert_segment(
            &prev, stage, ngram, mode, &stage_prev, &mut current,
        );
        prev.clear();
        stage_prev = new_prev.clone();
        final_prev = new_prev;
    }
    (current, final_prev)
}

/// One conversion stage: longest-prefix per position. On a match, emit the
/// default candidate and advance by the key length; on a miss, copy one
/// character through and advance by one character.
///
/// One conversion stage: longest-prefix per position. On a match, emit the
/// default candidate and advance by the key length; on a miss, copy one
/// character through and advance by one character.
///
/// `prev_emit` is the running emitted text from earlier segments in the
/// same input (or empty for the first segment). It provides the n-gram
/// `prev` context for multi-value matches at the start of this segment.
///
/// Returns `(out, new_prev)` where `new_prev` is `prev_emit + out`
/// (truncated to the last 2 chars), ready to be passed as `prev_emit`
/// for the next segment.
///
/// If `ngram` is `Some`, multi-value matches where the candidates differ
/// at the **first** char position are disambiguated by the n-gram model
/// using `prev_emit` (or the last char(s) of `out` for matches at later
/// positions) as context. The remaining chars of the candidate (positions
/// 1..) follow the first-char choice.
///
/// The "first char only" rule is deliberate: in Chinese, the n-gram
/// signal for in-phrase ambiguity (e.g. "一出" → "一齣" vs "一出") is
/// often misleading because the corpus is biased toward the more common
/// bigram. The dict's first candidate is more reliable for those cases.
/// The n-gram helps primarily when the ambiguity is in the *leading*
/// char (e.g. "这出戏" → "這出戲" vs "這齣戲" — where 這 is fixed but
/// 出/齣 depends on the left context).
fn convert_segment(
    segment: &str,
    group: &[Dict],
    ngram: Option<&NgramModel>,
    mode: NgramMode,
    prev_emit: &str,
    out: &mut String,
) -> String {
    // **perf (zhhz#21, J)**: write into a caller-provided `&mut String`
    // so the per-stage alloc is amortised across stages. Caller calls
    // `out.clear()` between stages (or moves the previous `current`
    // into here so we reuse its allocation).
    out.clear();
    out.reserve(segment.len() + segment.len() / 5);
    let mut pos = 0;
    while pos < segment.len() {
        let rest = &segment[pos..];
        // Fast path: no n-gram model loaded, so we never need the
        // candidates Vec that `group_longest_prefix_multi` allocates on
        // every match. The cheap `group_longest_prefix` returns
        // `&str` directly — zero allocation per FMM match.
        // (zhhz#14: this restores v0.6 fast-path throughput.)
        if ngram.is_none() {
            if let Some((key_len, value)) = group_longest_prefix(group, rest) {
                out.push_str(value);
                pos += key_len;
                continue;
            }
            let ch_len = rest
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or_else(|| rest.len().min(4));
            out.push_str(&rest[..ch_len]);
            pos += ch_len;
            continue;
        }
        // Disambig path: multi-value exposure needed for n-gram lookup.
        if let Some((key_len, cands)) = group_longest_prefix_multi(group, rest) {
            if cands.len() > 1 {
                let model = ngram.unwrap();
                let first_chars: Vec<String> = {
                    let mut v = Vec::new();
                    let mut seen = std::collections::HashSet::new();
                    for c in &cands {
                        if let Some(ch) = c.chars().next() {
                            let s = ch.to_string();
                            if seen.insert(s.clone()) {
                                v.push(s);
                            }
                        }
                    }
                    v
                };
                if first_chars.len() > 1 {
                    // Disambig. The prev is the last char(s) of
                    // (prev_emit + out): at position 0 of the segment,
                    // out is empty so prev is just prev_emit. Later in
                    // the segment, prev uses the trailing out.
                    let combined: String = {
                        let mut s = prev_emit.to_string();
                        s.push_str(&out);
                        s
                    };
                    let prev_owned: Option<String> = match mode {
                        NgramMode::Bigram => {
                            combined.chars().last().map(|c| c.to_string())
                        }
                        NgramMode::Trigram => {
                            let n: Vec<char> =
                                combined.chars().rev().take(2).collect();
                            if n.is_empty() {
                                None
                            } else {
                                Some(n.into_iter().rev().collect())
                            }
                        }
                        NgramMode::Off => unreachable!(),
                    };
                    let pick = model
                        .disambiguate(prev_owned.as_deref(), &first_chars)
                        .unwrap_or_else(|| first_chars[0].clone());
                    let mut rest_of_first = String::new();
                    for ch in cands[0].chars().skip(1) {
                        rest_of_first.push(ch);
                    }
                    out.push_str(&pick);
                    out.push_str(&rest_of_first);
                } else {
                    out.push_str(cands[0]);
                }
            } else {
                out.push_str(cands[0]);
            }
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
    // Truncate the running prev to the last 2 chars to bound memory.
    // **perf (zhhz#21, I)**: previous code did `out.chars().rev().take(2)
    // .collect::<Vec<char>>().into_iter().rev().collect::<String>()`,
    // which allocates a `Vec<char>` (8 bytes/elem) AND a fresh `String`
    // for every single segment — ~700K extra allocations per 10 MB
    // input in the fast path. macOS Instruments confirmed these as the
    // dominant allocator traffic. Replaced with byte-level slicing
    // (`str::char_indices` to find the last-2-char prefix) which
    // allocates nothing.
    let keep: String = if prev_emit.is_empty() {
        tail_2_chars(&out)
    } else {
        let mut combined = String::with_capacity(prev_emit.len() + out.len());
        combined.push_str(prev_emit);
        combined.push_str(&out);
        tail_2_chars(&combined)
    };
    keep
}

/// Return the last 2 chars of `s` as a new `String`, without
/// allocating a temporary `Vec<char>`. For Chinese text (3 bytes/char),
/// the last 2 chars are at most 6 bytes; we slice from the byte
/// boundary closest to that.
#[inline]
fn tail_2_chars(s: &str) -> String {
    let mut count = 0;
    let mut start = s.len();
    for (i, _) in s.char_indices().rev() {
        start = i;
        count += 1;
        if count == 2 {
            break;
        }
    }
    if count == 0 {
        String::new()
    } else {
        s[start..].to_string()
    }
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

    #[test]
    fn region_parse_roundtrip() {
        for r in Region::ALL {
            assert_eq!(Region::parse(r.code()).unwrap(), r);
        }
        assert!(Region::parse("xx").is_err());
    }

    #[test]
    fn region_pairs_mainline() {
        use Region::*;
        assert_eq!(region_pair_config(CnS, CnT).unwrap(), Config::S2t);
        assert_eq!(region_pair_config(CnT, CnS).unwrap(), Config::T2s);
        assert_eq!(region_pair_config(CnS, CnTw).unwrap(), Config::S2twp);
        assert_eq!(region_pair_config(CnTw, CnS).unwrap(), Config::Tw2sp);
        assert_eq!(region_pair_config(CnS, CnHk).unwrap(), Config::S2hkp);
        assert_eq!(region_pair_config(CnHk, CnS).unwrap(), Config::Hk2sp);
        assert_eq!(region_pair_config(CnT, CnTw).unwrap(), Config::T2tw);
        assert_eq!(region_pair_config(CnTw, CnT).unwrap(), Config::Tw2t);
        assert_eq!(region_pair_config(CnT, CnHk).unwrap(), Config::T2hk);
        assert_eq!(region_pair_config(CnHk, CnT).unwrap(), Config::Hk2t);
        assert_eq!(region_pair_config(JpN, JpT).unwrap(), Config::Jp2t);
        assert_eq!(region_pair_config(JpT, JpN).unwrap(), Config::T2jp);
    }

    #[test]
    fn region_pair_identity_errors() {
        for r in Region::ALL {
            assert!(region_pair_config(r, r).is_err());
        }
    }

    #[test]
    fn region_pair_no_direct_errors() {
        // cn-s -> jp-n has no single opencc config; suggest an intermediate.
        assert!(region_pair_config(Region::CnS, Region::JpN).is_err());
    }
}
