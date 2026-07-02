# `zhhz` Node.js example (zhhz#40)

Runnable smoke for the `zhhz` npm package. Exercises every exported API
and exits 0 only when every assertion holds.

## Run

> **Note**: this example depends on `zhhz@^0.7.8` from the npm registry.
> It will only succeed after the first npm publish of zhhz v0.7.8.
> Before that, `npm install` will fail with a 404.

```sh
npm install
npm start
```

Expected output ends with `24 passed, 0 failed` and exit code 0.

## What it covers

- One-shot `convert(text, config)` across multiple configs
- `convert_with_custom(text, config, entries)` — array form and string form
- `detect(text)` — script-variant detection (zhhz-only vs opencc-js)
- `listConfigs()` / `listLocales()` — introspection
- `configForRegionPair(from, to)` — semantic region flags
- `new Converter(config)` + `.convert` / `.convertWithCustom` / `.config`
- `Converter.forRegion(from, to)` — build from semantic region flags
- `Converter.withCustom(entries)` — bake custom words into a new instance

## File map

- `index.mjs` — the smoke script
- `package.json` — depends on `zhhz@^0.7.8` from npm, `type: module`,
  requires Node ≥ 18