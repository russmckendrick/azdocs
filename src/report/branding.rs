//! Resolved branding shared by every report emitter: validated colors and a
//! preloaded logo, so emitters never touch the filesystem or re-validate.

use std::path::Path;

use base64::Engine as _;
use serde::Serialize;

use super::theme::{ThemePack, ThemeTokens};
use crate::config::BrandingConfig;
use crate::error::ConfigError;
use crate::labels::Labels;

/// A branding logo loaded into memory: a data URI for HTML embedding plus the
/// raw bytes for emitters (PDF) that embed the image directly.
#[derive(Debug, Clone, Serialize)]
pub struct BrandingLogo {
    pub data_uri: String,
    /// Extension without the dot (`png`, `jpg`, `gif`, `svg`).
    #[serde(skip)]
    pub extension: String,
    #[serde(skip)]
    pub bytes: Vec<u8>,
}

/// Validated, ready-to-render branding. Built once from [`BrandingConfig`]
/// and passed to emitters alongside the (pure store data) `ReportContext`.
#[derive(Debug, Clone, Serialize)]
pub struct BrandingContext {
    pub company: String,
    pub title: String,
    pub subtitle: String,
    pub primary_color: String,
    pub accent_color: String,
    pub logo: Option<BrandingLogo>,
    pub page_size: String,
    pub margin: String,
    pub footer: String,
    /// The resolved theme: every design value every emitter uses.
    pub tokens: ThemeTokens,
    /// The resolved wording: every user-facing string every emitter prints.
    /// Not serialised — the HTML context and the Typst `branding` input keep
    /// their shape; emitters that need it pass it explicitly.
    #[serde(skip)]
    pub labels: Labels,
    /// Extra font faces from `branding.font_dir`, appended to the PDF font
    /// book. Bytes only — the PDF emitter reads them, nothing serializes them.
    #[serde(skip)]
    pub extra_fonts: Vec<Vec<u8>>,
    /// Only the requested installed print families; never serialized or
    /// downloaded. Loading here keeps native emitters independent of disk.
    #[serde(skip)]
    pub system_fonts: Vec<Vec<u8>>,
}

impl Default for BrandingContext {
    fn default() -> Self {
        Self::resolve(&BrandingConfig::default(), None)
            .expect("default branding config is statically valid")
    }
}

/// The configured theme resolved against the branding colours, without the
/// logo, fonts and labels a full [`BrandingContext`] loads. For consumers that
/// only draw with the palette, such as the diagram workbook's map.
pub fn theme_tokens(config: &BrandingConfig) -> Result<ThemeTokens, ConfigError> {
    validate_color("branding.primary_color", &config.primary_color)?;
    validate_color("branding.accent_color", &config.accent_color)?;
    Ok(ThemePack::load()?.resolve(
        &config.theme,
        &config.primary_color.to_lowercase(),
        &config.accent_color.to_lowercase(),
    )?)
}

impl BrandingContext {
    /// Validate colors and load the logo (if any); relative logo paths are
    /// resolved against `config_dir` (the directory the config file lives in).
    pub fn resolve(
        config: &BrandingConfig,
        config_dir: Option<&Path>,
    ) -> Result<Self, ConfigError> {
        validate_color("branding.primary_color", &config.primary_color)?;
        validate_color("branding.accent_color", &config.accent_color)?;
        let logo = match &config.logo {
            Some(path) => Some(load_logo(path, config_dir)?),
            None => None,
        };
        let primary_color = config.primary_color.to_lowercase();
        let accent_color = config.accent_color.to_lowercase();

        let mut tokens =
            ThemePack::load()?.resolve(&config.theme, &primary_color, &accent_color)?;
        // Branding beats the theme, which beats the built-in default.
        if !config.font_family.is_empty() {
            tokens.typography.sans = config.font_family.clone();
        }
        if !config.mono_family.is_empty() {
            tokens.typography.mono = config.mono_family.clone();
        }
        if !config.font_family.is_empty() || !config.mono_family.is_empty() {
            tokens.typography.pdf_use_docx_fonts = false;
        }

        let labels = crate::labels::resolve(config)?;

        let extra_fonts = match &config.font_dir {
            Some(path) => load_fonts(path, config_dir)?,
            None => Vec::new(),
        };
        let system_fonts = if tokens.typography.pdf_use_docx_fonts {
            installed_print_fonts(&[
                &tokens.typography.docx_serif,
                &tokens.typography.docx_sans,
                &tokens.typography.docx_mono,
            ])
        } else {
            Vec::new()
        };

        Ok(Self {
            company: config.company.clone(),
            title: config.title.clone(),
            subtitle: config.subtitle.clone(),
            primary_color,
            accent_color,
            logo,
            page_size: config.page_size.clone(),
            margin: config.margin.clone(),
            footer: config.footer.clone(),
            tokens,
            labels,
            extra_fonts,
            system_fonts,
        })
    }
}

