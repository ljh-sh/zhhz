//! Command-line interface for `zhhz`.
//!
//! The argument parser is hand-rolled (no clap) to keep the dependency tree
//! tiny and the static binary small. Supported form:
//!
//! ```text
//! zhhz [--config <CONFIG>] [--dict <FILE>...] [--in-place] [--list] [FILE...]
//! zhhz --help | --version
//! ```

use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::engine::{Config, Converter};
use crate::DEFAULT_CONFIG;

const HELP: &str = "\
zhhz — self-contained Simplified/Traditional Chinese converter

USAGE:
    zhhz [--config <CONFIG>] [--dict <FILE>...] [--in-place] [--list] [FILE...]
    zhhz < input.txt > output.txt

Reads from stdin when no FILE is given. Use '-' for stdin.

OPTIONS:
    -c, --config <CONFIG>   Conversion config (default: s2t). One of:
                             s2t t2s s2tw tw2s s2hk hk2s s2twp tw2sp
                             s2hkp hk2sp t2tw tw2t t2hk hk2t t2jp jp2t
        --dict <FILE>       Custom dictionary (TSV: key<TAB>value), highest
                             priority. May be repeated. '#' lines are ignored.
    -i, --in-place          Rewrite each input FILE in place (not stdin).
    -l, --list              List available configs and exit.
    -h, --help              Show this help.
    -V, --version           Show version.

EXAMPLES:
    echo '汉字' | zhhz                       # s2t: 漢字
    echo '漢字' | zhhz -c t2s                # t2s: 汉字
    echo '鼠标' | zhhz -c s2tw               # s2tw: 滑鼠
    zhhz -c s2t --dict mywords.txt input.txt # with custom conversions
";

pub struct Cli {
    pub config: String,
    pub dicts: Vec<PathBuf>,
    pub in_place: bool,
    pub list: bool,
    pub files: Vec<PathBuf>,
}

enum Action {
    Run(Cli),
    Help,
    Version,
}

fn parse_args(argv: Vec<String>) -> Result<Action> {
    let mut cli = Cli {
        config: DEFAULT_CONFIG.to_string(),
        dicts: Vec::new(),
        in_place: false,
        list: false,
        files: Vec::new(),
    };
    let mut args = argv.into_iter().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Action::Help),
            "-V" | "--version" => return Ok(Action::Version),
            "-l" | "--list" => cli.list = true,
            "-i" | "--in-place" => cli.in_place = true,
            "-c" | "--config" => {
                cli.config = take_value(&mut args, "--config")?;
            }
            "--dict" => {
                cli.dicts
                    .push(PathBuf::from(take_value(&mut args, "--dict")?));
            }
            "--" => {
                cli.files.extend(args.by_ref().map(PathBuf::from));
                break;
            }
            other if other.starts_with("--") => {
                // Handle `--flag=value` and reject `--in-place=...` / `--list=...`.
                let (flag, val) = match other.split_once('=') {
                    Some((f, v)) => (f, Some(v)),
                    None => (other, None),
                };
                match flag {
                    "--config" => {
                        cli.config = take_owned_value(flag, val)?;
                    }
                    "--dict" => cli.dicts.push(PathBuf::from(take_owned_value(flag, val)?)),
                    "--in-place" | "--list" => {
                        if val.is_some() {
                            return Err(anyhow::anyhow!("option {flag} does not take a value"));
                        }
                        // already handled as a flag above when no '='; no-op here
                    }
                    _ => return bail_unknown(other),
                }
            }
            other if other.starts_with('-') && other.len() > 1 => return bail_unknown(other),
            _ => cli.files.push(PathBuf::from(arg)),
        }
    }
    Ok(Action::Run(cli))
}

fn take_value<I: Iterator<Item = String>>(
    args: &mut std::iter::Peekable<I>,
    name: &str,
) -> Result<String> {
    match args.next() {
        Some(v) => Ok(v),
        None => Err(anyhow::anyhow!("{name} requires a value")),
    }
}

fn take_owned_value(flag: &str, val: Option<&str>) -> Result<String> {
    val.map(str::to_string)
        .ok_or_else(|| anyhow::anyhow!("{flag} requires a value"))
}

fn bail_unknown(arg: &str) -> Result<Action> {
    Err(anyhow::anyhow!("unknown option: {arg}\nsee `zhhz --help`"))
}

/// Entry point invoked by `main`.
pub fn run() -> Result<()> {
    match parse_args(std::env::args().collect())? {
        Action::Help => {
            println!("{HELP}");
            Ok(())
        }
        Action::Version => {
            println!("zhhz {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Action::Run(cli) => run_cli(cli),
    }
}

fn run_cli(cli: Cli) -> Result<()> {
    if cli.list {
        list_configs();
        return Ok(());
    }

    let config =
        Config::parse(&cli.config).map_err(|e| anyhow::anyhow!("invalid --config: {e}"))?;
    let custom = load_custom_dicts(&cli.dicts)?;
    let converter = Converter::with_custom(config, &custom);

    let read_stdin = cli.files.is_empty() || cli.files.iter().any(|f| f == &PathBuf::from("-"));

    if read_stdin && !cli.in_place {
        let mut input = String::new();
        std::io::stdin()
            .lock()
            .read_to_string(&mut input)
            .context("failed to read stdin")?;
        write_all_stdout(&converter.convert(&input))?;
        return Ok(());
    }

    for file in &cli.files {
        if file == &PathBuf::from("-") {
            continue;
        }
        let input = std::fs::read_to_string(file)
            .with_context(|| format!("failed to read {}", file.display()))?;
        let output = converter.convert(&input);
        if cli.in_place {
            std::fs::write(file, output.as_bytes())
                .with_context(|| format!("failed to write {}", file.display()))?;
        } else {
            write_all_stdout(&output)?;
        }
    }
    Ok(())
}

fn write_all_stdout(text: &str) -> Result<()> {
    let stdout = std::io::stdout().lock();
    let mut handle = std::io::BufWriter::new(stdout);
    handle
        .write_all(text.as_bytes())
        .context("failed to write stdout")?;
    Ok(())
}

fn load_custom_dicts(paths: &[PathBuf]) -> Result<Vec<(String, String)>> {
    let mut entries = Vec::new();
    for path in paths {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read custom dict {}", path.display()))?;
        for line in text.lines() {
            let line = line.trim_end_matches('\r');
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, vals)) = line.split_once('\t') else {
                continue;
            };
            let Some(value) = vals.split_whitespace().next() else {
                continue;
            };
            entries.push((key.to_string(), value.to_string()));
        }
    }
    Ok(entries)
}

fn list_configs() {
    println!("{:<8}  DIRECTION", "CONFIG");
    for cfg in Config::ALL {
        println!("{:<8}  {}", cfg.name(), cfg.description());
    }
}
