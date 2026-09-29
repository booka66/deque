//! Slides as the deck draws them. Every text slide has one layout, so
//! nothing jumps between them: the label, the headline, a short rule and the
//! lines, as one block in the middle.

use crate::fx::{self, Art};
use crate::images::Pictures;
use crate::markup::{self, Cell, Line, Style};
use crate::morph::{self, Part, Role};
use crate::screen::Screen;
use crate::sky::Kind;
use crate::talk::{Draw, Item, Slide, Talk};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// The slide as it is.
    Still,
    /// Arriving, each part drawing itself in turn.
    Arrive,
    /// Only the latest step coming in.
    Step,
    /// Arriving from the slide given, its code turning into this one's.
    Morph(usize),
}

/// Slide n's label, numbered from 00: "03 · W H Y".
pub fn label(s: &Screen, slide: &Slide, n: usize) -> Option<Line> {
    let t = slide.label.as_ref()?.to_uppercase();
    let spaced: Vec<String> = t.chars().map(String::from).collect();
    Some(markup::plain(&format!("{:02} · {}", n, spaced.join(" ")), s.muted()))
}

/// The screen empty but for the slides' dots along the bottom, or, when
/// they wouldn't fit, a bar as far along as the talk is.
pub fn clear(s: &mut Screen, talk: &Talk, n: usize) {
    s.clear();
    let (acc, mut_) = (s.accent(), s.muted());
    let total = talk.slides.len() as i32;
    if total * 2 + 3 > s.w {
        let w = (s.w / 2).max(4);
        let done = ((n as i32 + 1) * w + total - 1) / total;
        let mut d = markup::plain(&"━".repeat(done as usize), acc);
        d.extend(markup::plain(&"─".repeat((w - done) as usize), mut_));
        s.put(s.h - 1, s.mid(w), &d);
        return;
    }
    let mut d = Line::new();
    for i in 0..talk.slides.len() {
        let (ch, st) = if i < n { ('●', mut_) } else if i == n { ('●', acc) } else { ('·', mut_) };
        d.push(Cell { ch, st });
        d.push(Cell { ch: ' ', st: Style::default() });
    }
    let col = (s.w - talk.slides.len() as i32 * 2 + 1) / 2 + 1;
    s.put(s.h - 1, col, &d);
}

/// Slide n (0-based) with its first `shown` steps.
pub fn draw(s: &mut Screen, talk: &Talk, pics: &mut Pictures, n: usize, mode: Mode, shown: usize, hint: bool) {
    let slide = &talk.slides[n];
    // Pictures are drawn by the terminal, over whatever a sky would do.
    match slide.images.is_empty() {
        true => s.backdrop(Kind::from(&talk.sky(slide)), talk.glow(slide)),
        false => s.backdrop(Kind::None, false),
    }
    if !slide.images.is_empty() {
        pictures(s, talk, pics, n);
    } else if let Some(d) = &slide.draw {
        drawn(s, talk, n, d, mode, shown);
    } else {
        text(s, talk, n, mode, shown);
    }
    // Before the talk starts: how to size it, in the corner, faint.
    if hint && n == 0 {
        let st = s.muted();
        s.put_str(s.h, 2, "+ − size", st);
    }
}

/// Where a text slide's parts go: its label's row, its headline's, its
/// rule's (0 for none) and its first line's.
fn layout(s: &Screen, slide: &Slide) -> (i32, i32, i32, i32) {
    let ah = slide.art.len() as i32;
    let top = ((s.h - 16) / 2).max(1);
    let (arow, mut rrow, mut brow) = match slide.label {
        Some(_) => (top + 2, top + ah + 3, top + ah + 5),
        None => (top + 1, 0, top + ah + 3),
    };
    if slide.art.is_empty() {
        (rrow, brow) = (0, arow);
    }
    (top, arow, rrow, brow)
}

/// Where each part of text slide n goes, with its first `shown` steps, and
/// what it does in a morph: code moves, and with `whole` everything does.
fn placed(s: &Screen, talk: &Talk, n: usize, shown: usize, whole: bool) -> Vec<Part> {
    let slide = &talk.slides[n];
    let (lrow, arow, rrow, brow) = layout(s, slide);
    let (moves, art) = if whole { (Role::Moves, Role::Art) } else { (Role::Still, Role::Still) };
    let mut out: Vec<Part> = vec![];
    if let Some(l) = label(s, slide, n) {
        out.push((lrow, s.mid(markup::width(&l)), l, moves));
    }
    let aw = slide.art.first().map_or(0, |r| r.len() as i32);
    for (i, r) in slide.art.iter().enumerate() {
        out.push((arow + i as i32, (s.w - aw) / 2 + 1, markup::plain(&r.iter().collect::<String>(), s.accent()), art));
    }
    if rrow > 0 {
        out.push((rrow, s.mid(8), markup::plain("━━━━━━━━", s.accent()), Role::Still));
    }
    for (i, b) in slide.body.iter().enumerate().filter(|(_, b)| b.step <= shown) {
        let role = if b.code { Role::Moves } else { moves };
        out.push((brow + i as i32, s.mid(markup::width(&b.line)), b.line.clone(), role));
    }
    out
}

/// How slide n arrives from the one before it: its code turning into
/// n's, when both have code, or played in.
pub fn arrive(talk: &Talk, n: usize) -> Mode {
    if n > 0 && talk.morphs(n - 1, n) { Mode::Morph(n - 1) } else { Mode::Arrive }
}

/// Where a slide's run block's output goes: under its lines, a row between,
/// from the block's left edge.
pub fn under(s: &Screen, slide: &Slide) -> Option<(i32, i32)> {
    let r = slide.run.as_ref()?;
    let (.., brow) = layout(s, slide);
    let w = slide.body.get(r.at).map_or(0, |b| markup::width(&b.line));
    Some((brow + slide.body.len() as i32 + 1, s.mid(w)))
}

