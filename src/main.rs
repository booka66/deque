//! deque: a talk in the terminal, from a text file.

mod figlet;
mod fine;
mod cast;
mod code;
mod fx;
mod ghostty;
mod graph;
mod images;
mod life;
mod link;
mod lsp;
mod markup;
mod morph;
mod notes;
mod preview;
mod rehearse;
#[cfg(unix)]
mod relay;
mod render;
mod replay;
mod rtc;
mod run;
mod screen;
mod share;
mod sky;
mod spec;
mod talk;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal;
use render::Mode;
use screen::Screen;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant, SystemTime};
use talk::{Diag, Talk};

const HELP: &str = "\
deque: slides in your terminal

  deque TALK [N]       present TALK, from slide N; TALK can be a folder
                       with a talk.deque, or left out when there's one here
  deque new NAME       write NAME.deque, a talk to start from
  deque TALK --print   every slide as text
  deque TALK --cast FILE [--size WxH]
                       record the talk played through as an asciinema
                       cast, 100x30 unless --size says; agg makes a GIF
  deque TALK --html FILE [--size WxH]
                       the same, as a page to post: → and ← step through
                       it, animations and all
  deque TALK --tv      fullscreen in a new Ghostty window, the font sized for
                       the screen (macOS); + and − size it
  deque notes TALK     speaker notes, the next slide and a timer, for a
                       second screen; its keys drive the talk
  deque preview TALK   the slide an editor's cursor is in, still; see
                       the README for the Neovim plugin that runs it
  deque check TALK [--size WxH]
                       the talk's problems, if any, and the slides cut off
                       on a screen that size (80x24 unless it says)
  deque lsp            the language server, for editors

  --cursor, --no-cursor   show or hide the cursor (the talk's `cursor:` otherwise)
  --calm                  nothing moves that needn't: no skies, glow,
                          flourishes, transitions or morphs (as calm: on)
  --rehearse              present it as a practice run: when it's over, the
                          time you spent on each slide is written into the
                          talk as its time:, for deque notes to pace you by
  --share                 stream it live, for anyone who can't see, in a
                          browser: a link that works from anywhere, through a
                          Cloudflare tunnel (cloudflared), with no browser
                          warning; on your network, straight from this
                          machine. Watchers react and vote. w shows the link
                          and a QR code; a slide can say ${DEQUE_URL}
                          P shows your phone remote: next, back, notes, and a
                          pointer, by finger or by aiming the phone; just for
                          you, not the room
  --share-local           --share, on this network only, not through
                          Cloudflare: for no internet, or a talk that mustn't
                          leave the room (browsers warn once: the certificate
                          is deque's own). --share falls back to it when it
                          can't open a tunnel
  --share-curl            --share, and offer curl -sN to watch in a terminal
                          on this network too
  --share-no-font         --share, but the page draws in its own monospace,
                          not the terminal's font (DEQUE_FACE, or its config)

keys: → space enter n on · ← b back · 12 enter: slide 12 · ' back from a jump
      o all slides, / to find one · r replay · B blank · w how to watch
      (--share) · home end · q q quit · ? all of them
the mouse is a laser pointer
The talk reloads when you save it, and shows the slide you changed.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        // No talk named: the one here, if there's one.
        None if found(Path::new(".")).is_some() => run(present(&[found(Path::new(".")).unwrap().to_string_lossy().into_owned()])),
        None | Some("-h" | "--help" | "help") => {
            println!("{HELP}");
            ExitCode::SUCCESS
        }
        Some("-V" | "--version") => {
            println!("deque {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("new") => match args.get(1) {
            Some(name) => run(new(name)),
            None => {
                eprintln!("usage: deque new NAME");
                ExitCode::from(2)
            }
        },
        Some("notes") => match args.get(1).map(|p| talk_at(p)) {
            Some(Err(e)) => run(Err(e)),
            Some(Ok(p)) => match notes::run(&p, |p| load(p, true)) {
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
            match args.get(1).filter(|a| !a.starts_with('-')).map(|p| talk_at(p)) {
                Some(Err(e)) => run(Err(e)),
                Some(Ok(p)) => match preview::run(&p, from, args.iter().any(|a| a == "--still")) {
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
        Some("check") => run(check(&args[1..])),
        _ => run(present(&args)),
    }
}

fn run(r: Result<(), String>) -> ExitCode {
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprint!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// The talk in a folder: its talk.deque, or its only .deque file.
fn found(dir: &Path) -> Option<PathBuf> {
    let t = dir.join("talk.deque");
    if t.is_file() {
        return Some(t);
    }
    let mut all = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "deque"));
    let one = all.next()?;
    all.next().is_none().then_some(one)
}

/// The talk a path means: the file, or the talk in the folder.
fn talk_at(p: &str) -> Result<PathBuf, String> {
    let p = PathBuf::from(p);
    if !p.is_dir() {
        return Ok(p);
    }
    found(&p).ok_or(format!("deque: no talk in {}: a talk.deque, or one .deque file\n", p.display()))
}

/// deque check TALK [--size WxH]: its problems, and the slides that don't
/// fit a screen that size, 80x24 unless it says.
fn check(args: &[String]) -> Result<(), String> {
    let (mut path, mut size) = (None, (80, 24));
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--size" => size = parse_size(it.next())?,
            _ if a.starts_with('-') => return Err(format!("deque: no option {a}\nusage: deque check TALK [--size WxH]\n")),
            _ => path = Some(talk_at(a)?),
        }
    }
    let path = path.ok_or("usage: deque check TALK [--size WxH]\n")?;
    let talk = load(&path, false)?;
    let (w, h) = size;
    for (n, slide) in talk.slides.iter().enumerate() {
        if let Some((nw, nh)) = render::needs(&talk, n, w, h) {
            eprintln!("{}:{}:1: warning: slide {} needs {nw}x{nh}, more than {w}x{h}: cut off on a screen that size", path.display(), slide.line + 1, n + 1);
        }
        if let Some(c) = slide.play.as_deref().and_then(|p| replay::load(p).ok())
            && (c.w > w || c.h > h)
        {
            eprintln!("{}:{}:1: warning: slide {}'s recording is {}x{}, more than {w}x{h}: cut off on a screen that size", path.display(), slide.line + 1, n + 1, c.w, c.h);
        }
    }
    Ok(())
}

fn parse_size(v: Option<&String>) -> Result<(i32, i32), String> {
    let v = v.map(String::as_str).unwrap_or_default();
    v.split_once('x')
        .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
        .filter(|&(w, h)| w >= 20 && h >= 10)
        .ok_or(format!("deque: --size is WIDTHxHEIGHT, like 100x30, not \"{v}\"\n"))
}

/// deque new NAME: a talk to start from, NAME.deque, a slide of each of
/// the things people reach for.
fn new(name: &str) -> Result<(), String> {
    let file = if name.ends_with(".deque") { PathBuf::from(name) } else { PathBuf::from(format!("{name}.deque")) };
    if file.exists() {
        return Err(format!("deque: {} is there already\n", file.display()));
    }
    let stem = file.file_stem().map(|s| s.to_string_lossy().to_uppercase()).unwrap_or_default();
    let title = if (1..=10).contains(&stem.len()) && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') { stem } else { "HELLO".into() };
    std::fs::write(&file, STARTER.replace("{TITLE}", &title)).map_err(|e| format!("deque: {}: {e}\n", file.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755));
    }
    println!("{} written. present it:\n\n  deque {}\n\nit reloads as you save it; ? shows the keys", file.display(), file.display());
    Ok(())
}

const STARTER: &str = include_str!("starter.deque");

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

/// When the talk, and each file its code comes from, last changed.
fn stamps(p: &Path, talk: &Talk) -> Vec<Option<SystemTime>> {
    std::iter::once(p).chain(talk.files.iter().map(PathBuf::as_path)).map(modified).collect()
}

fn present(args: &[String]) -> Result<(), String> {
    let mut path: Option<PathBuf> = None;
    let (mut start, mut print, mut tv, mut cursor, mut sharing, mut curl) = (1usize, false, false, None, false, false);
    let mut local = false;
    let mut own_font = true;
    let (mut calm, mut practice) = (false, false);
    let (mut cast, mut html, mut size) = (None, None, (100, 30));
    let mut rest = vec![];
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--print" => print = true,
            "--tv" => tv = true,
            "--cast" => cast = Some(PathBuf::from(it.next().ok_or("deque: --cast FILE: the file to write\n")?)),
            "--html" => html = Some(PathBuf::from(it.next().ok_or("deque: --html FILE: the page to write\n")?)),
            "--size" => size = parse_size(it.next())?,
            "--cursor" => cursor = Some(true),
            "--no-cursor" => cursor = Some(false),
            "--share" => sharing = true,
            "--calm" => calm = true,
            "--rehearse" => practice = true,
            "--share-curl" => (sharing, curl) = (true, true),
            "--share-no-font" => (sharing, own_font) = (true, false),
            "--share-local" => (sharing, local) = (true, true),
            _ if a.starts_with('-') => return Err(format!("deque: no option {a}\n\n{HELP}\n")),
            _ if path.is_none() => {
                path = Some(talk_at(a)?);
                // Given again to the new window as the talk's file.
                continue;
            }
            _ => start = a.parse().map_err(|_| format!("deque: \"{a}\" isn't a slide number\n"))?,
        }
        if a != "--tv" {
            rest.push(a.clone());
        }
    }
    // Left out, the one here.
    let path = path.or_else(|| found(Path::new("."))).ok_or(format!("{HELP}\n"))?;
    // Before the talk's read, so its slides can say where to watch.
    let share = match sharing && !tv && !print {
        true => {
            if !local {
                eprintln!("deque: opening a link, through Cloudflare…");
            }
            let sh = share::start(&markup::Theme::default(), curl, fine::face().filter(|_| own_font), !local)?;
            if let Some(why) = &sh.local {
                eprintln!("deque: sharing on this network only: {why}");
                std::thread::sleep(Duration::from_secs(2));
            }
            Some(sh)
        }
        false => None,
    };
    if let Some(sh) = &share {
        // SAFETY: nothing else runs yet to read the environment.
        unsafe {
            std::env::set_var("DEQUE_URL", &sh.url);
            std::env::set_var("DEQUE_WATCH", sh.curl.as_ref().unwrap_or(&sh.url));
        }
    }
    let mut talk = load(&path, false)?;
    talk.calm |= calm;
    if print {
        print!("{}", render::print_all(&talk, std::io::stdout().is_terminal()));
        return Ok(());
    }
    if let Some(file) = cast {
        return cast::record(&talk, &file, size.0, size.1);
    }
    if let Some(file) = html {
        cast::html(&talk, &file, size.0, size.1)?;
        println!("{} written: open it in a browser; → and ← step through the talk", file.display());
        return Ok(());
    }
    if tv {
        let abs = path.canonicalize().map_err(|e| e.to_string())?;
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
    // The terminal's own background, for a talk that doesn't say.
    let term_bg = images::background();
    adopt(&mut talk, term_bg, calm);
    s.theme = talk.theme.clone();
    s.calm = talk.calm;
    s.kitty = proto == images::Proto::Kitty;
    // Watchers can't be sent pictures of moving text; with them, it smears.
    if s.kitty && share.is_none() {
        s.font = fine::font();
    }
    if let Some(sh) = &share {
        sh.hub.theme(&talk.theme);
        s.tap = Some(sh.hub.clone());
    }
    let mut pics = images::Pictures::new(proto);
    let show = if cursor { "\x1b[?25h" } else { "\x1b[?25l" };
    // The wheel, on this screen, is sent as ↑ and ↓ unless alternate scroll
    // is off (1007); off it goes. The mouse is followed (1003, in SGR's
    // form, 1006) for the laser pointer, and its own arrow hidden (OSC 22)
    // where the terminal will.
    let enter = format!("\x1b[?1049h\x1b[?1007l\x1b[?1003h\x1b[?1006h\x1b]22;none\x1b\\{show}");
    s.raw(&enter);
    s.flush();
    let mut font = ghostty::Font::from_env();
    if let Some(f) = font.as_mut() {
        f.fit(&mut s);
    }

    let mut n = start.clamp(1, talk.slides.len()) - 1;
    let (mut mode, mut shown, mut started) = (Mode::Arrive, 0usize, false);
    let mut stamp = stamps(&path, &talk);
    let mut src = std::fs::read_to_string(&path).unwrap_or_default();
    let mut problem: Option<String> = None;
    let mut link = link::Link::new(&path);
    if let Some(sh) = &share {
        link.publish_remote(&sh.remote);
    }
    // A slide number being typed, to go to on enter.
    let mut jump = String::new();
    // The slide on the screen before this one, for the way out of it.
    let mut was = n;
    // What each slide's run block printed when it last ran, by slide.
    let mut ran: std::collections::HashMap<usize, run::Output> = Default::default();
    // Where ' goes: the slide before the last jump, or, to start with, the
    // slide the last run was on, offered in the corner.
    let mut leap = link.last().filter(|&k| k != n && k < talk.slides.len());
    let mut offer = leap.is_some();
    // A word in the bottom corner for a moment, and when it went up.
    let mut note: Option<(String, Instant)> = None;
    // The slide is frosted under "make me bigger".
    let mut small = false;
    // The screen's as it should be but for a note: only that's drawn.
    let mut hold = false;
    // Rehearsing: how long each slide's been up, and since when this one.
    let mut spent = vec![0.0f64; talk.slides.len()];
    let mut since = Instant::now();
    let mut on = n;
    loop {
        if n != on {
            if let Some(t) = spent.get_mut(on) {
                *t += since.elapsed().as_secs_f64();
            }
            (on, since) = (n, Instant::now());
        }
        s.hurry = false;
        // Its poll open to watchers, its bars as they've voted.
        if let Some(sh) = &share {
            let p = poll_of(&talk, n);
            if let Some(p) = &p {
                let votes = sh.hub.tally(p);
                talk.slides[n].tally(&votes, talk.theme.accent, talk.theme.muted);
            }
            sh.hub.poll(p);
        }
        let slide = &talk.slides[n];
        if hold {
            hold = false;
        } else {
        if small {
            s.thaw();
            small = false;
        }
        // Before the slide plays in, so the notes change with the key, not
        // after the animation.
        link.publish(n, shown);
        link.remember(n);
        if let Some(sh) = &share {
            sh.hub.state(state(&talk, n, shown, &s));
        }
        if mode == Mode::Arrive && started && render::leave(&mut s, &talk, was.min(talk.slides.len() - 1), n) {
            mode = Mode::Still;
        }
        started = true;
        render::draw(&mut s, &talk, &mut pics, n, mode, shown, font.is_some());
        was = n;
        if let Some(r) = &slide.run {
            if mode == Mode::Step && shown == r.step {
                ran.insert(n, run::go(&mut s, &talk, n));
            } else if let Some(o) = ran.get(&n).filter(|_| shown >= r.step) {
                run::show(&mut s, &talk, n, o, false);
            }
        }
        // Too big for the window: say so over it, frosted.
        if let Some(need) = render::needs(&talk, n, s.w, s.h) {
            render::small(&mut s, &talk, n, need);
            small = true;
        }
        if offer {
            let st = markup::Style::fg(talk.theme.muted);
            let t = format!("' back to slide {}", leap.unwrap_or(0) + 1);
            s.put_str(s.h, s.w - t.chars().count() as i32, &t, st);
        }
        if let Some(p) = &problem {
            let st = markup::Style::fg(talk.theme.bad);
            s.put_str(s.h, 1, p, st);
        }
        if !jump.is_empty() {
            let st = markup::Style::fg(talk.theme.accent);
            s.put_str(s.h, 2, &format!("go to {jump}_  "), st);
        }
        mode = Mode::Still;
        }
        if let Some((t, _)) = &note {
            let st = markup::Style::fg(talk.theme.muted);
            s.put_str(s.h, s.w - t.chars().count() as i32, t, st);
        }
        s.flush();
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
                if s.redraw || share.as_ref().is_some_and(|sh| sh.hub.joined()) {
                    break Act::Redraw;
                }
                // The note's moment over: gone, the rest left as it is.
                if let Some((t, at)) = &note
                    && at.elapsed() > Duration::from_millis(1600)
                {
                    let blank = " ".repeat(t.chars().count());
                    s.put_str(s.h, s.w - t.chars().count() as i32, &blank, markup::Style::default());
                    s.flush();
                    note = None;
                }
                if event::poll(Duration::from_millis(if s.sky.is_some() { 15 } else { 100 })).unwrap_or(false) {
                    match event::read() {
                        Ok(Event::Key(k)) if k.kind != KeyEventKind::Release => break act(k),
                        Ok(Event::Resize(..)) => s.resized = true,
                        Ok(Event::Mouse(m)) => s.mouse(m),
                        _ => {}
                    }
                    continue;
                }
                s.sky_frame();
                s.flush();
                s.steer();
                // Votes in: the poll's bars grow to them.
                if let Some(sh) = &share
                    && sh.hub.voted()
                    && let Some(p) = poll_of(&talk, n)
                {
                    let votes = sh.hub.tally(&p);
                    let before = talk.slides[n].tally(&votes, talk.theme.accent, talk.theme.muted);
                    if !small {
                        render::tallied(&mut s, &talk, n, &before);
                        s.flush();
                    }
                }
                if let Some(c) = link.take().or_else(|| share.as_ref().and_then(|sh| sh.hub.take())) {
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
                let now = stamps(&path, &talk);
                if now != stamp {
                    stamp = now;
                    let new = std::fs::read_to_string(&path).unwrap_or_default();
                    match load(&path, true) {
                        Ok(t) => {
                            // To the slide just edited, whole: the talk
                            // beside the editor, as a preview.
                            let edited = changed(&src, &talk, &new, &t);
                            talk = t;
                            adopt(&mut talk, term_bg, calm);
                            spent.resize(talk.slides.len(), 0.0);
                            stamp = stamps(&path, &talk);
                            src = new;
                            ran.clear();
                            if let Some(sh) = &share {
                                sh.hub.theme(&talk.theme);
                            }
                            s.theme = talk.theme.clone();
                            s.calm = talk.calm;
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
            Act::Live if slide.enter.is_none() && slide.play.is_none() => Act::Next,
            a => a,
        };
        // Anything but a quit: the first q forgotten.
        if !matches!(act, Act::Quit | Act::None) && note.as_ref().is_some_and(|(t, _)| t == QUIT) {
            note = None;
        }
        // The offer's gone at the first key, and drawn over.
        let offered = offer;
        if !matches!(act, Act::None | Act::Redraw) {
            offer = false;
        }
        let from = n;
        match act {
            Act::Exit => break,
            // One q could be a slip, mid-talk: it takes two.
            Act::Quit if note.as_ref().is_some_and(|(t, _)| t == QUIT) => break,
            Act::Quit => {
                note = Some((QUIT.into(), Instant::now()));
                hold = !offered;
            }
            Act::Help => render::help(&mut s, &talk, n),
            Act::Blank => render::blank(&mut s, &talk, n, shown),
            Act::Leap => {
                if let Some(k) = leap {
                    (n, shown, mode) = (k.min(last), 0, Mode::Arrive);
                }
            }
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
            Act::Live => {
                // The recording first, if there's one; from it, the real thing.
                let go = match &slide.play {
                    Some(p) => match replay::load(p) {
                        Ok(c) => matches!(replay::play(&mut s, &c, slide.enter.is_some()), replay::Then::Live),
                        Err(e) => {
                            problem = Some(e);
                            false
                        }
                    },
                    None => true,
                };
                if go {
                    live(&mut s, &talk, slide, font.as_ref(), &enter);
                } else {
                    s.raw(&enter);
                }
            }
            Act::Overview => {
                if let Some(k) = render::overview(&mut s, &talk, n) {
                    (n, shown, mode) = (k, 0, Mode::Arrive);
                }
            }
            Act::Watch => {
                if let Some(sh) = &share {
                    render::watch(&mut s, &talk, n, &sh.url, sh.curl.as_deref(), sh.local.as_deref());
                }
            }
            Act::Remote => {
                if let Some(sh) = &share {
                    render::remote(&mut s, &talk, n, &sh.remote);
                }
            }
            Act::Next if shown < steps => (shown, mode) = (shown + 1, Mode::Step),
            Act::Next if n < last => (n, shown, mode) = (n + 1, 0, render::arrive(&talk, n + 1)),
            Act::Next => {
                note = Some(("the end".into(), Instant::now()));
                hold = !offered;
            }
            _ => {}
        }
        // A jump, not a step: ' comes back.
        if n != from && !matches!(act, Act::Next | Act::Back) {
            leap = Some(from);
        }
    }
    link.gone();
    if let Some(sh) = &share {
        sh.hub.end();
    }
    restore();
    if practice {
        if let Some(t) = spent.get_mut(on) {
            *t += since.elapsed().as_secs_f64();
        }
        return practiced(&path, &talk, &spent);
    }
    Ok(())
}

/// A practice run over: each slide it was on for a second or more gets
/// the time it took as its `time:`, written into the talk.
fn practiced(path: &Path, talk: &Talk, spent: &[f64]) -> Result<(), String> {
    let times: Vec<Option<u32>> = spent.iter().map(|&t| (t >= 1.0).then(|| rehearse::round(t))).collect();
    let n = times.iter().flatten().count();
    if n == 0 {
        return Ok(());
    }
    let src = std::fs::read_to_string(path).map_err(|e| format!("deque: {}: {e}\n", path.display()))?;
    // Read again, as it is now, in case it was saved since.
    let now = load(path, true)?;
    if now.slides.len() != talk.slides.len() {
        return Err(format!("deque: {} changed its slides while you practiced; times not written\n", path.display()));
    }
    std::fs::write(path, rehearse::with_times(&src, &now, &times)).map_err(|e| format!("deque: {}: {e}\n", path.display()))?;
    let total: u32 = times.iter().flatten().sum();
    eprintln!("{n} slides timed, {} in all: written to {} as each slide's time:", rehearse::say(total), path.display());
    Ok(())
}

/// What the first q says.
const QUIT: &str = "q again to quit";

/// What a key asks for.
enum Act {
    None,
    Redraw,
    Quit,
    Exit,
    Help,
    Blank,
    Leap,
    Next,
    Back,
    Last,
    Goto(usize),
    Replay,
    Live,
    Overview,
    Watch,
    Remote,
    Grow(i32),
    Digit(char),
    Erase,
}

fn act(k: KeyEvent) -> Act {
    let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
    match k.code {
        KeyCode::Char('c') if ctrl => Act::Exit,
        KeyCode::Char('?') => Act::Help,
        KeyCode::Char('B' | '.') => Act::Blank,
        KeyCode::Char('\'') => Act::Leap,
        KeyCode::Char('q') | KeyCode::Esc => Act::Quit,
        KeyCode::Char('+' | '=') => Act::Grow(1),
        KeyCode::Char('-') => Act::Grow(-1),
        KeyCode::Left | KeyCode::PageUp | KeyCode::Char('b' | 'p' | 'h' | 'k') => Act::Back,
        KeyCode::Home | KeyCode::Char('g') => Act::Goto(0),
        KeyCode::End | KeyCode::Char('G') => Act::Last,
        KeyCode::Char('r') => Act::Replay,
        KeyCode::Char('o') | KeyCode::Tab => Act::Overview,
        KeyCode::Char('w') => Act::Watch,
        KeyCode::Char('P') => Act::Remote,
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
    s.raw("\x1b[?1003l\x1b[?1006l\x1b]22;default\x1b\\\x1b[?25h\x1b[0m\x1b[H\x1b[2J");
    s.flush();
    // Watchers watching: through a terminal of deque's own, so they see it
    // too.
    #[cfg(unix)]
    if let Some(hub) = s.tap.clone() {
        if let Err(e) = relay::run(cmd, &talk.dir, &hub) {
            eprint!("deque: {cmd}: {e}\r\n");
            std::thread::sleep(Duration::from_secs(2));
        }
        s.raw(enter);
        s.flush();
        if let Some(f) = font {
            f.set(s, f.size);
        }
        s.size();
        return;
    }
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

/// Where the talk is, for the phone remote: the slide, its step, its
/// title and notes, what's next, and the screen's shape.
fn state(talk: &Talk, n: usize, shown: usize, s: &Screen) -> String {
    let slide = &talk.slides[n];
    let next = talk.slides.get(n + 1).map(|x| x.title());
    serde_json::json!({
        "n": n + 1,
        "total": talk.slides.len(),
        "shown": shown,
        "steps": slide.steps(),
        "title": slide.title(),
        "notes": slide.notes,
        "next": if shown < slide.steps() { Some("next step".to_string()) } else { next },
        "w": s.w,
        "h": s.h,
    })
    .to_string()
}

/// Slide n's poll, as watchers are asked it: its headline, or what it's
/// called, and its choices.
fn poll_of(talk: &Talk, n: usize) -> Option<share::Poll> {
    let s = &talk.slides[n];
    s.poll.as_ref().map(|p| share::Poll { id: p.id.clone(), question: s.title(), choices: p.choices.clone() })
}

/// The terminal's background for the talk's, unless the talk set its own;
/// and calm, when asked for.
fn adopt(talk: &mut Talk, bg: Option<markup::Rgb>, calm: bool) {
    talk.calm |= calm;
    if let Some(c) = bg.filter(|_| !talk.theme.bg_given) {
        talk.theme.bg = c;
    }
}

fn restore() {
    use std::io::Write;
    let _ = terminal::disable_raw_mode();
    let mut o = std::io::stdout();
    let _ = o.write_all(b"\x1b[0m\x1b[?1003l\x1b[?1006l\x1b]22;default\x1b\\\x1b[?1007h\x1b[?1049l\x1b[?25h");
    let _ = o.flush();
}
