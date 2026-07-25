#!/usr/bin/env python3
"""Generate release-specific AppStream metadata without editing the source XML."""

from __future__ import annotations

import argparse
import re
from pathlib import Path

VERSION_RE = re.compile(r"^[0-9A-Za-z][0-9A-Za-z.+-]*$")
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Insert or update the current Davenstein AppStream release entry",
    )
    parser.add_argument("--input", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--version", required=True)
    parser.add_argument("--date", required=True)
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    version = args.version.removeprefix("v")
    release_date = args.date

    if not VERSION_RE.fullmatch(version):
        raise SystemExit(f"Invalid release version: {version}")
    if not DATE_RE.fullmatch(release_date):
        raise SystemExit(f"Invalid release date: {release_date}")

    text = args.input.read_text(encoding="utf-8")

    releases_open = "  <releases>"
    if releases_open not in text:
        raise SystemExit(f"Missing <releases> section in {args.input}")

    escaped_version = re.escape(version)
    existing = re.compile(
        rf'(?m)^(\s*)<release version="{escaped_version}" date="[^"]*"\s*/>$'
    )

    replacement = rf'\1<release version="{version}" date="{release_date}"/>'
    updated, count = existing.subn(replacement, text, count=1)

    if count == 0:
        entry = f'    <release version="{version}" date="{release_date}"/>\n'
        updated = text.replace(
            releases_open + "\n",
            releases_open + "\n" + entry,
            1,
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(updated, encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
