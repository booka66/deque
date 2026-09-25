//! Text as the screen shows it: cells of a character and a style, and the
//! inline markup a talk writes them in.
//!
//!   **bold**            `code`, in the link color, taken as it is
//!   {accent}…{/}        a color: accent muted good bad warm link; they nest
//!   \x                  x itself: \* \` \{ \\

use unicode_width::UnicodeWidthChar;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    /// t of the way from self to other.
    pub fn mix(self, other: Rgb, t: f64) -> Rgb {
        let f = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t) as u8;
        Rgb(f(self.0, other.0), f(self.1, other.1), f(self.2, other.2))
    }

    /// `#rrggbb`, or SGR parameters as otis-theme gives them: `38;2;r;g;b`,
    /// `38;5;n`.
    pub fn parse(s: &str) -> Option<Rgb> {
        let s = s.trim();
        if let Some(h) = s.strip_prefix('#') {
            if h.len() != 6 {
                return None;
            }
            let b = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
            return Some(Rgb(b(0)?, b(2)?, b(4)?));
        }
        let p: Vec<u8> = s.split(';').map(|x| x.parse().ok()).collect::<Option<_>>()?;
        match p.as_slice() {
            [38 | 48, 2, r, g, b] => Some(Rgb(*r, *g, *b)),
            [38 | 48, 5, n] => Some(from_256(*n)),
            _ => None,
        }
    }
}

fn from_256(n: u8) -> Rgb {
    const BASE: [(u8, u8, u8); 16] = [
        (0, 0, 0), (205, 0, 0), (0, 205, 0), (205, 205, 0), (0, 0, 238), (205, 0, 205),
        (0, 205, 205), (229, 229, 229), (127, 127, 127), (255, 0, 0), (0, 255, 0),
        (255, 255, 0), (92, 92, 255), (255, 0, 255), (0, 255, 255), (255, 255, 255),
    ];
    match n {
        0..=15 => {
            let (r, g, b) = BASE[n as usize];
            Rgb(r, g, b)
        }
        16..=231 => {
            let v = |x: u8| if x == 0 { 0 } else { 55 + x * 40 };
            let n = n - 16;
            Rgb(v(n / 36), v(n / 6 % 6), v(n % 6))
        }
        _ => {
            let g = 8 + (n - 232) * 10;
            Rgb(g, g, g)
        }
    }
}

/// The talk's colors, gruvbox dark unless it says otherwise. fg and bg are
/// what fades end and start at.
#[derive(Clone, Debug)]
pub struct Theme {
    pub accent: Rgb,
    pub muted: Rgb,
    pub good: Rgb,
    pub bad: Rgb,
    pub warm: Rgb,
    pub link: Rgb,
    pub fg: Rgb,
    pub bg: Rgb,
}

impl Default for Theme {
    fn default() -> Self {
        Theme {
            accent: Rgb(250, 189, 47),
            muted: Rgb(146, 131, 116),
            good: Rgb(184, 187, 38),
            bad: Rgb(251, 73, 52),
            warm: Rgb(254, 128, 25),
            link: Rgb(131, 165, 152),
            fg: Rgb(235, 219, 178),
            bg: Rgb(40, 40, 40),
        }
    }
}

impl Theme {
    pub fn named(&self, name: &str) -> Option<Rgb> {
        Some(match name {
            "accent" => self.accent,
            "muted" => self.muted,
            "good" => self.good,
            "bad" => self.bad,
            "warm" => self.warm,
            "link" => self.link,
            _ => return None,
        })
    }

    pub fn set(&mut self, name: &str, c: Rgb) -> bool {
        match name {
            "accent" => self.accent = c,
            "muted" => self.muted = c,
            "good" => self.good = c,
            "bad" => self.bad = c,
            "warm" => self.warm = c,
            "link" => self.link = c,
            "fg" => self.fg = c,
            "bg" => self.bg = c,
            _ => return false,
        }
        true
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Style {
    pub fg: Option<Rgb>,
    pub bg: Option<Rgb>,
    pub bold: bool,
}

impl Style {
    pub fn fg(c: Rgb) -> Style {
        Style { fg: Some(c), ..Style::default() }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub ch: char,
    pub st: Style,
}

pub type Line = Vec<Cell>;

pub fn plain(s: &str, st: Style) -> Line {
    s.chars().map(|ch| Cell { ch, st }).collect()
}

/// How many columns a line takes.
pub fn width(l: &[Cell]) -> i32 {
    l.iter().map(|c| c.ch.width().unwrap_or(0) as i32).sum()
}

pub fn text(l: &[Cell]) -> String {
    l.iter().map(|c| c.ch).collect()
}

/// The line in one style throughout, over whatever it had, bold kept.
pub fn recolor(l: &[Cell], c: Rgb) -> Line {
    l.iter().map(|x| Cell { ch: x.ch, st: Style { fg: Some(c), ..x.st } }).collect()
}

/// The line with its colors gone: what typing shows before it settles.
pub fn bare(l: &[Cell]) -> Line {
    l.iter().map(|x| Cell { ch: x.ch, st: Style::default() }).collect()
}

/// Markup to a line, every cell starting from `base`. Unknown `{names}` are
/// kept as text, so braces in a slide need no escaping unless they spell a
/// color.
pub fn parse(s: &str, theme: &Theme, base: Style) -> Result<Line, String> {
    let mut out = Line::new();
    let mut stack: Vec<Option<Rgb>> = vec![];
    let mut st = base;
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < cs.len() {
        let c = cs[i];
        if c == '\\' && i + 1 < cs.len() {
            out.push(Cell { ch: cs[i + 1], st });
            i += 2;
        } else if c == '*' && cs.get(i + 1) == Some(&'*') {
            st.bold = !st.bold;
            i += 2;
        } else if c == '`' {
            let Some(end) = cs[i + 1..].iter().position(|&x| x == '`') else {
                return Err("a ` with no ` to close it".into());
            };
            let code = Style { fg: Some(theme.link), ..st };
            out.extend(cs[i + 1..i + 1 + end].iter().map(|&ch| Cell { ch, st: code }));
            i += end + 2;
        } else if c == '{' {
            let close = cs[i..].iter().position(|&x| x == '}').map(|p| i + p);
            let name: Option<String> = close.map(|e| cs[i + 1..e].iter().collect());
            match (close, name.as_deref()) {
                (Some(e), Some("/")) => {
                    let Some(prev) = stack.pop() else {
                        return Err("{/} with no color open to close".into());
                    };
                    st.fg = prev;
                    i = e + 1;
                }
                (Some(e), Some(n)) if theme.named(n).is_some() => {
                    stack.push(st.fg);
                    st.fg = theme.named(n);
                    i = e + 1;
                }
                _ => {
                    out.push(Cell { ch: c, st });
                    i += 1;
                }
            }
        } else {
            out.push(Cell { ch: c, st });
            i += 1;
        }
    }
    Ok(out)
}
