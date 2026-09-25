//! --tv: the talk fullscreen in a new window of the running Ghostty, at a
//! font that makes the screen about DEQUE_COLS (80) columns wide, and + and
//! − to size it. Setting a terminal's font from inside it has no portable
//! way, so this is Ghostty on macOS only, through its AppleScript; anywhere
//! else the layout fits whatever size the window is and you zoom yourself.

use crate::screen::Screen;
use std::process::Command;

pub fn cols() -> i32 {
    std::env::var("DEQUE_COLS").ok().and_then(|c| c.parse().ok()).unwrap_or(80)
}

/// The font the deck is at, when --tv opened it; None otherwise.
pub struct Font {
    pub size: i32,
    title: String,
}

impl Font {
    pub fn from_env() -> Option<Font> {
        let size = std::env::var("DEQUE_FONT").ok()?.parse().ok().filter(|&f| f > 0)?;
        Some(Font { size, title: format!("deque-{}", std::process::id()) })
    }

    /// A Ghostty action on the deck's window, which goes by a title of its
    /// own, set again before each change (a live part may have retitled it),
    /// so an action never lands on another window. Then the size again.
    fn action(&self, s: &mut Screen, a: &str) {
        s.raw(&format!("\x1b]2;{}\x07", self.title));
        s.flush();
        let script = format!("tell application \"Ghostty\" to perform action \"{a}\" on (first terminal whose name is \"{}\")", self.title);
        let _ = Command::new("osascript").args(["-e", &script]).output();
        std::thread::sleep(std::time::Duration::from_millis(300));
        s.size();
    }

    pub fn set(&self, s: &mut Screen, size: i32) {
        self.action(s, &format!("set_font_size:{size}"));
    }

    /// It starts at the font that makes the window DEQUE_COLS wide, measured
    /// once the window has settled: --tv's guess from the screen is only a
    /// guess, as macOS may give the width of every screen at once.
    pub fn fit(&mut self, s: &mut Screen) {
        std::thread::sleep(std::time::Duration::from_millis(1200));
        s.size();
        self.size = (self.size * s.w / cols()).max(8);
        self.set(s, self.size);
    }

    /// A tenth up or down.
    pub fn grow(&mut self, s: &mut Screen, by: i32) {
        self.size = (self.size + by * (self.size / 10).max(1)).max(8);
        self.set(s, self.size);
    }

    /// Small enough that the screen is `cols` wide, for a live part.
    pub fn small(&self, s: &mut Screen, cols: i32) {
        self.set(s, (self.size * s.w / cols).max(4));
    }
}

/// Opens the talk in a new fullscreen Ghostty window: this same deque, the
/// same arguments but --tv, and the environment the talk reads.
pub fn tv(args: &[String], vars: &[String]) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("--tv sizes the font through Ghostty's AppleScript, so it's macOS and Ghostty only. Elsewhere, make the window fullscreen and zoom the terminal".into());
    }
    let out = Command::new("osascript")
        .args(["-e", "tell application \"Finder\" to get item 3 of (get bounds of window of desktop)"])
        .output()
        .map_err(|e| format!("osascript: {e}"))?;
    let width: i32 = String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(1920);
    // A monospaced cell is about 0.6 of the font's size wide.
    let font = std::env::var("DEQUE_FONT").ok().and_then(|f| f.parse().ok()).unwrap_or(width * 10 / (cols() * 6));
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let q = |s: &str| format!("'{}'", s.replace('\'', "'\\''"));
    let cmd = std::iter::once(q(&exe.to_string_lossy())).chain(args.iter().map(|a| q(a))).collect::<Vec<_>>().join(" ");
    let mut env = vec![format!("DEQUE_FONT={font}"), format!("DEQUE_COLS={}", cols())];
    let mut names = vec!["PATH".to_string()];
    names.extend(std::env::var("DEQUE_ENV").unwrap_or_default().split_whitespace().map(String::from));
    names.extend(vars.iter().cloned());
    for n in names {
        if let Ok(v) = std::env::var(&n) {
            env.push(format!("{n}={v}"));
        }
    }
    let script = r#"on run argv
  tell application "Ghostty"
    set cfg to new surface configuration
    set font size of cfg to (item 1 of argv) as real
    set command of cfg to item 2 of argv
    set environment variables of cfg to items 3 thru -1 of argv
    set wait after command of cfg to false
    set w to new window with configuration cfg
    perform action "toggle_fullscreen" on focused terminal of selected tab of w
  end tell
end run"#;
    let st = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .arg(font.to_string())
        .arg(cmd)
        .args(env)
        .status()
        .map_err(|e| format!("osascript: {e}"))?;
    if st.success() { Ok(()) } else { Err("Ghostty didn't open the window; is it running?".into()) }
}
