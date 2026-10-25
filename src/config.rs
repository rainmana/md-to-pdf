//! Data-only configuration, merged in defaults < file < front matter < CLI order.
use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub dest: Option<PathBuf>,
    pub basedir: Option<PathBuf>,
    pub document_title: Option<String>,
    pub author: Option<String>,
    pub font: String,
    pub code_font: String,
    pub font_size: f64,
    pub line_height: f64,
    pub header: Option<String>,
    pub footer: Option<String>,
    pub page_numbers: bool,
    pub as_html: bool,
    pub md_file_encoding: String,
    pub pdf_options: PdfOptions,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            dest: None,
            basedir: None,
            document_title: None,
            author: None,
            font: "Libertinus Serif".into(),
            code_font: "DejaVu Sans Mono".into(),
            font_size: 11.0,
            line_height: 0.65,
            header: None,
            footer: None,
            page_numbers: false,
            as_html: false,
            md_file_encoding: "utf-8".into(),
            pdf_options: PdfOptions::default(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PdfOptions {
    pub format: String,
    pub landscape: bool,
    pub margin: Margin,
    pub width: Option<Length>,
    pub height: Option<Length>,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            format: "a4".into(),
            landscape: false,
            margin: Margin::Sides(Margins::default()),
            width: None,
            height: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Length {
    Text(String),
    /// Numeric lengths follow Puppeteer's convention: CSS pixels (96 per inch).
    Pixels(f64),
}

impl Length {
    pub fn points(&self) -> Result<f64> {
        match self {
            Self::Pixels(value) => checked_points(*value * 0.75),
            Self::Text(value) => length_points(value),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Margin {
    Shorthand(String),
    Sides(Margins),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Margins {
    pub top: Length,
    pub right: Length,
    pub bottom: Length,
    pub left: Length,
}

impl Default for Margins {
    fn default() -> Self {
        Self {
            top: Length::Text("30mm".into()),
            right: Length::Text("40mm".into()),
            bottom: Length::Text("30mm".into()),
            left: Length::Text("20mm".into()),
        }
    }
}

impl Margin {
    /// Top, right, bottom, left, in PDF points.
    pub fn points(&self) -> Result<[f64; 4]> {
        match self {
            Self::Sides(m) => Ok([
                m.top.points()?,
                m.right.points()?,
                m.bottom.points()?,
                m.left.points()?,
            ]),
            Self::Shorthand(value) => {
                let parts = value
                    .split_whitespace()
                    .map(length_points)
                    .collect::<Result<Vec<_>>>()?;
                match parts.as_slice() {
                    [a] => Ok([*a; 4]),
                    [a, b] => Ok([*a, *b, *a, *b]),
                    [a, b, c] => Ok([*a, *b, *c, *b]),
                    [a, b, c, d] => Ok([*a, *b, *c, *d]),
                    _ => bail!("margin requires one to four lengths (for example '20mm 15mm')"),
                }
            }
        }
    }
}

fn checked_points(value: f64) -> Result<f64> {
    ensure!(
        value.is_finite() && (0.0..=14400.0).contains(&value),
        "length must be finite and between 0 and 200 inches"
    );
    Ok(value)
}

pub fn length_points(value: &str) -> Result<f64> {
    let value = value.trim().to_ascii_lowercase();
    let (number, factor) = [
        ("mm", 72.0 / 25.4),
        ("cm", 72.0 / 2.54),
        ("in", 72.0),
        ("pt", 1.0),
        ("px", 0.75),
    ]
    .into_iter()
    .find_map(|(unit, factor)| value.strip_suffix(unit).map(|n| (n, factor)))
    .unwrap_or((&value, 0.75));
    checked_points(
        number
            .trim()
            .parse::<f64>()
            .with_context(|| format!("invalid length '{value}'; use mm, cm, in, pt, or px"))?
            * factor,
    )
}

impl Config {
    pub fn page_size(&self) -> Result<(f64, f64)> {
        let options = &self.pdf_options;
        let (mut width, mut height) = match (&options.width, &options.height) {
            (Some(w), Some(h)) => (w.points()?, h.points()?),
            (None, None) => match options.format.to_ascii_lowercase().as_str() {
                "a0" => (2383.94, 3370.39),
                "a1" => (1683.78, 2383.94),
                "a2" => (1190.55, 1683.78),
                "a3" => (841.89, 1190.55),
                "a4" => (595.276, 841.89),
                "a5" => (419.528, 595.276),
                "a6" => (297.638, 419.528),
                "letter" => (612.0, 792.0),
                "legal" => (612.0, 1008.0),
                "tabloid" => (792.0, 1224.0),
                "ledger" => (1224.0, 792.0),
                other => bail!(
                    "unsupported paper format '{other}'; use A0-A6, Letter, Legal, Tabloid, Ledger, or width and height"
                ),
            },
            _ => bail!("pdf_options.width and height must be supplied together"),
        };
        if options.landscape {
            std::mem::swap(&mut width, &mut height);
        }
        let [top, right, bottom, left] = options.margin.points()?;
        ensure!(
            width - left - right >= 36.0 && height - top - bottom >= 36.0,
            "page margins must leave at least 36pt of content width and height"
        );
        Ok((width, height))
    }

    pub fn validate(&self) -> Result<()> {
        self.page_size()?;
        ensure!(
            self.font_size.is_finite() && (5.0..=72.0).contains(&self.font_size),
            "font_size must be between 5 and 72 points"
        );
        ensure!(
            self.line_height.is_finite() && (0.0..=3.0).contains(&self.line_height),
            "line_height (additional leading in em) must be between 0 and 3"
        );
        ensure!(
            encoding_rs::Encoding::for_label(self.md_file_encoding.as_bytes()).is_some(),
            "unknown Markdown encoding '{}'",
            self.md_file_encoding
        );
        Ok(())
    }
}

/// Merge objects recursively; scalar and array values replace earlier values.
pub fn merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (key, value) in overlay {
                merge(base.entry(key).or_insert(Value::Null), value);
            }
        }
        (base, overlay) => *base = overlay,
    }
}

pub fn read_config(path: &Path) -> Result<Value> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("reading config {}", path.display()))?;
    let value: Value = match path.extension().and_then(|s| s.to_str()) {
        Some("json") => serde_json::from_str(&text).context("invalid JSON config")?,
        Some("yaml" | "yml") => serde_yaml::from_str(&text).context("invalid YAML config")?,
        _ => bail!(
            "config must be .json, .yaml, or .yml; executable JavaScript config is unsupported by the native renderer"
        ),
    };
    ensure!(
        value.is_object(),
        "config must contain an object of options"
    );
    Ok(value)
}

pub fn front_matter(input: &str) -> Result<(Value, &str)> {
    let input = input.trim_start_matches('\u{feff}');
    let mut lines = input.split_inclusive('\n');
    let Some(first) = lines.next() else {
        return Ok((Value::Null, input));
    };
    if first.trim_end_matches(['\r', '\n']) != "---" {
        return Ok((Value::Null, input));
    }
    let start = first.len();
    let mut end = start;
    for line in lines {
        if matches!(line.trim_end_matches(['\r', '\n']), "---" | "...") {
            let value: Value =
                serde_yaml::from_str(&input[start..end]).context("invalid YAML front matter")?;
            ensure!(
                value.is_null() || value.is_object(),
                "front matter must contain an object of options"
            );
            return Ok((value, &input[end + line.len()..]));
        }
        end += line.len();
    }
    bail!("unclosed YAML front matter; add a closing '---' line")
}

pub fn resolve(file: Value, front: Value, cli: Value) -> Result<Config> {
    let mut value = serde_json::to_value(Config::default())?;
    for layer in [file, front, cli] {
        if !layer.is_null() {
            ensure!(layer.is_object(), "configuration layer must be an object");
            merge(&mut value, layer);
        }
    }
    let config: Config = serde_json::from_value(value).context("invalid native configuration; browser CSS, JavaScript and Puppeteer options are unsupported (see readme.md)")?;
    config.validate()?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn precedence_merges_nested_options() {
        let cfg = resolve(
            json!({"pdf_options":{"format":"letter","landscape":true}}),
            json!({"font_size":14,"pdf_options":{"format":"a5"}}),
            json!({"font_size":12}),
        )
        .unwrap();
        assert_eq!(cfg.font_size, 12.0);
        assert_eq!(cfg.pdf_options.format, "a5");
        assert!(cfg.pdf_options.landscape);
    }

    #[test]
    fn margins_units_and_validation() {
        assert_eq!(
            Margin::Shorthand("1pt 2pt 3pt".into()).points().unwrap(),
            [1.0, 2.0, 3.0, 2.0]
        );
        assert_eq!(length_points("1in").unwrap(), 72.0);
        assert!(length_points("NaNpt").is_err());
        assert!(length_points("-1mm").is_err());
        assert!(
            resolve(
                json!({"pdf_options":{"margin":"100in"}}),
                Value::Null,
                Value::Null
            )
            .is_err()
        );
    }

    #[test]
    fn frontmatter_crlf_and_failures() {
        let (front, body) = front_matter("\u{feff}---\r\nfont_size: 12\r\n---\r\n# Title").unwrap();
        assert_eq!(front["font_size"], 12);
        assert_eq!(body, "# Title");
        assert!(front_matter("---\nfont_size: 12").is_err());
        assert!(front_matter("---\n- list\n---\n").is_err());
        assert!(resolve(json!({"launch_options":{}}), Value::Null, Value::Null).is_err());
    }
}
