#!/usr/bin/env python3
"""HTML → Markdown converter for x-cmd VitePress pages.

Goals:
- Extract only the article content (skip nav/sidebar/footer/JS)
- Preserve Chinese text byte-for-byte (no Unicode normalisation
  that might change characters)
- Preserve headings, lists, code blocks, links, tables
- Output plain UTF-8 markdown

Verification:
- Run on a file that has a known .md mirror; assert that
  Chinese-character set round-trips intact.
"""
import sys
import re
import argparse
from pathlib import Path

import html2text
from bs4 import BeautifulSoup


# Configure html2text: preserve Chinese text, no wrapping
H2T = html2text.HTML2Text()
H2T.body_width = 0          # do not wrap lines
H2T.ignore_links = False    # keep [text](url)
H2T.ignore_images = False
H2T.protect_links = True
H2T.bypass_tables = False
H2T.unicode_snob = True     # use Unicode chars (good for CJK)
H2T.ignore_emphasis = False
H2T.ul_item_mark = '-'
H2T.em_str = '_'
H2T.strong_str = '**'
H2T.code_style = '`'

# Zero-width chars html2text sprinkles after headings
_ZW_CHARS = [
    '​',  # ZERO WIDTH SPACE
    '﻿',  # ZERO WIDTH NO-BREAK SPACE / BOM
    '‌',  # ZERO WIDTH NON-JOINER
    '‍',  # ZERO WIDTH JOINER
]


def extract_article(html: str):
    """Pull the article body out of a VitePress page.

    VitePress wraps content in <main>...<div class="VPDoc">...<div class="content">.
    We grab the inner-most container that holds article content.
    Fall back progressively if no candidate matches.
    """
    soup = BeautifulSoup(html, "html.parser")

    # Remove script/style/nav/aside blocks entirely
    for tag in soup(["script", "style", "noscript", "nav", "aside", "footer"]):
        tag.decompose()

    article = (
        soup.select_one("main .VPDoc .content")
        or soup.select_one("main .vp-doc")
        or soup.select_one("article")
        or soup.select_one("main")
        or soup.body
    )
    return article


def post_process(md: str) -> str:
    """Tidy up html2text output."""
    md = md.replace(' ', ' ')  # nbsp -> space
    for zw in _ZW_CHARS:
        md = md.replace(zw, '')
    md = '\n'.join(line.rstrip() for line in md.split('\n'))
    md = re.sub(r'\n{3,}', '\n\n', md)
    md = re.sub(r'(^|\n)  - ', r'\1- ', md)
    return md.strip() + '\n'


def convert_file(src: Path) -> str:
    html = src.read_text(encoding="utf-8")
    article = extract_article(html)
    md = H2T.handle(str(article))
    return post_process(md)


def verify_chinese_preserved(ref_md_path: Path, converted_md: str) -> dict:
    """Compare Chinese-character set between original MD and our conversion.

    A perfect round-trip isn't required (HTML has different heading
    markers, etc.) but the *content* Chinese chars should all be
    present and unchanged.
    """
    ref = ref_md_path.read_text(encoding="utf-8")
    ref_chars = {c for c in ref if '一' <= c <= '鿿'}
    conv_chars = {c for c in converted_md if '一' <= c <= '鿿'}
    missing = ref_chars - conv_chars
    return {
        "ref_chars": len(ref_chars),
        "conv_chars": len(conv_chars),
        "missing": sorted(missing),
        "missing_count": len(missing),
    }


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("src", type=Path, help="HTML input file")
    ap.add_argument("-o", "--out", type=Path, help="output MD path (default: stdout)")
    ap.add_argument("--verify-with", type=Path, help="reference MD for char-level check")
    args = ap.parse_args()

    md = convert_file(args.src)

    if args.verify_with and args.verify_with.exists():
        v = verify_chinese_preserved(args.verify_with, md)
        print(f"# verify: ref_chars={v['ref_chars']} conv_chars={v['conv_chars']}",
              file=sys.stderr)
        print(f"#         missing_count={v['missing_count']}", file=sys.stderr)
        if v["missing"]:
            print(f"#         MISSING: {v['missing'][:20]}", file=sys.stderr)
            sys.exit(2)

    if args.out:
        args.out.write_text(md, encoding="utf-8")
        print(f"# wrote {args.out} ({len(md)} bytes, {md.count(chr(10))} lines)",
              file=sys.stderr)
    else:
        sys.stdout.write(md)


if __name__ == "__main__":
    main()