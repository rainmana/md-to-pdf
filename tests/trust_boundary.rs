//! Exercise the document/configuration trust boundary through the shipped CLIs.

use lopdf::Document;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

const BINARIES: [&str; 2] = [
    env!("CARGO_BIN_EXE_md-to-pdf"),
    env!("CARGO_BIN_EXE_md2pdf"),
];
const RESERVED: [(&str, &str); 4] = [
    ("dest", "../outside.txt"),
    ("basedir", ".."),
    ("as_html", "true"),
    ("md_file_encoding", "utf-8"),
];

fn invoke(executable: &str, cwd: &Path, args: &[&str], input: Option<&str>) -> Output {
    let mut command = Command::new(executable);
    command
        .current_dir(cwd)
        .args(args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(input) = input {
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    } else {
        command.output().unwrap()
    }
}

fn rejected(output: Output, key: &str) {
    assert!(
        !output.status.success(),
        "document-controlled {key} was accepted"
    );
    assert!(
        output.stdout.is_empty(),
        "rejected document emitted output bytes"
    );
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(
        error.contains(key),
        "rejection should identify {key:?}: {error}"
    );
    assert!(
        error.contains("front matter"),
        "rejection should identify the trust boundary: {error}"
    );
}

fn document(key: &str, value: &str) -> String {
    format!("---\n{key}: {value}\n---\n# Untrusted document\n")
}

#[test]
fn reserved_front_matter_is_rejected_for_files_stdin_and_both_binary_names() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("documents");
    std::fs::create_dir(&dir).unwrap();
    let sentinel = temp.path().join("outside.txt");
    std::fs::write(&sentinel, b"outside sentinel must survive").unwrap();
    std::fs::write(dir.join("input.pdf"), b"previous output must survive").unwrap();
    for executable in BINARIES {
        for (key, value) in RESERVED {
            let source = document(key, value);
            std::fs::write(dir.join("input.md"), &source).unwrap();
            rejected(invoke(executable, &dir, &["input.md"], None), key);
            rejected(invoke(executable, &dir, &["-"], Some(&source)), key);
            assert_eq!(
                std::fs::read(&sentinel).unwrap(),
                b"outside sentinel must survive"
            );
            assert_eq!(
                std::fs::read(dir.join("input.pdf")).unwrap(),
                b"previous output must survive"
            );
            assert_eq!(
                std::fs::read_to_string(dir.join("input.md")).unwrap(),
                source
            );
            assert!(!dir.join("input.html").exists());
        }
    }
}

#[test]
fn config_and_cli_overrides_cannot_mask_reserved_document_keys() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    std::fs::write(
        dir.join("trusted.json"),
        r#"{"dest":"config.pdf","basedir":".","as_html":true,"md_file_encoding":"utf-8"}"#,
    )
    .unwrap();
    for executable in BINARIES {
        for (key, value) in RESERVED {
            let source = document(key, value);
            std::fs::write(dir.join("input.md"), &source).unwrap();
            for input in ["input.md", "-"] {
                for output_flag in ["--output", "--dest", "-o"] {
                    let args = [
                        input,
                        "--config-file",
                        "trusted.json",
                        output_flag,
                        "trusted.html",
                        "--basedir",
                        ".",
                        "--as-html",
                        "--md-file-encoding",
                        "utf-8",
                    ];
                    rejected(
                        invoke(
                            executable,
                            dir,
                            &args,
                            (input == "-").then_some(source.as_str()),
                        ),
                        key,
                    );
                    assert!(!dir.join("trusted.html").exists());
                    assert!(!dir.join("config.pdf").exists());
                }
            }
        }
    }
}

#[test]
fn yaml_aliases_quoted_keys_and_null_values_do_not_hide_reserved_keys() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    let aliased = [
        (
            "dest",
            "document_title: &destination ../outside.txt\ndest: *destination".to_string(),
        ),
        (
            "dest",
            "document_title: &key dest\n? *key\n: ../outside.txt".to_string(),
        ),
        ("dest", "\"\\u0064est\": ../outside.txt".to_string()),
    ];
    let nulls = RESERVED.map(|(key, _)| (key, format!("{key}: null")));
    for (key, front) in aliased.into_iter().chain(nulls) {
        let source = format!("---\n{front}\n---\n# Alias attempt\n");
        rejected(
            invoke(
                BINARIES[0],
                dir,
                &["-", "--dest", "trusted.pdf"],
                Some(&source),
            ),
            key,
        );
        assert!(!dir.join("trusted.pdf").exists());
    }
}

