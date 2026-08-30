//! Themes are data. Built-in themes are TOML files in `data/themes/`, embedded
//! via `include_dir` and overridable from `<config dir>/azdocs/themes/` — the
//! same shape as the query pack and `display_names.toml`.
//!
//! A theme file supplies *values*: colours (as expressions over the branding
//! palette), a type scale, and a choice from a closed set of layout
//! strategies. Every emitter implements each strategy once and branches only
//! on those values, so adding a theme is a new TOML file, never new Rust.

pub mod color;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};
use serde::{Deserialize, Serialize};

use crate::error::ThemeError;
use color::{ColorVars, Rgb};

static BUILTIN_THEMES: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/data/themes");

/// The theme used when `[branding] theme` is unset.
pub const DEFAULT_THEME: &str = "dashboard";

/// `<platform config dir>/azdocs/themes`, the drop-in directory for user
/// themes (same pattern as `querypack::loader::user_queries_dir`).
pub fn user_themes_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "azdocs").map(|dirs| dirs.config_dir().join("themes"))
}

// ---------------------------------------------------------------- schema ----

/// One theme as parsed from TOML. Palette fields hold *expressions* until
/// [`ThemeSpec::resolve`] turns them into literal `#rrggbb`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ThemeSpec {
    pub description: String,
    pub palette: Palette,
    pub typography: Typography,
    pub layout: Layout,
}

/// Colour roles. Holds expressions before [`ThemeSpec::resolve`] and literal
/// `#rrggbb` after; see [`color::resolve`] for the expression syntax.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Palette {
    pub primary: String,
    pub primary_dark: String,
    pub primary_tint: String,
    pub accent: String,
    pub accent_tint: String,
    /// Text drawn on a `primary` fill.
    pub on_primary: String,
    /// Cover band/block background and its text.
    pub band: String,
    pub on_band: String,
    pub ink: String,
    pub muted: String,
    pub rule: String,
    pub surface: String,
    /// Alternating table row fill; only used when `layout.zebra_rows`.
    pub zebra: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Severity {
    pub high: SeverityColors,
    pub medium: SeverityColors,
    pub low: SeverityColors,
    pub info: SeverityColors,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SeverityColors {
    pub text: String,
    pub fill: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Typography {
    /// Family used by the PDF and HTML; must be loadable by the PDF font book
    /// (vendored in `data/fonts/` or supplied via `[branding] font_dir`).
    pub sans: String,
    pub mono: String,
    /// DOCX resolves fonts by name on the reader's machine and cannot embed
    /// them, so these default to families that ship with Office everywhere.
    pub docx_sans: String,
    pub docx_mono: String,
    pub base_pt: f32,
    pub small_pt: f32,
    pub table_pt: f32,
    pub table_header_pt: f32,
    pub title_pt: f32,
    pub subtitle_pt: f32,
    pub h1_pt: f32,
    pub h2_pt: f32,
    pub h3_pt: f32,
    pub stat_value_pt: f32,
    pub stat_label_pt: f32,
    pub line_height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    pub cover: CoverStyle,
    pub table: TableStyle,
    pub stat: StatStyle,
    pub heading_numbering: bool,
    /// Full-page divider before each top-level chapter.
    pub divider_pages: bool,
    pub running_header: bool,
    pub zebra_rows: bool,
    pub rule_pt: f32,
    pub radius_pt: f32,
    pub table_inset_pt: f32,
    /// Height of the cover band/block, in points.
    pub cover_band_pt: f32,
}

/// Cover strategies. Every emitter implements all three; a theme picks one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoverStyle {
    /// Colour band across the top, content left-aligned below it.
    Band,
    /// Centred type with a hairline rule, no fills.
    Editorial,
    /// Full-bleed colour block with reversed-out title.
    Block,
}

/// Table header/body strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TableStyle {
    /// Filled `primary` header with `on_primary` text, full grid.
    SolidHeader,
    /// Horizontal hairlines only, no vertical rules, no header fill.
    Hairline,
    /// Tinted header, horizontal rules only.
    Banded,
}

