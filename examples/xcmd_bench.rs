// Emit zhhz S2T output for the x-cmd corpus (or any text file).
// Usage:
//   xcmd_bench <mode> <input.txt> <output.txt>
//   mode = fast | bigram | trigram
use std::env;
use std::fs;
use std::time::Instant;

use zhhz::{Config, Converter, NgramMode, NgramModel};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("usage: xcmd_bench <mode> <input.txt> <output.txt>");
        eprintln!("  mode = fast | bigram | trigram");
        std::process::exit(2);
    }
    let mode = &args[1];
    let input_path = &args[2];
    let output_path = &args[3];

    let text = fs::read_to_string(input_path).expect("read input");

    let mut c = Converter::new(Config::S2t);
    match mode.as_str() {
        "fast" => {}
        "bigram" => {
            let model = NgramModel::from_file("/tmp/ngram-out/2gram.arpa").expect("2gram");
            c = c.with_ngram(model, NgramMode::Bigram);
        }
        "trigram" => {
            let model = NgramModel::from_file("/tmp/ngram-out/3gram.arpa").expect("3gram");
            c = c.with_ngram(model, NgramMode::Trigram);
        }
        other => {
            eprintln!("unknown mode: {}", other);
            std::process::exit(2);
        }
    }

    let t0 = Instant::now();
    let out = c.convert(&text);
    let elapsed = t0.elapsed();
    fs::write(output_path, &out).expect("write output");

    let cn_chars_in = text.chars().filter(|c| '一' <= *c && *c <= '鿿').count();
    let cn_chars_out = out.chars().filter(|c| '一' <= *c && *c <= '鿿').count();
    eprintln!(
        "mode={} in={}B out={}B cn_in={} cn_out={} wall={:.2?} ({:.1} KB/s)",
        mode,
        text.len(),
        out.len(),
        cn_chars_in,
        cn_chars_out,
        elapsed,
        text.len() as f64 / 1024.0 / elapsed.as_secs_f64(),
    );
}
