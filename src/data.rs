//! Embedded OpenCC data.
//!
//! All conversion data is baked into the binary at compile time — there is no
//! runtime download and no separate data directory. Source dictionaries live in
//! [`data/dictionary`](../data/dictionary) (a pure upstream mirror); the five
//! build-time-generated dictionaries are produced by [`build.rs`](../build.rs)
//! into `OUT_DIR` and referenced here.

/// Resolve a dictionary base name (e.g. `STPhrases`, `TWVariantsRev`) to its
/// embedded UTF-8 text. Returns `None` for unknown names.
pub fn dict_text(name: &str) -> Option<&'static str> {
    // 17 vendored upstream source dictionaries.
    let raw = match name {
        "STCharacters" => include_str!("../data/dictionary/STCharacters.txt"),
        "STPhrases" => include_str!("../data/dictionary/STPhrases.txt"),
        "TSCharacters" => include_str!("../data/dictionary/TSCharacters.txt"),
        "TSPhrases" => include_str!("../data/dictionary/TSPhrases.txt"),
        "TWPhrases" => include_str!("../data/dictionary/TWPhrases.txt"),
        "TWPhrasesRev" => include_str!("../data/dictionary/TWPhrasesRev.txt"),
        "TWVariantsPhrases" => include_str!("../data/dictionary/TWVariantsPhrases.txt"),
        "TWVariants" => include_str!("../data/dictionary/TWVariants.txt"),
        "TWVariantsRevPhrases" => include_str!("../data/dictionary/TWVariantsRevPhrases.txt"),
        "HKVariantsPhrases" => include_str!("../data/dictionary/HKVariantsPhrases.txt"),
        "HKVariants" => include_str!("../data/dictionary/HKVariants.txt"),
        "HKVariantsRevPhrases" => include_str!("../data/dictionary/HKVariantsRevPhrases.txt"),
        "HKPhrases" => include_str!("../data/dictionary/HKPhrases.txt"),
        "HKPhrasesRev" => include_str!("../data/dictionary/HKPhrasesRev.txt"),
        "JPShinjitaiCharacters" => include_str!("../data/dictionary/JPShinjitaiCharacters.txt"),
        "JPShinjitaiPhrases" => include_str!("../data/dictionary/JPShinjitaiPhrases.txt"),
        "CJK_Compatibility_Ideographs" => {
            include_str!("../data/dictionary/CJK_Compatibility_Ideographs.txt")
        }
        // 5 build-time-generated dictionaries (from OUT_DIR).
        "TSCharactersExt" => include_str!(concat!(env!("OUT_DIR"), "/TSCharactersExt.txt")),
        "STPhrases_GeneratedFromRegionalPhrases" => {
            include_str!(concat!(
                env!("OUT_DIR"),
                "/STPhrases_GeneratedFromRegionalPhrases.txt"
            ))
        }
        "TWVariantsRev" => include_str!(concat!(env!("OUT_DIR"), "/TWVariantsRev.txt")),
        "HKVariantsRev" => include_str!(concat!(env!("OUT_DIR"), "/HKVariantsRev.txt")),
        "JPShinjitaiCharactersRev" => {
            include_str!(concat!(env!("OUT_DIR"), "/JPShinjitaiCharactersRev.txt"))
        }
        _ => return None,
    };
    Some(raw)
}

/// Resolve a config base name (e.g. `s2t`, `tw2sp`) to its embedded JSON text.
pub fn config_text(name: &str) -> Option<&'static str> {
    let raw = match name {
        "s2t" => include_str!("../data/config/s2t.json"),
        "t2s" => include_str!("../data/config/t2s.json"),
        "s2tw" => include_str!("../data/config/s2tw.json"),
        "tw2s" => include_str!("../data/config/tw2s.json"),
        "s2hk" => include_str!("../data/config/s2hk.json"),
        "hk2s" => include_str!("../data/config/hk2s.json"),
        "s2twp" => include_str!("../data/config/s2twp.json"),
        "tw2sp" => include_str!("../data/config/tw2sp.json"),
        "s2hkp" => include_str!("../data/config/s2hkp.json"),
        "hk2sp" => include_str!("../data/config/hk2sp.json"),
        "t2tw" => include_str!("../data/config/t2tw.json"),
        "tw2t" => include_str!("../data/config/tw2t.json"),
        "t2hk" => include_str!("../data/config/t2hk.json"),
        "hk2t" => include_str!("../data/config/hk2t.json"),
        "t2jp" => include_str!("../data/config/t2jp.json"),
        "jp2t" => include_str!("../data/config/jp2t.json"),
        _ => return None,
    };
    Some(raw)
}
