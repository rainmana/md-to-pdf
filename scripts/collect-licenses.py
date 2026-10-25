#!/usr/bin/env python3
"""Maintain binary-distribution notices using cached Cargo sources only.

This standard-library-only developer helper is not invoked by the application.
It never builds code or fetches dependencies. Run Cargo's normal dependency
fetch/build first if the locked metadata is not available offline.
"""

import argparse
import json
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "assets" / "licenses"
PREFIXES = ("LICENSE", "LICENCE", "NOTICE", "COPYING", "COPYRIGHT")
LICENSE_DIRS = {"license", "licenses", "licence", "licences"}
SOURCE_DESCRIPTION = (
    "Cargo.lock via cargo metadata --locked --offline; reachable normal "
    "dependencies only; all target conditions retained"
)


def cargo_metadata():
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--offline", "--format-version", "1"],
        cwd=ROOT,
        check=True,
        stdout=subprocess.PIPE,
        text=True,
    )
    return json.loads(result.stdout)


def normal_dependencies(metadata):
    """Retain normal dependency paths, including all target conditions."""
    resolve = metadata["resolve"]
    root = resolve["root"]
    if root is None:
        raise ValueError("expected a root Cargo package, not a virtual workspace")
    nodes = {node["id"]: node for node in resolve["nodes"]}
    packages = {package["id"]: package for package in metadata["packages"]}
    visited = set()
    pending = [root]
    while pending:
        package_id = pending.pop()
        if package_id in visited:
            continue
        visited.add(package_id)
        for dependency in nodes[package_id]["deps"]:
            if any(kind["kind"] is None for kind in dependency["dep_kinds"]):
                pending.append(dependency["pkg"])
    return sorted(
        (packages[package_id] for package_id in visited if package_id != root),
        key=lambda package: (package["name"], package["version"]),
    )


def license_texts(package):
    directory = Path(package["manifest_path"]).parent
    files = set()
    for path in directory.iterdir():
        if path.is_file() and path.name.upper().startswith(PREFIXES):
            files.add(path)
        elif path.is_dir() and path.name.lower() in LICENSE_DIRS:
            files.update(child for child in path.rglob("*") if child.is_file())
    if package.get("license_file"):
        explicit = directory / package["license_file"]
        # Do not collect arbitrary paths outside the cached package directory.
        explicit.resolve().relative_to(directory.resolve())
        if not explicit.is_file():
            raise ValueError(f"declared license file is missing: {explicit}")
        files.add(explicit)
    return directory, sorted(files)


def collect(packages):
    expected = {}
    records = []
    for package in packages:
        directory, files = license_texts(package)
        prefix = Path("crates") / f"{package['name']}-{package['version']}"
        copied = []
        for path in files:
            relative = prefix / path.relative_to(directory)
            expected[relative] = path.read_bytes()
            copied.append(relative.as_posix())
        records.append(
            {
                "name": package["name"],
                "version": package["version"],
                "license": package["license"],
                "repository": package["repository"],
                "source": package["source"],
                "license_files": copied,
            }
        )
    # Keep the embedded asset notices easy to find in every release archive.
    assets = next(package for package in packages if package["name"] == "typst-assets")
    directory = Path(assets["manifest_path"]).parent
    for name in ("NOTICE", "LICENSE"):
        expected[Path(f"typst-assets-{name}.txt")] = (directory / name).read_bytes()
    inventory = {"source": SOURCE_DESCRIPTION, "crates": records}
    expected[Path("crate-inventory.json")] = (
        json.dumps(inventory, indent=2) + "\n"
    ).encode("utf-8")
    return expected, records


def generated_files():
    files = set()
    crates = OUTPUT / "crates"
    if crates.exists():
        files.update(path.relative_to(OUTPUT) for path in crates.rglob("*") if path.is_file())
    for name in ("crate-inventory.json", "typst-assets-NOTICE.txt", "typst-assets-LICENSE.txt"):
        if (OUTPUT / name).exists():
            files.add(Path(name))
    return files


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--check", action="store_true", help="verify generated files without changing them"
    )
    arguments = parser.parse_args()
    expected, records = collect(normal_dependencies(cargo_metadata()))
    existing = generated_files()
    stale = existing - expected.keys()
    changed = {
        path
        for path, data in expected.items()
        if not (OUTPUT / path).is_file() or (OUTPUT / path).read_bytes() != data
    }
    if arguments.check and (stale or changed):
        print(
            f"License inventory is stale: {len(changed)} missing/changed and "
            f"{len(stale)} obsolete files. Run python3 scripts/collect-licenses.py.",
            file=sys.stderr,
        )
        return 1
    if not arguments.check:
        for path in stale:
            (OUTPUT / path).unlink()
        for path in changed:
            destination = OUTPUT / path
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(expected[path])
        crates = OUTPUT / "crates"
        if crates.exists():
            for directory in sorted(
                (path for path in crates.rglob("*") if path.is_dir()),
                key=lambda path: len(path.parts),
                reverse=True,
            ):
                if not any(directory.iterdir()):
                    directory.rmdir()
    texts = sum(len(record["license_files"]) for record in records)
    missing = sum(not record["license_files"] for record in records)
    mode = "Verified" if arguments.check else "Collected"
    print(
        f"{mode} {texts} license/notice files for {len(records)} packages, "
        f"plus 2 embedded-asset notice copies; {missing} packages declare "
        "licenses without standalone notice text."
    )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"License collection failed: {error}", file=sys.stderr)
        sys.exit(1)
