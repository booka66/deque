//! The screen: where things are put, and the frames effects wait between.
//! Rows and columns count from 1, as the terminal does.

use crate::markup::{self, Cell, Rgb, Style, Theme};
use crate::sky::{Kind, Sky};
use crossterm::event::{self, Event, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
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
    /// Recording, for --cast: what's flushed is kept with the time it was,
    /// and a frame's wait is only the clock moving on.
    pub rec: Option<Rec>,
    /// What's behind the slide, when it has something. Then what's put is
    /// kept, cell by cell, to draw the sky around and under it.
    pub sky: Option<Sky>,
    /// What's been put, None where it's see-through, and what the terminal
    /// shows, None where that isn't known: only what changed is sent.
    front: Vec<Option<Cell>>,
    seen: Vec<Option<Cell>>,
    /// When the sky was last drawn, on the screen's clock.
    sky_at: f64,
    born: Instant,
    /// The terminal's font, for text that moves as pictures; see fine.rs.
    pub font: Option<fontdue::Font>,
    /// When the mouse last moved, on the screen's clock: while it's in use
    /// there's a sky to draw its light, slide or no.
    pointed: f64,
    /// The sky came on over a screen it doesn't know: the slide wants
    /// drawing again, and till then the sky keeps off it.
    pub redraw: bool,
    /// Watchers, with --share: everything drawn goes to them too.
    pub tap: Option<crate::share::Hub>,
    /// Whether something put fell off the screen, across and down: the
    /// window's too small for it.
    pub clip: (bool, bool),
}

