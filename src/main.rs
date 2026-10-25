use anyhow::{Context, Result, bail, ensure};
use clap::{ArgAction, CommandFactory, Parser};
use md_to_pdf::config::{self, Config};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Parser)]
#[command(
    name = "md-to-pdf",
    version,
    disable_version_flag = true,
    about = "Convert Markdown to PDF entirely offline, with embedded fonts",
    after_help = "Examples:\n  md-to-pdf notes.md\n  md-to-pdf notes.md -o report.pdf --format letter\n  md-to-pdf *.md --page-numbers\n  md-to-pdf - < notes.md > report.pdf\n\nBrowser CSS/JavaScript/Puppeteer settings are unsupported. See readme.md for migration."
)]
struct Cli {
    /// Markdown files; use - for standard input
    files: Vec<PathBuf>,
    #[arg(short = 'v', long, action = ArgAction::Version)]
    version: Option<bool>,
    /// Output path (one input only); - writes binary data to stdout
    #[arg(short, long, alias = "dest")]
    output: Option<PathBuf>,
    /// JSON or YAML data-only configuration
    #[arg(long)]
    config_file: Option<PathBuf>,
    /// Watch inputs, local images, and config for changes (polls every 500 ms)
    #[arg(short, long)]
    watch: bool,
    /// Allowed asset root; image paths stay relative to the input file
    #[arg(long)]
    basedir: Option<PathBuf>,
    #[arg(long)]
    document_title: Option<String>,
    #[arg(long)]
    author: Option<String>,
    #[arg(long)]
    font: Option<String>,
    #[arg(long)]
    code_font: Option<String>,
    /// Body font size in points
    #[arg(long)]
    font_size: Option<f64>,
    /// Additional line leading in em (default: 0.65)
    #[arg(long)]
    line_height: Option<f64>,
    /// Plain text running header
    #[arg(long)]
    header: Option<String>,
    /// Plain text running footer
    #[arg(long)]
    footer: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    page_numbers: bool,
    /// A0-A6, Letter, Legal, Tabloid, or Ledger
    #[arg(long)]
    format: Option<String>,
    #[arg(long, action = ArgAction::SetTrue)]
    landscape: bool,
    /// One to four lengths, e.g. "20mm 15mm"
    #[arg(long)]
    margin: Option<String>,
    /// Native PDF options as JSON: format, margin, landscape, width, height
    #[arg(long)]
    pdf_options: Option<String>,
    /// Decode file/stdin bytes using this encoding (default: utf-8)
    #[arg(long)]
    md_file_encoding: Option<String>,
    /// Export static HTML instead of PDF
    #[arg(long, action = ArgAction::SetTrue)]
    as_html: bool,
}

impl Cli {
    fn overrides(&self) -> Result<Value> {
        let mut value = json!({});
        macro_rules! option {
            ($field:ident) => {
                if let Some(v) = &self.$field {
                    value[stringify!($field)] = serde_json::to_value(v)?;
                }
            };
        }
        option!(basedir);
        option!(document_title);
        option!(author);
        option!(font);
        option!(code_font);
        option!(font_size);
        option!(line_height);
        option!(header);
        option!(footer);
        option!(md_file_encoding);
        if let Some(dest) = &self.output {
            value["dest"] = serde_json::to_value(dest)?;
        }
        if self.as_html {
            value["as_html"] = true.into();
        }
        if self.page_numbers {
            value["page_numbers"] = true.into();
        }
        let mut pdf = match &self.pdf_options {
            Some(text) => {
                serde_json::from_str(text).context("--pdf-options must be a JSON object")?
            }
            None => json!({}),
        };
        ensure!(pdf.is_object(), "--pdf-options must be a JSON object");
        if let Some(format) = &self.format {
            pdf["format"] = format.clone().into();
        }
        if let Some(margin) = &self.margin {
            pdf["margin"] = margin.clone().into();
        }
        if self.landscape {
            pdf["landscape"] = true.into();
        }
        value["pdf_options"] = pdf;
        Ok(value)
    }
}

struct Job {
    source_path: Option<PathBuf>,
    body: String,
    config: Config,
    root: PathBuf,
    destination: Option<PathBuf>,
}

fn decode(bytes: &[u8], encoding: &str) -> Result<String> {
    let encoding = encoding_rs::Encoding::for_label(encoding.as_bytes())
        .with_context(|| format!("unknown Markdown encoding '{encoding}'"))?;
    let (text, _, malformed) = encoding.decode(bytes);
    ensure!(
        !malformed,
        "input is not valid {}; select --md-file-encoding for a different encoding",
        encoding.name()
    );
    Ok(text.into_owned())
}

