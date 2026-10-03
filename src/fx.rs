//! Effects, by the names a talk gives them: how a headline arrives (fx), a
//! line (lines, reveal), a flourish once the slide is all there (then), and
//! how the slide before leaves (tr). Each stops waiting once a key is
//! pressed, and ends where it would have.

use crate::fine::{Movers, Plate};
use crate::markup::{self, Cell, Line, Rgb, Style};
use crate::screen::{Screen, ease};
use std::f64::consts::PI;

/// A headline where it goes, and the label over it, which some effects
/// draw again.
pub struct Art<'a> {
    pub rows: &'a [Vec<char>],
    pub aw: i32,
    pub ax: i32,
    pub arow: i32,
    pub label: Option<&'a Line>,
    pub lrow: i32,
}

const WHITE: Rgb = Rgb(255, 255, 240);

fn cells(row: &[char], st: Style) -> Line {
    row.iter().map(|&ch| Cell { ch, st }).collect()
}

impl Art<'_> {
    fn row(&self, i: usize) -> i32 {
        self.arow + i as i32
    }

    /// The headline as it ends up.
    pub fn done(&self, s: &mut Screen) {
        let st = s.accent();
        for (i, r) in self.rows.iter().enumerate() {
            s.put(self.row(i), self.ax, &cells(r, st));
        }
    }

    /// Where each of its solid blocks goes, in pixels, two to a cell each
    /// way, with the row of the headline it's in.
    fn blocks(&self) -> Vec<(f64, f64, usize)> {
        let mut out = vec![];
        for (i, r) in self.rows.iter().enumerate() {
            for (j, &ch) in r.iter().enumerate() {
                if ch == '█' {
                    out.push((2.0 * (self.ax - 1 + j as i32) as f64, 2.0 * (self.row(i) - 1) as f64, i));
                }
            }
        }
        out
    }

    /// The blocks where they go, and their shadow coming up after them.
    pub fn land(&self, s: &mut Screen) {
        let (bg, acc) = (s.theme.bg, s.theme.accent);
        for f in 1..=5 {
            if s.hurry {
                break;
            }
            let sh = Style::fg(bg.mix(acc, f as f64 / 5.0));
            for (i, r) in self.rows.iter().enumerate() {
                let l: Line = r.iter().map(|&ch| Cell { ch, st: if ch == '█' { Style::fg(acc) } else { sh } }).collect();
                s.put(self.row(i), self.ax, &l);
            }
            s.tick(0.02);
        }
        self.done(s);
    }

    fn label(&self, s: &mut Screen) {
        if let Some(l) = self.label {
            s.center(self.lrow, l);
        }
    }
}

/// The headline effects random picks from: all but none, glitch and itself.
const RANDOM: [&str; 9] = ["assemble", "drop", "fade", "iris", "scramble", "slide", "typewriter", "wipe", "zip"];

pub fn headline(s: &mut Screen, a: &Art, name: &str, n: usize) {
    match name {
        "wipe" => wipe(s, a, 12),
        "iris" => iris(s, a),
        "slide" => slide(s, a),
        "zip" => zip(s, a),
        "drop" => drop(s, a),
        "fade" => fade(s, a),
        "scramble" => scramble(s, a),
        "assemble" => assemble(s, a),
        "glitch" => glitch(s, a),
        "typewriter" => typewriter(s, a),
        "random" => headline(s, a, RANDOM[n % RANDOM.len()], n),
        _ => a.done(s),
    }
}

/// wipe: from the left, a column at a time.
fn wipe(s: &mut Screen, a: &Art, n: i32) {
    let st = s.accent();
    for f in 1..=n {
        if s.hurry {
            break;
        }
        let k = ((a.aw * f + n - 1) / n) as usize;
        for (i, r) in a.rows.iter().enumerate() {
            s.put(a.row(i), a.ax, &cells(&r[..k.min(r.len())], st));
        }
        s.tick(0.02);
    }
    a.done(s);
}

