use lopdf::{Document, Object};
use md_to_pdf::{Config, to_pdf};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

fn binary() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_md-to-pdf"));
    command.stdin(Stdio::null());
    command
}

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn pdf(bytes: &[u8]) -> Document {
    assert!(bytes.starts_with(b"%PDF-"));
    Document::load_mem(bytes).expect("valid PDF structure")
}

fn text(document: &Document) -> String {
    document
        .extract_text(&document.get_pages().keys().copied().collect::<Vec<_>>())
        .expect("extractable Unicode text")
}

#[test]
fn actual_pdf_geometry_text_links_fonts_and_pagination() {
    let config = Config {
        document_title: Some("Native test".into()),
        header: Some("Running header".into()),
        footer: Some("Running footer".into()),
        page_numbers: true,
        ..Config::default()
    };
    let markdown = "# First page\n\nHello **Rust** and café, Ελληνικά, Кириллица.\n\n[External](https://example.com)\n\n<div class=\"page-break\"></div>\n\n# Second page\n\n[Internal](#first-page)";
    let document = pdf(&to_pdf(markdown, &config, Path::new(".")).unwrap());
    assert_eq!(document.get_pages().len(), 2);
    let extracted = text(&document);
    for expected in [
        "First page",
        "Hello",
        "Rust",
        "café",
        "Ελληνικά",
        "Кириллица",
        "Second page",
        "Running header",
        "Running footer",
        "1 / 2",
        "2 / 2",
    ] {
        assert!(
            extracted.contains(expected),
            "missing {expected:?} in {extracted}"
        );
    }
    let mut links = 0;
    let mut embedded_fonts = 0;
    for object in document.objects.values() {
        if let Ok(dict) = object.as_dict() {
            if dict.get(b"Subtype").and_then(Object::as_name).ok() == Some(b"Link") {
                links += 1;
            }
            if dict.has(b"FontFile2") || dict.has(b"FontFile3") {
                embedded_fonts += 1;
            }
        }
    }
    assert!(
        links >= 2,
        "external and internal links must be PDF annotations"
    );
    assert!(embedded_fonts > 0, "font programs must be embedded");
    let first = document.get_pages()[&1];
    let media = document
        .get_object(first)
        .unwrap()
        .as_dict()
        .unwrap()
        .get(b"MediaBox")
        .unwrap()
        .as_array()
        .unwrap();
    assert!((media[2].as_float().unwrap() - 595.276).abs() < 0.1);
    assert!((media[3].as_float().unwrap() - 841.89).abs() < 0.1);
}

#[test]
fn stdin_stdout_is_binary_only_and_alias_works() {
    let temp = tempfile::tempdir().unwrap();
    for executable in [
        env!("CARGO_BIN_EXE_md-to-pdf"),
        env!("CARGO_BIN_EXE_md2pdf"),
    ] {
        let mut child = Command::new(executable)
            .current_dir(temp.path())
            .env("PATH", "")
            .env("HOME", temp.path())
            .arg("-")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"# Offline stdin\n\nNo runtime downloads.")
            .unwrap();
        let output = success(child.wait_with_output().unwrap());
        assert!(text(&pdf(&output.stdout)).contains("Offline stdin"));
    }
}

#[test]
fn native_showcase_and_local_images() {
    let temp = tempfile::tempdir().unwrap();
    let input = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/showcase.md");
    let output = temp.path().join("nested/showcase.pdf");
    success(
        binary()
            .current_dir(temp.path())
            .arg(input)
            .arg("-o")
            .arg(&output)
            .output()
            .unwrap(),
    );
    let document = pdf(&std::fs::read(output).unwrap());
    assert_eq!(document.get_pages().len(), 2);
    assert!(text(&document).contains("A deliberate second page"));
    let content =
        lopdf::content::Content::decode(&document.get_page_content(document.get_pages()[&2]))
            .unwrap();
    assert!(
        content
            .operations
            .iter()
            .filter(|op| op.operator == "m")
            .count()
            >= 6,
        "SVG illustration must contain vector paths"
    );
}

