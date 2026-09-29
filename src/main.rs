//! deque: a talk in the terminal, from a text file.

mod figlet;
mod cast;
mod code;
mod fx;
mod ghostty;
mod images;
mod link;
mod lsp;
mod markup;
mod morph;
mod notes;
mod preview;
mod render;
mod run;
mod screen;
mod spec;
mod talk;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use render::Mode;
use screen::Screen;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, SystemTime};
use talk::{Diag, Talk};

const HELP: &str = "\
deque: slides in your terminal

  deque TALK [N]       present TALK, from slide N
  deque TALK --print   every slide as text
  deque TALK --cast FILE [--size WxH]
                       record the talk played through as an asciinema
                       cast, 100x30 unless --size says; agg makes a GIF
  deque TALK --tv      fullscreen in a new Ghostty window, the font sized for
                       the screen (macOS); + and − size it
  deque notes TALK     speaker notes, the next slide and a timer, for a
                       second screen; its keys drive the talk
  deque preview TALK   the slide an editor's cursor is in, still; see
                       the README for the Neovim plugin that runs it
  deque check TALK     the talk's problems, if any
  deque lsp            the language server, for editors

  --cursor, --no-cursor   show or hide the cursor (the talk's `cursor:` otherwise)

keys: → space enter n on · ← b back · 12 enter: slide 12 · o all slides
      r replay the slide · home end · q quit
The talk reloads when you save it, and shows the slide you changed.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("-h" | "--help" | "help") => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Some("-V" | "--version") => {
            println!("deque {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("notes") => match args.get(1) {
            Some(p) => match notes::run(Path::new(p), |p| load(p, true)) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => {
                    eprint!("{e}");
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!("usage: deque notes TALK");
                ExitCode::from(2)
            }
        },
        Some("preview") => {
            let from = args.iter().position(|a| a == "--from").and_then(|i| args.get(i + 1)).map(Path::new);
            match args.get(1).filter(|a| !a.starts_with('-')) {
                Some(p) => match preview::run(Path::new(p), from, args.iter().any(|a| a == "--still")) {
                    Ok(()) => ExitCode::SUCCESS,
                    Err(e) => {
                        eprintln!("{e}");
                        ExitCode::FAILURE
                    }
                },
                None => {
                    eprintln!("usage: deque preview TALK [--from FILE] [--still]");
                    ExitCode::from(2)
                }
            }
        }
        Some("lsp") => {
            lsp::run();
            ExitCode::SUCCESS
        }
        Some("check") => match args.get(1) {
            Some(p) => match load(Path::new(p), false) {
                Ok(_) => ExitCode::SUCCESS,
                Err(e) => {
                    eprint!("{e}");
                    ExitCode::FAILURE
                }
            },
            None => {
                eprintln!("usage: deque check TALK");
                ExitCode::from(2)
            }
        },
        _ => match present(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprint!("{e}");
                ExitCode::FAILURE
            }
        },
    }
}

fn describe(path: &Path, diags: &[Diag]) -> String {
    diags
        .iter()
        .map(|d| format!("{}:{}:{}: {}: {}\n", path.display(), d.line + 1, d.start + 1, if d.warn { "warning" } else { "error" }, d.msg))
        .collect()
}

/// The talk, or its errors. Warnings are printed and let through.
fn load(path: &Path, quiet: bool) -> Result<Talk, String> {
    let src = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}\n", path.display()))?;
    let dir = path.parent().filter(|d| !d.as_os_str().is_empty()).unwrap_or(Path::new("."));
    let dir = dir.canonicalize().unwrap_or(dir.to_path_buf());
    let (talk, diags) = talk::parse(&src, &dir, false);
    if diags.iter().any(|d| !d.warn) {
        return Err(describe(path, &diags));
    }
    if !quiet {
        eprint!("{}", describe(path, &diags));
    }
    Ok(talk)
}