fn installed_print_fonts(families: &[&str]) -> Vec<Vec<u8>> {
    static DATABASE: std::sync::OnceLock<usvg::fontdb::Database> = std::sync::OnceLock::new();
    let database = DATABASE.get_or_init(|| {
        let mut db = usvg::fontdb::Database::new();
        db.load_system_fonts();
        db
    });
    let mut fonts: Vec<_> = database
        .faces()
        .filter(|face| {
            face.families.iter().any(|(name, _)| {
                families
                    .iter()
                    .any(|family| name.eq_ignore_ascii_case(family))
            })
        })
        .filter_map(|face| database.with_face_data(face.id, |data, _| data.to_vec()))
        .collect();
    // Font discovery order varies by OS. Sort the bytes and deduplicate font
    // collections, which fontdb exposes once per contained face.
    fonts.sort();
    fonts.dedup();
    fonts
}

fn validate_color(field: &'static str, value: &str) -> Result<(), ConfigError> {
    let valid = value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|c| c.is_ascii_hexdigit());
    if valid {
        Ok(())
    } else {
        Err(ConfigError::InvalidColor {
            field,
            value: value.to_owned(),
        })
    }
}

/// Read every `.ttf`/`.otf` in `dir` so the PDF can use a typeface that is not
/// vendored. Sorted so the font book — and therefore the PDF bytes — stay
/// deterministic; anything that is not a font file is ignored.
fn load_fonts(dir: &Path, config_dir: Option<&Path>) -> Result<Vec<Vec<u8>>, ConfigError> {
    let resolved = resolve_path(dir, config_dir);
    let read_err = |source: std::io::Error| ConfigError::FontDirRead {
        path: resolved.clone(),
        source,
    };

    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in std::fs::read_dir(&resolved).map_err(read_err)? {
        let path = entry.map_err(read_err)?.path();
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        if path.is_file() && matches!(extension.as_deref(), Some("ttf" | "otf")) {
            paths.push(path);
        }
    }
    paths.sort();

    paths
        .into_iter()
        .map(|path| {
            std::fs::read(&path).map_err(|source| ConfigError::FontDirRead { path, source })
        })
        .collect()
}

fn resolve_path(path: &Path, config_dir: Option<&Path>) -> std::path::PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_dir.unwrap_or(Path::new(".")).join(path)
    }
}

fn load_logo(path: &Path, config_dir: Option<&Path>) -> Result<BrandingLogo, ConfigError> {
    let resolved = resolve_path(path, config_dir);
    let extension = resolved
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let mime = match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        _ => {
            return Err(ConfigError::LogoFormat {
                path: resolved.clone(),
            });
        }
    };
    let bytes = std::fs::read(&resolved).map_err(|source| ConfigError::LogoRead {
        path: resolved.clone(),
        source,
    })?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(BrandingLogo {
        data_uri: format!("data:{mime};base64,{encoded}"),
        extension,
        bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_resolve_accepts_defaults_without_config_dir() {
        let branding = BrandingContext::resolve(&BrandingConfig::default(), None).unwrap();

        assert_eq!(branding.primary_color, "#0078d4");
        assert!(branding.logo.is_none());
    }

    #[test]
    fn unit_resolve_rejects_invalid_hex_color() {
        let config = BrandingConfig {
            primary_color: "blue".to_owned(),
            ..BrandingConfig::default()
        };

        let err = BrandingContext::resolve(&config, None).unwrap_err();

        assert!(
            matches!(err, ConfigError::InvalidColor { field, .. } if field == "branding.primary_color")
        );
    }

    #[test]
    fn unit_resolve_rejects_shorthand_hex_color() {
        let config = BrandingConfig {
            accent_color: "#abc".to_owned(),
            ..BrandingConfig::default()
        };

        let err = BrandingContext::resolve(&config, None).unwrap_err();

        assert!(
            matches!(err, ConfigError::InvalidColor { field, .. } if field == "branding.accent_color")
        );
    }

    #[test]
    fn unit_resolve_errors_when_logo_file_missing() {
        let dir = tempfile::tempdir().unwrap();
        let config = BrandingConfig {
            logo: Some("missing.png".into()),
            ..BrandingConfig::default()
        };

        let err = BrandingContext::resolve(&config, Some(dir.path())).unwrap_err();

        assert!(matches!(err, ConfigError::LogoRead { .. }));
    }

    #[test]
    fn unit_resolve_rejects_unknown_logo_extension() {
        let config = BrandingConfig {
            logo: Some("logo.bmp".into()),
            ..BrandingConfig::default()
        };

        let err = BrandingContext::resolve(&config, None).unwrap_err();

        assert!(matches!(err, ConfigError::LogoFormat { .. }));
    }

    #[test]
    fn unit_resolve_loads_logo_relative_to_config_dir() {
        let dir = tempfile::tempdir().unwrap();
        // Smallest well-formed PNG header prefix; content is irrelevant here.
        std::fs::write(dir.path().join("logo.png"), [0x89, b'P', b'N', b'G']).unwrap();
        let config = BrandingConfig {
            logo: Some("logo.png".into()),
            ..BrandingConfig::default()
        };

        let branding = BrandingContext::resolve(&config, Some(dir.path())).unwrap();

        let logo = branding.logo.unwrap();
        assert!(logo.data_uri.starts_with("data:image/png;base64,"));
        assert_eq!(logo.extension, "png");
    }
}