#[test]
fn unknown_aliases_and_nested_io_keys_cannot_gain_document_authority() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path();
    for front in [
        "output: outside.pdf",
        "destination: outside.pdf",
        "base_dir: ..",
        "base-dir: ..",
        "as-html: true",
        "md-file-encoding: utf-8",
        "pdf_options:\n  dest: outside.pdf",
        "pdf_options:\n  basedir: ..",
        "defaults: &defaults\n  dest: outside.pdf\n<<: *defaults",
    ] {
        let source = format!("---\n{front}\n---\n# Equivalent key attempt\n");
        let output = invoke(
            BINARIES[0],
            dir,
            &["-", "--output", "trusted.pdf"],
            Some(&source),
        );
        assert!(
            !output.status.success(),
            "untrusted configuration accepted: {front}"
        );
        assert!(output.stdout.is_empty());
        assert!(!dir.join("trusted.pdf").exists());
        assert!(!dir.join("outside.pdf").exists());
    }
}

#[test]
fn front_matter_cannot_expand_asset_root_to_embed_outside_files() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("documents");
    std::fs::create_dir(&dir).unwrap();
    let secret = temp.path().join("private.svg");
    let secret_bytes = b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20\" height=\"20\"><rect width=\"20\" height=\"20\" fill=\"red\"/></svg>";
    std::fs::write(&secret, secret_bytes).unwrap();
    let source = "---\nbasedir: ..\n---\n# Private asset attempt\n\n![private](../private.svg)";
    std::fs::write(dir.join("input.md"), source).unwrap();
    rejected(invoke(BINARIES[0], &dir, &["input.md"], None), "basedir");
    rejected(invoke(BINARIES[1], &dir, &["-"], Some(source)), "basedir");
    assert!(!dir.join("input.pdf").exists());
    assert_eq!(std::fs::read(secret).unwrap(), secret_bytes);
}

#[test]
fn trusted_config_and_cli_keep_io_authority_while_layout_front_matter_works() {
    let temp = tempfile::tempdir().unwrap();
    let dir = temp.path().join("documents");
    std::fs::create_dir(&dir).unwrap();
    std::fs::write(temp.path().join("shared.svg"), b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"120\" height=\"40\"><rect width=\"120\" height=\"40\" fill=\"blue\"/></svg>").unwrap();
    std::fs::write(
        temp.path().join("trusted.json"),
        r#"{"dest":"../trusted.pdf","basedir":"..","md_file_encoding":"utf-8"}"#,
    )
    .unwrap();
    std::fs::write(dir.join("input.md"), "---\ndocument_title: Layout from document\nfont_size: 12\nheader: Document header\npdf_options:\n  margin: 15mm\n---\n# Native layout\n\n![shared asset](../shared.svg)\n").unwrap();
    let output = invoke(
        BINARIES[0],
        &dir,
        &["input.md", "--config-file", "../trusted.json"],
        None,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let pdf_bytes = std::fs::read(temp.path().join("trusted.pdf")).unwrap();
    let document = Document::load_mem(&pdf_bytes).unwrap();
    let extracted = document
        .extract_text(&document.get_pages().keys().copied().collect::<Vec<_>>())
        .unwrap();
    assert!(extracted.contains("Native layout"));
    assert!(extracted.contains("Document header"));
    let output = invoke(
        BINARIES[1],
        &dir,
        &[
            "input.md",
            "--config-file",
            "../trusted.json",
            "--basedir",
            "..",
            "--as-html",
            "--dest",
            "../trusted.html",
        ],
        None,
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let html = std::fs::read_to_string(temp.path().join("trusted.html")).unwrap();
    assert!(html.contains("<title>Layout from document</title>"));
    assert!(html.contains("../shared.svg"));
    assert_eq!(
        std::fs::read(temp.path().join("trusted.pdf")).unwrap(),
        pdf_bytes
    );
}
