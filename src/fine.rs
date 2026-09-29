//! Motion finer than a cell, three ways.
//!
//! Headlines' blocks are pixels already: a Plate draws them anywhere to a
//! quarter of a cell, in the quadrant blocks (▘ ▚ ▟ …), so they slide
//! instead of hopping.
//!
//! Text moving where the terminal speaks kitty's graphics (kitty, Ghostty)
//! goes as pictures of itself, drawn in the terminal's own font, placed to
//! the pixel, and turns back into text when it stops. Elsewhere it smears:
//! between two rows it's in both, faded by how near each it is, and it
//! leaves a ghost where it was a moment ago.
//!
//! The font is the one the terminal's config names (Ghostty's font-family,
//! kitty's font_family), found among the installed fonts, or DEQUE_FACE, or
//! a monospace font the system has. DEQUE_SMOOTH=off smears instead.

use crate::markup::{self, Cell, Line, Rgb, Style};
use crate::screen::Screen;
use crate::sky::QUAD;
use base64::Engine;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// Blocks drawn at pixels, two to a cell each way, and what they covered
/// last, to take away what they've left.
#[derive(Default)]
pub struct Plate {
    was: Vec<(i32, i32)>,
}

impl Plate {
    /// A block, two pixels square, at each (x, y), from 0 at the screen's
    /// top left.
    pub fn draw(&mut self, s: &mut Screen, blocks: impl Iterator<Item = (f64, f64)>, st: Style) {
        let mut cells: BTreeMap<(i32, i32), usize> = BTreeMap::new();
        for (x, y) in blocks {
            let (x, y) = (x.round() as i32, y.round() as i32);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let (px, py) = (x + dx, y + dy);
                *cells.entry((py.div_euclid(2), px.div_euclid(2))).or_default() |= 1 << (py.rem_euclid(2) * 2 + px.rem_euclid(2));
            }
        }
        for &(r, c) in &self.was {
            if !cells.contains_key(&(r, c)) {
                s.put_str(r + 1, c + 1, " ", Style::default());
            }
        }
        for (&(r, c), &m) in &cells {
            s.put(r + 1, c + 1, &[Cell { ch: QUAD[m], st }]);
        }
        self.was = cells.into_keys().collect();
    }

    pub fn clear(&mut self, s: &mut Screen) {
        for &(r, c) in &self.was {
            s.put_str(r + 1, c + 1, " ", Style::default());
        }
        self.was.clear();
    }
}

/// The line faded k of the way up from the background.
pub fn faded(l: &[Cell], k: f64, bg: Rgb, fg: Rgb) -> Line {
    l.iter().map(|c| Cell { ch: c.ch, st: Style { fg: Some(bg.mix(c.st.fg.unwrap_or(fg), k)), ..c.st } }).collect()
}

/// The terminal's font, or None to smear.
pub fn font() -> Option<fontdue::Font> {
    if std::env::var("DEQUE_SMOOTH").as_deref() == Ok("off") {
        return None;
    }
    let path = face().or_else(|| FALLBACK.iter().map(PathBuf::from).find(|p| p.is_file()))?;
    fontdue::Font::from_bytes(std::fs::read(path).ok()?, fontdue::FontSettings::default()).ok()
}

/// The terminal's own font file: DEQUE_FACE, or the family its config
/// names, found. None when deque can't tell, rather than a guess.
pub fn face() -> Option<PathBuf> {
    std::env::var_os("DEQUE_FACE").map(PathBuf::from).or_else(|| configured().and_then(|f| installed(&f)))
}

const FALLBACK: [&str; 7] = [
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/TTF/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation/LiberationMono-Regular.ttf",
    "C:\\Windows\\Fonts\\consola.ttf",
];

