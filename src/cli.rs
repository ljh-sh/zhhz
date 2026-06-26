//! Command-line interface for `zhhz`.
//!
//! The argument parser is hand-rolled (no clap) to keep the dependency tree
//! tiny and the static binary small. Supported form:
//!
//! ```text
//! zhhz [--config <CONFIG>] [--from <R> --to <R>] [--dict <FILE>...] [--in-place] [--list] [FILE...]
//! zhhz --help | --version
//! ```

use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::engine::{region_pair_config, Config, Converter, Region};
use crate::DEFAULT_CONFIG;

const HELP: &str = "\
zhhz — self-contained Simplified/Traditional Chinese converter

USAGE:
    zhhz [--config <CONFIG> | --from <R> --to <R>] [--dict <FILE>...] [--in-place] [FILE...]
    zhhz --list
    zhhz < input.txt > output.txt

Reads from stdin when no FILE is given. Use '-' for stdin.

OPTIONS:
    -c, --config <CONFIG>   Conversion config (default: s2t). One of:
                             s2t t2s s2tw tw2s s2hk hk2s s2twp tw2sp
                             s2hkp hk2sp t2tw tw2t t2hk hk2t t2jp jp2t
        --from <REGION>     Source script region (alternative to --config).
        --to   <REGION>     Target script region (requires --from).
                             Regions: cn-s cn-t cn-tw cn-hk jp-t jp-n
        --dict <FILE>       Custom dictionary (TSV: key<TAB>value), highest
                             priority. May be repeated. '#' lines are ignored.
    -i, --in-place          Rewrite each input FILE in place (not stdin).
    -l, --list              List regions and supported conversions.
    -h, --help              Show this help.
    -V, --version           Show version.

EXAMPLES:
    echo '汉字' | zhhz --from cn-s --to cn-t      # 漢字
    echo '汉字' | zhhz --from cn-s --to cn-tw     # 資訊-style Taiwan phrases
    echo '鼠标' | zhhz --from cn-s --to cn-tw     # 滑鼠
    echo '漢字' | zhhz --from cn-tw --to cn-s     # simplified
    zhhz -c s2t --dict mywords.txt input.txt      # legacy --config form
";

pub struct Cli {
    pub config: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
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
        config: None,
        from: None,
        to: None,
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
                cli.config = Some(take_value(&mut args, "--config")?);
            }
            "--from" => cli.from = Some(take_value(&mut args, "--from")?),
            "--to" => cli.to = Some(take_value(&mut args, "--to")?),
            "--dict" => {
                cli.dicts
                    .push(PathBuf::from(take_value(&mut args, "--dict")?));
            }
            "--" => {
                cli.files.extend(args.by_ref().map(PathBuf::from));
                break;
            }
            other if other.starts_with("--") => {
                let (flag, val) = match other.split_once('=') {
                    Some((f, v)) => (f, Some(v)),
                    None => (other, None),
                };
                match flag {
                    "--config" => cli.config = Some(take_owned_value(flag, val)?),
                    "--from" => cli.from = Some(take_owned_value(flag, val)?),
                    "--to" => cli.to = Some(take_owned_value(flag, val)?),
                    "--dict" => cli.dicts.push(PathBuf::from(take_owned_value(flag, val)?)),
                    "--in-place" | "--list" => {
                        if val.is_some() {
                            return Err(anyhow::anyhow!("option {flag} does not take a value"));
                        }
                    }
                    _ => return bail_unknown(other),
                }
            }
            other if other.starts_with('-') && other.len() > 1 => return bail_unknown(other),
            _ => cli.files.push(PathBuf::from(arg)),
        }
    }
    if (cli.from.is_some()) != (cli.to.is_some()) {
        return Err(anyhow::anyhow!("--from and --to must be used together"));
    }
    if cli.config.is_some() && (cli.from.is_some() || cli.to.is_some()) {
        return Err(anyhow::anyhow!(
            "--config is mutually exclusive with --from/--to"
        ));
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

fn resolve_config(cli: &Cli) -> Result<Config> {
    if let (Some(from), Some(to)) = (cli.from.as_deref(), cli.to.as_deref()) {
        let f = Region::parse(from).map_err(|e| anyhow::anyhow!("invalid --from: {e}"))?;
        let t = Region::parse(to).map_err(|e| anyhow::anyhow!("invalid --to: {e}"))?;
        return region_pair_config(f, t).map_err(|e| {
            anyhow::anyhow!("{e}\nhint: try an intermediate (e.g. {from} -> cn-t -> {to})")
        });
    }
    let cfg_name = cli.config.as_deref().unwrap_or(DEFAULT_CONFIG);
    Config::parse(cfg_name).map_err(|e| anyhow::anyhow!("invalid --config: {e}"))
}

fn run_cli(cli: Cli) -> Result<()> {
    if cli.list {
        list_regions_and_configs();
        return Ok(());
    }

    let config = resolve_config(&cli)?;
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

fn list_regions_and_configs() {
    println!("{:<6}  DESCRIPTION", "REGION");
    for r in Region::ALL {
        println!("{:<6}  {}", r.code(), r.description());
    }
    println!();
    println!("{:<6}  {:<6}  OPENCC CONFIG", "FROM", "TO");
    let pairs = [
        ("cn-s", "cn-t"),
        ("cn-t", "cn-s"),
        ("cn-s", "cn-tw"),
        ("cn-tw", "cn-s"),
        ("cn-s", "cn-hk"),
        ("cn-hk", "cn-s"),
        ("cn-t", "cn-tw"),
        ("cn-tw", "cn-t"),
        ("cn-t", "cn-hk"),
        ("cn-hk", "cn-t"),
        ("jp-n", "jp-t"),
        ("jp-t", "jp-n"),
        ("jp-n", "cn-t"),
        ("cn-t", "jp-n"),
    ];
    for (f, t) in pairs {
        if let Ok(cfg) = region_pair_config(Region::parse(f).unwrap(), Region::parse(t).unwrap()) {
            println!("{:<6}  {:<6}  {}", f, t, cfg.name());
        }
    }
}
