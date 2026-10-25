---
document_title: Native Markdown to PDF
author: md-to-pdf contributors
page_numbers: true
header: Self-contained Rust document
footer: Offline rendering
pdf_options:
  margin: 22mm 20mm
---

# Markdown, without a browser

This document was generated **entirely in Rust**, with embedded fonts and no network access. It includes *emphasis*, ~~deleted text~~, `inline code`, and a [clickable link](https://www.rust-lang.org/).

## Structured content

1. Parse CommonMark and GitHub-flavored Markdown.
2. Lay out the document using the linked native typesetter.
   - Shape Unicode text with embedded fonts.
   - Export selectable text and link annotations.
3. Write the finished PDF atomically.

- [x] A standalone binary
- [x] Fonts embedded at compile time
- [ ] No browser needed for this unchecked item either

> A native layout engine gives us pagination and typography without starting an HTTP server or a Chromium process.

| Feature | Native behavior | Available offline |
| :--- | :---: | ---: |
| Headings and links | Structured content | Yes |
| Code blocks | Syntax highlighting | Yes |
| Fonts and PDF | Embedded renderer | Yes |

```rust
let config = md_to_pdf::Config::default();
let pdf = md_to_pdf::to_pdf("# Hello, Rust", &config, root)?;
std::fs::write("hello.pdf", pdf)?;
```

Unicode: café, naïve, Ελληνικά, Кириллица, and the symbols α ≠ β.

An ordinary footnote stays near its reference.[^native]

[^native]: This is a real typeset footnote, with **formatted text**.

<div class="page-break"></div>

# A deliberate second page

This heading is the target of an [internal link](#structured-content) to the first page.

The source text `#read("secret.txt")` is displayed literally. Markdown content never becomes executable Typst source.

![A local vector illustration](diagram.svg)

---

The binary and this PDF can travel independently of the source repository.
