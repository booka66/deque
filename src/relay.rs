//! An `enter:` command run while watchers are watching: in a terminal of
//! deque's own (a pseudo-terminal) the screen's size, so what it draws can
//! go to them as well as to the screen. Keys go to it as they're typed;
//! what it draws goes out as it comes, to the watchers only what drawing
//! needs (see share.rs). Unix only; elsewhere the command has the terminal
//! to itself and watchers see the slide it left.
//!
//! With `keys: on` the command's terminal is a row shorter than the screen,
//! and the row under it shows the keys as they're pressed (Strip, below).

use crate::markup::Rgb;
use crate::share::Hub;
use std::io::Write;
use std::time::{Duration, Instant};

/// How long the keys stay up after the last one.
const LINGER: Duration = Duration::from_millis(2500);

/// The keys in what was just typed, each named as its keycap is. What the
/// terminal itself sends (the mouse, answers to what the command asked it)
/// isn't a key and is left out.
fn keys(typed: &[u8]) -> Vec<String> {
    let c: Vec<char> = String::from_utf8_lossy(typed).chars().collect();
    let (mut out, mut i) = (vec![], 0);
    while i < c.len() {
        let ch = c[i];
        i += 1;
        let name = match ch {
            '\x1b' => match c.get(i).copied() {
                None => "esc".to_string(),
                Some('[' | 'O') => {
                    let from = i + 1;
                    i = from;
                    while i < c.len() && !('\x40'..='\x7e').contains(&c[i]) {
                        i += 1;
                    }
                    let Some(&end) = c.get(i) else { break };
                    i += 1;
                    let args: String = c[from..i - 1].iter().collect();
                    match (end, args.split(';').next().unwrap_or("")) {
                        ('A', _) => "↑",
                        ('B', _) => "↓",
                        ('C', _) => "→",
                        ('D', _) => "←",
                        ('H', _) => "home",
                        ('F', _) => "end",
                        ('Z', _) => "⇧⇥",
                        ('~', "3") => "del",
                        ('~', "5") => "pgup",
                        ('~', "6") => "pgdn",
                        _ => continue,
                    }
                    .to_string()
                }
                // An answer from the terminal: to its end, a bell or ESC \.
                Some(']' | '_' | 'P' | '^') => {
                    while i < c.len() && c[i] != '\x07' && !(c[i] == '\\' && c[i - 1] == '\x1b') {
                        i += 1;
                    }
                    i += 1;
                    continue;
                }
                Some(k) => {
                    i += 1;
                    format!("⌥{k}")
                }
            },
            '\r' | '\n' => "⏎".to_string(),
            '\t' => "⇥".to_string(),
            '\x7f' | '\x08' => "⌫".to_string(),
            ' ' => "␣".to_string(),
            k if (k as u32) < 32 => format!("^{}", (k as u8 + 64) as char),
            k => k.to_string(),
        };
        out.push(name);
    }
    out
}

/// The row of keys under a live command: the last few pressed, the newest
/// lit, one pressed again counted rather than repeated.
pub struct Strip {
    accent: Rgb,
    muted: Rgb,
    bg: Rgb,
    pressed: Vec<(String, usize)>,
    last: Instant,
    dirty: bool,
}

impl Strip {
    pub fn new(accent: Rgb, muted: Rgb, bg: Rgb) -> Strip {
        Strip { accent, muted, bg, pressed: vec![], last: Instant::now(), dirty: true }
    }

    fn typed(&mut self, b: &[u8]) {
        for k in keys(b) {
            match self.pressed.last_mut() {
                Some((was, n)) if *was == k => *n += 1,
                _ => self.pressed.push((k, 1)),
            }
            (self.last, self.dirty) = (Instant::now(), true);
        }
    }

    /// The row as it should be now, drawn on row `rows` with the cursor put
    /// back where the command had it; nothing, if it's as it was.
    fn draw(&mut self, (cols, rows): (u16, u16)) -> Option<Vec<u8>> {
        if !self.pressed.is_empty() && self.last.elapsed() > LINGER {
            self.pressed.clear();
            self.dirty = true;
        }
        if !std::mem::take(&mut self.dirty) {
            return None;
        }
        let cap = |(k, n): &(String, usize)| if *n > 1 { format!(" {k} ×{n} ") } else { format!(" {k} ") };
        let wide = |p: &[(String, usize)]| p.iter().map(|x| cap(x).chars().count() + 1).sum::<usize>();
        while wide(&self.pressed) + 2 > cols as usize && self.pressed.len() > 1 {
            self.pressed.remove(0);
        }
        let col = (cols as usize).saturating_sub(wide(&self.pressed) + 1).max(1);
        let mut out = format!("\x1b7\x1b[{rows};1H\x1b[0m\x1b[2K\x1b[{rows};{col}H");
        let Rgb(r, g, b) = self.bg;
        for (i, k) in self.pressed.iter().enumerate() {
            let Rgb(cr, cg, cb) = if i + 1 == self.pressed.len() { self.accent } else { self.muted };
            out.push_str(&format!("\x1b[1;38;2;{r};{g};{b};48;2;{cr};{cg};{cb}m{}\x1b[0m ", cap(k)));
        }
        out.push_str("\x1b8");
        Some(out.into_bytes())
    }
}
use std::os::fd::{FromRawFd, OwnedFd};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

fn size() -> (u16, u16) {
    crossterm::terminal::size().unwrap_or((80, 24))
}

fn winsize((cols, rows): (u16, u16)) -> libc::winsize {
    libc::winsize { ws_row: rows, ws_col: cols, ws_xpixel: 0, ws_ypixel: 0 }
}

