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
use std::net::{TcpListener, UdpSocket};
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
pub const XTERM: &str = "https://cdn.jsdelivr.net/npm/@xterm/xterm@5.5.0";
pub const XTERM_JS: &str = "sha384-M169f14mRZOXm3hD/v2Ti0ThIT/RnAQagXA9nlE15yHAtrW19gdePJh/HaTzUOe/";
pub const XTERM_CSS: &str = "sha384-8Xk9wy/gzEDUKrXtrmCFa2bBuK3BpjpDuL/p0SeKQX19Khl/M+lHOgD/CyYf7efP";

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
    /// The remote's own token: control, which watching doesn't give.
    remote: String,
    /// What the remote asked for, for the talk to do: next, back, …
    cmds: Mutex<Vec<String>>,
    /// Where the remote's finger is, as fractions of the screen, and
    /// whether it tapped.
    points: Mutex<Vec<(f64, f64, bool)>>,
    /// Where the talk is, as JSON, for the remote to show.
    state: Mutex<String>,
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

    /// A command from the remote, if one's waiting.
    pub fn take(&self) -> Option<String> {
        let mut c = self.0.cmds.lock().unwrap();
        (!c.is_empty()).then(|| c.remove(0))
    }

    /// Where the remote pointed, and tapped, since last asked.
    pub fn points(&self) -> Vec<(f64, f64, bool)> {
        std::mem::take(&mut *self.0.points.lock().unwrap())
    }

    /// Where the talk is, for the remote.
    pub fn state(&self, json: String) {
        *self.0.state.lock().unwrap() = json;
    }

    /// The end, said to everyone still watching.
    pub fn end(&self) {
        self.send(b"\x1b[0m\x1b[2J\x1b[H\x1b[?25hthe talk's over. thanks for watching.\r\n", 0, 0);
    }
}

