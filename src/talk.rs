//! A talk file, read into slides. Problems come back as diagnostics with the
//! line they are on, for deque to refuse to start with and for the language
//! server to show in the editor.

use crate::code;
use crate::figlet;
use crate::markup::{self, Line, Rgb, Style, Theme};
use crate::spec;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Diag {
    /// 0-based line, and the columns (in chars) it covers.
    pub line: usize,
    pub start: usize,
    pub end: usize,
    pub msg: String,
    pub warn: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Effects {
    pub fx: Option<String>,
    pub lines: Option<String>,
    pub reveal: Option<String>,
    pub then: Option<Vec<String>>,
    pub tr: Option<String>,
    pub sky: Option<String>,
    pub glow: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Body {
    pub line: Line,
    /// The line of the talk it's from.
    pub src: usize,
    /// The step it comes in on, 0 for there with the slide.
    pub step: usize,
    /// Whether it's a line of a ```lang block.
    pub code: bool,
    /// A ```chart line's bar, to grow in.
    pub bar: Option<Bar>,
}

/// A bar of a ```chart block: the label before it, how full it is, how
/// many cells it has at most, and the value after it.
#[derive(Clone, Debug)]
pub struct Bar {
    pub lead: Line,
    pub frac: f64,
    pub width: usize,
    pub tail: Line,
}

impl Bar {
    /// The line with the bar k of the way grown, to an eighth of a cell.
    pub fn line(&self, k: f64, st: Style) -> Line {
        const PART: [char; 7] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉'];
        let eighths = (self.frac * k.clamp(0.0, 1.0) * self.width as f64 * 8.0).round() as usize;
        let mut bar: String = "█".repeat(eighths / 8);
        if !eighths.is_multiple_of(8) {
            bar.push(PART[eighths % 8 - 1]);
        }
        let pad = self.width - bar.chars().count();
        let mut l = self.lead.clone();
        l.extend(markup::plain(&bar, st));
        l.extend(markup::plain(&" ".repeat(pad), Style::default()));
        l.extend(self.tail.iter().copied());
        l
    }
}

/// A ```poll block: its choices, where they start in the body, and a name
/// for it that stays the same while its choices do, to keep its votes by.
#[derive(Clone, Debug)]
pub struct Poll {
    pub id: String,
    pub choices: Vec<String>,
    pub at: usize,
    /// Its `* ` choice, the answer, and the step that says so.
    pub answer: Option<(usize, usize)>,
}

/// A poll's bar, as wide as a chart's, and its count, for `votes` votes of
/// `most` at most.
pub fn poll_bar(lead: Line, votes: usize, most: usize, muted: Rgb) -> Bar {
    let tail = markup::plain(&format!(" {votes:>4}"), Style::fg(muted));
    Bar { lead, frac: votes as f64 / most.max(1) as f64, width: 32, tail }
}

/// A ```lang focus: step: on it, the block's lines but these dim.
#[derive(Clone, Debug)]
pub struct Focus {
    pub step: usize,
    /// The block's lines in the body, and those lit.
    pub block: std::ops::Range<usize>,
    pub lines: Vec<usize>,
}

/// A ```lang run block: what runs it, and when.
#[derive(Clone, Debug)]
pub struct Run {
    /// The program, its flag for code to run, and the block's code.
    pub argv: Vec<String>,
    /// The step its output comes in on.
    pub step: usize,
    /// Its first line's place in the body.
    pub at: usize,
}

#[derive(Clone, Debug)]
pub struct Image {
    pub path: PathBuf,
    pub alt: String,
}

#[derive(Clone, Debug)]
pub enum Item {
    Text(i32, i32, Line),
    Center(i32, Line),
    Box(i32, i32, i32, i32, Line),
    Path(Vec<(i32, i32)>, bool),
    Clear(i32, i32),
}

#[derive(Clone, Debug)]
pub struct Draw {
    pub w: i32,
    pub h: i32,
    /// What shows with the slide, then one group a step, each with the
    /// line of the talk it's from.
    pub groups: Vec<Vec<(usize, Item)>>,
}

#[derive(Clone, Debug, Default)]
pub struct Slide {
    /// The line of its `---`.
    pub line: usize,
    pub label: Option<String>,
    pub headline: Option<String>,
    /// The lines of the talk they're on.
    pub label_src: Option<usize>,
    pub headline_src: Option<usize>,
    pub art: Vec<Vec<char>>,
    /// The lines, or for a picture slide its caption.
    pub body: Vec<Body>,
    pub images: Vec<Image>,
    pub side: bool,
    pub draw: Option<Draw>,
    pub enter: Option<String>,
    /// A recording to play on enter, instead of or before `enter`.
    pub play: Option<PathBuf>,
    pub cols: Option<i32>,
    /// keys: on or off, where the slide says: the keys pressed while
    /// `enter` runs, shown under it.
    pub keys: Option<bool>,
    pub fx: Effects,
    /// The language of its first ```lang block.
    pub lang: Option<String>,
    pub run: Option<Run>,
    pub focus: Vec<Focus>,
    /// How long it's meant to be up, in seconds, for pacing.
    pub time: Option<u32>,
    /// Its speaker notes, the `//` lines.
    pub notes: Vec<String>,
    /// Its ```poll block, voted on by those watching.
    pub poll: Option<Poll>,
}

impl Slide {
    /// What to call it in a list: its headline, label, or first words.
    pub fn title(&self) -> String {
        if let Some(h) = &self.headline {
            return h.clone();
        }
        if let Some(l) = &self.label {
            return l.clone();
        }
        let first = self.body.iter().map(|b| markup::text(&b.line).trim().to_string()).find(|t| !t.is_empty());
        match (first, &self.draw, self.images.is_empty()) {
            (_, Some(_), _) => "a drawing".into(),
            (_, _, false) => "pictures".into(),
            (Some(t), ..) => t,
            _ => "(empty)".into(),
        }
    }

    /// Its poll's bars, for these votes for each choice; how full they
    /// were before.
    pub fn tally(&mut self, votes: &[usize], accent: Rgb, muted: Rgb) -> Vec<f64> {
        let Some(p) = &self.poll else { return vec![] };
        let most = votes.iter().copied().max().unwrap_or(0);
        let mut before = vec![];
        for k in 0..p.choices.len() {
            let b = &mut self.body[p.at + k];
            before.push(b.bar.as_ref().map_or(0.0, |x| x.frac));
            let lead = b.bar.as_ref().map(|x| x.lead.clone()).unwrap_or_default();
            let bar = poll_bar(lead, votes.get(k).copied().unwrap_or(0), most, muted);
            b.line = bar.line(1.0, Style::fg(accent));
            b.bar = Some(bar);
        }
        before
    }

    pub fn steps(&self) -> usize {
        match &self.draw {
            Some(d) => d.groups.len() - 1,
            None => {
                let (run, focus) = (self.run.as_ref().map(|r| r.step), self.focus.iter().map(|f| f.step));
                let answer = self.poll.as_ref().and_then(|p| p.answer).map(|a| a.1);
                self.body.iter().map(|b| b.step).chain(run).chain(focus).chain(answer).max().unwrap_or(0)
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Talk {
    pub theme: Theme,
    pub fx: Effects,
    pub cursor: bool,
    /// calm: on, or --calm: nothing moves that needn't. No skies, glow,
    /// flourishes, transitions or morphs; what arrives fades in.
    pub calm: bool,
    /// keys: for every slide that doesn't say.
    pub keys: Option<bool>,
    pub slides: Vec<Slide>,
    pub dir: PathBuf,
    /// The environment variables it reads, for --tv to take along.
    pub vars: Vec<String>,
    /// The files its code comes from, to read again when they change.
    pub files: Vec<PathBuf>,
    /// Whether it says what a command printed (`${sh: …}`): read again each
    /// minute, for what's changed.
    pub live: bool,
}

impl Talk {
    pub fn fx(&self, s: &Slide) -> String {
        if self.calm {
            return "fade".into();
        }
        s.fx.fx.clone().or(self.fx.fx.clone()).unwrap_or("wipe".into())
    }
    pub fn lines(&self, s: &Slide) -> String {
        if self.calm {
            return "fade".into();
        }
        s.fx.lines.clone().or(self.fx.lines.clone()).unwrap_or("type".into())
    }
    pub fn reveal(&self, s: &Slide) -> String {
        if self.calm {
            return "fade".into();
        }
        s.fx.reveal.clone().or(self.fx.reveal.clone()).unwrap_or("glide".into())
    }
    pub fn then(&self, s: &Slide) -> Vec<String> {
        if self.calm {
            return vec![];
        }
        s.fx.then.clone().or(self.fx.then.clone()).unwrap_or_default()
    }
    pub fn tr(&self, s: &Slide) -> String {
        if self.calm {
            return "none".into();
        }
        s.fx.tr.clone().or(self.fx.tr.clone()).unwrap_or("none".into())
    }
    pub fn sky(&self, s: &Slide) -> String {
        if self.calm {
            return "none".into();
        }
        s.fx.sky.clone().or(self.fx.sky.clone()).unwrap_or("none".into())
    }
    pub fn glow(&self, s: &Slide) -> bool {
        !self.calm && s.fx.glow.clone().or(self.fx.glow.clone()).is_some_and(|g| g == "on")
    }
    /// Whether slide `to` arrives from `from` by turning into it: all of
    /// it with `tr: morph`, or its code, when both have code in the same
    /// language and `to` sets no `tr:` of its own.
    pub fn morphs(&self, from: usize, to: usize) -> bool {
        let text = |s: &Slide| s.images.is_empty() && s.draw.is_none();
        let (a, b) = (&self.slides[from], &self.slides[to]);
        if self.calm || !text(a) || !text(b) {
            return false;
        }
        self.tr(b) == "morph" || (a.lang.is_some() && a.lang == b.lang && b.fx.tr.is_none())
    }
}

/// `key: value`, a key being lowercase letters.
pub fn key_line(l: &str) -> Option<(&str, &str)> {
    let (k, v) = l.split_once(':')?;
    (!k.is_empty() && k.chars().all(|c| c.is_ascii_lowercase()) && (v.is_empty() || v.starts_with(' ')))
        .then(|| (k, v.trim()))
}

/// What runs a ```lang run block: the program and its flag for code.
pub fn runner(lang: &str) -> Option<[&'static str; 2]> {
    Some(match lang {
        "sh" => ["sh", "-c"],
        "bash" => ["bash", "-c"],
        "zsh" => ["zsh", "-c"],
        "fish" => ["fish", "-c"],
        "py" | "python" => ["python3", "-c"],
        "js" | "javascript" => ["node", "-e"],
        "rb" | "ruby" => ["ruby", "-e"],
        _ => return None,
    })
}

/// The markdown image syntax, when a line is nothing but pictures.
fn image_line(l: &str) -> Option<Vec<(String, String)>> {
    let mut out = vec![];
    let mut rest = l.trim();
    if !rest.starts_with("![") {
        return None;
    }
    while !rest.is_empty() {
        let r = rest.strip_prefix("![")?;
        let (alt, r) = r.split_once("](")?;
        let (path, r) = r.split_once(')')?;
        out.push((alt.to_string(), path.to_string()));
        rest = r.trim_start();
    }
    Some(out)
}

struct P<'a> {
    files: Vec<PathBuf>,
    diags: Vec<Diag>,
    theme: Theme,
    vars: Vec<String>,
    lenient: bool,
    dir: &'a Path,
    n: usize,
    live: bool,
}

/// Where the `{` a string starts with closes, its own braces counted, so
/// a command can have them: awk '{print $1}'.
fn closing(s: &str) -> Option<usize> {
    let mut depth = 0;
    s.char_indices().find_map(|(k, c)| {
        match c {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
        (depth == 0).then_some(k)
    })
}

/// What a command prints, run by sh (cmd on Windows) in the talk's folder:
/// its lines on one, the space round them gone.
fn sh(cmd: &str, dir: &Path) -> Result<String, String> {
    let mut c = std::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" });
    c.arg(if cfg!(windows) { "/C" } else { "-c" }).arg(cmd).current_dir(dir).stdin(std::process::Stdio::null());
    let out = c.output().map_err(|e| format!("`{cmd}`: {e}"))?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr);
        let why = why.lines().find(|l| !l.trim().is_empty()).map_or(format!("{}", out.status), |l| l.trim().to_string());
        return Err(format!("`{cmd}` failed: {why}"));
    }
    Ok(String::from_utf8_lossy(&out.stdout).split_whitespace().collect::<Vec<_>>().join(" "))
}

impl P<'_> {
    fn err(&mut self, start: usize, end: usize, msg: impl Into<String>) {
        self.diags.push(Diag { line: self.n, start, end, msg: msg.into(), warn: false });
    }
    fn warn(&mut self, start: usize, end: usize, msg: impl Into<String>) {
        self.diags.push(Diag { line: self.n, start, end, msg: msg.into(), warn: true });
    }

    /// `${NAME}` from the environment. Missing is an error to present with,
    /// a warning in the editor, which may not have what the talk's wrapper
    /// sets. `${sh: command}` is what the command prints; the editor
    /// doesn't run it, and shows 0. `$${` is a `${` left as it's written.
    fn interp(&mut self, s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(i) = rest.find("${") {
            if rest[..i].ends_with('$') {
                out.push_str(&rest[..i - 1]);
                out.push_str("${");
                rest = &rest[i + 2..];
                continue;
            }
            let cmd = rest[i + 2..].starts_with("sh:");
            let Some(j) = (if cmd { closing(&rest[i + 1..]).map(|k| k + 1) } else { rest[i..].find('}') }) else { break };
            let name = &rest[i + 2..i + j];
            out.push_str(&rest[..i]);
            let col = s.len() - rest.len() + i;
            let (col, end) = (s[..col].chars().count(), s[..col + j + 1].chars().count());
            rest = &rest[i + j + 1..];
            if cmd {
                self.live = true;
                match self.lenient {
                    true => out.push('0'),
                    false => match sh(name[3..].trim(), self.dir) {
                        Ok(v) => out.push_str(&v),
                        Err(e) => self.err(col, end, e),
                    },
                }
                continue;
            }
            if !self.vars.iter().any(|v| v == name) {
                self.vars.push(name.to_string());
            }
            match std::env::var(name) {
                Ok(v) => out.push_str(&v),
                Err(_) if self.lenient => self.warn(col, end, format!("${{{name}}} isn't set here; it must be when the talk runs")),
                Err(_) => self.err(col, end, format!("${{{name}}} isn't set")),
            }
        }
        out.push_str(rest);
        out
    }

    fn markup(&mut self, s: &str, col: usize) -> Line {
        let s = self.interp(s);
        match markup::parse(&s, &self.theme, Style::default()) {
            Ok(l) => l,
            Err(e) => {
                let end = col + s.chars().count();
                self.err(col, end, e);
                vec![]
            }
        }
    }

    /// A value one of a set, or a diagnostic naming the set.
    fn one_of(&mut self, what: &str, v: &str, col: usize, set: &[spec::Named]) -> Option<String> {
        if set.iter().any(|(n, _)| *n == v) {
            return Some(v.to_string());
        }
        let msg = match v {
            "" => format!("{what}: which? there's {}", spec::names(set)),
            _ => format!("no {what} \"{v}\"; {}there's {}", spec::near(v, set.iter().map(|(n, _)| *n)), spec::names(set)),
        };
        self.err(col, col + v.chars().count(), msg);
        None
    }

    /// A sky, and after it how many: `rain 200`, or `boids 12 3`, twelve
    /// birds and three hawks.
    fn sky(&mut self, v: &str, col: usize) -> Option<String> {
        let mut words = v.split_whitespace();
        let kind = self.one_of("sky", words.next().unwrap_or(""), col, spec::SKY)?;
        // An ant colony says more: which view, which ants, what they're in.
        if kind == "ants" {
            let (mut sets, mut n) = ([false; 3], 0);
            for w in words {
                match spec::ANTS.iter().position(|set| set.iter().any(|(name, _)| *name == w)) {
                    Some(k) if !sets[k] => sets[k] = true,
                    None if w.parse::<usize>().is_ok() && n == 0 => n += 1,
                    _ => {
                        let all = spec::ANTS.iter().map(|set| spec::names(set)).collect::<Vec<_>>().join("; ");
                        let near = spec::near(w, spec::ANTS.iter().flat_map(|set| set.iter().map(|(name, _)| *name)));
                        self.err(col, col + v.chars().count(), format!("sky: ants takes one of each, and how many: {near}there's {all}"));
                        return Some(kind);
                    }
                }
            }
            return Some(v.split_whitespace().collect::<Vec<_>>().join(" "));
        }
        let nums: Vec<&str> = words.collect();
        let most = match kind.as_str() {
            "boids" => 2,
            "stars" | "snow" | "rain" | "embers" | "fireflies" | "koi" => 1,
            _ => 0,
        };
        if nums.len() > most || nums.iter().any(|n| n.parse::<usize>().is_err()) {
            let msg = match most {
                0 => format!("sky: {kind} takes nothing after it"),
                1 => format!("after sky: {kind} goes how many, a number: sky: {kind} 40"),
                _ => "after sky: boids goes how many birds, then how many hawks: sky: boids 12 3".into(),
            };
            self.err(col, col + v.chars().count(), msg);
            return Some(kind);
        }
        Some(std::iter::once(kind.as_str()).chain(nums).collect::<Vec<_>>().join(" "))
    }

    /// An effect option, into e. Returns whether the key was one.
    fn effect(&mut self, e: &mut Effects, k: &str, v: &str, col: usize) -> bool {
        match k {
            "fx" => e.fx = self.one_of("fx", v, col, spec::FX),
            "lines" => e.lines = self.one_of("lines effect", v, col, spec::LINES),
            "reveal" => e.reveal = self.one_of("reveal effect", v, col, spec::LINES),
            "tr" => e.tr = self.one_of("transition", v, col, spec::TR),
            "sky" => e.sky = self.sky(v, col),
            "glow" => e.glow = self.one_of("glow", v, col, spec::ON_OFF),
            "then" => {
                let mut all = vec![];
                let mut at = col;
                for w in v.split(' ') {
                    if !w.is_empty() {
                        all.extend(self.one_of("flourish", w, at, spec::THEN));
                    }
                    at += w.chars().count() + 1;
                }
                e.then = Some(all);
            }
            _ => return false,
        }
        true
    }
}

pub fn parse(src: &str, dir: &Path, lenient: bool) -> (Talk, Vec<Diag>) {
    let mut p = P { files: vec![], diags: vec![], theme: Theme::default(), vars: vec![], lenient, dir, n: 0, live: false };
    let mut talk_fx = Effects::default();
    let mut cursor = true;
    let mut calm = false;
    let mut keys = None;
    let lines: Vec<&str> = src.lines().collect();
    let mut i = 0;
    if lines.first().is_some_and(|l| l.starts_with("#!")) {
        i = 1;
    }
    // The talk's settings, up to the first slide.
    while i < lines.len() && lines[i].trim_end() != "---" {
        p.n = i;
        let l = lines[i];
        i += 1;
        if l.trim().is_empty() || l.starts_with("//") {
            continue;
        }
        let Some((k, v)) = key_line(l) else {
            p.err(0, l.chars().count(), "before the first `---` go settings (key: value) and comments; slides start at `---`");
            continue;
        };
        let col = l.find(v).map(|b| l[..b].chars().count()).unwrap_or(0);
        let v = p.interp(v);
        if p.effect(&mut talk_fx, k, &v, col) {
            continue;
        }
        match k {
            "cursor" => match v.as_str() {
                "on" | "yes" | "true" => cursor = true,
                "off" | "no" | "false" => cursor = false,
                _ => p.err(col, l.chars().count(), "cursor is on or off"),
            },
            "calm" => match v.as_str() {
                "on" | "yes" | "true" => calm = true,
                "off" | "no" | "false" => calm = false,
                _ => p.err(col, l.chars().count(), "calm is on or off"),
            },
            "keys" => match v.as_str() {
                "on" | "yes" | "true" => keys = Some(true),
                "off" | "no" | "false" => keys = Some(false),
                _ => p.err(col, l.chars().count(), "keys is on or off"),
            },
            _ if spec::find(spec::TALK, k).is_some() => match Rgb::parse(&v) {
                Some(c) => {
                    p.theme.set(k, c);
                }
                None if v.is_empty() && lenient => {}
                None => p.err(col, l.chars().count(), format!("\"{v}\" isn't a color: #rrggbb, or 38;2;r;g;b")),
            },
            _ => {
                let msg = format!("no setting \"{k}\"; {}there's {}", spec::near(k, spec::TALK.iter().map(|o| o.key)), spec::TALK.iter().map(|o| o.key).collect::<Vec<_>>().join(", "));
                p.err(0, k.len(), msg);
            }
        }
    }
    let mut slides = vec![];
    while i < lines.len() {
        // At a `---`: the slide runs to the next one outside a ``` block.
        let start = i;
        i += 1;
        let mut fence = false;
        while i < lines.len() && (fence || lines[i].trim_end() != "---") {
            if lines[i].trim_start().starts_with("```") {
                fence = !fence;
            }
            i += 1;
        }
        slides.push(slide(&mut p, &lines, start, i));
    }
    if slides.is_empty() {
        p.n = lines.len().saturating_sub(1);
        p.err(0, 0, "no slides: a slide starts at a line of `---`");
    }
    let talk = Talk { theme: p.theme.clone(), fx: talk_fx, cursor, calm, keys, slides, dir: dir.to_path_buf(), vars: p.vars.clone(), files: p.files.clone(), live: p.live };
    (talk, p.diags)
}

fn slide(p: &mut P, lines: &[&str], start: usize, end: usize) -> Slide {
    let mut s = Slide { line: start, ..Slide::default() };
    let mut i = start + 1;
    // Options: the lines right after the `---` that are `key: value`.
    let mut draw_size = None;
    while i < end {
        p.n = i;
        let l = lines[i];
        let Some((k, v)) = key_line(l) else { break };
        if spec::find(spec::SLIDE, k).is_none() {
            let keys = spec::SLIDE.iter().map(|o| o.key).collect::<Vec<_>>().join(", ");
            let near = spec::near(k, spec::SLIDE.iter().map(|o| o.key));
            if i == start + 1 {
                // Maybe text that has a colon in it; say so, but show it.
                p.warn(0, k.len(), format!("shown as text: \"{k}\" isn't a slide option; {near}there's {keys}. Options go right after `---`"));
                break;
            }
            p.err(0, k.len(), format!("no slide option \"{k}\"; {near}there's {keys}. Put a blank line between options and text"));
            i += 1;
            continue;
        }
        let col = l.find(v).map(|b| l[..b].chars().count()).unwrap_or(0);
        let v = p.interp(v);
        i += 1;
        if p.effect(&mut s.fx, k, &v, col) {
            continue;
        }
        match k {
            "enter" => s.enter = Some(v),
            "play" => {
                let full = p.dir.join(&v);
                p.files.push(full.clone());
                match crate::replay::load(&full) {
                    Ok(_) => s.play = Some(full),
                    Err(e) => p.err(col, l.chars().count(), e),
                }
            }
            "keys" => match v.as_str() {
                "on" | "yes" | "true" => s.keys = Some(true),
                "off" | "no" | "false" => s.keys = Some(false),
                _ => p.err(col, l.chars().count(), "keys is on or off"),
            },
            "cols" => match v.parse() {
                Ok(n) if n >= 20 => s.cols = Some(n),
                _ => p.err(col, l.chars().count(), "cols is a number of columns, 20 or more"),
            },
            "time" => match seconds(&v) {
                Some(t) => s.time = Some(t),
                None => p.err(col, l.chars().count(), "time is how long the slide's meant to be up: 90s, 2m, 1m30s"),
            },
            "draw" => match v.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))) {
                Some((w, h)) => draw_size = Some((w, h)),
                None => p.err(col, l.chars().count(), "draw is the drawing's size, WIDTHxHEIGHT: draw: 76x20"),
            },
            _ => unreachable!(),
        }
    }
    if s.play.is_some() && s.draw.is_some() {
        p.n = start;
        p.err(0, 3, "a drawn slide can't play a recording");
    }
    if s.cols.is_some() && s.enter.is_none() {
        p.n = start;
        p.warn(0, 3, "cols does nothing without enter");
    }
    if let Some((w, h)) = draw_size {
        drawing(p, &mut s, lines, i, end, w, h);
    } else if lines[i..end].iter().any(|l| l.trim_start().starts_with("```graph")) {
        graph(p, &mut s, lines, i, end);
    } else {
        textual(p, &mut s, lines, i, end);
    }
    s
}