/// iris: from the middle out.
fn iris(s: &mut Screen, a: &Art) {
    let (n, mid) = (12, a.aw / 2);
    let st = s.accent();
    for f in 1..=n {
        if s.hurry {
            break;
        }
        let e = ease(f, n);
        let lo = ((mid as f64 - (mid + 1) as f64 * e) as i32).max(0);
        let hi = ((mid as f64 + (a.aw - mid) as f64 * e) as i32).min(a.aw);
        for (i, r) in a.rows.iter().enumerate() {
            if lo < hi {
                s.put(a.row(i), a.ax + lo, &cells(&r[lo as usize..hi as usize], st));
            }
        }
        s.tick(0.02);
    }
    a.done(s);
}

/// slide: in from off the left edge, a quarter of a cell at a time.
fn slide(s: &mut Screen, a: &Art) {
    let (n, st) = (22, s.accent());
    let run = 2.0 * (a.ax - 1 + a.aw) as f64;
    let blocks = a.blocks();
    let mut p = Plate::default();
    for f in 1..=n {
        if s.hurry {
            break;
        }
        let dx = run * (1.0 - ease(f, n));
        p.draw(s, blocks.iter().map(|&(x, y, _)| (x - dx, y)), st);
        s.tick(0.014);
    }
    p.clear(s);
    a.land(s);
}

/// zip: the rows in from both edges at once, alternating.
fn zip(s: &mut Screen, a: &Art) {
    let (n, st) = (22, s.accent());
    let (left, right) = (2.0 * (a.ax - 1 + a.aw) as f64, 2.0 * (s.w - a.ax + 1) as f64);
    let blocks = a.blocks();
    let mut p = Plate::default();
    for f in 1..=n {
        if s.hurry {
            break;
        }
        let k = 1.0 - ease(f, n);
        p.draw(s, blocks.iter().map(|&(x, y, i)| (if i % 2 == 0 { x - left * k } else { x + right * k }, y)), st);
        s.tick(0.014);
    }
    p.clear(s);
    a.land(s);
}

/// drop: the rows fall from the top, the bottom one first, and stack.
fn drop(s: &mut Screen, a: &Art) {
    let (n, d, k, st) = (16, 3, a.rows.len() as i32, s.accent());
    let blocks = a.blocks();
    let mut p = Plate::default();
    for f in 1..=n + d * (k - 1) {
        if s.hurry {
            break;
        }
        // Each row from the top of the screen, once its turn comes.
        let fall = |i: usize| {
            let t = f - d * (k - 1 - i as i32);
            (t > 0).then(|| ease(t.min(n), n))
        };
        p.draw(s, blocks.iter().filter_map(|&(x, y, i)| fall(i).map(|e| (x, y * e))), st);
        a.label(s);
        s.tick(0.016);
    }
    p.clear(s);
    a.label(s);
    a.land(s);
}

/// fade: up out of the background.
fn fade(s: &mut Screen, a: &Art) {
    let n = 14;
    for f in 1..=n {
        if s.hurry {
            break;
        }
        let st = Style::fg(s.theme.bg.mix(s.theme.accent, f as f64 / n as f64));
        for (i, r) in a.rows.iter().enumerate() {
            s.put(a.row(i), a.ax, &cells(r, st));
        }
        s.tick(0.03);
    }
    a.done(s);
}

/// scramble: noise that settles into the letters, a cell at a time.
fn scramble(s: &mut Screen, a: &Art) {
    let n = 18;
    let noise = ['█', '▓', '▒', '░', '╔', '╗', '╚', '╝', '═', '║', '╬', '▀', '▄'];
    let (acc, mut_) = (s.accent(), s.muted());
    s.rng.seed(a.aw as u32);
    let at: Vec<Vec<i32>> = a.rows.iter().map(|r| r.iter().map(|_| s.rng.below(n) + 1).collect()).collect();
    for f in 1..=n {
        if s.hurry {
            break;
        }
        for (i, r) in a.rows.iter().enumerate() {
            let l: Line = r
                .iter()
                .enumerate()
                .map(|(j, &ch)| {
                    if ch != ' ' && at[i][j] > f {
                        Cell { ch: noise[s.rng.below(noise.len() as i32) as usize], st: mut_ }
                    } else {
                        Cell { ch, st: acc }
                    }
                })
                .collect();
            s.put(a.row(i), a.ax, &l);
        }
        s.tick(0.03);
    }
    a.done(s);
}

