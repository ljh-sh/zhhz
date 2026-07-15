// 4 个真正差异 case, 用 with_custom patch 测试 phrase dict 等价
use zhhz::{Config, Converter, NgramMode, NgramModel};

fn main() {
    // 穷举出的 4 个真正差异短语
    let cases = vec![
        ("你干了", "你幹了 / 你乾了", "你干了", "你幹了"), // verb "do"
        ("你干着", "你幹着 / 你乾着", "你干着", "你幹着"), // doing
        ("这出戏", "這出戲 / 這齣戲", "这出戏", "這齣戲"),
        ("那出戏", "那出戲 / 那齣戲", "那出戏", "那齣戲"),
        ("我干了", "我幹了 / 我乾了", "我干了", "我幹了"),
        ("我干着", "我幹着 / 我乾着", "我干着", "我幹着"),
    ];

    println!("=== A. pure fast ===");
    let c_fast = Converter::new(Config::S2t);
    for (src, exp_choice, _short, _) in &cases {
        let got = c_fast.convert(src);
        println!("  {:12} → {:12} (multi-value: {})", src, got, exp_choice);
    }

    println!("\n=== B. fast + custom patch (4 行) ===");
    let c_patch = Converter::with_custom(
        Config::S2t,
        &[
            ("你干了".to_string(), "你幹了".to_string()),
            ("你干着".to_string(), "你幹着".to_string()),
            ("我干了".to_string(), "我幹了".to_string()),
            ("我干着".to_string(), "我幹着".to_string()),
            ("这出戏".to_string(), "這齣戲".to_string()),
            ("那出戏".to_string(), "那齣戲".to_string()),
        ],
    );
    for (src, _, _, _) in &cases {
        let got = c_patch.convert(src);
        println!("  {:12} → {:12}", src, got);
    }

    println!("\n=== C. trigram ===");
    let model = NgramModel::from_file("/tmp/ngram-out/3gram.arpa").expect("3gram");
    let c_tg = Converter::new(Config::S2t).with_ngram(model, NgramMode::Trigram);
    for (src, _, _, _) in &cases {
        let got = c_tg.convert(src);
        println!("  {:12} → {:12}", src, got);
    }
}
