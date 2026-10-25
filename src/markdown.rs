//! Markdown conversion without interpreting document text as Typst source.

use anyhow::{Context, Result, anyhow, bail};
use percent_encoding::percent_decode_str;
use pulldown_cmark::{Alignment, CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use std::collections::{HashMap, HashSet};

fn options() -> Options {
    Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_GFM
}

#[derive(Clone)]
enum Node<'a> {
    Element(Tag<'a>, Vec<Node<'a>>),
    Leaf(Event<'a>),
}

fn tree<'a>(events: &mut impl Iterator<Item = Event<'a>>, depth: usize) -> Result<Vec<Node<'a>>> {
    if depth > 128 {
        bail!("Markdown nesting exceeds the supported depth of 128");
    }
    let mut nodes = Vec::new();
    while let Some(event) = events.next() {
        match event {
            Event::Start(tag) => nodes.push(Node::Element(tag, tree(events, depth + 1)?)),
            Event::End(_) => break,
            event => nodes.push(Node::Leaf(event)),
        }
    }
    Ok(nodes)
}

/// Produce safe Typst markup. Every document-supplied string is a string literal.
pub fn to_typst(markdown: &str) -> Result<String> {
    let nodes = tree(&mut Parser::new_ext(markdown, options()), 0)?;
    let mut renderer = Renderer::default();
    renderer.collect_metadata(&nodes)?;
    renderer.render(&nodes)
}

/// Produce an HTML fragment. Browser scripts and arbitrary raw HTML are rejected.
pub fn html(markdown: &str) -> Result<String> {
    let mut ids = HashMap::new();
    let mut events = Vec::new();
    let parsed: Vec<_> = Parser::new_ext(markdown, options()).collect();
    // Explicit identifiers take priority even when their heading occurs later.
    for event in &parsed {
        if let Event::Start(Tag::Heading { id: Some(id), .. }) = event
            && ids.insert(id.to_string(), 1).is_some()
        {
            bail!("duplicate heading identifier {id:?}");
        }
    }
    let mut skip_until = 0;
    for (index, event) in parsed.iter().enumerate() {
        if index < skip_until {
            continue;
        }
        if matches!(event, Event::Start(Tag::HtmlBlock)) {
            let end = parsed[index + 1..]
                .iter()
                .position(|event| matches!(event, Event::End(TagEnd::HtmlBlock)))
                .map(|offset| index + 1 + offset)
                .ok_or_else(|| anyhow!("unclosed HTML block"))?;
            let raw: String = parsed[index + 1..end]
                .iter()
                .filter_map(|event| match event {
                    Event::Html(raw) => Some(raw.as_ref()),
                    _ => None,
                })
                .collect();
            events.push(Event::Html(html_replacement(&raw)?.into()));
            skip_until = end + 1;
            continue;
        }
        let mut event = event.clone();
        match &mut event {
            Event::Start(Tag::Heading {
                id, classes, attrs, ..
            }) => {
                if !classes.is_empty() || !attrs.is_empty() {
                    bail!(
                        "heading classes and custom attributes are unsupported; use a heading identifier {{#name}} only"
                    );
                }
                if id.is_none() {
                    let content = inline_plaintext(&parsed[index + 1..]);
                    *id = Some(CowStr::Boxed(
                        unique_slug(&content, &mut ids).into_boxed_str(),
                    ));
                }
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                *dest_url = CowStr::Boxed(image_path(dest_url)?.into_boxed_str());
            }
            Event::Start(Tag::Link { dest_url, .. }) => validate_link(dest_url)?,
            Event::Html(raw) | Event::InlineHtml(raw) => {
                *raw = html_replacement(raw)?.into();
            }
            _ => {}
        }
        events.push(event);
    }
    let mut output = String::new();
    pulldown_cmark::html::push_html(&mut output, events.into_iter());
    Ok(output)
}

