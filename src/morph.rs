//! From one slide's code to the next's, the way manim's
//! TransformMatchingTex does it: each token in both glides from where it was
//! to where it goes, what's gone fades out, and what's new fades in. Tokens
//! are words and single marks, so a renamed variable or an added argument
//! changes only itself, and the rest makes room.

use crate::markup::{Cell, Line, Rgb, Style};
use crate::screen::Screen;
use unicode_width::UnicodeWidthChar;

/// A part of a slide where it goes: (row, col, cells, whether it's code).
pub type Part = (i32, i32, Line, bool);

struct Tok {
    row: i32,
    col: i32,
    cells: Line,
}

impl Tok {
    fn text(&self) -> String {
        self.cells.iter().map(|c| c.ch).collect()
    }
}

/// The code's tokens: runs of letters, digits and _, and every other mark
/// on its own. Spaces aren't tokens; they're where tokens aren't.
fn tokens(parts: &[Part]) -> Vec<Tok> {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let mut out: Vec<Tok> = vec![];
    for (row, col, line, _) in parts.iter().filter(|p| p.3) {
        let mut at = *col;
        let mut was: Option<char> = None;
        for &c in line {
            if !c.ch.is_whitespace() {
                match out.last_mut() {
                    Some(t) if was.is_some_and(|w| word(w) && word(c.ch)) => t.cells.push(c),
                    _ => out.push(Tok { row: *row, col: at, cells: vec![c] }),
                }
            }
            was = Some(c.ch);
            at += c.ch.width().unwrap_or(0) as i32;
        }
    }
    out
}

/// For each new token, the old one it was: the longest run of them in the
/// same order first, then any word left with the same text, nearest first,
/// so a block moved elsewhere moves too. A lone mark left over, a `;` or a
/// `(`, fades rather than flying across the screen to another.
fn pair(old: &[Tok], new: &[Tok]) -> Vec<Option<usize>> {
    let (a, b): (Vec<String>, Vec<String>) = (old.iter().map(Tok::text).collect(), new.iter().map(Tok::text).collect());
    let (n, m) = (a.len(), b.len());
    let mut to: Vec<Option<usize>> = vec![None; m];
    // Longest common subsequence, when the table is a reasonable size.
    if n * m <= 4_000_000 {
        let mut t = vec![0u32; (n + 1) * (m + 1)];
        let at = |i: usize, j: usize| i * (m + 1) + j;
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                t[at(i, j)] = if a[i] == b[j] { t[at(i + 1, j + 1)] + 1 } else { t[at(i + 1, j)].max(t[at(i, j + 1)]) };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n && j < m {
            if a[i] == b[j] {
                to[j] = Some(i);
                (i, j) = (i + 1, j + 1);
            } else if t[at(i + 1, j)] >= t[at(i, j + 1)] {
                i += 1;
            } else {
                j += 1;
            }
        }
    }
    let mut used = vec![false; n];
    for i in to.iter().flatten() {
        used[*i] = true;
    }
    for j in 0..m {
        if to[j].is_some() {
            continue;
        }
        let far = |i: usize| (old[i].row - new[j].row).abs() * 1000 + (old[i].col - new[j].col).abs();
        let word = b[j].chars().any(|c| c.is_alphanumeric() || c == '_');
        if let Some(i) = (0..n).filter(|&i| word && !used[i] && a[i] == b[j]).min_by_key(|&i| far(i)) {
            used[i] = true;
            to[j] = Some(i);
        }
    }
    to
}

fn smooth(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: i32, b: i32, t: f64) -> i32 {
    (a as f64 + (b - a) as f64 * t).round() as i32
}

/// The cells at k of the way from a's colors to b's, over the background
/// when either is None.
fn blend(a: &[Cell], b: &[Cell], k: f64, fg: Rgb) -> Line {
    a.iter()
        .zip(b)
        .map(|(x, y)| {
            let c = x.st.fg.unwrap_or(fg).mix(y.st.fg.unwrap_or(fg), k);
            Cell { ch: y.ch, st: Style { fg: Some(c), ..y.st } }
        })
        .collect()
}

fn faded(l: &[Cell], k: f64, bg: Rgb, fg: Rgb) -> Line {
    l.iter().map(|c| Cell { ch: c.ch, st: Style { fg: Some(bg.mix(c.st.fg.unwrap_or(fg), k)), ..c.st } }).collect()
}

