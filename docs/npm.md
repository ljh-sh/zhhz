# `zhhz` for Node.js (zhhz#40)

The npm package `zhhz` ships the same conversion engine as the `zhhz` CLI,
compiled to WebAssembly. OpenCC dictionaries are baked into the `.wasm`
blob, so there is no data directory to ship alongside and no network
fetch at startup — `npm install zhhz` is the entire setup.

The API surface is **strictly richer than [`opencc-js`](https://github.com/nk2028/opencc-js)**:

| Capability                       | `zhhz` | `opencc-js` |
|----------------------------------|:------:|:-----------:|
| 16 OpenCC conversion configs     |   ✅   |     ✅      |
| Custom words (array form)        |   ✅   |     ✅      |
| Custom words (string form)       |   ✅   |     ✅      |
| Reusable converter instance      |   ✅   |     ✅      |
| Per-instance `convertWithCustom` |   ✅   |     ❌      |
| Script-variant detection         |   ✅   |     ❌      |
| Config / locale introspection    |   ✅   |   partial   |
| Semantic region flags (`from`/`to`) | ✅   |     ❌      |
| Zero native dependencies         |   ✅   |     ✅      |
| Same bytes in / same bytes out as the CLI | ✅ | partial  |

## Install

```sh
npm install zhhz
```

Requires Node.js ≥ 18 (the package targets the ESM bundler artifact).

## Quick start (ESM)

```js
import { convert, detect, Converter, listConfigs } from "zhhz";

// One-shot
console.log(convert("汉字", "s2t")); // 漢字

// Script-variant detection (unique to zhhz)
console.log(detect("他去了西維珍尼亞州"));
// { region: "cn-hk", confidence: 70 }

// Reusable instance — no need to rebuild per call
const c = new Converter("s2twp");
console.log(c.convert("信息"));     // 資訊
console.log(c.convert("鼠标"));     // 滑鼠  (Taiwan phrase projection)

// One-line: which configs are available?
console.log(listConfigs());
// ["s2t", "t2s", "s2tw", "tw2s", "s2hk", "hk2s", "s2twp", "tw2sp",
//  "s2hkp", "hk2sp", "t2tw", "tw2t", "t2hk", "hk2t", "t2jp", "jp2t"]
```

## Quick start (CommonJS)

```js
(async () => {
  const { convert, Converter } = await import("zhhz");
  // or: const { convert, Converter } = require("zhhz");
  console.log(convert("汉字", "s2t")); // 漢字
  const c = new Converter("s2twp");
  console.log(c.convert("信息"));     // 資訊
})();
```

## Conversion configs

All 16 OpenCC configs are supported. `twp` / `hkp` variants project
Taiwan / Hong Kong idioms (`鼠标 → 滑鼠`, `信息 → 資訊`).

| Config   | Direction                                   |
|----------|---------------------------------------------|
| `s2t`    | Simplified → Traditional (OpenCC standard)  |
| `t2s`    | Traditional (OpenCC standard) → Simplified |
| `s2tw`   | Simplified → Traditional (Taiwan)           |
| `tw2s`   | Traditional (Taiwan) → Simplified           |
| `s2hk`   | Simplified → Traditional (Hong Kong)        |
| `hk2s`   | Traditional (Hong Kong) → Simplified        |
| `s2twp`  | Simplified → Traditional (Taiwan, **with phrases**) |
| `tw2sp`  | Traditional (Taiwan, with phrases) → Simplified |
| `s2hkp`  | Simplified → Traditional (Hong Kong, **with phrases**) |
| `hk2sp`  | Traditional (Hong Kong, with phrases) → Simplified |
| `t2tw`   | Traditional (OpenCC standard) → Traditional (Taiwan) |
| `tw2t`   | Traditional (Taiwan) → Traditional (OpenCC standard) |
| `t2hk`   | Traditional (OpenCC standard) → Traditional (Hong Kong) |
| `hk2t`   | Traditional (Hong Kong) → Traditional (OpenCC standard) |
| `t2jp`   | Traditional → Japanese Shinjitai            |
| `jp2t`   | Japanese Kyūjitai → Traditional             |

## Custom words

Two equivalent forms for custom-word overrides (highest priority):

```js
import { convert_with_custom, Converter } from "zhhz";

// Array form (preferred in JS)
convert_with_custom("买软件", "s2t", [["软件", "軟體"]]);
// "買軟體"

// String form — same as opencc-js's DictLike. Entries separated by "|",
// each entry is "key value" (first space splits key from value).
convert_with_custom("买软件", "s2t", "软件 軟體|苹果 蘋果");
// "買軟體"
```

On the `Converter` instance, you can inject custom words per call or bake
them into a new instance:

```js
const c = new Converter("s2t");

// Per call — original instance unchanged
console.log(c.convertWithCustom("买软件", [["软件", "軟體"]])); // 買軟體
console.log(c.convert("买软件"));                              // 買軟體 (no custom)

// Baked in — new instance with custom always applied
const cCustom = c.withCustom([["软件", "軟體"]]);
console.log(cCustom.convert("买软件")); // 買軟體
```

## Script-variant detection

```js
import { detect } from "zhhz";

detect("汉字计算机软件");
// { region: "cn-s", confidence: 90 }

detect("漢字計算機軟體");
// { region: "cn-t", confidence: 80 }

detect("他去了西維珍尼亞州");
// { region: "cn-hk", confidence: 70 }

detect("こんにちは世界");
// { region: "jp-n", confidence: 80 }

detect("hello world");
// null  (no CJK characters)
```

`region` is one of `cn-s` / `cn-t` / `cn-tw` / `cn-hk` / `jp-n` / `jp-t`;
`confidence` is 0–100.

## Semantic region flags

`zhhz`'s CLI accepts `--from cn-s --to cn-tw` instead of memorising
config names. The same idea is exposed to JS:

```js
import { configForRegionPair, Converter } from "zhhz";

configForRegionPair("cn-s", "cn-tw"); // "s2twp"  (phrase-aware variant)
configForRegionPair("cn-s", "cn-hk"); // "s2hkp"
configForRegionPair("cn-t", "cn-s");  // "t2s"

// Or directly build a Converter from a region pair:
const c = Converter.forRegion("cn-s", "cn-tw");
console.log(c.config);  // "s2twp"
console.log(c.convert("鼠标")); // 滑鼠
```

## Introspection

```js
import { listConfigs, listLocales } from "zhhz";

listConfigs();
// ["s2t", "t2s", "s2tw", "tw2s", "s2hk", "hk2s", "s2twp", "tw2sp",
//  "s2hkp", "hk2sp", "t2tw", "tw2t", "t2hk", "hk2t", "t2jp", "jp2t"]

listLocales();
// ["cn-s", "cn-t", "cn-tw", "cn-hk", "jp-t", "jp-n"]
```

## Notes

- **Bundle size**: ~1.5 MiB unzipped (the `.wasm` blob holds the OpenCC
  dictionary data; gzip-compressed transfer is ~500 KiB). Same size as
  opencc-js.
- **No network access at runtime.** The `.wasm` is self-contained.
- **Sync only.** Conversion is CPU-bound and fast enough that an async
  wrapper would add overhead without benefit. If you have a multi-MB
  text, batch it and call `convert` per chunk on a worker thread.
- **Parity with the CLI.** Conversion output is byte-identical to
  `zhhz -c <config>` running on the same input (538/538 parity cases
  verified vs the `opencc` reference CLI; see `examples/parity.rs`).

## License

Apache 2.0. Dictionary data vendored from
[BYVoid/OpenCC](https://github.com/BYVoid/OpenCC) at the pinned commit
in `data/UPSTREAM`.