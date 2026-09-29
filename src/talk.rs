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
    pub cols: Option<i32>,
    pub fx: Effects,
    /// The language of its first ```lang block.
    pub lang: Option<String>,
    pub run: Option<Run>,
    /// Its speaker notes, the `//` lines.
    pub notes: Vec<String>,
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

    pub fn steps(&self) -> usize {
        match &self.draw {
            Some(d) => d.groups.len() - 1,
            None => self.body.iter().map(|b| b.step).chain(self.run.as_ref().map(|r| r.step)).max().unwrap_or(0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Talk {
    pub theme: Theme,
    pub fx: Effects,
    pub cursor: bool,
    pub slides: Vec<Slide>,
    pub dir: PathBuf,
    /// The environment variables it reads, for --tv to take along.
    pub vars: Vec<String>,
}

impl Talk {
    pub fn fx(&self, s: &Slide) -> String {
        s.fx.fx.clone().or(self.fx.fx.clone()).unwrap_or("wipe".into())
    }
    pub fn lines(&self, s: &Slide) -> String {
        s.fx.lines.clone().or(self.fx.lines.clone()).unwrap_or("type".into())
    }
    pub fn reveal(&self, s: &Slide) -> String {
        s.fx.reveal.clone().or(self.fx.reveal.clone()).unwrap_or("glide".into())
    }
    pub fn then(&self, s: &Slide) -> Vec<String> {
        s.fx.then.clone().or(self.fx.then.clone()).unwrap_or_default()
    }
    pub fn tr(&self, s: &Slide) -> String {
        s.fx.tr.clone().or(self.fx.tr.clone()).unwrap_or("none".into())
    }
    pub fn sky(&self, s: &Slide) -> String {
        s.fx.sky.clone().or(self.fx.sky.clone()).unwrap_or("none".into())
    }
    pub fn glow(&self, s: &Slide) -> bool {
        s.fx.glow.clone().or(self.fx.glow.clone()).is_some_and(|g| g == "on")
    }
    /// Whether slide `to` arrives from `from` by turning into it: all of
    /// it with `tr: morph`, or its code, when both have code in the same
    /// language and `to` sets no `tr:` of its own.
    pub fn morphs(&self, from: usize, to: usize) -> bool {
        let text = |s: &Slide| s.images.is_empty() && s.draw.is_none();
        let (a, b) = (&self.slides[from], &self.slides[to]);
        if !text(a) || !text(b) {
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
    diags: Vec<Diag>,
    theme: Theme,
    vars: Vec<String>,
    lenient: bool,
    dir: &'a Path,
    n: usize,
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
    /// sets.
    fn interp(&mut self, s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(i) = rest.find("${") {
            let Some(j) = rest[i..].find('}') else { break };
            let name = &rest[i + 2..i + j];
            out.push_str(&rest[..i]);
            let col = s.len() - rest.len() + i;
            let (col, end) = (s[..col].chars().count(), s[..col + j + 1].chars().count());
            if !self.vars.iter().any(|v| v == name) {
                self.vars.push(name.to_string());
            }
            match std::env::var(name) {
                Ok(v) => out.push_str(&v),
                Err(_) if self.lenient => self.warn(col, end, format!("${{{name}}} isn't set here; it must be when the talk runs")),
                Err(_) => self.err(col, end, format!("${{{name}}} isn't set")),
            }
            rest = &rest[i + j + 1..];
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
            _ => format!("no {what} \"{v}\"; there's {}", spec::names(set)),
        };
        self.err(col, col + v.chars().count(), msg);
        None
    }

    /// An effect option, into e. Returns whether the key was one.
    fn effect(&mut self, e: &mut Effects, k: &str, v: &str, col: usize) -> bool {
        match k {
            "fx" => e.fx = self.one_of("fx", v, col, spec::FX),
            "lines" => e.lines = self.one_of("lines effect", v, col, spec::LINES),
            "reveal" => e.reveal = self.one_of("reveal effect", v, col, spec::LINES),
            "tr" => e.tr = self.one_of("transition", v, col, spec::TR),
            "sky" => e.sky = self.one_of("sky", v, col, spec::SKY),
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
    let mut p = P { diags: vec![], theme: Theme::default(), vars: vec![], lenient, dir, n: 0 };
    let mut talk_fx = Effects::default();
    let mut cursor = true;
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
            _ if spec::find(spec::TALK, k).is_some() => match Rgb::parse(&v) {
                Some(c) => {
                    p.theme.set(k, c);
                }
                None if v.is_empty() && lenient => {}
                None => p.err(col, l.chars().count(), format!("\"{v}\" isn't a color: #rrggbb, or 38;2;r;g;b")),
            },
            _ => {
                let msg = format!("no setting \"{k}\"; there's {}", spec::TALK.iter().map(|o| o.key).collect::<Vec<_>>().join(", "));
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
    let talk = Talk { theme: p.theme.clone(), fx: talk_fx, cursor, slides, dir: dir.to_path_buf(), vars: p.vars.clone() };
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
            if i == start + 1 {
                // Maybe text that has a colon in it; say so, but show it.
                p.warn(0, k.len(), format!("shown as text: \"{k}\" isn't a slide option ({keys}). Options go right after `---`"));
                break;
            }
            p.err(0, k.len(), format!("no slide option \"{k}\"; there's {keys}. Put a blank line between options and text"));
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
            "cols" => match v.parse() {
                Ok(n) if n >= 20 => s.cols = Some(n),
                _ => p.err(col, l.chars().count(), "cols is a number of columns, 20 or more"),
            },
            "draw" => match v.split_once('x').and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?))) {
                Some((w, h)) => draw_size = Some((w, h)),
                None => p.err(col, l.chars().count(), "draw is the drawing's size, WIDTHxHEIGHT: draw: 76x20"),
            },
            _ => unreachable!(),
        }
    }
    if s.cols.is_some() && s.enter.is_none() {
        p.n = start;
        p.warn(0, 3, "cols does nothing without enter");
    }
    if let Some((w, h)) = draw_size {
        drawing(p, &mut s, lines, i, end, w, h);
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
}

fn textual(p: &mut P, s: &mut Slide, lines: &[&str], from: usize, end: usize) {
    let mut blocks: Vec<Block> = vec![];
    let mut fence: Option<usize> = None;
    // Inside ```lang: the language, whether it runs, the line after the ```,
    // and the lines so far, as they are.
    let mut code: Option<(String, Option<[&str; 2]>, usize, Vec<String>)> = None;
    for i in from..end {
        p.n = i;
        let l = lines[i];
        if let Some(info) = l.trim_start().strip_prefix("```") {
            let mut words = info.split_whitespace();
            let lang = words.next().unwrap_or("");
            match fence {
                None => {
                    fence = Some(i);
                    blocks.push(Block::default());
                    if !lang.is_empty() {
                        if !code::known(lang) {
                            let at = l.find(lang).unwrap_or(0);
                            p.err(at, at + lang.len(), format!("no language \"{lang}\"; try ts, tsx, js, rs, py, go, sh, json, yaml, sql, diff"));
                        }
                        let mut run = None;
                        if let Some(w) = words.next() {
                            let at = l.rfind(w).unwrap_or(0);
                            if w != "run" {
                                p.err(at, at + w.len(), format!("after the language goes `run`, or nothing; not \"{w}\""));
                            } else if blocks.iter().any(|b| b.run.is_some()) {
                                p.err(at, at + w.len(), "a second block that runs; a slide has one");
                            } else {
                                run = runner(lang);
                                if run.is_none() {
                                    p.err(at, at + w.len(), format!("deque can't run {lang}: sh, bash, zsh, fish, py, js and rb run"));
                                }
                            }
                        }
                        code = Some((lang.to_string(), run, i + 1, vec![]));
                    }
                }
                Some(_) => {
                    fence = None;
                    if let Some((lang, run, at, src)) = code.take() {
                        let lit = code::highlight(&src, &lang, &p.theme);
                        s.lang.get_or_insert(lang);
                        let b = blocks.last_mut().unwrap();
                        b.lines.extend(lit.into_iter().enumerate().map(|(k, l)| (false, l, at + k)));
                        b.code = true;
                        b.run = run.map(|[prog, flag]| vec![prog.to_string(), flag.to_string(), src.join("\n")]);
                    }
                }
            }
            continue;
        }
        if let Some((.., src)) = code.as_mut() {
            src.push(l.to_string());
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
        for (is_step, mut line, src) in b.lines {
            let pad = w - markup::width(&line);
            line.extend(std::iter::repeat_n(markup::Cell { ch: ' ', st: Style::default() }, pad as usize));
            if is_step {
                step += 1;
            }
            s.body.push(Body { line, src, step: if is_step { step } else { 0 }, code: b.code });
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
                p.err(0, cmd.chars().count(), format!("no drawing command \"{cmd}\"; there's {cmds}"));
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
