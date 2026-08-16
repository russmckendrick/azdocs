//! Colour maths plus the one-call expression language theme files use for
//! palette values, so a theme derives its whole palette from the two branding
//! colours instead of hardcoding hex that ignores the user's brand.

use crate::error::ThemeError;

const WHITE: Rgb = Rgb {
    r: 0xff,
    g: 0xff,
    b: 0xff,
};
const BLACK: Rgb = Rgb { r: 0, g: 0, b: 0 };
/// `readable_on` returns near-black rather than pure black: it reads softer in
/// print and matches the default `ink`.
const NEAR_BLACK: Rgb = Rgb {
    r: 0x11,
    g: 0x11,
    b: 0x11,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    /// Parse `#rrggbb`; shorthand and named colours are deliberately rejected
    /// so theme files and `[branding]` accept exactly the same syntax.
    pub fn parse(value: &str) -> Option<Self> {
        let hex = value.strip_prefix('#')?;
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        Some(Self {
            r: u8::from_str_radix(&hex[0..2], 16).ok()?,
            g: u8::from_str_radix(&hex[2..4], 16).ok()?,
            b: u8::from_str_radix(&hex[4..6], 16).ok()?,
        })
    }

    pub fn to_hex(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    /// Blend toward `other`, where `weight` is how much of `other` to take.
    fn mix(self, other: Self, weight: f32) -> Self {
        let w = weight.clamp(0.0, 1.0);
        let blend = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * w).round() as u8;
        Self {
            r: blend(self.r, other.r),
            g: blend(self.g, other.g),
            b: blend(self.b, other.b),
        }
    }

    /// WCAG 2.1 relative luminance.
    fn relative_luminance(self) -> f32 {
        fn channel(value: u8) -> f32 {
            let s = f32::from(value) / 255.0;
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        }
        0.2126 * channel(self.r) + 0.7152 * channel(self.g) + 0.0722 * channel(self.b)
    }

    /// WCAG 2.1 contrast ratio, 1.0 (identical) to 21.0 (black on white).
    fn contrast(self, other: Self) -> f32 {
        let (a, b) = (self.relative_luminance(), other.relative_luminance());
        let (hi, lo) = if a > b { (a, b) } else { (b, a) };
        (hi + 0.05) / (lo + 0.05)
    }

    /// Whichever of white / near-black is more legible on `self`. This is what
    /// stops a pale `primary_color` producing white-on-white table headers.
    fn readable_on(self) -> Self {
        if self.contrast(WHITE) >= self.contrast(NEAR_BLACK) {
            WHITE
        } else {
            NEAR_BLACK
        }
    }
}

/// The branding colours a theme expression can refer to as `$primary` / `$accent`.
pub struct ColorVars {
    pub primary: Rgb,
    pub accent: Rgb,
}

/// Guards against a runaway expression in a hand-edited theme file; no real
/// palette nests anywhere near this deep.
const MAX_DEPTH: usize = 8;

/// Resolve one palette expression to a literal `#rrggbb`.
///
/// Accepts a literal (`#0078d4`), a branding reference (`$primary` /
/// `$accent`), or a call — `lighten($primary, 0.88)`, `darken(#0078d4, 0.2)`,
/// `mix($primary, #ffffff, 0.9)`, `readable_on($primary)`. Colour arguments
/// may themselves be calls, so a theme that derives its `primary` can keep
/// `on_primary` in step: `readable_on(darken($primary, 0.25))`.
pub fn resolve(field: &str, expr: &str, vars: &ColorVars) -> Result<String, ThemeError> {
    Ok(eval(field, expr, expr, vars, 0)?.to_hex())
}

/// `whole` is the original expression, carried through recursion so an error
/// inside a nested call still reports the expression the user actually wrote.
fn eval(
    field: &str,
    whole: &str,
    expr: &str,
    vars: &ColorVars,
    depth: usize,
) -> Result<Rgb, ThemeError> {
    if depth > MAX_DEPTH {
        return Err(invalid(field, whole, "expression nests too deeply"));
    }
    let trimmed = expr.trim();

    match trimmed {
        "$primary" => return Ok(vars.primary),
        "$accent" => return Ok(vars.accent),
        _ => {}
    }
    if trimmed.starts_with('#') {
        return Rgb::parse(trimmed)
            .ok_or_else(|| invalid(field, whole, &format!("`{trimmed}` is not #rrggbb")));
    }
    if trimmed.starts_with('$') {
        return Err(invalid(
            field,
            whole,
            &format!("unknown reference `{trimmed}` (expected $primary or $accent)"),
        ));
    }

    let (name, rest) = trimmed
        .split_once('(')
        .ok_or_else(|| invalid(field, whole, "expected #rrggbb, $primary, $accent, or a call"))?;
    let inner = rest
        .strip_suffix(')')
        .ok_or_else(|| invalid(field, whole, "missing closing `)`"))?;
    let args = split_args(inner);
    let color = |arg: &str| eval(field, whole, arg, vars, depth + 1);

    match (name.trim(), args.as_slice()) {
        ("lighten", [c, weight]) => Ok(color(c)?.mix(WHITE, factor(field, whole, weight)?)),
        ("darken", [c, weight]) => Ok(color(c)?.mix(BLACK, factor(field, whole, weight)?)),
        ("mix", [from, to, weight]) => {
            Ok(color(from)?.mix(color(to)?, factor(field, whole, weight)?))
        }
        ("readable_on", [c]) => Ok(color(c)?.readable_on()),
        ("lighten" | "darken" | "readable_on" | "mix", _) => Err(invalid(
            field,
            whole,
            &format!("wrong number of arguments for `{}`", name.trim()),
        )),
        (other, _) => Err(invalid(
            field,
            whole,
            &format!("unknown function `{other}` (expected lighten, darken, mix, or readable_on)"),
        )),
    }
}