/// Where a drawing's top left goes.
fn canvas(s: &Screen, d: &Draw) -> (i32, i32) {
    (((s.h - d.h) / 2).max(1), ((s.w - d.w) / 2 + 1).max(1))
}

/// How lit body line i is with `shown` steps: dim when a focus step of its
/// block is on and it isn't one of the lines lit.
fn lit(slide: &Slide, i: usize, shown: usize) -> f64 {
    match slide.focus.iter().rfind(|f| f.step <= shown && f.block.contains(&i)) {
        Some(f) if !f.lines.contains(&i) => 0.3,
        _ => 1.0,
    }
}

/// Body line i as it's shown with `shown` steps: dimmed, if focus says.
fn body_line(s: &Screen, slide: &Slide, i: usize, shown: usize) -> Line {
    let k = lit(slide, i, shown);
    let l = &slide.body[i].line;
    if k < 1.0 { crate::fine::faded(l, k, s.theme.bg, s.theme.fg) } else { l.clone() }
}

fn text(s: &mut Screen, talk: &Talk, n: usize, mode: Mode, shown: usize) {
    let slide = &talk.slides[n];
    let label = label(s, slide, n);
    let art = &slide.art;
    let aw = art.first().map_or(0, |r| r.len() as i32);
    let (lrow, arow, rrow, brow) = layout(s, slide);
    let rule = markup::plain("━━━━━━━━", s.accent());
    let a = Art { rows: art, aw, ax: (s.w - aw) / 2 + 1, arow, label: label.as_ref(), lrow };
    let seen = |b: &crate::talk::Body| b.step <= shown;
    if mode == Mode::Step {
        // A focus step: the block's lines easing to their new brightness.
        if let Some(f) = slide.focus.iter().find(|f| f.step == shown) {
            let frames = 10;
            for k in 1..=frames {
                if s.hurry {
                    break;
                }
                let e = crate::screen::ease(k, frames);
                for i in f.block.clone() {
                    let (was, now) = (lit(slide, i, shown - 1), lit(slide, i, shown));
                    let l = crate::fine::faded(&slide.body[i].line, was + (now - was) * e, s.theme.bg, s.theme.fg);
                    s.center(brow + i as i32, &l);
                }
                s.tick(0.02);
            }
            for i in f.block.clone() {
                let l = body_line(s, slide, i, shown);
                s.center(brow + i as i32, &l);
            }
            return;
        }
        if let Some((i, b)) = slide.body.iter().enumerate().find(|(_, b)| b.step == shown) {
            let row = brow + i as i32;
            s.clear_row(row);
            fx::line_in(s, &talk.reveal(slide), row, &b.line);
        }
        return;
    }
    clear(s, talk, n);
    if mode == Mode::Arrive {
        if let Some(l) = &label {
            fx::line_in(s, "type", lrow, l);
        }
        if !art.is_empty() {
            fx::headline(s, &a, &talk.fx(slide), n + 1);
        }
        if rrow > 0 {
            for f in 1..=8 {
                if s.hurry {
                    break;
                }
                s.center(rrow, &rule[..f]);
                s.tick(0.015);
            }
        }
        let how = talk.lines(slide);
        for (i, b) in slide.body.iter().enumerate().filter(|(_, b)| seen(b) && b.bar.is_none()) {
            fx::line_in(s, &how, brow + i as i32, &b.line);
        }
        // A chart's bars, growing together, to an eighth of a cell.
        let bars: Vec<(usize, &crate::talk::Bar)> = slide.body.iter().enumerate().filter_map(|(i, b)| Some((i, b.bar.as_ref()?))).collect();
        let frames = 24;
        for f in 1..=frames {
            if s.hurry || bars.is_empty() {
                break;
            }
            for (i, bar) in &bars {
                // Padded as the finished line is, so it's centered the same
                // and doesn't jump when it's done.
                let st = s.accent();
                let mut l = bar.line(crate::screen::ease(f, frames), st);
                let pad = markup::width(&slide.body[*i].line) - markup::width(&l);
                l.extend(markup::plain(&" ".repeat(pad.max(0) as usize), Style::default()));
                s.center(brow + *i as i32, &l);
            }
            s.tick(0.02);
        }
        for t in talk.then(slide) {
            fx::then(s, &a, &t);
        }
    }
    if let Mode::Morph(from) = mode {
        let whole = talk.tr(slide) == "morph";
        morph::play(s, &placed(s, talk, from, usize::MAX, whole), &placed(s, talk, n, shown, whole));
        if whole && !art.is_empty() {
            a.land(s);
        }
        for t in talk.then(slide) {
            fx::then(s, &a, &t);
        }
    }
    if let Some(l) = &label {
        s.center(lrow, l);
    }
    a.done(s);
    if rrow > 0 {
        s.center(rrow, &rule);
    }
    for i in (0..slide.body.len()).filter(|&i| seen(&slide.body[i])) {
        let l = body_line(s, slide, i, shown);
        s.center(brow + i as i32, &l);
    }
}

/// Slide n's poll bars moving from how full they were, `before`, to how
/// full they are now.
pub fn tallied(s: &mut Screen, talk: &Talk, n: usize, before: &[f64]) {
    let slide = &talk.slides[n];
    let Some(p) = &slide.poll else { return };
    let (.., brow) = layout(s, slide);
    let frames = if talk.calm { 1 } else { 10 };
    for f in 1..=frames {
        let e = crate::screen::ease(f, frames);
        for k in 0..p.choices.len() {
            let i = p.at + k;
            let Some(bar) = &slide.body[i].bar else { continue };
            let was = before.get(k).copied().unwrap_or(bar.frac);
            let b = crate::talk::Bar { frac: was + (bar.frac - was) * e, ..bar.clone() };
            s.center(brow + i as i32, &b.line(1.0, s.accent()));
        }
        if f < frames {
            s.tick(0.02);
        }
    }
}

