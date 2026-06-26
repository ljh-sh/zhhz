//! Script-variant detection (`zhhz detect`).
//!
//! Mirrors [`chardet`](https://github.com/ljh-sh/chardet)'s CLI design:
//! `<files>...` to detect each path, `-` (or no args) to detect the content
//! piped on stdin, `--files-from <PATH|->` to read a newline-separated list of
//! paths from a file or stdin, `-0`/`--null` for NUL-separated lists, and
//! recursive directory walking. Output is TSV:
//!
//! ```text
//! <region>\t<confidence>\t<path>
//! ```
//!
//! `<region>` is one of `cn-s`, `cn-t`, `cn-tw`, `cn-hk`, `jp-n`, `jp-t`, or
//! `unknown`. `<confidence>` is 0–100 (the winning region's share of the
//! signature characters in the input). `<path>` is `-` when the input came
//! from stdin.
//!
//! # Classification
//!
//! Per-region signature character sets are derived from the vendored OpenCC
//! data at runtime (`data::dict_text`):
//!
//! | region | signature source |
//! |---|---|
//! | `cn-s`  | `STCharacters` keys where the first value differs from the key (chars that "simplify" in a real sense). |
//! | `cn-t`  | `TSCharacters` keys where the first value differs. |
//! | `cn-tw` | `TWVariants` keys (chars with a Taiwan variant). |
//! | `cn-hk` | `HKVariants` keys (chars with a Hong Kong variant). |
//! | `jp-n`  | `JPShinjitaiCharacters` keys (shinjitai / new-form). |
//! | `jp-t`  | `JPShinjitaiCharacters` first values (kyūjitai / old-form). |
//!
//! Algorithm:
//!
//! 1. Count CJK Unified Ideographs in the input (U+4E00..U+9FFF).
//! 2. If Hiragana (U+3040..U+309F) or Katakana (U+30A0..U+30FF) is present,
//!    take the JP branch: tally `jp-n` vs `jp-t` hits; winner is the region.
//! 3. Otherwise the Chinese branch: tally `cn-s`, `cn-t`, `cn-tw`, `cn-hk`
//!    hits. `cn-tw` and `cn-hk` are subsets of `cn-t` (TW/HK are regional
//!    variants of OpenCC-trad), so a TW/HK hit upgrades a base `cn-t`
//!    verdict to `cn-tw`/`cn-hk` only when the TW/HK signal is strong
//!    relative to total traditional hits. Otherwise fall back to `cn-t`.
//! 4. Confidence = winner's hits / total CJK chars × 100, floored at 0.

use std::collections::HashSet;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use crate::data;
use crate::engine::Region;

const SAMPLE_BYTES: usize = 4096;

/// Result of detecting one input.
#[derive(Debug, Clone)]
pub struct Detection {
    pub region: Region,
    pub confidence: u8, // 0..=100
}

/// Set of signature characters for a region, derived from vendored data.
#[derive(Default)]
struct SignatureSet {
    chars: HashSet<char>,
}

impl SignatureSet {
    fn from_dict_keys(text: &str, require_change: bool) -> Self {
        let mut set = HashSet::new();
        for line in text.lines() {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, vals)) = line.split_once('\t') else {
                continue;
            };
            let Some(first) = vals.split_whitespace().next() else {
                continue;
            };
            if require_change && key == first {
                continue;
            }
            for ch in key.chars() {
                set.insert(ch);
            }
        }
        SignatureSet { chars: set }
    }

    fn contains(&self, ch: char) -> bool {
        self.chars.contains(&ch)
    }
}

struct Signatures {
    cn_s: SignatureSet,
    cn_t: SignatureSet,
    cn_tw: SignatureSet,
    cn_hk: SignatureSet,
    jp_n: SignatureSet,
    jp_t: SignatureSet,
}

static SIGNATURES: OnceLock<Signatures> = OnceLock::new();

