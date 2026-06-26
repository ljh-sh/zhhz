# Canonical 齣/出 context test cases

> Two sentences the user supplied as canonical test cases for the
> 齣/出 disambiguation problem. They live as `tests/context_cases.rs`
> so any future N-gram / context-disambiguation work has a regression
> gate that proves the obvious cases still resolve correctly.

## Why these two

Both sentences use the simplified character 出 twice with **different
correct traditional renderings** depending on what follows:

| input | 1st 出 context | 2nd 出 context | correct traditional |
|---|---|---|---|
| `一出机场就看到一出好戏` | 出 + 机场 = **出**機場 (depart) | 出 + 好戏 = **齣**好戲 (show measure) | `一出機場就看到一齣好戲` |
| `一出戏院就看到一出好戏` | 出 + 戏院 = **出**戲院 (depart) | 出 + 好戏 = **齣**好戲 (show measure) | `一出戲院就看到一齣好戲` |

A rule-based converter cannot tell the two apart without context — the
simplified text is byte-identical, only the following noun disambiguates.
This is the irreducible ambiguity floor of any rule-based simplified→traditional
converter (zhhz, opencc, anything).

## Current behavior (data-driven, not principled)

| input | zhhz | opencc 1.2.0 | expected |
|---|---|---|---|
| `一出机场就看到一出好戏` | `一出機場就看到一齣好戲` ✓ | `一齣機場就看到一齣好戲` ✗ (`齣機` 不通) | `一出機場就看到一齣好戲` |
| `一出戏院就看到一出好戏` | `一出戲院就看到一齣好戲` ✓ | `一齣戲院就看到一齣好戲` ✗ (`齣院` 不通) | `一出戲院就看到一齣好戲` |

zhhz gets both right by **data luck**, not by a principled rule:

* `出 → 齣` in `STPhrases` phrase-level entries (e.g. `齣好戏` is a
  key, so the FMM segmenter matches `出` → `齣`).
* `出 → 出` in `STCharacters` (first value), picked by the segmenter
  when no phrase overrides it (e.g. before `机场` / `戏院`).

opencc's `齣` rule is the symmetric data luck the other way: it gets
`齣戏` / `齣好戏` right (data consistent) and `齣机场` / `齣戏院`
wrong (data inconsistent — `齣` only modifies `戏`, not `院`).

Neither tool is principled. Both depend on which multi-value entry the
data happens to list first. **The 0 % "shared wrong" floor (e.g. `齣机场`
where opencc is wrong) is the same data problem on both sides.**

These two sentences are the regression gate for any future principled
disambiguation (N-gram / context rules / opencc-style rule table). The
current zhhz output is asserted in `tests/context_cases.rs`; any change
that breaks it must be justified (e.g. an N-gram that picks `齣` for
`一出機場` would be a regression).

## Reproduce

```sh
./target/release/zhhz -c s2t <<< "一出机场就看到一出好戏"
opencc -c s2t --path "$(brew --prefix opencc)/share/opencc" <<< "一出机场就看到一出好戏"
./target/release/zhhz -c s2t <<< "一出戏院就看到一出好戏"
opencc -c s2t --path "$(brew --prefix opencc)/share/opencc" <<< "一出戏院就看到一出好戏"
```
