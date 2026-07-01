// Test: FMM+ngram vs opencc mmseg on disambig cases
use zhhz::{Config, Converter, NgramModel, NgramMode};

fn main() {
    let cases = vec![
        ("一出机场", "一出機場", "opencc 错 (一齣機場), zhhz trie 对"),
        ("一出停车场", "一出停車場", "opencc + zhhz fast 都错 (一齣停車場)"),
        ("一出电影院", "一出電影院", "opencc 错, zhhz 对 (STPhrases 有单值)"),
        ("一出教室", "一出教室", "opencc + zhhz 都错"),
        ("这出戏真好看", "這齣戲真好看", "fast 错, ngram 对"),
        ("彷佛", "彷彿", "fast 错 (彷佛), ngram 对 (彷彿)"),
        ("你干了", "你幹了", "fast 对, ngram 反向 (你乾了)"),
    ];

    println!("=== zhhz fast (FMM + cands[0]) ===");
    let c_fast = Converter::new(Config::S2t);
    for (src, exp, note) in &cases {
        let got = c_fast.convert(src);
        let ok = if got == *exp { "✓" } else { "✗" };
        println!("  [{}] {:12} → {:12} (期望 {:12})  {}", ok, src, got, exp, note);
    }

    println!("\n=== zhhz trigram (FMM + ngram) ===");
    let model = NgramModel::from_file("/tmp/ngram-out/3gram.arpa").expect("3gram");
    let c_tg = Converter::new(Config::S2t).with_ngram(model, NgramMode::Trigram);
    for (src, exp, note) in &cases {
        let got = c_tg.convert(src);
        let ok = if got == *exp { "✓" } else { "✗" };
        println!("  [{}] {:12} → {:12} (期望 {:12})  {}", ok, src, got, exp, note);
    }
}
