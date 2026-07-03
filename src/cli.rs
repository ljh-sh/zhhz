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
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::engine::{region_pair_config, Config, Converter, NgramMode, Region};
use crate::ngram::NgramModel;
use crate::{detect, DEFAULT_CONFIG};

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
        --auto              Detect each input's script variant and convert
                             it to Simplified (cn-s). For Japanese input,
                             runs a 2-stage pipeline (jp2t then t2s).
        --dict <FILE>       Custom dictionary (TSV: key<TAB>value), highest
                             priority. May be repeated. '#' lines are ignored.
        --ngram <FILE>      Path to an ARPA n-gram model for multi-value
                             character disambiguation. Required when
                             --bigram or --trigram is used; ignored
                             otherwise. Model files are not shipped in
                             this crate — see ljh-sh/ngram-exp.
        --fast              Disable n-gram disambig (dict-only). This is
                             the original zhhz behaviour. Mutually
                             exclusive with --bigram / --trigram.
        --bigram            Use a 2-gram model for multi-value disambig.
                             Implies --ngram. Mutually exclusive with
                             --fast / --trigram.
        --trigram           (Default when --ngram is given without
                             --bigram / --fast.) Use a 3-gram model for
                             multi-value disambig. Implies --ngram.
        --files-from <PATH|->  Read a newline-separated list of paths from
                             a file or stdin ('-'). Directories are walked
                             recursively.
    -0, --null              With --files-from, paths are NUL-separated.
    -i, --in-place          Rewrite each input FILE in place (not stdin).
    -l, --list, --ls        List regions and supported conversions.
    -h, --help              Show this help.
    -V, --version           Show version.

EXAMPLES:
    echo '汉字' | zhhz --from cn-s --to cn-t      # 漢字
    echo '鼠标' | zhhz --from cn-s --to cn-tw     # 滑鼠
    echo '万与两' | zhhz --auto                   # detect -> Simplified
    zhhz -c s2t --dict mywords.txt input.txt      # legacy --config form
    cat urls.txt | zhhz --files-from -            # chardet-style batch
    zhhz --trigram --ngram 3gram.arpa input.txt  # 齣/出 disambig
    zhhz --fast input.txt                         # byte-level same as v0.6
";

pub struct Cli {
    pub config: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub dicts: Vec<PathBuf>,
    pub in_place: bool,
    pub list: bool,
    pub files: Vec<PathBuf>,
    pub auto: bool,
    pub files_from: Option<PathBuf>,
    pub null: bool,
    pub ngram: Option<PathBuf>,
    pub mode_flag: ModeFlag,
}