fn modified(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

fn present(args: &[String]) -> Result<(), String> {
    let mut path: Option<PathBuf> = None;
    let (mut start, mut print, mut tv, mut cursor) = (1usize, false, false, None);
    let (mut cast, mut size) = (None, (100, 30));
    let mut rest = vec![];
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--print" => print = true,
            "--tv" => tv = true,
            "--cast" => cast = Some(PathBuf::from(it.next().ok_or("deque: --cast FILE: the file to write\n")?)),
            "--size" => {
                let v = it.next().map(String::as_str).unwrap_or_default();
                size = v
                    .split_once('x')
                    .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
                    .filter(|&(w, h)| w >= 20 && h >= 10)
                    .ok_or(format!("deque: --size is WIDTHxHEIGHT, like 100x30, not \"{v}\"\n"))?;
            }
            "--cursor" => cursor = Some(true),
            "--no-cursor" => cursor = Some(false),
            _ if a.starts_with('-') => return Err(format!("deque: no option {a}\n\n{HELP}\n")),
            _ if path.is_none() => path = Some(PathBuf::from(a)),
            _ => start = a.parse().map_err(|_| format!("deque: \"{a}\" isn't a slide number\n"))?,
        }
        if a != "--tv" {
            rest.push(a.clone());
        }
    }
    let path = path.ok_or(format!("{HELP}\n"))?;
    let mut talk = load(&path, false)?;
    if print {
        print!("{}", render::print_all(&talk, std::io::stdout().is_terminal()));
        return Ok(());
    }
    if let Some(file) = cast {
        return cast::record(&talk, &file, size.0, size.1);
    }
    if tv {
        let abs = path.canonicalize().map_err(|e| e.to_string())?;
        rest.retain(|a| Path::new(a) != path);
        rest.insert(0, abs.to_string_lossy().into_owned());
        return ghostty::tv(&rest, &talk.vars);
    }
    if !std::io::stdout().is_terminal() {
        return Err("deque: presenting needs a terminal (--print writes the slides as text)\n".into());
    }
    let cursor = cursor.unwrap_or(talk.cursor);

    let mut s = Screen::new(talk.theme.clone());
    terminal::enable_raw_mode().map_err(|e| e.to_string())?;
    // A panic puts the terminal back before it says what happened.
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |i| {
        restore();
        hook(i)
    }));
    let proto = images::detect();
    s.kitty = proto == images::Proto::Kitty;
    let mut pics = images::Pictures::new(proto);
    let show = if cursor { "\x1b[?25h" } else { "\x1b[?25l" };
    // The wheel, on this screen, is sent as ↑ and ↓ unless alternate scroll
    // is off (1007); off it goes.
    let enter = format!("\x1b[?1049h\x1b[?1007l{show}");
    s.raw(&enter);
    s.flush();
    let mut font = ghostty::Font::from_env();
    if let Some(f) = font.as_mut() {
        f.fit(&mut s);
    }

    let mut n = start.clamp(1, talk.slides.len()) - 1;
    let (mut mode, mut shown, mut started) = (Mode::Arrive, 0usize, false);
    let mut stamp = modified(&path);
    let mut src = std::fs::read_to_string(&path).unwrap_or_default();
    let mut problem: Option<String> = None;
    let mut link = link::Link::new(&path);
    // A slide number being typed, to go to on enter.
    let mut jump = String::new();
    // What each slide's run block printed when it last ran, by slide.
    let mut ran: std::collections::HashMap<usize, run::Output> = Default::default();
    loop {
        s.hurry = false;
        let slide = &talk.slides[n];
        // Before the slide plays in, so the notes change with the key, not
        // after the animation.
        link.publish(n, shown);
        if mode == Mode::Arrive && started {
            fx::transition(&mut s, &talk.tr(slide));
        }
        started = true;
        render::draw(&mut s, &talk, &mut pics, n, mode, shown, font.is_some());
        if let Some(r) = &slide.run {
            if mode == Mode::Step && shown == r.step {
                ran.insert(n, run::go(&mut s, &talk, n));
            } else if let Some(o) = ran.get(&n).filter(|_| shown >= r.step) {
                run::show(&mut s, &talk, n, o, false);
            }
        }
        if let Some(p) = &problem {
            let st = markup::Style::fg(talk.theme.bad);
            s.put_str(s.h, 1, p, st);
        }
        if !jump.is_empty() {
            let st = markup::Style::fg(talk.theme.accent);
            s.put_str(s.h, 2, &format!("go to {jump}_  "), st);
        }
        s.flush();
        mode = Mode::Still;
        // Wait for a key, looking every tenth of a second for a resize, the
        // talk saved, or the notes window asking for something.
        let act = match s.pending.take() {
            Some(k) => act(k),
            None => loop {
                if s.resized {
                    s.resized = false;
                    s.size();
                    break Act::Redraw;
                }
                if event::poll(Duration::from_millis(100)).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => break act(k),
                        Ok(Event::Resize(..)) => s.resized = true,
                        _ => {}
                    }
                    continue;
                }
                if let Some(c) = link.take() {
                    break match c.split_once(' ') {
                        Some(("goto", k)) => k.parse().map_or(Act::Redraw, Act::Goto),
                        _ => match c.as_str() {
                            "next" => Act::Next,
                            "back" => Act::Back,
                            "replay" => Act::Replay,
                            "first" => Act::Goto(0),
                            "last" => Act::Last,
                            _ => Act::Redraw,
                        },
                    };
                }
                let now = modified(&path);
                if now != stamp {
                    stamp = now;
                    let new = std::fs::read_to_string(&path).unwrap_or_default();
                    match load(&path, true) {
                        Ok(t) => {
                            // To the slide just edited, whole: the talk
                            // beside the editor, as a preview.
                            let edited = changed(&src, &talk, &new, &t);
                            talk = t;
                            src = new;
                            ran.clear();
                            s.theme = talk.theme.clone();
                            problem = None;
                            n = edited.unwrap_or(n).min(talk.slides.len() - 1);
                            shown = if edited.is_some() { talk.slides[n].steps() } else { shown.min(talk.slides[n].steps()) };
                        }
                        Err(e) => problem = e.lines().next().map(String::from),
                    }
                    break Act::Redraw;
                }
            },
        };
        let slide = &talk.slides[n];
        let steps = slide.steps();
        let last = talk.slides.len() - 1;
        // Enter goes to a typed number first, then runs a live part.
        let act = match act {
            Act::Digit(d) => {
                jump.push(d);
                continue;
            }
            Act::Erase if !jump.is_empty() => {
                jump.pop();
                continue;
            }
            Act::Quit if !jump.is_empty() => {
                jump.clear();
                continue;
            }
            Act::Next | Act::Live if !jump.is_empty() => {
                let k = jump.parse::<usize>().unwrap_or(1).clamp(1, last + 1) - 1;
                jump.clear();
                Act::Goto(k)
            }
            Act::Live if slide.enter.is_none() => Act::Next,
            a => a,
        };
        match act {
            Act::Quit => break,
            Act::Grow(by) => {
                if let Some(f) = font.as_mut() {
                    f.grow(&mut s, by);
                }
            }
            // Back is the slide before, whole: nobody steps through a list twice.
            Act::Back if n > 0 => {
                n -= 1;
                shown = talk.slides[n].steps();
            }
            Act::Last => (n, shown) = (last, talk.slides[last].steps()),
            Act::Goto(k) => (n, shown, mode) = (k.min(last), 0, Mode::Arrive),
            Act::Replay => (shown, mode, started) = (0, render::arrive(&talk, n), false),
            Act::Live => live(&mut s, &talk, slide, font.as_ref(), &enter),
            Act::Overview => {
                if let Some(k) = render::overview(&mut s, &talk, n) {
                    (n, shown, mode) = (k, 0, Mode::Arrive);
                }
            }
            Act::Next if shown < steps => (shown, mode) = (shown + 1, Mode::Step),
            Act::Next if n < last => (n, shown, mode) = (n + 1, 0, render::arrive(&talk, n + 1)),
            _ => {}
        }
    }
    link.gone();
    restore();
    Ok(())
}

