#!/usr/bin/env python3
"""Build a factual API inventory from Flutter's official Dartdoc search index.
Stores symbol names/links/counts only, not documentation text or implementation.
An indexed declaration is not considered reviewed or implemented.
"""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import time
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
URL = "https://api.flutter.dev/flutter/index.json"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--input", type=Path, default=ROOT / "target/flutter-api-index.json")
parser.add_argument("--refresh", action="store_true", help="download a fresh official index, with a 120-second transfer limit")
args = parser.parse_args()
if args.refresh:
    args.input.parent.mkdir(parents=True, exist_ok=True)
    chunks = []
    deadline = time.monotonic() + 120
    with urlopen(URL, timeout=20) as response:
        while chunk := response.read(65536):
            if time.monotonic() > deadline:
                raise TimeoutError("official index transfer exceeded 120 seconds")
            chunks.append(chunk)
    args.input.write_bytes(b"".join(chunks))
source = args.input.read_bytes()
entries = json.loads(source)
libraries = [e for e in entries if e.get("kind") == 9 and (
    e.get("packageRank") == 0 or e["href"].startswith("flutter_") or e["href"] == "dart-ui/"
)]
by_library = {}
for library in libraries:
    prefix = library["href"].split("/")[0]
    by_library[prefix] = {"name": library["name"], "url": "https://api.flutter.dev/flutter/" + library["href"],
                          "indexed_symbols": 0, "declarations": []}
for entry in entries:
    prefix = entry["href"].split("/")[0]
    if prefix not in by_library:
        continue
    library = by_library[prefix]
    library["indexed_symbols"] += 1
    if entry.get("enclosedBy", {}).get("kind") == 9:
        library["declarations"].append([entry["qualifiedName"], entry["kind"], entry["href"]])
for library in by_library.values():
    library["declarations"].sort(key=lambda d: d[0])
result = {
    "source": URL, "source_sha256": hashlib.sha256(source).hexdigest(),
    "generated_utc": datetime.now(timezone.utc).isoformat(),
    "scope": "Flutter SDK libraries, dart:ui, and flutter_* companion packages present in the official index",
    "default_review_status": "unreviewed",
    "declaration_columns": ["qualified_name", "upstream_kind", "relative_documentation_url"],
    "limitations": "Search-index declarations are an inventory, not a semantic audit or compatibility claim. Members are counted but not individually listed. Backend equivalents are not Flutter parity. The upstream index does not declare an SDK version; review against a pinned Flutter release before claiming completeness.",
    "upstream_total_indexed_symbols": len(entries),
    "included_indexed_symbols": sum(v["indexed_symbols"] for v in by_library.values()),
    "included_top_level_declarations": sum(len(v["declarations"]) for v in by_library.values()),
    "libraries": sorted(by_library.values(), key=lambda v: v["name"]),
}
output = ROOT / "docs/flutter-api-inventory.json"
output.write_text(json.dumps(result, ensure_ascii=False, indent=2) + "\n")
summary = ROOT / "docs/flutter-api-inventory.md"
lines = ["# Flutter API inventory", "", f"Source: [official Dartdoc search index]({URL}).", "",
         f"The source contains **{len(entries):,}** indexed symbols. The selected SDK and companion libraries contain **{result['included_indexed_symbols']:,}** symbols and **{result['included_top_level_declarations']:,}** top-level declarations.", "",
         "Names, links and counts are stored in `flutter-api-inventory.json`; documentation descriptions are excluded. All declarations default to **unreviewed**. Counts do not establish implementation or behavioral compatibility. Members are counted but are not individually listed. The source index does not expose a release version, so a pinned-release audit remains necessary.", "",
         "Regenerate with `python3 scripts/flutter-inventory.py --refresh`, or use `--input` with a cached official index. See `flutter-parity.md` for the implementation-level feature audit.", "",
         "| Library | Indexed symbols | Top-level declarations |", "| --- | ---: | ---: |"]
for library in result["libraries"]:
    lines.append(f"| [{library['name']}]({library['url']}) | {library['indexed_symbols']:,} | {len(library['declarations']):,} |")
summary.write_text("\n".join(lines) + "\n")
print(f"Inventoried {result['included_top_level_declarations']:,} declarations across {len(libraries)} libraries; all remain unreviewed until audited")