/// Summary-statistic strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StatStyle {
    /// Tinted card with an accent top rule.
    Card,
    /// Hairline-outlined box.
    Outline,
    /// No box: value over label.
    Bare,
}

// -------------------------------------------------------------- defaults ----

impl Default for Palette {
    fn default() -> Self {
        Self {
            primary: "$primary".to_owned(),
            primary_dark: "darken($primary, 0.15)".to_owned(),
            primary_tint: "lighten($primary, 0.88)".to_owned(),
            accent: "$accent".to_owned(),
            accent_tint: "lighten($accent, 0.85)".to_owned(),
            on_primary: "readable_on($primary)".to_owned(),
            band: "$primary".to_owned(),
            on_band: "readable_on($primary)".to_owned(),
            ink: "#1a1a2e".to_owned(),
            muted: "#6a6a75".to_owned(),
            rule: "#d8d8e0".to_owned(),
            surface: "#ffffff".to_owned(),
            zebra: "lighten($primary, 0.95)".to_owned(),
            severity: Severity::default(),
        }
    }
}

impl Default for Severity {
    fn default() -> Self {
        Self {
            high: SeverityColors::new("#a4262c", "#f8cecc"),
            medium: SeverityColors::new("#8a5300", "#ffe6cc"),
            low: SeverityColors::new("#6f5b00", "#fff2cc"),
            info: SeverityColors::new("#1f4e79", "#dae8fc"),
        }
    }
}

impl Default for SeverityColors {
    fn default() -> Self {
        Self::new("#1a1a2e", "#eeeef2")
    }
}

impl SeverityColors {
    fn new(text: &str, fill: &str) -> Self {
        Self {
            text: text.to_owned(),
            fill: fill.to_owned(),
        }
    }
}

impl Severity {
    /// Colours for a finding severity, or `None` for a level no theme defines.
    pub fn level(&self, name: &str) -> Option<&SeverityColors> {
        match name {
            "high" => Some(&self.high),
            "medium" => Some(&self.medium),
            "low" => Some(&self.low),
            "info" => Some(&self.info),
            _ => None,
        }
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            sans: "IBM Plex Sans".to_owned(),
            mono: "IBM Plex Mono".to_owned(),
            docx_sans: "Aptos".to_owned(),
            docx_mono: "Aptos Mono".to_owned(),
            base_pt: 11.0,
            small_pt: 9.0,
            table_pt: 9.0,
            table_header_pt: 9.0,
            title_pt: 32.0,
            subtitle_pt: 14.0,
            h1_pt: 20.0,
            h2_pt: 15.0,
            h3_pt: 12.0,
            stat_value_pt: 22.0,
            stat_label_pt: 9.0,
            line_height: 1.45,
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            cover: CoverStyle::Band,
            table: TableStyle::SolidHeader,
            stat: StatStyle::Card,
            heading_numbering: true,
            divider_pages: false,
            running_header: true,
            zebra_rows: true,
            rule_pt: 0.5,
            radius_pt: 4.0,
            table_inset_pt: 5.5,
            cover_band_pt: 96.0,
        }
    }
}

// ------------------------------------------------------------- resolution ----

/// A theme with every palette expression resolved to `#rrggbb`. Built once and
/// carried on `BrandingContext`; emitters read it and never re-derive colours.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeTokens {
    pub name: String,
    pub description: String,
    pub palette: Palette,
    pub typography: Typography,
    pub layout: Layout,
}

