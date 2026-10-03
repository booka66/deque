//! A ```lang run block, run on its step in the talk's folder, its output
//! streamed in under the slide as it comes. A key stops it, and is then
//! taken as it would have been. Run again by --loop, what it printed last
//! time stays up till it's done, then turns into what it printed this time.

use crate::markup::{self, Cell, Line, Style};
use crate::morph::{self, Part, Role};
use crate::render;
use crate::screen::Screen;
use crate::talk::Talk;
use crossterm::event::{self, Event, KeyEventKind};
use std::io::{BufRead, BufReader, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};

#[derive(Default, Clone, PartialEq)]
pub struct Output {
    pub lines: Vec<String>,
    /// How it ended, when that's worth saying: a failure, or stopped.
    pub end: Option<String>,
}

/// Escapes gone, a tab four spaces, and a line a carriage return went back
/// over as it ended up.
fn clean(raw: &str) -> String {
    let raw = raw.trim_end_matches(['\n', '\r']);
    let raw = raw.rsplit('\r').next().unwrap_or(raw);
    let mut out = String::new();
    let mut cs = raw.chars().peekable();
    while let Some(c) = cs.next() {
        match c {
            '\x1b' => match cs.next() {
                // CSI: to its final byte.
                Some('[') => while cs.next().is_some_and(|c| !('@'..='~').contains(&c)) {},
                // OSC: to BEL or ST.
                Some(']') => {
                    while let Some(c) = cs.next() {
                        if c == '\x07' || (c == '\x1b' && cs.next_if_eq(&'\\').is_some()) {
                            break;
                        }
                    }
                }
                _ => {}
            },
            '\t' => out.push_str("    "),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out
}

/// The output's rows under slide n, as much of its end as fits, and where
/// they go: the first's row, and their column.
fn rows(s: &Screen, talk: &Talk, n: usize, out: &Output, running: bool) -> Option<(i32, i32, Vec<Line>)> {
    let (top, col) = render::under(s, &talk.slides[n])?;
    let bottom = s.h - 2;
    let mut rows: Vec<Line> = out.lines.iter().map(|l| markup::plain(l, Style::default())).collect();
    if let Some(e) = &out.end {
        rows.push(markup::plain(e, Style::fg(s.theme.bad)));
    }
    if rows.is_empty() && !running {
        rows.push(markup::plain("(no output)", s.muted()));
    }
    let fit = (bottom - top + 1).max(0) as usize;
    let room = (s.w - col + 1).max(0) as usize;
    let rows = rows[rows.len().saturating_sub(fit)..].iter().map(|l| l[..l.len().min(room)].to_vec()).collect();
    Some((top, col, rows))
}

/// The output under slide n, a bar beside it: the accent color while it
/// runs.
pub fn show(s: &mut Screen, talk: &Talk, n: usize, out: &Output, running: bool) {
    let Some((top, col, rows)) = rows(s, talk, n, out, running) else { return };
    let bar = Cell { ch: '▎', st: if running { s.accent() } else { s.muted() } };
    for (i, l) in rows.iter().enumerate() {
        let r = top + i as i32;
        s.clear_row(r);
        s.put(r, col - 2, &[bar]);
        s.put(r, col, l);
    }
    if running && rows.is_empty() {
        s.clear_row(top);
        s.put(top, col - 2, &[bar]);
    }
}

/// What slide n's block printed before turning into what it printed now:
/// lines in both swing to their new places, the rest fade out and in, the
/// slide over them left as it is.
fn turn(s: &mut Screen, talk: &Talk, n: usize, was: &Output, now: &Output) {
    let (Some((top, col, a)), Some((_, _, b))) = (rows(s, talk, n, was, false), rows(s, talk, n, now, false)) else { return };
    let slide = render::fixed(s, talk, n, talk.slides[n].steps());
    let bar = vec![Cell { ch: '▎', st: s.muted() }];
    let parts = |rows: Vec<Line>| -> Vec<Part> {
        let mut p = slide.clone();
        for (i, l) in rows.into_iter().enumerate() {
            p.push((top + i as i32, col - 2, bar.clone(), Role::Still));
            p.push((top + i as i32, col, l, Role::Moves));
        }
        p
    };
    // Rows it no longer has, emptied first: the morph only clears near
    // what moves.
    for r in top + b.len() as i32..top + a.len() as i32 {
        s.clear_row(r);
    }
    morph::play(s, &parts(a), &parts(b));
}

/// Slide n's block run, its output drawn as it comes; or, with what it
/// printed last time, that left up while it runs, then turned into this.
pub fn go(s: &mut Screen, talk: &Talk, n: usize, prev: Option<&Output>) -> Output {
    let run = talk.slides[n].run.as_ref().expect("a slide that runs");
    let mut out = Output::default();
    let mut cmd = Command::new(&run.argv[0]);
    cmd.args(&run.argv[1..]).current_dir(&talk.dir).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    // Its own process group, so stopping it stops what it started too.
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            out.end = Some(format!("{}: {e}", run.argv[0]));
            show(s, talk, n, &out, false);
            return out;
        }
    };
    let (tx, rx) = mpsc::channel();
    let pipes: [Box<dyn Read + Send>; 2] = [Box::new(child.stdout.take().unwrap()), Box::new(child.stderr.take().unwrap())];
    for p in pipes {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mut r = BufReader::new(p);
            let mut buf = vec![];
            while matches!(r.read_until(b'\n', &mut buf), Ok(1..)) {
                if tx.send(String::from_utf8_lossy(&buf).into_owned()).is_err() {
                    break;
                }
                buf.clear();
            }
        });
    }
    drop(tx);
    show(s, talk, n, prev.unwrap_or(&out), true);
    s.flush();
    let mut last = Instant::now();
    let mut stopped = false;
    loop {
        let got = match rx.recv_timeout(Duration::from_millis(15)) {
            Ok(l) => Some(l),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        s.sky_frame();
        s.flush();
        if s.rec.is_none() && event::poll(Duration::ZERO).unwrap_or(false) {
            match event::read() {
                Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => {
                    s.pending = Some(k);
                    stopped = true;
                    break;
                }
                Ok(Event::Resize(..)) => s.resized = true,
                Ok(Event::Mouse(m)) => s.mouse(m),
                _ => {}
            }
        }
        if let Some(l) = got {
            out.lines.push(clean(&l));
            // Kept to what could ever show.
            if out.lines.len() > 500 {
                out.lines.drain(..250);
            }
            // Recorded, the output comes when it did.
            if s.rec.is_some() {
                s.tick(last.elapsed().as_secs_f64());
                last = Instant::now();
            }
            if prev.is_none() {
                show(s, talk, n, &out, true);
                s.flush();
            }
        }
    }
    if stopped {
        #[cfg(unix)]
        unsafe {
            libc::kill(-(child.id() as i32), libc::SIGKILL);
        }
        #[cfg(not(unix))]
        let _ = child.kill();
        out.end = Some("stopped".into());
    }
    match child.wait() {
        Ok(st) if !st.success() && !stopped => out.end = Some(st.code().map_or("killed".into(), |c| format!("exit {c}"))),
        Err(e) => out.end = Some(e.to_string()),
        _ => {}
    }
    if s.rec.is_some() {
        s.tick(last.elapsed().as_secs_f64());
    }
    if let Some(was) = prev.filter(|p| **p != out && !talk.calm && !stopped) {
        turn(s, talk, n, was, &out);
    }
    show(s, talk, n, &out, false);
    s.flush();
    out
}

#[cfg(test)]
mod tests {
    use super::clean;

    #[test]
    fn cleaned() {
        assert_eq!(clean("\x1b[1;32mok\x1b[0m\n"), "ok");
        assert_eq!(clean("10%\r50%\r100%\r\n"), "100%");
        assert_eq!(clean("a\tb"), "a    b");
        assert_eq!(clean("\x1b]0;title\x07x"), "x");
    }
}
