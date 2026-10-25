#!/usr/bin/env python3
"""Render a local coverage snapshot from cargo-llvm-cov's LLVM JSON export."""

import argparse
import datetime
import html
import json
import pathlib
import re
import sys
from decimal import Decimal, ROUND_HALF_UP


def source_coverage(report, root, lcov):
    """Count every exported Rust file under src/, without selective exclusions."""
    if report.get("type") != "llvm.coverage.json.export":
        raise ValueError("expected an LLVM coverage JSON export")
    source_root = (root / "src").resolve()
    expected = {}
    for dataset in report["data"]:
        for entry in dataset["files"]:
            path = pathlib.Path(entry["filename"])
            if not path.is_absolute():
                path = root / path
            path = path.resolve()
            if path.suffix != ".rs" or not path.is_relative_to(source_root):
                continue
            name = path.relative_to(root).as_posix()
            if entry["filename"] in expected:
                raise ValueError(f"duplicate LLVM filename: {entry['filename']}")
            lines = entry["summary"]["lines"]
            total, covered = lines["count"], lines["covered"]
            if not all(type(value) is int for value in (total, covered)):
                raise ValueError(f"non-integer line counts: {name}")
            if not 0 <= covered <= total:
                raise ValueError(f"invalid line counts: {name}")
            expected[entry["filename"]] = (name, covered, total)
    # LLVM records main.rs twice because md2pdf uses include!("../main.rs").
    # Use LLVM's own per-line LCOV counts to union those source lines accurately.
    # A line covered by either binary counts once; no hand-written Rust parsing.
    source = None
    lines = {}
    declared = {}
    unique = {}
    seen = set()
    for record in lcov.splitlines():
        if record.startswith("SF:"):
            if source is not None:
                raise ValueError("LCOV source record missing end_of_record")
            source, lines, declared = record[3:], {}, {}
        elif record.startswith("DA:") and source in expected:
            number, hits, *_ = record[3:].split(",")
            number, hits = int(number), int(hits)
            if number <= 0 or hits < 0 or number in lines:
                raise ValueError(f"invalid or duplicate LCOV line: {source}:{number}")
            lines[number] = hits > 0
        elif record.startswith(("LF:", "LH:")) and source in expected:
            key, value = record.split(":", 1)
            declared[key] = int(value)
        elif record == "end_of_record":
            if source in expected:
                name, covered, total = expected[source]
                if source in seen:
                    raise ValueError(f"duplicate LCOV source: {source}")
                if declared != {"LF": total, "LH": covered}:
                    raise ValueError(f"JSON and LCOV summary counts disagree: {source}")
                if len(lines) > total or sum(lines.values()) > covered:
                    raise ValueError(f"LCOV line records exceed summary counts: {source}")
                seen.add(source)
                merged = unique.setdefault(name, {})
                for number, hit in lines.items():
                    merged[number] = merged.get(number, False) or hit
            source = None
    if source is not None or seen != set(expected):
        raise ValueError("LCOV report is incomplete or differs from LLVM JSON")
    files = {name: (sum(lines.values()), len(lines))
             for name, lines in unique.items()}
    if not files or not sum(total for _, total in files.values()):
        raise ValueError("no instrumented project Rust source lines found")
    # Missing files are reported, rather than silently disappearing from scope.
    missing = sorted(
        path.relative_to(root).as_posix()
        for path in source_root.rglob("*.rs")
        if path.relative_to(root).as_posix() not in files
    )
    return dict(sorted(files.items())), missing


def percent(covered, total):
    if total == 0:
        return "n/a"
    value = (Decimal(covered) * 100 / Decimal(total)).quantize(
        Decimal("0.1"), rounding=ROUND_HALF_UP
    )
    return f"{value}%"


def badge(coverage):
    label = "coverage snapshot"
    value = html.escape(coverage)
    title = f"{label}: {value} of instrumented Rust source lines covered"
    return f'''<svg xmlns="http://www.w3.org/2000/svg" width="206" height="20" role="img" aria-label="{title}">
  <title>{title}</title>
  <linearGradient id="shade" x2="0" y2="100%"><stop offset="0" stop-color="#bbb" stop-opacity=".1"/><stop offset="1" stop-opacity=".1"/></linearGradient>
  <clipPath id="round"><rect width="206" height="20" rx="3"/></clipPath>
  <g clip-path="url(#round)"><path fill="#555" d="M0 0h137v20H0z"/><path fill="#007f86" d="M137 0h69v20h-69z"/><path fill="url(#shade)" d="M0 0h206v20H0z"/></g>
  <g fill="#fff" text-anchor="middle" font-family="Verdana,DejaVu Sans,sans-serif" font-size="11"><text x="68.5" y="14">{label}</text><text x="171.5" y="14">{value}</text></g>
</svg>
'''


