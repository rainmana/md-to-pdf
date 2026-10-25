# Third-party notices

The project's original MIT license is preserved in [license](license).
Third-party libraries and embedded assets retain their respective copyright
notices and license terms.

## Embedded typesetter and fonts

The native PDF renderer links the Typst 0.15.1 Rust libraries. Typst's crate
metadata declares Apache-2.0. The embedded fonts and other asset components are
supplied by `typst-assets` 0.15.1 and carry the individual notices included by
that crate.

These two files are byte-for-byte copies from the locally cached
`typst-assets-0.15.1` crate used for the build:

- [typst-assets-LICENSE.txt](assets/licenses/typst-assets-LICENSE.txt): the crate's
  Apache License 2.0 text.
- [typst-assets-NOTICE.txt](assets/licenses/typst-assets-NOTICE.txt): upstream
  component notices and font license texts, including Libertinus, DejaVu, New
  Computer Modern, and bundled PDF font assets.

Keep these files with redistributed binary archives. Fonts are embedded in the
executable; installing fonts or downloading an external font package is not a
runtime step.

## Rust dependency inventory

[assets/licenses/crate-inventory.json](assets/licenses/crate-inventory.json)
records the name, version, declared license expression, source, repository, and
copied notice paths for packages reachable through normal dependency edges
in the current locked Cargo graph. The inventory retains all target conditions,
so it is a superset for any single release target. Dev-only and build-only
dependency edges were excluded. Metadata was obtained locally with
`cargo metadata --locked --offline --format-version 1`; no license files were
downloaded separately.

The [crates directory](assets/licenses/crates) preserves available license,
copyright, and notice files from those cached crate distributions without
editing their contents. Some crate distributions declare their license in
Cargo metadata but do not include a standalone license/notice file; their
inventory `license_files` arrays are empty. This inventory reports that source
packaging limitation rather than inventing author or copyright notices.

The release workflow includes this notice, the project license, and the entire
`assets/licenses` directory in each binary archive. Refresh the inventory and
copied texts when changing `Cargo.lock` or the embedded asset version.

The standard-library-only Python helper is a developer maintenance tool, not a
dependency of the Rust application or a step required to run release binaries.
It reads the locked Cargo graph offline, copies available upstream texts, removes
obsolete generated files, and records packages with metadata-only declarations:

```sh
python3 scripts/collect-licenses.py
python3 scripts/collect-licenses.py --check
```

Run the normal Cargo dependency fetch or build first if the locked sources are
not yet cached. This helper does not download packages or execute a source build.