/// A drawn slide: its drawing centered, its label at the top of it, and its
/// groups up to the step shown, in order, so a later clear takes away what
/// an earlier step drew.
fn drawn(s: &mut Screen, talk: &Talk, n: usize, d: &Draw, mode: Mode, shown: usize) {
    let (top, left) = canvas(s, d);
    if mode == Mode::Step {
        s.anim = true;
        for (_, it) in &d.groups[shown] {
            item(s, it, top, left, d.w);
        }
        s.anim = false;
        return;
    }
    clear(s, talk, n);
    if let Some(l) = label(s, &talk.slides[n], n) {
        s.center(top, &l);
    }
    for g in &d.groups[..=shown.min(d.groups.len() - 1)] {
        for (_, it) in g {
            item(s, it, top, left, d.w);
        }
    }
}

fn item(s: &mut Screen, it: &Item, top: i32, left: i32, w: i32) {
    match it {
        Item::Text(r, c, l) => s.typeput(top + r, left + c, l),
        Item::Center(r, l) => s.typeput(top + r, left + (w - markup::width(l)) / 2, l),
        Item::Box(r, c, bw, bh, title) => boxed(s, top + r, left + c, *bw, *bh, title),
        Item::Path(pts, dotted) => {
            let pts: Vec<(i32, i32)> = pts.iter().map(|&(r, c)| (top + r, left + c)).collect();
            path(s, &pts, *dotted)
        }
        Item::Clear(a, b) => {
            for r in *a..=*b {
                s.clear_row(top + r);
            }
        }
    }
}

/// A box, its border traced in clockwise from the top left when animating,
/// then its title typed.
/// A box's border, clockwise from the top left corner.
fn border(row: i32, col: i32, w: i32, h: i32) -> Vec<(i32, i32, char)> {
    let h = h - 1;
    let mut p: Vec<(i32, i32, char)> = vec![];
    for j in 0..w {
        p.push((row, col + j, '─'));
    }
    for j in 1..h {
        p.push((row + j, col + w - 1, '│'));
    }
    for j in (0..w).rev() {
        p.push((row + h, col + j, '─'));
    }
    for j in (1..h).rev() {
        p.push((row + j, col, '│'));
    }
    p[0].2 = '╭';
    p[(w - 1) as usize].2 = '╮';
    p[(w + h - 1) as usize].2 = '╯';
    p[(2 * w + h - 2) as usize].2 = '╰';
    p
}

fn boxed(s: &mut Screen, row: i32, col: i32, w: i32, h: i32, title: &Line) {
    let p = border(row, col, w, h);
    let st = s.muted();
    let frames = if s.anim { 20 } else { 1 };
    let mut done = 0;
    for f in 1..=frames {
        let k = p.len() - (p.len() as f64 * ((frames - f) as f64 / frames as f64).powi(3)) as usize;
        for &(r, c, ch) in &p[done..k] {
            s.put(r, c, &[Cell { ch, st }]);
        }
        done = k;
        if s.anim {
            s.tick(0.016);
        }
    }
    let mut t: Line = title.iter().map(|c| Cell { ch: c.ch, st: Style { bold: true, ..c.st } }).collect();
    let pad = (w - 4 - markup::width(&t)).max(0);
    t.extend(markup::plain(&" ".repeat(pad as usize), Style::default()));
    s.typeput(row + 1, col + 2, &t);
}

fn head(d: (i32, i32)) -> char {
    match d {
        (0, 1) => '▶',
        (0, -1) => '◀',
        (1, 0) => '▼',
        _ => '▲',
    }
}

/// Every cell of an arrow, with the way it goes there, its head last.
pub fn arrow(pts: &[(i32, i32)], dotted: bool) -> Vec<(i32, i32, char, (i32, i32))> {
    let dir = |a: (i32, i32), b: (i32, i32)| ((b.0 - a.0).signum(), (b.1 - a.1).signum());
    let line = |d: (i32, i32)| match (d.0 != 0, dotted) {
        (false, false) => '─',
        (true, false) => '│',
        (false, true) => '┄',
        (true, true) => '┊',
    };
    let mut cells: Vec<(i32, i32, char, (i32, i32))> = vec![];
    for (k, w) in pts.windows(2).enumerate() {
        let d = dir(w[0], w[1]);
        let mut at = w[0];
        while at != w[1] {
            let ch = if at == w[0] && k > 0 {
                let before = dir(pts[k - 1], w[0]);
                match (before, d) {
                    ((0, 1), (1, 0)) | ((-1, 0), (0, -1)) => '╮',
                    ((0, 1), (-1, 0)) | ((1, 0), (0, -1)) => '╯',
                    ((0, -1), (1, 0)) | ((-1, 0), (0, 1)) => '╭',
                    _ => '╰',
                }
            } else {
                line(d)
            };
            cells.push((at.0, at.1, ch, d));
            at = (at.0 + d.0, at.1 + d.1);
        }
    }
    let last = *pts.last().unwrap();
    let d = cells.last().unwrap().3;
    cells.push((last.0, last.1, head(d), d));
    cells
}

/// An arrow through the points, straight between them: the head travels
/// along it when animating, most of the way at once, then settling.
fn path(s: &mut Screen, pts: &[(i32, i32)], dotted: bool) {
    let cells = arrow(pts, dotted);
    let st = s.warm();
    let len = cells.len() - 1;
    let frames = if s.anim { 18 } else { 1 };
    let mut done = 0;
    for f in 1..=frames {
        let k = len - (len as f64 * ((frames - f) as f64 / frames as f64).powi(3)) as usize;
        for &(r, c, ch, _) in &cells[done..k] {
            s.put(r, c, &[Cell { ch, st }]);
        }
        let tip = cells[k];
        let ch = if k == len { tip.2 } else { head(tip.3) };
        s.put(tip.0, tip.1, &[Cell { ch, st }]);
        done = k;
        if s.anim {
            s.tick(0.016);
        }
    }
}

