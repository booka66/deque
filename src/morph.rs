//! From one slide to the next, the way manim's TransformMatchingTex does
//! it: each token in both swings from where it was to where it goes, what's
//! gone drifts off and fades, and what's new rises in. Tokens are words and
//! single marks, so a renamed variable or an added argument changes only
//! itself, and the rest makes room. A headline's blocks flock from the old
//! word into the new one.

use crate::fine::{Movers, Plate, faded};
use crate::markup::{Cell, Line, Rgb, Style};
use crate::screen::Screen;
use unicode_width::UnicodeWidthChar;

/// What a part of a slide does as it turns into the next: stays, its
/// tokens move, or it's a headline, whose blocks do.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Role {
    Still,
    Moves,
    Art,
}

/// A part of a slide where it goes: (row, col, cells, what it does).
pub type Part = (i32, i32, Line, Role);

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
    for (row, col, line, _) in parts.iter().filter(|p| p.3 == Role::Moves) {
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

/// Out to where it goes, a little past, and back: a spring settling.
fn spring(t: f64) -> f64 {
    match t {
        t if t <= 0.0 => 0.0,
        t if t >= 1.0 => 1.0,
        t => 1.0 - (-7.0 * t).exp() * (9.0 * t).cos(),
    }
}

/// From a to b, k of the way along a curve bowed off the straight line by
/// a tenth of its length, upward where it can, so things swing rather than
/// slide. Rows count double, a cell being twice as tall as wide.
fn arc(a: (f64, f64), b: (f64, f64), k: f64) -> (f64, f64) {
    let (dr, dc) = (b.0 - a.0, b.1 - a.1);
    let (x, y) = (dc, 2.0 * dr);
    let len = (x * x + y * y).sqrt();
    let (r, c) = (a.0 + dr * k, a.1 + dc * k);
    if len < 1.0 {
        return (r, c);
    }
    // At right angles to the way it goes, upward, or rightward going
    // straight up or down.
    let (mut px, mut py) = (-y / len, x / len);
    if py > 0.0 || (py == 0.0 && px < 0.0) {
        (px, py) = (-px, -py);
    }
    let bow = 0.1 * len * (std::f64::consts::PI * k.clamp(0.0, 1.0)).sin();
    (r + py * bow / 2.0, c + px * bow)
}

fn lerp(a: i32, b: i32, t: f64) -> f64 {
    a as f64 + (b - a) as f64 * t
}

/// The cells at k of the way from a's colors to b's, over the background
/// when either is None.
fn blend(a: &[Cell], b: &[Cell], k: f64, fg: Rgb) -> Line {
    a.iter()
        .zip(b)
        .map(|(x, y)| {
            let c = x.st.fg.unwrap_or(fg).mix(y.st.fg.unwrap_or(fg), k.clamp(0.0, 1.0));
            Cell { ch: y.ch, st: Style { fg: Some(c), ..y.st } }
        })
        .collect()
}

/// A headline's solid blocks, in pixels, two to a cell each way.
fn blocks(parts: &[Part]) -> Vec<(f64, f64)> {
    let mut out = vec![];
    for (row, col, line, role) in parts {
        if *role != Role::Art {
            continue;
        }
        let mut c = *col;
        for cell in line {
            if cell.ch == '█' {
                out.push((2.0 * (c - 1) as f64, 2.0 * (row - 1) as f64));
            }
            c += cell.ch.width().unwrap_or(0) as i32;
        }
    }
    out
}

/// Where each block of one headline goes in the next, and where each of
/// the next's comes from: each to the nearest in the other, measured
/// within each headline's own bounds, so a letter's left edge finds the
/// next letter's. A block with none coming splits one off its nearest; a
/// headline with none at all grows from, or shrinks to, the other's middle.
fn flock(a: &[(f64, f64)], b: &[(f64, f64)]) -> Vec<((f64, f64), (f64, f64))> {
    let bounds = |v: &[(f64, f64)]| {
        let (x0, x1) = v.iter().fold((f64::MAX, f64::MIN), |m, p| (m.0.min(p.0), m.1.max(p.0)));
        let (y0, y1) = v.iter().fold((f64::MAX, f64::MIN), |m, p| (m.0.min(p.1), m.1.max(p.1)));
        (x0, y0, (x1 - x0).max(1.0), (y1 - y0).max(1.0))
    };
    let mid = |v: &[(f64, f64)]| {
        let (x0, x1) = v.iter().fold((f64::MAX, f64::MIN), |m, p| (m.0.min(p.0), m.1.max(p.0)));
        let (y0, y1) = v.iter().fold((f64::MAX, f64::MIN), |m, p| (m.0.min(p.1), m.1.max(p.1)));
        ((x0 + x1) / 2.0, (y0 + y1) / 2.0)
    };
    match (a.is_empty(), b.is_empty()) {
        (true, true) => return vec![],
        (true, false) => return b.iter().map(|&p| (mid(b), p)).collect(),
        (false, true) => return a.iter().map(|&p| (p, mid(a))).collect(),
        _ => {}
    }
    let (ba, bb) = (bounds(a), bounds(b));
    let na: Vec<(f64, f64)> = a.iter().map(|p| ((p.0 - ba.0) / ba.2, (p.1 - ba.1) / ba.3)).collect();
    let nb: Vec<(f64, f64)> = b.iter().map(|p| ((p.0 - bb.0) / bb.2, (p.1 - bb.1) / bb.3)).collect();
    let near = |p: (f64, f64), v: &[(f64, f64)]| {
        (0..v.len()).min_by(|&i, &j| {
            let d = |q: (f64, f64)| (p.0 - q.0).powi(2) + (p.1 - q.1).powi(2);
            d(v[i]).total_cmp(&d(v[j]))
        })
    };
    let mut out = vec![];
    let mut fed = vec![false; b.len()];
    for (i, &p) in na.iter().enumerate() {
        let j = near(p, &nb).unwrap();
        fed[j] = true;
        out.push((a[i], b[j]));
    }
    for (j, &q) in nb.iter().enumerate() {
        if !fed[j] {
            out.push((a[near(q, &na).unwrap()], b[j]));
        }
    }
    out
}

/// The old slide turning into the new one: the tokens of the parts that
/// move (code, or with `tr: morph` every line) finding their new places,
/// and the headline's blocks flocking into the new headline's. The new
/// slide's other parts are there from the start; the old one's are gone.
pub fn play(s: &mut Screen, old: &[Part], new: &[Part]) {
    let (a, b) = (tokens(old), tokens(new));
    let to = pair(&a, &b);
    let mut kept = vec![false; a.len()];
    for i in to.iter().flatten() {
        kept[*i] = true;
    }
    let flight = flock(&blocks(old), &blocks(new));
    // Every row that moves, before and after, and a margin for the curves,
    // emptied each frame; the new slide's other parts on them go back after.
    let rows = old.iter().chain(new).filter(|p| p.3 != Role::Still).map(|p| p.0);
    let top = (rows.clone().min().unwrap_or(1) - 2).max(1);
    let bottom = (rows.max().unwrap_or(0) + 2).min(s.h - 2);
    let still: Vec<&Part> = new.iter().filter(|p| p.3 == Role::Still).collect();
    for (r, c, l, _) in &still {
        s.put(*r, *c, l);
    }
    // Tokens in both: those that stay put change color where they are; the
    // rest move, finer than a cell where the terminal can, starting a
    // little apart, top to bottom, so they don't all go at once.
    let here = |j: usize| to[j].is_some_and(|i| (a[i].row, a[i].col) == (b[j].row, b[j].col));
    let stay: Vec<usize> = (0..b.len()).filter(|&j| here(j)).collect();
    let moved: Vec<usize> = (0..b.len()).filter(|&j| to[j].is_some() && !here(j)).collect();
    let lag = |m: usize| 0.2 * m as f64 / moved.len().max(1) as f64;
    // Blocks go left to right, a wave through the headline.
    let (x0, x1) = flight.iter().fold((f64::MAX, f64::MIN), |m, (_, q)| (m.0.min(q.0), m.1.max(q.0)));
    let wave = |x: f64| 0.15 * (x - x0) / (x1 - x0).max(1.0);
    let mut plate = Plate::default();
    let lines: Vec<&[Cell]> = moved.iter().map(|&j| &b[j].cells[..]).collect();
    let mut mv = Movers::new(s, &lines);
    let (bg, fg, acc) = (s.theme.bg, s.theme.fg, s.accent());
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
        plate.draw(
            s,
            flight.iter().map(|&(p, q)| {
                let k = spring((t - wave(q.0)) / 0.8);
                let (y, x) = arc((p.1, p.0), (q.1, q.0), k);
                (x, y)
            }),
            acc,
        );
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
        for &j in &stay {
            let (from, tok) = (&a[to[j].unwrap()], &b[j]);
            s.put(tok.row, tok.col, &blend(&from.cells, &tok.cells, smooth(t), fg));
        }
        for (m, &j) in moved.iter().enumerate() {
            let (from, tok) = (&a[to[j].unwrap()], &b[j]);
            let k = smooth((t - lag(m)) / 0.8);
            mv.at(m, lerp(from.row, tok.row, k), lerp(from.col, tok.col, k), blend(&from.cells, &tok.cells, k, fg));
        }
        mv.show(s);
        s.raw("\x1b[?2026l");
        s.tick(0.016);
    }
    // Ending on the new slide as it is, but for the headline, which lands
    // after.
    mv.done(s);
    plate.clear(s);
    for r in top..=bottom {
        s.clear_row(r);
    }
    for (r, c, l, _) in new.iter().filter(|p| p.3 != Role::Art) {
        s.put(*r, *c, l);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::plain;

    fn part(row: i32, s: &str) -> Part {
        (row, 1, plain(s, Style::default()), Role::Moves)
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
    fn every_block_comes_from_somewhere() {
        let a = [(0.0, 0.0), (2.0, 0.0)];
        let b = [(0.0, 0.0), (4.0, 0.0), (8.0, 0.0)];
        let f = flock(&a, &b);
        assert!(b.iter().all(|q| f.iter().any(|(_, t)| t == q)));
        assert!(a.iter().all(|p| f.iter().any(|(s, _)| s == p)));
        // A headline from nothing grows from the new one's middle.
        assert!(flock(&[], &b).iter().all(|(s, _)| *s == (4.0, 0.0)));
    }

    #[test]
    fn curves_end_where_they_go() {
        let (a, b) = ((3.0, 10.0), (9.0, 40.0));
        assert_eq!(arc(a, b, 0.0), a);
        let (r, c) = arc(a, b, 1.0);
        assert!((r - b.0).abs() < 1e-9 && (c - b.1).abs() < 1e-9);
        // Halfway it's off the straight line, above it.
        let (r, _) = arc(a, b, 0.5);
        assert!(r < 6.0);
        assert_eq!(spring(1.0), 1.0);
        assert!((0.0..1.3).contains(&spring(0.4)));
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
