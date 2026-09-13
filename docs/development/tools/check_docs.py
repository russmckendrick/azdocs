#!/usr/bin/env python3
"""Check local documentation links, TOML examples and the built-in query catalogue."""
from __future__ import annotations

from collections import Counter
import html
from pathlib import Path
import re
import sys
import tomllib
from urllib.parse import unquote, urlsplit

ROOT = Path(__file__).resolve().parents[3]


def without_fences(source: str) -> str:
    return re.sub(r"(?ms)^(`{3,}|~{3,})[^\n]*\n.*?^\1\s*$", "", source)


def anchors(source: str) -> set[str]:
    result: set[str] = set()
    seen: Counter[str] = Counter()
    for title in re.findall(r"(?m)^#{1,6}\s+(.+?)\s*#*\s*$", without_fences(source)):
        title = re.sub(r"!?\[([^]]+)\]\([^)]*\)", r"\1", title)
        title = re.sub(r"<[^>]+>", "", title)
        title = html.unescape(title).lower()
        slug = "".join(c for c in title if c.isalnum() or c in " _-").replace(" ", "-")
        suffix = f"-{seen[slug]}" if seen[slug] else ""
        seen[slug] += 1
        result.add(slug + suffix)
    result.update(re.findall(r'''(?:id|name)=["']([^"']+)["']''', source))
    return result


def links(source: str) -> list[str]:
    source = without_fences(source)
    # Reference definitions and ordinary Markdown links cover the guides;
    # HTML attributes cover picture/source elements in the mark catalogue.
    result = re.findall(r"!?\[[^\]\n]*\]\(<?([^\s)>]+)>?(?:\s+\"[^\"]*\")?\)", source)
    result += re.findall(r"(?m)^\[[^]]+\]:\s*<?([^\s>]+)", source)
    result += re.findall(r'''(?:href|src)=["']([^"']+)["']''', source)
    for srcset in re.findall(r'''srcset=["']([^"']+)["']''', source):
        result.extend(entry.strip().split()[0] for entry in srcset.split(",") if entry.strip())
    return result


def main() -> int:
    documents = sorted([*ROOT.glob("*.md"), *ROOT.joinpath("docs").rglob("*.md")])
    errors: list[str] = []
    link_count = toml_count = 0
    for path in documents:
        source = path.read_text()
        label = path.relative_to(ROOT)
        for target in links(source):
            target = html.unescape(target)
            parts = urlsplit(target)
            if parts.scheme or parts.netloc:
                continue
            link_count += 1
            resolved = (path.parent / unquote(parts.path)).resolve() if parts.path else path
            if not resolved.is_relative_to(ROOT):
                errors.append(f"{label}: link leaves repository: {target}")
            elif not resolved.exists():
                errors.append(f"{label}: missing link target: {target}")
            elif parts.fragment and resolved.suffix in (".md", ".html"):
                if unquote(parts.fragment) not in anchors(resolved.read_text()):
                    errors.append(f"{label}: missing anchor: {target}")
        for block in re.findall(r"(?ms)^```toml\s*\n(.*?)^```\s*$", source):
            toml_count += 1
            try:
                tomllib.loads(block)
            except tomllib.TOMLDecodeError as exc:
                errors.append(f"{label}: invalid TOML example: {exc}")

    definitions = [tomllib.loads(path.read_text()) for path in sorted(ROOT.joinpath("queries").rglob("*.toml"))]
    reference = ROOT.joinpath("docs/reference/queries.md").read_text()
    names = Counter(item["name"] for item in definitions)
    listed = Counter(re.findall(r"(?m)^\| `([a-z0-9_]+)` \|", reference.split("## Rust-side audits")[0]))
    # The operational table occurs later in the document than the shared audit notes.
    operational = reference.split("## Operational and access evidence", 1)[-1]
    listed.update(re.findall(r"(?m)^\| `([a-z0-9_]+)` \|", operational))
    for name, count in names.items():
        if count != 1 or listed[name] != 1:
            errors.append(f"query catalogue: {name} has {count} definitions and {listed[name]} entries")
    for name in listed.keys() - names.keys():
        errors.append(f"query catalogue: unknown entry {name}")
    count = re.search(r"(?m)^(\d+) queries ship", reference)
    if not count or int(count[1]) != len(definitions):
        errors.append("query catalogue: total count does not match queries/")
    categories = Counter(item["category"] for item in definitions)
    pie = dict((name, int(count)) for name, count in re.findall(r'(?m)^    "([^"]+)" : (\d+)$', reference))
    if dict(categories) != pie:
        errors.append("query catalogue: category chart does not match queries/")
    kinds = Counter(item["kind"] for item in definitions)
    declared = re.search(r"\*\*(\d+) inventory queries and (\d+) finding queries\*\*", reference)
    if not declared or (int(declared[1]), int(declared[2])) != (kinds["inventory"], kinds["finding"]):
        errors.append("query catalogue: inventory/finding counts do not match queries/")
    if errors:
        print("\n".join(errors), file=sys.stderr)
        return 1
    print(f"Documentation OK: {len(documents)} Markdown files, {link_count} local links, {toml_count} TOML examples, {len(definitions)} queries.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
