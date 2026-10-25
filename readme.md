# md-to-pdf

Convert Markdown into PDF with a self-contained native Rust binary. The PDF
typesetter, syntax support, and fonts are embedded in the executable. A release
binary runs offline without Node.js, Chromium, Puppeteer, a separately installed
Typst executable, or runtime downloads.

Version 6 replaces the original browser-based implementation with the Typst
0.15.1 Rust libraries. It produces native typeset documents rather than matching
a browser's HTML/CSS layout. See the migration table below before reusing an
older configuration.

## Install a binary

When a release is published, download the archive for your operating system and
architecture from [GitHub Releases](https://github.com/rainmana/md-to-pdf/releases).
The configured release workflow produces these targets:

| Operating system | Architecture | Archive target |
| --- | --- | --- |
| Linux | x86-64 | `x86_64-unknown-linux-musl` |
| Linux | ARM64 | `aarch64-unknown-linux-musl` |
| macOS | Intel | `x86_64-apple-darwin` |
| macOS | Apple Silicon | `aarch64-apple-darwin` |
| Windows | x86-64 | `x86_64-pc-windows-msvc` |

Extract the archive and run `md-to-pdf` (`md-to-pdf.exe` on Windows). Put the
executable on your `PATH` if desired. Each archive also includes `md2pdf`, an
equivalent binary name, and license notices. The Linux archives use MUSL; macOS
and Windows binaries use their operating systems' standard libraries.

These are workflow targets, not a claim that GitHub Actions has already built or
published a release from this checkout.

## Use

```sh
# Write notes.pdf next to the Markdown file.
md-to-pdf notes.md

# Choose an output file and paper size.
md-to-pdf notes.md --output report.pdf --format letter

# Convert multiple files, each to its own PDF.
md-to-pdf first.md second.md --page-numbers

# Expand a quoted pattern inside the CLI, including on Windows.
md-to-pdf '*.md' --page-numbers

# Read Markdown from stdin and write PDF bytes to stdout.
md-to-pdf - < notes.md > report.pdf

# Read stdin and write a named file.
printf '# Hello\n\nA native PDF.\n' | md2pdf --output hello.pdf

# Export a static HTML document with built-in CSS.
md-to-pdf notes.md --as-html

# Rebuild when input files, local images, or the config change.
md-to-pdf notes.md --watch
```

File inputs may use `.md`, `.markdown`, `.mdown`, or `.mkd`. A PDF or HTML output
defaults to the input's filename with the corresponding extension. `-o`,
`--output`, and `--dest` select one output path for a single input; `-` selects
stdout. PDF bytes are refused when stdout is an interactive terminal.
With no file arguments, input is read from stdin. Explicit file arguments take
precedence over piped stdin; an explicit `-` selects stdin and cannot be combined
with file inputs. The CLI expands quoted file patterns on every platform.

Watch mode polls every 500 ms and requires file inputs and file outputs. It
watches the specified config and local PNG, JPEG, GIF, WebP, and SVG files under
the resource root, descending at most 16 directory levels. Hidden directories,
`target`, and `node_modules` are skipped. A conversion error is reported and
watching continues until another change or Ctrl-C.

## Markdown and images

The native PDF renderer supports headings, emphasis, strikethrough, ordered and
unordered lists, task lists, block quotes, fenced and indented code blocks,
tables with alignment and repeating headers, footnotes, and links. Fenced code
uses Typst's built-in syntax highlighting when its language is recognized.
Headings receive link targets; an explicit heading ID uses `# Heading {#name}`.
Link text can point to `http`, `https`, `mailto`, relative paths, or heading
anchors. Rendering does not follow those links or make network requests.

Use local image paths such as `![Diagram](images/diagram.png)`. PDF images are
resolved relative to the Markdown file's directory. The allowed resource root
defaults to that directory; `--basedir` can select a wider root without changing
how relative image paths resolve. Both the input file and its images must remain
inside that resource root, including after symlink resolution.
Remote URLs, data URIs, and paths outside the root are rejected. For example:

```sh
# Allow docs/report.md to reference ../images/diagram.png within the project.
md-to-pdf docs/report.md --basedir .
```

System fonts are not searched. The embedded families are Libertinus Serif,
DejaVu Sans Mono, New Computer Modern, and New Computer Modern Math. The defaults
are Libertinus Serif for body text and DejaVu Sans Mono for code. Font coverage
is finite; a PDF conversion fails if the typesetter reports a missing glyph.

Math expressions remain plain text: this tool does not render LaTeX, MathJax, or
KaTeX. Arbitrary raw HTML is rejected. The supported exceptions are `<br>`, HTML
comments, and the explicit page break `<div class="page-break"></div>`.
Static HTML export includes the built-in stylesheet and image references; image
files remain external to that HTML document.
Font, page, margin, running header/footer, and page-number controls apply to PDF;
HTML export uses its built-in static CSS.

## Configuration

Pass a data-only JSON or YAML file with `--config-file config.yaml`. Markdown
may also start with YAML front matter for presentation options. Allowed options
merge recursively in this order, with later values taking precedence:

1. Built-in defaults.
2. The explicit config file.
3. The document's YAML front matter.
4. Explicit CLI options.

The tool does not automatically discover config files. Unknown options are
errors. Executable JavaScript configuration is unsupported.

Document front matter cannot set `dest`, `basedir`, `as_html`, or
`md_file_encoding`. The CLI rejects these keys even if a later CLI option would
override them. Select output paths, filesystem scope, PDF/HTML selection, and the
input decoder through CLI flags or an explicitly selected config file.
`--config-file` treats that local file as trusted configuration; choose and
review it as the invoking user, since it can direct output writes and broaden
the allowed resource root. The local CLI operates with the invoking user's
filesystem permissions.

```yaml
# config.yaml
document_title: Engineering notes
author: Example Author
font: Libertinus Serif
code_font: DejaVu Sans Mono
font_size: 11
line_height: 0.65
header: Engineering notes
footer: Internal draft
page_numbers: true
pdf_options:
  format: a4
  landscape: false
  margin: "20mm 15mm"
```

```markdown
---
document_title: A document-specific title
pdf_options:
  format: letter
---

# Engineering notes
```

```sh
md-to-pdf notes.md --config-file config.yaml --font-size 12
md-to-pdf notes.md --pdf-options '{"width":"150mm","height":"210mm","margin":"15mm"}'
```

| CLI option | Config field | Default or meaning |
| --- | --- | --- |
| `-o`, `--output`, `--dest` | `dest` | Input filename with `.pdf` or `.html`; stdin uses stdout |
| `--basedir` | `basedir` | Allowed resource root: Markdown file's directory; current directory for stdin |
| `--document-title` | `document_title` | Optional document metadata/title |
| `--author` | `author` | Optional PDF author metadata |
| `--font` | `font` | `Libertinus Serif` |
| `--code-font` | `code_font` | `DejaVu Sans Mono` |
| `--font-size` | `font_size` | `11` points, allowed range 5–72 |
| `--line-height` | `line_height` | `0.65` em of additional leading, allowed range 0–3 |
| `--header` | `header` | Optional centered plain text running header |
| `--footer` | `footer` | Optional centered plain text running footer |
| `--page-numbers` | `page_numbers` | `false`; adds current/total page numbers to the footer |
| `--format` | `pdf_options.format` | `a4`; supports A0–A6, Letter, Legal, Tabloid, Ledger |
| `--landscape` | `pdf_options.landscape` | `false`; swaps page width and height |
| `--margin` | `pdf_options.margin` | Top/right/bottom/left: `30mm 40mm 30mm 20mm` |
| `--pdf-options` | `pdf_options` | JSON object with `format`, `margin`, `landscape`, `width`, `height` |
| `--md-file-encoding` | `md_file_encoding` | `utf-8` |
| `--as-html` | `as_html` | `false`; emit static HTML instead of PDF |

The `dest`, `basedir`, `md_file_encoding`, and `as_html` rows are CLI/config-file
options only. The remaining document fields may be supplied in YAML front
matter.

`-w`/`--watch`, `--config-file`, `-h`/`--help`, and `-v`/`--version` are CLI
controls rather than document config fields. Run `md-to-pdf --help` for the
complete command-line help.

Margin shorthand accepts one to four lengths in top/right/bottom/left order,
using the usual CSS shorthand expansion. Config files can instead supply a
`margin` object with `top`, `right`, `bottom`, and `left` fields. Length strings
accept `mm`, `cm`, `in`, `pt`, or `px`; numeric JSON/YAML lengths mean pixels at
96 pixels per inch. Custom `width` and `height` must be supplied together.
Margins must leave at least 36 points of content width and height.

File and stdin input default to UTF-8. Select another supported encoding before
decoding, for example `--md-file-encoding windows-1252`. The encoding belongs in
the CLI or config file, never front matter, because decoding precedes front
matter parsing. Malformed input and unknown encoding labels produce errors.

## Migrate from the TypeScript version

| Earlier browser behavior | Native replacement or limit |
| --- | --- |
| Node.js, Puppeteer, Chromium installation | Download and extract a native binary |
| Browser CSS, `stylesheet`, `css`, body classes, highlight themes | Native `font`, `code_font`, `font_size`, `line_height`, page size, margins, headers, and footers |
| HTML input or arbitrary raw HTML within Markdown | Markdown input; raw HTML is limited to the exceptions listed above |
| JavaScript config, scripts, MathJax/KaTeX | Data-only JSON/YAML config; math remains plain text |
| Front-matter output destination, resource root, PDF/HTML selection, or input encoding | Use `--output`, `--basedir`, `--as-html`, `--md-file-encoding`, or a trusted explicit config file; these front-matter keys are rejected |
| Puppeteer launch, navigation, devtools, media, or browser PDF settings | Native options only; unknown flags/fields are rejected |
| HTML header/footer templates | Plain text `header`, `footer`, and `page_numbers` |
| HTTP image loading or local file server | Local images confined to the resource root |
| Fonts installed on the machine or fetched from the web | Embedded font families only |
| JavaScript API | Rust `Config`, `to_pdf`, and `to_html` |

The original implementation and its documentation are preserved under
[`legacy/typescript`](legacy/typescript) for reference. They do not participate
in the Rust build or run when a native binary is invoked. The original source
was indexed with the locally installed Joern before being archived; see
[`docs/JOERN.md`](docs/JOERN.md) for evidence and reproduction commands.

## Rust library

```rust,no_run
use md_to_pdf::{Config, to_html, to_pdf};
use std::path::Path;

fn main() -> anyhow::Result<()> {
    let config = Config::default();
    let pdf = to_pdf("# Hello\n\nNative Markdown rendering.", &config, Path::new("."))?;
    std::fs::write("hello.pdf", pdf)?;

    let html = to_html("# Hello", &config)?;
    std::fs::write("hello.html", html)?;
    Ok(())
}
```

Library functions accept Markdown without front matter and an already resolved
`Config`. `to_pdf` also takes the allowed resource root. Applications that need
CLI-style configuration can call the public helpers in `md_to_pdf::config` to
parse front matter and merge config layers.
For an existing Markdown file whose image paths must stay relative to that file
inside a wider root, call
`to_pdf_with_source_path(markdown, &config, resource_root, source_path)`.

## Build and verify

The pinned Rust toolchain is in `rust-toolchain.toml`. Cargo fetches third-party
Rust crates during a source build; `Cargo.lock` fixes their resolved versions.
Those crates and font assets are compiled into the release binary rather than
downloaded at runtime.

```sh
cargo build --release --locked
cargo fmt --all -- --check
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
```

The binaries are written to `target/release/` (with `.exe` on Windows).
[`Rust CI`](.github/workflows/main.yml) configures Linux, macOS, and Windows
tests, formatting/Clippy checks, and standalone conversion smoke tests.
[`Release binaries`](.github/workflows/release.yml) configures tests, release
builds, archive packaging, and checksums for the five targets above. A pushed
version tag must match the Cargo version, currently `v6.0.0`; successful tagged
runs publish a GitHub Release. A manual run builds downloadable workflow
artifacts without publishing a release.

## License

The project retains its [MIT license](license). Embedded fonts and Rust
dependencies retain their respective licenses; see
[THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) and [assets/licenses](assets/licenses).