/// The label and headline, which any slide can have: whether the line was one.
fn heading(p: &mut P, s: &mut Slide, l: &str) -> bool {
    if let Some(t) = l.strip_prefix("## ") {
        if s.label.is_some() {
            p.err(0, l.chars().count(), "a second label; a slide has one");
        }
        s.label = Some(p.interp(t.trim()));
        s.label_src = Some(p.n);
        true
    } else if let Some(t) = l.strip_prefix("# ") {
        if s.headline.is_some() {
            p.err(0, l.chars().count(), "a second headline; a slide has one");
        }
        let t = p.interp(t.trim());
        if let Some((at, c)) = t.chars().enumerate().find(|(_, c)| !figlet::has(*c)) {
            p.err(2 + at, 3 + at, format!("the headline font has no \"{c}\": letters, digits and ASCII punctuation only"));
        }
        s.art = figlet::render(&t.to_uppercase());
        let w = s.art.first().map_or(0, Vec::len);
        if w > 80 {
            p.warn(0, l.chars().count(), format!("{w} columns wide: a screen 80 columns wide, as --tv makes it, cuts it off"));
        }
        s.headline = Some(t);
        s.headline_src = Some(p.n);
        true
    } else {
        false
    }
}

/// Lines that go together: a ``` block, lined up in a column, or any other
/// line on its own. Each line is (step?, line, the line of the talk).
#[derive(Default)]
struct Block {
    lines: Vec<(bool, Line, usize)>,
    code: bool,
    /// For a ```lang run block, what runs it.
    run: Option<Vec<String>>,
    /// For a ```chart block, each line's bar.
    bars: Vec<Option<Bar>>,
    /// For a ```lang focus: block, the lines lit on each step, from 0.
    focus: Vec<Vec<usize>>,
    /// For a ```poll block, its choices.
    poll: Option<Vec<String>>,
    /// Which of them is the answer, if one's marked.
    answer: Option<usize>,
}

