//! deque TALK --rehearse: the talk as usual, the time spent on each slide
//! kept, and when it's over, written into the talk as each slide's
//! `time:`, for `deque notes` to pace the real thing by.

use crate::talk::{self, Talk};

/// Seconds as a talk writes them: 45s, 2m, 1m30s.
pub fn say(t: u32) -> String {
    match (t / 60, t % 60) {
        (0, s) => format!("{s}s"),
        (m, 0) => format!("{m}m"),
        (m, s) => format!("{m}m{s}s"),
    }
}

/// Seconds rounded to the nearest 5, and at least 5.
pub fn round(secs: f64) -> u32 {
    ((secs / 5.0).round() as u32 * 5).max(5)
}

/// The talk's source with `times[n]` as slide n's `time:`, where there is
/// one: its own `time:` line changed, or one put right after its `---`.
pub fn with_times(src: &str, talk: &Talk, times: &[Option<u32>]) -> String {
    let lines: Vec<&str> = src.lines().collect();
    let mut out: Vec<String> = vec![];
    let mut next = 0;
    for (n, slide) in talk.slides.iter().enumerate() {
        let Some(t) = times.get(n).copied().flatten() else { continue };
        let at = slide.line;
        out.extend(lines[next..=at].iter().map(|l| l.to_string()));
        next = at + 1;
        let line = format!("time: {}", say(t));
        // Its options: the lines after `---` that are `key: value`, each a
        // slide option.
        let mut k = at + 1;
        let mut had = false;
        while k < lines.len() && talk::key_line(lines[k]).is_some_and(|(key, _)| crate::spec::find(crate::spec::SLIDE, key).is_some()) {
            if talk::key_line(lines[k]).is_some_and(|(key, _)| key == "time") {
                out.extend(lines[next..k].iter().map(|l| l.to_string()));
                out.push(line.clone());
                next = k + 1;
                had = true;
            }
            k += 1;
        }
        if !had {
            out.push(line);
            // Text that looks like an option, right under it, would be
            // read as one: a blank line between.
            if next < lines.len() && next == k && talk::key_line(lines[next]).is_some() {
                out.push(String::new());
            }
        }
    }
    out.extend(lines[next..].iter().map(|l| l.to_string()));
    let mut s = out.join("\n");
    if src.ends_with('\n') {
        s.push('\n');
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn times_are_said_and_rounded() {
        assert_eq!((say(45), say(120), say(90)), ("45s".into(), "2m".into(), "1m30s".into()));
        assert_eq!((round(1.0), round(62.4), round(63.0)), (5, 60, 65));
    }

    #[test]
    fn times_go_in_as_options() {
        let src = "#!/usr/bin/env deque\nfx: wipe\n\n---\n# ONE\n---\nfx: drop\ntime: 2m\n\n# TWO\n---\nnote: hi\n---\n# FOUR\n";
        let (t, d) = talk::parse(src, Path::new("."), false);
        assert!(d.iter().all(|d| d.warn), "{d:?}");
        let got = with_times(src, &t, &[Some(30), Some(95), Some(10), None]);
        assert_eq!(got, "#!/usr/bin/env deque\nfx: wipe\n\n---\ntime: 30s\n# ONE\n---\nfx: drop\ntime: 1m35s\n\n# TWO\n---\ntime: 10s\n\nnote: hi\n---\n# FOUR\n");
        // And it reads back the same.
        let (t2, d2) = talk::parse(&got, Path::new("."), false);
        assert!(d2.iter().all(|d| d.warn), "{d2:?}");
        assert_eq!(t2.slides.iter().map(|s| s.time).collect::<Vec<_>>(), [Some(30), Some(95), Some(10), None]);
        assert_eq!(markup_text(&t2.slides[2]), "note: hi");
    }

    fn markup_text(s: &talk::Slide) -> String {
        crate::markup::text(&s.body[0].line).trim().to_string()
    }
}
