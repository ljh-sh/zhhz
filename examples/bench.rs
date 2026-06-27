// Benchmark 3 zhhz modes vs opencc on a 10MB Chinese text.
use std::time::Instant;
use zhhz::{Config, Converter, NgramMode, NgramModel};

fn main() {
    // Generate a ~10MB Chinese text. Repeat a small corpus 1000x.
    // Use text that includes 齣/出, 像/象, 里/裏 to exercise the
    // n-gram disambig path.
    let base: &str = "一出好戏一出机场一出戏院一出子看戏这是一出好戏\
        这出戏真好看戏齣了一半像象的差别里裏街道他出去了\
        计算机软件信息网络数据库服务器计算机汉字字符串格式输出";
    let mut text = String::new();
    while text.len() < 10 * 1024 * 1024 {
        text.push_str(base);
    }
    eprintln!("text size: {:.2} MB", text.len() as f64 / 1_048_576.0);

    // Load ngram model
    let model = NgramModel::from_file("/tmp/ngram-out/2gram.arpa").ok();
    let model = match model {
        Some(m) => m,
        None => {
            eprintln!("model not loaded; skipping ngram benches");
            return;
        }
    };

    // Build converters
    let fast = Converter::new(Config::S2t);
    let bigram = Converter::new(Config::S2t)
        .with_ngram(model.clone_model(), NgramMode::Bigram);
    let trigram = Converter::new(Config::S2t)
        .with_ngram(model.clone_model(), NgramMode::Trigram);

    // Warmup
    let warm = text.chars().take(256).collect::<String>();
    let _ = fast.convert(&warm);

    // Time each mode
    let runs = 3;
    let t_fast = time_it(runs, || { fast.convert(&text); });
    let t_bigram = time_it(runs, || { bigram.convert(&text); });
    let t_trigram = time_it(runs, || { trigram.convert(&text); });
    let mbps = |us: u128| text.len() as f64 / 1_048_576.0 / (us as f64 / 1_000_000.0);
    println!("{:<10}  {:>12}  {:>12}", "mode", "avg (ms)", "MB/s");
    println!("{}", "-".repeat(40));
    println!("{:<10}  {:>12.1}  {:>12.2}", "fast",    t_fast as f64 / runs as f64 / 1000.0, mbps(t_fast / runs as u128));
    println!("{:<10}  {:>12.1}  {:>12.2}", "bigram",  t_bigram as f64 / runs as f64 / 1000.0, mbps(t_bigram / runs as u128));
    println!("{:<10}  {:>12.1}  {:>12.2}", "trigram", t_trigram as f64 / runs as f64 / 1000.0, mbps(t_trigram / runs as u128));

    // opencc comparison
    let _ = std::fs::write("/tmp/bench-input.txt", &text);
    let t_opencc = time_it(runs, || {
        let out = std::process::Command::new("opencc")
            .args(["-c", "s2t.json", "-i", "/tmp/bench-input.txt"])
            .output()
            .expect("opencc");
        assert!(out.status.success());
    });
    println!("{:<10}  {:>12.1}  {:>12.2}", "opencc",   t_opencc as f64 / runs as f64 / 1000.0, mbps(t_opencc / runs as u128));

    // Disambig accuracy probe (this part was already in probe.rs).
    println!();
    println!("=== Disambig accuracy ===");
    let cases: &[(&str, &str)] = &[
        ("一出好戏",          "一齣好戲"),
        ("一出机场",          "一出機場"),
        ("一出戏院",          "一出戲院"),
        ("一出机场就看到一出好戏", "一出機場就看到一齣好戲"),
        ("这出戏真好看",      "這齣戲真好看"),
        ("这出剧",            "這齣劇"),
        ("这出电影",          "這齣電影"),
        ("他出去了",          "他出去了"),
        ("看出问题",          "看出問題"),
        ("出门",              "出門"),
    ];
    println!("{:<26}  {:<26}  {:<6}  {:<6}  {:<6}  {:<6}", "input", "expected", "fast", "bigram", "trigram", "opencc");
    println!("{}", "-".repeat(96));
    for (input, expected) in cases {
        let f = fast.convert(input);
        let b = bigram.convert(input);
        let t = trigram.convert(input);
        let o = std::process::Command::new("opencc")
            .args(["-c", "s2t.json"])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut c| {
                use std::io::Write;
                c.stdin.as_mut().unwrap().write_all(input.as_bytes()).ok();
                let out = c.wait_with_output().unwrap();
                Ok(String::from_utf8_lossy(&out.stdout).into_owned())
            })
            .unwrap_or_default();
        let mark = |got: &str| if got == *expected { "OK  " } else { "FAIL" };
        println!("{:<26}  {:<26}  {:<6}  {:<6}  {:<6}  {:<6}",
            input, expected, mark(&f), mark(&b), mark(&t), mark(&o.trim()));
    }
}

fn time_it<F: FnMut()>(runs: usize, mut f: F) -> u128 {
    let mut total = 0u128;
    for _ in 0..runs {
        let start = Instant::now();
        f();
        total += start.elapsed().as_micros();
    }
    total
}