/// assemble: every block flies in from somewhere on the screen.
fn assemble(s: &mut Screen, a: &Art) {
    let (n, st) = (26, s.accent());
    s.rng.seed(a.aw as u32);
    let blocks: Vec<(f64, f64, f64, f64)> = a
        .blocks()
        .into_iter()
        .map(|(x, y, _)| (x, y, s.rng.below(2 * s.w) as f64, s.rng.below(2 * (s.h - 2)) as f64))
        .collect();
    let mut p = Plate::default();
    for f in 1..=n {
        if s.hurry {
            break;
        }
        let e = ease(f, n);
        p.draw(s, blocks.iter().map(|&(x, y, sx, sy)| (sx + (x - sx) * e, sy + (y - sy) * e)), st);
        s.tick(0.018);
    }
    p.clear(s);
    a.label(s);
    a.land(s);
}

/// glitch: wipes in, tears and breaks up, goes dark a moment, and comes
/// back. The noise is seeded, so it is the same every time.
fn glitch(s: &mut Screen, a: &Art) {
    wipe(s, a, 12);
    let noise = ['░', '▒', '▓', '█', '╳', ' '];
    s.rng.seed(7);
    for f in 1..=9 {
        if s.hurry {
            break;
        }
        let st = Style::fg(if f % 2 == 1 { s.theme.bad } else { s.theme.accent });
        for (i, r) in a.rows.iter().enumerate() {
            let o: Vec<char> =
                r.iter().map(|&c| if s.rng.below(100) < 6 + f * 5 { noise[s.rng.below(6) as usize] } else { c }).collect();
            let sh = s.rng.below(9) - 4;
            s.at(a.row(i), a.ax + sh, &cells(&o, st));
        }
        s.tick(0.05);
    }
    if !s.hurry {
        for r in 1..=s.h - 2 {
            s.clear_row(r);
        }
        s.tick(0.6);
    }
    wipe(s, a, 8);
}

/// typewriter: typed a column at a time with a block cursor riding the edge.
fn typewriter(s: &mut Screen, a: &Art) {
    let (acc, warm) = (s.accent(), s.warm());
    let mut f = 1;
    while f <= a.aw && !s.hurry {
        for (i, r) in a.rows.iter().enumerate() {
            let mut l = cells(&r[..f as usize], acc);
            l.push(Cell { ch: '▌', st: warm });
            s.put(a.row(i), a.ax, &l);
        }
        s.tick(0.012);
        f += 2;
    }
    for i in 0..a.rows.len() {
        s.clear_row(a.row(i));
    }
    a.done(s);
}

/// Lines decoding together: noise settling into every line at once, left to
/// right, so a block of code takes as long as one line would.
pub fn lines_scramble(s: &mut Screen, lines: &[(i32, &Line)]) {
    let noise: Vec<char> = "abcdefghkmnpqrstuvwxyz0123456789#%&*+=?".chars().collect();
    let st = s.muted();
    let big = 10;
    for f in 1..=big {
        if s.hurry {
            break;
        }
        for (row, t) in lines {
            let p = markup::bare(t);
            let (col, n) = (s.mid(markup::width(t)), p.len() as i32);
            let o: Line = p
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let ch = if c.ch == ' ' || (i as i32 + 1) * big <= f * n {
                        c.ch
                    } else {
                        noise[s.rng.below(noise.len() as i32) as usize]
                    };
                    Cell { ch, st }
                })
                .collect();
            s.put(*row, col, &o);
        }
        s.tick(0.02);
    }
    for (row, t) in lines {
        let col = s.mid(markup::width(t));
        s.put(*row, col, t);
    }
}

