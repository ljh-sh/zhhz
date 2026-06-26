#!/usr/bin/env bash
# Benchmark zhhz vs the system `opencc` CLI.
#
# Measures:
#   1. Cold startup: `zhhz --version`, `zhhz -c s2t < /dev/null`, and the
#      equivalents for `opencc` (with its data dir, since opencc 1.2.0
#      doesn't embed its data).
#   2. Conversion throughput on a ~10MB Chinese corpus synthesised from the
#      vendored OpenCC dictionary keys (real Simplified phrases, no external
#      download).
#
# Usage:
#   scripts/benchmark.sh [text_size_MB] [iterations]
#   defaults: 10 MB, 5 iterations
#
# Requirements: bash, `time` (shell builtin or /usr/bin/time), zhhz on PATH
# (or at $REPO/target/release/zhhz), opencc on PATH with its data dir.
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
ZHHZ_BIN="${ZHHZ_BIN:-$REPO/target/release/zhhz}"
OPENCC_BIN="${OPENCC_BIN:-opencc}"

TEXT_SIZE_MB="${1:-10}"
ITERATIONS="${2:-5}"

if ! command -v "$OPENCC_BIN" >/dev/null 2>&1; then
    echo "error: $OPENCC_BIN not found on PATH" >&2
    exit 1
fi
if [ ! -x "$ZHHZ_BIN" ]; then
    echo "error: $ZHHZ_BIN not found or not executable; build it first:" >&2
    echo "  (cd $REPO && cargo build --release)" >&2
    exit 1
fi

# Locate opencc's data dir (brew installs ship it under share/opencc/).
OPENCC_DATA_DIR="${OPENCC_DATA_DIR:-}"
if [ -z "$OPENCC_DATA_DIR" ]; then
    if command -v brew >/dev/null 2>&1; then
        prefix="$(brew --prefix "$OPENCC_BIN" 2>/dev/null || true)"
        if [ -d "$prefix/share/opencc" ]; then
            OPENCC_DATA_DIR="$prefix/share/opencc"
        fi
    fi
fi
if [ -z "$OPENCC_DATA_DIR" ] || [ ! -d "$OPENCC_DATA_DIR" ]; then
    echo "error: could not locate opencc data dir; set OPENCC_DATA_DIR=..." >&2
    exit 1
fi

# Generate a ~N MB Chinese corpus from the vendored Simplified phrases.
# We join every key in STPhrases / TWPhrases / HKPhrases / JPShinjitaiPhrases
# (real Simplified Chinese content) and repeat the join until the corpus
# reaches TEXT_SIZE_MB.
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
CORPUS="$WORK/corpus.txt"

python3 - "$REPO" "$TEXT_SIZE_MB" "$CORPUS" <<'PY'
import sys, os
repo, target_mb, out = sys.argv[1], float(sys.argv[2]), sys.argv[3]
keys = []
for name in ("STPhrases", "TWPhrases", "HKPhrases", "JPShinjitaiPhrases"):
    path = os.path.join(repo, "data", "dictionary", f"{name}.txt")
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.startswith("#") or not line.strip():
                continue
            k, _ = line.split("\t", 1)
            if k:
                keys.append(k)
target_bytes = int(target_mb * 1024 * 1024)
written = 0
with open(out, "w", encoding="utf-8") as f:
    while written < target_bytes:
        chunk = "。".join(keys) + "。"  # one sentence-ish blob
        f.write(chunk)
        written += len(chunk.encode("utf-8"))
print(f"corpus: {out} ({os.path.getsize(out)/1024/1024:.2f} MB, {len(keys)} unique keys)")
PY

CORPUS_BYTES=$(wc -c < "$CORPUS")
echo ""
echo "corpus: $CORPUS ($((CORPUS_BYTES / 1024)) KB, ${TEXT_SIZE_MB} MB target)"
echo "iterations per measurement: $ITERATIONS"
echo "opencc data dir: $OPENCC_DATA_DIR"
echo "zhhz bin: $ZHHZ_BIN"
echo "opencc bin: $OPENCC_BIN"
echo ""

# Median of N timings (wall time, in milliseconds). Uses `date +%s%N` for
# monotonic ns precision (macOS `date` supports %N).
median_ms() {
    local -a samples
    local i cmd
    cmd="$1"; shift
    for ((i=0; i<ITERATIONS; i++)); do
        local t0 t1
        t0=$(date +%s%N)
        "$cmd" "$@" >/dev/null 2>&1
        t1=$(date +%s%N)
        samples+=($(( (t1 - t0) / 1000000 )))
    done
    # sort and pick the middle
    printf '%s\n' "${samples[@]}" | sort -n | awk -v n="$ITERATIONS" 'NR==int((n+1)/2) {print $1; exit}'
}

# 1) Startup --version (minimal path; just print + exit).
zhhz_ver=$(median_ms "$ZHHZ_BIN" --version)
opencc_ver=$(median_ms "$OPENCC_BIN" --version)

# 2) Startup + convert of empty input. Closer to the real cost of invoking
# the binary for a tiny conversion.
zhhz_empty=$(median_ms "$ZHHZ_BIN" -c s2t < /dev/null)
opencc_empty=$(median_ms "$OPENCC_BIN" -c s2t --path "$OPENCC_DATA_DIR" < /dev/null)

# 3) Long-text conversion throughput. Warm up once (page cache), then
# measure ITERATIONS times.
"$ZHHZ_BIN" -c s2t < "$CORPUS" >/dev/null
"$OPENCC_BIN" -c s2t --path "$OPENCC_DATA_DIR" < "$CORPUS" >/dev/null

zhhz_conv=$(median_ms "$ZHHZ_BIN" -c s2t < "$CORPUS")
opencc_conv=$(median_ms "$OPENCC_BIN" -c s2t --path "$OPENCC_DATA_DIR" < "$CORPUS")

mb_per_s_zhhz=$(python3 -c "print(round(${CORPUS_BYTES}/1024/1024/(${zhhz_conv}/1000), 2))")
mb_per_s_opencc=$(python3 -c "print(round(${CORPUS_BYTES}/1024/1024/(${opencc_conv}/1000), 2))")

cat <<EOF
=== zhhz vs opencc (median over $ITERATIONS runs) ===

| measurement                       | zhhz          | opencc         |
|-----------------------------------|---------------|----------------|
| --version (startup)               | ${zhhz_ver} ms | ${opencc_ver} ms |
| -c s2t < /dev/null (cold convert) | ${zhhz_empty} ms | ${opencc_empty} ms |
| convert $((CORPUS_BYTES/1024/1024)) MB corpus (s2t) | ${zhhz_conv} ms | ${opencc_conv} ms |
| throughput (s2t)                  | ${mb_per_s_zhhz} MB/s | ${mb_per_s_opencc} MB/s |
EOF
