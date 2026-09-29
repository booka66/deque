//! deque TALK --share: the talk, live, for anyone on the network who can't
//! see the screen. What deque draws goes to each watcher as it's drawn:
//! `curl -sN` plays it in their terminal, and a browser gets a page that
//! plays it the same way. Someone joining has the slide drawn again, whole,
//! for them.
//!
//! What goes out is only what drawing takes: text, cursor moves, colors,
//! erasing, and the cursor hidden or shown. Anything else deque writes,
//! switching screens, following the mouse, pictures, questions for the
//! terminal, never reaches a watcher, so a terminal piping the stream in
//! is left as it was found.
//!
//! Watching is all it does, and only for those given the link: every path
//! carries a random token, anything else is a 404. Nothing a watcher sends
//! is read past the request line, echoed, or kept. Each watcher can hold
//! only so much unsent before it's let go, a request must come quickly, and
//! only so many are served at once, so a slow or hostile one can't take
//! memory or threads from the talk. It's plain HTTP: anyone on the network
//! between can read what's shown.

use crate::markup::{Rgb, Theme};
use std::hash::{BuildHasher, Hasher};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicUsize, Ordering};
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// At most this many connections at once, watching or asking.
const MOST: usize = 32;
/// Frames a watcher may fall behind by before it's let go.
const BEHIND: usize = 256;
/// How long a request may take to arrive, and a write to go out.
const PATIENCE: Duration = Duration::from_secs(5);

/// xterm.js, pinned to the version and the bytes: a browser won't run it
/// if the CDN hands over anything else.
const XTERM: &str = "https://cdn.jsdelivr.net/npm/@xterm/xterm@5.5.0";
const XTERM_JS: &str = "sha384-M169f14mRZOXm3hD/v2Ti0ThIT/RnAQagXA9nlE15yHAtrW19gdePJh/HaTzUOe/";
const XTERM_CSS: &str = "sha384-8Xk9wy/gzEDUKrXtrmCFa2bBuK3BpjpDuL/p0SeKQX19Khl/M+lHOgD/CyYf7efP";

/// Where watchers get what's drawn.
#[derive(Clone)]
pub struct Hub(Arc<Inner>);

struct Inner {
    viewers: Mutex<Vec<SyncSender<Arc<[u8]>>>>,
    /// Someone new is watching: the slide wants drawing again, whole.
    joined: AtomicBool,
    /// Connections open now.
    open: AtomicUsize,
    /// The screen's size, for the page to size its terminal to.
    w: AtomicI32,
    h: AtomicI32,
    theme: Mutex<Theme>,
    /// The size watchers were last told, to tell them again when it changes.
    said: Mutex<(i32, i32)>,
    token: String,
    /// The terminal's font, for the page to draw in, and its type.
    font: Option<(Vec<u8>, &'static str)>,
}

impl Hub {
    /// What was just drawn, to everyone watching, as drawing needs it;
    /// those gone, or too far behind, let go.
    pub fn send(&self, b: &[u8], w: i32, h: i32) {
        if w > 0 {
            self.0.w.store(w, Ordering::Relaxed);
            self.0.h.store(h, Ordering::Relaxed);
        }
        let mut b = drawing(b);
        // A new size, for the page to resize to: an OSC of deque's own,
        // which a terminal that doesn't know it ignores.
        let mut said = self.0.said.lock().unwrap();
        if w > 0 && *said != (w, h) {
            *said = (w, h);
            let mut size = format!("\x1b]7741;{w};{h}\x07").into_bytes();
            size.extend_from_slice(&b);
            b = size;
        }
        drop(said);
        if b.is_empty() {
            return;
        }
        let b: Arc<[u8]> = b.into();
        self.0.viewers.lock().unwrap().retain(|v| v.try_send(b.clone()).is_ok());
    }

    /// Whether anyone's joined since last asked.
    pub fn joined(&self) -> bool {
        self.0.joined.swap(false, Ordering::Relaxed)
    }

    pub fn theme(&self, t: &Theme) {
        *self.0.theme.lock().unwrap() = t.clone();
    }

    /// The end, said to everyone still watching.
    pub fn end(&self) {
        self.send(b"\x1b[0m\x1b[2J\x1b[H\x1b[?25hthe talk's over. thanks for watching.\r\n", 0, 0);
    }
}

pub struct Share {
    pub hub: Hub,
    pub url: String,
    /// The command to watch it in a terminal, when that's offered: over
    /// plain HTTP, whoever's on the network between can write to a
    /// terminal that pipes the stream in, so it's for networks you trust.
    pub curl: Option<String>,
}

/// This machine's address on the network: the one a packet out would come
/// from. Nothing is sent to find it.
fn here() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|s| {
            s.connect("192.0.2.1:80")?;
            s.local_addr()
        })
        .map(|a| a.ip().to_string())
        .unwrap_or("localhost".into())
}

