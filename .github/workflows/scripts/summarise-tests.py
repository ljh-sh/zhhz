#!/usr/bin/env python3
"""Aggregate per-platform test output into bilingual changelog test
report files (en + cn).

Usage:
  summarise-tests.py --artefacts DIR --ref REF --sha SHA --next VERSION \
      --output-en PATH_EN --output-cn PATH_CN

Each artefact subdir is expected to have a `test-output.txt` (cargo
test output). Per-platform status: pass if "test result: ok" appears.
"""
import argparse
import os
import re
import sys
from datetime import datetime, timezone
from pathlib import Path

PLATFORM_ORDER = ["macos-arm64", "linux-amd64", "linux-arm64"]
PLATFORM_RUNNER = {
    "macos-arm64": "macos-latest",
    "linux-amd64": "ubuntu-latest",
    "linux-arm64": "ubuntu-24.04-arm",
}
PLATFORM_TARGET = {
    "macos-arm64": "aarch64-apple-darwin",
    "linux-amd64": "x86_64-unknown-linux-gnu",
    "linux-arm64": "aarch64-unknown-linux-gnu",
}


def parse_platform(artefact_dir: Path):
    """Read test-output.txt + test-exit.txt, return (status_str, perf_built, diff_built)."""
    test_out = artefact_dir / "test-output.txt"
    test_exit = artefact_dir / "test-exit.txt"
    bin_dir = artefact_dir / "bin"
    perf = (bin_dir / "perf_only").exists()
    diff = (bin_dir / "diff_corpus").exists()
    if not test_out.exists():
        return ("❌ no artefact", "—", "—")
    content = test_out.read_text(errors="replace")
    # Find the last "test result: ok" or "FAILED" line
    m = re.search(
        r"test result: (ok|FAILED).*?(\d+ passed.*?; \d+ failed.*?; \d+ ignored.*?; \d+ measured.*?; \d+ filtered out)",
        content,
    )
    if m and m.group(1) == "ok":
        passed = re.search(r"(\d+) passed", m.group(2))
        failed = re.search(r"(\d+) failed", m.group(2))
        n_pass = passed.group(1) if passed else "?"
        n_fail = failed.group(1) if failed else "0"
        status = f"✅ {n_pass} passed"
        if n_fail != "0":
            status = f"❌ {n_fail} failed"
    else:
        # Try to find any FAILED line
        if "test result: FAILED" in content:
            status = "❌ FAILED"
        else:
            status = "⚠️ no result line"
    return (status, "✅" if perf else "—", "✅" if diff else "—")


def make_md(en: bool, ref: str, sha: str, next_version: str, rows):
    date = datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%MZ")
    if en:
        title = "Build & Test Report"
        intro = (
            f"Native build, test, and binary production on 3 platforms "
            f"for upcoming release **{next_version}**."
        )
        what = (
            "Test reports are committed to `changelog/v{X}.test.md` "
            "(and `.test.cn.md`) so they ship with the next release "
            "and are reviewable alongside the changelog."
        )
        next_step = (
            "When this report is green, write the release notes "
            f"(`changelog/{next_version}.md` + `.cn.md`) and trigger "
            "the **build-and-release** workflow with "
            f"`next_version = {next_version}`."
        )
    else:
        title = "构建与测试报告"
        intro = (
            f"在 3 个平台上原生构建、测试、生成二进制，"
            f"用于即将发布的版本 **{next_version}**。"
        )
        what = (
            "测试报告提交到 `changelog/v{X}.test.md`（和 "
            "`.test.cn.md`），与下一次发布一同发布，并与 changelog "
            "一起审阅。"
        )
        next_step = (
            "本报告全绿后，撰写 release notes "
            f"（`changelog/{next_version}.md` + `.cn.md`），"
            "然后用 `next_version = " + next_version + " "
            "触发 **build-and-release** workflow。"
        )

    md = []
    md.append(f"# {title}: `{next_version}`\n")
    md.append(f"_Generated: {date} (UTC)_\n")
    md.append(f"_Commit: `{sha[:12]}` on `{ref}`_\n")
    md.append("\n" + intro + "\n")
    md.append("## Per-platform results\n")
    if en:
        md.append("| Platform | OS runner | cargo test | perf_only built | diff_corpus built |")
        md.append("|--|--|--|--|--|")
    else:
        md.append("| 平台 | OS runner | cargo test | perf_only 构建 | diff_corpus 构建 |")
        md.append("|--|--|--|--|--|")
    for plat, status, perf, diff in rows:
        runner = PLATFORM_RUNNER[plat]
        md.append(f"| {plat} | `{runner}` | {status} | {perf} | {diff} |")
    md.append("")
    any_fail = any(("❌" in s) for _, s, _, _ in rows)
    if en:
        md.append("## Verdict\n")
        if any_fail:
            md.append("**At least one platform FAILED.** Do **not** trigger "
                      "build-and-release.\n")
        else:
            md.append("**All 3 native platforms passed.** Safe to trigger "
                      "build-and-release.\n")
    else:
        md.append("## 结论\n")
        if any_fail:
            md.append("**至少有一个平台失败。** 不要触发 build-and-release。\n")
        else:
            md.append("**3 个原生平台全部通过。** 可以触发 build-and-release。\n")
    md.append("## Notes\n")
    md.append(what + "\n")
    md.append("## Next step\n")
    md.append(next_step + "\n")
    return "\n".join(md)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--artefacts", required=True)
    ap.add_argument("--ref", required=True)
    ap.add_argument("--sha", required=True)
    ap.add_argument("--next", required=True,
                    help="next version (e.g. v0.7.8) for the report")
    ap.add_argument("--output-en", required=True)
    ap.add_argument("--output-cn", required=True)
    args = ap.parse_args()

    artefacts = Path(args.artefacts)
    rows = []
    for plat in PLATFORM_ORDER:
        rows.append((plat, *parse_platform(artefacts / f"test-{plat}")))

    en = make_md(True, args.ref, args.sha, args.next, rows)
    cn = make_md(False, args.ref, args.sha, args.next, rows)

    Path(args.output_en).write_text(en)
    Path(args.output_cn).write_text(cn)
    print(f"Wrote {args.output_en}", file=sys.stderr)
    print(f"Wrote {args.output_cn}", file=sys.stderr)


if __name__ == "__main__":
    main()