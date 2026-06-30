#!/usr/bin/env python3
"""Check local Markdown links in selected files or directories."""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from urllib.parse import unquote, urlparse


LINK_RE = re.compile(r"(?<!!)\[[^\]]+\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
HEADING_RE = re.compile(r"^(#{1,6})\s+(.+?)\s*#*\s*$")
SKIP_SCHEMES = {"http", "https", "mailto", "tel"}


def slugify(heading: str) -> str:
    text = re.sub(r"<[^>]+>", "", heading)
    text = re.sub(r"[^\w\s-]", "", text, flags=re.UNICODE).strip().lower()
    return re.sub(r"[\s_-]+", "-", text).strip("-")


def markdown_files(paths: list[str]) -> list[Path]:
    files: list[Path] = []
    for raw_path in paths:
        path = Path(raw_path)
        if path.is_dir():
            files.extend(sorted(path.rglob("*.md")))
        elif path.suffix == ".md":
            files.append(path)
    return files


def anchors_for(path: Path) -> set[str]:
    anchors: set[str] = set()
    used: dict[str, int] = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        match = HEADING_RE.match(line)
        if not match:
            continue
        base = slugify(match.group(2))
        count = used.get(base, 0)
        used[base] = count + 1
        anchors.add(base if count == 0 else f"{base}-{count}")
    return anchors


def check_file(path: Path) -> list[str]:
    errors: list[str] = []
    text = path.read_text(encoding="utf-8")
    for line_number, line in enumerate(text.splitlines(), start=1):
        for raw_target in LINK_RE.findall(line):
            parsed = urlparse(raw_target)
            if parsed.scheme in SKIP_SCHEMES:
                continue
            if parsed.scheme or raw_target.startswith("#"):
                target_path = path
                fragment = parsed.fragment or raw_target.removeprefix("#")
            else:
                target_path = (path.parent / unquote(parsed.path)).resolve()
                fragment = parsed.fragment
            if not target_path.exists():
                errors.append(f"{path}:{line_number}: missing link target {raw_target}")
                continue
            if fragment and slugify(unquote(fragment)) not in anchors_for(target_path):
                errors.append(f"{path}:{line_number}: missing anchor {raw_target}")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("paths", nargs="+")
    args = parser.parse_args()

    errors: list[str] = []
    for path in markdown_files(args.paths):
        errors.extend(check_file(path))

    if errors:
        print("Markdown link check failed", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print("Markdown link check passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
