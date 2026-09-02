//! `{placeholder}` substitution for label strings.
//!
//! A placeholder is `{` + `[a-z_][a-z0-9_]*` + `}`; anything else that
//! contains braces is ordinary text, so a JSON value or an ARM id in a label
//! is never mis-parsed. Values are substituted verbatim and never re-scanned.

use std::fmt::Display;

use super::schema::Plural;

/// One piece of a template: literal text, or the name of a placeholder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment<'a> {
    Text(&'a str),
    Placeholder(&'a str),
}

/// Split a template around its placeholders. Text segments keep every
/// character, including braces that do not form a placeholder.
pub fn segments(template: &str) -> Vec<Segment<'_>> {
    let mut out = Vec::new();
    let mut rest = template;
    let mut text_start = 0usize;
    let base = template.as_ptr() as usize;

    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let name_len = placeholder_len(after);
        match name_len {
            Some(len) => {
                let name = &after[..len];
                let open_at = rest.as_ptr() as usize - base + open;
                if open_at > text_start {
                    out.push(Segment::Text(&template[text_start..open_at]));
                }
                out.push(Segment::Placeholder(name));
                rest = &after[len + 1..];
                text_start = rest.as_ptr() as usize - base;
            }
            None => rest = &rest[open + 1..],
        }
    }
    if text_start < template.len() {
        out.push(Segment::Text(&template[text_start..]));
    }
    out
}

/// Length of a well-formed placeholder name at the start of `after` (the
/// text following a `{`), when it is closed by `}`.
fn placeholder_len(after: &str) -> Option<usize> {
    let mut len = 0;
    for (index, ch) in after.char_indices() {
        if ch == '}' {
            return if index > 0 { Some(index) } else { None };
        }
        let valid = if index == 0 {
            ch.is_ascii_lowercase() || ch == '_'
        } else {
            ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_'
        };
        if !valid {
            return None;
        }
        len = index + ch.len_utf8();
    }
    let _ = len;
    None
}

/// Substitute `args` into `template`. A placeholder with no argument is left
/// literally in the output — visible and deterministic — and trips a debug
/// assertion so the test suite, which renders every surface, catches a
/// template and its call site disagreeing. Unused arguments are fine: a user
/// file may legitimately drop a placeholder.
pub fn fill(template: &str, args: &[(&str, &dyn Display)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    for segment in segments(template) {
        match segment {
            Segment::Text(text) => out.push_str(text),
            Segment::Placeholder(name) => match args.iter().find(|(key, _)| *key == name) {
                Some((_, value)) => out.push_str(&value.to_string()),
                None => {
                    debug_assert!(false, "label placeholder `{{{name}}}` has no argument");
                    out.push('{');
                    out.push_str(name);
                    out.push('}');
                }
            },
        }
    }
    out
}

/// A count and its noun, joined by `pattern` (normally `{count} {noun}`).
pub fn counted(pattern: &str, count: usize, noun: &Plural) -> String {
    fill(pattern, &[("count", &count), ("noun", &noun.pick(count))])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_segments_split_around_placeholders() {
        assert_eq!(
            segments("a {b} c {d}"),
            vec![
                Segment::Text("a "),
                Segment::Placeholder("b"),
                Segment::Text(" c "),
                Segment::Placeholder("d"),
            ]
        );
    }

    #[test]
    fn unit_segments_keep_braces_that_are_not_placeholders() {
        assert_eq!(
            segments("{} {Big} {a-b} {x}"),
            vec![Segment::Text("{} {Big} {a-b} "), Segment::Placeholder("x")]
        );
    }

    #[test]
    fn unit_fill_substitutes_and_ignores_unused_args() {
        let out = fill(
            "{count} of {total}",
            &[("count", &3), ("total", &9), ("x", &"y")],
        );
        assert_eq!(out, "3 of 9");
    }

    #[test]
    fn unit_fill_leaves_braces_in_values_alone() {
        let out = fill("id {id}", &[("id", &"{name}")]);
        assert_eq!(out, "id {name}");
    }

    #[test]
    fn unit_plural_picks_one_for_exactly_one() {
        let noun = Plural {
            one: "resource".into(),
            other: "resources".into(),
        };
        assert_eq!(counted("{count} {noun}", 1, &noun), "1 resource");
        assert_eq!(counted("{count} {noun}", 0, &noun), "0 resources");
        assert_eq!(counted("{count} {noun}", 2, &noun), "2 resources");
    }
}
