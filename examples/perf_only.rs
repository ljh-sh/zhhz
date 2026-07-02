// Single-call measurement: time exactly one convert() call.
// Use /usr/bin/time -l around this to get instructions/cycles.
//
// Usage:
//   /usr/bin/time -l ./target/release/examples/perf_only [mode]
use std::time::Instant;
use zhhz::{Config, Converter, NgramMode, NgramModel};

fn main() {
    let arg_mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "fast".to_string());

    // Arg 4 (optional): path to a corpus file. If absent, use built-in realistic.
    let text: String = if let Some(path) = std::env::args().nth(4) {
        std::fs::read_to_string(&path).expect("read corpus")
    } else {
        let base = REALISTIC_BASE;
        let mut text = String::new();
        while text.len() < 10 * 1024 * 1024 {
            text.push_str(base);
        }
        text
    };

    let mut c = Converter::new(Config::S2t);
    if arg_mode != "fast" {
        let model_path = match arg_mode.as_str() {
            "trigram" => "/tmp/ngram-out/3gram.arpa",
            _ => "/tmp/ngram-out/2gram.arpa",
        };
        let model = NgramModel::from_file(model_path).expect("ngram model");
        let mode = match arg_mode.as_str() {
            "bigram" => NgramMode::Bigram,
            "trigram" => NgramMode::Trigram,
            _ => unreachable!(),
        };
        c = c.with_ngram(model, mode);
    }

    // Hot: warmup + measured runs
    // Arg 2 = total measured runs. Arg 3 (optional) = warmup runs.
    let runs: usize = std::env::args()
        .nth(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    let warmup: usize = std::env::args()
        .nth(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(3);

    // Warmup
    for _ in 0..warmup {
        let _ = c.convert(&text);
    }

    let t0 = Instant::now();
    let mut out_bytes = 0usize;
    for _ in 0..runs {
        let out = c.convert(&text);
        out_bytes = out.len();
    }
    let elapsed = t0.elapsed();
    let per_run_ms = elapsed.as_secs_f64() * 1000.0 / runs as f64;
    let mbps = text.len() as f64 / 1_048_576.0 / (elapsed.as_secs_f64() / runs as f64);

    eprintln!("runs: {}", runs);
    eprintln!("per_run: {:.2} ms", per_run_ms);
    eprintln!("throughput: {:.2} MB/s", mbps);
    eprintln!("out_bytes: {}", out_bytes);
    eprintln!("input_bytes: {}", text.len());
    println!("done");
}

const REALISTIC_BASE: &str = "汉字计算机软件繁体网络数据库服务器汉字字符串格式输出输入文件目录路径\
    系统应用软件程序代码语言框架结构算法数据结构类型变量函数参数返回值\
    用户界面交互设计模式实现机制原理方法技巧经验总结归纳推理演绎证明反证\
    中文繁简体转换工具命令行界面操作使用说明文档帮助支持维护更新版本号";