fn job(bytes: &[u8], path: Option<&Path>, file_config: &Value, overrides: &Value) -> Result<Job> {
    let mut initial = json!({"md_file_encoding": "utf-8"});
    for layer in [file_config, overrides] {
        if let Some(encoding) = layer.get("md_file_encoding") {
            initial["md_file_encoding"] = encoding.clone();
        }
    }
    let encoding = initial["md_file_encoding"]
        .as_str()
        .context("md_file_encoding must be a string")?;
    let text = decode(bytes, encoding)?;
    let (front, body) = config::front_matter(&text)?;
    // Documents may style their content, but only the invoking user can select
    // filesystem scope, output destinations/formats, or the input decoder.
    for key in ["dest", "basedir", "as_html", "md_file_encoding"] {
        ensure!(
            front.get(key).is_none(),
            "{key} may only be set by the CLI or an explicitly selected config file; it is not allowed in document front matter"
        );
    }
    let config = config::resolve(file_config.clone(), front, overrides.clone())?;
    let destination = match &config.dest {
        Some(dest) if dest == Path::new("-") || dest == Path::new("stdout") => None,
        Some(dest) => Some(dest.clone()),
        None => path.map(|path| path.with_extension(if config.as_html { "html" } else { "pdf" })),
    };
    let root = config.basedir.clone().unwrap_or_else(|| {
        path.and_then(Path::parent)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."))
            .to_path_buf()
    });
    Ok(Job {
        source_path: path.map(Path::to_path_buf),
        body: body.to_owned(),
        config,
        root,
        destination,
    })
}

fn identity(path: &Path) -> Result<PathBuf> {
    if path.exists() {
        return path
            .canonicalize()
            .with_context(|| format!("resolving {}", path.display()));
    }
    let absolute = std::path::absolute(path)?;
    // Resolve existing ancestor symlinks even when the output's parents do not exist yet.
    let mut ancestor = absolute.as_path();
    let mut tail = Vec::new();
    while !ancestor.exists() {
        tail.push(
            ancestor
                .file_name()
                .context("output path has no filename")?
                .to_owned(),
        );
        ancestor = ancestor.parent().context("output path has no parent")?;
    }
    let mut resolved = ancestor.canonicalize()?;
    for part in tail.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    let mut temp = tempfile::NamedTempFile::new_in(parent)?;
    temp.write_all(bytes)?;
    temp.as_file().sync_all()?;
    temp.persist(path)
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn destination_key(path: &Path) -> Result<PathBuf> {
    let mut parent = path.parent().context("output has no parent directory")?;
    while !parent.exists() {
        parent = parent.parent().context("output has no existing ancestor")?;
    }
    // Query the actual output volume: macOS/Windows volumes can be case-sensitive,
    // and Unix can mount case-insensitive filesystems. The probe is removed on drop.
    let probe = tempfile::Builder::new()
        .prefix(".md-to-pdf-case-a")
        .tempfile_in(parent)?;
    let alternate = probe.path().with_file_name(
        probe
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_ascii_uppercase(),
    );
    if alternate.exists() {
        Ok(PathBuf::from(
            path.to_string_lossy()
                .nfd()
                .flat_map(char::to_lowercase)
                .collect::<String>(),
        ))
    } else {
        Ok(path.to_path_buf())
    }
}

fn plan(cli: &Cli, stdin: Option<&[u8]>) -> Result<Vec<Job>> {
    let file_config = cli
        .config_file
        .as_deref()
        .map(config::read_config)
        .transpose()?
        .unwrap_or(Value::Null);
    let overrides = cli.overrides()?;
    let mut jobs = Vec::new();
    let mut inputs = HashSet::new();
    if let Some(path) = &cli.config_file {
        inputs.insert(path.canonicalize()?);
    }
    if let Some(bytes) = stdin {
        jobs.push(job(bytes, None, &file_config, &overrides)?);
    } else {
        for path in &cli.files {
            ensure!(
                path.extension()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| matches!(
                        s.to_ascii_lowercase().as_str(),
                        "md" | "markdown" | "mdown" | "mkd"
                    )),
                "{} is not a Markdown file; native HTML-to-PDF input is unsupported",
                path.display()
            );
            inputs.insert(
                path.canonicalize()
                    .with_context(|| format!("reading {}", path.display()))?,
            );
            jobs.push(
                job(&std::fs::read(path)?, Some(path), &file_config, &overrides)
                    .with_context(|| format!("converting {}", path.display()))?,
            );
        }
    }
    let mut destinations = HashSet::new();
    for job in &jobs {
        if let Some(path) = &job.destination {
            let target = identity(path)?;
            ensure!(
                !inputs.contains(&target),
                "output {} would overwrite an input file",
                path.display()
            );
            ensure!(
                destinations.insert(destination_key(&target)?),
                "multiple inputs target {}; use one output per input",
                path.display()
            );
        } else {
            ensure!(
                jobs.len() == 1 && !cli.watch,
                "stdout output requires one input and cannot be watched"
            );
            ensure!(
                job.config.as_html || !std::io::stdout().is_terminal(),
                "refusing to write binary PDF to a terminal; use -o output.pdf or redirect stdout"
            );
        }
    }
    Ok(jobs)
}