/// A ``` block being read: what its opening line said, where its lines
/// start, and its lines so far.
struct Open {
    lang: String,
    run: Option<[&'static str; 2]>,
    /// Lines from a file instead of its own.
    file: Option<Vec<String>>,
    /// The focus: groups as written, from 1.
    focus: Vec<Vec<usize>>,
    at: usize,
    src: Vec<String>,
}

/// "90", "90s", "2m", "1m30s": seconds.
fn seconds(v: &str) -> Option<u32> {
    let (mut total, mut num) = (0u32, String::new());
    for c in v.trim().chars() {
        match c {
            '0'..='9' => num.push(c),
            'm' | 's' => {
                let n: u32 = num.parse().ok()?;
                total += if c == 'm' { n * 60 } else { n };
                num.clear();
            }
            _ => return None,
        }
    }
    if !num.is_empty() {
        total += num.parse::<u32>().ok()?;
    }
    (total > 0).then_some(total)
}

/// `2|4-5,8`: the groups, each lines and ranges of them, from 1.
fn groups(spec: &str) -> Option<Vec<Vec<usize>>> {
    spec.split('|')
        .map(|g| {
            let mut out = vec![];
            for part in g.split(',') {
                match part.split_once('-') {
                    Some((a, b)) => out.extend(a.trim().parse::<usize>().ok()?..=b.trim().parse::<usize>().ok()?),
                    None => out.push(part.trim().parse().ok()?),
                }
            }
            Some(out)
        })
        .collect()
}

/// A file's lines for a block: `path:10-24` those, `path#name` the lines
/// from the first holding `name` through the end of what's indented under
/// it (and its closing bracket), dedented.
fn excerpt(text: &str, sel: Option<&str>) -> Result<Vec<String>, String> {
    let all: Vec<&str> = text.lines().collect();
    let indent = |l: &str| l.len() - l.trim_start().len();
    let got: Vec<&str> = match sel {
        None => all.clone(),
        Some(r) if r.starts_with(':') => {
            let (a, b) = r[1..].split_once('-').unwrap_or((&r[1..], &r[1..]));
            let (a, b): (usize, usize) = (a.parse().map_err(|_| "lines are :FROM-TO, like :10-24")?, b.parse().map_err(|_| "lines are :FROM-TO, like :10-24")?);
            if a < 1 || b < a || b > all.len() {
                return Err(format!("lines {a}-{b}, but the file has {}", all.len()));
            }
            all[a - 1..b].to_vec()
        }
        Some(r) => {
            let name = &r[1..];
            // The name as a whole word: #fee finds `fn fee(`, not `fees`.
            let word = |c: char| c.is_alphanumeric() || c == '_';
            let has = |l: &str| {
                l.match_indices(name).any(|(i, _)| {
                    !l[..i].chars().next_back().is_some_and(word) && !l[i + name.len()..].chars().next().is_some_and(word)
                })
            };
            let start = all.iter().position(|l| has(l)).ok_or(format!("no line with \"{name}\" in it"))?;
            let i0 = indent(all[start]);
            let mut end = start + 1;
            while end < all.len() && (all[end].trim().is_empty() || indent(all[end]) > i0) {
                end += 1;
            }
            // Its closing bracket, at its own indentation.
            if end < all.len() && indent(all[end]) == i0 && all[end].trim_start().starts_with(['}', ')', ']']) {
                end += 1;
            }
            while end > start + 1 && all[end - 1].trim().is_empty() {
                end -= 1;
            }
            all[start..end].to_vec()
        }
    };
    let cut = got.iter().filter(|l| !l.trim().is_empty()).map(|l| indent(l)).min().unwrap_or(0);
    Ok(got.iter().map(|l| l.get(cut..).unwrap_or("").to_string()).collect())
}

/// A ```chart line: a label, then a number, as it's to be shown.
fn chart_line(l: &str) -> Option<(String, String, f64)> {
    let l = l.trim();
    let (label, value) = l.rsplit_once(char::is_whitespace)?;
    let v: f64 = value.trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != '-').replace(',', "").parse().ok()?;
    Some((label.trim().to_string(), value.to_string(), v))
}

