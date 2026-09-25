//! Code blocks, ```ts and the like, highlighted in the talk's own colors so
//! they match the rest of it. The languages are bat's.

use crate::markup::{Cell, Line, Rgb, Style, Theme};
use std::str::FromStr;
use std::sync::OnceLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{self as hl, ScopeSelectors, StyleModifier, ThemeItem, ThemeSettings};
use syntect::parsing::{SyntaxReference, SyntaxSet};

fn syntaxes() -> &'static SyntaxSet {
    static S: OnceLock<SyntaxSet> = OnceLock::new();
    S.get_or_init(two_face::syntax::extra_newlines)
}

fn find(lang: &str) -> Option<&'static SyntaxReference> {
    let ss = syntaxes();
    ss.find_syntax_by_token(lang).or_else(|| ss.syntaxes().iter().find(|s| s.name.eq_ignore_ascii_case(lang)))
}

pub fn known(lang: &str) -> bool {
    find(lang).is_some()
}

/// Every language, as (the name to write after ```, its full name).
pub fn languages() -> Vec<(String, String)> {
    syntaxes()
        .syntaxes()
        .iter()
        .filter_map(|s| Some((s.file_extensions.first()?.clone(), s.name.clone())))
        .collect()
}

fn color(c: Rgb) -> hl::Color {
    hl::Color { r: c.0, g: c.1, b: c.2, a: 0xff }
}

/// The talk's palette as a syntect theme, gruvbox's way round: keywords
/// bad, strings good, types accent, functions link, numbers warm.
fn theme(t: &Theme) -> hl::Theme {
    let rules: [(&str, Rgb); 8] = [
        ("comment, comment punctuation", t.muted),
        ("string, constant.character, string punctuation", t.good),
        ("constant.numeric, constant.language, entity.other.attribute-name", t.warm),
        ("keyword, storage", t.bad),
        ("entity.name.type, entity.name.class, support.type, support.class", t.accent),
        ("entity.name.function, support.function, entity.name.tag", t.link),
        ("variable.parameter, variable.other, punctuation", t.fg),
        ("markup.heading, markup.bold", t.accent),
    ];
    hl::Theme {
        settings: ThemeSettings { foreground: Some(color(t.fg)), ..ThemeSettings::default() },
        scopes: rules
            .iter()
            .map(|(sel, c)| ThemeItem {
                scope: ScopeSelectors::from_str(sel).unwrap(),
                style: StyleModifier { foreground: Some(color(*c)), ..StyleModifier::default() },
            })
            .collect(),
        ..hl::Theme::default()
    }
}

/// The lines highlighted, tabs as four spaces.
pub fn highlight(lines: &[String], lang: &str, t: &Theme) -> Vec<Line> {
    let syntax = find(lang).unwrap_or(syntaxes().find_syntax_plain_text());
    let th = theme(t);
    let mut h = HighlightLines::new(syntax, &th);
    lines
        .iter()
        .map(|l| {
            let l = format!("{}\n", l.replace('\t', "    "));
            let spans = h.highlight_line(&l, syntaxes()).unwrap_or_default();
            spans
                .iter()
                .flat_map(|(st, s)| {
                    let fg = Rgb(st.foreground.r, st.foreground.g, st.foreground.b);
                    s.trim_end_matches('\n').chars().map(move |ch| Cell { ch, st: Style::fg(fg) })
                })
                .collect()
        })
        .collect()
}
