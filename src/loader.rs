//! While --share opens its link: the shell that was on the screen, taken
//! as it was (from tmux, herdr, kitty or WezTerm, which can say what's on
//! the screen, or Ghostty, which writes it to a file when asked; elsewhere,
//! the command that started deque), comes loose a
//! letter at a time from the cursor out and spirals into a galaxy, turning
//! till the link's ready; then every letter flies to its place in the
//! first slide.

use crate::markup::{Cell, Rgb, Style, Theme};
use crate::screen::{Screen, ease};
use std::process::Command;
use std::time::{Duration, Instant};

/// The most letters that fly; past that, a sample of them.
const MOST: usize = 2400;

/// A letter on its way: what it is, where it was, where it is, how fast
/// it's going, when it came loose, and its orbit: radius, phase, speed.
#[derive(Clone, Copy)]
pub struct Mote {
    cell: Cell,
    home: (f64, f64),
    at: (f64, f64),
    v: (f64, f64),
    loose: f64,
    orbit: (f64, f64, f64),
}

/// What's on the screen now, row by row, as the terminal or multiplexer
/// can tell it; failing that, the command that started deque, on the line
/// above the prompt it'll come back to.
pub fn capture(h: i32) -> Vec<Vec<Cell>> {
    let run = |cmd: &str, args: &[&str]| Command::new(cmd).args(args).output().ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned());
    let env = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    // The innermost first: tmux run in herdr knows what's on its screen.
    let text = env("TMUX")
        .and_then(|_| run("tmux", &["capture-pane", "-p", "-e"]))
        .or_else(|| env("HERDR_PANE_ID").and_then(|id| run("herdr", &["pane", "read", &id, "--source", "visible", "--format", "ansi"])))
        .or_else(|| env("KITTY_WINDOW_ID").and_then(|_| run("kitty", &["@", "get-text", "--ansi", "--extent", "screen"])))
        .or_else(|| env("WEZTERM_PANE").and_then(|_| run("wezterm", &["cli", "get-text", "--escapes"])))
        .or_else(ghostty);
    let mut rows: Vec<Vec<Cell>> = match text {
        Some(t) => t.lines().map(sgr).collect(),
        None => {
            let args: Vec<String> = std::env::args().skip(1).collect();
            let line = crate::markup::plain(&format!("$ deque {}", args.join(" ")), Style::default());
            let mut rows = vec![vec![]; (h - 2).max(0) as usize];
            rows.push(line);
            rows
        }
    };
    // The bottom of it, if there's more than fits.
    let keep = h.max(0) as usize;
    if rows.len() > keep {
        rows.drain(..rows.len() - keep);
    }
    rows
}

/// Ghostty's screen, colors and all: on macOS, asked by AppleScript to
/// write it to a file and paste the file's name in, which comes to deque
/// as keys (the clipboard's left alone). Only when Ghostty's in front, as
/// it is when enter's just started deque in it. Raw mode must be on, so
/// what's pasted isn't echoed.
fn ghostty() -> Option<String> {
    use crossterm::event::{self, Event, KeyCode};
    if !cfg!(target_os = "macos") || std::env::var("TERM_PROGRAM").ok()? != "ghostty" {
        return None;
    }
    let script = r#"tell application "Ghostty"
        if not frontmost then return "no"
        perform action "write_screen_file:paste,vt" on focused terminal of selected tab of front window
        return "ok"
    end tell"#;
    let out = Command::new("osascript").args(["-e", script]).output().ok()?;
    if String::from_utf8_lossy(&out.stdout).trim() != "ok" {
        return None;
    }
    // The file's name, typed in; done when nothing more comes for a moment.
    let (mut path, t) = (String::new(), Instant::now());
    while t.elapsed() < Duration::from_secs(2) {
        if !event::poll(Duration::from_millis(if path.ends_with(".txt") { 60 } else { 200 })).ok()? {
            if path.ends_with(".txt") {
                break;
            }
            continue;
        }
        if let Ok(Event::Key(k)) = event::read()
            && let KeyCode::Char(c) = k.code
        {
            path.push(c);
        }
    }
    let file = std::path::PathBuf::from(path.trim());
    if file.file_name()? != "screen.txt" {
        return None;
    }
    let text = std::fs::read_to_string(&file).ok();
    // Gone once read, and the folder made for it, if that leaves it empty.
    let _ = std::fs::remove_file(&file);
    if let Some(dir) = file.parent() {
        let _ = std::fs::remove_dir(dir);
    }
    text
}