/// A picture slide: its pictures at one scale, as large as the screen has
/// room for, stacked or side by side, and the caption under them.
fn pictures(s: &mut Screen, talk: &Talk, pics: &mut Pictures, n: usize) {
    let slide = &talk.slides[n];
    clear(s, talk, n);
    if let Some(l) = label(s, slide, n) {
        s.center(2, &l);
    }
    let (cw, ch) = pics.cell();
    let aspect = cw / ch;
    let dims: Vec<(f64, f64)> = slide.images.iter().map(|i| pics.dims(&i.path)).collect();
    let k = dims.len() as f64;
    let gap = 4.0;
    let (w, h) = (s.w as f64, s.h as f64);
    let caption = slide.body.len() as f64;
    let mut row;
    if slide.side {
        // Side by side, the widths add up and the tallest counts.
        let sw: f64 = dims.iter().map(|d| d.0).sum();
        let sh = dims.iter().map(|d| d.1 * aspect).fold(0.0, f64::max);
        let sc = ((w - 4.0 - gap * (k - 1.0)) / sw).min((h - 8.0 - caption) / sh);
        let mut col = ((w - sw * sc - gap * (k - 1.0)) / 2.0) as i32 + 1;
        row = 5;
        let mut tallest = 0;
        let st = s.muted();
        for (img, d) in slide.images.iter().zip(&dims) {
            let (cols, rows) = ((d.0 * sc) as i32, ((d.1 * sc * aspect) as i32).max(1));
            let alt = markup::plain(&img.alt, st);
            s.put(4, col + (cols - markup::width(&alt)) / 2, &alt);
            pics.show(s, &img.path, row, col, cols, rows);
            col += cols + gap as i32;
            tallest = tallest.max(rows);
        }
        row += tallest + 1;
    } else {
        // Stacked, the widest counts and the heights add up, a row between each.
        let sw = dims.iter().map(|d| d.0).fold(0.0, f64::max);
        let sh: f64 = dims.iter().map(|d| d.1 * aspect).sum();
        let sc = ((w - 4.0) / sw).min((h - 7.0 - caption - (k - 1.0)) / sh);
        row = 4;
        for (img, d) in slide.images.iter().zip(&dims) {
            let (cols, rows) = ((d.0 * sc) as i32, ((d.1 * sc * aspect) as i32).max(1));
            pics.show(s, &img.path, row, (s.w - cols) / 2 + 1, cols, rows);
            row += rows + 1;
        }
    }
    for b in &slide.body {
        s.center(row, &b.line);
        row += 1;
    }
}

/// Every slide as text, for reading over.
pub fn print_all(talk: &Talk, color: bool) -> String {
    let theme = &talk.theme;
    let mut out = String::new();
    let paint = |l: &Line| -> String {
        if !color {
            return markup::text(l).trim_end().to_string();
        }
        let mut o = String::new();
        let mut st = None;
        for c in l {
            if st != Some(c.st) {
                o.push_str("\x1b[0m");
                if c.st.bold {
                    o.push_str("\x1b[1m");
                }
                if let Some(f) = c.st.fg {
                    o.push_str(&format!("\x1b[38;2;{};{};{}m", f.0, f.1, f.2));
                }
                st = Some(c.st);
            }
            o.push(c.ch);
        }
        o.push_str("\x1b[0m");
        o
    };
    let total = talk.slides.len();
    for (n, s) in talk.slides.iter().enumerate() {
        let head = format!("── {}/{total}{} ──", n + 1, s.label.as_ref().map(|l| format!(" · {l}")).unwrap_or_default());
        out += &paint(&markup::plain(&head, Style::fg(theme.muted)));
        out.push('\n');
        if !s.images.is_empty() {
            let names: Vec<String> =
                s.images.iter().map(|i| i.path.file_name().unwrap_or_default().to_string_lossy().into_owned()).collect();
            out += &format!("[{}]\n", names.join(", "));
        } else if let Some(d) = &s.draw {
            let words: Vec<String> = d
                .groups
                .iter()
                .flatten()
                .filter_map(|(_, it)| match it {
                    Item::Text(_, _, l) | Item::Center(_, l) | Item::Box(.., l) => Some(markup::text(l).trim().to_string()),
                    _ => None,
                })
                .collect();
            out += &words.join(" · ");
            out.push('\n');
        } else if !s.art.is_empty() {
            for r in &s.art {
                out += &paint(&markup::plain(&r.iter().collect::<String>(), Style::fg(theme.accent)));
                out.push('\n');
            }
            out.push('\n');
        }
        for b in &s.body {
            out += &paint(&b.line);
            out.push('\n');
        }
        out.push('\n');
    }
    out
}

/// Every slide in a grid, to pick one from: arrows move, enter goes, esc
/// or o leaves. The slide on the screen is marked.
pub fn overview(s: &mut Screen, talk: &Talk, now: usize) -> Option<usize> {
    let got = pick(s, talk, now);
    s.thaw();
    got
}

/// The slide before, `from`, going as slide n's `tr:` says: true when that
/// brought n in whole, as focus does, so it needn't arrive again.
pub fn leave(s: &mut Screen, talk: &Talk, from: usize, n: usize) -> bool {
    match talk.tr(&talk.slides[n]).as_str() {
        "life" => {
            still(s, talk, from);
            let cells = s.cells().to_vec();
            crate::life::play(s, &cells);
            false
        }
        "focus" => pull(s, talk, from, n),
        t => {
            fx::transition(s, t);
            false
        }
    }
}

/// Slide k whole, with the screen kept cell by cell, to be read back.
fn still(s: &mut Screen, talk: &Talk, k: usize) {
    let slide = &talk.slides[k];
    s.keep();
    match &slide.draw {
        _ if !slide.images.is_empty() => s.clear(),
        Some(d) => drawn(s, talk, k, d, Mode::Still, slide.steps()),
        None => text(s, talk, k, Mode::Still, slide.steps()),
    }
}

