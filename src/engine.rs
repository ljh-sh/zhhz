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
    /// **perf (zhhz#35, B)**: reusable scratch buffer for `convert()`.
    /// Holds the output bytes accumulated across all segments. Materialised
    /// into a `String` at the end of each convert. Eliminates the
    /// per-segment `String::with_capacity` allocation (~350K per 10 MB
    /// input). Capacity grows monotonically; reuse keeps the working set
    /// stable across repeated calls.
    scratch: std::cell::RefCell<Vec<u8>>,
    /// **perf (zhhz#35, B)**: reusable scratch buffer for the n-gram
    /// prev_emit (last 2 chars of emitted text per segment). Bounded
    /// to 6 bytes (2 CJK chars). Only used when ngram is active.
    prev_emit_scratch: std::cell::RefCell<Vec<u8>>,
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
            scratch: std::cell::RefCell::new(Vec::new()),
            prev_emit_scratch: std::cell::RefCell::new(Vec::new()),
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
        // **perf (zhhz#35, B)**: use a reusable scratch Vec<u8> instead
        // of per-convert String allocation. We swap the buffer out
        // (preserving its allocation), fill it via segments, then
        // convert to String and put the buffer back.
        let mut scratch = std::mem::take(&mut *self.scratch.borrow_mut());
        scratch.clear();
        scratch.reserve(text.len() + text.len() / 5);

        // Snapshot the prev_emit bytes (immutable borrow drops at end of
        // statement) so we can re-borrow mutably below. Re-snapshot
        // at the top of each iteration.
        let mut prev_emit_snapshot: Vec<u8> = self.prev_emit_scratch.borrow().clone();

        match &self.cfg.segmentation {
            Some(seg_group) => {
                for segment in SegmentIter::new(text, seg_group) {
                    convert_through_chain_into(
                        segment,
                        &self.cfg.chain,
                        self.ngram.as_ref(),
                        self.ngram_mode,
                        &prev_emit_snapshot,
                        &mut scratch,
                        &mut self.prev_emit_scratch.borrow_mut(),
                    );
                    // Re-snapshot for the next segment (cheap — ≤ 6 bytes).
                    prev_emit_snapshot = self.prev_emit_scratch.borrow().clone();
                }
            }
            None => {
                convert_through_chain_into(
                    text,
                    &self.cfg.chain,
                    self.ngram.as_ref(),
                    self.ngram_mode,
                    &prev_emit_snapshot,
                    &mut scratch,
                    &mut self.prev_emit_scratch.borrow_mut(),
                );
            }
        }

        // Take the scratch bytes and convert to String in one move.
        // SAFETY: scratch only contains valid UTF-8 (see comment above).
        let bytes = std::mem::take(&mut scratch);
        let out = unsafe { String::from_utf8_unchecked(bytes) };

        // Put the (now-empty) scratch buffer back so its allocation
        // is reused on the next convert call.
        *self.scratch.borrow_mut() = scratch;
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