fn convert(cli: &Cli, stdin: Option<&[u8]>) -> Result<Vec<PathBuf>> {
    let jobs = plan(cli, stdin)?;
    let roots = jobs
        .iter()
        .filter(|job| !job.config.as_html)
        .map(|job| job.root.clone())
        .collect();
    for job in jobs {
        let bytes = if job.config.as_html {
            md_to_pdf::to_html(&job.body, &job.config)?.into_bytes()
        } else {
            match &job.source_path {
                Some(path) => {
                    md_to_pdf::to_pdf_with_source_path(&job.body, &job.config, &job.root, path)?
                }
                None => md_to_pdf::to_pdf(&job.body, &job.config, &job.root)?,
            }
        };
        if let Some(path) = job.destination {
            write_atomic(&path, &bytes)?;
            eprintln!("Wrote {} ({} bytes)", path.display(), bytes.len());
        } else {
            std::io::stdout().lock().write_all(&bytes)?;
        }
    }
    Ok(roots)
}

fn snapshot(cli: &Cli, roots: &[PathBuf]) -> Vec<(PathBuf, Option<(SystemTime, u64)>)> {
    let mut paths = cli.files.clone();
    if let Some(path) = &cli.config_file {
        paths.push(path.clone());
    }
    fn images(dir: &Path, paths: &mut Vec<PathBuf>, depth: usize) {
        if depth == 0 {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if !matches!(
                    name.as_ref(),
                    ".git" | ".analysis" | ".claude" | "target" | "node_modules"
                ) {
                    images(&path, paths, depth - 1);
                }
            } else if path
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|ext| {
                    matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg"
                    )
                })
            {
                paths.push(path);
            }
        }
    }
    for root in roots {
        images(root, &mut paths, 16);
    }
    paths.sort();
    paths.dedup();
    paths
        .into_iter()
        .map(|path| {
            let stamp = std::fs::metadata(&path)
                .ok()
                .and_then(|m| Some((m.modified().ok()?, m.len())));
            (path, stamp)
        })
        .collect()
}

fn expand_globs(files: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let mut expanded = Vec::new();
    for file in files {
        if file.exists() || !file.to_string_lossy().contains(['*', '?', '[']) {
            expanded.push(file);
            continue;
        }
        let pattern = file
            .to_str()
            .context("glob patterns must be valid Unicode")?;
        #[cfg(windows)]
        let normalized = pattern.replace('\\', "/");
        #[cfg(windows)]
        let pattern = normalized.as_str();
        let matches = glob::glob(pattern)?.collect::<std::result::Result<Vec<_>, _>>()?;
        ensure!(
            !matches.is_empty(),
            "input pattern '{}' matched no files",
            file.display()
        );
        expanded.extend(matches);
    }
    Ok(expanded)
}

fn run() -> Result<()> {
    let mut cli = Cli::parse();
    cli.files = expand_globs(cli.files)?;
    let explicit_stdin = cli.files.iter().any(|path| path == Path::new("-"));
    ensure!(
        !explicit_stdin || cli.files.len() == 1,
        "'-' cannot be combined with file inputs"
    );
    ensure!(
        !cli.watch || !cli.files.is_empty() && !explicit_stdin,
        "--watch requires file inputs"
    );
    ensure!(
        cli.output.is_none() || cli.files.len() <= 1,
        "--output accepts one input only"
    );
    let mut stdin = Vec::new();
    if explicit_stdin || cli.files.is_empty() && !std::io::stdin().is_terminal() {
        std::io::stdin().read_to_end(&mut stdin)?;
    }
    if cli.files.is_empty() && stdin.is_empty() {
        Cli::command().print_help()?;
        println!();
        return Ok(());
    }
    if cli.watch && !stdin.is_empty() {
        bail!("--watch cannot consume stdin");
    }
    let stdin = if explicit_stdin || !stdin.is_empty() {
        Some(stdin.as_slice())
    } else {
        None
    };
    let mut roots = convert(&cli, stdin)?;
    if cli.watch {
        eprintln!("Watching files, config and local images; press Ctrl-C to stop.");
        let mut previous = snapshot(&cli, &roots);
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let current = snapshot(&cli, &roots);
            if current != previous {
                previous = current;
                match convert(&cli, None) {
                    Ok(next_roots) => {
                        roots = next_roots;
                        previous = snapshot(&cli, &roots);
                    }
                    Err(error) => {
                        if let Ok(jobs) = plan(&cli, None) {
                            roots = jobs
                                .into_iter()
                                .filter(|job| !job.config.as_html)
                                .map(|job| job.root)
                                .collect();
                            previous = snapshot(&cli, &roots);
                        }
                        eprintln!("Conversion failed; waiting for another change: {error:#}")
                    }
                }
                // Bound Typst's memoization cache during long watch sessions.
                typst::comemo::evict(10);
            }
        }
    }
    Ok(())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("md-to-pdf: {error:#}");
            std::process::ExitCode::FAILURE
        }
    }
}
