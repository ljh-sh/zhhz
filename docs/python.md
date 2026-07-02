---
layout: default
title: Python integration
permalink: /python/
---

# Python integration

zhhz is a Rust binary with a WebAssembly npm package and a Rust library. A native PyO3 binding (`pip install zhhz`) is on the [roadmap](#roadmap). Until then, the simplest Python integration is to call the `zhhz` CLI via `subprocess`.

## Today: subprocess (stdlib-only)

```python
import subprocess

def convert(text: str, config: str = "s2t") -> str:
    """Convert text using the zhhz CLI. Requires zhhz on PATH."""
    out = subprocess.run(
        ["zhhz", "-c", config],
        input=text,
        capture_output=True,
        text=True,
        check=True,
    )
    return out.stdout

# Convert Simplified -> Traditional
print(convert("汉字"))                # 漢字
print(convert("信息", "s2twp"))       # 資訊 (Taiwan phrases)
print(convert("鼠标", "s2twp"))       # 滑鼠
```

Install `zhhz` first (one of):

```sh
cargo install zhhz
# or
npm install -g zhhz-cli   # when the global CLI wrapper ships
# or download a release binary
```

## Script variant detection

```python
import subprocess

def detect(text: str) -> tuple[str, int] | None:
    """Return (region, confidence) or None for non-CJK text."""
    out = subprocess.run(
        ["zhhz", "detect"],
        input=text,
        capture_output=True,
        text=True,
        check=True,
    )
    line = out.stdout.strip()
    if not line:
        return None
    region, confidence, _path = line.split("\t")
    return region, int(confidence)

print(detect("他去了西維珍尼亞州"))    # ('cn-hk', 70)
print(detect("汉字计算机软件"))        # ('cn-s', 90)
```

## Batch helper

For larger workloads, batch the subprocess calls — startup cost dominates per-call timing:

```python
import subprocess

def convert_batch(texts: list[str], config: str = "s2t") -> list[str]:
    """Convert many texts in one subprocess."""
    payload = "\n".join(texts)
    out = subprocess.run(
        ["zhhz", "-c", config],
        input=payload,
        capture_output=True,
        text=True,
        check=True,
    )
    return out.stdout.split("\n")
```

For higher throughput, call the CLI with one process per chunk on a `concurrent.futures.ProcessPoolExecutor`. The conversion itself is CPU-bound and scales linearly with cores.

## Async / event loops

The `zhhz` CLI is sync; wrap the subprocess call in `asyncio.to_thread` or `loop.run_in_executor` if you're calling it from an async context:

```python
import asyncio
import subprocess

async def convert_async(text: str, config: str = "s2t") -> str:
    return await asyncio.to_thread(
        lambda: subprocess.run(
            ["zhhz", "-c", config],
            input=text,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
    )
```

## Why subprocess and not ctypes/cffi?

The OpenCC dictionaries are ~1.3 MiB and the engine is a static Rust binary. Loading that through ctypes requires either shipping a C-compatible `.so`/`.dylib`/`.dll` or reimplementing the trie walk in Python (which would lose the perf advantage). Subprocess gives you:

- **No compilation**: works on any platform with a `zhhz` binary.
- **No extra dependencies**: stdlib only.
- **Predictable memory**: the binary has a fixed RSS footprint.
- **Easy to swap**: when the PyO3 binding ships, only the `convert()` function changes — call sites stay the same.

The cost is one fork+exec per call (~5–20 ms overhead) plus stdout buffering. For single-document conversions this is fine; for tight inner loops over many small strings, batch them.

## Roadmap

A native PyO3 + `maturin` binding is the next step:

| Step | Status |
|---|---|
| Design API (mirror `Converter` class from Rust library + `npm` package) | Planned |
| `pyo3` crate wiring in `Cargo.toml` (gated by `python` feature) | Planned |
| `maturin` build + manylinux / musllinux wheels | Planned |
| `pip install zhhz` → PyPI publish | Planned |
| Migrate existing `convert()` helpers to the native binding | Documented |

Until then, the subprocess helper above is the supported path. It will keep working unchanged when the native binding lands — drop-in replacement.