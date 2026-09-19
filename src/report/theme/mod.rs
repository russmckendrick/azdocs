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
pub const DEFAULT_THEME: &str = "field-report";

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
    /// The HTML surfaces' `prefers-color-scheme: dark` remap. Expressions
    /// may name any light field (`$surface`, `$ink`, ...); the defaults derive
    /// everything from the light palette so a theme need not spell it out.
    pub dark: DarkPalette,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DarkPalette {
    pub surface: String,
    pub ink: String,
    pub muted: String,
    pub rule: String,
    pub tint: String,
    pub zebra: String,
    /// Links and highlights on the dark surface; must stay readable there,
    /// which is why the default lifts the accent rather than reusing it.
    pub accent: String,
}

impl Default for DarkPalette {
    fn default() -> Self {
        Self {
            surface: "darken($surface, 0.92)".to_owned(),
            // Pure white on near-black glares; a touch of the paper colour
            // gives the same warm off-white the light palette's ink sits on.
            ink: "mix(readable_on($dark_surface), $surface, 0.12)".to_owned(),
            muted: "mix($dark_ink, $dark_surface, 0.45)".to_owned(),
            rule: "mix($dark_ink, $dark_surface, 0.78)".to_owned(),
            tint: "lighten($dark_surface, 0.05)".to_owned(),
            zebra: "lighten($dark_surface, 0.04)".to_owned(),
            accent: "lighten($accent, 0.35)".to_owned(),
        }
    }
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
    /// Prefer the Word families in PDF when those faces are installed.
    pub pdf_use_docx_fonts: bool,
    /// Display family used by covers, headings and prominent figures.
    pub serif: String,
    /// Working families used by the PDF and HTML; they must be loadable by the
    /// PDF font book (vendored or supplied via `[branding] font_dir`).
    pub sans: String,
    pub mono: String,
    /// DOCX resolves fonts by name on the reader's machine and cannot embed
    /// them, so these default to families that ship with Office everywhere.
    pub docx_serif: String,
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
    /// Working text scale for the optional technical reference.
    pub reference_scale: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    pub reference_table_borders: bool,
    pub reference_table_inset_pt: f32,
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
            primary: "#1c2430".to_owned(),
            primary_dark: "#14181d".to_owned(),
            primary_tint: "#f3efe7".to_owned(),
            accent: "$primary".to_owned(),
            accent_tint: "lighten($primary, 0.9)".to_owned(),
            on_primary: "#faf8f4".to_owned(),
            band: "#1c2430".to_owned(),
            on_band: "#faf8f4".to_owned(),
            ink: "#1c2430".to_owned(),
            muted: "#5a6470".to_owned(),
            rule: "#d8d2c6".to_owned(),
            surface: "#faf8f4".to_owned(),
            zebra: "#f3efe7".to_owned(),
            severity: Severity::default(),
            dark: DarkPalette::default(),
        }
    }
}

impl Default for Severity {
    fn default() -> Self {
        Self {
            high: SeverityColors::new("#a83a22", "#f5e4df"),
            medium: SeverityColors::new("#8a6d00", "#f3edd1"),
            low: SeverityColors::new("#5a6470", "#f3efe7"),
            info: SeverityColors::new("$primary", "lighten($primary, 0.9)"),
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
            serif: "IBM Plex Serif".to_owned(),
            sans: "IBM Plex Sans".to_owned(),
            mono: "IBM Plex Mono".to_owned(),
            pdf_use_docx_fonts: false,
            docx_serif: "Georgia".to_owned(),
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
            reference_scale: 0.9,
        }
    }
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            reference_table_borders: true,
            reference_table_inset_pt: 3.0,
            cover: CoverStyle::Editorial,
            table: TableStyle::Hairline,
            stat: StatStyle::Bare,
            heading_numbering: false,
            divider_pages: false,
            running_header: true,
            zebra_rows: false,
            rule_pt: 0.5,
            radius_pt: 3.0,
            table_inset_pt: 6.0,
            cover_band_pt: 0.0,
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
        let mut vars = ColorVars::new(
            Rgb::parse(primary).ok_or_else(|| branding_not_hex("primary", primary))?,
            Rgb::parse(accent).ok_or_else(|| branding_not_hex("accent", accent))?,
        );

        let p = &self.palette;
        // Light fields resolve in declaration order and each becomes a named
        // reference for the ones after it, then the dark block sees them all.
        let resolve_named = |vars: &mut ColorVars,
                             field: &str,
                             expr: &str,
                             name_as: &str|
         -> Result<String, ThemeError> {
            let hex = color::resolve(&format!("{name}: {field}"), expr, vars)?;
            if let Some(rgb) = Rgb::parse(&hex) {
                vars.named.insert(name_as.to_owned(), rgb);
            }
            Ok(hex)
        };
        let light = [
            ("primary", &p.primary),
            ("primary_dark", &p.primary_dark),
            ("primary_tint", &p.primary_tint),
            ("accent", &p.accent),
            ("accent_tint", &p.accent_tint),
            ("on_primary", &p.on_primary),
            ("band", &p.band),
            ("on_band", &p.on_band),
            ("ink", &p.ink),
            ("muted", &p.muted),
            ("rule", &p.rule),
            ("surface", &p.surface),
            ("zebra", &p.zebra),
        ];
        let mut resolved: std::collections::BTreeMap<&str, String> = Default::default();
        for (field, expr) in light {
            let hex = resolve_named(&mut vars, &format!("palette.{field}"), expr, field)?;
            resolved.insert(field, hex);
        }
        let dark = [
            ("surface", &p.dark.surface),
            ("ink", &p.dark.ink),
            ("muted", &p.dark.muted),
            ("rule", &p.dark.rule),
            ("tint", &p.dark.tint),
            ("zebra", &p.dark.zebra),
            ("accent", &p.dark.accent),
        ];
        let mut dark_resolved: std::collections::BTreeMap<&str, String> = Default::default();
        for (field, expr) in dark {
            let hex = resolve_named(
                &mut vars,
                &format!("palette.dark.{field}"),
                expr,
                &format!("dark_{field}"),
            )?;
            dark_resolved.insert(field, hex);
        }
        let at = |field: &str, expr: &str| color::resolve(&format!("{name}: {field}"), expr, &vars);
        let severity = |field: &str, s: &SeverityColors| -> Result<SeverityColors, ThemeError> {
            Ok(SeverityColors {
                text: at(&format!("palette.severity.{field}.text"), &s.text)?,
                fill: at(&format!("palette.severity.{field}.fill"), &s.fill)?,
            })
        };
        let take = |map: &mut std::collections::BTreeMap<&str, String>, key: &str| {
            map.remove(key).unwrap_or_default()
        };