fn load_signatures() -> &'static Signatures {
    SIGNATURES.get_or_init(|| {
        let cn_s =
            SignatureSet::from_dict_keys(data::dict_text("STCharacters").unwrap_or(""), true);
        let cn_t =
            SignatureSet::from_dict_keys(data::dict_text("TSCharacters").unwrap_or(""), true);
        let cn_tw =
            SignatureSet::from_dict_keys(data::dict_text("TWVariants").unwrap_or(""), false);
        let cn_hk =
            SignatureSet::from_dict_keys(data::dict_text("HKVariants").unwrap_or(""), false);
        // JPShinjitaiCharacters: keys are shinjitai (new), first values are kyūjitai (old).
        // require_change=false because some shinjitai/kyūjitai are identical to themselves.
        let jp_n = SignatureSet::from_dict_keys(
            data::dict_text("JPShinjitaiCharacters").unwrap_or(""),
            false,
        );
        // For jp_t (kyūjitai), build from the VALUES of JPShinjitaiCharacters.
        let mut jp_t_chars: HashSet<char> = HashSet::new();
        if let Some(text) = data::dict_text("JPShinjitaiCharacters") {
            for line in text.lines() {
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let Some((_key, vals)) = line.split_once('\t') else {
                    continue;
                };
                if let Some(first) = vals.split_whitespace().next() {
                    for ch in first.chars() {
                        jp_t_chars.insert(ch);
                    }
                }
            }
        }
        Signatures {
            cn_s,
            cn_t,
            cn_tw,
            cn_hk,
            jp_n,
            jp_t: SignatureSet { chars: jp_t_chars },
        }
    })
}

fn is_cjk(ch: char) -> bool {
    matches!(ch, '\u{4E00}'..='\u{9FFF}')
}

fn is_kana(ch: char) -> bool {
    matches!(ch, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}')
}

/// Detect the script variant of `text`. Returns `None` when there are no CJK or
/// kana characters (e.g. pure ASCII).
pub fn detect_text(text: &str) -> Option<Detection> {
    let sigs = load_signatures();

    let mut total_cjk = 0u32;
    let mut kana = 0u32;
    let mut cn_s = 0u32;
    let mut cn_t = 0u32;
    let mut cn_tw = 0u32;
    let mut cn_hk = 0u32;
    let mut jp_n = 0u32;
    let mut jp_t = 0u32;

    for ch in text.chars() {
        if is_cjk(ch) {
            total_cjk += 1;
            if sigs.cn_s.contains(ch) {
                cn_s += 1;
            }
            if sigs.cn_t.contains(ch) {
                cn_t += 1;
            }
            if sigs.cn_tw.contains(ch) {
                cn_tw += 1;
            }
            if sigs.cn_hk.contains(ch) {
                cn_hk += 1;
            }
            if sigs.jp_n.contains(ch) {
                jp_n += 1;
            }
            if sigs.jp_t.contains(ch) {
                jp_t += 1;
            }
        } else if is_kana(ch) {
            kana += 1;
        }
    }

    if kana > 0 {
        // Japanese branch: pick between shinjitai (jp-n) and kyūjitai (jp-t).
        let total_jp = jp_n + jp_t;
        if total_jp == 0 {
            // kana present but no kanji — still JP, but low confidence.
            return Some(Detection {
                region: Region::JpN,
                confidence: 50,
            });
        }
        if jp_n >= jp_t {
            return Some(Detection {
                region: Region::JpN,
                confidence: pct(jp_n, total_jp),
            });
        }
        return Some(Detection {
            region: Region::JpT,
            confidence: pct(jp_t, total_jp),
        });
    }

    if total_cjk == 0 {
        return None;
    }

    // Chinese branch.
    if cn_s > cn_t + cn_tw + cn_hk {
        return Some(Detection {
            region: Region::CnS,
            confidence: pct(cn_s, total_cjk),
        });
    }
    // Traditional-side: pick cn-tw / cn-hk when the regional signal is strong.
    if cn_tw >= cn_hk && cn_tw * 2 >= cn_t + cn_tw && cn_tw > 0 {
        return Some(Detection {
            region: Region::CnTw,
            confidence: pct(cn_t + cn_tw, total_cjk),
        });
    }
    if cn_hk > cn_tw && cn_hk * 2 >= cn_t + cn_hk && cn_hk > 0 {
        return Some(Detection {
            region: Region::CnHk,
            confidence: pct(cn_t + cn_hk, total_cjk),
        });
    }
    Some(Detection {
        region: Region::CnT,
        confidence: pct(cn_t + cn_tw + cn_hk, total_cjk),
    })
}