fn inline_plaintext(events: &[Event<'_>]) -> String {
    let mut depth = 0;
    let mut text = String::new();
    for event in events {
        match event {
            Event::Start(_) => depth += 1,
            Event::End(_) if depth == 0 => break,
            Event::End(_) => depth -= 1,
            Event::Text(value) | Event::Code(value) => text.push_str(value),
            Event::SoftBreak | Event::HardBreak => text.push(' '),
            _ => {}
        }
    }
    text
}

fn plaintext(nodes: &[Node<'_>]) -> String {
    let mut text = String::new();
    for node in nodes {
        match node {
            Node::Element(_, children) => text.push_str(&plaintext(children)),
            Node::Leaf(Event::Text(value) | Event::Code(value)) => text.push_str(value),
            Node::Leaf(Event::SoftBreak | Event::HardBreak) => text.push(' '),
            _ => {}
        }
    }
    text
}

fn unique_slug(text: &str, used: &mut HashMap<String, usize>) -> String {
    let base: String = text
        .to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_whitespace() {
                Some('-')
            } else if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else {
                None
            }
        })
        .collect();
    let mut slug = base.clone();
    let mut suffix = 0;
    while used.contains_key(&slug) {
        suffix += 1;
        slug = format!("{base}-{suffix}");
    }
    used.insert(slug.clone(), 1);
    slug
}

/// JSON supplies the escaping; Typst uses braced Unicode escapes for controls.
pub(crate) fn literal(value: &str) -> String {
    let json = serde_json::to_string(value).expect("serializing a string cannot fail");
    let mut chars = json.chars();
    let mut escaped = String::with_capacity(json.len());
    while let Some(c) = chars.next() {
        if c != '\\' {
            escaped.push(c);
            continue;
        }
        let next = chars.next().expect("JSON escape is complete");
        match next {
            'u' => {
                let hex: String = chars.by_ref().take(4).collect();
                escaped.push_str(&format!("\\u{{{hex}}}"));
            }
            'b' => escaped.push_str("\\u{8}"),
            'f' => escaped.push_str("\\u{c}"),
            other => {
                escaped.push('\\');
                escaped.push(other);
            }
        }
    }
    escaped
}

fn text(value: &str) -> String {
    format!("#text({})", literal(value))
}

/// Decode before validation so percent-encoded schemes cannot bypass checks.
pub(crate) fn image_path(value: &str) -> Result<String> {
    let decoded = percent_decode_str(value)
        .decode_utf8()
        .context("image path is not valid UTF-8")?;
    validate_image(&decoded)?;
    Ok(decoded.into_owned())
}

fn validate_image(value: &str) -> Result<()> {
    if value.is_empty()
        || value.chars().any(char::is_control)
        || value.contains(':')
        || value.starts_with("//")
        || value.starts_with('\\')
        || value.contains('?')
        || value.contains('#')
    {
        bail!(
            "image {value:?} must be a local file path; URLs, data URIs, and remote resources are unsupported"
        );
    }
    Ok(())
}

fn validate_link(value: &str) -> Result<()> {
    let decoded = percent_decode_str(value).decode_utf8_lossy();
    if decoded.chars().any(char::is_control) {
        bail!("link contains a control character");
    }
    if value.starts_with('#') {
        return Ok(());
    }
    if let Some((scheme, _)) = decoded.trim_start().split_once(':') {
        // A colon in a path or fragment does not introduce a URI scheme.
        let is_scheme = scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
        if is_scheme
            && !matches!(
                scheme.to_ascii_lowercase().as_str(),
                "http" | "https" | "mailto"
            )
        {
            bail!(
                "unsupported link scheme {scheme:?}; use http, https, mailto, an anchor, or a relative path"
            );
        }
    }
    Ok(())
}

enum HtmlKind {
    Break,
    PageBreak,
    Comment,
}

fn html_replacement(raw: &str) -> Result<&'static str> {
    Ok(match html_kind(raw)? {
        HtmlKind::Break => "<br>",
        HtmlKind::PageBreak => "<div class=\"page-break\"></div>",
        HtmlKind::Comment => "",
    })
}

