#!/usr/bin/env bash
# Apples-to-apples perf bench across zhhz v0.7.4, v0.8.0-marisa, opencc 1.3.1.
# Same corpora, same warmup, same procedure.
set -euo pipefail

ZHHZ_V074=/tmp/zhhz-v074-target/release/examples/perf_only
ZHHZ_MARISA=/tmp/zhhz-marisa-target/release/examples/perf_only

CORPORA=(
  "realistic /tmp/corpus-10mb.txt"
  "news /tmp/corpus-news.txt"
  "code /tmp/corpus-code.txt"
  "classical /tmp/corpus-classical.txt"
)

INNER_RUNS=20
WARMUP=3
TOTAL_RUNS=$((INNER_RUNS + WARMUP))

run_perf_only() {
    local bin="$1" mode="$2" corpus_path="$3" label="$4"
    # perf_only runs INNER_RUNS converts internally + WARMUP warmup discarded.
    # Capture /usr/bin/time -l output and parse.
    local rawfile
    rawfile=$(mktemp)
    /usr/bin/time -l "$bin" "$mode" "$INNER_RUNS" "$WARMUP" "$corpus_path" >/dev/null 2>"$rawfile"
    local padded_bytes
    padded_bytes=$(python3 -c "
with open('$corpus_path') as f:
    s = f.read()
while len(s) < 1024 * 1024:
    s = s + s
print(len(s.encode('utf-8')))
")
    python3 - "$label" "$mode" "$rawfile" "$INNER_RUNS" "$padded_bytes" <<'PY'
import sys, re
label, mode, path, inner_runs, padded = sys.argv[1:6]
inner_runs = int(inner_runs)
padded = int(padded)
text = open(path).read()
# perf_only prints "throughput: X.XX MB/s (best)" — we use this for accuracy
m_mbps = re.search(r"throughput:\s+(\d+\.\d+)", text)
m_instr = re.search(r"(\d[\d,]*)\s+instructions retired", text)
m_cyc   = re.search(r"(\d[\d,]*)\s+cycles elapsed", text)
if not (m_mbps and m_instr and m_cyc):
    print(f"  {label}: NO DATA")
    sys.exit(0)
mbps = float(m_mbps.group(1))
instr = int(m_instr.group(1).replace(",",""))
cyc = int(m_cyc.group(1).replace(",",""))
instr_per = instr / inner_runs
cyc_per = cyc / inner_runs
ipc = instr / cyc
print(f"{label:<22} {mode:<8} {mbps:10.2f} MB/s  instr/conv={instr_per:.2e}  cyc/conv={cyc_per:.2e}  IPC={ipc:.2f}")
PY
    rm -f "$rawfile"
}

run_opencc() {
    local corpus_path="$1" label="$2"
    # For opencc, each run is a full process. Average over $TOTAL_RUNS runs.
    local tmpf
    tmpf=$(mktemp)
    local padded
    padded=$(python3 -c "
import sys
with open('$corpus_path') as f:
    s = f.read()
while len(s) < 1024 * 1024:
    s = s + s
with open('$tmpf','w') as f:
    f.write(s)
print(len(s.encode('utf-8')))
")
    # Collect all samples then summarize in Python (simpler awk).
    local rawfile
    rawfile=$(mktemp)
    for ((i = 0; i < TOTAL_RUNS; i++)); do
        /usr/bin/time -l opencc -c s2t -i "$tmpf" >/dev/null 2>>"$rawfile"
        echo "---END---" >>"$rawfile"
    done
    python3 - "$padded" "$INNER_RUNS" "$WARMUP" "$label" "$rawfile" <<'PY'
import sys, re
padded, runs, warmup, label, path = sys.argv[1:6]
runs, warmup = int(runs), int(warmup)
padded = int(padded)
samples = open(path).read().split("---END---")[:-1]
parsed = []
for s in samples:
    m_real = re.search(r"(\d+\.\d+)\s+real\s+(\d+\.\d+)\s+user", s)
    m_instr = re.search(r"(\d[\d,]*)\s+instructions retired", s)
    m_cyc   = re.search(r"(\d[\d,]*)\s+cycles elapsed", s)
    if m_real and m_instr and m_cyc:
        parsed.append((
            float(m_real.group(2)),
            int(m_instr.group(1).replace(",","")),
            int(m_cyc.group(1).replace(",","")),
        ))
measured = parsed[warmup:]
if not measured:
    print(f"  opencc 1.3.1 ({label}): NO SAMPLES — got {len(samples)} runs, {len(parsed)} parsed")
    sys.exit(0)
mbps_list = [padded/1048576.0/u for u,_,_ in measured]
avg_mbps = sum(mbps_list) / len(mbps_list)
avg_instr = sum(i for _,i,_ in measured) / len(measured)
avg_cyc = sum(c for _,_,c in measured) / len(measured)
ipc = avg_instr / avg_cyc
print(f"{label+' (opencc 1.3.1)':<22} {'fast':<8} {avg_mbps:10.2f} MB/s  instr/conv={avg_instr:.2e}  cyc/conv={avg_cyc:.2e}  IPC={ipc:.2f}")
PY
    rm -f "$tmpf" "$rawfile"
}

echo "=== apples-to-apples bench ==="
echo "(zhhz: in-process $INNER_RUNS inner converts + $WARMUP warmup discarded)"
echo "(opencc: $INNER_RUNS subprocess calls + $WARMUP warmup discarded)"
echo ""
printf "%-22s %-8s %-22s %-22s %s\n" "binary" "mode" "throughput" "instr/convert" "IPC"
echo "-------------------------------------------------------------------------------------------------"

for entry in "${CORPORA[@]}"; do
    label=${entry%% *}
    path=${entry##* }
    echo "--- corpus: $label ---"
    run_perf_only "$ZHHZ_V074" fast    "$path" "zhhz v0.7.4 fast"
    run_perf_only "$ZHHZ_V074" trigram "$path" "zhhz v0.7.4 trigram"
    run_perf_only "$ZHHZ_MARISA" fast    "$path" "zhhz v0.8.0-marisa fast"
    run_perf_only "$ZHHZ_MARISA" trigram "$path" "zhhz v0.8.0-marisa trigram"
    run_opencc "$path" "opencc 1.3.1"
    echo ""
done