/// A line with SGR colors in it as cells in those colors; any other escape
/// left out.
fn sgr(line: &str) -> Vec<Cell> {
    let basic = |n: u32| {
        const P: [(u8, u8, u8); 16] = [
            (40, 40, 40), (204, 36, 29), (152, 151, 26), (215, 153, 33), (69, 133, 136), (177, 98, 134), (104, 157, 106), (168, 153, 132),
            (146, 131, 116), (251, 73, 52), (184, 187, 38), (250, 189, 47), (131, 165, 152), (211, 134, 155), (142, 192, 124), (235, 219, 178),
        ];
        let (r, g, b) = P[n as usize % 16];
        Rgb(r, g, b)
    };
    let cube = |n: u32| match n {
        0..=15 => basic(n),
        16..=231 => {
            let n = n - 16;
            let v = |x: u32| if x == 0 { 0 } else { (55 + 40 * x) as u8 };
            Rgb(v(n / 36), v(n / 6 % 6), v(n % 6))
        }
        _ => {
            let g = (8 + 10 * (n - 232)) as u8;
            Rgb(g, g, g)
        }
    };
    let (mut out, mut st) = (vec![], Style::default());
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        if c != '\x1b' {
            if !c.is_control() {
                out.push(Cell { ch: c, st });
            }
            continue;
        }
        if it.peek() != Some(&'[') {
            // Any other escape: its next character, and (for strings) to BEL or ST.
            if let Some(k) = it.next()
                && matches!(k, ']' | 'P' | '_' | '^' | 'X')
            {
                while let Some(x) = it.next() {
                    if x == '\x07' || (x == '\x1b' && it.next_if_eq(&'\\').is_some()) {
                        break;
                    }
                }
            }
            continue;
        }
        it.next();
        let mut params = String::new();
        let fin = loop {
            match it.next() {
                Some(x) if ('\x20'..'\x40').contains(&x) => params.push(x),
                other => break other,
            }
        };
        if fin != Some('m') {
            continue;
        }
        let ns: Vec<u32> = params.split([';', ':']).map(|x| x.parse().unwrap_or(0)).collect();
        let mut i = 0;
        while i < ns.len() {
            match ns[i] {
                0 => st = Style::default(),
                1 => st.bold = true,
                22 => st.bold = false,
                n @ (30..=37) => st.fg = Some(basic(n - 30)),
                n @ (90..=97) => st.fg = Some(basic(n - 90 + 8)),
                39 => st.fg = None,
                38 if ns.get(i + 1) == Some(&2) && i + 4 < ns.len() => {
                    st.fg = Some(Rgb(ns[i + 2] as u8, ns[i + 3] as u8, ns[i + 4] as u8));
                    i += 4;
                }
                38 if ns.get(i + 1) == Some(&5) && i + 2 < ns.len() => {
                    st.fg = Some(cube(ns[i + 2]));
                    i += 2;
                }
                // Backgrounds: skipped, the numbers of their colors too.
                48 if ns.get(i + 1) == Some(&2) => i += 4,
                48 if ns.get(i + 1) == Some(&5) => i += 2,
                _ => {}
            }
            i += 1;
        }
    }
    out
}

/// Only what changed since the last frame, to the screen.
fn show(s: &mut Screen, was: &mut Vec<Option<Cell>>, now: &[Option<Cell>]) {
    let w = s.w as usize;
    if was.len() != now.len() {
        s.clear();
        *was = vec![None; now.len()];
    }
    for (i, (a, b)) in was.iter_mut().zip(now).enumerate() {
        if a != b {
            let c = b.unwrap_or(Cell { ch: ' ', st: Style::default() });
            s.put((i / w) as i32 + 1, (i % w) as i32 + 1, &[c]);
            *a = *b;
        }
    }
    s.flush();
}