/// **perf (zhhz#35, B)**: arena-buffer variant of
/// `convert_through_chain`. Writes the converted segment into the caller's
/// reusable `scratch: Vec<u8>` instead of returning a fresh `String`
/// (the original returned `(String, String)` per segment — 350K allocs
/// per 10 MB input). Also writes the prev_emit (last 2 chars of emitted
/// text) into the caller's `prev_emit_buf: Vec<u8>`, only when ngram is
/// active (otherwise it stays empty).
///
/// For multi-stage chains, intermediate stage input is a slice of
/// `scratch` (the previous stage's output). On a single-stage, ngram-off
/// chain, the segment is converted directly with no intermediate buffer.
///
/// **Borrowing**: caller must pass `prev_emit_buf` as both input
/// (immutable borrow, read-only) and output (mutable borrow). We work
/// around the aliasing by reading the prev into a stack string first
/// when ngram is active.
fn convert_through_chain_into(
    segment: &str,
    chain: &[Vec<Dict>],
    ngram: Option<&NgramModel>,
    mode: NgramMode,
    prev_emit_bytes: &[u8],
    scratch: &mut Vec<u8>,
    prev_emit_buf: &mut Vec<u8>,
) {
    if chain.is_empty() {
        scratch.extend_from_slice(segment.as_bytes());
        if ngram.is_some() {
            tail_n_bytes_into(segment.as_bytes(), prev_emit_buf, 6);
        }
        return;
    }
    // SAFETY: prev_emit_bytes is always valid UTF-8 (we only ever push
    // valid &str into it via tail_n_bytes_into). Borrow it as &str.
    let prev_emit: &str = unsafe { std::str::from_utf8_unchecked(prev_emit_bytes) };
    // Fast path: single stage, no ngram. Write directly to the tail of
    // scratch, no intermediate buffer.
    if chain.len() == 1 && ngram.is_none() {
        convert_segment_into(
            segment, &chain[0], ngram, mode, "", scratch,
        );
        return;
    }
    // Slow path: multi-stage chain OR ngram. Build an intermediate
    // stage buffer in the same scratch space (right after the
    // previous segment's output).
    let stage_input_start = scratch.len();
    scratch.extend_from_slice(segment.as_bytes());
    let mut stage_prev_owned: Option<String> = None; // owned, so we can re-borrow scratch
    for stage in chain {
        // Copy the input slice out of scratch into a local String so
        // the &str doesn't alias with the &mut scratch we need to pass
        // to convert_segment_into. For a 10 MB input the stage input
        // is bounded by the longest segment (~50 KB), so the copy is
        // cheap relative to the work below.
        let input_owned: String = {
            // SAFETY: scratch contains valid UTF-8.
            let s = unsafe { std::str::from_utf8_unchecked(&scratch[stage_input_start..]) };
            s.to_string()
        };
        let prev_str: &str = stage_prev_owned.as_deref().unwrap_or(prev_emit);
        // Truncate scratch to stage_input_start, then have the stage
        // write into it. After the call, scratch[stage_input_start..]
        // is the new stage output.
        scratch.truncate(stage_input_start);
        convert_segment_into(
            &input_owned, stage, ngram, mode, prev_str, scratch,
        );
        // Update prev for the next stage: take the last 6 bytes of the
        // new output as an owned String (bounded to 2 chars).
        let new_end = scratch.len();
        let new_start = tail_n_bytes_start(&scratch[stage_input_start..new_end], 6)
            + stage_input_start;
        stage_prev_owned = Some(unsafe {
            std::str::from_utf8_unchecked(&scratch[new_start..new_end]).to_string()
        });
    }
    // If ngram active, also write the final segment's prev into the
    // caller's prev_emit_buf.
    if ngram.is_some() {
        let segment_end = scratch.len();
        let tail_start = tail_n_bytes_start(&scratch[stage_input_start..segment_end], 6) + stage_input_start;
        prev_emit_buf.clear();
        prev_emit_buf.extend_from_slice(&scratch[tail_start..segment_end]);
    }
}

/// Return the byte offset of the start of the last `max_bytes` of
/// `bytes`. Used to compute the tail slice for prev_emit.
#[inline]
fn tail_n_bytes_start(bytes: &[u8], max_bytes: usize) -> usize {
    if bytes.len() <= max_bytes {
        0
    } else {
        bytes.len() - max_bytes
    }
}

/// Copy the last `max_bytes` of `bytes` into `out_buf`, truncated to a
/// char boundary so the result is valid UTF-8.
#[inline]
fn tail_n_bytes_into(bytes: &[u8], out_buf: &mut Vec<u8>, max_bytes: usize) {
    let start = tail_n_bytes_start(bytes, max_bytes);
    // Walk back to a char boundary if needed.
    let mut s = start;
    while s < bytes.len() && (bytes[s] & 0xC0) == 0x80 {
        s += 1;
    }
    out_buf.clear();
    out_buf.extend_from_slice(&bytes[s..]);
}