/// A line coming in, centered on its row.
pub fn line_in(s: &mut Screen, name: &str, row: i32, t: &Line) {
    let p = markup::bare(t);
    let col = s.mid(markup::width(t));
    if markup::text(&p).trim().is_empty() {
        s.put(row, col, t);
        return;
    }
    let n = p.len();
    match name {
        // type: typed out, a few letters a frame.
        "type" => {
            let mut nc = 1;
            while nc < n && !s.hurry {
                s.put(row, col, &p[..nc]);
                s.tick(0.008);
                nc += n.div_ceil(12).max(1);
            }
        }
        // glide: in from the right edge, easing into place.
        "glide" => {
            let mut mv = Movers::new(s, &[t]);
            let n = 16;
            for f in 1..=n {
                if s.hurry {
                    break;
                }
                let c = s.w as f64 - (s.w - col) as f64 * ease(f, n);
                s.clear_row(row);
                mv.at(0, row as f64, c, p.clone());
                mv.show(s);
                s.tick(0.01);
            }
            mv.done(s);
            s.clear_row(row);
        }
        // fade: up out of the background, then in its colors.
        "fade" => {
            for f in 1..=10 {
                if s.hurry {
                    break;
                }
                let c = s.theme.bg.mix(s.theme.fg, f as f64 / 10.0);
                s.put(row, col, &markup::recolor(&p, c));
                s.tick(0.03);
            }
        }
        // scramble: noise settling into the letters, left to right.
        "scramble" => {
            let noise: Vec<char> = "abcdefghkmnpqrstuvwxyz0123456789#%&*+=?".chars().collect();
            let st = s.muted();
            let big = 14;
            for f in 1..=big {
                if s.hurry {
                    break;
                }
                let o: Line = p
                    .iter()
                    .enumerate()
                    .map(|(i, c)| {
                        let ch = if c.ch == ' ' || (i as i32 + 1) * big <= f * n as i32 {
                            c.ch
                        } else {
                            noise[s.rng.below(noise.len() as i32) as usize]
                        };
                        Cell { ch, st }
                    })
                    .collect();
                s.put(row, col, &o);
                s.tick(0.03);
            }
        }
        // count: its numbers counting up from 0.
        "count" => {
            let big = 20;
            for f in 1..=big {
                if s.hurry {
                    break;
                }
                let mut o = counted(t, ease(f, big));
                let w = markup::width(t) - markup::width(&o);
                o.extend(std::iter::repeat_n(Cell { ch: ' ', st: Style::default() }, w.max(0) as usize));
                s.put(row, col, &o);
                s.tick(0.04);
            }
        }
        _ => {}
    }
    s.put(row, col, t);
}

/// The line with each run of digits at e of its value.
fn counted(t: &Line, e: f64) -> Line {
    let mut out = Line::new();
    let mut i = 0;
    while i < t.len() {
        if t[i].ch.is_ascii_digit() {
            let j = (i..t.len()).find(|&j| !t[j].ch.is_ascii_digit()).unwrap_or(t.len());
            let v: f64 = markup::text(&t[i..j]).parse().unwrap_or(0.0);
            out.extend(markup::plain(&((v * e) as u64).to_string(), t[i].st));
            i = j;
        } else {
            out.push(t[i]);
            i += 1;
        }
    }
    out
}

pub fn then(s: &mut Screen, a: &Art, name: &str) {
    match name {
        "shine" => shine(s, a),
        "pulse" => pulse(s, a),
        "shake" => shake(s, a),
        "rainbow" => rainbow(s, a),
        "sparkle" => sparkle(s, a),
        "confetti" => confetti(s, 60),
        _ => {}
    }
}

