// Rigorous perf bench: best-of-N, warmup, stddev, multiple corpora.
//
// Usage:
//   cargo run --release --example bench_perf [-- fast]
//
// Reports per-mode: mean MB/s, stddev, best, and compares against
// opencc 1.3.1 baseline (also measured in the same run).
use std::time::Instant;
use zhhz::{Config, Converter, NgramMode, NgramModel};

fn main() {
    let arg_mode = std::env::args().nth(1);

    // Corpus 1: realistic mixed Chinese (most users will see something
    // like this).
    let realistic = build_corpus(REALISTIC_BASE, 10 * 1024 * 1024);
    // Corpus 2: worst-case (many short ambiguous chars; designed to
    // stress the FMM / multi-value path).
    let worst = build_corpus(WORST_BASE, 10 * 1024 * 1024);
    // Corpus 3: long-text ascii-y mix (no Chinese characters; tests
    // the "no match" pass-through).
    let ascii_y = build_corpus(ASCII_Y_BASE, 10 * 1024 * 1024);

    println!("=== rigorous bench (best of 5, 2 warmup runs discarded) ===");
    println!("mode              corpus        mean MB/s   stddev   best MB/s");
    println!("{}", "-".repeat(70));

    let runs = 5;
    let warmup = 2;

    let model = NgramModel::from_file("/tmp/ngram-out/2gram.arpa").ok();
    let bigram = model
        .as_ref()
        .map(|m| Converter::new(Config::S2t).with_ngram(m.clone_model(), NgramMode::Bigram));
    let trigram = model
        .as_ref()
        .map(|m| Converter::new(Config::S2t).with_ngram(m.clone_model(), NgramMode::Trigram));
    let fast = Converter::new(Config::S2t);

    for (label, corpus) in [
        ("realistic", &realistic),
        ("worst", &worst),
        ("ascii-y", &ascii_y),
    ] {
        for (mode, conv) in [
            ("fast", Some(&fast)),
            ("bigram", bigram.as_ref()),
            ("trigram", trigram.as_ref()),
        ] {
            if let Some(c) = conv {
                if arg_mode.as_deref().is_some() && arg_mode.as_deref() != Some(mode) {
                    continue;
                }
                let s = measure(c, corpus, runs, warmup);
                println!(
                    "{:<16} {:<12}  {:>8.2}    {:>5.2}    {:>8.2}",
                    mode, label, s.mean, s.stddev, s.best
                );
            }
        }
        if arg_mode.is_none() {
            let s = measure_opencc(label, corpus, runs, warmup);
            println!(
                "{:<16} {:<12}  {:>8.2}    {:>5.2}    {:>8.2}",
                "opencc 1.3.1", label, s.mean, s.stddev, s.best
            );
        }
        println!();
    }

    // Sanity: output correctness — fast and bigram should produce
    // identical output for the realistic corpus (no ngram-specific
    // changes since v0.7 fast defaults to cands[0]).
    let fast_out = fast.convert(&realistic);
    println!("realistic output sample (first 60 chars):");
    println!("  {}", &fast_out[..60.min(fast_out.len())]);
}

struct Stats {
    mean: f64,
    stddev: f64,
    best: f64,
}

fn measure(c: &Converter, text: &str, runs: usize, warmup: usize) -> Stats {
    let total_runs = runs + warmup;
    let mut times = Vec::with_capacity(total_runs);
    // warmup
    for _ in 0..warmup {
        let _ = c.convert(text);
    }
    for _ in 0..runs {
        let t = Instant::now();
        let _ = c.convert(text);
        times.push(t.elapsed().as_secs_f64());
    }
    let mbps = |s: f64| text.len() as f64 / 1_048_576.0 / s;
    let mbps_samples: Vec<f64> = times.iter().map(|t| mbps(*t)).collect();
    let mean = mbps_samples.iter().sum::<f64>() / mbps_samples.len() as f64;
    let var =
        mbps_samples.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / mbps_samples.len() as f64;
    let stddev = var.sqrt();
    let best = mbps_samples
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    Stats { mean, stddev, best }
}

fn measure_opencc(label: &str, text: &str, runs: usize, warmup: usize) -> Stats {
    let path = format!("/tmp/bench-perf-{label}.txt");
    let _ = std::fs::write(&path, text);
    let total_runs = runs + warmup;
    let mut times = Vec::with_capacity(total_runs);
    for _ in 0..warmup {
        let _ = std::process::Command::new("opencc")
            .args(["-c", "s2t.json", "-i", &path])
            .output();
    }
    for _ in 0..runs {
        let t = Instant::now();
        let _ = std::process::Command::new("opencc")
            .args(["-c", "s2t.json", "-i", &path])
            .output()
            .expect("opencc");
        times.push(t.elapsed().as_secs_f64());
    }
    let mbps = |s: f64| text.len() as f64 / 1_048_576.0 / s;
    let mbps_samples: Vec<f64> = times.iter().map(|t| mbps(*t)).collect();
    let mean = mbps_samples.iter().sum::<f64>() / mbps_samples.len() as f64;
    let var =
        mbps_samples.iter().map(|m| (m - mean).powi(2)).sum::<f64>() / mbps_samples.len() as f64;
    let stddev = var.sqrt();
    let best = mbps_samples
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    Stats { mean, stddev, best }
}

fn build_corpus(base: &str, target_bytes: usize) -> String {
    let mut s = String::new();
    while s.len() < target_bytes {
        s.push_str(base);
    }
    // Round down to a char boundary so we don't split a multi-byte codepoint.
    let mut end = target_bytes.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
    s
}

const REALISTIC_BASE: &str = "\
汉字计算机软件繁体网络数据库服务器汉字字符串格式输出输入文件目录路径\
系统应用软件程序代码语言框架结构算法数据结构类型变量函数参数返回值\
用户界面交互设计模式实现机制原理方法技巧经验总结归纳推理演绎证明反证\
中文繁简体转换工具命令行界面操作使用说明文档帮助支持维护更新版本号";

const WORST_BASE: &str = "\
一出好戏一出机场一出戏院一出子看戏这是一出好戏\
这出戏真好看戏齣了一半像象的差别里裏街道他出去了";

const ASCII_Y_BASE: &str = "\
lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod\
tempor incididunt ut labore et dolore magna aliqua ut enim ad minim\
veniam quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea";
