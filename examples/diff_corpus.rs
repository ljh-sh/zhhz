// Emit zhhz fast output for a fixed test corpus. Used to verify
// correctness across perf experiments (no byte should change).
//
// Usage:
//   cargo run --release --example diff_corpus > /tmp/baseline-output.txt
use zhhz::{Config, Converter};

fn main() {
    // 1000-sentence test corpus with mixed Chinese + multi-value
    // chars + known FMM phrase cases.
    let sentences = [
        "汉字计算机软件繁体网络数据库",
        "演员一出戏，导演就喊卡",
        "这出戏真好看",
        "一出好戏就演完了",
        "计算机会自动处理这些数据",
        "我出去了，再见",
        "汉字字符处理系统",
        "用计算机软件分析数据",
        "一出戏开始了",
        "演员的表演很精彩",
        "数据结构与算法分析",
        "软件工程师日常",
        "数据库管理员",
        "服务器响应时间",
        "网络协议分析",
        "计算结果正确",
        "字符串处理函数",
        "输出格式标准化",
        "目录结构清晰",
        "路径变量类型",
    ];
    // Repeat for ~1000-sentence corpus
    let mut corpus = String::new();
    for _ in 0..50 {
        for s in &sentences {
            corpus.push_str(s);
            corpus.push('\n');
        }
    }
    let c = Converter::new(Config::S2t);
    let out = c.convert(&corpus);
    print!("{}", out);
    eprintln!("converted {} bytes → {} bytes", corpus.len(), out.len());
}