#[test]
fn cli_precedence_encoding_multiple_inputs_and_html() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    std::fs::write(
        dir.join("config.json"),
        r#"{"document_title":"config","font_size":1000,"pdf_options":{"width":"210mm"}}"#,
    )
    .unwrap();
    std::fs::write(dir.join("one.md"), "---\ndocument_title: front\nfont_size: 12\npdf_options:\n  height: 297mm\n---\n# File one\n").unwrap();
    std::fs::write(dir.join("two.md"), "# File two\n").unwrap();
    success(
        binary()
            .current_dir(dir)
            .args([
                "one.md",
                "--config-file",
                "config.json",
                "--as-html",
                "--document-title",
                "CLI title",
            ])
            .output()
            .unwrap(),
    );
    let html = std::fs::read_to_string(dir.join("one.html")).unwrap();
    assert!(html.contains("<title>CLI title</title>"));
    assert!(html.contains("<h1 id=\"file-one\">File one</h1>"));
    std::fs::write(dir.join("one.md"), "# File one\n").unwrap();
    success(
        binary()
            .current_dir(dir)
            .args(["one.md", "two.md"])
            .output()
            .unwrap(),
    );
    assert!(text(&pdf(&std::fs::read(dir.join("one.pdf")).unwrap())).contains("File one"));
    assert!(text(&pdf(&std::fs::read(dir.join("two.pdf")).unwrap())).contains("File two"));
    std::fs::write(dir.join("encoded.md"), b"# caf\xe9").unwrap();
    success(
        binary()
            .current_dir(dir)
            .args(["encoded.md", "--md-file-encoding", "windows-1252"])
            .output()
            .unwrap(),
    );
    assert!(text(&pdf(&std::fs::read(dir.join("encoded.pdf")).unwrap())).contains("café"));
}

#[test]
fn failures_preserve_source_and_previous_output() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let source = "# Keep my source";
    std::fs::write(dir.join("one.md"), source).unwrap();
    let output = binary()
        .current_dir(dir)
        .args(["one.md", "-o", "one.md"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(std::fs::read_to_string(dir.join("one.md")).unwrap(), source);
    std::fs::write(dir.join("settings.json"), r#"{"dest":"settings.json"}"#).unwrap();
    let output = binary()
        .current_dir(dir)
        .args(["one.md", "--config-file", "settings.json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        std::fs::read_to_string(dir.join("settings.json")).unwrap(),
        r#"{"dest":"settings.json"}"#
    );
    std::fs::write(dir.join("one.pdf"), b"previous output").unwrap();
    for invalid in [
        "---\nfont_size: [broken\n---\n# Invalid",
        "<script>alert(1)</script>",
        "![remote](https://example.com/a.png)",
        "![missing](missing.png)",
    ] {
        std::fs::write(dir.join("one.md"), invalid).unwrap();
        let output = binary().current_dir(dir).arg("one.md").output().unwrap();
        assert!(!output.status.success(), "must reject {invalid:?}");
        assert_eq!(
            std::fs::read(dir.join("one.pdf")).unwrap(),
            b"previous output"
        );
    }
}

#[test]
fn duplicate_destinations_and_browser_settings_are_errors() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    for name in ["a.md", "a.markdown"] {
        std::fs::write(dir.join(name), "# Duplicate").unwrap();
    }
    assert!(
        !binary()
            .current_dir(dir)
            .args(["a.md", "a.markdown"])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!dir.join("a.pdf").exists());
    assert!(
        !binary()
            .current_dir(dir)
            .args([
                "a.md",
                "--pdf-options",
                r#"{"headerTemplate":"<b>Header</b>"}"#
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !binary()
            .current_dir(dir)
            .args(["a.md", "--css", "body {color:red}"])
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[cfg(unix)]
#[test]
fn image_symlinks_cannot_escape_resource_root() {
    let temp = tempfile::tempdir().unwrap();
    let assets = temp.path().join("assets");
    std::fs::create_dir(&assets).unwrap();
    std::fs::write(
        temp.path().join("outside.svg"),
        include_str!("fixtures/diagram.svg"),
    )
    .unwrap();
    std::os::unix::fs::symlink(temp.path().join("outside.svg"), assets.join("escape.svg")).unwrap();
    assert!(to_pdf("![Escape](escape.svg)", &Config::default(), &assets).is_err());
}

#[test]
fn source_relative_images_with_wider_basedir_and_native_globs() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    std::fs::create_dir(root.join("nested")).unwrap();
    std::fs::write(
        root.join("diagram.svg"),
        include_str!("fixtures/diagram.svg"),
    )
    .unwrap();
    std::fs::write(
        root.join("nested/a.md"),
        "# Relative image\n\n![Diagram](../diagram.svg)",
    )
    .unwrap();
    std::fs::write(root.join("nested/b.md"), "# Another document").unwrap();
    success(
        binary()
            .current_dir(root)
            .args(["nested/*.md", "--basedir", "."])
            .output()
            .unwrap(),
    );
    assert!(root.join("nested/a.pdf").exists());
    assert!(root.join("nested/b.pdf").exists());
    assert!(
        !binary()
            .current_dir(root)
            .arg("missing*.md")
            .output()
            .unwrap()
            .status
            .success()
    );
}