/// tr: focus. The slide before melts into frost; the frost turns into
/// slide n's, blurred; then n comes into focus out of it. For text slides:
/// the others arrive as usual after the melt.
fn pull(s: &mut Screen, talk: &Talk, from: usize, n: usize) -> bool {
    let slide = &talk.slides[n];
    still(s, talk, from);
    s.frost();
    let old = s.sky.as_mut().and_then(|k| k.take_frost()).unwrap_or_default();
    still(s, talk, n);
    s.frost();
    let new = s.sky.as_mut().and_then(|k| k.take_frost()).unwrap_or_default();
    let parts = if slide.images.is_empty() && slide.draw.is_none() { placed(s, talk, n, 0, false) } else { vec![] };
    s.clear();
    let fg = s.theme.fg;
    let smooth = |t: f64| {
        let t = t.clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    let frames = 36;
    for f in 0..=frames {
        if s.hurry {
            break;
        }
        let t = f as f64 / frames as f64;
        // The old frost into the new, then the new clearing as the text
        // sharpens over it.
        let (k, keep) = (smooth(t / 0.5), 1.0 - smooth((t - 0.45) / 0.55));
        let mix: Vec<_> = old.iter().zip(&new).map(|(a, b)| (a.0.mix(b.0, k), (a.1 + (b.1 - a.1) * k) * keep)).collect();
        if let Some(sky) = s.sky.as_mut() {
            sky.set_frost(mix);
        }
        s.raw("\x1b[?2026h");
        // The text up out of the blur behind it, not out of the dark.
        s.sky_now();
        let sharp = smooth((t - 0.45) / 0.55);
        if sharp > 0.0 {
            for (r, c, l, _) in &parts {
                let mut col = *c;
                let lit: Line = l
                    .iter()
                    .map(|x| {
                        let behind = s.under(*r, col);
                        col += 1;
                        Cell { ch: x.ch, st: Style { fg: Some(behind.mix(x.st.fg.unwrap_or(fg), sharp)), ..x.st } }
                    })
                    .collect();
                s.put(*r, *c, &lit);
            }
        }
        s.raw("\x1b[?2026l");
        s.tick(0.016);
    }
    s.thaw();
    !parts.is_empty()
}

/// For watchers: how to watch, on a card over the slide frosted, a QR code
/// for phones, the link and the command, till a key.
/// What a card says: its heading, the link for its QR code, the lines
/// under that, and whether to copy the link.
pub struct Card<'a> {
    pub title: &'a str,
    pub url: &'a str,
    pub extra: Vec<(&'a str, &'a str)>,
    pub copy: bool,
    /// The lines under the heading in a block, lined up at its left, not
    /// each centered: a table's columns.
    pub table: bool,
}

/// `local`: why the link only works on this network, when it was meant
/// to work anywhere.
pub fn watch(s: &mut Screen, talk: &Talk, n: usize, url: &str, cmd: Option<&str>, local: Option<&str>) {
    let mut extra = vec![("open  ", url)];
    extra.extend(cmd.map(|c| ("or run  ", c)));
    extra.extend(local.map(|w| ("this network only: ", w)));
    show_card(s, talk, n, Card { title: "can't see the screen? watch along", url, extra, copy: true, table: false });
}

/// The remote's card: for the presenter's phone, not the room.
pub fn remote(s: &mut Screen, talk: &Talk, n: usize, url: &str) {
    let extra = vec![("open  ", url), ("", ""), ("", "next, back, your notes, and a pointer"), ("", "just you: anyone who scans this can drive the talk")];
    show_card(s, talk, n, Card { title: "your remote", url, extra, copy: false, table: false });
}

fn show_card(s: &mut Screen, talk: &Talk, n: usize, k: Card) {
    use crossterm::event::{self, Event, KeyEventKind};
    use std::time::Duration;
    let copied = k.copy && clip(s, k.url);
    let mut shown = None;
    loop {
        if shown != Some((s.w, s.h)) {
            shown = Some((s.w, s.h));
            s.raw("\x1b[?2026h");
            behind(s, talk, n);
            s.clear();
            card(s, &k, copied);
            s.raw("\x1b[?2026l");
            s.flush();
        }
        if !event::poll(Duration::from_millis(15)).unwrap_or(false) {
            s.sky_frame();
            s.flush();
            continue;
        }
        match event::read() {
            Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => break,
            Ok(Event::Resize(..)) => s.size(),
            Ok(Event::Mouse(m)) => s.mouse(m),
            Err(_) => break,
            _ => {}
        }
    }
    s.thaw();
}

/// The QR code, two modules a cell down, a quiet zone round it, black on
/// white whatever the talk's colors, for cameras to read.
fn qr(url: &str) -> Vec<Line> {
    let Ok(code) = qrcode::QrCode::new(url.as_bytes()) else { return vec![] };
    let w = code.width();
    let dark = code.to_colors();
    let q = 2;
    let at = |x: isize, y: isize| {
        let (x, y) = (x - q, y - q);
        x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < w && dark[y as usize * w + x as usize] == qrcode::Color::Dark
    };
    let (black, white) = (markup::Rgb(0, 0, 0), markup::Rgb(255, 255, 255));
    let side = w as isize + 2 * q;
    (0..side)
        .step_by(2)
        .map(|y| {
            (0..side)
                .map(|x| {
                    let c = |d: bool| if d { black } else { white };
                    Cell { ch: '▀', st: Style { fg: Some(c(at(x, y))), bg: Some(c(y + 1 < side && at(x, y + 1))), bold: false } }
                })
                .collect()
        })
        .collect()
}