/// shine: a glint running across the headline.
fn shine(s: &mut Screen, a: &Art) {
    let n = 18;
    let from = s.theme.accent;
    let c = [Style::fg(from), Style::fg(from.mix(WHITE, 0.4)), Style::fg(from.mix(WHITE, 0.8))];
    for f in 0..=n {
        if s.hurry {
            break;
        }
        let at = -8 + (a.aw + 16) * f / n;
        for (i, r) in a.rows.iter().enumerate() {
            let l: Line = r
                .iter()
                .enumerate()
                .map(|(j, &ch)| {
                    let d = (j as i32 + 1 + (i as i32 + 1) * 2 - at).abs();
                    Cell { ch, st: c[if d <= 1 { 2 } else if d == 2 { 1 } else { 0 }] }
                })
                .collect();
            s.put(a.row(i), a.ax, &l);
        }
        s.tick(0.02);
    }
    a.done(s);
}

/// pulse: the headline brightening and back.
fn pulse(s: &mut Screen, a: &Art) {
    let n = 12;
    for f in 1..=n {
        if s.hurry {
            break;
        }
        let st = Style::fg(s.theme.accent.mix(WHITE, (PI * f as f64 / n as f64).sin() * 0.8));
        for (i, r) in a.rows.iter().enumerate() {
            s.put(a.row(i), a.ax, &cells(r, st));
        }
        s.tick(0.03);
    }
    a.done(s);
}

/// shake: the headline jolted side to side, settling.
fn shake(s: &mut Screen, a: &Art) {
    let (n, st) = (24, s.accent());
    let blocks = a.blocks();
    let mut p = Plate::default();
    for (i, r) in a.rows.iter().enumerate() {
        s.put(a.row(i), a.ax, &markup::plain(&" ".repeat(r.len()), Style::default()));
    }
    for f in 0..=n {
        if s.hurry {
            break;
        }
        let t = f as f64 / n as f64;
        let dx = 8.0 * (1.0 - t).powi(2) * (t * 6.0 * PI).sin();
        p.draw(s, blocks.iter().map(|&(x, y, _)| (x + dx, y)), st);
        s.tick(0.014);
    }
    p.clear(s);
    a.land(s);
}

/// rainbow: colors running through the headline, then back to itself.
fn rainbow(s: &mut Screen, a: &Art) {
    let hue = [
        Rgb(250, 189, 47),
        Rgb(254, 128, 25),
        Rgb(251, 73, 52),
        Rgb(211, 134, 155),
        Rgb(131, 165, 152),
        Rgb(142, 192, 124),
        Rgb(184, 187, 38),
    ];
    let n = 28;
    for f in 1..=n {
        if s.hurry {
            break;
        }
        for (i, r) in a.rows.iter().enumerate() {
            let l: Line = r
                .iter()
                .enumerate()
                .map(|(j, &ch)| {
                    let k = ((j as i32 + 1) / 3 + i as i32 + 1 - f + 1000) % hue.len() as i32;
                    Cell { ch, st: Style::fg(hue[k as usize]) }
                })
                .collect();
            s.put(a.row(i), a.ax, &l);
        }
        s.tick(0.03);
    }
    a.done(s);
}

/// sparkle: glints coming and going around the headline.
fn sparkle(s: &mut Screen, a: &Art) {
    let glint = ['·', '✧', '✦', '✧', '·'];
    let st = s.accent();
    let k = a.rows.len() as i32;
    let mut sp: Vec<(i32, i32, i32)> = vec![];
    for f in 1..=36 {
        if s.hurry {
            break;
        }
        if f <= 26 {
            for _ in 0..2 {
                let i = a.arow - 2 + s.rng.below(k + 3);
                let j = a.ax - 4 + s.rng.below(a.aw + 8);
                // Only where the headline has nothing.
                if i >= a.arow && i < a.arow + k && j >= a.ax && j < a.ax + a.aw && a.rows[(i - a.arow) as usize][(j - a.ax) as usize] != ' '
                {
                    continue;
                }
                sp.push((i, j, 0));
            }
        }
        for g in sp.iter_mut() {
            if g.2 < 0 {
                continue;
            }
            g.2 += 1;
            if g.2 > glint.len() as i32 {
                s.put_str(g.0, g.1, " ", Style::default());
                g.2 = -1;
            } else {
                s.put(g.0, g.1, &[Cell { ch: glint[g.2 as usize - 1], st }]);
            }
        }
        s.tick(0.04);
    }
    for g in sp.iter().filter(|g| g.2 >= 0) {
        s.put_str(g.0, g.1, " ", Style::default());
    }
}