impl ThemeSpec {
    /// Resolve palette expressions against the branding colours, which must
    /// already be validated `#rrggbb`.
    pub fn resolve(
        &self,
        name: &str,
        primary: &str,
        accent: &str,
    ) -> Result<ThemeTokens, ThemeError> {
        let vars = ColorVars {
            primary: Rgb::parse(primary).ok_or_else(|| branding_not_hex("primary", primary))?,
            accent: Rgb::parse(accent).ok_or_else(|| branding_not_hex("accent", accent))?,
        };

        let p = &self.palette;
        let at = |field: &str, expr: &str| color::resolve(&format!("{name}: {field}"), expr, &vars);
        let severity = |field: &str, s: &SeverityColors| -> Result<SeverityColors, ThemeError> {
            Ok(SeverityColors {
                text: at(&format!("palette.severity.{field}.text"), &s.text)?,
                fill: at(&format!("palette.severity.{field}.fill"), &s.fill)?,
            })
        };

        Ok(ThemeTokens {
            name: name.to_owned(),
            description: self.description.clone(),
            palette: Palette {
                primary: at("palette.primary", &p.primary)?,
                primary_dark: at("palette.primary_dark", &p.primary_dark)?,
                primary_tint: at("palette.primary_tint", &p.primary_tint)?,
                accent: at("palette.accent", &p.accent)?,
                accent_tint: at("palette.accent_tint", &p.accent_tint)?,
                on_primary: at("palette.on_primary", &p.on_primary)?,
                band: at("palette.band", &p.band)?,
                on_band: at("palette.on_band", &p.on_band)?,
                ink: at("palette.ink", &p.ink)?,
                muted: at("palette.muted", &p.muted)?,
                rule: at("palette.rule", &p.rule)?,
                surface: at("palette.surface", &p.surface)?,
                zebra: at("palette.zebra", &p.zebra)?,
                severity: Severity {
                    high: severity("high", &p.severity.high)?,
                    medium: severity("medium", &p.severity.medium)?,
                    low: severity("low", &p.severity.low)?,
                    info: severity("info", &p.severity.info)?,
                },
            },
            typography: self.typography.clone(),
            layout: self.layout.clone(),
        })
    }
}

fn branding_not_hex(field: &str, value: &str) -> ThemeError {
    ThemeError::Color {
        field: format!("branding.{field}_color"),
        expr: value.to_owned(),
        reason: "expected #rrggbb".to_owned(),
    }
}

impl Default for ThemeTokens {
    /// The default theme resolved against the default branding palette; used
    /// by emitters and tests that do not care about branding.
    fn default() -> Self {
        ThemeSpec::default()
            .resolve(DEFAULT_THEME, "#0078d4", "#4da3e8")
            .expect("default theme spec resolves against the default palette")
    }
}

// ----------------------------------------------------------------- loader ----

/// Built-in themes merged with user themes, keyed by name (the file stem).
#[derive(Debug, Default)]
pub struct ThemePack {
    themes: BTreeMap<String, ThemeSpec>,
}

impl ThemePack {
    /// Built-ins plus, when it exists, the user drop-in directory. A broken
    /// user theme is an error rather than a silent fallback: reports would
    /// otherwise render unbranded with no explanation.
    pub fn load() -> Result<Self, ThemeError> {
        let mut pack = Self::builtin()?;
        if let Some(dir) = user_themes_dir()
            && dir.is_dir()
        {
            pack.merge_dir(&dir)?;
        }
        Ok(pack)
    }

    pub fn builtin() -> Result<Self, ThemeError> {
        let mut pack = Self::default();
        for file in BUILTIN_THEMES.files() {
            let path = file.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Some(source) = file.contents_utf8() else {
                continue;
            };
            let origin = format!("builtin:{}", path.display());
            let spec = parse(source, &origin)?;
            pack.themes.insert(stem(path), spec);
        }
        Ok(pack)
    }

    /// Load every `*.toml` in `dir`, replacing same-named built-ins.
    pub fn merge_dir(&mut self, dir: &Path) -> Result<(), ThemeError> {
        let entries = std::fs::read_dir(dir).map_err(|source| ThemeError::ReadDir {
            path: dir.display().to_string(),
            source,
        })?;
        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| ThemeError::ReadDir {
                path: dir.display().to_string(),
                source,
            })?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("toml") {
                paths.push(path);
            }
        }
        // Deterministic order so two files claiming one name resolve the same
        // way on every platform.
        paths.sort();
        for path in paths {
            let origin = path.display().to_string();
            let source = std::fs::read_to_string(&path).map_err(|source| ThemeError::ReadDir {
                path: origin.clone(),
                source,
            })?;
            self.themes.insert(stem(&path), parse(&source, &origin)?);
        }
        Ok(())
    }

    /// Theme names, sorted.
    pub fn names(&self) -> Vec<&str> {
        self.themes.keys().map(String::as_str).collect()
    }

    /// Look up a theme, reporting the available names when it is missing so a
    /// typo in `[branding] theme` corrects itself.
    pub fn get(&self, name: &str) -> Result<&ThemeSpec, ThemeError> {
        self.themes.get(name).ok_or_else(|| ThemeError::Unknown {
            name: name.to_owned(),
            available: self.names().join(", "),
        })
    }
}

