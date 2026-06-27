//! Canonical 齣/出 context test cases (regression gate for N-gram / context
//! disambiguation work — see `docs/context-test-cases.md`).
//!
//! Two layers of tests:
//!
//! 1. **Fast path** (`--fast`, no n-gram): exercises the dict-only path.
//!    Cases handled by `STPhrases` (e.g. "一出好戏" → "一齣好戲") pass
//!    here; cases NOT in the phrase dict (e.g. "这出戏" → "這齣戲")
//!    currently fail and are marked #[ignore] on the fast path. They
//!    become expected-to-pass on the bigram / trigram path.
//!
//! 2. **N-gram path** (`--bigram` / `--trigram`): exercises the model
//!    disambiguation. The model trained on a mixed Simplified+Traditional
//!    corpus picks the right candidate for cases where the phrase dict
//!    has no entry.
//!
//! Test data is loaded from `tests/fixtures/2gram.arpa`. If the fixture
//! is missing, the n-gram tests are silently ignored (the model is not
//! part of the zhhz repo per `docs/ngram-policy.md`).

use std::path::Path;
use zhhz::{Config, Converter, NgramMode, NgramModel};

fn s2t(s: &str) -> String {
    Converter::new(Config::S2t).convert(s)
}

fn s2t_bigram(s: &str) -> String {
    let m = match load_model() {
        Some(m) => m,
        None => return s2t(s), // fixture missing: fall back to fast path
    };
    Converter::new(Config::S2t)
        .with_ngram(m.clone_model(), NgramMode::Bigram)
        .convert(s)
}

#[allow(dead_code)]
fn s2t_trigram(s: &str) -> String {
    let m = match load_model() {
        Some(m) => m,
        None => return s2t(s),
    };
    Converter::new(Config::S2t)
        .with_ngram(m.clone_model(), NgramMode::Trigram)
        .convert(s)
}

fn load_model() -> Option<NgramModel> {
    // Candidate paths: workspace-relative and absolute. The fixture is
    // not committed (see docs/ngram-policy.md) — it's expected to be
    // present only when running these tests locally after downloading
    // from ljh-sh/ngram-exp.
    let candidates = [
        "tests/fixtures/2gram.arpa",
        "/tmp/ngram-out/2gram.arpa",
    ];
    for p in candidates {
        if Path::new(p).exists() {
            return NgramModel::from_file(p).ok();
        }
    }
    None
}

// --- Fast path (dict-only) cases. These already pass in v0.6.0. ---

#[test]
fn case_一出机场_就看到_一出好戏() {
    // 一出机场 = depart the airport  → 出 (NOT 齣)
    // 一出好戏 = a good show         → 齣
    assert_eq!(
        s2t("一出机场就看到一出好戏"),
        "一出機場就看到一齣好戲"
    );
}

#[test]
fn case_一出戏院_就看到_一出好戏() {
    // 一出戏院 = depart the theater  → 出 (NOT 齣)
    // 一出好戏 = a good show         → 齣
    assert_eq!(
        s2t("一出戏院就看到一出好戏"),
        "一出戲院就看到一齣好戲"
    );
}

#[test]
fn case_一出_verb_depart() {
    // 出 as verb, no measure-word context.
    assert_eq!(s2t("他出去了"), "他出去了");
    assert_eq!(s2t("看出问题"), "看出問題");
    assert_eq!(s2t("出门"), "出門");
}

// --- N-gram path: cases that the n-gram is expected to fix. ---

#[test]
fn ngram_这出戏() {
    // 这出戏 = "this show" (齿), NOT "this depart play" (出)
    // "这" alone is in STCharacters; "出戏" is in STPhrases with
    // multi-value ["出戲", "齣戲"]. The disambig picks 齣 when prev=這
    // (model has 這齣 = -1.63, 這出 = None).
    let got = s2t_bigram("这出戏真好看");
    assert_eq!(got, "這齣戲真好看");
}

#[test]
fn ngram_这出剧() {
    // 这出剧 = "this show/play" → 齣
    let got = s2t_bigram("这出剧");
    assert_eq!(got, "這齣劇");
}

#[test]
fn ngram_这出电影() {
    // 这出电影 = "this movie" → 齣
    let got = s2t_bigram("这出电影");
    assert_eq!(got, "這齣電影");
}

// --- Cases that even the n-gram does NOT fix (documented limitations). ---

#[test]
fn known_limitation_戏出了一半() {
    // 戏出了一半 = "half the show played" → 齣
    // STPhrases has "出了" as a single-value 2-char match, so FMM
    // matches it and never reaches the multi-value "出". The n-gram
    // cannot help here without a dict change.
    // We assert the current (incorrect) behaviour to lock the limitation.
    let got = s2t("戏出了一半");
    assert_eq!(got, "戲出了一半");
    let got_bigram = s2t_bigram("戏出了一半");
    assert_eq!(got_bigram, "戲出了一半");
}

// --- Semantic disambig: 齣 is the measure word (a show), 出 is the verb
//     (depart, issue, break character, etc.). Both readings of "出戏" are
//     valid Mandarin, but only one fits the surrounding context. ---

#[test]
fn ngram_演员出戏了_verb() {
    // 演员出戏了 = "the actor broke character" (出 as verb).
    // NOT "演員齣戲了" which would be ungrammatical.
    // The n-gram has no P(出|員) / P(齣|員) entries, so the disambig
    // falls back to the dict's first candidate "出" — which happens to
    // be the right answer here.
    let got = s2t_bigram("演员出戏了");
    assert_eq!(got, "演員出戲了");
    // Also assert --fast gets the same right answer (the cands[0] = 出).
    let got_fast = s2t("演员出戏了");
    assert_eq!(got_fast, "演員出戲了");
}

// --- Another semantic case where the dict dominates the answer. ---

#[test]
fn known_limitation_演员一出戏_就喊卡() {
    // 演员一出戏，导演就喊卡 = "Once the actor breaks character,
    // the director calls cut." Here "一出戏" is the verb form (出 as
    // "go out of" / "break"), NOT the measure word "一齣戲".
    //
    // The v0.7.1 patch overlay promotes "一出戏" in STPhrases to
    // multi-value ["一齣戲", "一出戲"], which means FMM now exposes
    // the ambiguity. However the n-gram model lacks P(*|员一) entries
    // (sparse even in the trigram) so disambig still falls back to
    // cands[0] = "一齣戲".
    //
    // To force the correct answer today, use a custom-dict entry:
    //
    //     演员一出戏	演員一出戲
    //
    // (see `custom_dict_演员一出戏_verb` below). The proper long-term
    // fix is per-position disambig + a domain-specific corpus.
    let got = s2t("演员一出戏，导演就喊卡");
    assert_eq!(got, "演員一齣戲，導演就喊卡");
    let got_bigram = s2t_bigram("演员一出戏，导演就喊卡");
    assert_eq!(got_bigram, "演員一齣戲，導演就喊卡");
}

#[test]
fn custom_dict_演员一出戏_verb() {
    // Workaround for the limitation above: a custom-dict entry
    // forces the verb form for "演员一出戏". This is the recommended
    // escape hatch until per-position disambig + better corpus land.
    use zhhz::Config::S2t;
    use zhhz::Converter;
    let c = Converter::with_custom(
        S2t,
        &[("演员一出戏".to_string(), "演員一出戲".to_string())],
    );
    assert_eq!(c.convert("演员一出戏，导演就喊卡"), "演員一出戲，導演就喊卡");
}

// --- ConvertReport: detect multi-value positions for LLM post-process.
//     Report support is deferred to v0.8 (see zhhz#15). ---
