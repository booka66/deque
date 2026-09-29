//! `play: demo.cast`: a demo recorded beforehand (by asciinema, or deque's
//! own --cast), played on enter instead of run live, so it can't go wrong
//! on the day. It plays as it was recorded, long pauses cut short; space
//! pauses it, and it pauses itself at the recording's markers. When the
//! slide has an `enter:` command too, enter cuts from the recording to the
//! real thing.

use crate::screen::Screen;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use std::path::Path;
use std::time::{Duration, Instant};

/// The longest a pause in the recording is kept, in seconds.
const IDLE: f64 = 1.5;

pub struct Cast {
    pub w: i32,
    pub h: i32,
    /// What it drew, and when, in seconds from the start, pauses cut
    /// short; or a marker, None, to pause at.
    pub events: Vec<(f64, Option<String>)>,
}

/// A cast file read: asciinema's v2 (times from the start) or v3 (times
/// from the event before).
pub fn load(path: &Path) -> Result<Cast, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut lines = text.lines().filter(|l| !l.trim().is_empty() && !l.starts_with('#'));
    let head: serde_json::Value = lines.next().and_then(|l| serde_json::from_str(l).ok()).ok_or("not an asciinema cast: its first line isn't its header")?;
    let version = head["version"].as_i64().unwrap_or(0);
    let (w, h) = match version {
        2 => (head["width"].as_i64(), head["height"].as_i64()),
        3 => (head["term"]["cols"].as_i64(), head["term"]["rows"].as_i64()),
        _ => return Err(format!("an asciinema cast of version {version}; deque plays 2 and 3")),
    };
    let (Some(w), Some(h)) = (w, h) else { return Err("the cast's header has no size".into()) };
    let (mut events, mut was, mut at) = (vec![], 0.0, 0.0);
    for (k, l) in lines.enumerate() {
        let bad = || format!("event {} isn't [time, code, data]", k + 1);
        let e: serde_json::Value = serde_json::from_str(l).map_err(|_| bad())?;
        let (Some(t), Some(code), Some(data)) = (e[0].as_f64(), e[1].as_str(), e[2].as_str()) else { return Err(bad()) };
        let t = if version == 3 { was + t } else { t };
        at += (t - was).clamp(0.0, IDLE);
        was = t;
        match code {
            "o" => events.push((at, Some(data.to_string()))),
            "m" => events.push((at, None)),
            _ => {}
        }
    }
    Ok(Cast { w: w as i32, h: h as i32, events })
}

/// What the presenter asked for during a recording.
pub enum Then {
    /// Back to the slide.
    Back,
    /// On to the slide's command, live.
    Live,
}

/// The cast played on the screen, from its top left, till it's done and a
/// key's pressed, or it's stopped. `live`: enter goes to the real thing.
pub fn play(s: &mut Screen, cast: &Cast, live: bool) -> Then {
    // The sky would draw over it; it's put back after.
    let sky = s.sky.take();
    s.raw("\x1b[0m\x1b[H\x1b[2J");
    s.flush();
    let (mut i, mut clock, mut paused) = (0, 0.0, false);
    let mut last = Instant::now();
    let then = loop {
        let now = Instant::now();
        if !paused {
            clock += (now - last).as_secs_f64();
        }
        last = now;
        while !paused && i < cast.events.len() && cast.events[i].0 <= clock {
            match &cast.events[i].1 {
                Some(o) => s.raw(o),
                None => paused = true,
            }
            i += 1;
        }
        s.flush();
        let wait = match cast.events.get(i) {
            Some((t, _)) if !paused => Duration::from_secs_f64((t - clock).clamp(0.001, 0.05)),
            _ => Duration::from_millis(50),
        };
        if !event::poll(wait).unwrap_or(false) {
            continue;
        }
        let Ok(Event::Key(k)) = event::read() else { continue };
        if k.kind == KeyEventKind::Release {
            continue;
        }
        let done = i >= cast.events.len();
        match k.code {
            KeyCode::Enter if live => break Then::Live,
            KeyCode::Char('c') if k.modifiers.contains(KeyModifiers::CONTROL) => break Then::Back,
            KeyCode::Esc | KeyCode::Char('q') => break Then::Back,
            _ if done => break Then::Back,
            KeyCode::Char(' ') => paused = !paused,
            // On to the next marker, or the end, at once.
            KeyCode::Right | KeyCode::Char('n' | 'l') => {
                while let Some((t, e)) = cast.events.get(i) {
                    clock = *t;
                    i += 1;
                    match e {
                        Some(o) => s.raw(o),
                        None => break,
                    }
                }
                paused = i < cast.events.len();
            }
            _ => {}
        }
    };
    s.raw("\x1b[0m\x1b[H\x1b[2J");
    s.flush();
    s.sky = sky;
    then
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cast(name: &str, text: &str) -> Result<Cast, String> {
        let p = std::env::temp_dir().join(format!("deque-{name}-{}.cast", std::process::id()));
        std::fs::write(&p, text).unwrap();
        let c = load(&p);
        std::fs::remove_file(p).unwrap();
        c
    }

    #[test]
    fn casts_of_either_version_with_long_pauses_cut() {
        let v2 = cast("v2", "{\"version\": 2, \"width\": 80, \"height\": 24}\n[0.5, \"o\", \"$ \"]\n[10.5, \"o\", \"ls\"]\n[11, \"m\", \"\"]\n[11.2, \"i\", \"x\"]\n").unwrap();
        assert_eq!((v2.w, v2.h), (80, 24));
        assert_eq!(v2.events, [(0.5, Some("$ ".into())), (2.0, Some("ls".into())), (2.5, None)]);
        let v3 = cast("v3", "{\"version\": 3, \"term\": {\"cols\": 100, \"rows\": 30}}\n[0.5, \"o\", \"a\"]\n[0.25, \"o\", \"b\"]\n").unwrap();
        assert_eq!((v3.w, v3.h, v3.events), (100, 30, vec![(0.5, Some("a".into())), (0.75, Some("b".into()))]));
        assert!(cast("bad", "hello\n").is_err());
        assert!(cast("bad2", "{\"version\": 2, \"width\": 80, \"height\": 24}\n[\"x\"]\n").is_err());
    }
}
