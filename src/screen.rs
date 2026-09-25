//! The screen: where things are put, and the frames effects wait between.
//! Rows and columns count from 1, as the terminal does.

use crate::markup::{self, Cell, Rgb, Style, Theme};
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use std::io::Write;
use std::time::{Duration, Instant};
use unicode_width::UnicodeWidthChar;

pub struct Screen {
    out: Vec<u8>,
    pub w: i32,
    pub h: i32,
    pub theme: Theme,
    /// A key came while something was moving: from here on nothing waits,
    /// so a hurried presenter never sees half a slide.
    pub hurry: bool,
    pub pending: Option<KeyEvent>,
    pub resized: bool,
    /// Whether text is typed in (drawn slides, stepping) or put at once.
    pub anim: bool,
    pub truecolor: bool,
    /// Whether pictures go through kitty's graphics protocol, whose
    /// pictures a cleared screen doesn't take with it.
    pub kitty: bool,
    pub rng: Rng,
    /// Asked every frame whether to hurry: the preview's, which stops an
    /// animation the moment the editor changes something.
    pub watch: Option<Box<dyn FnMut() -> bool>>,
}

/// A small xorshift, seeded where an effect should look the same each time.
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Rng {
        Rng(seed.wrapping_mul(2654435761).max(1))
    }
    pub fn seed(&mut self, s: u32) {
        *self = Rng::new(s);
    }
    /// 0 to n - 1.
    pub fn below(&mut self, n: i32) -> i32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        if n <= 0 { 0 } else { (x % n as u32) as i32 }
    }
}

/// Frame f of F eased out, 0 to 1: most of the way at once, then settling.
pub fn ease(f: i32, n: i32) -> f64 {
    1.0 - (1.0 - f as f64 / n as f64).powi(3)
}

fn to_256(c: Rgb) -> u8 {
    let q = |v: u8| if v < 48 { 0 } else if v < 115 { 1 } else { (v - 35) / 40 };
    16 + 36 * q(c.0) + 6 * q(c.1) + q(c.2)
}