/// What a key asks for.
enum Act {
    None,
    Redraw,
    Quit,
    Next,
    Back,
    Last,
    Goto(usize),
    Replay,
    Live,
    Overview,
    Grow(i32),
    Digit(char),
    Erase,
}

fn act(k: KeyEvent) -> Act {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Char('c') if ctrl => Act::Quit,
        KeyCode::Char('q') | KeyCode::Esc => Act::Quit,
        KeyCode::Char('+' | '=') => Act::Grow(1),
        KeyCode::Char('-') => Act::Grow(-1),
        KeyCode::Left | KeyCode::PageUp | KeyCode::Char('b' | 'p' | 'h' | 'k') => Act::Back,
        KeyCode::Home | KeyCode::Char('g') => Act::Goto(0),
        KeyCode::End | KeyCode::Char('G') => Act::Last,
        KeyCode::Char('r') => Act::Replay,
        KeyCode::Char('o') | KeyCode::Tab => Act::Overview,
        KeyCode::Char(d) if d.is_ascii_digit() => Act::Digit(d),
        KeyCode::Backspace => Act::Erase,
        KeyCode::Enter | KeyCode::Char(' ') => Act::Live,
        KeyCode::Right | KeyCode::PageDown | KeyCode::Char('n' | 'l' | 'j') => Act::Next,
        _ => Act::None,
    }
}