/// confetti: falling the height of the screen. What it falls through is left
/// blank; the slide is drawn again after.
fn confetti(s: &mut Screen, n: usize) {
    let glyph = ['▪', '●', '◆', '▲', '■', '✦', '★', '▬'];
    let t = &s.theme;
    let col = [t.accent, t.good, t.bad, t.warm, t.link];
    let mut ps: Vec<(i32, i32, i32, Rgb, char)> = (0..n)
        .map(|_| {
            (
                -s.rng.below(s.h),
                s.rng.below(s.w) + 1,
                s.rng.below(2) + 1,
                col[s.rng.below(col.len() as i32) as usize],
                glyph[s.rng.below(glyph.len() as i32) as usize],
            )
        })
        .collect();
    let mut erase: Vec<(i32, i32)> = vec![];
    for _ in 1..=s.h + 20 {
        if s.hurry {
            break;
        }
        for &(r, c) in &erase {
            s.put_str(r, c, " ", Style::default());
        }
        erase.clear();
        for p in ps.iter_mut() {
            p.0 += p.2;
            if s.rng.below(4) == 0 {
                p.1 += s.rng.below(3) - 1;
            }
            if p.0 >= 1 && p.0 <= s.h - 2 && p.1 >= 1 && p.1 <= s.w {
                s.put(p.0, p.1, &[Cell { ch: p.4, st: Style::fg(p.3) }]);
                erase.push((p.0, p.1));
            }
        }
        s.tick(0.03);
    }
    for &(r, c) in &erase {
        s.put_str(r, c, " ", Style::default());
    }
}

/// The slide before going, before this one comes.
pub fn transition(s: &mut Screen, name: &str) {
    let rows = s.h - 2;
    match name {
        // dissolve: the screen falling away a cell at a time.
        "dissolve" => {
            let o = s.w * rows / 5;
            for _ in 1..=8 {
                if s.hurry {
                    break;
                }
                for _ in 0..o {
                    let (r, c) = (s.rng.below(rows) + 1, s.rng.below(s.w) + 1);
                    s.put_str(r, c, " ", Style::default());
                }
                s.tick(0.03);
            }
        }
        // sweep: a bar crossing the screen, leaving it empty.
        "sweep" => {
            let (n, mut was) = (14, 0);
            let st = s.accent();
            for f in 1..=n {
                if s.hurry {
                    break;
                }
                let c = (s.w as f64 * ease(f, n)) as i32;
                for r in 1..=rows {
                    let mut l = markup::plain(&" ".repeat((c - was).max(0) as usize), Style::default());
                    l.push(Cell { ch: '█', st });
                    s.put(r, was + 1, &l);
                }
                was = c;
                s.tick(0.02);
            }
        }
        // curtain: bars from both sides, meeting in the middle.
        "curtain" => {
            let (n, mut was, half) = (12, 0, (s.w + 1) / 2);
            let st = s.muted();
            for f in 1..=n {
                if s.hurry {
                    break;
                }
                let c = (half as f64 * ease(f, n)) as i32;
                let gap = " ".repeat((c - was).max(0) as usize);
                for r in 1..=rows {
                    let mut l = markup::plain(&gap, Style::default());
                    l.push(Cell { ch: '▌', st });
                    s.put(r, was + 1, &l);
                    let mut l = vec![Cell { ch: '▐', st }];
                    l.extend(markup::plain(&gap, Style::default()));
                    s.put(r, s.w - c, &l);
                }
                was = c;
                s.tick(0.025);
            }
        }
        _ => {}
    }
}
