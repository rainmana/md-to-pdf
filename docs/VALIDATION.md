# Validation

## Release validation - 2026-10-25

The published [v6.0.0 release](https://github.com/rainmana/md-to-pdf/releases/tag/v6.0.0)
was built from commit `0eca1048ba9b1021c00db3c96b32fc9bf06dd8aa`.
The [tagged workflow](https://github.com/rainmana/md-to-pdf/actions/runs/40224642560)
passed all five native targets and the publication job:

- Linux x86-64 and ARM64 using MUSL.
- macOS Intel and Apple Silicon.
- Windows x86-64 using MSVC with a statically linked C runtime.

Every target ran tests, built both executable names, and generated PDFs through
both release binaries before packaging. Independent inspection of all five
archives verified executable formats, documentation/license contents, and
checksums against the published `SHA256SUMS` and GitHub asset digests. Linux
binaries had no dynamic interpreter or shared-library dependencies. macOS
binaries used only `/usr/lib/libiconv.2.dylib` and `/usr/lib/libSystem.B.dylib`.
Windows binaries imported system DLLs without a separate Visual C++ runtime.

### Installed release without Node or npm

Both installed Apple Silicon executables were compared byte for byte with their
payloads in the published archive, whose SHA-256 is:

```text
bf5815ba4a296084fc1c361847b426a58c114bee9bfc276328b10140b7ad28fb
```

Each executable was copied to a fresh temporary directory outside the source
repository and run with an empty `PATH` and temporary `HOME`. A macOS sandbox
denied network access, subprocess forks, system/user font reads, known Node/npm
installation and cache paths, browser application paths, and Puppeteer/Playwright
caches. Negative controls confirmed that Node/npm and font reads and subprocess
creation were actually denied by the profile.

Both binaries reported version `6.0.0` and converted the showcase Markdown and
SVG into identical 48,896-byte, two-page PDFs. `qpdf --check` and extracted-text
checks passed, including French accents, Greek, and Cyrillic. The PDF SHA-256 was
`f1b52fc8000a603a8868af824163b8a5df243d6f649a16b1b3da039a47e9c129`.
Only the two standard macOS libraries above were linked.

The reproduction script and full results are retained locally in the ignored
`.analysis/readme-verification/verify-installed-release.py` and
`.analysis/readme-verification/installed-release-validation.json`. `qpdf` and
`pdftotext` were validation tools run after conversion, not converter dependencies.
Node/npm remained installed on the host; their known locations were denied to
the converter processes. This is a bounded runtime check of the Apple Silicon
release and showcase document, alongside hosted tests on the other platforms.
It does not establish a comparative speed advantage or eliminate security risk.

## Local validation - 2026-10-23

The Rust rewrite was validated on macOS ARM64 with Rust 1.98.1. Hosted Linux,
Windows, and Intel macOS builds had not yet run at the time of this local check;
the later release validation above records their successful hosted builds.

### Automated checks

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

### Standalone release check

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

### Document and source inspection

The showcase was validated with `qpdf --check`, `pdfinfo`, and Poppler text
extraction. Both pages were rendered with `pdftoppm` and visually inspected for
layout, image inclusion, code wrapping, footnote placement, and page numbering.
An additional long highlighted-code fixture was checked for clipping.

All 36 original files archived in `legacy/typescript/` were compared byte for
byte with the starting Git commit. The original source's verified Joern graphs,
counts, commands, and parser limitations are documented in [JOERN.md](JOERN.md).
