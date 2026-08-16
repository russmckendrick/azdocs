//! Resolved branding shared by every report emitter: validated colors and a
//! preloaded logo, so emitters never touch the filesystem or re-validate.

use std::path::Path;

use base64::Engine as _;
use serde::Serialize;

use crate::config::BrandingConfig;
use crate::error::ConfigError;

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
}

impl Default for BrandingContext {
    fn default() -> Self {
        Self::resolve(&BrandingConfig::default(), None)
            .expect("default branding config is statically valid")
    }
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
        Ok(Self {
            company: config.company.clone(),
            title: config.title.clone(),
            subtitle: config.subtitle.clone(),
            primary_color: config.primary_color.to_lowercase(),
            accent_color: config.accent_color.to_lowercase(),
            logo,
            page_size: config.page_size.clone(),
            margin: config.margin.clone(),
            footer: config.footer.clone(),
        })
    }
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

fn load_logo(path: &Path, config_dir: Option<&Path>) -> Result<BrandingLogo, ConfigError> {
    let resolved = if path.is_absolute() {
        path.to_path_buf()
    } else {
        config_dir.unwrap_or(Path::new(".")).join(path)
    };
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