        Ok(ThemeTokens {
            name: name.to_owned(),
            description: self.description.clone(),
            palette: Palette {
                primary: take(&mut resolved, "primary"),
                primary_dark: take(&mut resolved, "primary_dark"),
                primary_tint: take(&mut resolved, "primary_tint"),
                accent: take(&mut resolved, "accent"),
                accent_tint: take(&mut resolved, "accent_tint"),
                on_primary: take(&mut resolved, "on_primary"),
                band: take(&mut resolved, "band"),
                on_band: take(&mut resolved, "on_band"),
                ink: take(&mut resolved, "ink"),
                muted: take(&mut resolved, "muted"),
                rule: take(&mut resolved, "rule"),
                surface: take(&mut resolved, "surface"),
                zebra: take(&mut resolved, "zebra"),
                severity: Severity {
                    high: severity("high", &p.severity.high)?,
                    medium: severity("medium", &p.severity.medium)?,
                    low: severity("low", &p.severity.low)?,
                    info: severity("info", &p.severity.info)?,
                },
                dark: DarkPalette {
                    surface: take(&mut dark_resolved, "surface"),
                    ink: take(&mut dark_resolved, "ink"),
                    muted: take(&mut dark_resolved, "muted"),
                    rule: take(&mut dark_resolved, "rule"),
                    tint: take(&mut dark_resolved, "tint"),
                    zebra: take(&mut dark_resolved, "zebra"),
                    accent: take(&mut dark_resolved, "accent"),
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
    fn unit_builtin_pack_contains_only_the_field_report() {
        let pack = ThemePack::builtin().unwrap();

        assert_eq!(pack.names(), vec![DEFAULT_THEME]);
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
    fn unit_resolve_reserves_branding_color_for_the_accent_role() {
        let spec = ThemeSpec::default();

        let tokens = spec.resolve(DEFAULT_THEME, "#204060", "#80a0c0").unwrap();

        assert_eq!(tokens.palette.primary, "#1c2430");
        assert_eq!(tokens.palette.accent, "#204060");
        assert_eq!(tokens.palette.on_primary, "#faf8f4");
    }

    #[test]
    fn unit_resolve_keeps_a_pale_brand_color_out_of_document_chrome() {
        let tokens = ThemeSpec::default()
            .resolve(DEFAULT_THEME, "#ffe066", "#4da3e8")
            .unwrap();

        assert_eq!(tokens.palette.primary, "#1c2430");
        assert_eq!(tokens.palette.accent, "#ffe066");
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
            dir.path().join("field-report.toml"),
            "description = \"local override\"\n[layout]\ncover = \"editorial\"\n",
        )
        .unwrap();
        let mut pack = ThemePack::builtin().unwrap();

        pack.merge_dir(dir.path()).unwrap();

        let spec = pack.get(DEFAULT_THEME).unwrap();
        assert_eq!(spec.description, "local override");
        assert_eq!(spec.layout.cover, CoverStyle::Editorial);
    }

    #[test]
    fn unit_dark_palette_derives_from_light_when_theme_omits_it() {
        let tokens = ThemeSpec::default()
            .resolve("t", "#0078d4", "#4da3e8")
            .unwrap();
        let dark = &tokens.palette.dark;

        assert!(dark.surface.starts_with('#') && dark.surface != tokens.palette.surface);
        assert_ne!(
            dark.accent, tokens.palette.accent,
            "the accent is lifted for a dark surface"
        );
        assert!(dark.ink != dark.surface);
    }

    #[test]
    fn unit_dark_ink_is_readable_on_dark_surface() {
        let tokens = ThemeSpec::default()
            .resolve("t", "#0078d4", "#4da3e8")
            .unwrap();
        let surface = Rgb::parse(&tokens.palette.dark.surface).unwrap();
        let ink = Rgb::parse(&tokens.palette.dark.ink).unwrap();
        let accent = Rgb::parse(&tokens.palette.dark.accent).unwrap();

        assert!(
            ink.contrast(surface) >= 7.0,
            "ink {:?}",
            tokens.palette.dark
        );
        assert!(
            accent.contrast(surface) >= 3.0,
            "accent {:?}",
            tokens.palette.dark
        );
    }

    #[test]
    fn unit_dark_palette_expressions_can_name_light_fields() {
        let mut spec = ThemeSpec::default();
        spec.palette.dark.surface = "$primary_dark".into();
        spec.palette.dark.ink = "$surface".into();

        let tokens = spec.resolve("t", "#0078d4", "#4da3e8").unwrap();

        assert_eq!(tokens.palette.dark.surface, tokens.palette.primary_dark);
        assert_eq!(tokens.palette.dark.ink, tokens.palette.surface);
    }
}