/// User-facing mode selection. `Default` means: no flag given; if a
/// `--ngram` file is also given, the default is Trigram; otherwise Off.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ModeFlag {
    Default,
    Fast,
    Bigram,
    Trigram,
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
        auto: false,
        files_from: None,
        null: false,
        ngram: None,
        mode_flag: ModeFlag::Default,
    };
    let mut args = argv.into_iter().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => return Ok(Action::Help),
            "-V" | "--version" => return Ok(Action::Version),
            "-l" | "--list" | "--ls" => cli.list = true,
            "-i" | "--in-place" => cli.in_place = true,
            "--auto" => cli.auto = true,
            "-0" | "--null" => cli.null = true,
            "--fast" => cli.mode_flag = ModeFlag::Fast,
            "--bigram" => cli.mode_flag = ModeFlag::Bigram,
            "--trigram" => cli.mode_flag = ModeFlag::Trigram,
            "--ngram" => {
                cli.ngram = Some(PathBuf::from(take_value(&mut args, "--ngram")?));
            }
            "--files-from" => {
                cli.files_from = Some(PathBuf::from(take_value(&mut args, "--files-from")?));
            }
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
                    "--ngram" => {
                        cli.ngram = Some(PathBuf::from(take_owned_value(flag, val)?));
                    }
                    "--auto" | "--in-place" | "--list" => {
                        if val.is_some() {
                            return Err(anyhow::anyhow!("option {flag} does not take a value"));
                        }
                    }
                    "--files-from" => {
                        cli.files_from = Some(PathBuf::from(take_owned_value(flag, val)?));
                    }
                    s if s == "-0" || s == "--null" => {} // handled by the other branch
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
    // Mode flag mutual exclusion
    let explicit = [ModeFlag::Fast, ModeFlag::Bigram, ModeFlag::Trigram]
        .iter()
        .filter(|m| cli.mode_flag == **m)
        .count();
    if explicit > 1 {
        return Err(anyhow::anyhow!(
            "--fast, --bigram and --trigram are mutually exclusive"
        ));
    }
    // If a ngram file is given without an explicit mode flag, default
    // to --trigram.
    if cli.mode_flag == ModeFlag::Default && cli.ngram.is_some() {
        cli.mode_flag = ModeFlag::Trigram;
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
    let argv: Vec<String> = std::env::args().collect();
    // Route to `detect` subcommand if the first non-flag arg is "detect".
    if argv.iter().skip(1).any(|a| a == "detect") {
        // Strip the "detect" token and forward the rest to `detect::run`.
        let mut rest = Vec::with_capacity(argv.len());
        let mut skipping = false;
        for a in argv.iter().skip(1) {
            if !skipping && a == "detect" {
                skipping = true;
                continue;
            }
            rest.push(a.clone());
        }
        return detect::run(&rest);
    }
    match parse_args(argv)? {
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

/// Resolve the chardet-style input set:
///
/// * `--files-from <PATH|->` reads a newline-separated (or NUL-separated
///   with `-0`) list of paths from a file or from stdin.
/// * Positional arguments are treated as files; directories are walked
///   recursively (regular files only, sorted).
/// * `-` (anywhere) means "read content from stdin"; multiple `-` collapse
///   into a single stdin read.
///
/// Returns `(path, content)` per input. For files the content is read
/// eagerly. For stdin (path == `-`) the content string is empty; the caller
/// must read stdin exactly once when it encounters this entry.
fn resolve_convert_inputs(cli: &Cli) -> Result<Vec<(PathBuf, String)>> {
    let mut paths: Vec<PathBuf> = cli.files.clone();
    if let Some(src) = &cli.files_from {
        let raw = if src == &PathBuf::from("-") {
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .context("read stdin")?;
            buf
        } else {
            std::fs::read(src).with_context(|| format!("read file list {}", src.display()))?
        };
        let sep: u8 = if cli.null { 0 } else { b'\n' };
        for chunk in raw.split(|&b| b == sep) {
            let s = String::from_utf8_lossy(chunk).trim().to_string();
            if s.is_empty() {
                continue;
            }
            paths.push(PathBuf::from(s));
        }
    }
    let mut out: Vec<(PathBuf, String)> = Vec::new();
    let mut stdin_seen = false;
    for p in &paths {
        if p == &PathBuf::from("-") {
            if !stdin_seen {
                out.push((PathBuf::from("-"), String::new()));
                stdin_seen = true;
            }
            continue;
        }
        if p.is_dir() {
            for entry in walk_dir(p)? {
                let data = std::fs::read_to_string(&entry)
                    .with_context(|| format!("read {}", entry.display()))?;
                out.push((entry, data));
            }
        } else {
            let data =
                std::fs::read_to_string(p).with_context(|| format!("read {}", p.display()))?;
            out.push((p.clone(), data));
        }
    }
    if out.is_empty() {
        out.push((PathBuf::from("-"), String::new()));
    }
    Ok(out)
}

fn walk_dir(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(p) = stack.pop() {
        for entry in std::fs::read_dir(&p).with_context(|| format!("read_dir {}", p.display()))? {
            let entry = entry.with_context(|| format!("read_dir entry in {}", p.display()))?;
            let ft = entry.file_type().with_context(|| "file_type")?;
            if ft.is_dir() {
                stack.push(entry.path());
            } else if ft.is_file() {
                files.push(entry.path());
            }
        }
    }
    files.sort();
    Ok(files)
}

fn write_one(path: &Path, content: &str, cli: &Cli) -> Result<()> {
    if cli.in_place && path.as_os_str() != "-" {
        std::fs::write(path, content.as_bytes())
            .with_context(|| format!("failed to write {}", path.display()))?;
    } else {
        write_all_stdout(content)?;
    }
    Ok(())
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

/// Load the ARPA n-gram model and translate the user's `ModeFlag` to the
/// engine `NgramMode`. Returns `Ok(None)` for the fast path.
fn load_ngram(cli: &Cli) -> Result<Option<(NgramModel, NgramMode)>> {
    match cli.mode_flag {
        ModeFlag::Default | ModeFlag::Fast => {
            if cli.ngram.is_some() {
                // Spec: --ngram is silently ignored in fast mode. (Not
                // an error; we just don't load the model.)
            }
            Ok(None)
        }
        ModeFlag::Bigram | ModeFlag::Trigram => {
            let path = cli.ngram.as_ref().ok_or_else(|| {
                anyhow::anyhow!(
                    "--{} requires --ngram <arpa-file>",
                    match cli.mode_flag {
                        ModeFlag::Bigram => "bigram",
                        ModeFlag::Trigram => "trigram",
                        _ => unreachable!(),
                    }
                )
            })?;
            let model = NgramModel::from_file(path.to_str().ok_or_else(|| {
                anyhow::anyhow!("n-gram path is not valid UTF-8: {}", path.display())
            })?)
            .with_context(|| format!("failed to load n-gram model {}", path.display()))?;
            let mode = match cli.mode_flag {
                ModeFlag::Bigram => NgramMode::Bigram,
                ModeFlag::Trigram => NgramMode::Trigram,
                _ => unreachable!(),
            };
            Ok(Some((model, mode)))
        }
    }
}

fn run_cli(cli: Cli) -> Result<()> {
    if cli.list {
        list_regions_and_configs();
        return Ok(());
    }

    let custom = load_custom_dicts(&cli.dicts)?;
    let ngram = load_ngram(&cli)?;

    // Gather inputs: positional files, plus any from --files-from, with dirs
    // walked recursively (chardet pattern). `None` in the returned Vec means
    // "read content from stdin once" (a single placeholder, even if `-`
    // appears multiple times).
    let inputs = resolve_convert_inputs(&cli)?;

    if cli.auto {
        // --auto: detect each input's script variant and convert it to
        // Simplified (cn-s). Each region's direct-to-cn-s config is used;
        // for jp-n (shinjitai) a 2-stage pipeline jp2t → t2s is required.
        let mut t2s = Converter::with_custom(Config::T2s, &custom);
        let mut tw2sp = Converter::with_custom(Config::Tw2sp, &custom);
        let mut hk2sp = Converter::with_custom(Config::Hk2sp, &custom);
        let mut jp2t = Converter::with_custom(Config::Jp2t, &custom);
        if let Some((model, mode)) = &ngram {
            t2s = t2s.with_ngram(clone_ngram(model), *mode);
            tw2sp = tw2sp.with_ngram(clone_ngram(model), *mode);
            hk2sp = hk2sp.with_ngram(clone_ngram(model), *mode);
            jp2t = jp2t.with_ngram(clone_ngram(model), *mode);
        }
        for (path, mut content) in inputs {
            if path == std::path::Path::new("-") && content.is_empty() {
                std::io::stdin()
                    .lock()
                    .read_to_string(&mut content)
                    .context("failed to read stdin")?;
            }
            let det = crate::detect::detect_text(&content);
            let out = match det.map(|d| d.region) {
                Some(Region::CnS) => content,
                Some(Region::CnT) => t2s.convert(&content),
                Some(Region::CnTw) => tw2sp.convert(&content),
                Some(Region::CnHk) => hk2sp.convert(&content),
                Some(Region::JpN) => t2s.convert(&jp2t.convert(&content)),
                Some(Region::JpT) => t2s.convert(&content),
                None => content, // unknown: pass through
            };
            write_one(&path, &out, &cli)?;
        }
        return Ok(());
    }

    let config = resolve_config(&cli)?;
    let mut converter = Converter::with_custom(config, &custom);
    if let Some((model, mode)) = ngram {
        converter = converter.with_ngram(model, mode);
    }

    for (path, mut content) in inputs {
        if path == std::path::Path::new("-") && content.is_empty() {
            std::io::stdin()
                .lock()
                .read_to_string(&mut content)
                .context("failed to read stdin")?;
        }
        let output = converter.convert(&content);
        write_one(&path, &output, &cli)?;
    }
    Ok(())
}

/// Cheap-ish clone of the model for the 4 --auto converters: deep-clone
/// the HashMaps. ARPA models are typically a few MB; this is fine.
fn clone_ngram(m: &NgramModel) -> NgramModel {
    m.clone_model()
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
