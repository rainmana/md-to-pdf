use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct Watcher {
    child: Child,
    log: PathBuf,
}

impl Watcher {
    fn start(directory: &Path, input: &Path, output: &Path, config: Option<&Path>) -> Self {
        let log = directory.join("watch.log");
        let stderr = fs::File::create(&log).unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_md-to-pdf"));
        command
            .current_dir(directory)
            .arg(input)
            .arg("--watch")
            .arg("--output")
            .arg(output)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::from(stderr));
        if let Some(config) = config {
            command.arg("--config-file").arg(config);
        }
        Self {
            child: command.spawn().unwrap(),
            log,
        }
    }

    fn log(&self) -> String {
        fs::read_to_string(&self.log).unwrap_or_default()
    }

    fn wait_for(&mut self, description: &str, ready: impl Fn() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            assert!(
                self.child.try_wait().unwrap().is_none(),
                "watch process exited while waiting for {description}: {}",
                self.log()
            );
            if ready() {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "timed out waiting for {description}: {}",
                self.log()
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn wait_ready(&mut self, output: &Path) {
        let log = self.log.clone();
        self.wait_for("initial PDF and watch loop", || {
            fs::read(output).is_ok_and(|bytes| bytes.starts_with(b"%PDF-"))
                && fs::read_to_string(&log).is_ok_and(|text| text.contains("Watching files"))
        });
        // The announcement precedes the initial snapshot; let that snapshot finish
        // before introducing a change, including on slower hosted runners.
        std::thread::sleep(Duration::from_millis(600));
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn svg(color: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="50"><rect width="100" height="50" fill="{color}"/></svg>"#
    )
}

#[test]
fn watch_rebuilds_when_an_image_in_a_hidden_directory_changes() {
    let directory = tempfile::tempdir().unwrap();
    let hidden = directory.path().join(".images");
    fs::create_dir(&hidden).unwrap();
    let image = hidden.join("diagram.svg");
    fs::write(&image, svg("red")).unwrap();
    let input = directory.path().join("document.md");
    fs::write(&input, "# Diagram\n\n![Diagram](.images/diagram.svg)\n").unwrap();
    let output = directory.path().join("document.pdf");
    let mut watcher = Watcher::start(directory.path(), &input, &output, None);
    watcher.wait_ready(&output);
    let before = fs::read(&output).unwrap();

    fs::write(&image, svg("blue")).unwrap();

    watcher.wait_for("PDF updated after hidden image change", || {
        fs::read(&output).is_ok_and(|bytes| bytes.starts_with(b"%PDF-") && bytes != before)
    });
}

#[test]
fn watch_recovers_when_missing_image_appears_in_new_resource_root() {
    let directory = tempfile::tempdir().unwrap();
    let first_root = directory.path().join("root-a");
    let next_root = directory.path().to_path_buf();
    fs::create_dir(&first_root).unwrap();
    fs::write(first_root.join("diagram.svg"), svg("red")).unwrap();
    let input = first_root.join("document.md");
    fs::write(&input, "# Diagram\n\n![Diagram](/diagram.svg)\n").unwrap();
    let output = directory.path().join("document.pdf");
    let config = directory.path().join("config.json");
    fs::write(
        &config,
        serde_json::to_vec(&serde_json::json!({"basedir": first_root})).unwrap(),
    )
    .unwrap();
    let mut watcher = Watcher::start(directory.path(), &input, &output, Some(&config));
    watcher.wait_ready(&output);
    let before = fs::read(&output).unwrap();

    fs::write(
        &config,
        serde_json::to_vec(&serde_json::json!({"basedir": next_root})).unwrap(),
    )
    .unwrap();
    let log = watcher.log.clone();
    watcher.wait_for("failed conversion in changed resource root", || {
        fs::read_to_string(&log).is_ok_and(|text| text.contains("Conversion failed"))
    });
    assert_eq!(fs::read(&output).unwrap(), before);

    fs::write(next_root.join("diagram.svg"), svg("blue")).unwrap();

    watcher.wait_for("PDF recovered after new root image appeared", || {
        fs::read(&output).is_ok_and(|bytes| bytes.starts_with(b"%PDF-") && bytes != before)
    });
}

#[test]
fn case_insensitive_output_aliases_are_rejected_before_any_write() {
    let directory = tempfile::tempdir().unwrap();
    let probe = directory.path().join("case-probe");
    fs::write(&probe, "probe").unwrap();
    if !directory.path().join("CASE-PROBE").exists() {
        // A case-sensitive filesystem supports these as two distinct outputs.
        return;
    }
    fs::remove_file(probe).unwrap();
    let first = directory.path().join("source.md");
    let second = directory.path().join("SOURCE.markdown");
    let first_output = directory.path().join("source.html");
    let second_output = directory.path().join("SOURCE.html");
    fs::write(&first, "First document\n").unwrap();
    fs::write(&second, "Second document\n").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_md-to-pdf"))
        .current_dir(directory.path())
        .arg("--as-html")
        .arg(first)
        .arg(second)
        .stdin(Stdio::null())
        .output()
        .unwrap();

    assert!(
        !result.status.success(),
        "conversion accepted aliases for a single output on a case-insensitive filesystem"
    );
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("multiple inputs"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!first_output.exists());
    assert!(!second_output.exists());
}