def snapshot(files, missing, args):
    covered = sum(value[0] for value in files.values())
    total = sum(value[1] for value in files.values())
    rows = "\n".join(
        f"| `{name}` | {hit:,} | {count:,} | {percent(hit, count)} |"
        for name, (hit, count) in files.items()
    )
    absent = (
        ", ".join(f"`{path}`" for path in missing)
        if missing else "None."
    )
    return f'''# Rust coverage snapshot

**{percent(covered, total)} line coverage — {covered:,} / {total:,} unique instrumented source lines.**

This is a measured snapshot, not a live coverage service or a security guarantee.
The README badge links to the checked-in snapshot; CI artifacts can be newer.

| Measurement | Value |
|---|---|
| Source commit | [`{args.source_commit[:12]}`](https://github.com/rainmana/md-to-pdf/commit/{args.source_commit}) |
| Measured (UTC) | {args.date} |
| Platform | {args.platform} |
| Rust toolchain | {args.toolchain} |
| Coverage tool | cargo-llvm-cov 0.8.4 / LLVM LCOV per-line records |
| Test command | `cargo llvm-cov --locked --all-targets --json` |

The denominator includes every instrumented Rust file under `src/`, including
inline unit-test modules in those files. Standalone `tests/` files execute but
are excluded from the denominator, as are dependencies, the archived TypeScript
implementation, and generated/vendor files outside `src/`. Files that LLVM does
not report are listed below; they do not silently count as covered. Coverage
depends on the host platform and compiler, and is not branch coverage. Stable
Rust doctests are not instrumented by this command. LLVM's JSON and LCOV file
summary counts must agree. This badge counts unique `DA` line records from LLVM's
LCOV export; LLVM's own aggregate summaries can count more lines than its explicit
per-line records, so their aggregate percentage can differ. The raw exports are
retained for inspection. The generator unions line records after resolving
paths: `md2pdf` includes `main.rs`, so its separate compilation is merged with
`md-to-pdf` instead of counting the same source lines twice. A line hit by either
binary counts once.

| Source file | Covered lines | Instrumented lines | Coverage |
|---|---:|---:|---:|
{rows}
| **Total** | **{covered:,}** | **{total:,}** | **{percent(covered, total)}** |

Source files absent from LLVM's report: {absent}

## Refresh locally

Use the repository's pinned Rust toolchain and install the coverage tooling:

```sh
rustup component add llvm-tools-preview
cargo install cargo-llvm-cov --locked --version 0.8.4
mkdir -p .analysis/coverage
CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 \\
  cargo llvm-cov --locked --all-targets --json \\
  --output-path .analysis/coverage/coverage.json
cargo llvm-cov report --lcov --output-path .analysis/coverage/coverage.lcov
python3 scripts/coverage-badge.py .analysis/coverage/coverage.json \\
  --lcov .analysis/coverage/coverage.lcov \\
  --source-commit "$(git rev-parse HEAD)" --date "$(date -u +%F)" \\
  --platform "$(rustc -vV | sed -n 's/^host: //p')" \\
  --toolchain "$(rustc --version)"
git diff -- assets/badges/coverage.svg docs/COVERAGE.md
```

The generator uses only Python's standard library. It takes LLVM's measured
line records rather than counting source text or selecting successful functions.
Review the two generated files before committing a new snapshot. Use `--offline`
with `cargo llvm-cov` when all locked Cargo dependencies are already cached.

## Continuous reports

The [Coverage workflow](https://github.com/rainmana/md-to-pdf/actions/workflows/coverage.yml) runs on pushes, pull
requests, and manual dispatch. It uploads the raw LLVM JSON and LCOV, generated SVG, and
report, and adds the report to the GitHub job summary. It has read-only repository
permissions and does not upload to an external coverage service, publish Pages,
or commit changes. Its latest artifacts can be newer than this checked-in badge;
refreshing the snapshot remains an explicit reviewed documentation change.
'''


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("coverage_json", type=pathlib.Path)
    parser.add_argument("--lcov", type=pathlib.Path, required=True)
    parser.add_argument("--repo-root", type=pathlib.Path,
                        default=pathlib.Path(__file__).resolve().parents[1])
    parser.add_argument("--badge", type=pathlib.Path,
                        default=pathlib.Path("assets/badges/coverage.svg"))
    parser.add_argument("--report", type=pathlib.Path,
                        default=pathlib.Path("docs/COVERAGE.md"))
    parser.add_argument("--source-commit", required=True)
    parser.add_argument("--date", required=True)
    parser.add_argument("--platform", required=True)
    parser.add_argument("--toolchain", required=True)
    args = parser.parse_args()
    if not re.fullmatch(r"[0-9a-f]{40}", args.source_commit):
        parser.error("--source-commit must be a full lowercase Git commit hash")
    try:
        datetime.date.fromisoformat(args.date)
        # Metadata is inserted into Markdown tables, so reject extra rows/columns.
        if any("\n" in value or "\r" in value or "|" in value
               for value in (args.platform, args.toolchain)):
            raise ValueError("platform/toolchain metadata must be single-line table values")
        root = args.repo_root.resolve()
        files, missing = source_coverage(
            json.loads(args.coverage_json.read_text(encoding="utf-8")), root,
            args.lcov.read_text(encoding="utf-8"),
        )
        covered = sum(value[0] for value in files.values())
        total = sum(value[1] for value in files.values())
        for path, contents in (
            (args.badge, badge(percent(covered, total))),
            (args.report, snapshot(files, missing, args)),
        ):
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(contents, encoding="utf-8", newline="\n")
        print(f"Rust source line coverage: {covered}/{total} ({percent(covered, total)})")
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"coverage-badge: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