/// The link onto the clipboard: by the system's own tool, which says
/// whether it worked, or failing that by asking the terminal (OSC 52),
/// which doesn't. Watchers never get the asking; it isn't drawing.
fn clip(s: &mut Screen, text: &str) -> bool {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let tools: &[&[&str]] = if cfg!(target_os = "macos") {
        &[&["pbcopy"]]
    } else if cfg!(windows) {
        &[&["clip"]]
    } else {
        &[&["wl-copy"], &["xclip", "-selection", "clipboard"], &["xsel", "--clipboard", "--input"]]
    };
    for t in tools {
        let Ok(mut c) = Command::new(t[0]).args(&t[1..]).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() else { continue };
        let fed = c.stdin.take().is_some_and(|mut i| i.write_all(text.as_bytes()).is_ok());
        if c.wait().is_ok_and(|st| st.success()) && fed {
            return true;
        }
    }
    use base64::Engine;
    s.raw(&format!("\x1b]52;c;{}\x07", base64::engine::general_purpose::STANDARD.encode(text)));
    false
}

fn card(s: &mut Screen, k: &Card, copied: bool) {
    let code = if k.url.is_empty() { vec![] } else { qr(k.url) };
    let (acc, mt, fg) = (s.accent(), s.muted(), Style::fg(s.theme.fg));
    let mut lines: Vec<Line> = vec![markup::plain(k.title, Style { bold: true, ..acc }), vec![]];
    // The code only where it fits, with the words under it.
    if code.len() as i32 + 8 <= s.h {
        lines.extend(code);
        lines.push(vec![]);
    }
    let key = if k.table { acc } else { mt };
    let mut row = |label: &str, what: &str| {
        let mut l = markup::plain(label, key);
        l.extend(markup::plain(what, fg));
        lines.push(l);
    };
    for (label, what) in &k.extra {
        row(label, what);
    }
    if copied {
        lines.push(vec![]);
        lines.push(markup::plain("link copied", mt));
    }
    let top = ((s.h - lines.len() as i32) / 2).max(1);
    let wide = lines.iter().skip(1).map(|l| markup::width(l)).max().unwrap_or(0);
    for (i, l) in lines.iter().enumerate() {
        let col = if k.table && i > 0 { s.mid(wide) } else { s.mid(markup::width(l)) };
        s.put(top + i as i32, col, l);
    }
}

/// What the keys do, on a card over the slide frosted, till a key.
pub fn help(s: &mut Screen, talk: &Talk, n: usize) {
    let keys = [
        ("→ space n", "on"),
        ("← b", "back a slide"),
        ("12 enter", "slide 12"),
        ("'", "back to where you jumped from"),
        ("g G", "first, last"),
        ("o", "every slide; / finds one"),
        ("r", "play the slide again"),
        ("enter", "run the slide's command"),
        ("B .", "blank the screen"),
        ("w P", "how to watch, your remote (--share)"),
        ("+ −", "the font's size (--tv)"),
        ("q q", "quit"),
        ("", ""),
        ("mouse", "a laser pointer; click for a ring"),
    ];
    let wide = keys.iter().map(|(k, _)| k.chars().count()).max().unwrap_or(0);
    let pad: Vec<String> = keys.iter().map(|(k, _)| format!("{k:>wide$}   ")).collect();
    let extra = keys.iter().zip(&pad).map(|((_, what), k)| (k.as_str(), *what)).collect();
    show_card(s, talk, n, Card { title: "keys", url: "", extra, copy: false, table: true });
}

/// The slide on a smaller screen than it needs: frosted, a card over it
/// saying so, till the window's bigger or the slide changes.
pub fn small(s: &mut Screen, talk: &Talk, n: usize, (w, h): (i32, i32)) {
    s.raw("\x1b[?2026h");
    behind(s, talk, n);
    s.clear();
    let lines = [
        markup::plain("make me bigger", Style { bold: true, ..s.accent() }),
        vec![],
        markup::plain(&format!("this slide needs {w}×{h}; the window's {}×{}", s.w, s.h), s.muted()),
    ];
    let top = ((s.h - 3) / 2).max(1);
    for (i, l) in lines.iter().enumerate() {
        s.put(top + i as i32, s.mid(markup::width(l)).max(1), l);
    }
    s.raw("\x1b[?2026l");
}

/// Slide n as it first shows on a w×h screen, cell by cell, for what
/// arrives to land on: (row, col, cell), the blanks left out.
pub fn cells(talk: &Talk, n: usize, w: i32, h: i32) -> Vec<(i32, i32, Cell)> {
    let slide = &talk.slides[n];
    let mut s = Screen::recording(talk.theme.clone(), w, h);
    s.keep();
    match (&slide.draw, slide.images.is_empty()) {
        (_, false) => return vec![],
        (Some(d), _) => drawn(&mut s, talk, n, d, Mode::Still, 0),
        (None, _) => text(&mut s, talk, n, Mode::Still, 0),
    }
    s.cells()
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.filter(|c| c.ch != ' ' && c.ch != '\0').map(|c| (i as i32 / w + 1, i as i32 % w + 1, c)))
        .collect()
}

/// What falls off a w×h screen of slide n shown whole: across, and down.
fn clips(talk: &Talk, n: usize, w: i32, h: i32) -> (bool, bool) {
    let slide = &talk.slides[n];
    if !slide.images.is_empty() {
        return (false, false);
    }
    let mut s = Screen::recording(talk.theme.clone(), w, h);
    match &slide.draw {
        Some(d) => drawn(&mut s, talk, n, d, Mode::Still, slide.steps()),
        None => text(&mut s, talk, n, Mode::Still, slide.steps()),
    }
    s.clip
}

/// The smallest screen slide n fits, when it doesn't fit w×h.
pub fn needs(talk: &Talk, n: usize, w: i32, h: i32) -> Option<(i32, i32)> {
    if clips(talk, n, w, h) == (false, false) {
        return None;
    }
    let nw = (w..=600).find(|&x| !clips(talk, n, x, h.max(300)).0).unwrap_or(600);
    let nh = (h..=300).find(|&y| !clips(talk, n, nw, y).1).unwrap_or(300);
    Some((nw, nh))
}

