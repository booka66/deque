//! Pictures, in whatever the terminal speaks: kitty's graphics protocol
//! (kitty, Ghostty, WezTerm), iTerm2's (iTerm2, WezTerm), sixel (foot,
//! Windows Terminal, xterm -ti vt340, …), or failing all of them, colored
//! half blocks, which any terminal with color can show.
//! DEQUE_IMAGES=kitty|iterm|sixel|blocks says which outright.

use crate::markup::{Cell, Rgb, Style};
use crate::screen::Screen;
use base64::Engine;
use image::{DynamicImage, imageops::FilterType};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Proto {
    Kitty,
    Iterm,
    Sixel,
    Blocks,
}

pub struct Pictures {
    pub proto: Proto,
    loaded: HashMap<PathBuf, DynamicImage>,
    /// What was sent for a picture at a size, to send again as it was.
    sent: HashMap<(PathBuf, i32, i32), Vec<u8>>,
    /// Pictures at a size in blocks, two pixels a cell.
    small: HashMap<(PathBuf, i32, i32), image::RgbImage>,
}

impl Pictures {
    pub fn new(proto: Proto) -> Pictures {
        Pictures { proto, loaded: HashMap::new(), sent: HashMap::new(), small: HashMap::new() }
    }

    fn load(&mut self, p: &Path) -> Option<&DynamicImage> {
        if !self.loaded.contains_key(p) {
            let img = image::open(p).ok()?;
            self.loaded.insert(p.to_path_buf(), img);
        }
        self.loaded.get(p)
    }

    /// Its width and height in pixels.
    pub fn dims(&mut self, p: &Path) -> (f64, f64) {
        match self.load(p) {
            Some(i) => (i.width() as f64, i.height() as f64),
            None => (4.0, 3.0),
        }
    }

    /// A cell's size in pixels, from the terminal, or a guess of 1 by 2.
    pub fn cell(&self) -> (f64, f64) {
        if self.proto != Proto::Blocks {
            if let Ok(ws) = crossterm::terminal::window_size() {
                if ws.width > 0 && ws.columns > 0 && ws.height > 0 && ws.rows > 0 {
                    return (ws.width as f64 / ws.columns as f64, ws.height as f64 / ws.rows as f64);
                }
            }
        }
        (1.0, 2.0)
    }

    /// The picture filling cols × rows cells at a place.
    pub fn show(&mut self, s: &mut Screen, p: &Path, row: i32, col: i32, cols: i32, rows: i32) {
        if cols < 1 || rows < 1 {
            return;
        }
        let key = (p.to_path_buf(), cols, rows);
        if self.proto == Proto::Blocks {
            self.blocks(s, p, row, col, cols, rows);
            return;
        }
        if !self.sent.contains_key(&key) {
            let (cw, ch) = self.cell();
            let proto = self.proto;
            let Some(img) = self.load(p) else { return };
            let bytes = match proto {
                Proto::Kitty => kitty(p, img, cols, rows),
                Proto::Iterm => iterm(p, img, cols, rows),
                _ => sixel(img, (cols as f64 * cw) as u32, (rows as f64 * ch) as u32),
            };
            self.sent.insert(key.clone(), bytes);
        }
        s.goto(row, col);
        s.bytes(&self.sent[&key]);
    }

    /// Two pixels a cell, the upper half block in one's color over the
    /// other's.
    fn blocks(&mut self, s: &mut Screen, p: &Path, row: i32, col: i32, cols: i32, rows: i32) {
        let key = (p.to_path_buf(), cols, rows);
        if !self.small.contains_key(&key) {
            let Some(img) = self.load(p) else { return };
            let small = img.resize_exact(cols as u32, rows as u32 * 2, FilterType::Triangle).to_rgb8();
            self.small.insert(key.clone(), small);
        }
        let small = &self.small[&key];
        for r in 0..rows {
            let line: Vec<Cell> = (0..cols)
                .map(|c| {
                    let t = small.get_pixel(c as u32, r as u32 * 2);
                    let b = small.get_pixel(c as u32, r as u32 * 2 + 1);
                    let st = Style { fg: Some(Rgb(t[0], t[1], t[2])), bg: Some(Rgb(b[0], b[1], b[2])), bold: false };
                    Cell { ch: '▀', st }
                })
                .collect();
            s.put(row + r, col, &line);
        }
    }
}

fn png(p: &Path, img: &DynamicImage) -> Vec<u8> {
    if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("png")) {
        if let Ok(b) = std::fs::read(p) {
            return b;
        }
    }
    let mut out = std::io::Cursor::new(vec![]);
    let _ = img.write_to(&mut out, image::ImageFormat::Png);
    out.into_inner()
}

/// Sent and shown at the cursor in one go, in chunks. The last chunk
/// carries the end of the data and m=0: Ghostty drops an image whose m=0
/// comes as an empty chunk of its own.
fn kitty(p: &Path, img: &DynamicImage, cols: i32, rows: i32) -> Vec<u8> {
    let data = base64::engine::general_purpose::STANDARD.encode(png(p, img));
    let chunks: Vec<&[u8]> = data.as_bytes().chunks(4096).collect();
    let mut out = vec![];
    for (i, c) in chunks.iter().enumerate() {
        let more = (i + 1 < chunks.len()) as u8;
        let head = if i == 0 { format!("\x1b_Gf=100,a=T,q=2,C=1,c={cols},r={rows},m={more};") } else { format!("\x1b_Gm={more};") };
        out.extend_from_slice(head.as_bytes());
        out.extend_from_slice(c);
        out.extend_from_slice(b"\x1b\\");
    }
    out
}

