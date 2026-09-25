//! deque preview TALK [--from FILE] [--still]: the slide an editor's cursor
//! is in, played in when the cursor comes to it, then drawn again, still, as
//! it changes. The editor writes FILE: the cursor's line (from 0) and a count
//! of replays asked for on the first line, then the talk as it is in the
//! buffer, saved or not. Pictures are found beside TALK. Without --from, it
//! follows TALK itself.

use crate::fx;
use crate::images;
use crate::markup::Style;
use crate::render::{self, Mode};
use crate::screen::Screen;
use crate::talk::{self, Talk};
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use std::path::Path;
use std::time::Duration;

/// What the editor wrote, or the talk as a file with the cursor at the top.
fn read(talk: &Path, from: Option<&Path>) -> Option<String> {
    match from {
        Some(f) => std::fs::read_to_string(f).ok(),
        None => Some(format!("0\n{}", std::fs::read_to_string(talk).ok()?)),
    }
}

/// Its first line, the cursor's line and how many replays have been asked
/// for, then the talk.
fn split(raw: &str) -> (usize, u64, &str) {
    let (head, src) = raw.split_once('\n').unwrap_or((raw, ""));
    let mut h = head.split_whitespace().map(|x| x.parse().unwrap_or(0));
    (h.next().unwrap_or(0) as usize, h.next().unwrap_or(0), src)
}

pub fn run(path: &Path, from: Option<&Path>, still: bool) -> Result<(), String> {
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let dir = dir.canonicalize().unwrap_or(dir.to_path_buf());
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    let proto = images::detect();
    let mut s = Screen::new(Default::default());
    s.kitty = proto == images::Proto::Kitty;
    let mut pics = images::Pictures::new(proto);
    s.raw("\x1b[?1049h\x1b[?25l");
    let mut seen = String::new();
    let mut last: Option<Talk> = None;
    let mut error: Option<String> = None;
    let mut drawn = String::new();
    // The slide last shown and the replay count then: a slide plays in when
    // the cursor comes into it or a replay is asked for, not as it's typed in.
    let mut showing: Option<(usize, u64)> = None;
    loop {
        // Read each time: it's small, and a time stamp can miss two writes
        // in the same second.
        if let Some(now) = read(path, from).filter(|r| *r != seen) {
            seen = now;
            let (t, diags) = talk::parse(split(&seen).2, &dir, true);
            error = diags.iter().find(|d| !d.warn).map(|d| format!("line {}: {}", d.line + 1, d.msg));
            // A talk that doesn't parse yet keeps its last good slides up.
            if !t.slides.is_empty() && error.is_none() {
                last = Some(t);
            }
        }
        let (row, seq, src) = split(&seen);
        s.size();
        let frame = format!("{row} {seq}\n{}x{}\n{error:?}\n{src}", s.w, s.h);
        if frame != drawn {
            drawn = frame;
            match &last {
                Some(t) => {
                    s.theme = t.theme.clone();
                    // The slide the cursor is in: the last to start at or above it.
                    let n = t.slides.iter().rposition(|sl| sl.line <= row).unwrap_or(0);
                    let steps = t.slides[n].steps();
                    let play = !still && showing != Some((n, seq));
                    if play {
                        // Played in, and cut short by an edit, a replay, or
                        // the cursor leaving the slide; moving about in it
                        // lets it play.
                        let lines = t.slides[n].line..t.slides.get(n + 1).map_or(usize::MAX, |x| x.line);
                        let (snap, p, f) = (src.to_string(), path.to_path_buf(), from.map(Path::to_path_buf));
                        s.watch = Some(Box::new(move || {
                            read(&p, f.as_deref()).is_some_and(|r| {
                                let (row, again, now) = split(&r);
                                now != snap || again != seq || !lines.contains(&row)
                            })
                        }));
                        s.hurry = false;
                        if showing.is_some_and(|(was, _)| was != n) {
                            fx::transition(&mut s, &t.tr(&t.slides[n]));
                        }
                        render::draw(&mut s, t, &mut pics, n, Mode::Arrive, 0, false);
                        for k in 1..=steps {
                            s.tick(0.6);
                            if s.hurry {
                                break;
                            }
                            render::draw(&mut s, t, &mut pics, n, Mode::Step, k, false);
                        }
                        s.watch = None;
                    }
                    if !play || s.hurry {
                        s.raw("\x1b[?2026h");
                        s.hurry = true;
                        render::draw(&mut s, t, &mut pics, n, Mode::Still, steps, false);
                        s.raw("\x1b[?2026l");
                    }
                    s.hurry = false;
                    render::focus(&mut s, t, n, row);
                    showing = Some((n, seq));
                }
                None => {
                    s.clear();
                    let st = s.muted();
                    s.put_str(2, 2, "no slides yet: a slide starts at a line of ---", st);
                }
            }
            if let Some(e) = &error {
                let st = Style::fg(s.theme.bad);
                s.put_str(s.h, 1, e, st);
            }
            s.flush();
        }
        let key = match s.pending.take() {
            Some(k) => Some(k),
            None if event::poll(Duration::from_millis(60)).unwrap_or(false) => match event::read() {
                Ok(Event::Key(k)) => Some(k),
                _ => None,
            },
            None => None,
        };
        if let Some(k) = key {
            let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
            if k.kind != KeyEventKind::Release && (k.code == KeyCode::Char('q') || (ctrl && k.code == KeyCode::Char('c'))) {
                break;
            }
        }
    }
    let _ = terminal::disable_raw_mode();
    s.raw("\x1b[0m\x1b[?1049l\x1b[?25h");
    s.flush();
    Ok(())
}