/// The first slide whose text changed between two versions of the talk.
fn changed(old_src: &str, old: &Talk, new_src: &str, new: &Talk) -> Option<usize> {
    let chunks = |src: &str, t: &Talk| -> Vec<String> {
        let lines: Vec<&str> = src.lines().collect();
        let starts: Vec<usize> = t.slides.iter().map(|s| s.line).chain([lines.len()]).collect();
        starts.windows(2).map(|w| lines[w[0]..w[1]].join("\n")).collect()
    };
    let (a, b) = (chunks(old_src, old), chunks(new_src, new));
    (0..b.len()).find(|&i| a.get(i) != Some(&b[i]))
}

/// A slide's live part: its command, run by the system's shell in the
/// talk's folder, with the terminal as it is outside the deck.
fn live(s: &mut Screen, talk: &Talk, slide: &talk::Slide, font: Option<&ghostty::Font>, enter: &str) {
    let cmd = slide.enter.as_deref().unwrap_or_default();
    if let (Some(f), Some(c)) = (font, slide.cols) {
        f.small(s, c);
    }
    s.raw("\x1b[?25h\x1b[0m\x1b[H\x1b[2J");
    s.flush();
    let _ = terminal::disable_raw_mode();
    let mut c = if cfg!(windows) {
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg(cmd);
        c
    } else {
        let mut c = std::process::Command::new("sh");
        c.arg("-c").arg(cmd);
        c
    };
    if let Err(e) = c.current_dir(&talk.dir).status() {
        eprintln!("deque: {cmd}: {e}");
        std::thread::sleep(Duration::from_secs(2));
    }
    let _ = terminal::enable_raw_mode();
    s.raw(enter);
    s.flush();
    if let Some(f) = font {
        f.set(s, f.size);
    }
    s.size();
}

fn restore() {
    use std::io::Write;
    let _ = terminal::disable_raw_mode();
    let mut o = std::io::stdout();
    let _ = o.write_all(b"\x1b[0m\x1b[?1007h\x1b[?1049l\x1b[?25h");
    let _ = o.flush();
}