/// **perf (zhhz#35, B)**: arena-buffer variant of `convert_segment`.
/// Appends the converted segment bytes to `out: Vec<u8>` instead of
/// building a `String`. `prev_emit` is supplied as bytes; the ngram
/// disambig path consults it for left context.
fn convert_segment_into(
    segment: &str,
    group: &[Dict],
    ngram: Option<&NgramModel>,
    mode: NgramMode,
    prev_emit: &str,
    out: &mut Vec<u8>,
) {
    let mut pos = 0;
    // Same SIMD-style ASCII pass-through as before; just writes into
    // `out` (Vec<u8>) directly instead of into a String.
    if ngram.is_none() {
        let bytes = segment.as_bytes();
        if !bytes.is_empty() && bytes[0] < 0x80 {
            // ASCII-leading segment.
            while pos < bytes.len() {
                let rest = &bytes[pos..];
                let ascii_end = find_non_ascii(rest);
                if ascii_end > 0 {
                    let ascii_run = &rest[..ascii_end];
                    out.extend_from_slice(ascii_run);
                    pos += ascii_end;
                    if pos >= bytes.len() {
                        break;
                    }
                }
                let rest = &bytes[pos..];
                let rest_str: &str = unsafe { std::str::from_utf8_unchecked(rest) };
                if let Some((key_len, value)) = group_longest_prefix(group, rest_str) {
                    out.extend_from_slice(value.as_bytes());
                    pos += key_len;
                    continue;
                }
                let ch_len = rest_str
                    .chars()
                    .next()
                    .map(|c| c.len_utf8())
                    .unwrap_or_else(|| rest_str.len().min(4));
                out.extend_from_slice(&rest[..ch_len]);
                pos += ch_len;
            }
            return;
        }
        // Chinese-leading segment: fall through to char walk.
        while pos < segment.len() {
            let rest = &segment[pos..];
            if let Some((key_len, value)) = group_longest_prefix(group, rest) {
                out.extend_from_slice(value.as_bytes());
                pos += key_len;
                continue;
            }
            let ch_len = rest
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or_else(|| rest.len().min(4));
            out.extend_from_slice(rest[..ch_len].as_bytes());
            pos += ch_len;
        }
        return;
    }
    // Disambig path (ngram.is_some()): same as before but writes to Vec<u8>.
    while pos < segment.len() {
        let rest = &segment[pos..];
        if let Some((key_len, cands)) = group_longest_prefix_multi(group, rest) {
            if cands.len() > 1 {
                let model = ngram.unwrap();
                let mut first_chars_buf: [String; 4] = [
                    String::new(), String::new(), String::new(), String::new(),
                ];
                let mut first_chars_count: usize = 0;
                'outer: for c in &cands {
                    if let Some(ch) = c.chars().next() {
                        let s = ch.to_string();
                        for j in 0..first_chars_count {
                            if first_chars_buf[j] == s {
                                continue 'outer;
                            }
                        }
                        if first_chars_count < 4 {
                            first_chars_buf[first_chars_count] = s;
                            first_chars_count += 1;
                        }
                    }
                }
                if first_chars_count > 1 {
                    let has_context = !prev_emit.is_empty() || !out.is_empty();
                    if !has_context {
                        let default_ch = &first_chars_buf[0];
                        out.extend_from_slice(default_ch.as_bytes());
                        for c in &cands {
                            if c.starts_with(default_ch.as_str()) {
                                if let Some(first) = c.chars().next() {
                                    out.extend_from_slice(c[first.len_utf8()..].as_bytes());
                                }
                                break;
                            }
                        }
                    } else {
                        let prev_borrowed: &str = match mode {
                            NgramMode::Bigram => last_n_chars(prev_emit, "", 1),
                            NgramMode::Trigram => last_n_chars(prev_emit, "", 2),
                            NgramMode::Off => unreachable!(),
                        };
                        // For out, we need the last n chars of the bytes already written.
                        // Compute on-the-fly from the Vec<u8> tail.
                        let out_tail: &str = match mode {
                            NgramMode::Bigram => last_n_bytes_str(out, 3), // 1 char max in CJK = 3 bytes
                            NgramMode::Trigram => last_n_bytes_str(out, 6), // 2 chars max
                            NgramMode::Off => unreachable!(),
                        };
                        let chosen = if !out_tail.is_empty() {
                            out_tail
                        } else {
                            prev_borrowed
                        };
                        let prev_opt: Option<&str> = if chosen.is_empty() {
                            None
                        } else {
                            Some(chosen)
                        };
                        let first_chars_slice: &[String] =
                            &first_chars_buf[..first_chars_count];
                        let pick = model
                            .disambiguate(prev_opt, first_chars_slice)
                            .unwrap_or_else(|| first_chars_buf[0].clone());
                        out.extend_from_slice(pick.as_bytes());
                        for c in &cands {
                            if c.starts_with(pick.as_str()) {
                                if let Some(first) = c.chars().next() {
                                    out.extend_from_slice(c[first.len_utf8()..].as_bytes());
                                }
                                break;
                            }
                        }
                    }
                } else if first_chars_count == 1 {
                    out.extend_from_slice(cands[0].as_bytes());
                } else {
                    out.extend_from_slice(cands[0].as_bytes());
                }
            } else {
                out.extend_from_slice(cands[0].as_bytes());
            }
            pos += key_len;
        } else {
            let ch_len = rest
                .chars()
                .next()
                .map(|c| c.len_utf8())
                .unwrap_or_else(|| rest.len().min(4));
            out.extend_from_slice(rest[..ch_len].as_bytes());
            pos += ch_len;
        }
    }
}