/// The screen blank but for its sky: the slide fading out to it, then, at
/// a key, back in.
pub fn blank(s: &mut Screen, talk: &Talk, n: usize, shown: usize) {
    use crossterm::event::{self, Event, KeyEventKind};
    use std::time::Duration;
    let slide = &talk.slides[n];
    s.keep();
    match &slide.draw {
        _ if !slide.images.is_empty() => s.clear(),
        Some(d) => drawn(s, talk, n, d, Mode::Still, shown),
        None => text(s, talk, n, Mode::Still, shown),
    }
    let cells = s.cells().to_vec();
    fade(s, &cells, false);
    s.clear();
    s.flush();
    if s.pending.take().is_none() {
        loop {
            if !event::poll(Duration::from_millis(15)).unwrap_or(false) {
                s.sky_frame();
                s.flush();
                continue;
            }
            match event::read() {
                Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => break,
                Ok(Event::Resize(..)) => s.size(),
                Ok(Event::Mouse(m)) => s.mouse(m),
                Err(_) => break,
                _ => {}
            }
        }
    }
    s.hurry = false;
    if s.w as usize * s.h as usize == cells.len() {
        fade(s, &cells, true);
    }
    s.thaw();
}

/// Cells kept from the screen fading into what's behind them, or out of it.
fn fade(s: &mut Screen, cells: &[Option<Cell>], back: bool) {
    let (frames, w) = (14, s.w);
    for f in 1..=frames {
        if s.hurry {
            break;
        }
        let k = crate::screen::ease(f, frames);
        let k = if back { k } else { 1.0 - k };
        s.raw("\x1b[?2026h");
        for (i, c) in cells.iter().enumerate() {
            let Some(c) = c.filter(|c| c.ch != '\0') else { continue };
            let (r, col) = (i as i32 / w + 1, i as i32 % w + 1);
            let under = s.under(r, col);
            let st = Style { fg: Some(under.mix(c.st.fg.unwrap_or(s.theme.fg), k)), bg: c.st.bg.map(|b| under.mix(b, k)), ..c.st };
            s.put(r, col, &[Cell { ch: c.ch, st }]);
        }
        s.raw("\x1b[?2026l");
        s.tick(0.022);
    }
}

/// Slide k drawn whole and frozen behind the list, frosted.
fn behind(s: &mut Screen, talk: &Talk, k: usize) {
    s.thaw();
    still(s, talk, k);
    s.frost();
}

fn pick(s: &mut Screen, talk: &Talk, now: usize) -> Option<usize> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use std::time::Duration;
    let total = talk.slides.len() as i32;
    let mut sel = now as i32;
    let mut shown = None;
    // What's being looked for, after a /: the selection goes to the
    // slides with it, and the rest go dim.
    let mut find: Option<String> = None;
    let hit = |k: i32, q: &str| matches(&talk.slides[k as usize], q);
    // The next slide with it, from k on, going by step, round the end.
    let seek = |k: i32, by: i32, q: &str| (0..total).map(|i| (k + by * i).rem_euclid(total)).find(|&i| hit(i, q));
    loop {
        let cw = 30.min(s.w - 2).max(8);
        let cols = ((s.w - 2) / cw).max(1);
        // Drawn again when the selection moves: the slide it's on, frosted
        // behind, then the list, in one frame.
        if shown != Some((sel, s.w, s.h, find.clone())) {
            shown = Some((sel, s.w, s.h, find.clone()));
            s.raw("\x1b[?2026h");
            behind(s, talk, sel as usize);
            list(s, talk, now, sel as usize, cw, cols, find.as_deref());
            s.raw("\x1b[?2026l");
            s.flush();
        }
        // The sky goes on behind the list.
        if !event::poll(Duration::from_millis(15)).unwrap_or(false) {
            s.sky_frame();
            s.flush();
            continue;
        }
        let Ok(ev) = event::read() else { return None };
        match ev {
            Event::Key(k) if k.kind != KeyEventKind::Release && find.is_some() => {
                let q = find.as_mut().unwrap();
                match k.code {
                    KeyCode::Esc => find = None,
                    KeyCode::Enter => return Some(sel as usize),
                    KeyCode::Backspace if q.is_empty() => find = None,
                    KeyCode::Backspace => _ = q.pop(),
                    KeyCode::Down | KeyCode::Right | KeyCode::Tab => sel = seek(sel + 1, 1, q).unwrap_or(sel),
                    KeyCode::Up | KeyCode::Left | KeyCode::BackTab => sel = seek(sel - 1, -1, q).unwrap_or(sel),
                    KeyCode::Char(c) => {
                        q.push(c);
                        sel = seek(sel, 1, q).unwrap_or(sel);
                    }
                    _ => {}
                }
            }
            Event::Key(k) if k.kind != KeyEventKind::Release => match k.code {
                KeyCode::Char('/') => find = Some(String::new()),
                KeyCode::Esc | KeyCode::Char('o' | 'q') | KeyCode::Tab => return None,
                KeyCode::Enter | KeyCode::Char(' ') => return Some(sel as usize),
                KeyCode::Right | KeyCode::Char('l') => sel += 1,
                KeyCode::Left | KeyCode::Char('h') => sel -= 1,
                KeyCode::Down | KeyCode::Char('j') => sel += cols,
                KeyCode::Up | KeyCode::Char('k') => sel -= cols,
                KeyCode::Home => sel = 0,
                KeyCode::End => sel = total - 1,
                _ => {}
            },
            Event::Resize(..) => s.size(),
            Event::Mouse(m) => s.mouse(m),
            _ => {}
        }
        sel = sel.clamp(0, total - 1);
    }
}