/// Markdown table rows as lines: columns lined up, numbers to the right,
/// the header bold with a rule under it.
fn table(p: &mut P, rows: &[(usize, &str)]) -> Block {
    let cells = |l: &str| -> Vec<String> {
        let t = l.trim();
        let t = t.strip_prefix('|').unwrap_or(t);
        let t = t.strip_suffix('|').unwrap_or(t);
        t.split('|').map(|c| c.trim().to_string()).collect()
    };
    let rule = |c: &[String]| c.iter().all(|x| !x.is_empty() && x.trim_matches(':').chars().all(|ch| ch == '-'));
    let mut head = 0;
    let mut grid: Vec<(usize, Vec<Line>)> = vec![];
    for &(n, l) in rows {
        let c = cells(l);
        if rule(&c) && head == 0 {
            head = grid.len();
            continue;
        }
        p.n = n;
        let lines = c.iter().map(|x| p.markup(x, 0)).collect();
        grid.push((n, lines));
    }
    let cols = grid.iter().map(|(_, r)| r.len()).max().unwrap_or(0);
    let width = |k: usize| grid.iter().filter_map(|(_, r)| r.get(k)).map(|l| markup::width(l)).max().unwrap_or(0);
    let widths: Vec<i32> = (0..cols).map(width).collect();
    let number = |k: usize| {
        grid.iter().skip(head).filter_map(|(_, r)| r.get(k)).all(|l| {
            let t = markup::text(l);
            let t = t.trim().trim_matches(|c: char| "$%€£".contains(c)).replace(',', "");
            t.is_empty() || t.parse::<f64>().is_ok()
        })
    };
    let right: Vec<bool> = (0..cols).map(number).collect();
    let mut b = Block::default();
    for (i, (n, r)) in grid.iter().enumerate() {
        let mut line = Line::new();
        for k in 0..cols {
            let empty = Line::new();
            let cell = r.get(k).unwrap_or(&empty);
            let cell: Line = if i < head { cell.iter().map(|c| markup::Cell { ch: c.ch, st: Style { bold: true, ..c.st } }).collect() } else { cell.clone() };
            let pad = markup::plain(&" ".repeat((widths[k] - markup::width(&cell)) as usize), Style::default());
            if k > 0 {
                line.extend(markup::plain("   ", Style::default()));
            }
            if right[k] && i >= head {
                line.extend(pad);
                line.extend(cell);
            } else {
                line.extend(cell);
                line.extend(pad);
            }
        }
        b.lines.push((false, line, *n));
        if i + 1 == head {
            let total = widths.iter().sum::<i32>() + 3 * (cols as i32 - 1).max(0);
            b.lines.push((false, markup::plain(&"─".repeat(total.max(0) as usize), Style::fg(p.theme.muted)), *n));
        }
    }
    b
}

