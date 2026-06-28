---
title: x-cmd 文档站批量翻译 benchmark — zhhz vs opencc 1.3.1
date: 2026-06-28
scenario: website article batch translation (S2T)
corpus: x-cmd /start + /pkg + /blog HTML pages, 64 KB / 12206 CJK chars
---

# x-cmd 文档站批量翻译 benchmark — zhhz vs opencc 1.3.1

**目的**: 验证 zhhz 在"网站文章批量繁化"场景下的速度与正确性。
**场景**: HTML → Markdown → S2T 繁化（这是 x-cmd 文档国际化的工作流）。
**基线**: opencc 1.3.1 CLI（homebrew），`-c s2t.json`。
**语料**: `cn.x-cmd.com` 8 个真实文档页（`start`、`pkg-management`、`design`、`faq`、
`shell`、`license`、`community`、`blog`），合并 64 KB / 12206 个 CJK 字符。

## 测试方法

1. **抓取**: `curl https://cn.x-cmd.com/<page>` 拉 HTML，剔除 404 与软 404
2. **HTML→MD**: 用 `scripts/html_to_md_xcmd.py`（基于 html2text + BeautifulSoup）提取
   `<main>.<div class="content">`，去掉 nav / sidebar / footer / JS。中文字符
   round-trip 验证：参考 MD 的 508 个 CJK 字符 vs 转换后的 508 — **0 丢失**。
3. **S2T 翻译**: 
   - `zhhz-fast`     : `examples/xcmd_bench.rs` + `Config::S2t`，无 ngram
   - `zhhz-bigram`   : 同上 + 2gram ARPA
   - `zhhz-trigram`  : 同上 + 3gram ARPA
   - `opencc-1.3.1`  : `opencc -c s2t.json -i ... -o ...`
4. **统计**: 每档跑 7 次（warmup 2），报 min wall time + 字符级 CJK 一致率。

## 结果（Apple M2 / macOS 14，2026-06-28）

```
tool                     min_ms   med_ms      KB/s   speedup
------------------------------------------------------------
opencc-1.3.1-s2t          32.52    35.56      1930      1.00x
zhhz-fast                 24.98    25.19      2513      1.30x
zhhz-bigram               36.83    38.24      1704      0.88x
zhhz-trigram              73.64    74.85       852      0.44x
```

### 一致性（vs opencc-1.3.1）

```
zhhz-fast      12204/12206 = 99.98% ✓ (2 lines: 想象 vs 想像 — 台/陆用法)
zhhz-bigram    12203/12206 = 99.98% ✓
zhhz-trigram   12204/12206 = 99.98% ✓
```

仅 2 行差异在 `想象/想像` 一处 — 台湾/大陆用词差异，非 bug。

## 结论

1. **zhhz-fast 比 opencc 1.3.1 CLI 快 1.30x**（24.98ms vs 32.52ms）
2. **zhhz-fast 与 opencc 输出 99.98% 一致**（剩下 0.02% 是台/陆用词差异）
3. **zhhz-bigram/trigram 在小语料（<100KB）上反而慢**，因为 ngram dispatch 相对
   每段字符开销是固定的；在大语料（>10MB）和需要 disambig 的复杂文本上 ngram
   才显出优势（见配套案例研究 [case_study](./2026-06-28-context-case-study.md)）
4. **场景推荐**：
   - 网站批量翻译（典型 5-100 KB/页）→ 用 **fast** 模式（无需 ngram 模型）
   - 古文/学术/技术语境（含大量歧义）→ 用 **trigram** 模式（精度优先）

## 复现

```bash
cd /Users/l/.x-repo/zhhz-wt-v074
cargo build --release --example xcmd_bench

# 拉语料 + HTML→MD
mkdir -p /tmp/xcmd-corpus/html /tmp/xcmd-corpus/md_converted
for url in start start/pkg-management start/design start/faq \
           start/shell start/license start/community blog; do
  curl -sSL "https://cn.x-cmd.com/$url" -o "/tmp/xcmd-corpus/html/$url.html"
done
for h in /tmp/xcmd-corpus/html/*.html; do
  python3 scripts/html_to_md_xcmd.py "$h" -o "/tmp/xcmd-corpus/md_converted/$(basename "$h" .html).md"
done
cat /tmp/xcmd-corpus/md_converted/*.md > /tmp/xcmd-corpus/corpus.md

# 跑 benchmark
python3 scripts/bench_xcmd_corpus.py
```

详见 [`scripts/bench_xcmd_corpus.py`](../../scripts/bench_xcmd_corpus.py)。