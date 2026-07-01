// Test: can fast mode + custom dict fix the cases that ngram fixes?
// Run: cargo run --release --example test_custom_dict
use zhhz::{Config, Converter};

fn main() {
    let cases = vec![
        ("这出戏真好看", "這齣戲真好看", "ngram 修过的核心 case"),
        ("这出戏", "這齣戲", "ngram 修过的核心 case (短)"),
        ("彷佛", "彷彿", "my erroneous test case (应当 phrase dict 修)"),
        ("仿佛", "彷彿", "phrase dict 已修 — 基线"),
    ];

    // baseline: pure fast (no custom)
    println!("=== 1. pure fast (无 custom, 无 ngram) ===");
    let c = Converter::new(Config::S2t);
    for (src, exp, note) in &cases {
        let got = c.convert(src);
        let ok = if got == *exp { "✓" } else { "✗" };
        println!("  [{}] {:20} → {:20} (期望 {:20}) {}", ok, src, got, exp, note);
    }

    // try: fast + custom dict for 这出戏
    println!();
    println!("=== 2. fast + custom dict (这出戏 → 這齣戲) ===");
    let c = Converter::with_custom(
        Config::S2t,
        &[("这出戏".to_string(), "這齣戲".to_string())],
    );
    for (src, exp, note) in &cases {
        let got = c.convert(src);
        let ok = if got == *exp { "✓" } else { "✗" };
        println!("  [{}] {:20} → {:20} (期望 {:20}) {}", ok, src, got, exp, note);
    }

    // baseline: trigram
    println!();
    println!("=== 3. trigram (no custom) ===");
    use zhhz::{NgramModel, NgramMode};
    let model = NgramModel::from_file("/tmp/ngram-out/3gram.arpa").expect("3gram");
    let c = Converter::new(Config::S2t).with_ngram(model, NgramMode::Trigram);
    for (src, exp, note) in &cases {
        let got = c.convert(src);
        let ok = if got == *exp { "✓" } else { "✗" };
        println!("  [{}] {:20} → {:20} (期望 {:20}) {}", ok, src, got, exp, note);
    }
}