/// The font family the terminal's config names, if it's one whose config
/// deque can read.
fn configured() -> Option<String> {
    let home = PathBuf::from(std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?);
    let cfg = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or(home.join(".config"));
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    let (files, key) = if env("TERM_PROGRAM") == "ghostty" {
        (vec![cfg.join("ghostty/config"), home.join("Library/Application Support/com.mitchellh.ghostty/config")], "font-family")
    } else if env("TERM").contains("kitty") || !env("KITTY_WINDOW_ID").is_empty() {
        (vec![cfg.join("kitty/kitty.conf")], "font_family")
    } else {
        return None;
    };
    for f in files {
        let Ok(text) = std::fs::read_to_string(f) else { continue };
        for l in text.lines() {
            let Some(rest) = l.trim().strip_prefix(key).filter(|r| r.starts_with([' ', '\t', '='])) else { continue };
            let v = rest.trim_start().trim_start_matches('=').trim().trim_matches('"');
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn norm(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}

/// A font file of the family, the regular one if it can tell.
fn installed(family: &str) -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let dirs = [
        home.join("Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/System/Library/Fonts"),
        home.join(".local/share/fonts"),
        home.join(".fonts"),
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
        PathBuf::from("C:\\Windows\\Fonts"),
    ];
    let mut found = vec![];
    for d in dirs {
        walk(&d, 3, &mut found);
    }
    pick(&norm(family), found)
}

/// Of the files, the family's, fewest styles first.
fn pick(want: &str, files: Vec<PathBuf>) -> Option<PathBuf> {
    let styled = ["bold", "italic", "oblique", "light", "thin", "medium", "semibold", "black", "heavy", "condensed"];
    files
        .into_iter()
        .filter_map(|p| {
            let stem = norm(&p.file_stem()?.to_string_lossy());
            let rest = stem.strip_prefix(want)?;
            let score = (styled.iter().any(|w| rest.contains(w)), !rest.contains("regular"), stem.len());
            Some((score, p))
        })
        .min()
        .map(|(_, p)| p)
}

fn walk(d: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(d) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && depth > 0 {
            walk(&p, depth - 1, out);
        } else if p.extension().is_some_and(|x| ["ttf", "otf", "ttc"].iter().any(|k| x.eq_ignore_ascii_case(k))) {
            out.push(p);
        }
    }
}

/// The font at the terminal's cell size, and where in a cell its baseline
/// goes.
struct Pen {
    size: f32,
    cw: f64,
    ch: f64,
    base: f32,
}

fn pen(font: &fontdue::Font) -> Option<Pen> {
    let ws = crossterm::terminal::window_size().ok()?;
    if ws.width == 0 || ws.columns == 0 || ws.height == 0 || ws.rows == 0 {
        return None;
    }
    let (cw, ch) = (ws.width as f64 / ws.columns as f64, ws.height as f64 / ws.rows as f64);
    let adv = font.metrics('M', 100.0).advance_width / 100.0;
    let size = cw as f32 / adv.max(0.1);
    let lm = font.horizontal_line_metrics(size)?;
    let base = ((ch as f32 - (lm.ascent - lm.descent)) / 2.0 + lm.ascent).round();
    Some(Pen { size, cw, ch, base })
}

/// A line drawn in the font, a cell's width a column, as RGBA.
fn picture(font: &fontdue::Font, p: &Pen, l: &[Cell], fg: Rgb) -> (Vec<u8>, u32, u32) {
    let (w, h) = ((markup::width(l).max(1) as f64 * p.cw).ceil() as u32, p.ch.ceil() as u32);
    let mut px = vec![0u8; (w * h * 4) as usize];
    let mut x0 = 0.0;
    for c in l {
        let col = c.st.fg.unwrap_or(fg);
        let (m, bm) = font.rasterize(c.ch, p.size);
        let (left, top) = (x0 as i32 + m.xmin, (p.base - m.height as f32 - m.ymin as f32) as i32);
        for bold in 0..=c.st.bold as i32 {
            for y in 0..m.height {
                for x in 0..m.width {
                    let (tx, ty) = (left + bold + x as i32, top + y as i32);
                    if tx < 0 || ty < 0 || tx >= w as i32 || ty >= h as i32 {
                        continue;
                    }
                    let i = (ty as u32 * w + tx as u32) as usize * 4;
                    let a = bm[y * m.width + x];
                    if a > px[i + 3] {
                        px[i..i + 4].copy_from_slice(&[col.0, col.1, col.2, a]);
                    }
                }
            }
        }
        x0 += markup::width(&[*c]) as f64 * p.cw;
    }
    (px, w, h)
}

/// Kitty's image ids for these, clear of anything else deque shows.
static NEXT: AtomicU32 = AtomicU32::new(0x6d0000);

/// Lines moving to places between cells, however the terminal can show
/// that.
pub struct Movers {
    /// The pen and each line's picture, when they go as pictures.
    pen: Option<Pen>,
    ids: Vec<u32>,
    /// Where each line is to be this frame, and was last.
    now: Vec<(usize, f64, f64, Line)>,
    last: Vec<Option<(i32, i32, Line)>>,
}

impl Movers {
    /// Ready to move these lines, sent to the terminal as pictures if it
    /// takes them.
    pub fn new(s: &mut Screen, lines: &[&[Cell]]) -> Movers {
        let pen = s.font.as_ref().filter(|_| s.kitty && s.rec.is_none()).and_then(pen);
        let mut ids = vec![];
        if let Some(p) = &pen {
            for l in lines {
                let id = NEXT.fetch_add(1, Ordering::Relaxed);
                let (rgba, w, h) = picture(s.font.as_ref().unwrap(), p, l, s.theme.fg);
                send(s, id, &rgba, w, h);
                ids.push(id);
            }
        }
        Movers { pen, ids, now: vec![], last: vec![None; lines.len()] }
    }

    /// Line k, its cells as they look now, to have its top left at (row,
    /// col) this frame: whole or not.
    pub fn at(&mut self, k: usize, row: f64, col: f64, l: Line) {
        self.now.push((k, row, col, l));
    }

    /// This frame's lines, where they're to be.
    pub fn show(&mut self, s: &mut Screen) {
        let now = std::mem::take(&mut self.now);
        if let Some(p) = &self.pen {
            for (k, row, col, _) in &now {
                place(s, self.ids[*k], (col - 1.0) * p.cw, (row - 1.0) * p.ch, p);
            }
            return;
        }
        let (bg, fg) = (s.theme.bg, s.theme.fg);
        // A ghost where each was, if it's moved on, under them all.
        for (k, row, col, _) in &now {
            if let Some((r, c, l)) = &self.last[*k]
                && (*r, *c) != (row.round() as i32, col.round() as i32)
            {
                s.put(*r, *c, &faded(l, 0.3, bg, fg));
            }
        }
        for (k, row, col, l) in now {
            let (top, c) = (row.floor() as i32, col.round() as i32);
            let f = row - row.floor();
            match f {
                f if f < 0.12 => s.put(top, c, &l),
                f if f > 0.88 => s.put(top + 1, c, &l),
                // Between two rows: in both, each as bright as it's near.
                f => {
                    s.put(top, c, &faded(&l, 1.0 - f, bg, fg));
                    s.put(top + 1, c, &faded(&l, f, bg, fg));
                }
            }
            self.last[k] = Some((row.round() as i32, c, l));
        }
    }

    /// The pictures gone, for the text to take their places.
    pub fn done(self, s: &mut Screen) {
        for id in self.ids {
            s.raw(&format!("\x1b_Ga=d,d=I,i={id},q=2\x1b\\"));
        }
    }
}

/// An image sent, not shown. As images.rs does it: the last chunk carries
/// the end of the data and m=0, for Ghostty.
fn send(s: &mut Screen, id: u32, rgba: &[u8], w: u32, h: u32) {
    let data = base64::engine::general_purpose::STANDARD.encode(rgba);
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(4096).collect();
    for (i, c) in chunks.iter().enumerate() {
        let more = (i + 1 < chunks.len()) as u8;
        let head = if i == 0 { format!("\x1b_Ga=t,f=32,s={w},v={h},i={id},q=2,m={more};") } else { format!("\x1b_Gm={more};") };
        s.raw(&head);
        s.bytes(c);
        s.raw("\x1b\\");
    }
}

/// Image id shown with its top left at pixel (x, y) of the screen, moving
/// it there if it's already showing; hidden when that's off the screen.
fn place(s: &mut Screen, id: u32, x: f64, y: f64, p: &Pen) {
    let (col, row) = ((x / p.cw).floor(), (y / p.ch).floor());
    if col < 0.0 || row < 0.0 || row as i32 >= s.h || col as i32 >= s.w {
        s.raw(&format!("\x1b_Ga=d,d=i,i={id},q=2\x1b\\"));
        return;
    }
    // Kitty wants the offset inside the cell: less than its size.
    let inside = |d: f64, cell: f64| (d.round() as u32).min(cell.ceil() as u32 - 1);
    let (dx, dy) = (inside(x - col * p.cw, p.cw), inside(y - row * p.ch, p.ch));
    s.goto(row as i32 + 1, col as i32 + 1);
    s.raw(&format!("\x1b_Ga=p,i={id},p=1,X={dx},Y={dy},C=1,q=2\x1b\\"));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_family_finds_its_regular_file() {
        let files = ["TX-02-Data-Bold.1.otf", "TX-02-Data-Regular.2.otf", "Other-Regular.ttf", "TX-02-Data-Italic.otf"];
        let got = pick(&norm("TX-02-Data"), files.iter().map(PathBuf::from).collect());
        assert_eq!(got, Some(PathBuf::from("TX-02-Data-Regular.2.otf")));
        let got = pick(&norm("JetBrains Mono"), vec![PathBuf::from("JetBrainsMono-Bold.ttf"), PathBuf::from("JetBrainsMono.ttf")]);
        assert_eq!(got, Some(PathBuf::from("JetBrainsMono.ttf")));
        assert_eq!(pick("nope", files.iter().map(PathBuf::from).collect()), None);
    }
}