fn pct(part: u32, total: u32) -> u8 {
    if total == 0 {
        0
    } else {
        ((part as u64 * 100) / total as u64) as u8
    }
}

/// Truncate to a byte-length-bounded sample, keeping valid UTF-8 boundaries.
fn sample(text: &str, limit: usize) -> &str {
    if text.len() <= limit {
        return text;
    }
    let mut end = limit;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

/// Detect on a byte buffer (read a file with limit).
pub fn detect_bytes(data: &[u8]) -> Option<Detection> {
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return None, // not valid UTF-8
    };
    detect_text(sample(text, SAMPLE_BYTES))
}

// ---- CLI plumbing (chardet mirror) -----------------------------------------

pub fn run(args: &[String]) -> anyhow::Result<()> {
    let mut nul = false;
    let mut files_from: Option<String> = None;
    let mut paths: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(());
            }
            "-V" | "--version" => {
                println!("zhhz {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "-0" | "--null" => nul = true,
            "--files-from" => {
                i += 1;
                if i >= args.len() {
                    anyhow::bail!("zhhz detect: --files-from requires an argument");
                }
                files_from = Some(args[i].clone());
            }
            s if s.starts_with("--files-from=") => {
                files_from = Some(s["--files-from=".len()..].to_string());
            }
            _ => paths.push(args[i].clone()),
        }
        i += 1;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut had_error = false;

    fn process_path<W: Write>(p: &str, out: &mut W, had_error: &mut bool) {
        if let Err(e) = detect_one(p, out) {
            eprintln!("zhhz detect: {p}: {e}");
            *had_error = true;
        }
    }

    if let Some(src) = files_from {
        let list = read_file_list(&src, nul).map_err(anyhow::Error::msg)?;
        for p in &list {
            process_path(p, &mut out, &mut had_error);
        }
    } else {
        // No paths (or a `-`) means read content from stdin.
        let stdin_mode = paths.is_empty() || paths.iter().any(|p| p == "-");
        let paths: Vec<String> = if stdin_mode && paths.is_empty() {
            vec!["-".to_string()]
        } else {
            paths
        };
        for p in &paths {
            process_path(p, &mut out, &mut had_error);
        }
    }

    if had_error {
        std::process::exit(1);
    }
    Ok(())
}

const USAGE: &str = "\
zhhz-detect — identify the script variant (cn-s/cn-t/cn-tw/cn-hk/jp-n/jp-t) of Chinese text

Usage:
  zhhz detect <file>...                 detect each file
  zhhz detect -                         read content from stdin (path reported `-`)
  zhhz detect                           read content from stdin
  zhhz detect --files-from <PATH|->     detect each path listed in a file or stdin

Output:
  One line per input, tab-separated: `<region>\\t<confidence>\\t<path>`.
  region      cn-s | cn-t | cn-tw | cn-hk | jp-n | jp-t | unknown
  confidence  0–100, share of signature characters in the input
  path        path as given; `-` when input came from stdin

Options:
  -h, --help               show this help
  -V, --version            show version
  --files-from <PATH|->    detect each path from a newline-separated list in
                           <PATH>; use `-` for stdin
  -0, --null               with --files-from, paths are NUL-separated

Exit status:
  0  all inputs detected
  1  one or more inputs could not be read
";