impl Screen {
    pub fn new(theme: Theme) -> Screen {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.subsec_nanos()).unwrap_or(1);
        let mut s = Screen {
            out: vec![],
            w: 80,
            h: 24,
            theme,
            hurry: false,
            pending: None,
            resized: false,
            anim: false,
            truecolor: truecolor(),
            kitty: false,
            rng: Rng::new(seed),
            watch: None,
        };
        s.size();
        s
    }

    pub fn size(&mut self) {
        if let Ok((w, h)) = crossterm::terminal::size() {
            (self.w, self.h) = (w as i32, h as i32);
        }
    }

    pub fn raw(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.out.extend_from_slice(b);
    }

    pub fn flush(&mut self) {
        let mut o = std::io::stdout().lock();
        let _ = o.write_all(&self.out);
        let _ = o.flush();
        self.out.clear();
    }

    fn color(&mut self, c: Rgb, bg: bool) {
        let s = match (self.truecolor, bg) {
            (true, false) => format!("\x1b[38;2;{};{};{}m", c.0, c.1, c.2),
            (true, true) => format!("\x1b[48;2;{};{};{}m", c.0, c.1, c.2),
            (false, false) => format!("\x1b[38;5;{}m", to_256(c)),
            (false, true) => format!("\x1b[48;5;{}m", to_256(c)),
        };
        self.raw(&s);
    }

    pub fn style(&mut self, st: Style) {
        self.raw("\x1b[0m");
        if st.bold {
            self.raw("\x1b[1m");
        }
        if let Some(c) = st.fg {
            self.color(c, false);
        }
        if let Some(c) = st.bg {
            self.color(c, true);
        }
    }

    pub fn goto(&mut self, row: i32, col: i32) {
        self.raw(&format!("\x1b[{row};{col}H"));
    }

    /// Cells at a place, as much of them as is on the screen.
    pub fn put(&mut self, row: i32, col: i32, l: &[Cell]) {
        if row < 1 || row > self.h {
            return;
        }
        let mut c = col;
        let mut at: Option<i32> = None;
        let mut st: Option<Style> = None;
        for cell in l {
            let cw = cell.ch.width().unwrap_or(0) as i32;
            if c >= 1 && c + cw - 1 <= self.w {
                if at != Some(c) {
                    self.goto(row, c);
                }
                if st != Some(cell.st) {
                    self.style(cell.st);
                    st = Some(cell.st);
                }
                let mut b = [0; 4];
                self.raw(cell.ch.encode_utf8(&mut b));
                at = Some(c + cw);
            }
            c += cw;
        }
        if st.is_some() {
            self.raw("\x1b[0m");
        }
    }

    pub fn put_str(&mut self, row: i32, col: i32, s: &str, st: Style) {
        self.put(row, col, &markup::plain(s, st));
    }

    pub fn clear_row(&mut self, row: i32) {
        self.raw(&format!("\x1b[{row};1H\x1b[2K"));
    }

    /// A row cleared and the cells put on it.
    pub fn at(&mut self, row: i32, col: i32, l: &[Cell]) {
        self.clear_row(row);
        self.put(row, col, l);
    }

    pub fn clear(&mut self) {
        if self.kitty {
            self.raw("\x1b_Ga=d,d=a,q=2\x1b\\");
        }
        self.raw("\x1b[H\x1b[2J");
    }

    /// Where a line of this width starts, centered.
    pub fn mid(&self, w: i32) -> i32 {
        (self.w - w) / 2 + 1
    }

    /// Cells in the middle of a row, typed when anim is set.
    pub fn center(&mut self, row: i32, l: &[Cell]) {
        let col = self.mid(markup::width(l));
        self.typeput(row, col, l);
    }

    /// put, typed out a few letters a frame when anim is set, then in its
    /// colors.
    pub fn typeput(&mut self, row: i32, col: i32, l: &[Cell]) {
        if self.anim {
            let bare = markup::bare(l);
            let n = bare.len();
            let mut nc = 1;
            while nc < n {
                self.put(row, col, &bare[..nc]);
                self.tick(0.01);
                nc += n.div_ceil(14).max(1);
            }
        }
        self.put(row, col, l);
    }

    /// Wait out a frame. A key pressed meanwhile is kept for the deck, and
    /// from then on no frame waits.
    pub fn tick(&mut self, secs: f64) {
        self.flush();
        if self.hurry {
            return;
        }
        let end = Instant::now() + Duration::from_secs_f64(secs);
        loop {
            let now = Instant::now();
            if now >= end {
                return;
            }
            if self.watch.as_mut().is_some_and(|w| w()) {
                self.hurry = true;
                return;
            }
            let wait = if self.watch.is_some() { (end - now).min(Duration::from_millis(20)) } else { end - now };
            if !event::poll(wait).unwrap_or(false) {
                continue;
            }
            match event::read() {
                Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => {
                    self.pending = Some(k);
                    self.hurry = true;
                    return;
                }
                Ok(Event::Resize(..)) => self.resized = true,
                _ => {}
            }
        }
    }

    pub fn accent(&self) -> Style {
        Style::fg(self.theme.accent)
    }
    pub fn muted(&self) -> Style {
        Style::fg(self.theme.muted)
    }
    pub fn warm(&self) -> Style {
        Style::fg(self.theme.warm)
    }
}

/// Whether the terminal takes 24-bit color; the rest get the nearest of 256.
fn truecolor() -> bool {
    if cfg!(windows) {
        return true;
    }
    let env = |k: &str| std::env::var(k).unwrap_or_default();
    matches!(env("COLORTERM").as_str(), "truecolor" | "24bit")
        || matches!(env("TERM_PROGRAM").as_str(), "ghostty" | "iTerm.app" | "WezTerm" | "vscode" | "Hyper" | "rio")
        || env("TERM").contains("kitty")
        || env("TERM").contains("direct")
        || !env("WT_SESSION").is_empty()
}
