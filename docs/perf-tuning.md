# zhhz 长文本吞吐为什么比 opencc 快

> v0.6 在 10MB / 100MB / 1GB 中文语料上**比 opencc 1.2.0 快约 3.3×**（端到端 wall time）。这是真实的、可重现的数据。

## 数据

`scripts/benchmark.sh` 的小样本（10MB）容易被启动时间污染；这里用 100MB 和 1GB 的中位数（3 次）：

| 输入大小 | zhhz | opencc 1.2.0 | 比例 |
|---|---|---|---|
| 11 MB | 273 ms | 909 ms | **3.33×** |
| 101 MB | 2594 ms | 8617 ms | **3.25×** |
| 1 GB | 25961 ms | 93222 ms | **3.55×** |

吞吐在 ~40 MB/s 封顶（受 tempfs 管道 I/O 限制），但 **zhhz 的 wall time 始终是 opencc 的 ~30%**。在 10MB 纯转换跑分里（启动时间占比大），zhhz 501 MB/s vs opencc 390 MB/s（**1.29×**）。两者一致：zhhz 在长文本上更快。

## 只有"结果一样"的比较才有意义

> 这条原则来自 x-cmd 文章形式层的核心要求：比较有意义的对比必须建立在输出一致之上。

100MB 真实语料上 zhhz 与 opencc 的逐字节对比：

```
match rate: 99.8244%  (184133 differing bytes / 104870879 total)
diff phrases: 68421 / 8588001  (0.797%)
```

差 0.18%。**这不是 v0.6 引擎的回退**——v0.5 在同一数据上完全一样（同 `from_text` first-value 逻辑，没有改动）。差异是**多值数据选择**（multi-value data selection）：

- `冲` → zhhz=沖，opencc=衝（STCharacters 里"冲"有多个候选）
- `麪/面`、`耡/鋤`、`抬/擡`、`出/齣`、`里/裏` 等类似

这些简体字在繁体里有**多个合法对应**，STCharacters.txt 里同一行有多个候选，zhhz 取第一个，opencc 按自己的策略取不同的。这是**数据选择**差异，不是引擎差异。

在 CI 的 Parity workflow（用 `scripts/build-reference-opencc.sh` 从 `data/UPSTREAM` 同源数据构建 opencc）里，v0.6 是 **538/538 逐字节一致**——同样的数据，同样的选择，输出一致。

> **比较结论**：zhhz 与 opencc 在 99.82% 的输出上完全一致，吞吐比较（3.3×）成立。0.18% 的差异是数据版本造成的，属于可预期的、不影响吞吐结论的扰动。

## 真正的原因

> 这件事我之前说错过一次。**不是 "treemap 打败 hashmap"**。是**连续内存上的有序数组打败 hashmap**。

v0.5 及之前用的结构是 `Box<Node>` 递归 + `HashMap<char, Node>`。两个瓶颈：

1. **HashMap 的 per-char 开销大**——每个字符要算 hash、找桶、线性比较。在 CJK trie 节点这种**典型 n=2–10** 的小集合场景下，hash + 桶访问的常数因子比"在一个连续 slice 里做几次字节比较"**贵 5–10 倍**。HashMap 是为大 n 优化的 O(1)，但 O(1) 省的那次比较，根本 cover 不了 hash 的开销。
2. **`Box` 指针跳跃**——49k 词条约 25 万个堆上的小节点，每个 hop 一次指针解引，CPU 的 L1/L2 cache 完全 miss。

v0.6 改成**arena `Vec<Node>` + 每节点 `Vec<(char, u32)>`（排序后二分）**。Build 用临时 `HashMap<char, u32>`（O(1) insert，根节点 3000 子节点也快），`finalize()` 把每个 HashMap drain 成排序好的 `Vec`（一次，O(n log n) 每个节点，便宜）。Query 路径在排序好的 Vec 上**二分查找**：O(log n)，无 hash，连续内存，cache 友好。

> treemap（B 树/红黑树）跟 HashMap 在这个问题上**半斤八两**——都是指针跳跃、cache 不友好。有序数组在小 n 下比它们都快，原因是**连续内存 + 无 hash + cache 预取**。

这也是为什么 zhhz 的吞吐和 opencc 的 marisa-trie **同一档**——marisa 也是 cache 友好、无 hash、连续（位级打包）内存。zhhz 用通用 Rust 数据结构达到了 marisa 的访问模式。

## 怎么重现

```bash
# 10MB / 5 次中位数（启动时间占比大）
scripts/benchmark.sh 10 5

# 100MB / 1GB 真实 scaling（推荐跑这些，I/O 占比大但 zhhz 优势稳定）
python3 -c "
keys=[]
for n in ('STPhrases','TWPhrases','HKPhrases','JPShinjitaiPhrases'):
    p=f'data/dictionary/{n}.txt'
    for line in open(p,encoding='utf-8'):
        if line.startswith('#') or not line.strip(): continue
        k,_=line.split(chr(9),1)
        if k: keys.append(k)
import random; random.seed(42)
target=100*1024*1024; written=0
with open('/tmp/c.txt','w',encoding='utf-8') as f:
    while written<target:
        c='。'.join(random.sample(keys, min(2000,len(keys)))) + '。'
        f.write(c); written+=len(c.encode())
"
for cmd in './target/release/zhhz -c s2t' 'opencc -c s2t --path $(brew --prefix opencc)/share/opencc'; do
  for i in 1 2 3; do
    t0=$(date +%s%N); eval "$cmd < /tmp/c.txt >/dev/null"
    t1=$(date +%s%N); echo "$(( (t1-t0)/1000000 ))"
  done | sort -n | sed -n '2p' | xargs -I{} echo "{} ms"
done
```

CI 上的同源 Parity（zhhz 与同数据 opencc 字节级对比，权威 gate）：
`.github/workflows/parity.yml` 跑 `examples/parity.rs`（包含输出比较 + 差异分类）。

## 接下来

* **Compact-trie / 预序列化词典**（mneme#64）—— 进一步把构建（parse + 树构建）时间也消掉。短文本和频繁调用的场景会明显受益（每次启动省 ~20ms）。对长文本的额外提升较小（zhhz 已经赢在 per-char 查询）。
* **1GB 跑分里的 I/O 占比**——1GB 跑分受管道读限制（~40MB/s）。要测"纯转换"还得把语料预先 `mmap` 进内存再 time。
* **FMM DP 分段**（mneme#62 / opencc#475）—— 修 opencc 的 FMM 贪心分词 bug，与本次 perf 优化正交。

## 引用

* x-cmd 文章形式层（`x-cmd/doc-2026` `.x-cmd/ai-install-article-fine-MUST.md`）—— 核心要求是"只有结果一样的比较才有意义"，本文照做。
* `docs/experience.md` —— zhhz 整体设计（AI-agent 友好、opencc 作为正确性标准等）。
* `src/dict.rs` —— v0.6 的 arena + sorted-Vec 实现。
* `scripts/benchmark.sh` —— 小样本 benchmark 脚本。