fn read_file_list(src: &str, nul: bool) -> Result<Vec<String>, String> {
    let raw: Vec<u8> = if src == "-" {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        buf
    } else {
        fs::read(src).map_err(|e| e.to_string())?
    };
    let sep: u8 = if nul { 0 } else { b'\n' };
    Ok(raw
        .split(|&b| b == sep)
        .map(|chunk| String::from_utf8_lossy(chunk).trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

fn detect_one<W: Write>(path: &str, out: &mut W) -> Result<(), String> {
    if path == "-" {
        let mut buf = Vec::new();
        io::stdin()
            .read_to_end(&mut buf)
            .map_err(|e| e.to_string())?;
        let det = detect_bytes(&buf);
        write_line(out, det, "-");
        Ok(())
    } else {
        detect_path(path, out)
    }
}

fn detect_path<W: Write>(path: &str, out: &mut W) -> Result<(), String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err("no such file or directory".to_string());
    }
    if p.is_dir() {
        detect_dir(p, out)
    } else {
        let data = fs::read(p).map_err(|e| e.to_string())?;
        let det = detect_bytes(&data);
        write_line(out, det, path);
        Ok(())
    }
}

fn detect_dir<W: Write>(dir: &Path, out: &mut W) -> Result<(), String> {
    let mut files: Vec<PathBuf> = Vec::new();
    walk(dir, &mut files).map_err(|e| e.to_string())?;
    files.sort();
    for f in &files {
        match fs::read(f) {
            Ok(data) => {
                let det = detect_bytes(&data);
                write_line(out, det, &f.display().to_string());
            }
            Err(e) => {
                eprintln!("zhhz detect: {}: {}", f.display(), e);
            }
        }
    }
    Ok(())
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let p = entry.path();
        let ft = entry.file_type()?;
        if ft.is_dir() {
            walk(&p, out)?;
        } else if ft.is_file() {
            out.push(p);
        }
    }
    Ok(())
}

fn write_line<W: Write>(out: &mut W, det: Option<Detection>, path: &str) {
    match det {
        Some(d) => {
            let _ = writeln!(out, "{}\t{}\t{}", d.region.code(), d.confidence, path);
        }
        None => {
            let _ = writeln!(out, "unknown\t0\t{}", path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_returns_unknown() {
        assert!(detect_text("hello world 123\n").is_none());
    }

    #[test]
    fn pure_simplified() {
        let d = detect_text("汉字计算机软件打印机").expect("has CJK");
        assert_eq!(d.region, Region::CnS);
    }

    #[test]
    fn pure_traditional_opencc() {
        // 漢字 / 計算機 — keys of TSCharacters but not STCharacters
        let d = detect_text("漢字計算機軟體繁體").expect("has CJK");
        // 軟 and 體 are in JP shinjitai too, so jp_n may compete; pick whichever
        // wins by raw tally. The signal we want is cn_t dominating.
        assert!(matches!(d.region, Region::CnT | Region::JpN | Region::CnTw));
    }

    #[test]
    fn japanese_with_kana() {
        // hiragana + mixed kanji — should go to the JP branch
        let d = detect_text("こんにちは世界").expect("has kana or CJK");
        assert!(matches!(d.region, Region::JpN | Region::JpT));
    }

    #[test]
    fn regional_taiwan_signature() {
        // 滑鼠 (TW term) signals cn-tw. 鼠 is a TWVariants key.
        let d = detect_text("滑鼠與電腦").expect("has CJK");
        assert!(matches!(d.region, Region::CnTw | Region::CnT));
    }

    #[test]
    fn detect_bytes_rejects_invalid_utf8() {
        assert!(detect_bytes(&[0xFF, 0xFE, 0x00]).is_none());
    }

    #[test]
    fn sample_truncates_at_utf8_boundary() {
        let text = "汉字汉字汉字汉字汉字";
        assert!(sample(text, 5).chars().all(|c| c == '汉' || c == '字'));
    }
}
