---
layout: default
title: Deno
permalink: /deno/
---

# Deno integration

zhhz works with [Deno](https://deno.com) in two ways:

1. **Via npm: specifier** — Deno can import npm packages directly,
   no special setup. This works today with the published
   [`zhhz@0.7.9`](https://www.npmjs.com/package/zhhz).
2. **Via JSR** — Deno's own registry. zhhz is published there as
   `@ljh-sh/zhhz` (or whatever scope matches your account).

Both expose the same API surface. The npm path is the simplest
starting point; JSR is the Deno-native experience for projects
that prefer not to depend on npm.

## Quick start: `npm:` specifier

```ts
// hello-zhhz-deno.ts
import { convert, detect, Converter } from "npm:zhhz@0.7.9";

const text = "汉字计算机软件";
console.log("s2t  :", convert(text, "s2t"));            // 漢字計算機軟件
console.log("s2twp:", convert("信息", "s2twp"));         // 資訊

const c = new Converter("s2twp");
console.log("reuse:", c.convert("鼠标"));                // 滑鼠

const d = detect("他去了西維珍尼亞州");
console.log("detect:", d?.region, d?.confidence);        // cn-hk 50
```

Run with:

```sh
deno run --allow-read --allow-env --allow-net hello-zhhz-deno.ts
```

`npm:zhhz@0.7.9` pulls the same WebAssembly blob that npm
consumers get. No build step, no native dependency, no separate
data file — the OpenCC dictionaries are baked into the `.wasm`.

Add a `deno.json` to pin the version:

```json
{
  "nodeModulesDir": "auto",
  "imports": {
    "zhhz": "npm:zhhz@0.7.9"
  }
}
```

Then `import { ... } from "zhhz"` resolves to the pinned version.

## JSR (`@ljh-sh/zhhz`)

For projects that prefer Deno's native registry:

```ts
import { convert, Converter } from "jsr:@ljh-sh/zhhz";
```

JSR mirrors the npm release. Install the CLI tool (`deno install
-A jsr --global`), then `deno publish` from the `zhhz` repo (or
via `.github/workflows/deno.yml`) to push a new version.

See the [JSR docs](https://jsr.io/docs/publishing-packages) for
the publish flow. zhhz uses GitHub Trusted Publishing — no API
token to manage.

## Performance

Deno's V8 is essentially the same engine as Node.js, so the
wasm-bindgen output performs comparably. On the 5.18 MiB
mixed CJK / Latin corpus, M2:

| config | Deno 2.7 | Node.js 22 | Deno/Node |
|---|---:|---:|---:|
| `s2t`   | 58 MB/s | 63 MB/s | 92% |
| `t2s`   | **108 MB/s** | 104 MB/s | **103%** |
| `s2twp` | 39 MB/s | 41 MB/s | 94% |

Run the benchmark yourself:

```sh
deno run --allow-read --allow-env --allow-net \
  https://raw.githubusercontent.com/ljh-sh/zhhz/main/tests/bench-deno.ts
```

Or clone and `deno run --allow-read --allow-env --allow-net tests/bench-deno.ts`.

## Why both?

- **npm** is the largest registry; most JS/TS consumers already
  have it. Works in Node, Deno, Bun, browsers (via bundlers).
- **JSR** is Deno's first-party registry. Better discoverability
  inside the Deno ecosystem; tighter type integration; explicit
  ESM-only.

Most users will use the npm path. Deno-ecosystem-only projects
may prefer JSR. Both are first-class.

## See also

- [Benchmarks]({{ '/benchmarks' | relative_url }}) — full perf table
- [Node.js / npm]({{ '/npm' | relative_url }}) — same API via Node
- [Python integration]({{ '/python' | relative_url }}) — same API via Python
- [Install]({{ '/install' | relative_url }}) — all four channels