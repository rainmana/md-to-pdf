//! A closed, offline Typst world: embedded fonts, generated source, local images.
use crate::config::Config;
use anyhow::{Result, anyhow, ensure};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

struct Fonts {
    fonts: Vec<Font>,
    book: LazyHash<FontBook>,
}

static FONTS: LazyLock<Fonts> = LazyLock::new(|| {
    let fonts: Vec<_> = typst_assets::fonts()
        .flat_map(|data| Font::iter(Bytes::new(data)))
        .collect();
    let book = LazyHash::new(FontBook::from_fonts(&fonts));
    Fonts { fonts, book }
});

struct OfflineWorld {
    library: LazyHash<Library>,
    source: Source,
    root: PathBuf,
    files: Mutex<HashMap<FileId, FileResult<Bytes>>>,
}

impl World for OfflineWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }
    fn book(&self) -> &LazyHash<FontBook> {
        &FONTS.book
    }
    fn main(&self) -> FileId {
        self.source.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main() {
            Ok(self.source.clone())
        } else {
            Err(FileError::AccessDenied)
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.main() {
            return Ok(Bytes::from_string(self.source.text().to_owned()));
        }
        if !matches!(id.root(), VirtualRoot::Project) {
            return Err(FileError::AccessDenied);
        }
        let mut files = self.files.lock().map_err(|_| FileError::AccessDenied)?;
        files
            .entry(id)
            .or_insert_with(|| {
                let path = id
                    .vpath()
                    .realize(&self.root)
                    .map_err(|_| FileError::AccessDenied)?;
                let canonical = path
                    .canonicalize()
                    .map_err(|err| FileError::from_io(err, &path))?;
                if !canonical.starts_with(&self.root) {
                    return Err(FileError::AccessDenied);
                }
                std::fs::read(&canonical)
                    .map(Bytes::new)
                    .map_err(|err| FileError::from_io(err, &canonical))
            })
            .clone()
    }

    fn font(&self, index: usize) -> Option<Font> {
        FONTS.fonts.get(index).cloned()
    }

    // No wall clock dependency; the generated document never invokes datetime.today().
    fn today(&self, _: Option<Duration>) -> Option<Datetime> {
        None
    }
}

pub fn render(
    body: String,
    config: &Config,
    resource_root: &Path,
    source_path: Option<&Path>,
) -> Result<Vec<u8>> {
    config.validate()?;
    for family in [&config.font, &config.code_font] {
        ensure!(
            FONTS.book.contains_family(&family.to_lowercase()),
            "font '{family}' is not embedded; choose Libertinus Serif, New Computer Modern, New Computer Modern Math, or DejaVu Sans Mono"
        );
    }
    let source = preamble(config)? + &body;
    let root = resource_root.canonicalize()?;
    let source_virtual_path = match source_path {
        Some(path) => VirtualPath::virtualize(&root, &path.canonicalize()?).map_err(|_| {
            anyhow!(
                "input {} is outside --basedir {}; choose a common ancestor directory",
                path.display(),
                root.display()
            )
        })?,
        None => VirtualPath::new("/__md_to_pdf__.typ")?,
    };
    let world = OfflineWorld {
        library: LazyHash::new(Library::default()),
        source: Source::new(
            FileId::new(RootedPath::new(VirtualRoot::Project, source_virtual_path)),
            source,
        ),
        root,
        files: Mutex::new(HashMap::new()),
    };
    // Typst's evaluator benefits from a larger stack, including when called as a library.
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .name("md-to-pdf-render".into())
            .stack_size(16 * 1024 * 1024)
            .spawn_scoped(scope, || {
                let result = typst::compile::<typst_layout::PagedDocument>(&world);
                let document = result.output.map_err(|errors| diagnostics(&errors))?;
                // Missing glyphs must not turn into silent blank characters in a successful PDF.
                if let Some(warning) = result.warnings.iter().find(|w| w.message.contains("glyph"))
                {
                    return Err(anyhow!(
                        "{}; the embedded fonts do not cover this character",
                        warning.message
                    ));
                }
                typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
                    .map_err(|errors| diagnostics(&errors))
            })?
            .join()
            .map_err(|_| anyhow!("native typesetter thread panicked"))?
    })
}

fn diagnostics(errors: &[typst::diag::SourceDiagnostic]) -> anyhow::Error {
    anyhow!(
        "native layout failed: {}",
        errors
            .iter()
            .map(|err| err.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    )
}

fn preamble(config: &Config) -> Result<String> {
    let (width, height) = config.page_size()?;
    let [top, right, bottom, left] = config.pdf_options.margin.points()?;
    let string = crate::markdown::literal;
    let mut out = format!(
        "#set document(title: {}, author: {})\n\
         #set page(width: {width}pt, height: {height}pt, margin: (top: {top}pt, right: {right}pt, bottom: {bottom}pt, left: {left}pt))\n\
         #set text(font: {}, size: {}pt, lang: \"en\")\n\
         #set par(leading: {}em, spacing: 0.8em)\n\
         #set heading(numbering: none)\n\
         #show heading: set text(fill: rgb(\"#1b334b\"))\n\
         #show raw: set text(font: {}, size: 0.83em)\n\
         #show raw.where(block: true): it => block(width: 100%, fill: rgb(\"#f3f5f7\"), inset: 9pt, radius: 3pt, breakable: true, it)\n\
         #show link: set text(fill: rgb(\"#126a94\"))\n",
        config
            .document_title
            .as_deref()
            .map(string)
            .unwrap_or("none".into()),
        config.author.as_deref().map(string).unwrap_or("()".into()),
        string(&config.font),
        config.font_size,
        config.line_height,
        string(&config.code_font),
    );
    if let Some(header) = &config.header {
        out.push_str(&format!(
            "#set page(header: align(center, text(size: 9pt, {})))\n",
            string(header)
        ));
    }
    if config.footer.is_some() || config.page_numbers {
        let footer = config.footer.as_deref().unwrap_or("");
        let number = if config.page_numbers {
            " + [ #context counter(page).display(\"1 / 1\", both: true)]"
        } else {
            ""
        };
        out.push_str(&format!(
            "#set page(footer: align(center, text(size: 9pt, {}{})))\n",
            string(footer),
            number
        ));
    }
    Ok(out)
}
