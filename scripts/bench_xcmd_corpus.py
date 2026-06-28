#!/usr/bin/env python3
"""zhhz vs opencc on x-cmd corpus.

Each tool runs RUNS times (after WARMUP); we report min + median
wall time and throughput.

Also computes inter-tool agreement:
  - zhhz-fast vs opencc
  - zhhz-bigram vs opencc
  - zhhz-trigram vs opencc
"""
import subprocess
import sys
import time
from pathlib import Path
from statistics import median

ZHHZ_DIR = Path("/Users/l/.x-repo/zhhz-wt-v074")
CORPUS = Path("/tmp/xcmd-corpus/_xcmd_s2t_input.txt")
XCMD_BENCH = ZHHZ_DIR / "target/release/examples/xcmd_bench"
WARMUP = 2
RUNS = 7

CN = sum(1 for c in CORPUS.read_text(encoding="utf-8") if '一' <= c <= '鿿')


def bench_zhhz(mode: str) -> tuple[float, list[float]]:
    args = [str(XCMD_BENCH), mode, str(CORPUS), f"/tmp/xcmd-corpus/_zhhz_{mode}.txt"]
    # Warmup
    for _ in range(WARMUP):
        subprocess.run(args, cwd=str(ZHHZ_DIR), capture_output=True, check=True)
    times = []
    for _ in range(RUNS):
        t0 = time.perf_counter()
        subprocess.run(args, cwd=str(ZHHZ_DIR), capture_output=True, check=True)
        times.append(time.perf_counter() - t0)
    return min(times), times


def bench_opencc() -> tuple[float, list[float]]:
    # opencc CLI: each invocation does config-load + convert + flush
    for _ in range(WARMUP):
        subprocess.run(
            ["opencc", "-c", "s2t.json", "-i", str(CORPUS),
             "-o", "/tmp/xcmd-corpus/_opencc_s2t.txt"],
            capture_output=True, check=True,
        )
    times = []
    for _ in range(RUNS):
        t0 = time.perf_counter()
        subprocess.run(
            ["opencc", "-c", "s2t.json", "-i", str(CORPUS),
             "-o", "/tmp/xcmd-corpus/_opencc_s2t.txt"],
            capture_output=True, check=True,
        )
        times.append(time.perf_counter() - t0)
    return min(times), times


def diff_pct(a: Path, b: Path) -> tuple[int, int, float]:
    """Return (matching_bytes, total_bytes, pct_match)."""
    ab = a.read_bytes()
    bb = b.read_bytes()
    n = min(len(ab), len(bb))
    same = sum(1 for i in range(n) if ab[i] == bb[i])
    # Penalty for length mismatch
    length_diff = abs(len(ab) - len(bb))
    total = max(len(ab), len(bb))
    return same, total, same / total * 100.0


def char_diff(a: Path, b: Path) -> tuple[int, int, float]:
    """Return (matching_cjk_chars, total_cjk_chars, pct_match) by char."""
    a_text = a.read_text(encoding="utf-8")
    b_text = b.read_text(encoding="utf-8")
    ac = [c for c in a_text if '一' <= c <= '鿿']
    bc = [c for c in b_text if '一' <= c <= '鿿']
    n = min(len(ac), len(bc))
    same = sum(1 for i in range(n) if ac[i] == bc[i])
    total = max(len(ac), len(bc))
    return same, total, same / total * 100.0 if total else 0.0


def main():
    print(f"corpus: {CORPUS.stat().st_size} bytes, {CN} CJK chars")
    print(f"runs: {RUNS} (warmup {WARMUP}, report min + median)\n")

    results = {}

    # opencc
    t_min, t_all = bench_opencc()
    results["opencc-1.3.1-s2t"] = (t_min, t_all)
    opencc_out = Path("/tmp/xcmd-corpus/_opencc_s2t.txt")

    # zhhz
    for mode in ["fast", "bigram", "trigram"]:
        t_min, t_all = bench_zhhz(mode)
        results[f"zhhz-{mode}"] = (t_min, t_all)

    print(f"{'tool':<22} {'min_ms':>8} {'med_ms':>8} {'KB/s':>9} {'speedup':>9}")
    print("-" * 60)
    opencc_min = results["opencc-1.3.1-s2t"][0]
    corpus_kb = CORPUS.stat().st_size / 1024.0
    for name, (t_min, t_all) in results.items():
        t_med = median(t_all)
        kb_s = corpus_kb / t_min
        speedup = opencc_min / t_min
        print(f"{name:<22} {t_min*1000:>8.2f} {t_med*1000:>8.2f} "
              f"{kb_s:>9.0f} {speedup:>9.2f}x")

    print()
    print("=" * 60)
    print("Agreement with opencc-1.3.1-s2t (CJK char match %)")
    print("=" * 60)
    for mode in ["fast", "bigram", "trigram"]:
        out = Path(f"/tmp/xcmd-corpus/_zhhz_{mode}.txt")
        same, total, pct = char_diff(out, opencc_out)
        marker = "✓" if pct >= 99.0 else ("~" if pct >= 95.0 else "✗")
        print(f"  zhhz-{mode:<8}  {same}/{total}  = {pct:.2f}%  {marker}")
        # byte-level
        same_b, total_b, pct_b = diff_pct(out, opencc_out)
        print(f"  zhhz-{mode:<8}  (bytes) {pct_b:.2f}%")

    print()
    print("=" * 60)
    print("Sample disagreements (first 20 lines that differ)")
    print("=" * 60)
    # Show first 20 lines where zhhz-trigram differs from opencc
    zt = Path("/tmp/xcmd-corpus/_zhhz_trigram.txt").read_text(encoding="utf-8")
    ot = opencc_out.read_text(encoding="utf-8")
    zt_lines = zt.splitlines()
    ot_lines = ot.splitlines()
    n = min(len(zt_lines), len(ot_lines))
    diffs = 0
    for i in range(n):
        if zt_lines[i] != ot_lines[i] and diffs < 20:
            print(f"  L{i}: opencc | {ot_lines[i][:80]!r}")
            print(f"        zhhz   | {zt_lines[i][:80]!r}")
            diffs += 1
    print(f"  total diff lines: {sum(1 for i in range(n) if zt_lines[i] != ot_lines[i])}")


if __name__ == "__main__":
    main()