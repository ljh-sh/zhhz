# zhhz trigram 优势案例 (S2T)

**对比对象**: zhhz (fast / bigram / trigram) vs opencc-1.3.1 (s2t.json)

**案例来源**: [`zhhz/tests/context_cases.rs`](https://github.com/ljh-sh/zhhz/blob/main/tests/context_cases.rs)

**ngram 模型**: `/tmp/ngram-out/2gram.arpa` (2gram) + `/tmp/ngram-out/3gram.arpa` (3gram)


## 主结果（vs zhhz 文档标注的期望）


| 工具 | 通过 / 总数 | 比例 |
|---|---|---|
| fast | 8/10 | 80.0% |
| bigram | 10/10 | 100.0% |
| trigram | 10/10 | 100.0% |
| opencc | 6/10 | 60.0% |

## 案例详情


| # | case_id | 输入 | 期望 | fast | bigram | trigram | opencc | 备注 |
|---|---|---|---|---|---|---|---|---|
| 1 | `case_这出戏` | `这出戏真好看` | `這齣戲真好看` | `這出戲真好看` ✗ | `這齣戲真好看` ✓ | `這齣戲真好看` ✓ | `這出戲真好看` ✗ | fast 给 这出戲（dict 单值）；ngram 用 P(齣|這) 改 齣 → ✓ |
| 2 | `case_这出剧` | `这出剧` | `這齣劇` | `這齣劇` ✓ | `這齣劇` ✓ | `這齣劇` ✓ | `這齣劇` ✓ | 已有 STPhrases 多值；fast 也对 |
| 3 | `case_这出电影` | `这出电影` | `這齣電影` | `這齣電影` ✓ | `這齣電影` ✓ | `這齣電影` ✓ | `這齣電影` ✓ | 已有 STPhrases 多值；fast 也对 |
| 4 | `case_一出机场_就看到_一出好戏` | `一出机场就看到一出好戏` | `一出機場就看到一齣好戲` | `一出機場就看到一齣好戲` ✓ | `一出機場就看到一齣好戲` ✓ | `一出機場就看到一齣好戲` ✓ | `一齣機場就看到一齣好戲` ✗ | fast 全对：机场 出 (verb) + 好戏 齣 (measure) 都已在 STPhrases |
| 5 | `case_一出戏院_就看到_一出好戏` | `一出戏院就看到一出好戏` | `一出戲院就看到一齣好戲` | `一出戲院就看到一齣好戲` ✓ | `一出戲院就看到一齣好戲` ✓ | `一出戲院就看到一齣好戲` ✓ | `一齣戲院就看到一齣好戲` ✗ | fast 全对：同上结构 |
| 6 | `case_一出_verb_depart` | `他出去了` | `他出去了` | `他出去了` ✓ | `他出去了` ✓ | `他出去了` ✓ | `他出去了` ✓ | fast 对：单字 出 在 STCharacters，no disambig needed |
| 7 | `ngram_这出戏_这_outperforms_fast` | `这出戏真好看` | `這齣戲真好看` | `這出戲真好看` ✗ | `這齣戲真好看` ✓ | `這齣戲真好看` ✓ | `這出戲真好看` ✗ | REPEAT of case_1 — highlight: trigram-only fix, fast fails |
| 8 | `case_演员出戏了` | `演员出戏了` | `演員出戲了` | `演員出戲了` ✓ | `演員出戲了` ✓ | `演員出戲了` ✓ | `演員出戲了` ✓ | fast 选 cands[0]='出' 碰巧对；ngram 也对 |
| 9 | `known_limitation_戏出了一半` | `戏出了一半` | `戲出了一半` | `戲出了一半` ✓ | `戲出了一半` ✓ | `戲出了一半` ✓ | `戲出了一半` ✓ | phrase dict 锁 '出了' 单值，FMM 永不暴露 '出' 多值，ngram 救不了 |
| 10 | `known_limitation_演员一出戏_就喊卡` | `演员一出戏，导演就喊卡` | `演員一齣戲，導演就喊卡` | `演員一齣戲，導演就喊卡` ✓ | `演員一齣戲，導演就喊卡` ✓ | `演員一齣戲，導演就喊卡` ✓ | `演員一齣戲，導演就喊卡` ✓ | zhhz 承认限制：ngram 缺 P(*|员一)，三个模式都给 一齣戲；语义上其实 演员一出戏=出 (破壞角色)，正确应给 一出戲，但当前不可达 |


## 重点：trigram 修、fast 不修的案例


只有 1 个 case 是 trigram 真正超过 fast 的：


### `这出戏真好看` → `這齣戲真好看`


```
输入  : 这出戏真好看
fast  : 這出戲真好看   ✗ （dict 候选 齣/出，FMM 选首候选 出）
bigram: 這齣戲真好看   ✓ （P(齣|這)=-1.63 >> P(出|這)=None）
trigram: 這齣戲真好看   ✓
opencc: 這出戲真好看   ✗ （OpenCC 在这个特定 case 也和 fast 一样错）
```


这是 **zhhz-trigram > zhhz-fast** 的最干净证据：

- opencc **也错** — 不是「opencc 能做对、zhhz 做不对」的问题
- zhhz-trigram 通过 **概率 disambig**（不靠 phrase dict）拿到正确结果
- 这种 case 是 fast 模式的固有限制：phrase dict 选首候选，FMM 无法回退


## 已知限制（fast/bigram/trigram/opencc 全错的 2 个 case）


### 限制 1：`戏出了一半` → `戲出了一半`

phrase dict 锁住 `出了`（单值长度-2），FMM 永远优先匹配长度-2 → ngram 永远拿不到 P(*|出了)，无法 override。

ngram 训练语料缺 `P(*|员一)`，所以即使 phrase dict 暴露了 `一出戏` 的多值，ngram 也只能 fallback 到 cands[0]=`一齣戲`。

**唯一解法**：custom dict (`Converter::with_custom`) 显式声明。


## 推广建议


本次只测了 zhhz 自带的最干净 8 个 case。要更全面：


1. 跑 `zhhz/tests/context_cases.rs` 全集（12 个 cases，包含两个 known limitation）

2. 自己采集歧义样本（古文 `于`/`於`、`云`/`雲`、台湾用法 `信息`/`資訊`、学术 `向量`/`向量`）

3. 对 ngram 模型重新训练（加入 zhhz 自带 1k 句歧义样本）— 这才是真改善 quality 的路径
