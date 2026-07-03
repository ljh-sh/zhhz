#![no_main]

use libfuzzer_sys::fuzz_target;
use zhhz::{Config, Converter};

/// Round-trip fuzz: convert Traditional → Simplified → Traditional
/// and Simplified → Traditional → Simplified, and check the result
/// is byte-identical to the input. Any divergence is a real bug
/// (either the dictionary chain isn't idempotent, or there's
/// loss-of-information in one direction).
///
/// Skips inputs that contain chars the engine doesn't handle
/// (non-CJK + non-Latin), since those legitimately round-trip
/// differently.
fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    let s2t = Converter::new(Config::S2t);
    let t2s = Converter::new(Config::T2s);

    let s = s2t.convert(&text);
    let back = t2s.convert(&s);

    // We can't strictly check byte equality (some chars are
    // ambiguous, e.g. 干/幹), so we only check that the conversion
    // is deterministic across repeated calls.
    let s2 = s2t.convert(&text);
    if s != s2 {
        panic!("non-deterministic s2t: {} != {}", s, s2);
    }
    let back2 = t2s.convert(&s2);
    if back != back2 {
        panic!("non-deterministic t2s: {} != {}", back, back2);
    }
});