fn parse(source: &str, origin: &str) -> Result<ThemeSpec, ThemeError> {
    toml::from_str(source).map_err(|source| ThemeError::Parse {
        path: origin.to_owned(),
        source: Box::new(source),
    })
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_builtin_themes_all_parse_and_resolve() {
        let pack = ThemePack::builtin().expect("built-in themes parse");

        assert!(
            pack.names().contains(&DEFAULT_THEME),
            "the default theme must ship: {:?}",
            pack.names()
        );
        for name in pack.names() {
            let tokens = pack
                .get(name)
                .unwrap()
                .resolve(name, "#0078d4", "#4da3e8")
                .unwrap_or_else(|e| panic!("theme {name} failed to resolve: {e}"));
            assert!(
                tokens.palette.primary.starts_with('#'),
                "theme {name} left an unresolved expression"
            );
            assert!(
                !tokens.description.is_empty(),
                "theme {name} needs a description"
            );
        }
    }

    #[test]
    fn unit_builtin_themes_keep_print_type_readable() {
        let pack = ThemePack::builtin().unwrap();

        for name in pack.names() {
            let typography = &pack.get(name).unwrap().typography;
            assert!(typography.base_pt >= 10.5, "theme {name} body type");
            assert!(typography.small_pt >= 8.5, "theme {name} small type");
            assert!(typography.table_pt >= 8.5, "theme {name} table type");
            assert!(typography.h3_pt >= 11.5, "theme {name} tertiary heading");
        }
    }

    #[test]
    fn unit_get_reports_available_names_for_unknown_theme() {
        let pack = ThemePack::builtin().unwrap();

        let err = pack.get("nope").unwrap_err();

        let message = err.to_string();
        assert!(message.contains("nope"), "{message}");
        assert!(message.contains(DEFAULT_THEME), "{message}");
    }

    #[test]
    fn unit_resolve_derives_palette_from_branding_colors() {
        let spec = ThemeSpec::default();

        let tokens = spec.resolve("fluent", "#204060", "#80a0c0").unwrap();

        assert_eq!(tokens.palette.primary, "#204060");
        assert_eq!(tokens.palette.accent, "#80a0c0");
        // on_primary must invert for a dark brand colour.
        assert_eq!(tokens.palette.on_primary, "#ffffff");
    }

    #[test]
    fn unit_resolve_keeps_headers_legible_for_a_pale_brand_color() {
        let tokens = ThemeSpec::default()
            .resolve("fluent", "#ffe066", "#4da3e8")
            .unwrap();

        assert_eq!(tokens.palette.on_primary, "#111111");
    }

    #[test]
    fn unit_resolve_rejects_an_unparseable_palette_expression() {
        let spec = ThemeSpec {
            palette: Palette {
                ink: "chartreuse".to_owned(),
                ..Palette::default()
            },
            ..ThemeSpec::default()
        };

        let err = spec.resolve("broken", "#0078d4", "#4da3e8").unwrap_err();

        assert!(err.to_string().contains("broken: palette.ink"), "{err}");
    }

    #[test]
    fn unit_parse_rejects_unknown_keys() {
        let err = parse("[layout]\nkover = \"band\"\n", "test").unwrap_err();

        assert!(matches!(err, ThemeError::Parse { .. }), "{err}");
    }

    #[test]
    fn unit_merge_dir_replaces_builtin_theme_by_file_stem() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("fluent.toml"),
            "description = \"local override\"\n[layout]\ncover = \"editorial\"\n",
        )
        .unwrap();
        let mut pack = ThemePack::builtin().unwrap();

        pack.merge_dir(dir.path()).unwrap();

        let spec = pack.get("fluent").unwrap();
        assert_eq!(spec.description, "local override");
        assert_eq!(spec.layout.cover, CoverStyle::Editorial);
    }
}