#[derive(Default)]
pub struct Rec {
    pub clock: f64,
    pub frames: Vec<(f64, String)>,
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
            rec: None,
            sky: None,
            front: vec![],
            seen: vec![],
            sky_at: 0.0,
            born: Instant::now(),
            font: None,
            pointed: f64::NEG_INFINITY,
            redraw: false,
            tap: None,
            clip: (false, false),
        };
        s.size();
        s
    }

    /// A screen of a set size that records instead of showing.
    pub fn recording(theme: Theme, w: i32, h: i32) -> Screen {
        let mut s = Screen::new(theme);
        (s.w, s.h, s.truecolor) = (w, h, true);
        s.rec = Some(Rec::default());
        s
    }

    pub fn size(&mut self) {
        if self.rec.is_some() {
            return;
        }
        if let Ok((w, h)) = crossterm::terminal::size() {
            let was = (self.w, self.h);
            (self.w, self.h) = (w as i32, h as i32);
            if was != (self.w, self.h)
                && let Some(s) = self.sky.take()
            {
                self.backdrop(s.kind, s.glow);
            }
        }
    }

    /// Seconds since the screen was made, or into the recording.
    pub fn now(&self) -> f64 {
        self.rec.as_ref().map_or_else(|| self.born.elapsed().as_secs_f64(), |r| r.clock)
    }

    /// The slide's sky. One the same as the last is kept, so it goes on
    /// moving from slide to slide.
    pub fn backdrop(&mut self, kind: Kind, glow: bool) {
        if kind == Kind::None && !glow && !self.pointing() {
            self.sky = None;
            return;
        }
        if let Some(s) = self.sky.as_mut().filter(|s| s.kind == kind) {
            s.glow = glow;
            return;
        }
        let (pointer, ripples, trail) = self.sky.take().map(|s| (s.pointer, s.ripples, s.trail)).unwrap_or_default();
        self.make_sky(kind, glow);
        let s = self.sky.as_mut().unwrap();
        (s.pointer, s.ripples, s.trail) = (pointer, ripples, trail);
    }

    fn pointing(&self) -> bool {
        self.now() - self.pointed < 3.0
    }

    /// The mouse moved to (row, col): the pointer's there.
    pub fn point(&mut self, row: i32, col: i32) {
        if self.sky.is_none() {
            self.keep();
            self.redraw = true;
        }
        self.pointed = self.now();
        let at = (2.0 * (col - 1) as f64 + 1.0, 2.0 * (row - 1) as f64 + 1.0, self.pointed);
        let sky = self.sky.as_mut().unwrap();
        sky.pointer = Some(at);
        sky.trail.push(at);
    }

    /// Where the phone remote pointed, and tapped, as the mouse would.
    pub fn steer(&mut self) {
        let Some(h) = self.tap.clone() else { return };
        for (x, y, tap) in h.points() {
            let (row, col) = ((y * (self.h - 1) as f64).round() as i32 + 1, (x * (self.w - 1) as f64).round() as i32 + 1);
            if tap { self.click(row, col) } else { self.point(row, col) }
        }
    }

    /// What the mouse did, for the pointer: moving it, or a click.
    pub fn mouse(&mut self, m: MouseEvent) {
        let (row, col) = (m.row as i32 + 1, m.column as i32 + 1);
        match m.kind {
            MouseEventKind::Moved | MouseEventKind::Drag(_) => self.point(row, col),
            MouseEventKind::Down(_) => self.click(row, col),
            _ => {}
        }
    }

    /// A click at (row, col): a ring goes out from it.
    pub fn click(&mut self, row: i32, col: i32) {
        self.point(row, col);
        let (x, y, t) = self.sky.as_ref().unwrap().pointer.unwrap();
        self.sky.as_mut().unwrap().ripples.push((x, y, t));
    }

    /// What's been put, cell by cell, while the sky keeps it.
    pub fn cells(&self) -> &[Option<Cell>] {
        &self.front
    }

    /// The color behind the cell at (row, col): the sky's there, or the
    /// background.
    pub fn under(&self, row: i32, col: i32) -> Rgb {
        let probe = Cell { ch: 'x', st: Style::default() };
        match &self.sky {
            Some(s) if row >= 1 && col >= 1 && row <= self.h && col <= self.w => {
                s.look((row - 1) as usize, (col - 1) as usize, Some(probe)).st.bg.unwrap_or(self.theme.bg)
            }
            _ => self.theme.bg,
        }
    }

    /// The sky drawn now, due or not.
    pub fn sky_now(&mut self) {
        self.sky_at = f64::NEG_INFINITY;
        self.sky_frame();
    }

    fn make_sky(&mut self, kind: Kind, glow: bool) {
        let n = (self.w * self.h).max(0) as usize;
        (self.front, self.seen) = (vec![None; n], vec![None; n]);
        let mut sky = Sky::new(kind, glow, self.w, self.h, self.rng.below(1 << 30) as u32);
        self.sky_at = self.now();
        sky.frame(self.sky_at, &self.theme, &self.front, self.w as usize);
        self.sky = Some(sky);
    }

    /// Whatever's on the screen, frozen and blurred behind what's drawn
    /// next, until thawed; with a sky of nothing, if the slide has none.
    pub fn frost(&mut self) {
        let now = self.now();
        let Some(sky) = self.sky.as_mut() else { return };
        sky.freeze(&self.front, self.w as usize, &self.theme);
        sky.frame(now, &self.theme, &self.front, self.w as usize);
    }

    /// Ready to frost: the screen kept cell by cell from here on.
    pub fn keep(&mut self) {
        if self.sky.is_none() {
            self.make_sky(Kind::None, false);
        }
    }

    pub fn thaw(&mut self) {
        let pointing = self.pointing();
        match self.sky.as_mut() {
            Some(s) if s.kind == Kind::None && !s.glow && !pointing => self.sky = None,
            Some(s) => s.thaw(),
            None => {}
        }
    }

    /// The sky moved on and drawn again where it changed, when a frame of
    /// it is due.
    pub fn sky_frame(&mut self) {
        let fps = if self.rec.is_some() { 15.0 } else { 30.0 };
        let now = self.now();
        let Some(sky) = self.sky.as_mut() else { return };
        if self.redraw || now - self.sky_at < 1.0 / fps - 1e-6 {
            return;
        }
        self.sky_at = now;
        sky.frame(now, &self.theme, &self.front, self.w as usize);
        self.raw("\x1b[?2026h");
        for r in 1..=self.h {
            self.paint(r, 1, self.w);
        }
        self.raw("\x1b[?2026l");
    }

    fn idx(&self, row: i32, col: i32) -> usize {
        ((row - 1) * self.w + col - 1) as usize
    }

    /// Columns lo to hi of a row as the sky and what's on it make them,
    /// sending only the cells that changed.
    fn paint(&mut self, row: i32, lo: i32, hi: i32) {
        let mut at: Option<i32> = None;
        let mut st: Option<Style> = None;
        for c in lo..=hi {
            let i = self.idx(row, c);
            let f = self.front[i];
            // The right half of a wide letter, drawn with its left.
            if f.is_some_and(|f| f.ch == '\0') {
                continue;
            }
            let want = self.sky.as_ref().unwrap().look((row - 1) as usize, (c - 1) as usize, f);
            if self.seen[i] == Some(want) {
                continue;
            }
            self.seen[i] = Some(want);
            if at != Some(c) {
                self.goto(row, c);
            }
            match st {
                Some(was) if was == want.st => {}
                // Only the colors that changed, when that's all it is.
                Some(was) if was.bold == want.st.bold => {
                    if was.fg != want.st.fg {
                        match want.st.fg {
                            Some(c) => self.color(c, false),
                            None => self.raw("\x1b[39m"),
                        }
                    }
                    if was.bg != want.st.bg {
                        match want.st.bg {
                            Some(c) => self.color(c, true),
                            None => self.raw("\x1b[49m"),
                        }
                    }
                }
                _ => self.style(want.st),
            }
            st = Some(want.st);
            let mut b = [0; 4];
            self.raw(want.ch.encode_utf8(&mut b));
            at = Some(c + want.ch.width().unwrap_or(0) as i32);
        }
        if st.is_some() {
            self.raw("\x1b[0m");
        }
    }

    /// put, with a sky: kept, then painted.
    fn put_over(&mut self, row: i32, col: i32, l: &[Cell]) {
        let (mut c, mut lo, mut hi) = (col, i32::MAX, 0);
        for cell in l {
            let cw = cell.ch.width().unwrap_or(0) as i32;
            if c >= 1 && c + cw - 1 <= self.w {
                let i = self.idx(row, c);
                self.front[i] = (cell.ch != ' ' || cell.st.bg.is_some()).then_some(*cell);
                if cw == 2 {
                    self.front[i + 1] = Some(Cell { ch: '\0', st: cell.st });
                }
                (lo, hi) = (lo.min(c), hi.max(c + cw - 1));
            }
            c += cw;
        }
        if lo <= hi {
            self.paint(row, lo, hi);
        }
    }

    pub fn raw(&mut self, s: &str) {
        self.out.extend_from_slice(s.as_bytes());
    }

    pub fn bytes(&mut self, b: &[u8]) {
        self.out.extend_from_slice(b);
    }

    pub fn flush(&mut self) {
        if let Some(r) = self.rec.as_mut() {
            if !self.out.is_empty() {
                r.frames.push((r.clock, String::from_utf8_lossy(&self.out).into_owned()));
                self.out.clear();
            }
            return;
        }
        let mut o = std::io::stdout().lock();
        let _ = o.write_all(&self.out);
        let _ = o.flush();
        if let Some(t) = &self.tap
            && !self.out.is_empty()
        {
            t.send(&self.out, self.w, self.h);
        }
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
        let mut c = col;
        for cell in l {
            let cw = cell.ch.width().unwrap_or(0) as i32;
            if cell.ch != ' ' && (c < 1 || c + cw - 1 > self.w) {
                self.clip.0 = true;
            }
            c += cw;
        }
        if row < 1 || row > self.h {
            self.clip.1 |= l.iter().any(|c| c.ch != ' ');
            return;
        }
        if self.sky.is_some() {
            return self.put_over(row, col, l);
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
        if self.sky.is_some() {
            if (1..=self.h).contains(&row) {
                let i = self.idx(row, 1);
                self.front[i..i + self.w as usize].fill(None);
                self.paint(row, 1, self.w);
            }
            return;
        }
        self.raw(&format!("\x1b[{row};1H\x1b[2K"));
    }

    /// A row cleared and the cells put on it.
    pub fn at(&mut self, row: i32, col: i32, l: &[Cell]) {
        self.clear_row(row);
        self.put(row, col, l);
    }

    pub fn clear(&mut self) {
        self.redraw = false;
        if self.kitty {
            self.raw("\x1b_Ga=d,d=a,q=2\x1b\\");
        }
        if self.sky.is_some() {
            // Emptied in the sky's own background, then the sky put back.
            let blank = Cell { ch: ' ', st: Style { bg: Some(self.theme.bg), ..Style::default() } };
            self.style(blank.st);
            self.raw("\x1b[H\x1b[2J\x1b[0m");
            self.front.fill(None);
            self.seen.fill(Some(blank));
            for r in 1..=self.h {
                self.paint(r, 1, self.w);
            }
            return;
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
        if let Some(r) = self.rec.as_mut() {
            // Recorded, the clock goes on a sky frame at a time.
            let end = r.clock + secs;
            while self.sky.is_some() && self.sky_at + 1.0 / 15.0 < end {
                self.rec.as_mut().unwrap().clock = self.sky_at + 1.0 / 15.0;
                self.sky_frame();
                self.flush();
            }
            self.rec.as_mut().unwrap().clock = end;
            return;
        }
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
            let mut wait = if self.watch.is_some() { (end - now).min(Duration::from_millis(20)) } else { end - now };
            if self.sky.is_some() || self.tap.is_some() {
                wait = wait.min(Duration::from_millis(15));
            }
            if !event::poll(wait).unwrap_or(false) {
                self.steer();
                self.sky_frame();
                self.flush();
                continue;
            }
            match event::read() {
                Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => {
                    self.pending = Some(k);
                    self.hurry = true;
                    return;
                }
                Ok(Event::Resize(..)) => self.resized = true,
                Ok(Event::Mouse(m)) => self.mouse(m),
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
