//! Differential parity harness: compare `zhhz` against the `opencc` CLI
//! byte-for-byte across all 16 built-in configs.
//!
//! Run with:
//!
//! ```sh
//! cargo run --example parity                    # use `opencc` from PATH
//! OPENCC_BIN=/path/to/opencc cargo run --example parity
//! ```
//!
//! The harness calls `zhhz::Converter` directly for our side and shells out to
//! the `opencc` CLI as the reference. Test text is sampled from the vendored
//! OpenCC dictionaries (real Chinese phrases), a few common single CJK
//! characters, and edge cases (empty, ASCII, punctuation, newlines).
//!
//! `opencc` is used as the correctness standard until `zhhz` identifies
//! concrete improvements that diverge intentionally. For a meaningful gate,
//! the reference must use the **same data** as `zhhz` (vendored `cf0e4b6`);
//! build it with `scripts/build-reference-opencc.sh` and point `OPENCC_BIN`
//! at the resulting binary. Otherwise mismatches may reflect data-version
//! differences (the vendored data is newer than the system `opencc`).
//!
//! Result classification:
//! - **unsupported**: the reference `opencc` does not implement the config
//!   (e.g. `s2hkp`/`hk2sp` were added in opencc 1.3; system 1.2 lacks them).
//!   Skipped — not a parity-testable case here.
//! - **mismatch**: both sides ran, output differs. Exit 1.
//!
//! Exit code: 0 if zero mismatches (regardless of unsupported count);
//! 1 otherwise. Unsupported-by-reference is never a failure.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use zhhz::{Config, Converter};

fn opencc_bin() -> &'static str {
    static VAL: OnceLock<String> = OnceLock::new();
    VAL.get_or_init(|| std::env::var("OPENCC_BIN").unwrap_or_else(|_| "opencc".into()))
        .as_str()
}

fn opencc_data_dir() -> Option<&'static str> {
    static VAL: OnceLock<Option<String>> = OnceLock::new();
    VAL.get_or_init(|| std::env::var("OPENCC_DATA_DIR").ok())
        .as_deref()
}

/// Result of running the reference `opencc` on one input.
enum RefOutcome {
    /// Reference does not implement this config (older opencc missing s2hkp etc.).
    Unsupported,
    /// Reference ran; holds its stdout (post-strip).
    Ran(String),
    /// Reference failed to run (binary missing, IO error, etc.).
    Error(String),
}

fn run_opencc(config: &str, text: &str) -> RefOutcome {
    let bin = opencc_bin();
    let data = opencc_data_dir();
    let mut cmd = Command::new(bin);
    cmd.args(["-c", config]);
    if let Some(d) = data {
        cmd.args(["--path", d]);
    }
    let mut child = match cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return RefOutcome::Error(format!("spawn `{bin}`: {e}")),
    };
    if let Err(e) = child.stdin.as_mut().unwrap().write_all(text.as_bytes()) {
        return RefOutcome::Error(format!("stdin: {e}"));
    }
    let out = match child.wait_with_output() {
        Ok(o) => o,
        Err(e) => return RefOutcome::Error(format!("wait: {e}")),
    };
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("not found or not accessible") {
        return RefOutcome::Unsupported;
    }
    if !out.status.success() {
        return RefOutcome::Error(format!("exit {:?}: {}", out.status.code(), stderr));
    }
    RefOutcome::Ran(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Strip at most one trailing newline so inputs without `\n` compare fairly
/// against opencc which may append one.
fn trim_one_nl(s: &str) -> &str {
    s.strip_suffix('\n').unwrap_or(s)
}

/// Sample real phrase keys from a vendored dictionary (deterministic).
fn sample_keys(name: &str, stride: usize, max: usize) -> Vec<String> {
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "data",
        "dictionary",
        &format!("{name}.txt"),
    ]
    .into_iter()
    .collect();
    let Ok(text) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut keys = Vec::new();
    let mut i = 0;
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if i % stride == 0 {
            if let Some((k, _)) = line.split_once('\t') {
                keys.push(k.to_string());
            }
        }
        i += 1;
        if keys.len() >= max {
            break;
        }
    }
    keys
}