/// Whether a slide has q in its title, its lines or its notes, whatever
/// the case.
fn matches(slide: &Slide, q: &str) -> bool {
    let q = q.to_lowercase();
    let has = |t: &str| t.to_lowercase().contains(&q);
    has(&slide.title()) || slide.body.iter().any(|b| has(&markup::text(&b.line))) || slide.notes.iter().any(|n| has(n))
}

/// The slides in a grid, cw columns wide, scrolled so the selection shows;
/// with a search, those without it dim.
fn list(s: &mut Screen, talk: &Talk, now: usize, sel: usize, cw: i32, cols: i32, find: Option<&str>) {
    let total = talk.slides.len() as i32;
    let rows = (total + cols - 1) / cols;
    let fit = (s.h - 4).max(1);
    let top = ((sel as i32 / cols) - fit + 1).max(0).min((rows - fit).max(0));
    s.clear();
    let st = s.muted();
    match find {
        Some(q) => {
            s.put_str(1, 2, "find ", st);
            s.put_str(1, 7, &format!("{q}_"), s.accent());
            let n = (0..talk.slides.len()).filter(|&i| matches(&talk.slides[i], q)).count();
            let what = if q.is_empty() { String::new() } else { format!("   {n} found · ↓ ↑ next, back · enter goes · esc stops") };
            s.put_str(1, 8 + q.chars().count() as i32, &what, st);
        }
        None => s.put_str(1, 2, "slides · arrows move · enter goes · / finds · esc back", st),
    }
    for i in 0..total as usize {
        let (r, c) = (i as i32 / cols - top, i as i32 % cols);
        if r < 0 || r >= fit {
            continue;
        }
        let mark = if i == now { "●" } else { " " };
        let text = format!("{mark}{:>3} {}", i + 1, talk.slides[i].title());
        let text: String = text.chars().take((cw - 2) as usize).collect();
        let st = if i == sel {
            Style { fg: Some(s.theme.bg), bg: Some(s.theme.accent), bold: true }
        } else if find.is_some_and(|q| !q.is_empty() && !matches(&talk.slides[i], q)) {
            Style::fg(s.theme.bg.mix(s.theme.muted, 0.5))
        } else if i == now {
            s.accent()
        } else {
            Style::default()
        };
        s.put_str(3 + r, 2 + c * cw, &text, st);
    }
}

/// What the talk's line `src` drew on slide n, marked: tinted behind, and a
/// bar in the left edge beside it. For the preview, to show where the
/// editor's cursor is.
pub fn focus(s: &mut Screen, talk: &Talk, n: usize, src: usize) {
    let slide = &talk.slides[n];
    let tint = s.theme.bg.mix(s.theme.accent, 0.22);
    let tinted = |l: &[Cell]| -> Line { l.iter().map(|c| Cell { ch: c.ch, st: Style { bg: Some(tint), ..c.st } }).collect() };
    // (row, col, cells) to tint; rows alone get the bar.
    let mut marks: Vec<(i32, i32, Line)> = vec![];
    let mut rows: Vec<i32> = vec![];
    if let Some(d) = &slide.draw {
        let (top, left) = canvas(s, d);
        if slide.label_src == Some(src) {
            if let Some(l) = label(s, slide, n) {
                marks.push((top, s.mid(markup::width(&l)), l));
            }
        }
        for (_, it) in d.groups.iter().flatten().filter(|(at, _)| *at == src) {
            match it {
                Item::Text(r, c, l) => marks.push((top + r, left + c, l.clone())),
                Item::Center(r, l) => marks.push((top + r, left + (d.w - markup::width(l)) / 2, l.clone())),
                Item::Box(r, c, bw, bh, t) => {
                    let t: Line = t.iter().map(|x| Cell { ch: x.ch, st: Style { bold: true, ..x.st } }).collect();
                    marks.push((top + r + 1, left + c + 2, t));
                    let st = s.accent();
                    for (r, c, ch) in border(top + r, left + c, *bw, *bh) {
                        marks.push((r, c, vec![Cell { ch, st }]));
                    }
                }
                Item::Path(pts, dotted) => {
                    let pts: Vec<(i32, i32)> = pts.iter().map(|&(r, c)| (top + r, left + c)).collect();
                    let st = s.accent();
                    for (r, c, ch, _) in arrow(&pts, *dotted) {
                        marks.push((r, c, vec![Cell { ch, st }]));
                    }
                }
                Item::Clear(a, b) => rows.extend(top + a..=top + b),
            }
        }
    } else if slide.images.is_empty() {
        let (lrow, arow, _, brow) = layout(s, slide);
        if slide.label_src == Some(src) {
            if let Some(l) = label(s, slide, n) {
                marks.push((lrow, s.mid(markup::width(&l)), l));
            }
        }
        if slide.headline_src == Some(src) {
            let aw = slide.art.first().map_or(0, |r| r.len() as i32);
            for (i, r) in slide.art.iter().enumerate() {
                let l: Line = r.iter().map(|&ch| Cell { ch, st: s.accent() }).collect();
                marks.push((arow + i as i32, (s.w - aw) / 2 + 1, l));
            }
        }
        for (i, b) in slide.body.iter().enumerate().filter(|(_, b)| b.src == src) {
            marks.push((brow + i as i32, s.mid(markup::width(&b.line)), b.line.clone()));
        }
    }
    let bar = Cell { ch: '▌', st: s.accent() };
    for (r, c, l) in &marks {
        s.put(*r, *c, &tinted(l));
        s.put(*r, 1, &[bar]);
    }
    for r in rows {
        s.put(r, 1, &[bar]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_slide_too_wide_says_how_wide() {
        let wide = "x".repeat(90);
        let (talk, _) = crate::talk::parse(&format!("---\n# HI\n{wide}\n---\n# HI\nshort\n"), std::path::Path::new("."), false);
        assert_eq!(needs(&talk, 0, 80, 24).map(|(w, _)| w), Some(90));
        assert_eq!(needs(&talk, 1, 80, 24), None);
        assert_eq!(needs(&talk, 0, 100, 24), None);
    }
}
