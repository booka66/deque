//! One ant colony in two panes. Two deques showing the same talk find each
//! other by files in the temp folder, one set for each colony: the first
//! keeps the colony, and the other shows it as it is, from the side or
//! from above as its own slide and its own v say, on a screen of its own
//! size. Food dropped in either falls in the one colony.
//!
//! The keeper also writes the colony to a file under ~/.deque/colonies
//! every little while, and a colony begins as it was last written there:
//! it lives on from one run of deque to the next.
//!
//! The keeper writes `owner` twice a second, and, while the other writes
//! `want`, the colony itself to `state` twenty times a second. When the
//! keeper goes, or goes to a slide with no ants, the other takes the
//! colony up from where it was.

use crate::ants::Colony;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime};

static TALK: OnceLock<(PathBuf, bool)> = OnceLock::new();

/// The talk being presented: from here on its colonies are shared with any
/// other deque presenting it, and kept on disk between runs; with `fresh`,
/// they begin anew, not as they were kept.
pub fn open(talk: &Path, fresh: bool) {
    let _ = TALK.set((talk.canonicalize().unwrap_or(talk.to_path_buf()), fresh));
}

/// A name for a colony of the talk that's open, the same every run.
fn name(colony: &str) -> Option<String> {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    (&TALK.get()?.0, colony).hash(&mut h);
    Some(format!("{:016x}", h.finish()))
}

/// Where a colony's kept between runs.
fn kept(colony: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".deque").join("colonies").join(format!("{}.colony", name(colony)?)))
}

/// A colony as it was last kept, if it was, and isn't to begin anew.
pub fn saved(colony: &str) -> Option<Vec<u8>> {
    if TALK.get()?.1 {
        return None;
    }
    std::fs::read(kept(colony)?).ok()
}

pub struct Twin {
    dir: PathBuf,
    /// Who this is, among those sharing: its process.
    id: String,
    mine: bool,
    /// When it last looked at whose the colony is, and last wrote it out.
    looked: Option<Instant>,
    told: Instant,
    /// The colony as last read: when that was written.
    seen: Option<SystemTime>,
    /// How often it looks, and how long unheard from is gone.
    every: Duration,
    gone: Duration,
    /// Where the colony's kept between runs, and when it last was.
    kept: Option<PathBuf>,
    saved: Instant,
}

impl Twin {
    /// For a colony of the talk that's open, by what its `sky:` says of
    /// it; nothing, when no talk is.
    pub fn new(colony: &str) -> Option<Twin> {
        let dir = std::env::temp_dir().join(format!("deque-ants-{}", name(colony)?));
        let mut t = Twin::at(dir, std::process::id().to_string(), Duration::from_millis(500), Duration::from_secs(2));
        t.kept = kept(colony);
        // Soon after it's first looked at, then every so often: a view
        // changed, or a slide left and come back to, starts this again.
        t.saved = Instant::now().checked_sub(Duration::from_secs(8)).unwrap_or(t.saved);
        Some(t)
    }

    fn at(dir: PathBuf, id: String, every: Duration, gone: Duration) -> Twin {
        let _ = std::fs::create_dir_all(&dir);
        Twin { dir, id, mine: true, looked: None, told: Instant::now(), seen: None, every, gone, kept: None, saved: Instant::now() }
    }

    fn fresh(&self, name: &str) -> bool {
        let at = std::fs::metadata(self.dir.join(name)).and_then(|m| m.modified()).ok();
        at.is_some_and(|t| t.elapsed().map_or(true, |d| d < self.gone))
    }

    /// Whether the colony's this one's to keep: it is unless another's
    /// keeping it, and has said so lately.
    pub fn owns(&mut self) -> bool {
        if self.looked.is_none_or(|t| t.elapsed() >= self.every) {
            self.looked = Some(Instant::now());
            let other = std::fs::read_to_string(self.dir.join("owner")).is_ok_and(|p| p != self.id) && self.fresh("owner");
            self.mine = !other;
            let _ = std::fs::write(self.dir.join(if self.mine { "owner" } else { "want" }), &self.id);
        }
        self.mine
    }

