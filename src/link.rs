//! The presenter and `deque notes` talking to each other through two small
//! files in the temp folder, named for the talk: the presenter writes where
//! it is, the notes window writes what it wants done.

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

pub struct Link {
    state: PathBuf,
    cmd: PathBuf,
    /// The last command taken, so each is done once.
    seen: String,
}

/// Written whole under another name, then moved into place, so a reader
/// never sees half of it.
fn write(p: &Path, s: &str) {
    let tmp = p.with_extension(format!("tmp{}", std::process::id()));
    if std::fs::write(&tmp, s).is_ok() {
        let _ = std::fs::rename(&tmp, p);
    }
}

impl Link {
    pub fn new(talk: &Path) -> Link {
        let abs = talk.canonicalize().unwrap_or(talk.to_path_buf());
        let mut h = std::collections::hash_map::DefaultHasher::new();
        abs.hash(&mut h);
        let base = std::env::temp_dir().join(format!("deque-{:016x}", h.finish()));
        let mut l = Link { state: base.with_extension("state"), cmd: base.with_extension("cmd"), seen: String::new() };
        // Whatever was asked before this presenter started isn't for it.
        l.seen = std::fs::read_to_string(&l.cmd).unwrap_or_default();
        l
    }

    /// The presenter: where it is, the slide and how many steps are shown.
    pub fn publish(&self, n: usize, shown: usize) {
        write(&self.state, &format!("{n} {shown}"));
    }

    pub fn gone(&self) {
        let _ = std::fs::remove_file(&self.state);
        let _ = std::fs::remove_file(self.state.with_extension("remote"));
    }

    /// The presenter: the phone remote's link, for the notes window.
    pub fn publish_remote(&self, url: &str) {
        write(&self.state.with_extension("remote"), url);
    }

    /// The notes window: the remote's link, while the talk's shared.
    pub fn remote(&self) -> Option<String> {
        std::fs::read_to_string(self.state.with_extension("remote")).ok()
    }

    /// The notes window: where the presenter is, if it's running.
    pub fn where_(&self) -> Option<(usize, usize)> {
        let s = std::fs::read_to_string(&self.state).ok()?;
        let (n, shown) = s.split_once(' ')?;
        Some((n.parse().ok()?, shown.trim().parse().ok()?))
    }

    /// The notes window: next, back, replay, first, last or goto N.
    pub fn send(&self, cmd: &str) {
        let t = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
        write(&self.cmd, &format!("{t} {cmd}"));
    }

    /// The presenter: a command not yet done, if there is one.
    pub fn take(&mut self) -> Option<String> {
        let s = std::fs::read_to_string(&self.cmd).ok()?;
        if s == self.seen {
            return None;
        }
        self.seen = s.clone();
        s.split_once(' ').map(|(_, c)| c.trim().to_string())
    }
}