/// 128 random bits, as hex, from the system's randomness; on Windows, two
/// hashes under the keys the standard library draws from it.
fn token() -> String {
    let mut b = [0u8; 16];
    if std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut b)).is_ok() {
        return b.iter().map(|x| format!("{x:02x}")).collect();
    }
    let word = || {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u8(0);
        h.finish()
    };
    format!("{:016x}{:016x}", word(), word())
}

/// Whether two strings are the same, taking as long either way, so how
/// much of a guess was right doesn't show.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |d, (x, y)| d | (x ^ y)) == 0
}

/// A font file the page can use: its bytes and type, for the kinds
/// browsers take on their own (not a collection, .ttc).
fn load(path: &std::path::Path) -> Option<(Vec<u8>, &'static str)> {
    let kind = match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "otf" => "font/otf",
        "ttf" => "font/ttf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => return None,
    };
    Some((std::fs::read(path).ok()?, kind))
}

/// Listening on the first free port from 7700; `curl`, to offer the
/// command to watch in a terminal; `face`, the font to send the page.
pub fn start(theme: &Theme, curl: bool, face: Option<std::path::PathBuf>) -> Result<Share, String> {
    let (l, port) = (7700..7720)
        .find_map(|p| TcpListener::bind(("0.0.0.0", p)).ok().map(|l| (l, p)))
        .ok_or("deque: --share found no free port from 7700 to 7719\n")?;
    let token = token();
    let hub = Hub(Arc::new(Inner {
        viewers: Mutex::new(vec![]),
        joined: AtomicBool::new(false),
        open: AtomicUsize::new(0),
        w: AtomicI32::new(80),
        h: AtomicI32::new(24),
        theme: Mutex::new(theme.clone()),
        said: Mutex::new((0, 0)),
        token: token.clone(),
        font: face.as_deref().and_then(load),
    }));
    let h = hub.clone();
    std::thread::spawn(move || {
        for c in l.incoming().flatten() {
            // Past the most at once, closed straight away.
            if h.0.open.fetch_add(1, Ordering::SeqCst) >= MOST {
                h.0.open.fetch_sub(1, Ordering::SeqCst);
                continue;
            }
            let h = h.clone();
            std::thread::spawn(move || {
                serve(c, &h);
                h.0.open.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });
    let host = format!("{}:{port}", here());
    Ok(Share { hub, url: format!("http://{host}/{token}"), curl: curl.then(|| format!("curl -sN {host}/{token}")) })
}

/// One request. `/TOKEN` from a terminal (curl, wget), or `/TOKEN/tty`,
/// gets the talk as it's drawn; `/TOKEN` from a browser, the page, and
/// `/TOKEN/app.js` its script; anything else, 404.
fn serve(mut c: TcpStream, hub: &Hub) {
    let _ = c.set_read_timeout(Some(PATIENCE));
    let _ = c.set_write_timeout(Some(PATIENCE));
    let mut head = vec![];
    let mut b = [0u8; 1024];
    while !head.windows(4).any(|w| w == b"\r\n\r\n") {
        match c.read(&mut b) {
            Ok(n) if n > 0 && head.len() + n <= 8192 => head.extend_from_slice(&b[..n]),
            _ => return,
        }
    }
    let head = String::from_utf8_lossy(&head).to_lowercase();
    let mut first = head.split_whitespace();
    let (method, path) = (first.next().unwrap_or(""), first.next().unwrap_or("/"));
    let path = path.split('?').next().unwrap_or("");
    let mut parts = path.trim_start_matches('/').splitn(2, '/');
    let (key, rest) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    if method != "get" || !same(key, &hub.0.token.to_lowercase()) {
        let _ = c.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }
    let terminal = head.lines().any(|l| l.starts_with("user-agent:") && (l.contains("curl") || l.contains("wget")));
    match rest {
        "tty" => stream(c, hub),
        "" if terminal => stream(c, hub),
        "" => reply(&mut c, "text/html", &page(hub)),
        "app.js" => reply(&mut c, "text/javascript", &app(hub)),
        "font" if hub.0.font.is_some() => {
            let (bytes, kind) = hub.0.font.as_ref().unwrap();
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
                bytes.len()
            );
            let _ = c.write_all(head.as_bytes()).and_then(|_| c.write_all(bytes));
        }
        _ => {
            let _ = c.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    }
}

/// A page or script, told not to be framed, sniffed, or named to anyone
/// it fetches from: its address holds the token.
fn reply(c: &mut TcpStream, kind: &str, body: &str) {
    let csp = "default-src 'none'; script-src 'self' https://cdn.jsdelivr.net; style-src 'unsafe-inline' https://cdn.jsdelivr.net; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'";
    let _ = write!(
        c,
        "HTTP/1.1 200 OK\r\nContent-Type: {kind}; charset=utf-8\r\nContent-Length: {}\r\nContent-Security-Policy: {csp}\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
}

/// The talk as it's drawn, till the watcher goes, or falls too far behind.
fn stream(mut c: TcpStream, hub: &Hub) {
    let (tx, rx) = mpsc::sync_channel(BEHIND);
    let ok = "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n";
    if c.write_all(ok.as_bytes()).and_then(|_| c.write_all(b"\x1b[0m\x1b[?25l\x1b[2J\x1b[H")).is_err() {
        return;
    }
    hub.0.viewers.lock().unwrap().push(tx);
    hub.0.joined.store(true, Ordering::Relaxed);
    for b in rx {
        if c.write_all(&b).and_then(|_| c.flush()).is_err() {
            return;
        }
    }
}

/// Of what deque (or a live command) wrote, only what draws: printable
/// text, returns, newlines, tabs and backspaces, and of the escapes, those
/// that move the cursor, erase, insert, delete, scroll, color, and save or
/// restore the cursor; wrapping, the cursor hidden or shown, and
/// synchronized frames. Any other escape, anything that would switch the
/// watcher's screen, follow their mouse, set their clipboard or title, or
/// ask their terminal something, and any other control character, is
/// dropped whole.
fn drawing(b: &[u8]) -> Vec<u8> {
    scan(b).0
}

/// How much of b is whole: all but an escape cut off at its end, to wait
/// for the rest of.
#[cfg_attr(not(unix), allow(dead_code))]
pub fn whole(b: &[u8]) -> usize {
    scan(b).1
}

fn scan(b: &[u8]) -> (Vec<u8>, usize) {
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            0x1b => {
                let Some(&kind) = b.get(i + 1) else { return (out, i) };
                match kind {
                    b'[' => {
                        // Parameters, then one final byte.
                        let mut j = i + 2;
                        while j < b.len() && (0x20..0x40).contains(&b[j]) {
                            j += 1;
                        }
                        let Some(&fin) = b.get(j) else { return (out, i) };
                        let params = &b[i + 2..j];
                        let private = params.first() == Some(&b'?');
                        let plain = params.iter().all(|c| c.is_ascii_digit() || *c == b';');
                        let keep = match fin {
                            b'A' | b'B' | b'C' | b'D' | b'E' | b'F' | b'G' | b'H' | b'J' | b'K' | b'L' | b'M' | b'P' | b'S' | b'T'
                            | b'X' | b'Z' | b'd' | b'f' | b'm' | b'r' | b'@' | b'`' | b'b' | b's' | b'u' => plain,
                            b'h' | b'l' if private => matches!(&params[1..], b"25" | b"2026" | b"7" | b"6"),
                            b'h' | b'l' => params == b"4",
                            _ => false,
                        };
                        if keep {
                            out.extend_from_slice(&b[i..=j]);
                        }
                        i = j + 1;
                    }
                    // Strings (OSC, APC, DCS, PM, SOS): to BEL or ST, dropped.
                    b']' | b'_' | b'P' | b'^' | b'X' => {
                        let mut j = i + 2;
                        while j < b.len() && b[j] != 0x07 && !(b[j] == 0x1b && b.get(j + 1) == Some(&b'\\')) {
                            j += 1;
                        }
                        if j >= b.len() || (b[j] == 0x1b && j + 1 >= b.len()) {
                            return (out, i);
                        }
                        i = if b[j] == 0x07 { j + 1 } else { j + 2 };
                    }
                    // Line-drawing character sets, for boxes.
                    b'(' | b')' => {
                        let Some(&set) = b.get(i + 2) else { return (out, i) };
                        if matches!(set, b'B' | b'0' | b'A') {
                            out.extend_from_slice(&b[i..i + 3]);
                        }
                        i += 3;
                    }
                    // Save and restore the cursor, index, reverse index,
                    // next line.
                    b'7' | b'8' | b'M' | b'D' | b'E' => {
                        out.extend_from_slice(&b[i..i + 2]);
                        i += 2;
                    }
                    _ => i += 2,
                }
            }
            b'\r' | b'\n' | b'\t' | 0x08 => {
                out.push(b[i]);
                i += 1;
            }
            c if c < 0x20 || c == 0x7f => i += 1,
            _ => {
                out.push(b[i]);
                i += 1;
            }
        }
    }
    (out, b.len())
}

fn hex(c: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
}

/// A terminal in the page, xterm.js checked against its hash, in the
/// presenter's font when it's been sent.
fn page(hub: &Hub) -> String {
    let bg = hex(hub.0.theme.lock().unwrap().bg);
    let face = match hub.0.font {
        Some(_) => format!(r#"@font-face{{font-family:"deque";src:url("/{}/font")}}"#, hub.0.token),
        None => String::new(),
    };
    format!(
        r##"<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>deque · live</title>
<link rel="stylesheet" href="{XTERM}/css/xterm.css" integrity="{XTERM_CSS}" crossorigin="anonymous">
<style>
html,body{{margin:0;height:100%;background:{bg};overflow:hidden}}
body{{display:flex;align-items:center;justify-content:center}}
#t{{transform-origin:center}}
{face}
</style>
<div id="t"></div>
<script src="{XTERM}/lib/xterm.js" integrity="{XTERM_JS}" crossorigin="anonymous"></script>
<script src="{token}/app.js"></script>
"##,
        token = format!("/{}", hub.0.token)
    )
}

/// The page's script: the terminal the screen's size, scaled to fit the
/// window, playing the talk. Only numbers and colors deque made go in.
fn app(hub: &Hub) -> String {
    let (w, h) = (hub.0.w.load(Ordering::Relaxed), hub.0.h.load(Ordering::Relaxed));
    let t = hub.0.theme.lock().unwrap().clone();
    let (bg, fg, token) = (hex(t.bg), hex(t.fg), &hub.0.token);
    let font = hub.0.font.is_some();
    format!(
        r##"(async () => {{
// The presenter's font, loaded before the terminal measures its cells.
if ({font}) {{
  try {{ await document.fonts.load('16px "deque"'); }} catch (e) {{}}
}}
const term = new Terminal({{cols: {w}, rows: {h}, fontSize: 16, fontFamily: {font} ? '"deque", monospace' : "monospace", scrollback: 0, disableStdin: true, cursorBlink: false, theme: {{background: "{bg}", foreground: "{fg}"}}}});
term.open(document.getElementById("t"));
// The presenter's screen changed size.
term.parser.registerOscHandler(7741, d => {{
  const [c, r] = d.split(";").map(Number);
  if (c > 0 && r > 0) {{ term.resize(c, r); setTimeout(fit, 0); }}
  return true;
}});
function fit() {{
  const e = document.querySelector("#t .xterm-screen");
  if (!e) return;
  const k = Math.min(innerWidth / e.offsetWidth, innerHeight / e.offsetHeight) * 0.98;
  document.getElementById("t").style.transform = "scale(" + k + ")";
}}
addEventListener("resize", fit);
setTimeout(fit, 100);
(async () => {{
  for (;;) {{
    try {{
      const r = await fetch("/{token}/tty", {{cache: "no-store"}});
      const read = r.body.getReader();
      for (;;) {{
        const {{value, done}} = await read.read();
        if (done) break;
        term.write(value);
      }}
    }} catch (e) {{}}
    await new Promise(r => setTimeout(r, 1000));
  }}
}})();
}})();
"##
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(port: &str, path: &str, agent: &str) -> TcpStream {
        let mut c = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        write!(c, "GET {path} HTTP/1.1\r\nUser-Agent: {agent}\r\n\r\n").unwrap();
        c
    }

    fn all(mut c: TcpStream) -> String {
        let mut s = String::new();
        let _ = c.read_to_string(&mut s);
        s
    }

    #[test]
    fn only_the_token_gets_in() {
        let dir = std::env::temp_dir().join(format!("deque-face-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let face = dir.join("Mono-Regular.otf");
        std::fs::write(&face, b"OTTO font bytes").unwrap();
        let share = start(&Theme::default(), false, Some(face)).unwrap();
        let (port, token) = {
            let rest = share.url.rsplit("//").next().unwrap();
            let (host, token) = rest.split_once('/').unwrap();
            (host.rsplit(':').next().unwrap().to_string(), token.to_string())
        };
        assert_eq!(token.len(), 32);
        assert!(all(get(&port, "/", "curl/8")).starts_with("HTTP/1.1 404"));
        assert!(all(get(&port, "/nope/tty", "curl/8")).starts_with("HTTP/1.1 404"));
        let wrong: String = token.chars().rev().collect();
        assert!(all(get(&port, &format!("/{wrong}"), "curl/8")).starts_with("HTTP/1.1 404"));
        let page = all(get(&port, &format!("/{token}"), "Mozilla/5.0"));
        assert!(page.contains("Referrer-Policy: no-referrer") && page.contains(XTERM_JS));
        assert!(page.contains(&format!("/{token}/font")) && page.contains("font-src 'self'"));
        let font = all(get(&port, &format!("/{token}/font"), "Mozilla/5.0"));
        assert!(font.contains("Content-Type: font/otf") && font.ends_with("OTTO font bytes"));
        assert!(all(get(&port, "/font", "Mozilla/5.0")).starts_with("HTTP/1.1 404"));
        std::fs::remove_dir_all(dir).unwrap();

        // A watcher with the token gets what's drawn.
        let mut c = get(&port, &format!("/{token}"), "curl/8");
        while !share.hub.joined() {
            std::thread::sleep(Duration::from_millis(5));
        }
        share.hub.send(b"hello", 80, 24);
        let (mut got, mut s) = (vec![0u8; 256], String::new());
        while !s.contains("hello") {
            let n = c.read(&mut got).unwrap();
            s.push_str(&String::from_utf8_lossy(&got[..n]));
        }
    }

    #[test]
    fn a_watcher_too_far_behind_is_let_go() {
        let share = start(&Theme::default(), false, None).unwrap();
        let (tx, _rx) = mpsc::sync_channel(BEHIND);
        share.hub.0.viewers.lock().unwrap().push(tx);
        for _ in 0..=BEHIND {
            share.hub.send(b"x", 80, 24);
        }
        assert!(share.hub.0.viewers.lock().unwrap().is_empty());
    }

    #[test]
    fn only_drawing_goes_out() {
        let kept = b"\x1b[0m\x1b[3;4H\x1b[38;2;1;2;3mhi\x1b[2K\x1b[2J\x1b[?25l\x1b[?2026h\r\n";
        assert_eq!(drawing(kept), kept.to_vec());
        // Screens, mouse, pointer shape, clipboard, title, pictures,
        // queries, bells: gone, the text either side kept.
        let bad = b"a\x1b[?1049hb\x1b[?1003h\x1b]22;none\x1b\\c\x1b]52;c;ZXZpbA==\x07d\x1b]0;title\x07e\x1b_Ga=T;AAAA\x1b\\f\x1b[6n\x1b[cg\x07h\x1bPq#0\x1b\\i";
        assert_eq!(drawing(bad), b"abcdefghi".to_vec());
        // A cursor move with anything odd in it is dropped too.
        assert_eq!(drawing(b"\x1b[?5H\x1b[1:2mx"), b"x".to_vec());
        // An escape cut off at the end goes, not half of it, and whole
        // says where it starts, to wait for the rest.
        assert_eq!(drawing(b"ok\x1b[38;2"), b"ok".to_vec());
        assert_eq!(whole(b"ok\x1b[38;2"), 2);
        assert_eq!(whole(b"ok\x1b]0;tit"), 2);
        assert_eq!(whole(b"ok\x1b[2J"), 6);
        // What a TUI draws with: scrolling, inserting, boxes, the cursor saved.
        let tui = b"\x1b[2;20r\x1b[3L\x1b[2M\x1b[5X\x1b7\x1b(0qqq\x1b(B\x1b8\x1b[?7l\x08\t";
        assert_eq!(drawing(tui), tui.to_vec());
        // What it mustn't: keypad and cursor-key modes, bracketed paste,
        // focus reports, a full reset, window moves, cursor shape.
        let no = b"\x1b=\x1b[?1h\x1b[?2004h\x1b[?1004h\x1bc\x1b[3;1;1t\x1b[2 q";
        assert!(drawing(no).is_empty());
    }

    #[test]
    fn tokens_differ_and_compare_whole() {
        assert_ne!(token(), token());
        assert!(same("abc", "abc") && !same("abc", "abd") && !same("abc", "ab"));
    }
}
