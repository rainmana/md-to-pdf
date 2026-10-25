# Local validation - 2026-10-23

The Rust rewrite was validated on macOS ARM64 with Rust 1.98.1. Hosted Linux,
Windows, and Intel macOS builds have not been run from this checkout.

## Automated checks

The following commands passed:

```sh
cargo test --locked --offline --all-targets
cargo test --locked --offline --doc
cargo fmt --all -- --check
cargo clippy --locked --offline --all-targets -- -D warnings
cargo build --release --locked --offline --bins
python3 scripts/collect-licenses.py --check
git diff --check
```

There are 35 unit/integration tests and one compiling documentation example.
Tests parse real PDFs and check selectable Unicode text, embedded font programs,
A4 dimensions, link annotations, explicit page breaks, native image paint
operations, same-path SVG edits, repeated table headers across a 110-row table,
and preservation of every table row. CLI process tests cover both executable
names, stdin/stdout, config precedence, encodings, globs, nested images, source
and config overwrite protection, duplicate output paths, and unsupported options.
Watch tests verify hidden image edits, missing-image recovery after changing the
resource root, and case-insensitive output collisions.

Document front matter is restricted to presentation options. The CLI rejects
`dest`, `basedir`, `as_html`, and `md_file_encoding` before configuration merging
and output planning, including null values and YAML-escaped key names. Explicit
CLI options cannot mask a prohibited front-matter key. Those operational settings
remain available through CLI flags and an explicitly selected, trusted config
file. Regression checks preserve existing destination files when documents try
to select a different output path or broaden image access.

Actionlint 1.7.12 passed for both workflows with only its two outdated runner-label
diagnostics suppressed for the generally available Ubuntu 26.04/26.04 ARM runners.
Workflow YAML, shell scripts, toolchain/Cargo TOML, pinned action references, and
tar/zip packaging paths were also checked locally. These checks do not establish
that the five hosted target builds pass.

## Standalone release check

Both optimized executables were copied into a fresh temporary directory outside
the repository, alongside the showcase Markdown and its SVG image. Each ran with
an empty `PATH`, a temporary `HOME`, and this macOS sandbox policy (the user font
path was also denied):

```scheme
(version 1)
(allow default)
(deny network*)
(deny process-fork)
(deny file-read*
  (subpath "/System/Library/Fonts")
  (subpath "/Library/Fonts"))
```

Both returned success and produced identical, 48,896-byte PDFs containing two A4
pages, the SVG illustration, formatted code, Unicode text, links, a footnote,
running header/footer, and page numbers. The output SHA-256 was:

```text
f1b52fc8000a603a8868af824163b8a5df243d6f649a16b1b3da039a47e9c129
```

The local test harness and full results are retained in the ignored
`.analysis/verify-standalone.py` and `.analysis/standalone-results.json`.
`otool -L` reported only `/usr/lib/libiconv.2.dylib` and
`/usr/lib/libSystem.B.dylib`, both standard macOS libraries.

| Executable | Bytes | SHA-256 |
| --- | ---: | --- |
| `md-to-pdf` | 42,167,792 | `9ea499dda1f6883757b195b3716320955daa1bbd5b36dea56fb588b4dda7e196` |
| `md2pdf` | 42,167,776 | `fcfa4a71841757d290310a03e01d4766ad2f96d7c0db67a70eb21d449d3e5d18` |

Hashes identify this local build, not a guarantee of reproducible binaries across
different operating systems or toolchains.

## Document and source inspection

The showcase was validated with `qpdf --check`, `pdfinfo`, and Poppler text
extraction. Both pages were rendered with `pdftoppm` and visually inspected for
layout, image inclusion, code wrapping, footnote placement, and page numbering.
An additional long highlighted-code fixture was checked for clipping.

All 36 original files archived in `legacy/typescript/` were compared byte for
byte with the starting Git commit. The original source's verified Joern graphs,
counts, commands, and parser limitations are documented in [JOERN.md](JOERN.md).
