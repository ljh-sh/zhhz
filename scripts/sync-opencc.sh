#!/usr/bin/env bash
# Re-vendor OpenCC dictionary data from upstream into data/.
#
# Usage:
#   scripts/sync-opencc.sh             # master HEAD
#   scripts/sync-opencc.sh 1.3.1       # a tag, branch, or commit SHA
#
# What this does:
#   1. Fetches BYVoid/OpenCC at the requested ref (tarball, falling back to the
#      p.ljh.sh GitHub proxy if github.com is unreachable).
#   2. Overwrites data/dictionary/*.txt and data/config/*.json with the upstream
#      copies (pure mirror).
#   3. Records the commit SHA + date in data/UPSTREAM.
#
# The five build-time-generated dictionaries are NOT vendored here; build.rs
# regenerates them deterministically on the next `cargo build`.
set -euo pipefail

REF="${1:-master}"
REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

OPENCC_TARBALL="$TMP/opencc.tar.gz"

echo ">> fetching BYVoid/OpenCC @ ${REF}"
fetch() {
    local url="$1"
    curl -fL --retry 3 --max-time 300 -o "$OPENCC_TARBALL" "$url"
}
URLS=(
    "https://github.com/BYVoid/OpenCC/archive/${REF}.tar.gz"
    "https://p.ljh.sh/https://github.com/BYVoid/OpenCC/archive/${REF}.tar.gz"
)
ok=0
for url in "${URLS[@]}"; do
    if fetch "$url"; then ok=1; break; fi
    echo "   (failed: $url)"
done
if [ "$ok" -ne 1 ]; then
    echo "error: could not download OpenCC from any source" >&2
    exit 1
fi

tar -xzf "$OPENCC_TARBALL" -C "$TMP"
SRC="$(find "$TMP" -maxdepth 1 -type d -name 'OpenCC-*' | head -1)"
if [ -z "$SRC" ]; then
    echo "error: could not find extracted OpenCC source" >&2
    exit 1
fi

# Resolve the commit SHA. For a tag/branch ref, ask the GitHub API; for a SHA,
# the ref itself is the SHA.
case "$REF" in
    *[!0-9a-f]*|"")
        SHA="$(curl -fsSL --max-time 30 "https://api.github.com/repos/BYVoid/OpenCC/commits/${REF}" \
               | grep -m1 '"sha"' | sed -E 's/.*"sha": *"?([0-9a-f]+).*/\1/' || true)"
        ;;
    *)
        SHA="$REF"
        ;;
esac
# If the API call failed, fall back to whatever ref we used.
SHA="${SHA:-$REF}"
DATE="$(date -u +%Y-%m-%d)"

echo ">> SHA=${SHA} DATE=${DATE}"
echo ">> copying data/dictionary and data/config"
mkdir -p "$REPO_ROOT/data/dictionary" "$REPO_ROOT/data/config"
cp "$SRC"/data/dictionary/*.txt "$REPO_ROOT/data/dictionary/"
cp "$SRC"/data/config/*.json "$REPO_ROOT/data/config/"

# Keep the generation scripts around for provenance/auditability.
mkdir -p "$REPO_ROOT/data/scripts"
cp "$SRC"/data/scripts/reverse.py \
   "$SRC"/data/scripts/extract_tofu_risk.py \
   "$SRC"/data/scripts/generate_st_phrases_from_regional_phrases.py \
   "$SRC"/data/scripts/common.py \
   "$REPO_ROOT/data/scripts/" 2>/dev/null || true

cat > "$REPO_ROOT/data/UPSTREAM" <<EOF
# Vendored from BYVoid/OpenCC at build time. Do not edit by hand.
# Re-vendor with: scripts/sync-opencc.sh [ref]  (default: master)
repo:    https://github.com/BYVoid/OpenCC
commit:  ${SHA}
branch:  ${REF}
date:    ${DATE}
license: Apache-2.0 (code + dictionary data; see data/dictionary headers and root LICENSE)
contents:
  data/dictionary/*.txt  - 17 OpenCC source dictionaries (unchanged)
  data/config/*.json     - 16 conversion configs + opencc_config.schema.json
derived at build time (build.rs -> OUT_DIR, not committed):
  TSCharactersExt.txt                          <- extract_tofu_risk(TSCharacters.txt)
  TWVariantsRev.txt / HKVariantsRev.txt        <- reverse(TW|HKVariants.txt)
  JPShinjitaiCharactersRev.txt                 <- reverse(JPShinjitaiCharacters.txt)
  STPhrases_GeneratedFromRegionalPhrases.txt   <- t2s(HKPhrases+TWPhrases keys)
EOF

echo ">> verifying build.rs still generates cleanly"
( cd "$REPO_ROOT" && cargo build --release )

echo ">> done. Review the diff, then commit data/ + data/UPSTREAM."