fn iterm(p: &Path, img: &DynamicImage, cols: i32, rows: i32) -> Vec<u8> {
    let data = base64::engine::general_purpose::STANDARD.encode(png(p, img));
    format!("\x1b]1337;File=inline=1;width={cols};height={rows};preserveAspectRatio=0:{data}\x07").into_bytes()
}

fn sixel(img: &DynamicImage, w: u32, h: u32) -> Vec<u8> {
    let rgba = img.resize_exact(w.max(1), h.max(1), FilterType::Triangle).to_rgba8();
    let (w, h) = rgba.dimensions();
    icy_sixel::SixelImage::try_from_rgba(rgba.into_raw(), w as usize, h as usize)
        .and_then(|i| i.encode())
        .map(String::into_bytes)
        .unwrap_or_default()
}

/// What the terminal speaks. Asked outright where that is possible: kitty's
/// protocol answers a query, and the primary device attributes say whether
/// sixel is there. Must be called in raw mode, before anything else reads
/// the keyboard.
pub fn detect() -> Proto {
    match std::env::var("DEQUE_IMAGES").as_deref() {
        Ok("kitty") => return Proto::Kitty,
        Ok("iterm") => return Proto::Iterm,
        Ok("sixel") => return Proto::Sixel,
        Ok("blocks") => return Proto::Blocks,
        _ => {}
    }
    // Inside a multiplexer, pictures would land where it doesn't expect, at
    // a size it doesn't know.
    if ["TMUX", "HERDR_PANE_ID"].iter().any(|k| std::env::var_os(k).is_some()) {
        return Proto::Blocks;
    }
    let reply = ask("\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\\x1b[c");
    if reply.contains("_Gi=31;OK") {
        return Proto::Kitty;
    }
    if matches!(std::env::var("TERM_PROGRAM").as_deref(), Ok("iTerm.app" | "WezTerm")) {
        return Proto::Iterm;
    }
    // The device attributes: ESC [ ? 6x ; a ; b ; … c, 4 among them for sixel.
    if let Some(da) = reply.split("\x1b[?").nth(1).and_then(|r| r.split('c').next()) {
        if da.split(';').skip(1).any(|a| a == "4") {
            return Proto::Sixel;
        }
    }
    Proto::Blocks
}

/// The terminal's background color, asked outright (OSC 11), for a talk
/// that doesn't set its own: what skies and frost paint the screen with.
pub fn background() -> Option<Rgb> {
    parse_bg(&ask("\x1b]11;?\x1b\\\x1b[c"))
}

/// `rgb:RRRR/GGGG/BBBB`, each 1 to 4 hex digits, as xterm answers.
fn parse_bg(reply: &str) -> Option<Rgb> {
    let rest = reply.split("]11;rgb:").nth(1)?;
    let mut v = rest.split(['/', '\x07', '\x1b']).take(3).map(|h| {
        let n = u32::from_str_radix(h, 16).ok()?;
        Some((n * 255 / ((1u32 << (4 * h.len().clamp(1, 4))) - 1)) as u8)
    });
    Some(Rgb(v.next()??, v.next()??, v.next()??))
}

/// Writes a query and reads what comes back, up to the device attributes'
/// closing c or half a second.
#[cfg(unix)]
fn ask(q: &str) -> String {
    use std::io::Write;
    let mut o = std::io::stdout();
    let _ = o.write_all(q.as_bytes());
    let _ = o.flush();
    let mut got = vec![];
    let end = std::time::Instant::now() + std::time::Duration::from_millis(500);
    loop {
        let left = end.saturating_duration_since(std::time::Instant::now()).as_millis() as i32;
        if left == 0 {
            break;
        }
        let mut fd = libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 };
        if unsafe { libc::poll(&mut fd, 1, left) } <= 0 {
            break;
        }
        let mut b = [0u8; 256];
        let n = unsafe { libc::read(0, b.as_mut_ptr().cast(), b.len()) };
        if n <= 0 {
            break;
        }
        got.extend_from_slice(&b[..n as usize]);
        let s = String::from_utf8_lossy(&got);
        if s.split("\x1b[?").nth(1).is_some_and(|r| r.contains('c')) {
            break;
        }
    }
    String::from_utf8_lossy(&got).into_owned()
}

/// Windows has no plain way to read the console's replies; there it goes by
/// DEQUE_IMAGES, or blocks.
#[cfg(not(unix))]
fn ask(_: &str) -> String {
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_background_reply() {
        assert_eq!(parse_bg("\x1b]11;rgb:1d1d/2020/2121\x1b\\\x1b[?62c"), Some(Rgb(29, 32, 33)));
        assert_eq!(parse_bg("\x1b]11;rgb:ff/80/00\x07"), Some(Rgb(255, 128, 0)));
        assert_eq!(parse_bg("\x1b[?62c"), None);
    }
}