fn textual(p: &mut P, s: &mut Slide, lines: &[&str], from: usize, end: usize) {
    let mut blocks: Vec<Block> = vec![];
    let mut fence: Option<usize> = None;
    // Inside ```lang: what the opening line said, and the lines so far.
    let mut code: Option<Open> = None;
    // Markdown table rows, till a line that isn't one.
    let mut rows: Vec<(usize, &str)> = vec![];
    for i in from..end {
        p.n = i;
        let l = lines[i];
        let row = fence.is_none() && l.trim_start().starts_with('|');
        if !row && !rows.is_empty() {
            blocks.push(table(p, &rows));
            rows.clear();
            p.n = i;
        }
        if row {
            rows.push((i, l));
            continue;
        }
        if let Some(info) = l.trim_start().strip_prefix("```") {
            match fence {
                None => {
                    fence = Some(i);
                    blocks.push(Block::default());
                    code = open(p, l, info, i, blocks.iter().any(|b| b.run.is_some()));
                }
                Some(f) => {
                    fence = None;
                    if let Some(o) = code.take() {
                        close(p, s, blocks.last_mut().unwrap(), o, f);
                    }
                }
            }
            continue;
        }
        if let Some(o) = code.as_mut() {
            o.src.push(l.to_string());
            continue;
        }
        if fence.is_none() {
            if let Some(n) = l.strip_prefix("//") {
                s.notes.push(n.strip_prefix(' ').unwrap_or(n).to_string());
                continue;
            }
            if heading(p, s, l) {
                continue;
            }
            if let Some(imgs) = image_line(l) {
                if !s.images.is_empty() && imgs.len() > 1 {
                    p.err(0, l.chars().count(), "pictures go all on one line (side by side) or each on its own (stacked), not both");
                }
                s.side = imgs.len() > 1;
                for (alt, path) in imgs {
                    let path = p.interp(&path);
                    let full = p.dir.join(&path);
                    if !full.is_file() {
                        let col = l.find(&path).map(|b| l[..b].chars().count()).unwrap_or(0);
                        p.err(col, col + path.chars().count(), format!("no file {}", full.display()));
                    }
                    let alt = p.interp(&alt);
                    s.images.push(Image { path: full, alt });
                }
                continue;
            }
        }
        let (step, text, col) = match l.strip_prefix("> ").or(l.strip_prefix(">").filter(|r| r.is_empty())) {
            Some(t) => (true, t, 2),
            None => (false, l, 0),
        };
        let line = p.markup(text, col);
        match fence {
            Some(_) => blocks.last_mut().unwrap().lines.push((step, line, i)),
            None => blocks.push(Block { lines: vec![(step, line, i)], ..Block::default() }),
        }
    }
    if !rows.is_empty() {
        blocks.push(table(p, &rows));
    }
    if let Some(f) = fence {
        p.n = f;
        p.err(0, 3, "a ``` with no ``` to close it");
    }
    let blank = |b: &Block| b.lines.len() == 1 && b.lines[0].1.is_empty() && !b.lines[0].0;
    while blocks.first().is_some_and(blank) {
        blocks.remove(0);
    }
    while blocks.last().is_some_and(blank) {
        blocks.pop();
    }
    let mut step = 0;
    for b in blocks {
        let at = s.body.len();
        let w = b.lines.iter().map(|(_, l, _)| markup::width(l)).max().unwrap_or(0);
        for (k, (is_step, mut line, src)) in b.lines.into_iter().enumerate() {
            let pad = w - markup::width(&line);
            line.extend(std::iter::repeat_n(markup::Cell { ch: ' ', st: Style::default() }, pad as usize));
            if is_step {
                step += 1;
            }
            let bar = b.bars.get(k).cloned().flatten();
            s.body.push(Body { line, src, step: if is_step { step } else { 0 }, code: b.code, bar });
        }
        // Each focus group a step of its own, after the block.
        for g in b.focus {
            step += 1;
            s.focus.push(Focus { step, block: at..s.body.len(), lines: g.into_iter().map(|k| at + k).collect() });
        }
        if let Some(choices) = b.poll {
            if s.poll.is_some() {
                p.n = s.line;
                p.err(0, 3, "a second poll; a slide has one");
            }
            let id = format!("{}\n{}", s.title(), choices.join("\n"));
            // The answer comes out on a step of its own, after the votes.
            let answer = b.answer.map(|k| {
                step += 1;
                (k, step)
            });
            s.poll = Some(Poll { id, choices, at, answer });
        }
        // Its output comes in on a step of its own, after the block.
        if let Some(argv) = b.run {
            step += 1;
            s.run = Some(Run { argv, step, at });
        }
    }
    if !s.images.is_empty() {
        p.n = s.line;
        if s.headline.is_some() {
            p.err(0, 3, "a picture slide has no headline: its label, pictures and caption");
        }
        if step > 0 {
            p.err(0, 3, "a picture slide has no steps");
        }
    }
}

/// A ``` opening: its language, then any of `run`, a file to take the
/// lines from (`path`, `path:10-24`, `path#name`), and `focus: 2|4-5`.
fn open(p: &mut P, l: &str, info: &str, i: usize, ran: bool) -> Option<Open> {
    let mut words = info.split_whitespace().peekable();
    let lang = words.next()?.to_string();
    let chart = lang == "chart" || lang == "poll";
    if !chart && !code::known(&lang) {
        let at = l.find(&lang).unwrap_or(0);
        p.err(at, at + lang.len(), format!("no language \"{lang}\"; {}try ts, tsx, js, rs, py, go, sh, json, yaml, sql, diff, chart or poll", spec::near(&lang, code::languages().iter().flat_map(|(n, full)| [n.as_str(), full.as_str()]).chain(["chart", "poll"]))));
    }
    let mut o = Open { lang, run: None, file: None, focus: vec![], at: i + 1, src: vec![] };
    while let Some(w) = words.next() {
        let at = l.find(w).unwrap_or(0);
        let span = at + w.chars().count();
        if w == "run" {
            if ran {
                p.err(at, span, "a second block that runs; a slide has one");
            } else {
                o.run = runner(&o.lang);
                if o.run.is_none() {
                    p.err(at, span, format!("deque can't run {}: sh, bash, zsh, fish, py, js and rb run", o.lang));
                }
            }
        } else if let Some(spec) = w.strip_prefix("focus:") {
            let spec = if spec.is_empty() { words.next().unwrap_or("") } else { spec };
            match groups(spec) {
                Some(g) => o.focus = g,
                None => p.err(at, l.chars().count(), "focus: lines to light, a step each, like focus: 2|4-5,8"),
            }
        } else {
            // A file: path, path:10-24, or path#name; any of them @REV,
            // as it was at that revision in git.
            let (w, rev) = match w.rsplit_once('@') {
                Some((f, r)) => (f, Some(r)),
                None => (w, None),
            };
            let cut = w.find(['#', ':']).unwrap_or(w.len());
            let full = p.dir.join(&w[..cut]);
            let text = match rev {
                None => {
                    p.files.push(full.clone());
                    std::fs::read_to_string(&full).map_err(|_| format!("no file {}; after the language go run, focus: or a file", full.display()))
                }
                Some(r) => at_rev(p.dir, &w[..cut], r),
            };
            match text {
                Ok(text) => match excerpt(&text, (cut < w.len()).then(|| &w[cut..])) {
                    Ok(got) => o.file = Some(got),
                    Err(e) => p.err(at, span, format!("{}: {e}", &w[..cut])),
                },
                Err(e) => p.err(at, span, e),
            }
        }
    }
    Some(o)
}

/// A file as it was at a revision, by git, from the talk's folder.
fn at_rev(dir: &Path, file: &str, rev: &str) -> Result<String, String> {
    if rev.is_empty() || rev.starts_with('-') {
        return Err(format!("\"{rev}\" isn't a revision: after @ goes a commit, branch or tag, like @HEAD~2 or @v1.0"));
    }
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .arg("show")
        .arg(format!("{rev}:./{file}"))
        .output()
        .map_err(|e| format!("{file}@{rev} needs git: {e}"))?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr);
        return Err(format!("{file}@{rev}: {}", why.lines().next().unwrap_or("git couldn't show it").trim_start_matches("fatal: ")));
    }
    String::from_utf8(out.stdout).map_err(|_| format!("{file}@{rev} isn't text"))
}

