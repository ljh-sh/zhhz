//! Canonical 齣/出 context test cases (regression gate for N-gram / context
//! disambiguation work — see `docs/context-test-cases.md`).
//!
//! These two sentences use the simplified character 出 twice with
//! different correct traditional renderings (出 = depart, 齣 = show
//! measure word). zhhz's current output happens to be correct on both
//! (data-driven: `STPhrases` 齣好戏 → 齣好戲, `STCharacters` 出 → 出
//! first value), and these tests lock that behaviour.

use zhhz::Converter;

fn s2t(s: &str) -> String {
    Converter::new(zhhz::Config::S2t).convert(s)
}

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