/// The old slide's code turning into the new one's. The new slide's other
/// parts are there from the start; the old one's are gone.
pub fn play(s: &mut Screen, old: &[Part], new: &[Part]) {
    let (a, b) = (tokens(old), tokens(new));
    let to = pair(&a, &b);
    let mut kept = vec![false; a.len()];
    for i in to.iter().flatten() {
        kept[*i] = true;
    }
    // The rows the code is on, before and after, and all between, emptied
    // each frame; the new slide's other parts on them go back after.
    let rows = a.iter().chain(&b).map(|t| t.row);
    let (top, bottom) = (rows.clone().min().unwrap_or(0), rows.max().unwrap_or(-1).min(s.h - 2));
    let still: Vec<&Part> = new.iter().filter(|p| !p.3).collect();
    for (r, c, l, _) in &still {
        s.put(*r, *c, l);
    }
    // Moved tokens start a little apart, top to bottom, so they don't all
    // go at once.
    let moved: Vec<usize> = (0..b.len()).filter(|&j| to[j].is_some()).collect();
    let lag = |j: usize| {
        let k = moved.iter().position(|&x| x == j).unwrap_or(0);
        0.2 * k as f64 / moved.len().max(1) as f64
    };
    let (bg, fg) = (s.theme.bg, s.theme.fg);
    let frames = 32;
    for f in 0..=frames {
        if s.hurry {
            break;
        }
        let t = f as f64 / frames as f64;
        s.raw("\x1b[?2026h");
        for r in top..=bottom {
            s.clear_row(r);
        }
        for (r, c, l, _) in still.iter().filter(|p| (top..=bottom).contains(&p.0)) {
            s.put(*r, *c, l);
        }
        let out = 1.0 - smooth(t / 0.35);
        if out > 0.0 {
            for tok in a.iter().zip(&kept).filter(|(_, k)| !**k).map(|(t, _)| t) {
                s.put(tok.row, tok.col, &faded(&tok.cells, out, bg, fg));
            }
        }
        let fresh = smooth((t - 0.55) / 0.45);
        if fresh > 0.0 {
            for tok in b.iter().zip(&to).filter(|(_, i)| i.is_none()).map(|(t, _)| t) {
                s.put(tok.row, tok.col, &faded(&tok.cells, fresh, bg, fg));
            }
        }
        for &j in &moved {
            let (from, tok) = (&a[to[j].unwrap()], &b[j]);
            let k = smooth((t - lag(j)) / 0.8);
            let l = blend(&from.cells, &tok.cells, k, fg);
            s.put(lerp(from.row, tok.row, k), lerp(from.col, tok.col, k), &l);
        }
        s.raw("\x1b[?2026l");
        s.tick(0.016);
    }
    for r in top..=bottom {
        s.clear_row(r);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::plain;

    fn part(row: i32, s: &str) -> Part {
        (row, 1, plain(s, Style::default()), true)
    }

    fn texts(t: &[Tok]) -> Vec<String> {
        t.iter().map(Tok::text).collect()
    }

    #[test]
    fn tokens_are_words_and_marks() {
        let t = tokens(&[part(3, "let foo_1 = f(x);")]);
        assert_eq!(texts(&t), ["let", "foo_1", "=", "f", "(", "x", ")", ";"]);
        assert_eq!((t[1].row, t[1].col), (3, 5));
    }

    #[test]
    fn a_rename_keeps_the_rest() {
        let a = tokens(&[part(1, "fn fee(order) {")]);
        let b = tokens(&[part(1, "fn price(order, tax) {")]);
        let to = pair(&a, &b);
        let kept: Vec<String> = to.iter().enumerate().filter(|(_, i)| i.is_some()).map(|(j, _)| b[j].text()).collect();
        assert_eq!(kept, ["fn", "(", "order", ")", "{"]);
    }

    #[test]
    fn a_moved_line_moves() {
        let a = tokens(&[part(1, "one()"), part(2, "two()")]);
        let b = tokens(&[part(1, "two()"), part(2, "one()")]);
        let to = pair(&a, &b);
        assert!(to.iter().all(Option::is_some));
        // two, on the first row now, came from the second.
        assert_eq!(a[to[0].unwrap()].row, 2);
    }
}