/// Split on top-level commas so a nested call's own arguments stay intact.
fn split_args(input: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                args.push(input[start..index].trim());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    args.push(input[start..].trim());
    args
}

fn factor(field: &str, expr: &str, value: &str) -> Result<f32, ThemeError> {
    let parsed: f32 = value
        .parse()
        .map_err(|_| invalid(field, expr, &format!("`{value}` is not a number")))?;
    if (0.0..=1.0).contains(&parsed) {
        Ok(parsed)
    } else {
        Err(invalid(
            field,
            expr,
            &format!("factor `{value}` is outside 0.0..=1.0"),
        ))
    }
}

fn invalid(field: &str, expr: &str, reason: &str) -> ThemeError {
    ThemeError::Color {
        field: field.to_owned(),
        expr: expr.to_owned(),
        reason: reason.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> ColorVars {
        ColorVars {
            primary: Rgb::parse("#0078d4").unwrap(),
            accent: Rgb::parse("#4da3e8").unwrap(),
        }
    }

    fn resolved(expr: &str) -> String {
        resolve("test", expr, &vars()).unwrap()
    }

    #[test]
    fn unit_parse_rejects_shorthand_and_named_colors() {
        assert!(Rgb::parse("#abc").is_none());
        assert!(Rgb::parse("blue").is_none());
        assert!(Rgb::parse("#00zz00").is_none());
        assert_eq!(Rgb::parse("#0078D4").unwrap().to_hex(), "#0078d4");
    }

    #[test]
    fn unit_resolve_passes_through_literals_and_references() {
        assert_eq!(resolved("#123456"), "#123456");
        assert_eq!(resolved("  $primary "), "#0078d4");
        assert_eq!(resolved("$accent"), "#4da3e8");
    }

    #[test]
    fn unit_lighten_and_darken_move_toward_white_and_black() {
        assert_eq!(resolved("lighten($primary, 1.0)"), "#ffffff");
        assert_eq!(resolved("darken($primary, 1.0)"), "#000000");
        assert_eq!(resolved("lighten($primary, 0.0)"), "#0078d4");
    }

    #[test]
    fn unit_mix_weights_the_second_color() {
        assert_eq!(resolved("mix(#000000, #ffffff, 0.5)"), "#808080");
        assert_eq!(resolved("mix(#000000, #ffffff, 1.0)"), "#ffffff");
    }

    #[test]
    fn unit_readable_on_picks_white_over_dark_and_ink_over_pale() {
        assert_eq!(resolved("readable_on(#0b2545)"), "#ffffff");
        assert_eq!(resolved("readable_on(#ffe066)"), "#111111");
    }

    #[test]
    fn unit_resolve_evaluates_nested_calls() {
        // A theme that derives its primary keeps on_primary in step this way.
        assert_eq!(
            resolved("readable_on(darken($primary, 0.25))"),
            resolved("readable_on(#005a9f)")
        );
        assert_eq!(resolved("mix(darken(#ffffff, 1.0), #ffffff, 0.5)"), "#808080");
    }

    #[test]
    fn unit_resolve_rejects_unknown_function_and_bad_arity() {
        assert!(resolve("test", "sparkle($primary)", &vars()).is_err());
        assert!(resolve("test", "lighten($primary)", &vars()).is_err());
        assert!(resolve("test", "lighten($primary, 2.0)", &vars()).is_err());
        assert!(resolve("test", "lighten($primary, plenty)", &vars()).is_err());
        assert!(resolve("test", "lighten($primary, 0.5", &vars()).is_err());
        assert!(resolve("test", "chartreuse", &vars()).is_err());
        assert!(resolve("test", "$brand", &vars()).is_err());
    }

    #[test]
    fn unit_resolve_rejects_runaway_nesting() {
        let expr = format!("{}$primary{}", "lighten(".repeat(12), ", 0.1)".repeat(12));

        let err = resolve("test", &expr, &vars()).unwrap_err();

        assert!(err.to_string().contains("nests too deeply"), "{err}");
    }

    #[test]
    fn unit_error_reports_the_whole_expression_not_the_failing_fragment() {
        let err = resolve("palette.ink", "lighten(chartreuse, 0.2)", &vars()).unwrap_err();

        let message = err.to_string();
        assert!(message.contains("lighten(chartreuse, 0.2)"), "{message}");
        assert!(message.contains("palette.ink"), "{message}");
    }
}
