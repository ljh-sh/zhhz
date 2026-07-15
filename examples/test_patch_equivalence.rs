use zhhz::{Config, Converter, NgramMode, NgramModel};

fn main() {
    let cases = vec!["这出戏", "那出戏", "这出戏真好看", "那出戏好看"];

    println!("=== A. pure fast ===");
    let c_fast = Converter::new(Config::S2t);
    for s in &cases {
        println!("  {:20} → {:20}", s, c_fast.convert(s));
    }

    println!("\n=== B. fast + custom dict patch ===");
    let c_patch = Converter::with_custom(
        Config::S2t,
        &[
            ("这出戏".to_string(), "這齣戲".to_string()),
            ("那出戏".to_string(), "那齣戲".to_string()),
        ],
    );
    for s in &cases {
        println!("  {:20} → {:20}", s, c_patch.convert(s));
    }

    println!("\n=== C. trigram ===");
    let model = NgramModel::from_file("/tmp/ngram-out/3gram.arpa").expect("3gram");
    let c_tg = Converter::new(Config::S2t).with_ngram(model, NgramMode::Trigram);
    for s in &cases {
        println!("  {:20} → {:20}", s, c_tg.convert(s));
    }
}
