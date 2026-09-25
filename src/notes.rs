//! deque notes TALK: the presenter's view, for a second screen. It follows
//! wherever the talk is, shows the slide's notes (its `//` lines), what comes
//! next and how long it's been, and its keys drive the talk, so the laptop
//! can keep the keyboard while the TV shows the slides.

use crate::link::Link;
use crate::markup::{self, Style};
use crate::screen::Screen;
use crate::talk::{Slide, Talk};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use std::path::Path;
use std::time::{Duration, Instant, SystemTime};
use unicode_width::UnicodeWidthStr;

pub fn run(path: &Path, load: impl Fn(&Path) -> Result<Talk, String>) -> Result<(), String> {
    let mut talk = load(path)?;
    let link = Link::new(path);
    let mut s = Screen::new(talk.theme.clone());
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    s.raw("\x1b[?1049h\x1b[?25l");
    let mut stamp = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let mut since = Instant::now();
    let mut jump = String::new();
    let mut last = String::new();
    loop {
        let now = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        if now != stamp {
            stamp = now;
            if let Ok(t) = load(path) {
                s.theme = t.theme.clone();
                talk = t;
            }
        }
        // Drawn again only when something on it changed, the timer's second
        // included.
        s.size();
        let at = link.where_();
        let frame = format!("{at:?} {} {jump} {:?} {} {}", since.elapsed().as_secs(), stamp, s.w, s.h);
        if frame != last {
            last = frame;
            draw(&mut s, &talk, at, since, &jump);
        }
        if !event::poll(Duration::from_millis(200)).unwrap_or(false) {
            continue;
        }
        let Ok(ev) = event::read() else { continue };
        let Event::Key(k) = ev else {
            s.size();
            continue;
        };
        if k.kind == KeyEventKind::Release {
            continue;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Char('c') if ctrl => break,
            KeyCode::Esc if !jump.is_empty() => jump.clear(),
            KeyCode::Char('q') | KeyCode::Esc => break,
            KeyCode::Char(d) if d.is_ascii_digit() => jump.push(d),
            KeyCode::Backspace => {
                jump.pop();
            }
            KeyCode::Enter if !jump.is_empty() => {
                let k = jump.parse::<usize>().unwrap_or(1).max(1) - 1;
                link.send(&format!("goto {k}"));
                jump.clear();
            }
            KeyCode::Char('t') => since = Instant::now(),
            KeyCode::Char('r') => link.send("replay"),
            KeyCode::Home | KeyCode::Char('g') => link.send("first"),
            KeyCode::End | KeyCode::Char('G') => link.send("last"),
            KeyCode::Left | KeyCode::PageUp | KeyCode::Char('b' | 'p' | 'h' | 'k') => link.send("back"),
            KeyCode::Right | KeyCode::PageDown | KeyCode::Enter | KeyCode::Char(' ' | 'n' | 'l' | 'j') => link.send("next"),
            _ => {}
        }
    }
    let _ = terminal::disable_raw_mode();
    s.raw("\x1b[0m\x1b[?1049l\x1b[?25h");
    s.flush();
    Ok(())
}

/// Words to lines no wider than w.
fn wrap(t: &str, w: usize) -> Vec<String> {
    let mut out = vec![];
    let mut line = String::new();
    for word in t.split(' ') {
        if !line.is_empty() && line.width() + 1 + word.width() > w {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    out.push(line);
    out
}

/// The step's text, or the slide's.
fn what(slide: &Slide, step: Option<usize>) -> String {
    match step {
        Some(k) => slide.body.iter().filter(|b| b.step == k).map(|b| markup::text(&b.line).trim().to_string()).collect::<Vec<_>>().join(" "),
        None => slide.title(),
    }
}

fn draw(s: &mut Screen, talk: &Talk, at: Option<(usize, usize)>, since: Instant, jump: &str) {
    // One synchronized frame where the terminal supports it, so no flicker.
    s.raw("\x1b[?2026h");
    s.clear();
    let (acc, mut_) = (s.accent(), s.muted());
    let bold = Style { bold: true, ..acc };
    let total = talk.slides.len();
    let (n, shown) = at.unwrap_or((0, 0));
    let n = n.min(total - 1);
    let slide = &talk.slides[n];
    let w = (s.w - 4).max(10) as usize;

    // The top line: where the talk is, how long it's been, the time.
    let steps = slide.steps();
    let mut place = format!("{} / {total}", n + 1);
    if steps > 0 {
        place += &format!(" · step {}/{steps}", shown.min(steps));
    }
    s.put_str(1, 2, &place, mut_);
    let e = since.elapsed().as_secs();
    let clock = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let timer = format!("{:02}:{:02}", e / 60, e % 60);
    let right = format!("{timer}   {}", local_time(clock));
    s.put_str(1, s.w - right.width() as i32 - 1, &timer, acc);
    s.put_str(1, s.w - right.width() as i32 - 1 + timer.len() as i32, &right[timer.len()..], mut_);
    if at.is_none() {
        s.put_str(2, 2, "the talk isn't running: start it with deque, and this follows it", Style::fg(s.theme.bad));
    }

    s.put_str(3, 2, "now", mut_);
    s.put_str(4, 2, &slide.title(), bold);
    let mut row = 6;
    let bottom = s.h - 7;
    if slide.notes.iter().all(|l| l.trim().is_empty()) {
        s.put_str(row, 2, "no notes: // lines in a slide are its notes", mut_);
    }
    for note in &slide.notes {
        for l in wrap(note, w) {
            if row > bottom {
                break;
            }
            s.put_str(row, 2, &l, Style::fg(s.theme.fg));
            row += 1;
        }
    }

    // What the next key brings: the slide's next step, or the next slide.
    let rule: String = "─".repeat(w);
    s.put_str(s.h - 5, 2, &rule, mut_);
    let next = if shown < steps {
        Some(("next step", what(slide, Some(shown + 1))))
    } else if n + 1 < total {
        Some(("next", format!("{} · {}", n + 2, what(&talk.slides[n + 1], None))))
    } else {
        None
    };
    match next {
        Some((k, t)) => {
            s.put_str(s.h - 4, 2, k, mut_);
            let t: String = t.chars().take(w).collect();
            s.put_str(s.h - 3, 2, &t, Style { bold: true, ..Style::default() });
        }
        None => s.put_str(s.h - 4, 2, "the end", mut_),
    }
    let keys = if jump.is_empty() { "→ next · ← back · 12⏎ slide 12 · r replay · t timer to 0 · q quit".to_string() } else { format!("go to {jump}_") };
    s.put_str(s.h - 1, 2, &keys, if jump.is_empty() { mut_ } else { acc });
    s.raw("\x1b[?2026l");
    s.flush();
}

/// Hours and minutes of the day, local time.
fn local_time(secs: u64) -> String {
    #[cfg(unix)]
    {
        let t = secs as libc::time_t;
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        unsafe { libc::localtime_r(&t, &mut tm) };
        format!("{:02}:{:02}", tm.tm_hour, tm.tm_min)
    }
    #[cfg(not(unix))]
    {
        format!("{:02}:{:02} UTC", secs / 3600 % 24, secs / 60 % 60)
    }
}
