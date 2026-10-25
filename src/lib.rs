//! Self-contained, offline Markdown conversion.
//!
//! ```no_run
//! let config = md_to_pdf::Config::default();
//! let pdf = md_to_pdf::to_pdf("# Hello, Rust", &config, std::path::Path::new("."))?;
//! std::fs::write("hello.pdf", pdf)?;
//! # Ok::<(), anyhow::Error>(())
//! ```
pub mod config;
mod engine;
mod markdown;

use anyhow::Result;
pub use config::Config;
use std::path::Path;

/// Convert Markdown without front matter; pass resolved configuration explicitly.
/// Local images must be under `resource_root`. No network requests are made.
pub fn to_pdf(markdown: &str, config: &Config, resource_root: &Path) -> Result<Vec<u8>> {
    engine::render(markdown::to_typst(markdown)?, config, resource_root, None)
}

/// Keep image paths relative to an existing source file, while allowing assets
/// under a larger root (for example `../images/logo.svg`).
pub fn to_pdf_with_source_path(
    markdown: &str,
    config: &Config,
    resource_root: &Path,
    source_path: &Path,
) -> Result<Vec<u8>> {
    engine::render(
        markdown::to_typst(markdown)?,
        config,
        resource_root,
        Some(source_path),
    )
}

/// Export a static HTML document. This does not run JavaScript or fetch assets.
pub fn to_html(markdown: &str, config: &Config) -> Result<String> {
    config.validate()?;
    let escape = |text: &str| {
        text.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    let title = escape(
        config
            .document_title
            .as_deref()
            .unwrap_or("Markdown document"),
    );
    Ok(format!(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>{title}</title><style>{}</style></head><body><main>{}</main></body></html>\n",
        include_str!("../assets/markdown.css"),
        markdown::html(markdown)?
    ))
}
