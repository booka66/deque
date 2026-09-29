//! deque TALK --cast FILE: the talk played through, every slide and step,
//! recorded as an asciinema cast, v2. asciinema plays it and agg makes it a
//! GIF. Nothing waits: the frames are timed as they would have been. Run
//! blocks run for real; enter commands don't.
//!
//! deque TALK --html FILE: the same recording in a page of its own, to post
//! after the talk: → and ← step through it as the talk did, animations and
//! all.

use crate::images::{Pictures, Proto};
use crate::render::{self, Mode};
use crate::run;
use crate::screen::Screen;
use crate::share::{XTERM, XTERM_CSS, XTERM_JS};
use crate::talk::Talk;
use serde_json::json;
use std::path::Path;

/// How long a slide stays once it's all there, and a step once it's in.
const SLIDE: f64 = 2.5;
const STEP: f64 = 1.5;

/// The talk played: what went to the screen, with when, and where each
/// slide and step began, as the frame it began at.
struct Played {
    frames: Vec<(f64, String)>,
    marks: Vec<usize>,
}

fn play(talk: &Talk, w: i32, h: i32) -> Played {
    let mut s = Screen::recording(talk.theme.clone(), w, h);
    let mut pics = Pictures::new(Proto::Blocks);
    let mut marks = vec![];
    let mut mark = |s: &mut Screen| {
        s.flush();
        marks.push(s.rec.as_ref().unwrap().frames.len());
    };
    s.raw("\x1b[?25l");
    for n in 0..talk.slides.len() {
        let slide = &talk.slides[n];
        mark(&mut s);
        let mut mode = render::arrive(talk, n);
        if n > 0 && mode == Mode::Arrive && render::leave(&mut s, talk, n - 1, n) {
            mode = Mode::Still;
        }
        render::draw(&mut s, talk, &mut pics, n, mode, 0, false);
        s.tick(SLIDE);
        for k in 1..=slide.steps() {
            mark(&mut s);
            render::draw(&mut s, talk, &mut pics, n, Mode::Step, k, false);
            if slide.run.as_ref().is_some_and(|r| r.step == k) {
                run::go(&mut s, talk, n);
            }
            s.tick(STEP);
        }
    }
    s.flush();
    Played { frames: s.rec.take().unwrap().frames, marks }
}

