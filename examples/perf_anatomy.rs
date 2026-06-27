// Detailed perf anatomy: separate build phase from hot loop.
//
// 1. Build phase (trie construction)
// 2. Hot loop (convert 10 MB realistic corpus, 30 runs)
// 3. Per-char cost: total time / char count
// 4. Per-byte cost: total time / byte count
//
// Run with /usr/bin/time -l to also get instruction/cycle counts.
use std::time::Instant;
use zhhz::{Config, Converter};

fn main() {
    let base = "汉字计算机软件繁体网络数据库服务器汉字字符串格式输出输入文件目录路径\
        系统应用软件程序代码语言框架结构算法数据结构类型变量函数参数返回值\
        用户界面交互设计模式实现机制原理方法技巧经验总结归纳推理演绎证明反证\
        中文繁简体转换工具命令行界面操作使用说明文档帮助支持维护更新版本号";
    let mut text = String::new();
    while text.len() < 10 * 1024 * 1024 {
        text.push_str(base);
    }
    let char_count = text.chars().count();
    let byte_count = text.len();
    eprintln!("corpus: {} bytes, {} chars", byte_count, char_count);

    // Phase 1: build (cold start)
    let build_start = Instant::now();
    let c = Converter::new(Config::S2t);
    let build_elapsed = build_start.elapsed();
    eprintln!("build (cold, 1 Converter): {:.2} ms", build_elapsed.as_secs_f64() * 1000.0);

    // Phase 2: warmup (trie + dict are now cached; this exercises allocators)
    for _ in 0..3 {
        let _ = c.convert(&text);
    }

    // Phase 3: hot loop
    let runs = 30;
    let hot_start = Instant::now();
    let mut total_out_bytes = 0u64;
    for _ in 0..runs {
        let out = c.convert(&text);
        total_out_bytes += out.len() as u64;
    }
    let hot_elapsed = hot_start.elapsed();
    let per_run = hot_elapsed.as_secs_f64() / runs as f64;
    let mbps = byte_count as f64 / 1_048_576.0 / per_run;
    eprintln!("hot loop: {:.2} ms / run, {:.2} MB/s",
        per_run * 1000.0, mbps);
    eprintln!("output: {} bytes / run ({} bytes input)", total_out_bytes / runs, byte_count);
    eprintln!("per-char hot loop: {:.1} ns/char, {:.2} ns/byte",
        per_run * 1_000_000_000.0 / char_count as f64,
        per_run * 1_000_000_000.0 / byte_count as f64);
}