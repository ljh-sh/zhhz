---
layout: default
title: CLI reference
permalink: /cli/
---

# CLI reference

The `zhhz` binary reads text from stdin or files, writes converted text to stdout, and errors to stderr. Same input → byte-identical output every time. No TUI, no progress bars, no network calls.

## Synopsis

```sh
zhhz [OPTIONS] [FILE...]
zhhz detect [OPTIONS] [FILE...]
zhhz --list
zhhz --version
zhhz --help
```

## Conversion (`zhhz convert`, default)

```sh
echo '汉字' | zhhz                       # default s2t: 漢字
echo '漢字' | zhhz -c t2s                # t2s:        汉字
echo '信息' | zhhz -c s2twp              # s2twp:      資訊
zhhz -c s2t input.txt                   # convert a file
zhhz -c s2t -i input.txt                # rewrite in place
```

| Flag | Description |
|---|---|
| `-c`, `--config <NAME>` | One of the 16 OpenCC configs (default `s2t`). See `zhhz --list`. |
| `-i`, `--in-place` | Rewrite the input file(s) instead of writing to stdout. |
| `-f`, `--from <REGION>` | Semantic source region (`cn-s`, `cn-t`, `cn-tw`, `cn-hk`, `jp-t`, `jp-n`). Alternative to `-c`. |
| `-t`, `--to <REGION>` | Semantic target region. Alternative to `-c`. |
| `--dict <PATH>` | Custom dictionary file (TSV: `key<TAB>value`). Repeatable; entries override built-in tables at the highest priority. |
| `--no-detect` | Skip automatic script-variant detection when `-f`/`-t` are not given. |

`-` as a filename means stdin. Multiple input files are processed sequentially.

## Detection (`zhhz detect`)

Mirrors [`chardet`](https://github.com/ljh-sh/chardet)'s CLI: file or stdin, `--files-from`, `-0 --null`, recursive dir walk.

```sh
echo '汉字计算机软件' | zhhz detect          # cn-s    57   -
echo '漢字計算機軟體' | zhhz detect          # cn-t    66   -
echo 'こんにちは世界' | zhhz detect          # jp-n    50   -
zhhz detect corpus.txt                      # cn-s   ...   corpus.txt
```

Output is tab-separated: `<region>\t<confidence>\t<path>`. `confidence` is 0–100 (share of signature characters in the input). `path` is `-` for stdin. Region codes are the six listed above, or `unknown` when there are no CJK characters / kana.

| Flag | Description |
|---|---|
| `--files-from <PATH\|->` | Read newline-separated list of paths from a file (or stdin with `-`). |
| `-0`, `--null` | Use NUL-separated file lists (with `--files-from`). |
| `-r`, `-R`, `--recursive` | Recursively walk directories for CJK files. |
| `-q`, `--quiet` | Suppress non-output lines (e.g. "skipping binary file"). |

## Configs

16 OpenCC configs, listed via `zhhz --list`:

| Config | Direction |
|---|---|
| `s2t` / `t2s` | Simplified ↔ Traditional (OpenCC standard) |
| `s2tw` / `tw2s` | Simplified ↔ Traditional (Taiwan) |
| `s2twp` / `tw2sp` | …with Taiwan phrases |
| `s2hk` / `hk2s` | Simplified ↔ Traditional (Hong Kong) |
| `s2hkp` / `hk2sp` | …with Hong Kong phrases |
| `t2tw` / `tw2t` | Traditional (standard) ↔ Taiwan |
| `t2hk` / `hk2t` | Traditional (standard) ↔ Hong Kong |
| `t2jp` / `jp2t` | Japanese Kyūjitai ↔ Shinjitai |

Semantic region flags (`--from` / `--to`) are an alias for the config names — `--from cn-s --to cn-tw` is equivalent to `-c s2twp` (the phrase-aware variant is preferred when both exist).

## Examples

```sh
# Taiwan phrase conversion: 鼠标 → 滑鼠
echo '鼠标' | zhhz --from cn-s --to cn-tw
# 滑鼠

# Custom terminology override
cat > /tmp/mywords.tsv <<EOF
软件	軟體
独家	獨家
EOF
echo '买软件吃独家' | zhhz -c s2t --dict /tmp/mywords.tsv
# 買軟體喫獨家

# Batch convert a directory tree
find corpus/ -name '*.txt' -print0 | xargs -0 zhhz -c s2twp -i

# Pipe through other tools
echo '汉字' | zhhz -c s2t | tr ' ' '\n' | sort | uniq -c | sort -rn | head
```

## Exit codes

| Code | Meaning |
|---|---|
| `0` | Success. |
| `1` | Conversion error (bad config name, malformed custom-dict line, etc.). |
| `2` | I/O error (file not found, permission denied, etc.). |

Errors are written to stderr with a short message. The CLI never silently fails; if you see no output, that's the answer.