#[allow(clippy::vec_init_then_push)]
fn main() {
    let mut corpus: Vec<(&str, String)> = Vec::new();
    corpus.push(("empty", String::new()));
    corpus.push(("ascii-only", "Hello world 123".to_string()));
    corpus.push(("punct", "，。、；！？「」（）".to_string()));
    corpus.push(("single-cjk", "汉字字".to_string()));
    corpus.push(("newline", "第一行\n第二行\n".to_string()));
    corpus.push(("long-repeat", "汉字".repeat(40)));
    corpus.push(("mixed", "2026 年，Hello 漢字 mixed ASCII。".to_string()));
    for k in sample_keys("STPhrases", 1200, 12) {
        corpus.push(("stphrase", k));
    }
    for k in sample_keys("TWPhrases", 60, 8) {
        corpus.push(("twphrase", k));
    }
    for k in sample_keys("HKPhrases", 6, 6) {
        corpus.push(("hkphrase", k));
    }
    for k in sample_keys("JPShinjitaiPhrases", 40, 6) {
        corpus.push(("jpphrase", k));
    }

    let configs = Config::ALL;
    let mut pass = 0usize;
    let mut unsupported_by_cfg: BTreeMap<String, usize> = BTreeMap::new();
    let mut mismatch_by_cfg: BTreeMap<String, usize> = BTreeMap::new();
    let mut mismatches: Vec<(String, String, String, String, String)> = Vec::new();

    for (label, text) in &corpus {
        for cfg in configs {
            let cfg_name = cfg.name();
            let zhhz_out = Converter::new(cfg).convert(text);
            let opencc_out = match run_opencc(cfg_name, text) {
                RefOutcome::Unsupported => {
                    *unsupported_by_cfg.entry(cfg_name.to_string()).or_insert(0) += 1;
                    continue;
                }
                RefOutcome::Ran(s) => s,
                RefOutcome::Error(e) => {
                    eprintln!("[{cfg_name}] {label}: {e}");
                    continue;
                }
            };
            if trim_one_nl(&zhhz_out) == trim_one_nl(&opencc_out) {
                pass += 1;
            } else {
                *mismatch_by_cfg.entry(cfg_name.to_string()).or_insert(0) += 1;
                mismatches.push((
                    label.to_string(),
                    cfg_name.to_string(),
                    text.clone(),
                    opencc_out,
                    zhhz_out,
                ));
            }
        }
    }

    let total_supported = configs.len() * corpus.len();
    println!("\n=== zhhz vs {} parity ===", opencc_bin());
    println!("texts x configs: {} x {}", corpus.len(), configs.len());
    println!("pass:        {pass}");
    println!("mismatch:    {}", mismatches.len());
    let unsup_total: usize = unsupported_by_cfg.values().sum();
    println!("unsupported by reference: {unsup_total}");
    if !unsupported_by_cfg.is_empty() {
        println!("\nconfigs not implemented by reference (skipped, not a parity failure):");
        for (c, n) in &unsupported_by_cfg {
            println!("  {c:<8}  {n}");
        }
    }
    if !mismatch_by_cfg.is_empty() {
        println!("\nmismatches by config:");
        for (c, n) in &mismatch_by_cfg {
            println!("  {c:<8}  {n}");
        }
    }
    let show = mismatches.len().min(20);
    for (label, cfg, input, o, z) in mismatches.iter().take(show) {
        let trunc = |s: &str| -> String {
            if s.chars().count() > 60 {
                let t: String = s.chars().take(60).collect();
                format!("{t}…")
            } else {
                s.to_string()
            }
        };
        println!("\n[{cfg}] {label}");
        println!("  in   : {}", trunc(input));
        println!("  opencc: {}", trunc(o));
        println!("  zhhz : {}", trunc(z));
    }
    if mismatches.len() > show {
        println!("\n... {} more mismatches omitted.", mismatches.len() - show);
    }
    let _ = total_supported;

    if !mismatches.is_empty() {
        std::process::exit(1);
    }
}
