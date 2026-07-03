#![no_main]

use libfuzzer_sys::fuzz_target;
use zhhz::{Config, Converter};

/// Fuzz the FMM (forward-maximal-match) segmentation + conversion
/// pipeline. The interesting panics would be:
///   - OOB read in the trie walk
///   - N-gram disambiguation wrong-arity
///   - Detection returning invalid region codes
///
/// Conversion is pure (no I/O), so a panic here is a real bug in the
/// Rust core, not in I/O or allocation logic.
fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    // Treat the fuzz input as UTF-8 lossy. Invalid bytes become the
    // replacement char; that's fine — the goal is to stress the
    // segmentation pipeline, not the UTF-8 decoder.
    let text = String::from_utf8_lossy(data);
    let conv = Converter::new(Config::S2t);
    let _ = conv.convert(&text);
});