/// Motes drawn into a frame, in cells; later ones over earlier.
fn frame(s: &Screen, motes: &[Mote]) -> Vec<Option<Cell>> {
    let (w, h) = (s.w, s.h);
    let mut f = vec![None; (w * h).max(0) as usize];
    for m in motes {
        let (r, c) = (m.at.1.round() as i32, m.at.0.round() as i32);
        if r >= 0 && r < h && c >= 0 && c < w {
            f[(r * w + c) as usize] = Some(m.cell);
        }
    }
    f
}

/// The shell spun up into a galaxy, turning till `ready` has something;
/// that, and where the letters are, for `land`. None if ctrl-c was
/// pressed.
pub fn spin<T>(s: &mut Screen, rows: &[Vec<Cell>], th: &Theme, mut ready: impl FnMut() -> Option<T>) -> Option<(T, Vec<Mote>)> {
    use crossterm::event::{self, Event, KeyCode, KeyModifiers};
    let (w, h) = (s.w as f64, s.h as f64);
    let (cx, cy) = (w / 2.0, h / 2.0);
    let mut rng = crate::screen::Rng::new(7);
    let mut rand = || rng.below(1 << 20) as f64 / (1 << 20) as f64;
    // Where the cursor was: the bottom line with anything on it.
    let last = rows.iter().rposition(|r| r.iter().any(|c| c.ch != ' ')).unwrap_or(rows.len().saturating_sub(1)) as f64;
    let mut motes: Vec<Mote> = vec![];
    for (y, row) in rows.iter().enumerate() {
        let mut x = 0;
        for c in row {
            if c.ch != ' ' {
                motes.push(Mote { cell: *c, home: (x as f64, y as f64), at: (x as f64, y as f64), v: (0.0, 0.0), loose: 0.0, orbit: (0.0, 0.0, 0.0) });
            }
            x += unicode_width::UnicodeWidthChar::width(c.ch).unwrap_or(1);
        }
    }
    while motes.len() > MOST {
        let k = (rand() * motes.len() as f64) as usize;
        motes.swap_remove(k);
    }
    let reach = (w / 2.0 * 0.85).min(h * 0.85);
    for m in &mut motes {
        // Loose from the cursor out, a wave across a second or so.
        let d = ((m.home.0 - 0.0) / w * 0.3 + (last - m.home.1).abs() / h).min(1.3);
        m.loose = 0.15 + d * 0.8 + rand() * 0.15;
        // Its orbit: one of three arms, winding out.
        let r = 2.0 + reach * rand().powf(0.7);
        let arm = (rand() * 3.0).floor() * std::f64::consts::TAU / 3.0;
        m.orbit = (r, arm + r * 0.18 + rand() * 0.5, 1.1 / (r + 2.0).sqrt());
    }
    let born = Instant::now();
    let mut was = vec![];
    let mut last_t = 0.0;
    let mut got = None;
    loop {
        let t = born.elapsed().as_secs_f64();
        let dt = (t - last_t).min(0.05);
        last_t = t;
        if got.is_none() {
            got = ready();
        }
        // The wave's through, and the link's ready: land.
        if got.is_some() && t > 2.2 {
            return got.map(|g| (g, motes));
        }
        for m in &mut motes {
            if t < m.loose {
                continue;
            }
            let (r, ph, om) = m.orbit;
            let a = ph + om * t;
            let target = (cx + 2.0 * r * a.cos(), cy + r * a.sin());
            let k = 5.0;
            let acc = ((target.0 - m.at.0) * k - m.v.0 * 3.2, (target.1 - m.at.1) * k - m.v.1 * 3.2);
            m.v = (m.v.0 + acc.0 * dt, m.v.1 + acc.1 * dt);
            m.at = (m.at.0 + m.v.0 * dt, m.at.1 + m.v.1 * dt);
            // Its own color giving way to the galaxy's: warm at the middle,
            // the accent further out.
            let k = ((t - m.loose) / 1.5).min(1.0);
            let own = m.cell.st.fg.unwrap_or(th.fg);
            let sky = th.warm.mix(th.accent, (r / reach).min(1.0)).mix(th.fg, 0.15 * (1.0 + (t * 3.0 + ph).sin()));
            m.cell.st = Style { fg: Some(own.mix(sky, k)), ..m.cell.st };
        }
        let mut f = frame(s, &motes);
        if t > 1.2 {
            let dots = ".".repeat((t * 2.5) as usize % 4);
            let text = format!("shuffling your deque{dots:<3}");
            let col = (s.w - text.chars().count() as i32) / 2;
            for (i, ch) in text.chars().enumerate() {
                let at = (s.h / 2) as usize * s.w as usize + (col as usize + i);
                if at < f.len() {
                    f[at] = Some(Cell { ch, st: Style::fg(th.muted) });
                }
            }
        }
        show(s, &mut was, &f);
        if event::poll(Duration::from_millis(30)).unwrap_or(false)
            && let Ok(Event::Key(k)) = event::read()
            && k.code == KeyCode::Char('c')
            && k.modifiers.contains(KeyModifiers::CONTROL)
        {
            return None;
        }
    }
}