fn hex(c: crate::markup::Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// The terminal's own 16 colors, from the talk's, for anything a run
/// block's output might ask for.
fn palette(talk: &Talk) -> [String; 8] {
    let t = &talk.theme;
    [t.bg, t.bad, t.good, t.accent, t.link, t.warm, t.link, t.fg].map(hex)
}

pub fn record(talk: &Talk, file: &Path, w: i32, h: i32) -> Result<(), String> {
    let p = play(talk, w, h);
    let t = &talk.theme;
    let eight = palette(talk).join(":");
    let head = json!({
        "version": 2,
        "width": w,
        "height": h,
        "theme": { "fg": hex(t.fg), "bg": hex(t.bg), "palette": format!("{eight}:{eight}") },
    });
    let mut out = head.to_string();
    for (at, text) in &p.frames {
        out.push('\n');
        out.push_str(&json!([(at * 1000.0).round() / 1000.0, "o", text]).to_string());
    }
    out.push('\n');
    std::fs::write(file, out).map_err(|e| format!("deque: {}: {e}\n", file.display()))
}

/// The page: xterm.js, checked against its hash, playing the frames, in
/// the terminal's font where the viewer has it.
pub fn html(talk: &Talk, file: &Path, w: i32, h: i32) -> Result<(), String> {
    let p = play(talk, w, h);
    let t = &talk.theme;
    let eight = palette(talk);
    let data = json!({
        "w": w,
        "h": h,
        "frames": p.frames.iter().map(|(at, text)| json!([(at * 1000.0).round() / 1000.0, text])).collect::<Vec<_>>(),
        "marks": p.marks,
        "theme": {
            "background": hex(t.bg), "foreground": hex(t.fg), "cursor": hex(t.bg),
            "black": eight[0], "red": eight[1], "green": eight[2], "yellow": eight[3],
            "blue": eight[4], "magenta": eight[5], "cyan": eight[6], "white": eight[7],
            "brightBlack": hex(t.muted), "brightRed": eight[1], "brightGreen": eight[2], "brightYellow": eight[3],
            "brightBlue": eight[4], "brightMagenta": eight[5], "brightCyan": eight[6], "brightWhite": eight[7],
        },
        // The family, not the file: a font's license may not let it be
        // handed round, so a viewer who has it sees it.
        "font": crate::fine::configured(),
    });
    // Nothing in the data can close the script it's in.
    let data = data.to_string().replace("</", "<\\/");
    let title = talk.slides.first().map(|s| s.title()).unwrap_or_default();
    let title = title.replace('&', "&amp;").replace('<', "&lt;");
    let (bg, muted) = (hex(t.bg), hex(t.muted));
    let page = format!(
        r##"<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="generator" content="deque">
<title>{title}</title>
<link rel="stylesheet" href="{XTERM}/css/xterm.css" integrity="{XTERM_CSS}" crossorigin="anonymous">
<style>
html,body{{margin:0;height:100%;background:{bg};overflow:hidden}}
body{{display:flex;align-items:center;justify-content:center;cursor:pointer}}
#t{{transform-origin:center}}
#hint{{position:fixed;bottom:12px;left:0;right:0;text-align:center;font:13px ui-monospace,monospace;color:{muted};transition:opacity 1s}}
</style>
<div id="t"></div>
<div id="hint">→ or click: next · ← back · home: start · f: full screen</div>
<script src="{XTERM}/lib/xterm.js" integrity="{XTERM_JS}" crossorigin="anonymous"></script>
<script>const TALK = {data};</script>
<script>{PLAYER}</script>
"##
    );
    std::fs::write(file, page).map_err(|e| format!("deque: {}: {e}\n", file.display()))
}

/// Where the viewer is: at a mark, the step before it played out. → plays
/// the frames up to the next mark as they were timed; ← and the rest go
/// at once, written from the start.
const PLAYER: &str = r##"(() => {
const {w, h, frames, marks, theme, font} = TALK;
const family = (font ? JSON.stringify(font) + ", " : "") + "ui-monospace, Menlo, monospace";
const term = new Terminal({cols: w, rows: h, fontSize: 16, fontFamily: family, scrollback: 0, disableStdin: true, cursorBlink: false, theme});
term.open(document.getElementById("t"));
function fit() {
  const e = document.querySelector("#t .xterm-screen");
  if (!e) return;
  const k = Math.min(innerWidth / e.offsetWidth, innerHeight / e.offsetHeight) * 0.98;
  document.getElementById("t").style.transform = "scale(" + k + ")";
}
addEventListener("resize", fit);
setTimeout(fit, 100);
// The mark the screen's at; what's played so far; the playing, if any.
let at = -1, done = 0, timer = null;
const end = i => i + 1 < marks.length ? marks[i + 1] : frames.length;
function upto(f) {
  let s = "";
  for (; done < f; done++) s += frames[done][1];
  if (s) term.write(s);
}
function stop() {
  if (timer) { clearTimeout(timer); timer = null; }
}
// Mark i, played in: its frames at their times.
function play(i) {
  stop();
  if (done > marks[i]) { term.reset(); done = 0; }
  upto(marks[i]);
  at = i;
  const t0 = performance.now(), base = frames[marks[i]] ? frames[marks[i]][0] : 0, last = end(i);
  const step = () => {
    const now = base + (performance.now() - t0) / 1000;
    let s = "";
    while (done < last && frames[done][0] <= now) s += frames[done++][1];
    if (s) term.write(s);
    timer = done < last ? setTimeout(step, 15) : null;
  };
  step();
}
// Mark i, whole, at once.
function show(i) {
  stop();
  if (done > end(i)) { term.reset(); done = 0; }
  upto(end(i));
  at = i;
}
function next() {
  if (timer) { show(at); return; }
  if (at + 1 < marks.length) play(at + 1);
}
function back() {
  stop();
  if (at > 0) show(at - 1);
}
addEventListener("keydown", e => {
  if (e.metaKey || e.ctrlKey || e.altKey) return;
  const k = e.key;
  if (k === "ArrowRight" || k === " " || k === "Enter" || k === "PageDown" || k === "n" || k === "l" || k === "j") next();
  else if (k === "ArrowLeft" || k === "PageUp" || k === "Backspace" || k === "p" || k === "b" || k === "h" || k === "k") back();
  else if (k === "Home" || k === "g") { stop(); term.reset(); done = 0; play(0); }
  else if (k === "End" || k === "G") show(marks.length - 1);
  else if (k === "f") document.fullscreenElement ? document.exitFullscreen() : document.documentElement.requestFullscreen();
  else return;
  e.preventDefault();
});
addEventListener("click", next);
setTimeout(() => document.getElementById("hint").style.opacity = 0, 4000);
play(0);
})();"##;