fn html_kind(raw: &str) -> Result<HtmlKind> {
    let raw = raw.trim();
    if matches!(
        raw.to_ascii_lowercase().as_str(),
        "<br>" | "<br/>" | "<br />"
    ) {
        return Ok(HtmlKind::Break);
    }
    if matches!(
        raw,
        "<div class=\"page-break\"></div>" | "<div class='page-break'></div>"
    ) {
        return Ok(HtmlKind::PageBreak);
    }
    if raw.len() >= 7
        && raw.starts_with("<!--")
        && raw.ends_with("-->")
        && !raw[4..raw.len() - 3].contains("-->")
    {
        return Ok(HtmlKind::Comment);
    }
    bail!(
        "raw HTML is unsupported by the native renderer: {raw:?}. Use Markdown; only <br>, HTML comments, and an empty <div class=\"page-break\"></div> are accepted"
    )
}

#[derive(Default)]
struct Renderer<'a> {
    heading_ids: HashMap<String, usize>,
    footnotes: HashMap<String, Vec<Node<'a>>>,
    emitted_footnotes: HashMap<String, usize>,
    active_footnotes: HashSet<String>,
}

impl<'a> Renderer<'a> {
    fn collect_metadata(&mut self, nodes: &[Node<'a>]) -> Result<()> {
        for node in nodes {
            if let Node::Element(tag, children) = node {
                if let Tag::FootnoteDefinition(name) = tag
                    && self
                        .footnotes
                        .insert(name.to_string(), children.clone())
                        .is_some()
                {
                    bail!("duplicate footnote definition {name:?}");
                }
                if let Tag::Heading { id: Some(id), .. } = tag
                    && self.heading_ids.insert(id.to_string(), 1).is_some()
                {
                    bail!("duplicate heading identifier {id:?}");
                }
                self.collect_metadata(children)?;
            }
        }
        Ok(())
    }

    fn render(&mut self, nodes: &[Node<'a>]) -> Result<String> {
        let mut output = String::new();
        for node in nodes {
            output.push_str(&match node {
                Node::Element(tag, children) => self.element(tag, children)?,
                Node::Leaf(event) => self.leaf(event)?,
            });
        }
        Ok(output)
    }

    fn element(&mut self, tag: &Tag<'a>, children: &[Node<'a>]) -> Result<String> {
        match tag {
            Tag::FootnoteDefinition(_) => Ok(String::new()),
            Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => {
                if !classes.is_empty() || !attrs.is_empty() {
                    bail!(
                        "heading classes and custom attributes are unsupported; use a heading identifier {{#name}} only"
                    );
                }
                let slug = match id {
                    Some(id) => id.to_string(),
                    None => unique_slug(&plaintext(children), &mut self.heading_ids),
                };
                Ok(format!(
                    "\n#heading(level: {})[{}]#label({})\n",
                    *level as u8,
                    self.render(children)?,
                    literal(&format!("md-heading-{slug}"))
                ))
            }
            // Typst images are block elements. A par constructor retains only
            // inline content, so use a markup paragraph boundary around flow.
            Tag::Paragraph => Ok(format!("{}\n\n", self.render(children)?)),
            Tag::Emphasis => Ok(format!("#emph[{}]", self.render(children)?)),
            Tag::Strong => Ok(format!("#strong[{}]", self.render(children)?)),
            Tag::Strikethrough => Ok(format!("#strike[{}]", self.render(children)?)),
            Tag::BlockQuote(kind) => {
                let prefix = kind.map_or_else(String::new, |kind| {
                    format!("#strong[{}]#linebreak()", text(&format!("{kind:?}")))
                });
                Ok(format!(
                    "\n#quote(block: true, quotes: false)[{prefix}{}]\n",
                    self.render(children)?
                ))
            }
            Tag::CodeBlock(kind) => {
                let lang = match kind {
                    CodeBlockKind::Fenced(info) => info.split_whitespace().next().unwrap_or(""),
                    CodeBlockKind::Indented => "",
                };
                let mut code = plaintext(children);
                if code.ends_with('\n') {
                    code.pop();
                }
                Ok(format!(
                    "\n#raw({}, block: true, lang: {})\n",
                    literal(&code),
                    literal(lang)
                ))
            }
            Tag::List(start) => {
                let mut items = Vec::new();
                for node in children {
                    if let Node::Element(Tag::Item, content) = node {
                        items.push(format!("[{}]", self.render(content)?.trim()));
                    } else {
                        bail!("unexpected Markdown list structure");
                    }
                }
                let function = if let Some(start) = start {
                    format!("enum(start: {start},")
                } else {
                    "list(".to_string()
                };
                Ok(format!("\n#{function}{})\n", items.join(",\n")))
            }
            Tag::Item => self.render(children),
            Tag::Table(alignments) => self.table(alignments, children),
            Tag::Link { dest_url, .. } => {
                validate_link(dest_url)?;
                let target = if let Some(anchor) = dest_url.strip_prefix('#') {
                    let decoded = percent_decode_str(anchor).decode_utf8()?;
                    format!("label({})", literal(&format!("md-heading-{decoded}")))
                } else {
                    literal(dest_url)
                };
                Ok(format!("#link({target})[{}]", self.render(children)?))
            }
            Tag::Image { dest_url, .. } => {
                let path = image_path(dest_url)?;
                Ok(format!(
                    "#image({}, width: 100%, alt: {})",
                    literal(&path),
                    literal(&plaintext(children))
                ))
            }
            Tag::HtmlBlock => {
                // A raw block may be split by line; validate it as one unit.
                let raw: String = children
                    .iter()
                    .filter_map(|node| match node {
                        Node::Leaf(Event::Html(value) | Event::InlineHtml(value)) => {
                            Some(value.as_ref())
                        }
                        _ => None,
                    })
                    .collect();
                Self::raw_html(&raw)
            }
            _ => bail!("unsupported Markdown structure {tag:?}"),
        }
    }

    fn table(&mut self, alignments: &[Alignment], rows: &[Node<'a>]) -> Result<String> {
        let alignments: Vec<_> = alignments
            .iter()
            .map(|alignment| match alignment {
                Alignment::None | Alignment::Left => "left",
                Alignment::Center => "center",
                Alignment::Right => "right",
            })
            .collect();
        let mut output = format!(
            "\n#table(columns: {}, align: ({}), inset: 6pt, stroke: 0.5pt + luma(80%),\n",
            alignments.len(),
            alignments.join(", ") + ","
        );
        for row in rows {
            let Node::Element(tag @ (Tag::TableHead | Tag::TableRow), cells) = row else {
                bail!("unexpected Markdown table structure");
            };
            if matches!(tag, Tag::TableHead) {
                output.push_str("table.header(repeat: true,\n");
            }
            for cell in cells {
                let Node::Element(Tag::TableCell, content) = cell else {
                    bail!("unexpected Markdown table cell structure");
                };
                let body = self.render(content)?;
                if matches!(tag, Tag::TableHead) {
                    output.push_str(&format!("[#strong[{body}]],\n"));
                } else {
                    output.push_str(&format!("[{body}],\n"));
                }
            }
            if matches!(tag, Tag::TableHead) {
                output.push_str("),\n");
            }
        }
        output.push_str(")\n");
        Ok(output)
    }

    fn leaf(&mut self, event: &Event<'a>) -> Result<String> {
        match event {
            Event::Text(value) => Ok(text(value)),
            Event::Code(value) => Ok(format!("#raw({})", literal(value))),
            Event::SoftBreak => Ok(text(" ")),
            Event::HardBreak => Ok("#linebreak()".into()),
            Event::Rule => Ok("\n#line(length: 100%, stroke: 0.5pt + luma(70%))\n".into()),
            Event::TaskListMarker(checked) => Ok(text(if *checked { "[x] " } else { "[ ] " })),
            Event::Html(raw) | Event::InlineHtml(raw) => Self::raw_html(raw),
            Event::FootnoteReference(name) => self.footnote(name),
            _ => bail!("unsupported Markdown event {event:?}"),
        }
    }

    fn raw_html(raw: &str) -> Result<String> {
        Ok(match html_kind(raw)? {
            HtmlKind::Break => "#linebreak()".into(),
            HtmlKind::PageBreak => "\n#pagebreak()\n".into(),
            HtmlKind::Comment => String::new(),
        })
    }

    fn footnote(&mut self, name: &str) -> Result<String> {
        if self.active_footnotes.contains(name) {
            bail!("cyclic footnote reference {name:?}");
        }
        if let Some(index) = self.emitted_footnotes.get(name) {
            return Ok(format!(
                "#footnote(label({}))",
                literal(&format!("md-footnote-{index}"))
            ));
        }
        let content = self
            .footnotes
            .get(name)
            .cloned()
            .ok_or_else(|| anyhow!("undefined footnote {name:?}"))?;
        let index = self.emitted_footnotes.len();
        self.emitted_footnotes.insert(name.into(), index);
        self.active_footnotes.insert(name.into());
        let body = self.render(&content)?;
        self.active_footnotes.remove(name);
        Ok(format!(
            "#footnote[{}]#label({})",
            body.trim(),
            literal(&format!("md-footnote-{index}"))
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_text_never_becomes_typst_code() {
        let result = to_typst("# Title\n\n#include(\"secret\") [x] $x$ \\u{41}").unwrap();
        assert!(
            result.contains("#text(\"#include(\\\"secret\\\")"),
            "{result}"
        );
        assert!(!result.contains("\n#include(\"secret\")"));
        assert!(literal("\0\u{8}\u{c}\\u0000").contains("\\u{0000}\\u{8}\\u{c}\\\\u0000"));
    }

    #[test]
    fn table_has_alignment_and_repeating_header() {
        let result = to_typst("| Left | Right |\n| :--- | ---: |\n| a | **b** |\n").unwrap();
        assert!(result.contains("columns: 2, align: (left, right,)"));
        assert!(result.contains("table.header(repeat: true,"));
        assert!(result.contains("#strong[#text(\"b\")]"));
    }

    #[test]
    fn image_paths_reject_network_and_encoded_schemes() {
        for path in [
            "https://example.com/a.png",
            "data:image/png;base64,xx",
            "//host/a.png",
            "https%3A%2F%2Fhost/a.png",
        ] {
            assert!(to_typst(&format!("![alt]({path})")).is_err(), "{path}");
        }
        assert!(
            to_typst("![alt](images/local%20file.svg)")
                .unwrap()
                .contains("#image(\"images/local file.svg\"")
        );
    }

    #[test]
    fn duplicate_headings_anchors_and_explicit_ids() {
        let result =
            to_typst("# A *Heading*\n\n# A Heading\n\n# Custom {#mine}\n\n[go](#a-heading-1)")
                .unwrap();
        assert!(result.contains("#label(\"md-heading-a-heading\")"));
        assert!(result.contains("#label(\"md-heading-a-heading-1\")"));
        assert!(result.contains("#label(\"md-heading-mine\")"));
        assert!(result.contains("#link(label(\"md-heading-a-heading-1\"))"));
    }

    #[test]
    fn raw_html_fails_with_actionable_error() {
        assert!(
            to_typst("<script>alert(1)</script>")
                .unwrap_err()
                .to_string()
                .contains("raw HTML is unsupported")
        );
        assert_eq!(
            to_typst("<div class=\"page-break\"></div>").unwrap().trim(),
            "#pagebreak()"
        );
        assert!(to_typst("a<br>b").unwrap().contains("#linebreak()"));
        assert!(html("<script>alert(1)</script>").is_err());
        assert_eq!(html("<!-- multi\nline comment -->\n").unwrap(), "");
    }

    #[test]
    fn footnotes_are_collected_before_references() {
        let result = to_typst("Text[^a] again[^a].\n\n[^a]: A **note**.\n").unwrap();
        assert_eq!(result.matches("#footnote[").count(), 1);
        assert!(result.contains("#footnote(label(\"md-footnote-0\"))"));
        assert!(result.contains("#strong[#text(\"note\")]"));
    }

    #[test]
    fn native_pdf_renders_structured_markdown() {
        let source = "# Native heading\n\nA **bold** and *italic* [link](https://example.com), [anchor](#native-heading), and footnote[^note].\n\n3. Three\n   - Nested\n4. Four\n\n- [x] Checked\n- [ ] Pending\n\n> A quoted paragraph.\n\n| Left | Right |\n| :--- | ---: |\n| one | two |\n\n```rust\nlet value = 42;\n```\n\n[^note]: A footnote.\n\n<div class=\"page-break\"></div>\n\n# Second page\n";
        let bytes =
            crate::to_pdf(source, &crate::Config::default(), std::path::Path::new(".")).unwrap();
        let document = lopdf::Document::load_mem(&bytes).unwrap();
        assert_eq!(document.get_pages().len(), 2);
        let extracted = document.extract_text(&[1, 2]).unwrap();
        for expected in [
            "Native heading",
            "Nested",
            "Checked",
            "Left",
            "footnote",
            "Second page",
        ] {
            assert!(
                extracted.contains(expected),
                "missing {expected:?}: {extracted}"
            );
        }
    }

    #[test]
    fn native_pdf_keeps_typst_expressions_literal() {
        let source = "#include(\"secret\") and \\u{41} [literal] $literal$\n\nControl characters: \0\u{8}\u{c}.";
        let bytes =
            crate::to_pdf(source, &crate::Config::default(), std::path::Path::new(".")).unwrap();
        let document = lopdf::Document::load_mem(&bytes).unwrap();
        let extracted = document.extract_text(&[1]).unwrap();
        assert!(extracted.contains("#include(\"secret\")"), "{extracted}");
        assert!(extracted.contains("$literal$"), "{extracted}");
    }

    #[test]
    fn explicit_heading_identifiers_are_reserved_before_automatic_slugs() {
        for source in [
            "# Name\n\n# Custom {#name}\n\n# Name\n",
            "# Custom {#name}\n\n# Name\n\n# Name\n",
        ] {
            let typst = to_typst(source).unwrap();
            let html = html(source).unwrap();
            for id in ["name", "name-1", "name-2"] {
                assert!(typst.contains(&format!("#label(\"md-heading-{id}\")")));
                assert!(html.contains(&format!("id=\"{id}\"")));
            }
        }
        assert!(to_typst("# First {#same}\n\n# Other {#same}").is_err());
        assert!(html("# First {#same}\n\n# Other {#same}").is_err());
        assert!(to_typst("# Heading {.class}").is_err());
    }

    #[test]
    fn raw_comment_edge_cases_never_panic() {
        for comment in [
            "<!-->",
            "<!--->",
            "<!-- unclosed",
            "<!-- --><script>bad</script>",
        ] {
            assert!(html_kind(comment).is_err(), "{comment}");
            assert!(std::panic::catch_unwind(|| to_typst(comment)).is_ok());
            assert!(std::panic::catch_unwind(|| html(comment)).is_ok());
        }
        assert!(html_kind("<!---->").is_ok());
        assert_eq!(to_typst("<!-- multi\nline -->").unwrap(), "");
    }

    #[test]
    fn percent_encoded_links_preserve_anchors_and_reject_hidden_schemes() {
        for url in [
            "javascript%3Aalert(1)",
            "%6Aavascript:alert(1)",
            "data%3Atext/html,bad",
            "https://host/%00",
        ] {
            assert!(to_typst(&format!("[bad]({url})")).is_err(), "{url}");
            assert!(html(&format!("[bad]({url})")).is_err(), "{url}");
        }
        let source = "# Target {#a:b}\n\n[anchor](#a%3Ab) and [web](https://example.com/a%20b)";
        assert!(
            to_typst(source)
                .unwrap()
                .contains("#link(label(\"md-heading-a:b\"))")
        );
        assert!(html(source).unwrap().contains("https://example.com/a%20b"));
    }

    #[test]
    fn native_pdf_table_repeats_headers_on_every_page() {
        let mut source = String::from("| Record | Payload |\n| :--- | ---: |\n");
        for index in 0..110 {
            source.push_str(&format!("| row_{index:03} | data_{index:03} |\n"));
        }
        let bytes = crate::to_pdf(
            &source,
            &crate::Config::default(),
            std::path::Path::new("."),
        )
        .unwrap();
        let document = lopdf::Document::load_mem(&bytes).unwrap();
        let pages: Vec<_> = document.get_pages().keys().copied().collect();
        assert!(
            pages.len() >= 3,
            "expected multiple pages, got {}",
            pages.len()
        );
        for page in &pages {
            let extracted = document.extract_text(&[*page]).unwrap();
            assert!(
                extracted.contains("Record"),
                "header missing from page {page}: {extracted}"
            );
            assert!(
                extracted.contains("Payload"),
                "header missing from page {page}: {extracted}"
            );
        }
        let extracted = document.extract_text(&pages).unwrap();
        for index in 0..110 {
            let row = format!("row_{index:03}");
            assert_eq!(
                extracted.matches(&row).count(),
                1,
                "row missing or repeated: {row}"
            );
        }
    }

    #[test]
    fn native_pdf_preserves_inline_spacing_and_empty_content() {
        let source = "alpha **bold** omega *italic* after `inline` done.\n\n[external](https://example.com) link.\n\n# !!!\n\n# !!!\n";
        let bytes =
            crate::to_pdf(source, &crate::Config::default(), std::path::Path::new(".")).unwrap();
        let document = lopdf::Document::load_mem(&bytes).unwrap();
        let extracted = document.extract_text(&[1]).unwrap();
        let normalized = extracted.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(
            normalized.contains("alpha bold omega italic after inline done."),
            "{normalized}"
        );
        assert!(normalized.contains("external link."), "{normalized}");
        let empty =
            crate::to_pdf("", &crate::Config::default(), std::path::Path::new(".")).unwrap();
        assert!(
            !lopdf::Document::load_mem(&empty)
                .unwrap()
                .get_pages()
                .is_empty()
        );
    }

    #[test]
    fn native_pdf_contains_raster_image_objects() {
        let directory = tempfile::tempdir().unwrap();
        let png: &[u8] = &[
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1,
            8, 2, 0, 0, 0, 144, 119, 83, 222, 0, 0, 0, 12, 73, 68, 65, 84, 120, 156, 99, 248, 207,
            192, 0, 0, 3, 1, 1, 0, 201, 254, 146, 239, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96,
            130,
        ];
        std::fs::write(directory.path().join("pixel.png"), png).unwrap();
        let bytes = crate::to_pdf(
            "Before ![red pixel](pixel.png) after.",
            &crate::Config::default(),
            directory.path(),
        )
        .unwrap();
        let document = lopdf::Document::load_mem(&bytes).unwrap();
        assert!(
            document.objects.values().any(|object| {
                object.as_stream().ok().is_some_and(|stream| {
                    stream
                        .dict
                        .get(b"Subtype")
                        .ok()
                        .and_then(|value| value.as_name().ok())
                        == Some(b"Image".as_slice())
                })
            }),
            "PDF did not contain an image object"
        );
        let extracted = document.extract_text(&[1]).unwrap();
        assert!(extracted.contains("Before"), "{extracted}");
        assert!(extracted.contains("after."), "{extracted}");
    }

    #[test]
    fn native_pdf_vector_image_changes_when_its_content_changes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("mark.svg");
        let svg = |color| {
            format!(
                "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"160\" height=\"80\"><rect width=\"160\" height=\"80\" fill=\"{color}\"/></svg>"
            )
        };
        std::fs::write(&path, svg("red")).unwrap();
        let source = "Before\n\n![colored mark](mark.svg)\n\nAfter";
        let red = crate::to_pdf(source, &crate::Config::default(), directory.path()).unwrap();
        std::fs::write(&path, svg("blue")).unwrap();
        let blue = crate::to_pdf(source, &crate::Config::default(), directory.path()).unwrap();
        assert_ne!(red, blue, "SVG changes were absent from PDF output");
        for bytes in [&red, &blue] {
            let document = lopdf::Document::load_mem(bytes).unwrap();
            let page = document.get_pages()[&1];
            let content =
                lopdf::content::Content::decode(&document.get_page_content(page)).unwrap();
            assert!(
                content.operations.iter().any(|operation| matches!(
                    operation.operator.as_str(),
                    "f" | "f*" | "B" | "B*" | "Do"
                )),
                "PDF did not paint the vector image: {:?}",
                content.operations
            );
        }
    }
}
