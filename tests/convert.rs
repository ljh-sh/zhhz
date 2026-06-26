//! End-to-end conversion tests through the public `Converter` API.
//!
//! Expected values are pinned against the OpenCC dictionary data vendored at the
//! commit recorded in `data/UPSTREAM`. They guard against regressions in
//! segmentation, the conversion chain, and the build-time-generated dictionaries.

use zhhz::{Config, Converter};

fn conv(config: Config, text: &str) -> String {
    Converter::new(config).convert(text)
}

#[test]
fn s2t_then_t2s_roundtrip_on_phrases() {
    let s = "汉字计算机软件繁体";
    let t = conv(Config::S2t, s);
    assert_eq!(t, "漢字計算機軟件繁體");
    // t2s is not a perfect inverse of s2t by OpenCC design, but these particular
    // characters round-trip cleanly.
    assert_eq!(conv(Config::T2s, &t), s);
}

#[test]
fn s2twp_applies_taiwan_phrases() {
    // "信息" is a TWPhrases key (Simplified -> Taiwan: 資訊).
    assert_eq!(conv(Config::S2twp, "信息"), "資訊");
}

#[test]
fn hk_directions() {
    // s2hk -> hk2t should restore the OpenCC-standard traditional form where
    // the HK variant differs.
    let s = "鼠标";
    let hk = conv(Config::S2hk, s);
    assert_eq!(conv(Config::Hk2t, &hk), conv(Config::S2t, s));
}

#[test]
fn japanese_new_old_roundtrip() {
    // jp2t: new (shinjitai) -> old (kyūjitai); t2jp inverts via the generated reverse.
    assert_eq!(conv(Config::Jp2t, "万与"), "萬與");
    assert_eq!(conv(Config::T2jp, "萬與"), "万与");
}

#[test]
fn custom_words_override_builtin() {
    // s2t("软件") is "軟件" (char-by-char); a custom word forces "軟體".
    assert_eq!(conv(Config::S2t, "软件"), "軟件");
    let c = Converter::with_custom(Config::S2t, &[("软件".into(), "軟體".into())]);
    assert_eq!(c.convert("买软件"), "買軟體");
}

#[test]
fn non_chinese_passes_through() {
    let c = Converter::new(Config::S2t);
    assert_eq!(c.convert("hello 汉字 world 123"), "hello 漢字 world 123");
}

#[test]
fn empty_input() {
    assert_eq!(conv(Config::S2t, ""), "");
}
