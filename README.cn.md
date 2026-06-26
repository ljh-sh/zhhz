# zhhz

[![CI](https://github.com/ljh-sh/zhhz/actions/workflows/ci.yml/badge.svg)](https://github.com/ljh-sh/zhhz/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

> 自包含的简繁中文转换器 —— 用纯 Rust 重新实现的 [OpenCC](https://github.com/BYVoid/OpenCC)，数据全部内嵌。

`zhhz` 使用 OpenCC 词库在简体中文与繁体中文（含台湾、香港、日文新字体变体）之间互转。**所有词库在编译期内嵌进二进制** —— 单个约 1.6 MB 的静态二进制，运行时无需下载，也没有独立的数据目录。

名字是回文：**zh** hanzi（汉字），也是 **z**huan **h**uan **h**an **z**i（转换汉字）。

## 为什么做

OpenCC 是事实上的中文转换库，但它的参考实现是一个 C++ 库：依赖 CMake 构建、运行时加载 marisa-trie 二进制、且有[内存安全漏洞记录](https://github.com/BYVoid/OpenCC/issues/997)。`zhhz` 是从零开始的 Rust 移植：

- **单一自包含二进制。** 数据通过 `include_str!` 内嵌，不随安装额外抓取任何东西。
- **天然内存安全** —— 没有 C++，转换核心没有 `unsafe`。
- **支持自定义转换词**，优先级最高，适用于术语、品牌词或领域词汇。
- **持续跟进上游数据**，通过可复现的固定 SHA 同步脚本（`scripts/sync-opencc.sh`）。

## 为 AI agent 而设计

`zhhz` 首先是为 **AI agent**（Claude、Cursor、自定义 LLM 管道、批量任务）打造的工具。CLI 故意做到极简：

- **没有 TUI、没有进度条、没有动画。** 输出是 stdout 上的纯文本；错误到 stderr。Agent 可以同时捕获两者并确定性解析。
- **stdin / stdout 友好。** 管道输入，管道输出；文件作为位置参数；`-` 表示 stdin。
- **稳定、可预测、安全。** 同一输入每次输出**字节级一致**。无网络、无文件系统写入（除非显式 `--in-place`）、无临时文件、无后台进程。
- **批量 / filelist from stdin**（参考 chardet）：`<files>...`、`--files-from <PATH|->`、`-0` / `--null`、目录递归遍历。
- **单一自包含二进制。** 无原生依赖，无须附带数据文件。扔进容器里就能跑。

如果你想要花哨的交互体验，这个工具不适合——请用 OpenCC 或网页 demo。
如果你想要一个能 shell 调用或交给 agent 的工具，这就是。

## 安装

### Cargo

```bash
cargo install zhhz
```

### 直接下载二进制

```bash
curl -L https://github.com/ljh-sh/zhhz/releases/latest/download/zhhz-x86_64-unknown-linux-musl.tar.xz | tar xJ -
sudo mv zhhz-x86_64-unknown-linux-musl/bin/zhhz /usr/local/bin/
```

### 从源码构建

需要 Rust 1.74+。

```bash
git clone https://github.com/ljh-sh/zhhz
cd zhhz
cargo build --release   # 二进制在 target/release/zhhz
```

## 用法

```bash
echo '汉字' | zhhz                       # 默认 s2t：     漢字
echo '漢字' | zhhz -c t2s                # t2s：          汉字
echo '信息' | zhhz -c s2twp              # s2twp：        資訊
zhhz -c s2t input.txt                   # 转换文件
zhhz -c s2t -i input.txt                # 原地改写
zhhz --list                             # 列出全部配置
```

配置名与 OpenCC 一致：`s2t` / `t2s`、`s2tw` / `tw2s`、`s2twp` / `tw2sp`、`s2hk` / `hk2s`、`s2hkp` / `hk2sp`、`t2tw` / `tw2t`、`t2hk` / `hk2t`、`t2jp` / `jp2t`。

或使用语义化区域标志（`--from` / `--to`）：

```bash
echo '汉字'   | zhhz --from cn-s --to cn-t      # 漢字
echo '信息'   | zhhz --from cn-s --to cn-tw     # 資訊（台湾短语）
echo '鼠标'   | zhhz --from cn-s --to cn-tw     # 滑鼠
echo '漢字'   | zhhz --from cn-tw --to cn-s     # 简体
echo '万与两' | zhhz --from jp-n --to cn-t      # 萬與兩
```

区域代码：`cn-s` / `cn-t` / `cn-tw` / `cn-hk` / `jp-t` / `jp-n`。

### 检测中文文本的简繁/地区变体

```bash
echo '汉字计算机软件' | zhhz detect          # cn-s    57   -
echo '漢字計算機軟體' | zhhz detect          # cn-t    66   -
echo 'こんにちは世界' | zhhz detect          # jp-n    50   -
zhhz detect corpus.txt                      # cn-s    ...  corpus.txt
zhhz detect                                 # 从 stdin 读取内容检测
```

输出为 tab 分隔：`<region>\t<confidence>\t<path>`。置信度 0–100
（输入中特征字符的占比）。区域代码同上述六种；若不含 CJK 字符或假名则为 `unknown`。

`zhhz detect` 镜像 [`chardet`](https://github.com/ljh-sh/chardet) 的 CLI：
`<files>...` 检测每个路径；`-`（或不传参）从 stdin 读**内容**检测；`--files-from <PATH|->`
从文件/标准输入读换行分隔的路径列表；`-0` / `--null` 为 NUL 分隔；目录递归遍历。

### 自定义词典

自定义词典是 TSV 文件（`key<TAB>value`），`#` 开头的行被忽略。条目以最高优先级覆盖内置词表：

```bash
# mywords.txt
# key	value
软件	軟體
独家	獨家

echo '买软件吃独家' | zhhz -c s2t --dict mywords.txt   # 買軟體喫獨家
```

## 作为库

```rust
use zhhz::{Config, Converter};

let c = Converter::new(Config::S2t);
assert_eq!(c.convert("汉字"), "漢字");

// 自定义词覆盖内置词表。
let c = Converter::with_custom(Config::S2t, &[("软件".into(), "軟體".into())]);
assert_eq!(c.convert("买软件"), "買軟體");
```

引擎是纯 Rust，依赖极少（`serde_json`、`anyhow`），无文件系统与网络访问，便于后续绑定 WASM 与 Python（均在路线图中）。

## 工作原理

`zhhz` 严格复刻 OpenCC 的流水线：

1. 用正向最大匹配（FMM）对输入按分词词典组**分词**。
2. 每个分词片段依次通过一组有序的词典组**转换**；每一阶段对片段重新做最长前缀匹配，命中则输出第一个候选。

组匹配语义与 OpenCC 的 `PrefixMatch` 一致：有任意前缀命中的最高优先级词典胜出（跨词典时优先级高于长度；单词典内长度优先）。

OpenCC 构建系统在编译期生成 5 个词库（反序变体表、tofu-risk 子集、区域词投影）。`build.rs` 从 vendored 源数据确定性重现这 5 个文件，使 `data/` 始终是上游的纯净镜像。

## 数据与许可

词库数据 vendored 自 [BYVoid/OpenCC](https://github.com/BYVoid/OpenCC)（固定 commit 见 [`data/UPSTREAM`](data/UPSTREAM)），许可证为 **Apache-2.0**，与源码相同。重新 vendor 最新上游数据：

```bash
scripts/sync-opencc.sh            # master HEAD
scripts/sync-opencc.sh 1.3.1      # 指定 tag/commit
```

## 路线图

- [x] 纯 Rust 引擎、全部 16 个 OpenCC 配置、数据内嵌、自定义词
- [ ] 与 `opencc` CLI 的差分模糊测试，证明输出一致
- [ ] WASM 构建 + npm 包（`wasm32-unknown-unknown`）
- [ ] Python 原生扩展（PyO3 / `maturin`）
- [ ] 紧凑词库表示（FST / 双数组 trie）以缩小二进制

详见 [ROADMAP.md](ROADMAP.md)。

## 贡献

见 [CONTRIBUTING.md](CONTRIBUTING.md)。欢迎提 issue 和 PR。

## 安全

见 [SECURITY.md](SECURITY.md)。漏洞请邮件 [lijunhao@x-cmd.com](mailto:lijunhao@x-cmd.com)，勿开公开 issue。

## 许可证

Apache 2.0 —— 见 [LICENSE](LICENSE)。词库数据为 Apache-2.0，来自 OpenCC。