/// A ``` block closing: a chart's bars, or code, highlighted, from its own
/// lines or a file's, with its focus steps.
fn close(p: &mut P, s: &mut Slide, b: &mut Block, o: Open, opened: usize) {
    if o.file.is_some() && o.src.iter().any(|l| !l.trim().is_empty()) {
        p.n = opened;
        p.err(0, 3, "a block from a file has no lines of its own");
    }
    if o.lang == "poll" {
        let mut choices: Vec<(usize, String)> = o.src.iter().enumerate().filter(|(_, l)| !l.trim().is_empty()).map(|(k, l)| (o.at + k, l.trim().to_string())).collect();
        if choices.len() < 2 {
            p.n = opened;
            p.err(0, 3, "a poll has two choices or more, a line each");
        }
        // A `* ` choice is the answer, kept from the watchers till its step.
        for (k, (n, c)) in choices.iter_mut().enumerate() {
            let Some(rest) = c.strip_prefix("* ") else { continue };
            *c = rest.trim().to_string();
            if b.answer.is_some() {
                p.n = *n;
                p.err(0, 1, "a second answer; a poll has one");
            }
            b.answer = Some(k);
        }
        let leads: Vec<Line> = choices
            .iter()
            .map(|(n, c)| {
                p.n = *n;
                p.markup(c, 0)
            })
            .collect();
        let lw = leads.iter().map(|l| markup::width(l)).max().unwrap_or(0);
        b.poll = Some(leads.iter().map(|l| markup::text(l).trim().to_string()).collect());
        for ((n, _), mut lead) in choices.iter().zip(leads) {
            lead.extend(markup::plain(&" ".repeat((lw + 2 - markup::width(&lead)) as usize), Style::default()));
            let bar = poll_bar(lead, 0, 0, p.theme.muted);
            b.lines.push((false, bar.line(1.0, Style::fg(p.theme.accent)), *n));
            b.bars.push(Some(bar));
        }
        return;
    }
    if o.lang == "chart" {
        let got: Vec<(usize, (String, String, f64))> = o
            .src
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
            .filter_map(|(k, l)| {
                // A number can be a ${NAME}, so a chart shows what was
                // measured; one not set has been said already.
                p.n = o.at + k;
                let said = p.diags.len();
                let filled = p.interp(l);
                match chart_line(&filled) {
                    Some(c) => Some((o.at + k, c)),
                    None if p.diags.len() > said => chart_line(&format!("{} 0", l.split("${").next().unwrap_or(""))).map(|c| (o.at + k, c)),
                    None => {
                        p.err(0, l.chars().count(), "a chart line is a label, then a number: builds 42");
                        None
                    }
                }
            })
            .collect();
        let most = got.iter().map(|(_, c)| c.2).fold(0.0, f64::max).max(f64::MIN_POSITIVE);
        let lw = got.iter().map(|(_, c)| c.0.chars().count()).max().unwrap_or(0);
        for (n, (label, shown, v)) in got {
            let lead = markup::plain(&format!("{label:<lw$}  "), Style::default());
            let tail = markup::plain(&format!(" {shown}"), Style::fg(p.theme.muted));
            let bar = Bar { lead, frac: (v / most).max(0.0), width: 32, tail };
            b.lines.push((false, bar.line(1.0, Style::fg(p.theme.accent)), n));
            b.bars.push(Some(bar));
        }
        return;
    }
    let from_file = o.file.is_some();
    let src = o.file.unwrap_or(o.src);
    let lit = code::highlight(&src, &o.lang, &p.theme);
    let n = lit.len();
    b.lines.extend(lit.into_iter().enumerate().map(|(k, l)| (false, l, if from_file { opened } else { o.at + k })));
    b.bars.extend(std::iter::repeat_n(None, n));
    b.code = true;
    b.run = o.run.map(|[prog, flag]| vec![prog.to_string(), flag.to_string(), src.join("\n")]);
    for g in o.focus.iter().filter(|_| n > 0) {
        if let Some(&bad) = g.iter().find(|&&k| k == 0 || k > n) {
            p.n = opened;
            p.err(0, 3, format!("focus: line {bad}, but the block has {n}"));
            return;
        }
    }
    b.focus = o.focus.into_iter().map(|g| g.into_iter().map(|k| k - 1).collect()).collect();
    s.lang.get_or_insert(o.lang);
}

/// A slide with a ```graph: drawn, its boxes and arrows laid out from its
/// lines (`a -> b -> c`, `..>` dotted, `: label` after the last), left to
/// right or, ```graph down, top to bottom; each `> ` line's on a step, or
/// with ```graph steps each arrow is, the first box there to start from;
/// lines after the graph centered under it.
fn graph(p: &mut P, s: &mut Slide, lines: &[&str], from: usize, end: usize) {
    // Each box's name, as written, and title; each arrow's ends, whether
    // it's dotted, and the line it's from; what comes on each step.
    let (mut names, mut titles): (Vec<String>, Vec<Line>) = (vec![], vec![]);
    let mut arrows: Vec<(usize, usize, bool, Line)> = vec![];
    let mut down = false;
    let mut captions: Vec<(usize, Line, usize)> = vec![];
    // What each step brings: new boxes, arrows (by index), and the line.
    let mut groups: Vec<Vec<(usize, Result<usize, usize>)>> = vec![vec![]];
    let (mut inside, mut seen) = (false, false);
    // ```graph steps: an arrow a step, with the box it reaches.
    let mut each = false;
    for i in from..end {
        p.n = i;
        let l = lines[i];
        if !inside {
            if let Some(n) = l.strip_prefix("//") {
                s.notes.push(n.strip_prefix(' ').unwrap_or(n).to_string());
                continue;
            }
            if l.trim().is_empty() || heading(p, s, l) {
                continue;
            }
        }
        if let Some(info) = l.trim_start().strip_prefix("```") {
            if inside {
                inside = false;
            } else if let Some(how) = info.trim().strip_prefix("graph").filter(|h| !seen && h.split_whitespace().all(|w| matches!(w, "right" | "down" | "steps"))) {
                (inside, seen) = (true, true);
                (down, each) = (how.split_whitespace().any(|w| w == "down"), how.split_whitespace().any(|w| w == "steps"));
            } else {
                p.err(0, l.chars().count(), "a graph slide has one ```graph, and no other blocks");
            }
            continue;
        }
        let (step, text, col) = match l.strip_prefix("> ") {
            Some(t) => (true, t, 2),
            None => (false, l, 0),
        };
        if step {
            groups.push(vec![]);
        }
        let g = groups.len() - 1;
        if !inside {
            if seen {
                let line = p.markup(text, col);
                captions.push((i, line, g));
            } else {
                p.err(0, l.chars().count(), "text goes under the graph, after its closing ```");
            }
            continue;
        }
        if text.trim().is_empty() {
            continue;
        }
        // A label for the last arrow: after it, `: label`.
        let tail = ["->", "..>"].iter().filter_map(|a| text.rfind(a).map(|k| k + a.len())).max();
        let (text, label) = match tail.and_then(|t| text[t..].find(": ").map(|k| t + k)) {
            Some(k) => {
                let at = col + text[..k + 2].chars().count();
                (&text[..k], Some(p.markup(&text[k + 2..], at)))
            }
            None => (text, None),
        };
        // Names between arrows: -> and ..>, dotted.
        let mut parts: Vec<(&str, bool)> = vec![];
        let mut rest = text;
        loop {
            let next = [("->", false), ("..>", true)].into_iter().filter_map(|(a, d)| rest.find(a).map(|k| (k, a.len(), d))).min();
            match next {
                Some((k, len, d)) => {
                    parts.push((&rest[..k], d));
                    rest = &rest[k + len..];
                }
                None => {
                    parts.push((rest, false));
                    break;
                }
            }
        }
        let (mut ids, mut new) = (vec![], vec![]);
        // What its step held before this line.
        let had = groups[g].len();
        for (name, _) in &parts {
            let name = name.trim();
            if name.is_empty() {
                p.err(col, l.chars().count(), "a box has a name: a -> b");
                ids.clear();
                break;
            }
            let k = match names.iter().position(|n| n == name) {
                Some(k) => k,
                None => {
                    let at = col + text.find(name).map(|b| text[..b].chars().count()).unwrap_or(0);
                    let t = p.markup(name, at);
                    names.push(name.to_string());
                    titles.push(t);
                    new.push(names.len() - 1);
                    // An arrow a step: only the line's first box comes now.
                    if !each || ids.is_empty() {
                        groups[g].push((i, Ok(names.len() - 1)));
                    }
                    names.len() - 1
                }
            };
            ids.push(k);
        }
        let hops = ids.len().saturating_sub(1);
        for (h, (w, (_, dotted))) in ids.windows(2).zip(&parts).enumerate() {
            if w[0] == w[1] {
                p.err(col, l.chars().count(), "an arrow from a box to itself");
                continue;
            }
            let label = if h + 1 == hops { label.clone().unwrap_or_default() } else { vec![] };
            arrows.push((w[0], w[1], *dotted, label));
            if each {
                // A step of its own, but for a `> ` line's first arrow,
                // which that line's step brings with the box it's from.
                if h > 0 || g == 0 || had > 0 {
                    groups.push(vec![]);
                }
                if new.contains(&w[1]) {
                    groups.last_mut().unwrap().push((i, Ok(w[1])));
                }
            }
            groups.last_mut().unwrap().push((i, Err(arrows.len() - 1)));
        }
    }
    if inside {
        p.n = from;
        p.err(0, 3, "a ``` with no ``` to close it");
    }
    if s.headline.is_some() {
        p.n = s.line;
        p.err(0, 3, "a graph slide has no headline: its label, the graph, and lines under it");
    }
    if names.is_empty() {
        p.n = s.line;
        p.err(0, 3, "an empty graph: lines like a -> b -> c");
    }
    let widths: Vec<i32> = titles.iter().map(|t| markup::width(t) + 4).collect();
    let edges: Vec<crate::graph::Edge> = arrows.iter().map(|a| crate::graph::Edge { from: a.0, to: a.1, dotted: a.2, label: markup::width(&a.3) }).collect();
    let l = crate::graph::layout(&widths, &edges, down);
    // Two rows down, under the label.
    let top = 2;
    let paths: Vec<Vec<(i32, i32)>> = l.arrows.iter().map(|a| a.iter().map(|&(r, c)| (top + r, c)).collect()).collect();
    let (warm, muted) = (Style::fg(p.theme.warm), p.theme.muted);
    // Arrows drawn so far, as the steps go.
    let mut drawn: Vec<usize> = vec![];
    let mut draw: Vec<Vec<(usize, Item)>> = vec![];
    for g in groups {
        let mut items = vec![];
        let fresh: Vec<usize> = g.iter().filter_map(|(_, it)| it.err()).collect();
        // The step's own line, for what's added to it.
        let line = g.first().map_or(s.line, |x| x.0);
        for (i, it) in g {
            items.push(match it {
                Ok(k) => {
                    let (r, c, w) = l.boxes[k];
                    (i, Item::Box(top + r, c, w, 3, titles[k].clone()))
                }
                Err(e) => (i, Item::Path(paths[e].clone(), arrows[e].2)),
            });
        }
        drawn.extend(&fresh);
        // Where these arrows meet those before, and each other: joined.
        let all: Vec<(Vec<(i32, i32)>, bool, bool)> = drawn.iter().map(|&e| (paths[e].clone(), arrows[e].2, fresh.contains(&e))).collect();
        for (r, c, ch) in crate::graph::joins(&all) {
            items.push((line, Item::Text(r, c, vec![markup::Cell { ch, st: warm }])));
        }
        // Labels last, over their arrows, faint where they set no color.
        for &e in &fresh {
            if let Some((r, c)) = l.labels[e] {
                let text = arrows[e].3.iter().map(|x| markup::Cell { st: Style { fg: x.st.fg.or(Some(muted)), ..x.st }, ..*x }).collect();
                items.push((line, Item::Text(top + r, c, text)));
            }
        }
        draw.push(items);
    }
    let under = top + l.h + 1;
    for (k, (i, line, g)) in captions.iter().enumerate() {
        draw[*g].push((*i, Item::Center(under + k as i32, line.clone())));
    }
    let w = captions.iter().map(|c| markup::width(&c.1)).chain([l.w]).max().unwrap_or(0);
    s.draw = Some(Draw { w, h: under + captions.len() as i32, groups: draw });
}

