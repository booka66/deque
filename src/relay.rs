//! An `enter:` command run while watchers are watching: in a terminal of
//! deque's own (a pseudo-terminal) the screen's size, so what it draws can
//! go to them as well as to the screen. Keys go to it as they're typed;
//! what it draws goes out as it comes, to the watchers only what drawing
//! needs (see share.rs). Unix only; elsewhere the command has the terminal
//! to itself and watchers see the slide it left.

use crate::share::Hub;
use std::io::Write;
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
/// its keys to pass through untouched.
pub fn run(cmd: &str, dir: &Path, hub: &Hub) -> std::io::Result<()> {
    let mut now = size();
    let (mut master, mut slave) = (0, 0);
    let ws = winsize(now);
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
            let ws = winsize(now);
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
            carry.extend_from_slice(got);
            let whole = crate::share::whole(&carry);
            hub.send(&carry[..whole], now.0 as i32, now.1 as i32);
            carry.drain(..whole);
        }
        if let Ok(Some(_)) = child.try_wait()
            && fds[1].revents & libc::POLLIN == 0
        {
            break;
        }
    }
    child.wait()?;
    Ok(())
}
