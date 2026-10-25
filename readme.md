![A cheerful orange crab in a bow tie and tiny fancy trousers presents a PDF report.](assets/branding/report-crab.svg)

# md-to-pdf — Rust edition

**Your Markdown, dressed for the debrief.**

[![Rust CI](https://img.shields.io/github/actions/workflow/status/rainmana/md-to-pdf/main.yml?branch=master&style=flat-square&logo=github&label=Rust%20CI)](https://github.com/rainmana/md-to-pdf/actions/workflows/main.yml) [![Latest release](https://img.shields.io/github/v/release/rainmana/md-to-pdf?style=flat-square&color=9D431F)](https://github.com/rainmana/md-to-pdf/releases/latest) [![Hosted Rust line coverage](https://raw.githubusercontent.com/rainmana/md-to-pdf/coverage-data/coverage.svg)](https://github.com/rainmana/md-to-pdf/blob/coverage-data/COVERAGE.md) [![License: MIT](https://img.shields.io/badge/license-MIT-257F80?style=flat-square)](license)

[![GitHub stars](https://img.shields.io/github/stars/rainmana/md-to-pdf?style=social)](https://github.com/rainmana/md-to-pdf/stargazers) [![Release asset downloads](https://img.shields.io/github/downloads/rainmana/md-to-pdf/total?style=flat-square&label=release%20downloads&color=257F80)](https://github.com/rainmana/md-to-pdf/releases) [![Built with Rust](https://img.shields.io/badge/built%20with-Rust-9D431F?style=flat-square&logo=rust)](Cargo.toml) [![Runtime: no Node.js](https://img.shields.io/badge/runtime-no%20Node.js-17364D?style=flat-square)](#runtime-dependencies-and-verification)

**[Get a binary](#install) · [Make a report](#from-notes-to-a-report) · [Why Rust?](#why-i-rewrote-it-in-rust) · [Coverage](docs/COVERAGE.md) · [Credits](#credits-and-license)**

Turn Markdown notes into PDF reports with a native command-line tool. Download
one archive, extract it, and convert locally. **Node.js, npm, and a browser are
not required to install or run the release binaries.**

This is [Alec Akin's](https://github.com/rainmana) Rust rewrite of
[md-to-pdf](https://github.com/simonhaenisch/md-to-pdf), created by
[Simon Hänisch](https://github.com/simonhaenisch) and developed with the original
project's contributors. Their work provided the tool and workflow this fork
builds on. The original MIT copyright notice is preserved in [license](license).

```sh
md-to-pdf notes.md --output report.pdf --page-numbers
```

The stars, downloads, release, and build badges use live repository data.
The coverage badge updates automatically after successful `master` coverage runs.
Click it for the hosted report; [coverage details](docs/COVERAGE.md) explain its scope.

## Why I rewrote it in Rust

I'm a penetration tester. I write my notes, findings, and other information in
Markdown because it is easy to edit, search, and keep under version control.
When I'm ready to publish or share a finished report, I want the more formal,
portable presentation of a PDF.

I already used Simon's `md-to-pdf` for that workflow. My own security concerns
around Node.js/npm, along with the installation overhead of a browser-based
toolchain, led me to rewrite it in Rust. I wanted to download a native executable
and get on with writing the report, without installing Node, npm packages, or
Chromium on the machine doing the conversion.

Rust makes it practical to ship the typesetter and fonts inside the executable.
The result is a portable CLI that converts in a native process, works offline,
and avoids starting a browser for each report. The goal is fast, straightforward
conversion with a simple installation.

The spirit of the original project remains: a useful, approachable Markdown-to-PDF
tool with sensible defaults, command-line options, watch mode, and a library API.
This fork changes the implementation and packaging to suit my reporting workflow.
For the original Node/Puppeteer implementation and its browser styling features,
see [the upstream project](https://github.com/simonhaenisch/md-to-pdf).

## Install

Download the archive for your system from the
[GitHub release page](https://github.com/rainmana/md-to-pdf/releases).
[v6.0.0](https://github.com/rainmana/md-to-pdf/releases/tag/v6.0.0) is available for
all five platforms below:

| System | Architecture | v6.0.0 download |
| --- | --- | --- |
| Linux | x86-64 | [MUSL tar.gz](https://github.com/rainmana/md-to-pdf/releases/download/v6.0.0/md-to-pdf-v6.0.0-x86_64-unknown-linux-musl.tar.gz) |
| Linux | ARM64 | [MUSL tar.gz](https://github.com/rainmana/md-to-pdf/releases/download/v6.0.0/md-to-pdf-v6.0.0-aarch64-unknown-linux-musl.tar.gz) |
| macOS | Intel | [tar.gz](https://github.com/rainmana/md-to-pdf/releases/download/v6.0.0/md-to-pdf-v6.0.0-x86_64-apple-darwin.tar.gz) |
| macOS | Apple Silicon | [tar.gz](https://github.com/rainmana/md-to-pdf/releases/download/v6.0.0/md-to-pdf-v6.0.0-aarch64-apple-darwin.tar.gz) |
| Windows | x86-64 | [ZIP](https://github.com/rainmana/md-to-pdf/releases/download/v6.0.0/md-to-pdf-v6.0.0-x86_64-pc-windows-msvc.zip) |

Extract the archive. Each package includes two equivalent commands, `md-to-pdf`
and `md2pdf`, plus documentation and license notices. On Windows the executables
have the `.exe` extension. You can run either executable directly from its
extracted location, or put it in a directory on your `PATH`.

For example, after extracting the Apple Silicon archive beside `notes.md`:

```sh
./md-to-pdf-v6.0.0-aarch64-apple-darwin/md-to-pdf notes.md --output report.pdf
```

On Windows, from PowerShell after extracting the ZIP beside `notes.md`:

```powershell
.\md-to-pdf-v6.0.0-x86_64-pc-windows-msvc\md-to-pdf.exe notes.md --output report.pdf
```

No npm install, Rust toolchain, separate Typst executable, or font installation
is needed to run these packages. Linux binaries use static MUSL linking. macOS
binaries use standard macOS libraries; Windows binaries use system DLLs and a
statically linked C runtime. Keep the included license notices with redistributed
copies.

The release also provides
[SHA256SUMS](https://github.com/rainmana/md-to-pdf/releases/download/v6.0.0/SHA256SUMS).
For example, in a directory containing the Apple Silicon archive and that file:

```sh
shasum -a 256 md-to-pdf-v6.0.0-aarch64-apple-darwin.tar.gz
```

Compare the result with its entry in `SHA256SUMS`.

## From notes to a report

Once the command is on your `PATH`, the default conversion writes a PDF beside
the Markdown file:

```sh
md-to-pdf notes.md
```

For a report with a title, running header, page numbers, and explicit output:

```sh
md-to-pdf findings.md --output assessment.pdf \
  --document-title 'Security assessment' \
  --header 'Security assessment' --page-numbers --format a4
```

Put reusable presentation settings in a JSON or YAML configuration file:

```yaml
# report.yaml
document_title: Security assessment
author: Example Author
font: Libertinus Serif
code_font: DejaVu Sans Mono
font_size: 11
header: Security assessment
footer: Confidential
page_numbers: true
pdf_options:
  format: a4
  margin: "20mm 15mm"
```

```sh
md-to-pdf findings.md --config-file report.yaml --output assessment.pdf
```

Other common workflows:

```sh
# Use the equivalent command name.
md2pdf notes.md --output report.pdf

# Convert several files, each to its own PDF.
md-to-pdf first.md second.md --page-numbers

# Let the CLI expand a quoted pattern, including on Windows.
md-to-pdf '*.md' --page-numbers

# Read Markdown from stdin and write PDF bytes to stdout.
md-to-pdf - < notes.md > report.pdf

# Rebuild when inputs, local images, or the selected config change.
md-to-pdf notes.md --watch

# Export static HTML with the built-in stylesheet.
md-to-pdf notes.md --as-html
```

Inputs may use `.md`, `.markdown`, `.mdown`, or `.mkd`. With no file arguments,
input is read from stdin. Explicit file arguments take precedence over piped
stdin. `-` selects stdin explicitly and cannot be combined with file inputs.
Output defaults to the input filename with `.pdf` or `.html`; stdin defaults to
stdout. `--output -` also selects stdout. Binary PDF output is refused when
stdout is an interactive terminal. A named output supports one input at a time.

Watch mode polls every 500 ms and requires file inputs and file outputs. It
watches the selected config and supported images under the resource root, to a
maximum of 16 directory levels. Hidden directories, `target`, and `node_modules`
are skipped. Errors are reported and watching continues until another change
or Ctrl-C.

## Markdown, images, and layout

The PDF renderer supports headings, emphasis, strikethrough, ordered and
unordered lists, task lists, block quotes, fenced and indented code, tables with
alignment and repeating headers, footnotes, and links. Recognized code languages
receive Typst's built-in syntax highlighting. Headings have link targets;
`# Heading {#name}` supplies an explicit ID.

Local PNG, JPEG, GIF, WebP, and SVG images can be included with ordinary Markdown:

```markdown
![Evidence diagram](images/diagram.svg)
```

Image paths resolve relative to the input file. The allowed resource root defaults
to that file's directory. To allow `docs/report.md` to reference images elsewhere
inside the project:

```sh
md-to-pdf docs/report.md --basedir .
```

The input and images must stay inside that root, including after symlink
resolution. Remote image URLs, data URIs, and paths outside the root are rejected.
Clickable HTTP, HTTPS, mailto, local, and heading links are supported; rendering
does not visit them.

Fonts are embedded: Libertinus Serif, DejaVu Sans Mono, New Computer Modern, and
New Computer Modern Math. Body text defaults to Libertinus Serif; code defaults
to DejaVu Sans Mono. System fonts are not searched. Coverage is finite, and a
conversion fails if the typesetter reports a missing glyph.

Use `<div class="page-break"></div>` for an explicit page break. The other
supported HTML exceptions are `<br>` and comments. Arbitrary raw HTML is rejected.
Static HTML export includes image references rather than embedding image files.
PDF font, page, margin, header/footer, and page-number settings do not style HTML
export, which uses its built-in CSS.

## Configuration reference

Use `--config-file` to select a data-only JSON or YAML file. Configuration is
merged recursively in this order, with later values taking precedence:

1. Built-in defaults.
2. The explicitly selected configuration file.
3. Document YAML front matter, for presentation settings.
4. Explicit CLI options.

A document can override its title or paper size in front matter:

```markdown
---
document_title: Assessment findings
pdf_options:
  format: letter
---

# Findings
```

Configuration files are never discovered automatically. Unknown options are
errors. JavaScript configuration is unsupported.

| CLI option | Config field | Default or meaning |
| --- | --- | --- |
| `-o`, `--output`, `--dest` | `dest` | Input filename with `.pdf` or `.html`; stdin uses stdout |
| `--basedir` | `basedir` | Allowed resource root: input directory, or current directory for stdin |
| `--document-title` | `document_title` | Optional document title (PDF metadata / HTML title) |
| `--author` | `author` | Optional PDF author metadata |
| `--font` | `font` | `Libertinus Serif` |
| `--code-font` | `code_font` | `DejaVu Sans Mono` |
| `--font-size` | `font_size` | `11` points; range 5–72 |
| `--line-height` | `line_height` | `0.65` em of extra leading; range 0–3 |
| `--header` | `header` | Optional centered plain text header |
| `--footer` | `footer` | Optional centered plain text footer |
| `--page-numbers` | `page_numbers` | `false`; current/total page numbers in the footer |
| `--format` | `pdf_options.format` | `a4`; A0–A6, Letter, Legal, Tabloid, Ledger |
| `--landscape` | `pdf_options.landscape` | `false`; swap page width and height |
| `--margin` | `pdf_options.margin` | Top/right/bottom/left: `30mm 40mm 30mm 20mm` |
| `--pdf-options` | `pdf_options` | JSON object with `format`, `margin`, `landscape`, `width`, `height` |
| `--md-file-encoding` | `md_file_encoding` | `utf-8` |
| `--as-html` | `as_html` | `false`; static HTML output |

`dest`, `basedir`, `as_html`, and `md_file_encoding` may only come from CLI flags
or the explicitly selected config file. The CLI rejects them in document front
matter, even when a CLI override would replace them. The config file is trusted:
it can choose output paths and broaden the resource root, so select it as the
invoking user. Conversion uses that user's filesystem permissions.

`-w`/`--watch`, `--config-file`, `-h`/`--help`, and `-v`/`--version` are CLI controls.
Run `md-to-pdf --help` for the full command-line interface.

Margins accept one to four lengths using CSS shorthand order, or an object with
`top`, `right`, `bottom`, and `left`. Units are `mm`, `cm`, `in`, `pt`, and `px`;
numeric JSON/YAML lengths mean pixels at 96 dpi. Custom `width` and `height` must
be supplied together. Margins must leave at least 36 points of content width and height.
Malformed input and unknown encodings produce errors; use `--md-file-encoding`
to select a supported encoding before the file is decoded.

## Moving from the original version

Version 6 uses the [Typst](https://github.com/typst/typst) Rust libraries to lay
out documents directly. Browser CSS layouts and extensions do not carry over.

| Original browser-based behavior | Rust edition |
| --- | --- |
| Node.js, npm packages, Puppeteer, Chromium installation | Download and extract a native binary |
| Browser CSS, stylesheets, body classes, highlight themes | Native font, spacing, page, margin, header/footer, and page-number controls |
| HTML input and arbitrary inline HTML | Markdown input with the limited HTML exceptions above |
| JavaScript configuration and scripts | Data-only JSON/YAML configuration |
| MathJax, KaTeX, or LaTeX formulas | Math stays plain text |
| Browser launch, navigation, devtools, or media options | Native CLI options; unsupported options are rejected |
| HTML header/footer templates | Plain text headers and footers |
| HTTP images or a local HTTP file server | Local images confined to a resource root |
| System or web fonts | Embedded font families |
| JavaScript API | Rust library API |
| Operational front-matter settings | CLI flags or an explicitly trusted config file |

The archived TypeScript implementation and its documentation remain in
[`legacy/typescript`](legacy/typescript). The original code was indexed with
Joern before the rewrite; provenance and reproduction commands are in
[`docs/JOERN.md`](docs/JOERN.md).

## Runtime dependencies and verification

The release executable includes the Typst 0.15.1 typesetter, PDF exporter, Markdown
parser, and font assets. It runs locally without invoking Node, npm, Chromium,
or a separately installed Typst command. Rendering performs no network downloads.

The installed v6.0.0 Apple Silicon binaries were checked against the published
archive and tested from fresh directories with an empty `PATH` and temporary
`HOME`. A macOS sandbox blocked networking, subprocess creation, system fonts,
and known Node/npm/browser locations. Both `md-to-pdf` and `md2pdf` still rendered
a valid two-page PDF with an SVG image and selectable Unicode text.

The [tagged release workflow](https://github.com/rainmana/md-to-pdf/actions/runs/40224642560)
also passed tests, optimized builds, and both-command PDF smoke conversions on
all five platforms. Package inspection verified static Linux executables,
standard macOS libraries, and Windows system DLL imports without a separate
Visual C++ runtime. See [validation details](docs/VALIDATION.md).

The security motivation is reducing the installation and runtime dependencies I
need to maintain on my reporting machine. Rust crates and embedded font assets
are still third-party code with their own maintenance and supply-chain
considerations. Rust is not a blanket security guarantee.

## Rust library and source builds

For Rust applications, the crate exposes PDF and static HTML conversion:

```rust,no_run
use md_to_pdf::{Config, to_html, to_pdf};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let config = Config::default();
    let pdf = to_pdf("# Report\n\nAssessment findings.", &config, Path::new("."))?;
    std::fs::write("report.pdf", pdf)?;
    std::fs::write("report.html", to_html("# Report", &config)?)?;
    Ok(())
}
```

These functions accept Markdown without front matter and an already resolved
`Config`. `to_pdf` takes the allowed resource root. The public `config` helpers
support front-matter parsing and layered configuration. For input-relative image
paths within a wider root, use
`to_pdf_with_source_path(markdown, &config, resource_root, source_path)`.

Source builds need the pinned Rust toolchain in `rust-toolchain.toml`. Cargo
fetches the third-party crates fixed by `Cargo.lock`; those dependencies and
fonts are compiled into the application. Neither Rust nor Cargo is needed to
run a downloaded release binary.

```sh
cargo build --release --locked
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo test --locked --doc
cargo clippy --locked --all-targets -- -D warnings
```

The binaries are written to `target/release/` (`.exe` on Windows).
[CI](.github/workflows/main.yml) runs tests on Linux, macOS, and Windows.
[The release workflow](.github/workflows/release.yml) packages all five targets,
includes license notices, and publishes archives and checksums after a successful
version-tag build. Manual workflow runs produce artifacts without publishing.

## A little star power

If this helps turn your field notes into finished reports,
[give the report crab a star](https://github.com/rainmana/md-to-pdf).
Tiny trousers. Modest ambitions. Very presentable paperwork.

And if the original tool has helped you too,
[show Simon's project some love](https://github.com/simonhaenisch/md-to-pdf).

## Credits and license

**The original `md-to-pdf` was created by Simon Hänisch.** Thank you to Simon and
[the original project's contributors](https://github.com/simonhaenisch/md-to-pdf/graphs/contributors)
for the work that made this workflow useful in the first place. This Rust fork
is maintained by [Alec Akin](https://github.com/rainmana).

The original project's acknowledgments are also retained:

- [imcvampire](https://github.com/imcvampire) for handing over the npm package name.
- [Sindre Sorhus](https://github.com/sindresorhus) and
  [Vercel](https://github.com/vercel) for inspiration on writing CLI tools.
- [Josh Bruce](https://github.com/joshbruce) for helping revive Marked, the
  original renderer's Markdown parser.

The Rust edition also depends on the work of the Typst and pulldown-cmark
contributors, font designers, and the authors of its other Rust dependencies.
Their notices are included in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md)
and [assets/licenses](assets/licenses).

The [report-crab artwork](assets/branding/report-crab.svg) is original to this
fork and is included under the project's MIT license.

The project retains its [MIT license](license), including Simon Hänisch's
original copyright notice. Third-party components keep their respective licenses.
