// Benchmark zhhz modes (--fast / --bigram / --trigram) with and
// without --report, plus opencc for comparison.
//
// Measures: throughput (MB/s) on 10 MB Chinese text, plus the
// overhead of running the multi-value scan (report on vs off).
use std::time::Instant;
use zhhz::{Config, Converter, NgramMode, NgramModel};

fn main() {
    let base: &str = "一出好戏一出机场一出戏院一出子看戏这是一出好戏\
        这出戏真好看戏齣了一半像象的差别里裏街道他出去了\
        计算机软件信息网络数据库服务器计算机汉字字符串格式输出";
    let mut text = String::new();
    while text.len() < 10 * 1024 * 1024 {
        text.push_str(base);
    }
    eprintln!("text size: {:.2} MB", text.len() as f64 / 1_048_576.0);

    let model = match NgramModel::from_file("/tmp/ngram-out/2gram.arpa") {
        Ok(m) => m,
        Err(e) => {
            eprintln!("model not loaded: {e}");
            return;
        }
    };

    // Pre-build one converter per (mode, report-on/off) combination.
    let fast = Converter::new(Config::S2t);
    let bigram =
        Converter::new(Config::S2t).with_ngram(model.clone_model(), NgramMode::Bigram);
    let trigram =
        Converter::new(Config::S2t).with_ngram(model.clone_model(), NgramMode::Trigram);

    let runs = 3;
    let mbps = |us: u128| text.len() as f64 / 1_048_576.0 / (us as f64 / 1_000_000.0);
    let ms = |us: u128| us as f64 / runs as f64 / 1000.0;

    println!("{:<18}  {:<10}  {:>10}  {:>10}", "mode", "report", "avg (ms)", "MB/s");
    println!("{}", "-".repeat(54));

    // fast, no report
    let t = time_it(runs, || { fast.convert(&text); });
    println!("{:<18}  {:<10}  {:>10.1}  {:>10.2}", "fast", "off", ms(t), mbps(t / runs as u128));

    // fast, with report
    let t = time_it(runs, || { let _ = fast.convert_with_report(&text); });
    println!("{:<18}  {:<10}  {:>10.1}  {:>10.2}", "fast", "on", ms(t), mbps(t / runs as u128));

    // bigram, no report
    let t = time_it(runs, || { bigram.convert(&text); });
    println!("{:<18}  {:<10}  {:>10.1}  {:>10.2}", "bigram", "off", ms(t), mbps(t / runs as u128));

    // bigram, with report
    let t = time_it(runs, || { let _ = bigram.convert_with_report(&text); });
    println!("{:<18}  {:<10}  {:>10.1}  {:>10.2}", "bigram", "on", ms(t), mbps(t / runs as u128));

    // trigram, no report
    let t = time_it(runs, || { trigram.convert(&text); });
    println!("{:<18}  {:<10}  {:>10.1}  {:>10.2}", "trigram", "off", ms(t), mbps(t / runs as u128));

    // trigram, with report
    let t = time_it(runs, || { let _ = trigram.convert_with_report(&text); });
    println!("{:<18}  {:<10}  {:>10.1}  {:>10.2}", "trigram", "on", ms(t), mbps(t / runs as u128));

    // opencc
    let _ = std::fs::write("/tmp/bench-input.txt", &text);
    let t = time_it(runs, || {
        let out = std::process::Command::new("opencc")
            .args(["-c", "s2t.json", "-i", "/tmp/bench-input.txt"])
            .output()
            .expect("opencc");
        assert!(out.status.success());
    });
    println!("{:<18}  {:<10}  {:>10.1}  {:>10.2}", "opencc s2t", "-", ms(t), mbps(t / runs as u128));

    // Multi-value decision count for the sample text (to size the
    // report overhead vs. text size).
    let (_, decs) = fast.convert_with_report(&text);
    eprintln!("\nmulti-value decisions in 10 MB text: {}", decs.len());
    eprintln!("(that's {} decisions per MB)", decs.len() as f64 / 10.0);
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
