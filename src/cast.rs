//! deque TALK --cast FILE: the talk played through, every slide and step,
//! recorded as an asciinema cast, v2. asciinema plays it and agg makes it a
//! GIF. Nothing waits: the frames are timed as they would have been. Run
//! blocks run for real; enter commands don't.

use crate::fx;
use crate::images::{Pictures, Proto};
use crate::render::{self, Mode};
use crate::run;
use crate::screen::Screen;
use crate::talk::Talk;
use serde_json::json;
use std::path::Path;

/// How long a slide stays once it's all there, and a step once it's in.
const SLIDE: f64 = 2.5;
const STEP: f64 = 1.5;

pub fn record(talk: &Talk, file: &Path, w: i32, h: i32) -> Result<(), String> {
    let mut s = Screen::recording(talk.theme.clone(), w, h);
    let mut pics = Pictures::new(Proto::Blocks);
    s.raw("\x1b[?25l");
    for n in 0..talk.slides.len() {
        let slide = &talk.slides[n];
        let mode = render::arrive(talk, n);
        if n > 0 && mode == Mode::Arrive {
            fx::transition(&mut s, &talk.tr(slide));
        }
        render::draw(&mut s, talk, &mut pics, n, mode, 0, false);
        s.tick(SLIDE);
        for k in 1..=slide.steps() {
            render::draw(&mut s, talk, &mut pics, n, Mode::Step, k, false);
            if slide.run.as_ref().is_some_and(|r| r.step == k) {
                run::go(&mut s, talk, n);
            }
            s.tick(STEP);
        }
    }
    s.flush();
    let t = &talk.theme;
    let hex = |c: crate::markup::Rgb| format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2);
    // The terminal's own 16 colors, from the talk's, for anything a run
    // block's output might ask for.
    let eight = [t.bg, t.bad, t.good, t.accent, t.link, t.warm, t.link, t.fg].map(hex).join(":");
    let head = json!({
        "version": 2,
        "width": w,
        "height": h,
        "theme": { "fg": hex(t.fg), "bg": hex(t.bg), "palette": format!("{eight}:{eight}") },
    });
    let mut out = head.to_string();
    for (at, text) in &s.rec.take().unwrap().frames {
        out.push('\n');
        out.push_str(&json!([(at * 1000.0).round() / 1000.0, "o", text]).to_string());
    }
    out.push('\n');
    std::fs::write(file, out).map_err(|e| format!("deque: {}: {e}\n", file.display()))
}