    /// The colony written out, for the other to show, when there's one
    /// that wants it.
    pub fn publish(&mut self, colony: &Colony) {
        if self.told.elapsed() < Duration::from_millis(50) || !self.fresh("want") {
            return;
        }
        self.told = Instant::now();
        // Whole, or not at all: written beside, then put in its place.
        let tmp = self.dir.join(format!("state.{}", self.id));
        if std::fs::write(&tmp, colony.snapshot()).is_ok() {
            let _ = std::fs::rename(&tmp, self.dir.join("state"));
        }
    }

    /// The colony kept for next time, every ten seconds: whole, or not at
    /// all.
    pub fn save(&mut self, colony: &Colony) {
        let Some(file) = self.kept.as_ref().filter(|_| self.saved.elapsed() >= Duration::from_secs(10)) else { return };
        self.saved = Instant::now();
        let tmp = file.with_extension(format!("{}.tmp", self.id));
        let _ = file.parent().map(std::fs::create_dir_all);
        if std::fs::write(&tmp, colony.snapshot()).is_ok() {
            let _ = std::fs::rename(&tmp, file);
        }
    }

    /// The colony as its keeper last wrote it, when that's new.
    pub fn follow(&mut self, colony: &mut Colony) {
        let state = self.dir.join("state");
        let at = std::fs::metadata(&state).and_then(|m| m.modified()).ok();
        if at.is_some()
            && at != self.seen
            && let Ok(b) = std::fs::read(&state)
            && colony.restore(&b)
        {
            self.seen = at;
        }
    }

    /// Food dropped here, for the keeper to drop in the colony: where, as
    /// parts of the screen across and down, and whether seen from above.
    pub fn feed(&self, x: f64, y: f64, top: bool) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(self.dir.join("feed")) {
            let _ = writeln!(f, "{x} {y} {}", top as u8);
        }
    }

    /// Food the other dropped, since last asked.
    pub fn fed(&self) -> Vec<(f64, f64, bool)> {
        let feed = self.dir.join("feed");
        let Ok(text) = std::fs::read_to_string(&feed) else { return vec![] };
        let _ = std::fs::remove_file(&feed);
        text.lines()
            .filter_map(|l| {
                let mut w = l.split(' ');
                Some((w.next()?.parse().ok()?, w.next()?.parse().ok()?, w.next()? == "1"))
            })
            .filter(|(x, y, _): &(f64, f64, bool)| (0.0..=1.0).contains(x) && (0.0..=1.0).contains(y))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_panes_one_colony() {
        let dir = std::env::temp_dir().join(format!("deque-twin-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let (every, gone) = (Duration::from_millis(20), Duration::from_millis(400));
        let mut a = Twin::at(dir.clone(), "a".into(), every, gone);
        let mut b = Twin::at(dir.clone(), "b".into(), every, gone);
        // The first there keeps it; the second follows.
        assert!(a.owns() && !b.owns());
        let mut kept = Colony::new(160, 60, 5, "", Some(30));
        let mut shown = Colony::new(100, 40, 9, "", Some(30));
        shown.view(true);
        for k in 0..60 * 30 {
            kept.step(1.0 / 30.0, k as f64 / 30.0, &[], &[]);
        }
        std::thread::sleep(Duration::from_millis(60));
        a.publish(&kept);
        b.follow(&mut shown);
        // The same colony, on a screen of its own size.
        assert_eq!(shown.snapshot_of(), kept.snapshot_of());
        assert_eq!(shown.size(), (100, 40));
        // Food dropped in the second falls in the first.
        b.feed(0.25, 0.5, true);
        b.feed(7.0, 0.5, true);
        assert_eq!(a.fed(), [(0.25, 0.5, true)]);
        assert!(a.fed().is_empty());
        // The keeper gone, the other takes it up, and goes on from there.
        std::thread::sleep(gone + Duration::from_millis(100));
        assert!(b.owns());
        for k in 0..30 {
            shown.step(1.0 / 30.0, 60.0 + k as f64 / 30.0, &[], &[]);
        }
        // And the first, back, follows in its turn.
        a.looked = None;
        assert!(!a.owns());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