/// The letters fly from where they are to the first slide's cells, each
/// turning into the letter it lands as; those left over fly off, and
/// where there are too few, more come from the middle.
pub fn land(s: &mut Screen, mut motes: Vec<Mote>, targets: Vec<(i32, i32, Cell)>) {
    let (cx, cy) = (s.w as f64 / 2.0, s.h as f64 / 2.0);
    let angle = |x: f64, y: f64| (y - cy).atan2((x - cx) / 2.0);
    motes.sort_by(|a, b| angle(a.at.0, a.at.1).total_cmp(&angle(b.at.0, b.at.1)));
    let mut targets = targets;
    targets.sort_by(|a, b| angle(a.1 as f64 - 1.0, a.0 as f64 - 1.0).total_cmp(&angle(b.1 as f64 - 1.0, b.0 as f64 - 1.0)));
    // Each target takes a letter spread round the galaxy the same way it
    // is; letters left over fly off the edge, the way they're going.
    let (m, t) = (motes.len(), targets.len());
    let mut used = vec![false; m];
    let mut flights: Vec<((f64, f64), (f64, f64), Cell, Option<Cell>)> = vec![];
    for (i, &(r, c, cell)) in targets.iter().enumerate() {
        let j = (m > 0).then(|| i * m / t);
        if let Some(j) = j {
            used[j] = true;
        }
        let (from, start) = j.map_or(((cx, cy), cell), |j| (motes[j].at, motes[j].cell));
        flights.push((from, (c as f64 - 1.0, r as f64 - 1.0), start, Some(cell)));
    }
    for (_, mo) in motes.iter().enumerate().filter(|(j, _)| !used[*j]) {
        let a = angle(mo.at.0, mo.at.1);
        flights.push((mo.at, (cx + 2.2 * cx * a.cos(), cy + 2.2 * cy * a.sin()), mo.cell, None));
    }
    let mut was = vec![];
    let frames = 36;
    for f in 1..=frames {
        let k = ease(f, frames);
        let mut now = vec![None; (s.w * s.h).max(0) as usize];
        for (from, to, start, last) in &flights {
            // A little swing out as it goes, like it's still turning.
            let bow = (k * std::f64::consts::PI).sin() * 0.15;
            let (x, y) = (from.0 + (to.0 - from.0) * k - (to.1 - from.1) * bow * 2.0, from.1 + (to.1 - from.1) * k + (to.0 - from.0) * bow / 2.0);
            let cell = match last {
                Some(l) if k > 0.6 => *l,
                Some(l) => Cell { ch: start.ch, st: Style { fg: Some(start.st.fg.unwrap_or_default().mix(l.st.fg.unwrap_or(s.theme.fg), k)), ..start.st } },
                None if k > 0.85 => continue,
                None => *start,
            };
            let (r, c) = (y.round() as i32, x.round() as i32);
            if r >= 0 && r < s.h && c >= 0 && c < s.w {
                now[(r * s.w + c) as usize] = Some(cell);
            }
        }
        show(s, &mut was, &now);
        std::thread::sleep(Duration::from_millis(22));
    }
}