/// cmd run by sh in dir, till it's done, the terminal left in raw mode for
/// its keys to pass through untouched. What it draws goes to hub's watchers
/// too; with a strip, the bottom row is kept from it for the keys pressed.
pub fn run(cmd: &str, dir: &Path, hub: Option<&Hub>, mut strip: Option<Strip>) -> std::io::Result<()> {
    let mut now = size();
    // The command's terminal: the screen, less the strip's row. The screen
    // scrolls above that row only, so a line past its end doesn't take it.
    let room = |(cols, rows): (u16, u16), strip: &Option<Strip>| match strip {
        Some(_) => {
            print!("\x1b[1;{}r\x1b[H", rows.saturating_sub(1).max(1));
            (cols, rows.saturating_sub(1).max(1))
        }
        None => (cols, rows),
    };
    let (mut master, mut slave) = (0, 0);
    let ws = winsize(room(now, &strip));
    // Mutable where macOS says so, const where Linux does: *mut serves both.
    let wp = (&ws as *const libc::winsize).cast_mut();
    if unsafe { libc::openpty(&mut master, &mut slave, std::ptr::null_mut(), std::ptr::null_mut(), wp) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let (master, slave) = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
    let mut c = Command::new("sh");
    c.arg("-c").arg(cmd).current_dir(dir);
    c.stdin(Stdio::from(slave.try_clone()?)).stdout(Stdio::from(slave.try_clone()?)).stderr(Stdio::from(slave.try_clone()?));
    // Its own session, the pseudo-terminal its controlling terminal, so ^C
    // and job control go to it.
    let sfd = std::os::fd::AsRawFd::as_raw_fd(&slave);
    unsafe {
        c.pre_exec(move || {
            if libc::setsid() < 0 || libc::ioctl(sfd, libc::TIOCSCTTY as _, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = c.spawn()?;
    drop(slave);
    let mfd = std::os::fd::AsRawFd::as_raw_fd(&master);
    let mut out = std::io::stdout();
    // An escape cut off at the end of one read, kept for the next.
    let mut carry: Vec<u8> = vec![];
    let mut buf = [0u8; 16384];
    loop {
        // The screen resized: so is its terminal.
        let s = size();
        if s != now {
            now = s;
            let ws = winsize(room(now, &strip));
            if let Some(st) = &mut strip {
                st.dirty = true;
            }
            unsafe { libc::ioctl(mfd, libc::TIOCSWINSZ, &ws) };
        }
        let mut fds = [libc::pollfd { fd: 0, events: libc::POLLIN, revents: 0 }, libc::pollfd { fd: mfd, events: libc::POLLIN, revents: 0 }];
        if unsafe { libc::poll(fds.as_mut_ptr(), 2, 50) } < 0 {
            continue;
        }
        if fds[0].revents & libc::POLLIN != 0 {
            let n = unsafe { libc::read(0, buf.as_mut_ptr().cast(), buf.len()) };
            if n > 0 {
                unsafe { libc::write(mfd, buf.as_ptr().cast(), n as usize) };
                if let Some(st) = &mut strip {
                    st.typed(&buf[..n as usize]);
                }
            }
        }
        // The keys, drawn once the command has gone quiet, so they never
        // land in the middle of something it's drawing; and again after
        // it has drawn, since it may have cleared them.
        if fds[1].revents & libc::POLLIN == 0
            && let Some(row) = strip.as_mut().and_then(|st| st.draw(now))
        {
            out.write_all(&row)?;
            out.flush()?;
            if let Some(hub) = hub {
                hub.send(&row, now.0 as i32, now.1 as i32);
            }
        }
        if fds[1].revents & (libc::POLLIN | libc::POLLHUP) != 0 {
            let n = unsafe { libc::read(mfd, buf.as_mut_ptr().cast(), buf.len()) };
            // The command's gone and its terminal with it.
            if n <= 0 {
                break;
            }
            let got = &buf[..n as usize];
            out.write_all(got)?;
            out.flush()?;
            if let Some(st) = &mut strip {
                st.dirty = true;
            }
            if let Some(hub) = hub {
                carry.extend_from_slice(got);
                let whole = crate::share::whole(&carry);
                hub.send(&carry[..whole], now.0 as i32, now.1 as i32);
                carry.drain(..whole);
            }
        }
        if let Ok(Some(_)) = child.try_wait()
            && fds[1].revents & libc::POLLIN == 0
        {
            break;
        }
    }
    child.wait()?;
    if strip.is_some() {
        out.write_all(b"\x1b[r")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_named_and_the_terminals_own_answers_left_out() {
        assert_eq!(keys(b"gm\r \x1b[A\x03\x1b"), ["g", "m", "⏎", "␣", "↑", "^C", "esc"]);
        // The mouse, a cursor report, and a kitty graphics answer.
        assert!(keys(b"\x1b[<35;10;4M\x1b[12;40R\x1b_Gi=1;OK\x1b\\").is_empty());
    }

    #[test]
    fn the_strip_counts_a_key_pressed_again() {
        let mut st = Strip::new(Rgb(1, 2, 3), Rgb(4, 5, 6), Rgb(0, 0, 0));
        st.typed(b"jjjV");
        let row = String::from_utf8(st.draw((80, 24)).unwrap()).unwrap();
        assert!(row.starts_with("\x1b7\x1b[24;1H") && row.ends_with("\x1b8"), "{row:?}");
        assert!(row.contains(" j ×3 ") && row.contains("48;2;1;2;3m V "), "{row:?}");
        // Nothing new: nothing drawn.
        assert!(st.draw((80, 24)).is_none());
    }
}
