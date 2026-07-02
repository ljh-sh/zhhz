---
layout: default
title: Python integration
permalink: /python/
---

# Python integration

zhhz ships a **native CPython extension** built with [PyO3](https://pyo3.rs) +
[maturin](https://www.maturin.rs). `pip install zhhz` gives you the
same conversion engine the CLI / npm package exposes, with the OpenCC
dictionaries embedded.

```python
>>> import zhhz
>>> zhhz.convert("汉字", "s2t")
'漢字'
>>> zhhz.convert("信息", "s2twp")     # Taiwan phrases
'資訊'
>>> zhhz.detect("他去了西維珍尼亞州")
<zhhz.Detection region='cn-hk' confidence=50>
>>> zhhz.configs()
['s2t', 't2s', 's2tw', 'tw2s', 's2hk', 'hk2s', 's2twp', 'tw2sp',
 's2hkp', 'hk2sp', 't2tw', 'tw2t', 't2hk', 'hk2t', 't2jp', 'jp2t']
>>> zhhz.locales()
['cn-s', 'cn-t', 'cn-tw', 'cn-hk', 'jp-t', 'jp-n']
```

## Install

```sh
pip install zhhz
```

Wheels are published for CPython 3.9–3.13 on
manylinux (x86_64 / aarch64), musllinux (x86_64 / aarch64),
macOS (x86_64 / arm64), and Windows (x86_64).

The first call lazy-loads the conversion tables (~1.3 MiB of data,
mmap'd from the wheel). After that, `convert()` is CPU-bound
and async-loop-friendly.

## API surface

```python
import zhhz

# One-shot conversion (allocates a Converter per call — convenient
# for small inputs, ~1.3x slower than a reused instance).
zhhz.convert(text, "s2t")
zhhz.convert(text, "s2twp")
zhhz.convert_region(text, "cn-s", "cn-tw")    # resolves to s2twp

# Reusable factory instance — build once, convert many times.
c = zhhz.Converter("s2twp")
c.convert(text)
c.config                       # "s2twp"
c.convert_with_custom(text, [
    ["软件", "軟體"],
    ["独家", "獨家"],
])

# Custom words accept any of three forms (mirrors npm + opencc-js):
c.convert_with_custom(text, [["软件", "軟體"]])            # list of pairs
c.convert_with_custom(text, {"软件": "軟體", "独家": "獨家"})  # dict
c.convert_with_custom(text, "软件 軟體|独家 獨家")              # DictLike string

# Bake custom words into a new instance (every subsequent .convert()
# applies them — equivalent to the CLI's --dict file).
c_with = c.with_custom([["软件", "軟體"]])

# Semantic region flags (mirrors the CLI's --from / --to):
c2 = zhhz.Converter.for_region("cn-s", "cn-tw")
c2.config                       # "s2twp"
c2.convert("鼠标")              # "滑鼠"

# Script-variant detection:
d = zhhz.detect("汉字计算机软件")
d.region                        # "cn-s"
d.confidence                    # 57

# Introspection:
zhhz.configs()                   # all 16 OpenCC config names
zhhz.locales()                   # all 6 region codes
```

## Building from source

The wheel is built locally with [maturin](https://www.maturin.rs):

```sh
# Once: set up a venv with Python 3.9+ + maturin.
python -m venv .venv
source .venv/bin/activate
pip install "maturin>=1.0,<2.0"

# Then in the zhhz repo root:
maturin develop --release --features python
# or:
maturin build --release --features python
# → target/wheels/zhhz-0.7.8-cp310-cp310-macosx_11_0_arm64.whl
pip install target/wheels/zhhz-*.whl
```

The `python` Cargo feature pulls in `pyo3 = "0.21"` (extension-module).
Without it, the native CLI / library builds unaffected.

## Performance

On a 5.18 MiB mixed CJK + Latin corpus (M2, CPython 3.10, PyO3 0.21):

| config | Python (native) | Node.js (WASM) | native CLI |
|---|---:|---:|---:|
| s2t   | (same engine) | 63 MB/s | 88 MB/s |
| s2twp | (same engine) | 41 MB/s | 88 MB/s |
| t2s   | (same engine) | 104 MB/s | 88 MB/s |

All three channels (Python, Node.js, CLI) share the same Rust core —
the per-binding number is overhead. The Python binding has near-zero
per-call overhead because the OpenCC dictionaries live in the same
process via the loaded `.so` / `.pyd`.

## Threading

`Converter.convert` takes `&self` in Rust (via PyO3's `&self` extractor).
Conversion is `Sync` and thread-safe — build one per worker thread
for parallel batch jobs:

```python
from concurrent.futures import ThreadPoolExecutor
import zhhz

c = zhhz.Converter("s2twp")
texts = [chunk_a, chunk_b, chunk_c]

with ThreadPoolExecutor(max_workers=4) as ex:
    out = list(ex.map(c.convert, texts))
```

Each thread keeps its own `Converter` for maximum throughput.

## Async / event loops

The binding is sync. Wrap calls in `asyncio.to_thread` (or
`loop.run_in_executor`) when calling from an async context:

```python
import asyncio
import zhhz

async def convert_async(text, config="s2t"):
    return await asyncio.to_thread(zhhz.convert, text, config)
```

## Roadmap

- **PyPI publish workflow**: `.github/workflows/python.yml` is in place
  (builds wheels for manylinux / musllinux / macOS / Windows across
  Python 3.9–3.13, publishes to PyPI on tag push). Needs `PYPI_API_TOKEN`
  as an org secret before the first release goes out.
- **Async-native binding**: a `tokio`-style event-loop friendly API is
  possible (background thread pool + `await`) but adds complexity
  without obvious throughput win — the sync path is already at native
  speed.
- **Streaming / chunked**: chunk-by-chunk for very large texts (>50 MB).
  Add-on, not a rewrite.

## Related

- [Install]({{ '/install' | relative_url }})
- [Rust library]({{ '/library' | relative_url }})
- [Node.js / npm]({{ '/npm' | relative_url }})
- [CLI reference]({{ '/cli' | relative_url }})