/// Borrow the last `max_bytes` bytes of `out` as a `&str`. Walks back
/// to a UTF-8 char boundary.
#[inline]
fn last_n_bytes_str(out: &[u8], max_bytes: usize) -> &str {
    if out.is_empty() || max_bytes == 0 {
        return "";
    }
    let start = if out.len() <= max_bytes { 0 } else { out.len() - max_bytes };
    let mut s = start;
    while s < out.len() && (out[s] & 0xC0) == 0x80 {
        s += 1;
    }
    // SAFETY: out is valid UTF-8 (only ever appended via push of valid
    // &str bytes).
    unsafe { std::str::from_utf8_unchecked(&out[s..]) }
}


/// Return a `&str` borrowing the last `n` chars of (prev + out), without
/// any allocation. Handles the cases:
///   - both empty → ""
///   - out has ≥ n chars → last n chars of out (out is fresher context)
///   - out has < n chars → last n chars of prev (the older chars; we'll
///     miss the most recent ones but the ngram model can still rank
///     candidates correctly most of the time, and at segment-start this
///     is rare anyway because T1.4 catches the truly-context-less case).
///
/// **Note on boundary accuracy**: the ngram model wants the last n chars
/// of (prev ++ out) as if they were concatenated. We can't return a
/// cross-boundary slice from two `&str`. Taking from prev alone is
/// suboptimal when out is short — but out grows past n chars within a
/// few FMM positions, so this only matters at segment-start, where the
/// disambig accuracy matters least (the prior `tail_2_chars` on each
/// segment already gave us `prev_emit` of up to 2 chars).
///
/// **perf (zhhz#32, T1.1)**: replaces the old `combined: String = prev + out`
/// + `combined.chars().rev().take(2).collect()` pattern that allocated
/// twice per multi-value match.
#[inline]
fn last_n_chars<'a>(prev: &'a str, out: &'a str, n: usize) -> &'a str {
    if out.is_empty() || out.chars().count() < n {
        tail_n_chars(prev, n)
    } else {
        tail_n_chars(out, n)
    }
}

/// Borrow the last `n` chars of `s` as a `&str`. Returns `s` itself if
/// `s.chars().count() <= n`. Empty string if `s` is empty.
#[inline]
fn tail_n_chars(s: &str, n: usize) -> &str {
    if s.is_empty() || n == 0 {
        return "";
    }
    let char_count = s.chars().count();
    if char_count <= n {
        return s;
    }
    let mut count = 0;
    let mut start = s.len();
    for (i, _) in s.char_indices().rev() {
        start = i;
        count += 1;
        if count == n {
            break;
        }
    }
    &s[start..]
}

/// Find the first byte ≥ 0x80 in `bytes`. Returns the index, or
/// `bytes.len()` if none. For Chinese text, lead bytes are
/// 0xE0-0xEF and continuation bytes are 0x80-0xBF; any of these
/// (≥ 0x80) means we've left the ASCII run.
///
/// Note: this is a 1-byte-at-a-time loop. For a true SIMD scan,
/// use `memchr::memchr3(0x80, 0xC0, 0xE0, ...)` or the
/// `safe_arch` crate's NEON/x86 SSE2 intrinsics. We use the simple
/// loop here for portability and to keep the PoC small.
#[inline]
fn find_non_ascii(bytes: &[u8]) -> usize {
    for (i, &b) in bytes.iter().enumerate() {
        if b >= 0x80 {
            return i;
        }
    }
    bytes.len()
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