fn drawing(p: &mut P, s: &mut Slide, lines: &[&str], from: usize, end: usize, w: i32, h: i32) {
    let mut groups: Vec<Vec<(usize, Item)>> = vec![vec![]];
    for i in from..end {
        p.n = i;
        let l = lines[i];
        if let Some(n) = l.strip_prefix("//") {
            s.notes.push(n.strip_prefix(' ').unwrap_or(n).to_string());
            continue;
        }
        if l.trim().is_empty() || heading(p, s, l) {
            continue;
        }
        let (cmd, rest) = l.split_once(' ').unwrap_or((l, ""));
        let at = cmd.len() + 1;
        // The next word, and the text after it with its first space gone.
        let word = |r: &str| -> (String, String) {
            let (a, b) = r.split_once(' ').unwrap_or((r, ""));
            (a.to_string(), b.to_string())
        };
        let pos = |p: &mut P, t: &str, col: usize| -> Option<(i32, i32)> {
            let rc = t.split_once(',').and_then(|(r, c)| Some((r.parse().ok()?, c.parse().ok()?)));
            if rc.is_none() {
                p.err(col, col + t.len(), format!("\"{t}\" isn't a place: ROW,COL, like 3,10"));
            }
            rc.inspect(|&(r, c)| {
                if r < 0 || c < 0 || r >= h || c >= w {
                    p.warn(col, col + t.len(), format!("{r},{c} is outside the {w}x{h} drawing"));
                }
            })
        };
        let item = match cmd {
            "step" => {
                groups.push(vec![]);
                continue;
            }
            "text" => {
                let (a, t) = word(rest);
                pos(p, &a, at).map(|(r, c)| {
                    let line = p.markup(&t, at + a.len() + 1);
                    Item::Text(r, c, line)
                })
            }
            "center" => match rest.split_once(' ').map(|(r, t)| (r.parse::<i32>(), t)) {
                Some((Ok(r), t)) => Some(Item::Center(r, p.markup(t, at))),
                _ => {
                    p.err(at, l.chars().count(), "center ROW text");
                    None
                }
            },
            "box" => {
                let (a, r) = word(rest);
                let (size, title) = word(&r);
                let wh = size.split_once('x').and_then(|(w, h)| Some((w.parse::<i32>().ok()?, h.parse::<i32>().ok()?)));
                match (pos(p, &a, at), wh) {
                    (Some((r, c)), Some((bw, bh))) if bw >= 4 && bh >= 2 => {
                        let title = p.markup(&title, at + a.len() + size.len() + 2);
                        Some(Item::Box(r, c, bw, bh, title))
                    }
                    (Some(_), _) => {
                        let col = at + a.len() + 1;
                        p.err(col, col + size.len(), "a box's size is WIDTHxHEIGHT, at least 4x2: box 3,0 30x3 title");
                        None
                    }
                    _ => None,
                }
            }
            "arrow" | "dotted" => {
                let mut pts = vec![];
                let mut col = at;
                for t in rest.split(' ') {
                    if !t.is_empty() {
                        pts.extend(pos(p, t, col));
                    }
                    col += t.len() + 1;
                }
                let bent = pts.windows(2).any(|q| q[0].0 != q[1].0 && q[0].1 != q[1].1);
                if pts.len() < 2 || bent || pts.windows(2).any(|q| q[0] == q[1]) {
                    p.err(at, l.chars().count(), format!("{cmd} goes through two points or more, each straight across or down from the last"));
                    None
                } else {
                    Some(Item::Path(pts, cmd == "dotted"))
                }
            }
            "clear" => {
                let rows = rest.split_once('-').map(|(a, b)| (a.parse(), b.parse())).unwrap_or((rest.parse(), rest.parse()));
                match rows {
                    (Ok(a), Ok(b)) if a <= b => Some(Item::Clear(a, b)),
                    _ => {
                        p.err(at, l.chars().count(), "clear ROW-ROW, or clear ROW");
                        None
                    }
                }
            }
            _ => {
                let cmds = spec::DRAW.iter().map(|d| d.0).collect::<Vec<_>>().join(", ");
                p.err(0, cmd.chars().count(), format!("no drawing command \"{cmd}\"; {}there's {cmds}", spec::near(cmd, spec::DRAW.iter().map(|d| d.0))));
                None
            }
        };
        groups.last_mut().unwrap().extend(item.map(|it| (i, it)));
    }
    if s.headline.is_some() {
        p.n = s.line;
        p.err(0, 3, "a drawn slide has no headline; draw it with text and boxes");
    }
    s.draw = Some(Draw { w, h, groups });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times() {
        assert_eq!(seconds("90"), Some(90));
        assert_eq!(seconds("90s"), Some(90));
        assert_eq!(seconds("2m"), Some(120));
        assert_eq!(seconds("1m30s"), Some(90));
        assert_eq!(seconds("soon"), None);
        assert_eq!(seconds("0"), None);
    }

    #[test]
    fn focus_groups() {
        assert_eq!(groups("2|4-5,8"), Some(vec![vec![2], vec![4, 5, 8]]));
        assert_eq!(groups("x"), None);
    }

    const SRC: &str = "import x;\n\nexport function fee(o) {\n  if (o) {\n    return 0;\n  }\n\n  return 1;\n}\n\nfunction fees() {}\n";

    #[test]
    fn excerpts() {
        // A function by name, its body and closing brace, dedented.
        let got = excerpt(SRC, Some("#fee")).unwrap();
        assert_eq!(got.first().unwrap(), "export function fee(o) {");
        assert_eq!(got.last().unwrap(), "}");
        assert_eq!(got.len(), 7);
        // A whole word: #fees isn't in #fee.
        assert_eq!(excerpt(SRC, Some("#fees")).unwrap(), ["function fees() {}"]);
        // Lines, dedented.
        assert_eq!(excerpt(SRC, Some(":4-5")).unwrap(), ["if (o) {", "  return 0;"]);
        assert!(excerpt(SRC, Some(":40-50")).is_err());
        assert!(excerpt(SRC, Some("#nope")).is_err());
    }

    #[test]
    fn code_as_it_was_in_git() {
        let dir = std::env::temp_dir().join(format!("deque-git-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| assert!(std::process::Command::new("git").arg("-C").arg(&dir).args(args).output().unwrap().status.success());
        git(&["init", "-q"]);
        std::fs::write(dir.join("a.ts"), "function fee() {\n  return 1;\n}\n").unwrap();
        git(&["add", "."]);
        git(&["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-qm", "one"]);
        std::fs::write(dir.join("a.ts"), "function fee() {\n  return 2;\n}\n").unwrap();
        let (t, d) = parse("---\n```ts a.ts#fee@HEAD\n```\n---\n```ts a.ts#fee\n```\n", &dir, false);
        assert!(d.is_empty(), "{d:?}");
        let line = |n: usize| markup::text(&t.slides[n].body[1].line);
        assert_eq!((line(0).trim(), line(1).trim()), ("return 1;", "return 2;"));
        // Code in the same language on the next slide: it morphs.
        assert!(t.morphs(0, 1));
        let (_, d) = parse("---\n```ts a.ts@nope\n```\n---\n```ts a.ts@-x\n```\n", &dir, false);
        assert_eq!(d.len(), 2, "{d:?}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn charts_and_tables() {
        assert_eq!(chart_line("by hand  0.5"), Some(("by hand".into(), "0.5".into(), 0.5)));
        assert_eq!(chart_line("cost $3,500"), Some(("cost".into(), "$3,500".into(), 3500.0)));
        assert_eq!(chart_line("nothing"), None);
        let (t, d) = parse("---\n| a | n |\n|---|---|\n| x | 7 |\n| yy | 10 |\n", Path::new("."), false);
        assert!(d.is_empty(), "{d:?}");
        let rows: Vec<String> = t.slides[0].body.iter().map(|b| markup::text(&b.line)).collect();
        // Header, a rule, and the numbers to the right.
        assert_eq!(rows, ["a    n ", "───────", "x     7", "yy   10"]);
    }

    #[test]
    fn graph_steps_is_an_arrow_a_step() {
        let (t, d) = parse("---\n## g\n```graph steps\na -> b -> c\n> d -> b\n```\nunder\n", Path::new("."), false);
        assert!(d.is_empty(), "{d:?}");
        // a alone, then b, then c, then d with its arrow (and the caption).
        assert_eq!(t.slides[0].steps(), 3);
        let (t, _) = parse("---\n## g\n```graph\na -> b -> c\n> d -> b\n```\n", Path::new("."), false);
        assert_eq!(t.slides[0].steps(), 1);
    }

    #[test]
    fn a_chart_number_can_come_from_the_environment() {
        // Set for this test alone, under a name nothing else reads.
        unsafe { std::env::set_var("DEQUE_TEST_BUILDS", "42") };
        let (t, d) = parse("---\n```chart\nclaude  ${DEQUE_TEST_BUILDS}\nhand  21\n```\n", Path::new("."), false);
        assert!(d.is_empty(), "{d:?}");
        let bars: Vec<f64> = t.slides[0].body.iter().map(|b| b.bar.as_ref().unwrap().frac).collect();
        assert_eq!(bars, [1.0, 0.5]);
        // Not set: said once, as the name not being set, and the bar kept.
        let (t, d) = parse("---\n```chart\nclaude  ${DEQUE_TEST_NOPE}\nhand  21\n```\n", Path::new("."), false);
        assert_eq!((d.len(), t.slides[0].body.len()), (1, 2), "{d:?}");
    }

    #[test]
    fn a_doubled_dollar_is_left_as_written() {
        let (t, d) = parse("---\n`$${sh: date}` and $${HOME}, ${sh: echo 7}\n", Path::new("."), false);
        assert!(d.is_empty(), "{d:?}");
        assert_eq!(markup::text(&t.slides[0].body[0].line), "${sh: date} and ${HOME}, 7");
    }

    // What sh runs; cmd, on Windows, wouldn't.
    #[cfg(unix)]
    #[test]
    fn a_command_says_what_it_printed() {
        let src = "---\nsky: boids ${sh: echo 12} 3\n# ${sh: echo hi}\n${sh: printf 'a\\nb' | awk '{print $1}'} done\n";
        let (t, d) = parse(src, Path::new("."), false);
        assert!(d.is_empty(), "{d:?}");
        let s = &t.slides[0];
        assert_eq!((s.headline.as_deref(), markup::text(&s.body[0].line).as_str(), s.fx.sky.as_deref(), t.live), (Some("hi"), "a b done", Some("boids 12 3"), true));
        // In the editor it isn't run.
        let (t, d) = parse("---\n${sh: exit 1} tasks\n", Path::new("."), true);
        assert!(d.is_empty(), "{d:?}");
        assert_eq!(markup::text(&t.slides[0].body[0].line), "0 tasks");
        // An ant colony's said in words, any of them, and a number.
        let (t, d) = parse("---\nsky: ants ground fire sand 80\n---\nsky: ants\n---\nsky: ants gel leafcutter\n", Path::new("."), false);
        assert!(d.is_empty(), "{d:?}");
        assert_eq!(t.slides[0].fx.sky.as_deref(), Some("ants ground fire sand 80"));
        let (_, d) = parse("---\nsky: ants farm ground\n---\nsky: ants fier\n", Path::new("."), false);
        assert_eq!(d.len(), 2, "{d:?}");
        assert!(d[1].msg.contains("did you mean \"fire\""), "{d:?}");
        // One that fails says so, and a sky that takes no number.
        let (_, d) = parse("---\n${sh: echo no >&2; exit 3}\n---\nsky: life 4\n", Path::new("."), false);
        assert_eq!(d.len(), 2, "{d:?}");
        assert!(d[0].msg.ends_with("failed: no"), "{d:?}");
    }

    #[test]
    fn a_poll_answer_is_a_step_and_not_a_choice_of_its_own() {
        let (t, d) = parse("---\nkeys: on\n# HOW MANY\n```poll\n3\n* 7\n12\n```\n> then this\n", Path::new("."), false);
        assert!(d.is_empty(), "{d:?}");
        let s = &t.slides[0];
        let p = s.poll.clone().unwrap();
        // The star is gone from what watchers are asked, and from its name.
        assert_eq!((p.choices, p.answer, s.steps(), s.keys), (vec!["3".to_string(), "7".into(), "12".into()], Some((1, 1)), 2, Some(true)));
        let (_, d) = parse("---\n```poll\n* a\n* b\n```\n", Path::new("."), false);
        assert_eq!(d.len(), 1, "{d:?}");
    }

    #[test]
    fn polls_are_bars_of_votes() {
        let (mut t, d) = parse("---\n# WHICH\n```poll\ntabs\n**spaces**\n```\n", Path::new("."), false);
        assert!(d.is_empty(), "{d:?}");
        let s = &mut t.slides[0];
        let p = s.poll.clone().unwrap();
        assert_eq!((p.choices, p.at, p.id.as_str()), (vec!["tabs".to_string(), "spaces".to_string()], 0, "WHICH\ntabs\nspaces"));
        let text = |s: &Slide| s.body.iter().map(|b| markup::text(&b.line)).collect::<Vec<_>>();
        let before = text(s);
        s.tally(&[1, 4], Rgb(1, 1, 1), Rgb(2, 2, 2));
        let after = text(s);
        assert!(after[0].ends_with("    1") && after[1].ends_with("    4") && after[1].contains(&"█".repeat(32)));
        // Just as wide, so it stays put.
        assert_eq!(before.iter().map(|l| l.chars().count()).collect::<Vec<_>>(), after.iter().map(|l| l.chars().count()).collect::<Vec<_>>());
        let (_, d) = parse("---\n```poll\nonly\n```\n", Path::new("."), false);
        assert!(d.iter().any(|d| d.msg.contains("two choices")));
    }

    #[test]
    fn focus_steps_follow_the_block() {
        let (t, d) = parse("---\n```ts focus: 1|2\na\nb\n```\n> after\n", Path::new("."), false);
        assert!(d.is_empty(), "{d:?}");
        let s = &t.slides[0];
        assert_eq!(s.focus.iter().map(|f| (f.step, f.lines.clone())).collect::<Vec<_>>(), [(1, vec![0]), (2, vec![1])]);
        assert_eq!(s.body[2].step, 3);
        assert_eq!(s.steps(), 3);
    }
}