pub struct Share {
    pub hub: Hub,
    pub url: String,
    /// The remote, for the presenter's phone alone.
    pub remote: String,
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

/// A certificate of its own for this run, for this address and localhost,
/// signed by nobody: browsers warn once, then the connection's encrypted.
fn tls(ip: &str) -> Result<Arc<rustls::ServerConfig>, String> {
    let fail = |e: &dyn std::fmt::Display| format!("deque: --share couldn't make a certificate: {e}\n");
    let mut params = rcgen::CertificateParams::new(vec![ip.to_string(), "localhost".into()]).map_err(|e| fail(&e))?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params.distinguished_name.push(rcgen::DnType::CommonName, "deque, this talk only");
    let signing = rcgen::KeyPair::generate().map_err(|e| fail(&e))?;
    let cert = params.self_signed(&signing).map_err(|e| fail(&e))?;
    let key = rustls::pki_types::PrivateKeyDer::Pkcs8(signing.serialize_der().into());
    let cfg = rustls::ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
        .with_safe_default_protocol_versions()
        .map_err(|e| fail(&e))?
        .with_no_client_auth()
        .with_single_cert(vec![cert.der().clone()], key)
        .map_err(|e| fail(&e))?;
    Ok(Arc::new(cfg))
}

/// Listening on the first free port from 7700; `curl`, to offer the
/// command to watch in a terminal; `face`, the font to send the page.
pub fn start(theme: &Theme, curl: bool, face: Option<std::path::PathBuf>) -> Result<Share, String> {
    let (l, port) = (7700..7720)
        .find_map(|p| TcpListener::bind(("0.0.0.0", p)).ok().map(|l| (l, p)))
        .ok_or("deque: --share found no free port from 7700 to 7719\n")?;
    let (token, remote) = (token(), token());
    let ip = here();
    let tls = tls(&ip)?;
    let hub = Hub(Arc::new(Inner {
        viewers: Mutex::new(vec![]),
        joined: AtomicBool::new(false),
        open: AtomicUsize::new(0),
        w: AtomicI32::new(80),
        h: AtomicI32::new(24),
        theme: Mutex::new(theme.clone()),
        said: Mutex::new((0, 0)),
        token: token.clone(),
        remote: remote.clone(),
        cmds: Mutex::new(vec![]),
        points: Mutex::new(vec![]),
        state: Mutex::new("{}".into()),
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
            let (h, tls) = (h.clone(), tls.clone());
            std::thread::spawn(move || {
                let _ = c.set_read_timeout(Some(PATIENCE));
                let _ = c.set_write_timeout(Some(PATIENCE));
                // A TLS handshake starts with 0x16; anything else is plain.
                let mut b = [0u8; 1];
                match c.peek(&mut b) {
                    Ok(1) if b[0] == 0x16 => {
                        if let Ok(conn) = rustls::ServerConnection::new(tls) {
                            serve(rustls::StreamOwned::new(conn, c), &h, true);
                        }
                    }
                    Ok(1) => serve(c, &h, false),
                    _ => {}
                }
                h.0.open.fetch_sub(1, Ordering::SeqCst);
            });
        }
    });
    let host = format!("{ip}:{port}");
    Ok(Share {
        hub,
        url: format!("https://{host}/{token}"),
        remote: format!("https://{host}/{remote}"),
        curl: curl.then(|| format!("curl -skN https://{host}/{token}")),
    })
}

/// One request. `/TOKEN` from a terminal (curl, wget), or `/TOKEN/tty`,
/// gets the talk as it's drawn; `/TOKEN` from a browser, the page, and
/// `/TOKEN/app.js` its script; anything else, 404.
/// A request's head, read from c: up to its blank line, and no more than
/// 8 KiB of it. None when the other end's gone quiet or sent too much.
fn request(c: &mut impl Read) -> Option<String> {
    let mut head = vec![];
    let mut b = [0u8; 1];
    // A byte at a time, so nothing of a next request is read with this one.
    while !head.ends_with(b"\r\n\r\n") {
        match c.read(&mut b) {
            Ok(1) if head.len() < 8192 => head.push(b[0]),
            _ => return None,
        }
    }
    Some(String::from_utf8_lossy(&head).to_lowercase())
}

/// One connection, TLS or not: requests to watch get their answer, and the
/// connection's closed; the remote's are answered on the same connection
/// till it goes quiet, and only over TLS.
fn serve(mut c: impl Read + Write, hub: &Hub, secure: bool) {
    loop {
        let Some(head) = request(&mut c) else { return };
        let mut first = head.split_whitespace();
        let (method, path) = (first.next().unwrap_or(""), first.next().unwrap_or("/"));
        let path = path.split('?').next().unwrap_or("");
        let mut parts = path.trim_start_matches('/').splitn(2, '/');
        let (key, rest) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        if same(key, &hub.0.remote) {
            // Control, over plain HTTP, would put the remote's token on the
            // network for anyone to read.
            if !secure || !control(&mut c, hub, method, rest) {
                let _ = c.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                return;
            }
            continue;
        }
        return watch(c, hub, &head, method, key, rest);
    }
}

/// A request to watch: the page, its script, its font, or the stream.
fn watch(mut c: impl Read + Write, hub: &Hub, head: &str, method: &str, key: &str, rest: &str) {
    if method != "get" || !same(key, &hub.0.token.to_lowercase()) {
        let _ = c.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }
    let terminal = head.lines().any(|l| l.starts_with("user-agent:") && (l.contains("curl") || l.contains("wget")));
    match rest {
        "tty" => stream(c, hub),
        "" if terminal => stream(c, hub),
        "" => {
            reply(&mut c, "text/html", &page(hub), false);
        }
        "app.js" => {
            reply(&mut c, "text/javascript", &app(hub), false);
        }
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

/// The remote: its page and script, where the talk is, and what it asks
/// for: a step (POST do/next, back, first, last, replay), or the pointer
/// (POST point/X/Y or tap/X/Y, fractions of the screen).
fn control(c: &mut impl Write, hub: &Hub, method: &str, rest: &str) -> bool {
    let ok = |c: &mut dyn Write| c.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n").is_ok();
    let parts: Vec<&str> = rest.split('/').collect();
    match (method, parts.as_slice()) {
        ("get", [""]) => reply(c, "text/html", &remote_page(&hub.0.remote), true),
        ("get", ["remote.js"]) => reply(c, "text/javascript", REMOTE_JS, true),
        ("get", ["state"]) => {
            let s = hub.0.state.lock().unwrap().clone();
            reply(c, "application/json", &s, true)
        }
        ("post", ["do", what]) if ["next", "back", "first", "last", "replay"].contains(what) => {
            let mut q = hub.0.cmds.lock().unwrap();
            if q.len() < 16 {
                q.push(what.to_string());
            }
            drop(q);
            ok(c)
        }
        ("post", [kind @ ("point" | "tap"), x, y]) => match (x.parse::<f64>(), y.parse::<f64>()) {
            (Ok(x), Ok(y)) if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) => {
                let mut q = hub.0.points.lock().unwrap();
                if q.len() >= 64 {
                    q.remove(0);
                }
                q.push((x, y, *kind == "tap"));
                drop(q);
                ok(c)
            }
            _ => false,
        },
        _ => false,
    }
}

/// The remote: where the talk is, the notes, next and back, and a pad
/// the screen's shape to point with.
fn remote_page(token: &str) -> String {
    r##"<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,maximum-scale=1,minimum-scale=1,user-scalable=no,viewport-fit=cover">
<meta name="apple-mobile-web-app-capable" content="yes">
<meta name="mobile-web-app-capable" content="yes">
<meta name="apple-mobile-web-app-status-bar-style" content="black-translucent">
<meta name="theme-color" content="#1d2021">
<title>deque · remote</title>
<style>
*{box-sizing:border-box;-webkit-tap-highlight-color:transparent;-webkit-touch-callout:none}
html,body{margin:0;height:100%;overflow:hidden;overscroll-behavior:none;background:#1d2021;color:#ebdbb2;font:16px ui-monospace,Menlo,monospace;-webkit-text-size-adjust:100%;text-size-adjust:100%;touch-action:none;user-select:none;-webkit-user-select:none}
body{position:fixed;inset:0}
#app{position:fixed;left:50%;top:50%;width:100vw;height:100vh;transform:translate(-50%,-50%);display:flex;flex-direction:column;gap:10px;padding:max(12px,env(safe-area-inset-top)) max(12px,env(safe-area-inset-right)) max(12px,env(safe-area-inset-bottom)) max(12px,env(safe-area-inset-left))}
#top{display:flex;justify-content:space-between;color:#928374}
#where.off{color:#fb4934}
#title{font-weight:bold;color:#fabd2f;font-size:20px}
#notes{flex:1;overflow:auto;white-space:pre-wrap;line-height:1.4;min-height:60px;touch-action:pan-y;overscroll-behavior:contain;-webkit-overflow-scrolling:touch}
#next{color:#928374}
#pad{position:relative;flex:none;width:100%;border:1px solid #504945;border-radius:10px;touch-action:none;background:#282828;overflow:hidden}
#pad.held{border-color:#fb4934;background:#2e2626}
#label{position:absolute;inset:0;display:flex;align-items:center;justify-content:center;color:#665c54;font-size:14px;text-align:center;padding:8px;pointer-events:none}
#dot{position:absolute;width:12px;height:12px;border-radius:6px;background:#fb4934;transform:translate(-6px,-6px);display:none;box-shadow:0 0 12px #fb4934;pointer-events:none}
#aimrow,#keys{display:flex;gap:10px}
button{flex:1;font:inherit;font-size:22px;padding:22px 0;border:0;border-radius:12px;background:#3c3836;color:#ebdbb2;touch-action:manipulation}
button:active{filter:brightness(1.3)}
#go{flex:2;background:#fabd2f;color:#1d2021;font-weight:bold}
.small{font-size:16px;padding:12px 0}
#aim.on{background:#fb4934;color:#1d2021}
#hint{color:#928374;font-size:13px;min-height:1em}
</style>
<div id="app">
<div id="top"><span id="where">…</span><span id="clock"></span></div>
<div id="title"></div>
<div id="notes"></div>
<div id="next"></div>
<div id="pad"><div id="label"></div><div id="dot"></div></div>
<div id="aimrow"><button id="aim" class="small">point with the phone</button><button id="speed" class="small" hidden></button></div>
<div id="hint"></div>
<div id="keys"><button id="back">back</button><button id="go">next</button></div>
</div>
<script src="/TOKEN/remote.js"></script>
"##
    .replace("TOKEN", token)
}

/// The remote's script. Its own address holds the token, so everything it
/// asks for is relative to that.
const REMOTE_JS: &str = r##"const base = location.pathname.replace(/\/$/, "") + "/";
const $ = id => document.getElementById(id);
const post = (p, keep) => fetch(base + p, {method: "POST", keepalive: !!keep}).catch(() => {});
const clamp = v => Math.min(1, Math.max(0, v));
const store = {
  get(k) { try { return localStorage.getItem(k); } catch (e) { return null; } },
  set(k, v) { try { localStorage.setItem(k, v); } catch (e) {} },
};

// Nothing zooms: not a pinch, not a double tap, not iOS's own gestures;
// nothing scrolls but the notes.
for (const g of ["gesturestart", "gesturechange", "gestureend"]) document.addEventListener(g, e => e.preventDefault(), {passive: false});
document.addEventListener("touchmove", e => {
  if (e.touches.length > 1 || !e.target.closest("#notes")) e.preventDefault();
}, {passive: false});
document.addEventListener("dblclick", e => e.preventDefault(), {passive: false});
let lastEnd = 0;
document.addEventListener("touchend", e => {
  // A second tap that quick is a zoom, to iOS, unless it's stopped; on the
  // buttons it's a second press, so it's pressed here.
  const now = Date.now();
  if (now - lastEnd < 350 && e.target.closest("button")) {
    e.preventDefault();
    e.target.closest("button").click();
  }
  lastEnd = now;
}, {passive: false});

// Upright whichever way the phone thinks it's turned: where it can, it's
// held portrait (full screen, locked); where it can't, the page turns
// back against it.
let turn = 0;
function angle() {
  const a = screen.orientation && typeof screen.orientation.angle === "number" ? screen.orientation.angle : (window.orientation || 0);
  return ((a % 360) + 360) % 360;
}
function orient() {
  const app = $("app"), a = angle();
  turn = innerWidth > innerHeight && (a === 90 || a === 270) ? a : 0;
  app.style.width = (turn ? innerHeight : innerWidth) + "px";
  app.style.height = (turn ? innerWidth : innerHeight) + "px";
  app.style.transform = `translate(-50%, -50%) rotate(${-turn}deg)`;
  size();
}
addEventListener("resize", orient);
addEventListener("orientationchange", () => setTimeout(orient, 50));
if (screen.orientation) screen.orientation.addEventListener("change", orient);

// Awake while it's open: a remote that sleeps mid-talk isn't one.
let lock = null;
async function awake() {
  try {
    if (navigator.wakeLock && document.visibilityState === "visible" && !lock) {
      lock = await navigator.wakeLock.request("screen");
      lock.addEventListener("release", () => (lock = null));
    }
  } catch (e) {}
}
document.addEventListener("visibilitychange", awake);
let first = true;
addEventListener("pointerdown", () => {
  awake();
  if (!first) return;
  first = false;
  const d = document.documentElement;
  if (d.requestFullscreen && matchMedia("(pointer: coarse)").matches) {
    d.requestFullscreen({navigationUI: "hide"}).then(() => screen.orientation && screen.orientation.lock && screen.orientation.lock("portrait")).catch(() => {});
  }
});
awake();

function buzz() { if (navigator.vibrate) navigator.vibrate(8); }
$("go").onclick = () => { buzz(); post("do/next", true); };
$("back").onclick = () => { buzz(); post("do/back", true); };

let started = Date.now(), aspect = 16 / 9, misses = 0;
async function poll() {
  try {
    const s = await (await fetch(base + "state", {cache: "no-store"})).json();
    misses = 0;
    $("where").classList.remove("off");
    $("where").textContent = s.n ? `${s.n} / ${s.total}` + (s.steps ? ` · step ${s.shown}/${s.steps}` : "") : "…";
    $("title").textContent = s.title || "";
    const notes = (s.notes || []).join("\n");
    if ($("notes").textContent !== notes) { $("notes").textContent = notes; $("notes").scrollTop = 0; }
    $("next").textContent = s.next ? "next · " + s.next : "the end";
    if (s.w && s.h && Math.abs(aspect - s.w / (s.h * 2)) > 0.01) { aspect = s.w / (s.h * 2); size(); }
  } catch (e) {
    if (++misses > 1) {
      $("where").classList.add("off");
      $("where").textContent = "reconnecting…";
    }
  }
  setTimeout(poll, 600);
}
function size() {
  const pad = $("pad");
  pad.style.height = Math.min(pad.clientWidth / aspect, $("app").clientHeight * 0.4) + "px";
}
setInterval(() => {
  const e = Math.floor((Date.now() - started) / 1000);
  $("clock").textContent = Math.floor(e / 60) + ":" + String(e % 60).padStart(2, "0");
}, 1000);

// Where the pointer is, as fractions of the screen, and sending it: as
// often as the last one's answered, at most 30 a second.
let pos = [0.5, 0.5], sending = false, lastSent = 0;
function dot() {
  const p = $("pad");
  $("dot").style.display = "block";
  $("dot").style.left = pos[0] * p.clientWidth + "px";
  $("dot").style.top = pos[1] * p.clientHeight + "px";
}
function point(force) {
  dot();
  const now = Date.now();
  if (!force && (sending || now - lastSent < 33)) return;
  lastSent = now;
  sending = true;
  post(`point/${pos[0].toFixed(4)}/${pos[1].toFixed(4)}`).finally(() => (sending = false));
}
function ring() { buzz(); post(`tap/${pos[0].toFixed(4)}/${pos[1].toFixed(4)}`); }

// A place on the pad, from a place on the page, the page turned or not.
function padAt(t) {
  const r = $("pad").getBoundingClientRect();
  const u = (t.clientX - r.left) / r.width, v = (t.clientY - r.top) / r.height;
  const [x, y] = turn === 90 ? [1 - v, u] : turn === 270 ? [v, 1 - u] : [u, v];
  return [clamp(x), clamp(y)];
}

// Two ways to point. By finger: the pad is the screen, the pointer where
// the finger is. By the phone, like a presenter's clicker: hold the pad
// and the pointer moves as the phone turns, from the middle, or from where
// it was if you only just let go; it never needs setting up, and if it's
// off, push it to the edge and it catches up, as a mouse does. A finger
// sliding meanwhile nudges it.
let motion = false, holding = false, heldAt = 0, lastUp = 0, travelled = 0, from = null;
const SPEEDS = [["slow", 50], ["medium", 32], ["fast", 20]];
let speed = Math.min(2, Math.max(0, parseInt(store.get("deque-speed") || "1", 10) || 0));
function label() {
  $("label").textContent = motion ? "hold here and point the phone · tap for a ring" : "drag to point · tap for a ring";
  $("speed").textContent = "speed: " + SPEEDS[speed][0];
  $("speed").hidden = !motion;
  $("aim").classList.toggle("on", motion);
  $("aim").textContent = motion ? "pointing with the phone" : "point with the phone";
}
$("speed").onclick = () => { speed = (speed + 1) % SPEEDS.length; store.set("deque-speed", speed); label(); };

const pad = $("pad");
pad.addEventListener("touchstart", e => {
  e.preventDefault();
  const t = e.touches[0];
  from = padAt(t);
  travelled = 0;
  heldAt = Date.now();
  if (motion) {
    holding = true;
    pad.classList.add("held");
    if (Date.now() - lastUp > 1500) pos = [0.5, 0.5];
  } else {
    pos = from;
  }
  point(true);
}, {passive: false});
pad.addEventListener("touchmove", e => {
  e.preventDefault();
  const at = padAt(e.touches[0]);
  if (motion) {
    pos = [clamp(pos[0] + at[0] - from[0]), clamp(pos[1] + at[1] - from[1])];
  } else {
    pos = at;
  }
  travelled += Math.abs(at[0] - from[0]) + Math.abs(at[1] - from[1]);
  from = at;
  point();
}, {passive: false});
function up(e) {
  e.preventDefault();
  if (holding) { holding = false; lastUp = Date.now(); pad.classList.remove("held"); }
  if (Date.now() - heldAt < 350 && travelled < 0.03) ring();
  else point(true);
}
pad.addEventListener("touchend", up, {passive: false});
pad.addEventListener("touchcancel", up, {passive: false});
// With a mouse, for trying it on a laptop.
pad.addEventListener("mousemove", e => { if (e.buttons && !motion) { pos = padAt(e); point(); } });
pad.addEventListener("click", e => { if (!motion) { pos = padAt(e); ring(); } });
// Held still, the pointer's said again, so it doesn't fade on the screen.
setInterval(() => { if (holding) point(true); }, 400);

// Which way is up, in the phone's own axes, from its tilt; and the phone
// turning, from its gyroscope: turning about up moves the pointer across;
// tipping what points at the screen (its top, or its back when it's held
// upright) moves it up and down. However it's held or rolled.
let upv = [0, 0, 1], lastT = 0;
const D = Math.PI / 180;
function tilt(e) {
  if (e.beta == null || e.gamma == null) return;
  const b = e.beta * D, g = e.gamma * D;
  upv = [-Math.cos(b) * Math.sin(g), Math.sin(b), Math.cos(b) * Math.cos(g)];
}
function spin(e) {
  const r = e.rotationRate;
  if (!r || r.alpha == null) return;
  const now = e.timeStamp || performance.now();
  const dt = lastT ? Math.min(0.1, (now - lastT) / 1000) : 0;
  lastT = now;
  if (!holding || !dt) return;
  const w = [r.beta, r.gamma, r.alpha];
  const dotp = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const p = Math.abs(upv[1]) < Math.abs(upv[2]) ? [0, 1, 0] : [0, 0, -1];
  let h = [p[1] * upv[2] - p[2] * upv[1], p[2] * upv[0] - p[0] * upv[2], p[0] * upv[1] - p[1] * upv[0]];
  const n = Math.hypot(h[0], h[1], h[2]) || 1;
  h = h.map(v => v / n);
  // A hand held still still shakes a little: that's left out.
  const calm = v => Math.abs(v) < 1.2 ? 0 : v - Math.sign(v) * 1.2;
  const yaw = calm(dotp(w, upv)), pitch = calm(dotp(w, h));
  const span = SPEEDS[speed][1];
  const dx = -yaw * dt / span, dy = -pitch * dt / span * aspect;
  if (!dx && !dy) return;
  pos = [clamp(pos[0] + dx), clamp(pos[1] + dy)];
  travelled += Math.abs(dx) + Math.abs(dy);
  point();
}

async function allowed() {
  // iPhones ask first, and only on a tap.
  for (const E of [window.DeviceMotionEvent, window.DeviceOrientationEvent]) {
    if (E && typeof E.requestPermission === "function") {
      try { if ((await E.requestPermission()) !== "granted") return false; } catch (e) { return false; }
    }
  }
  return true;
}
function start() {
  addEventListener("devicemotion", spin);
  addEventListener("deviceorientation", tilt);
  motion = true;
  store.set("deque-aim", "phone");
  $("hint").textContent = "";
  label();
}
$("aim").onclick = async () => {
  if (motion) {
    removeEventListener("devicemotion", spin);
    removeEventListener("deviceorientation", tilt);
    motion = false;
    store.set("deque-aim", "finger");
    label();
    return;
  }
  if (!window.isSecureContext) { $("hint").textContent = "pointing with the phone needs the https link"; return; }
  if (!window.DeviceMotionEvent) { $("hint").textContent = "this browser can't tell how the phone moves"; return; }
  if (!(await allowed())) { $("hint").textContent = "motion wasn't allowed: Settings › Safari › Motion & Orientation Access"; return; }
  start();
};
// Pointing with the phone last time: again, where no tap's needed to ask.
if (store.get("deque-aim") === "phone" && window.isSecureContext && window.DeviceMotionEvent) {
  if (typeof DeviceMotionEvent.requestPermission === "function") $("hint").textContent = "tap “point with the phone” to point with it again";
  else start();
}

label();
orient();
poll();
"##;

/// A page or script, told not to be framed, sniffed, or named to anyone
/// it fetches from: its address holds the token.
/// And kept open, `keep`, for the next request on it.
fn reply(c: &mut impl Write, kind: &str, body: &str, keep: bool) -> bool {
    let csp = "default-src 'none'; script-src 'self' https://cdn.jsdelivr.net; style-src 'unsafe-inline' https://cdn.jsdelivr.net; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'";
    let close = if keep { "" } else { "Connection: close\r\n" };
    write!(
        c,
        "HTTP/1.1 200 OK\r\nContent-Type: {kind}; charset=utf-8\r\nContent-Length: {}\r\nContent-Security-Policy: {csp}\r\nReferrer-Policy: no-referrer\r\nX-Content-Type-Options: nosniff\r\nCache-Control: no-store\r\n{close}\r\n{body}",
        body.len()
    )
    .and_then(|_| c.flush())
    .is_ok()
}

/// The talk as it's drawn, till the watcher goes, or falls too far behind.
fn stream(mut c: impl Write, hub: &Hub) {
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
    let token = &hub.0.token;
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
<script src="/{token}/app.js"></script>
"##
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
    use std::net::TcpStream;

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

    fn post(port: &str, path: &str) -> String {
        let mut c = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        write!(c, "POST {path} HTTP/1.1\r\nContent-Length: 0\r\n\r\n").unwrap();
        all(c)
    }

    /// A TLS client that takes deque's unsigned certificate, as a
    /// browser does once told to.
    #[derive(Debug)]
    struct Any;

    impl rustls::client::danger::ServerCertVerifier for Any {
        fn verify_server_cert(
            &self,
            _: &rustls::pki_types::CertificateDer,
            _: &[rustls::pki_types::CertificateDer],
            _: &rustls::pki_types::ServerName,
            _: &[u8],
            _: rustls::pki_types::UnixTime,
        ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        }
        fn verify_tls12_signature(
            &self,
            _: &[u8],
            _: &rustls::pki_types::CertificateDer,
            _: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }
        fn verify_tls13_signature(
            &self,
            _: &[u8],
            _: &rustls::pki_types::CertificateDer,
            _: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }
        fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
            rustls::crypto::ring::default_provider().signature_verification_algorithms.supported_schemes()
        }
    }

    /// Requests one after another over one TLS connection, and the status
    /// line of each answer.
    fn tls(port: &str, reqs: &[String]) -> Vec<String> {
        let cfg = rustls::ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .unwrap()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(Any))
            .with_no_client_auth();
        let name = rustls::pki_types::ServerName::try_from("localhost").unwrap();
        let conn = rustls::ClientConnection::new(Arc::new(cfg), name).unwrap();
        let mut c = rustls::StreamOwned::new(conn, TcpStream::connect(format!("127.0.0.1:{port}")).unwrap());
        reqs.iter()
            .map(|r| {
                c.write_all(r.as_bytes()).unwrap();
                // The head, then as much body as it says.
                let head = request(&mut c).unwrap_or_default();
                let len = head.lines().find_map(|l| l.strip_prefix("content-length: ")).and_then(|n| n.trim().parse().ok()).unwrap_or(0);
                let mut body = vec![0u8; len];
                c.read_exact(&mut body).unwrap();
                head.lines().next().unwrap_or("").to_string()
            })
            .collect()
    }

    #[test]
    fn only_the_remote_drives() {
        let share = start(&Theme::default(), false, None).unwrap();
        assert!(share.url.starts_with("https://") && share.remote.starts_with("https://"));
        let port = share.url.rsplit(':').next().unwrap().split('/').next().unwrap().to_string();
        let (watch, remote) = (share.url.rsplit('/').next().unwrap(), share.remote.rsplit('/').next().unwrap());
        assert_ne!(watch, remote);
        let post = |path: &str| format!("POST {path} HTTP/1.1\r\nContent-Length: 0\r\n\r\n");
        // Not over plain HTTP, where its token would be there to read.
        let mut c = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        c.write_all(post(&format!("/{remote}/do/next")).as_bytes()).unwrap();
        assert!(all(c).starts_with("HTTP/1.1 404"));
        assert_eq!(share.hub.take(), None);
        // Over TLS, several on one connection; the watch link drives nothing.
        let got = tls(
            &port,
            &[
                post(&format!("/{remote}/do/next")),
                post(&format!("/{remote}/point/0.5/0.25")),
                post(&format!("/{remote}/do/back")),
            ],
        );
        assert_eq!(got, ["http/1.1 204 no content"; 3]);
        assert_eq!(tls(&port, &[post(&format!("/{watch}/do/next"))]), ["http/1.1 404 not found"]);
        assert_eq!(share.hub.take().as_deref(), Some("next"));
        assert_eq!(share.hub.take().as_deref(), Some("back"));
        assert_eq!(share.hub.points(), [(0.5, 0.25, false)]);
        assert_eq!(tls(&port, &[post(&format!("/{remote}/tap/2/0.25"))]), ["http/1.1 404 not found"]);
        // Watching works either way.
        assert!(all(get(&port, &format!("/{watch}"), "Mozilla")).starts_with("HTTP/1.1 200"));
        let page = format!("GET /{watch} HTTP/1.1\r\n\r\n");
        assert_eq!(tls(&port, &[page]), ["http/1.1 200 ok"]);
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
