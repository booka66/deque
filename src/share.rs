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
//! Watching, reacting and voting are all it does, and only for those given
//! the link: every path carries a random token, anything else is a 404.
//! What a watcher sends back is a number: which of a few emoji they
//! reacted with, or which of the poll's choices they picked (and a random
//! name their browser made, so a second vote replaces the first). No text
//! of theirs reaches the screen. Nothing else a watcher sends is read past
//! the request line, echoed, or kept. Each watcher can hold
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
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

/// At most this many connections at once, watching or asking.
const MOST: usize = 128;
/// Frames a watcher may fall behind by before it's let go.
const BEHIND: usize = 256;
/// How long a request may take to arrive, and a write to go out.
const PATIENCE: Duration = Duration::from_secs(5);
/// What's drawn is kept, this many frames and bytes of it at most, for
/// pages to ask for what they haven't had.
const KEPT: usize = 512;
const KEPT_BYTES: usize = 8 << 20;
/// How long a page's ask for more waits for something to be drawn.
const WAIT: Duration = Duration::from_secs(20);
/// Reactions waiting for the screen, at most; past that, they're dropped.
const REACTING: usize = 32;
/// Voters a poll keeps, at most.
const VOTERS: usize = 2000;

/// What watchers can react with.
pub const REACTIONS: [char; 6] = ['👏', '🔥', '😂', '🤯', '🎉', '👀'];

/// The poll on the screen: its name, its question, and its choices.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Poll {
    pub id: String,
    pub question: String,
    pub choices: Vec<String>,
}

/// xterm.js, pinned to the version and the bytes: a browser won't run it
/// if the CDN hands over anything else.
pub const XTERM: &str = "https://cdn.jsdelivr.net/npm/@xterm/xterm@5.5.0";
pub const XTERM_JS: &str = "sha384-M169f14mRZOXm3hD/v2Ti0ThIT/RnAQagXA9nlE15yHAtrW19gdePJh/HaTzUOe/";
pub const XTERM_CSS: &str = "sha384-8Xk9wy/gzEDUKrXtrmCFa2bBuK3BpjpDuL/p0SeKQX19Khl/M+lHOgD/CyYf7efP";

/// Where watchers get what's drawn.
#[derive(Clone)]
pub struct Hub(Arc<Inner>);

/// What the remote did where it pointed: moved there, tapped for a ring,
/// or let go, the pointer out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Touch {
    Point,
    Tap,
    Lift,
}

struct Inner {
    viewers: Mutex<Vec<SyncSender<Arc<[u8]>>>>,
    /// What's been drawn lately, for pages: each frame numbered, the
    /// number the next will have, and a wake for those waiting on it.
    log: Mutex<Log>,
    more: Condvar,
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
    /// Where the remote's pointing, as fractions of the screen, and what
    /// it did there.
    points: Mutex<Vec<(f64, f64, Touch)>>,
    /// Where the talk is, as JSON, for the remote to show.
    state: Mutex<String>,
    /// The terminal's font, for the page to draw in, and its type.
    font: Option<(Vec<u8>, &'static str)>,
    /// Reactions come in, by their place in REACTIONS, for the screen.
    reactions: Mutex<Vec<usize>>,
    /// The poll open now, if any.
    poll: Mutex<Option<Poll>>,
    /// Each poll's votes, by its name: each voter's choice.
    votes: Mutex<std::collections::HashMap<String, std::collections::HashMap<String, usize>>>,
    /// A vote's come in since the screen last asked.
    voted: AtomicBool,
    /// This machine's address on the network, and whether the links go
    /// through a tunnel: then pages ask to connect to that address
    /// straight, if they can.
    ip: std::net::IpAddr,
    public: bool,
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
        let mut log = self.0.log.lock().unwrap();
        log.bytes += b.len();
        log.frames.push_back(b);
        log.next += 1;
        while log.frames.len() > KEPT || log.bytes > KEPT_BYTES {
            let f = log.frames.pop_front().unwrap();
            log.bytes -= f.len();
        }
        drop(log);
        self.0.more.notify_all();
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
    pub fn points(&self) -> Vec<(f64, f64, Touch)> {
        std::mem::take(&mut *self.0.points.lock().unwrap())
    }

    /// Where the talk is, for the remote.
    pub fn state(&self, json: String) {
        *self.0.state.lock().unwrap() = json;
    }

    /// A new watcher: what's drawn from here on, the slide drawn again,
    /// whole, to start them off.
    pub fn viewer(&self) -> mpsc::Receiver<Arc<[u8]>> {
        let (tx, rx) = mpsc::sync_channel(BEHIND);
        self.0.viewers.lock().unwrap().push(tx);
        self.0.joined.store(true, Ordering::Relaxed);
        rx
    }

    /// Reactions since last asked, by their place in REACTIONS.
    pub fn reactions(&self) -> Vec<usize> {
        std::mem::take(&mut *self.0.reactions.lock().unwrap())
    }

    /// The poll on the screen now, or none.
    pub fn poll(&self, p: Option<Poll>) {
        *self.0.poll.lock().unwrap() = p;
    }

    /// How many voted for each of the poll's choices.
    pub fn tally(&self, p: &Poll) -> Vec<usize> {
        let mut n = vec![0; p.choices.len()];
        if let Some(v) = self.0.votes.lock().unwrap().get(&p.id) {
            for &k in v.values() {
                if let Some(c) = n.get_mut(k) {
                    *c += 1;
                }
            }
        }
        n
    }

    /// Whether anyone's voted since last asked.
    pub fn voted(&self) -> bool {
        self.0.voted.swap(false, Ordering::Relaxed)
    }

    /// The end, said to everyone still watching.
    pub fn end(&self) {
        self.send(b"\x1b[0m\x1b[2J\x1b[H\x1b[?25hthe talk's over. thanks for watching.\r\n", 0, 0);
    }
}

/// The frames lately drawn, the last of them numbered `next - 1`.
#[derive(Default)]
struct Log {
    frames: std::collections::VecDeque<Arc<[u8]>>,
    bytes: usize,
    next: u64,
}

impl Log {
    fn first(&self) -> u64 {
        self.next - self.frames.len() as u64
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
    /// The tunnel the links go through, closed with the share.
    tunnel: Option<std::process::Child>,
    /// Why the links are for this network only, when a public one was
    /// asked for and couldn't be had.
    pub local: Option<String>,
}

impl Drop for Share {
    fn drop(&mut self) {
        if let Some(t) = self.tunnel.as_mut() {
            let _ = t.kill();
            let _ = t.wait();
        }
    }
}

/// A public address for the port, or why there isn't one, by a Cloudflare
/// quick tunnel: an HTTPS
/// link with a certificate browsers trust, reachable from anywhere, going
/// to deque's own HTTPS here. cloudflared says the address on its errors;
/// what else it says is read and let go, so it never blocks.
fn tunnel(port: u16) -> Result<(std::process::Child, String), String> {
    use std::io::BufRead;
    use std::process::{Command, Stdio};
    let mut child = Command::new("cloudflared")
        .args(["tunnel", "--no-autoupdate", "--no-tls-verify", "--url", &format!("https://localhost:{port}")])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| "no cloudflared to open one with (brew install cloudflared)".to_string())?;
    let err = child.stderr.take().unwrap();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        for l in std::io::BufReader::new(err).lines().map_while(Result::ok) {
            if let Some(url) = l.split_whitespace().find(|w| w.starts_with("https://") && w.ends_with(".trycloudflare.com")) {
                let _ = tx.send(url.to_string());
            }
        }
    });
    match rx.recv_timeout(Duration::from_secs(30)) {
        Ok(url) => {
            // Not given out till the world can find it: a phone that looks
            // it up too soon is told it doesn't exist, and believes it for
            // a minute.
            let host = url.trim_start_matches("https://");
            let t = std::time::Instant::now();
            while !known(host) && t.elapsed() < Duration::from_secs(30) {
                std::thread::sleep(Duration::from_millis(500));
            }
            Ok((child, url))
        }
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            Err("cloudflared gave no link in 30 seconds: is this machine online?".into())
        }
    }
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

/// Whether trycloudflare.com's own nameservers have an address for host
/// yet. Asked of them straight, by UDP: a resolver asked too soon (this
/// machine's, or a public one) would remember the no, for everyone.
fn known(host: &str) -> bool {
    use std::net::ToSocketAddrs;
    let mut q = vec![0xde, 0xc0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    for label in host.split('.') {
        q.push(label.len() as u8);
        q.extend_from_slice(label.as_bytes());
    }
    q.extend_from_slice(&[0, 0, 1, 0, 1]);
    let Ok(s) = UdpSocket::bind("0.0.0.0:0") else { return false };
    let _ = s.set_read_timeout(Some(Duration::from_secs(1)));
    let mut a = [0u8; 512];
    // Its id back, no error, and an answer.
    ["kevin.ns.cloudflare.com:53", "marjory.ns.cloudflare.com:53"].iter().filter_map(|ns| ns.to_socket_addrs().ok()?.find(|a| a.is_ipv4())).any(|ns| {
        s.send_to(&q, ns).is_ok() && s.recv(&mut a).is_ok_and(|n| n >= 12 && a[..2] == q[..2] && a[3] & 0xf == 0 && u16::from_be_bytes([a[6], a[7]]) > 0)
    })
}

/// Listening on the first free port from 7700; `curl`, to offer the
/// command to watch in a terminal; `face`, the font to send the page;
/// `public`, the links through a tunnel, for anywhere, not only this
/// network, if one will open.
pub fn start(theme: &Theme, curl: bool, face: Option<std::path::PathBuf>, public: bool) -> Result<Share, String> {
    let (l, port) = (7700..7720)
        .find_map(|p| TcpListener::bind(("0.0.0.0", p)).ok().map(|l| (l, p)))
        .ok_or("deque: --share found no free port from 7700 to 7719\n")?;
    let (token, remote) = (token(), token());
    let ip = here();
    let tls = tls(&ip)?;
    let (tunnel, local) = match public.then(|| tunnel(port)) {
        Some(Ok((child, base))) => (Some((child, base)), None),
        Some(Err(why)) => (None, Some(why)),
        None => (None, None),
    };
    let public = tunnel.is_some();
    let hub = Hub(Arc::new(Inner {
        viewers: Mutex::new(vec![]),
        log: Mutex::new(Log::default()),
        more: Condvar::new(),
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
        reactions: Mutex::new(vec![]),
        poll: Mutex::new(None),
        votes: Mutex::new(Default::default()),
        voted: AtomicBool::new(false),
        ip: ip.parse().unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        public,
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
    // Watching in a terminal is on this network whatever: the tunnel holds
    // a stream back till it ends.
    let curl = curl.then(|| format!("curl -skN https://{host}/{token}"));
    let base = tunnel.as_ref().map_or(format!("https://{host}"), |t| t.1.clone());
    Ok(Share { hub, url: format!("{base}/{token}"), remote: format!("{base}/{remote}"), curl, tunnel: tunnel.map(|t| t.0), local })
}

/// One request. `/TOKEN` from a terminal (curl, wget), or `/TOKEN/tty`,
/// gets the talk as it's drawn; `/TOKEN` from a browser, the page, and
/// `/TOKEN/app.js` its script; anything else, 404.
/// A request's head, read from c: up to its blank line, and no more than
/// 8 KiB of it, as sent. None when the other end's gone quiet or sent too
/// much.
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
    Some(String::from_utf8_lossy(&head).into_owned())
}

/// One connection, TLS or not: requests to watch get their answer, and the
/// connection's closed; the remote's are answered on the same connection
/// till it goes quiet, and only over TLS.
fn serve(mut c: impl Read + Write, hub: &Hub, secure: bool) {
    loop {
        let Some(sent) = request(&mut c) else { return };
        let head = sent.to_lowercase();
        let mut first = head.split_whitespace();
        let (method, path) = (first.next().unwrap_or(""), first.next().unwrap_or("/"));
        let path = path.split('?').next().unwrap_or("");
        let mut parts = path.trim_start_matches('/').splitn(2, '/');
        let (key, rest) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        if same(key, &hub.0.remote) {
            // Control, over plain HTTP, would put the remote's token on the
            // network for anyone to read.
            if !secure || !control(&mut c, hub, &head, method, rest) {
                let _ = c.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                return;
            }
            continue;
        }
        if rest == "ws" && method == "get" && same(key, &hub.0.token) {
            return socket(c, hub, &sent);
        }
        return watch(c, hub, &head, method, key, rest);
    }
}

/// The talk as it's drawn, over a WebSocket, for the page: a frame for
/// each piece drawn, sent as it's drawn, which a proxy between (a
/// tunnel's) passes straight on, as it won't a stream. Nothing the page
/// sends on it is read; a ping every so often keeps it open through
/// proxies that close a quiet one.
fn socket(mut c: impl Read + Write, hub: &Hub, sent: &str) {
    let Some(key) = sent.lines().find_map(|l| l.split_once(':').filter(|(k, _)| k.eq_ignore_ascii_case("sec-websocket-key")).map(|(_, v)| v.trim())) else {
        let _ = c.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    };
    use base64::Engine;
    let digest = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, format!("{key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11").as_bytes());
    let accept = base64::engine::general_purpose::STANDARD.encode(digest.as_ref());
    let head = format!("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {accept}\r\n\r\n");
    // A frame from the server: final, binary (or a ping), unmasked.
    let frame = |c: &mut dyn Write, op: u8, b: &[u8]| {
        let mut f = vec![0x80 | op];
        match b.len() {
            n if n < 126 => f.push(n as u8),
            n if n < 65536 => {
                f.push(126);
                f.extend_from_slice(&(n as u16).to_be_bytes());
            }
            n => {
                f.push(127);
                f.extend_from_slice(&(n as u64).to_be_bytes());
            }
        }
        f.extend_from_slice(b);
        c.write_all(&f).and_then(|_| c.flush())
    };
    if c.write_all(head.as_bytes()).and_then(|_| frame(&mut c, 2, b"\x1b[0m\x1b[?25l\x1b[2J\x1b[H")).is_err() {
        return;
    }
    let rx = hub.viewer();
    loop {
        let sent = match rx.recv_timeout(Duration::from_secs(20)) {
            Ok(b) => frame(&mut c, 2, &b),
            Err(mpsc::RecvTimeoutError::Timeout) => frame(&mut c, 9, b""),
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };
        if sent.is_err() {
            return;
        }
    }
}

/// A request to watch: the page, its script, its font, or the stream; or
/// to react, or vote.
fn watch(mut c: impl Read + Write, hub: &Hub, head: &str, method: &str, key: &str, rest: &str) {
    let no = |c: &mut dyn Write| {
        let _ = c.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
    };
    if !same(key, &hub.0.token.to_lowercase()) {
        return no(&mut c);
    }
    // A page's offer to connect straight: the answer, and a port for it.
    if method == "post" && rest == "rtc" && hub.0.public {
        let Some(offer) = body(&mut c, head) else { return no(&mut c) };
        match crate::rtc::answer(hub, &offer, hub.0.ip, crate::rtc::Side::Watch) {
            Ok(a) => {
                reply(&mut c, "application/json", &a, false);
            }
            Err(_) => no(&mut c),
        }
        return;
    }
    if method == "post" {
        if !answer(hub, rest) {
            return no(&mut c);
        }
        let _ = c.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        return;
    }
    if method != "get" {
        return no(&mut c);
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
        // What's been drawn since frame N, for the page: at once if there's
        // any, or as soon as there is. `new`, or N gone from what's kept, and
        // the slide's drawn again, whole, to start from.
        _ if rest.starts_with("frames/") => {
            let (next, reset, body) = frames(hub, &rest["frames/".len()..]);
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nX-Next: {next}\r\n{}Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
                body.len(),
                if reset { "X-Reset: 1\r\n" } else { "" }
            );
            let _ = c.write_all(head.as_bytes()).and_then(|_| c.write_all(&body));
        }
        "poll" => {
            let p = hub.0.poll.lock().unwrap().clone();
            let json = match p {
                Some(p) => serde_json::json!({"id": p.id, "question": p.question, "choices": p.choices}),
                None => serde_json::json!({}),
            };
            reply(&mut c, "application/json", &json.to_string(), false);
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

/// Frames from `after` on, waiting a while for one if there are none yet:
/// the number to ask from next, whether the page's to start again (it
/// asked for `new`, or for frames no longer kept), and the frames.
fn frames(hub: &Hub, after: &str) -> (u64, bool, Vec<u8>) {
    let log = hub.0.log.lock().unwrap();
    let from = match after.parse::<u64>() {
        Ok(n) if n >= log.first() && n <= log.next => n,
        _ => {
            hub.0.joined.store(true, Ordering::Relaxed);
            return (log.next, true, vec![]);
        }
    };
    let (log, _) = hub.0.more.wait_timeout_while(log, WAIT, |l| l.next == from).unwrap();
    // Gone from what's kept while it waited: start again.
    if from < log.first() {
        hub.0.joined.store(true, Ordering::Relaxed);
        return (log.next, true, vec![]);
    }
    let skip = (from - log.first()) as usize;
    (log.next, false, log.frames.iter().skip(skip).flat_map(|f| f.iter().copied()).collect())
}

/// A watcher's reaction (react/K, K its place in REACTIONS), or vote
/// (vote/VOTER/K, for the poll open now, VOTER the random name their
/// browser made): whether it was one.
fn answer(hub: &Hub, rest: &str) -> bool {
    let parts: Vec<&str> = rest.split('/').collect();
    match parts.as_slice() {
        ["react", k] => match k.parse::<usize>() {
            Ok(k) if k < REACTIONS.len() => {
                let mut q = hub.0.reactions.lock().unwrap();
                if q.len() < REACTING {
                    q.push(k);
                }
                true
            }
            _ => false,
        },
        ["vote", who, k] if (8..=32).contains(&who.len()) && who.bytes().all(|b| b.is_ascii_hexdigit()) => {
            let Some(p) = hub.0.poll.lock().unwrap().clone() else { return false };
            let Ok(k) = k.parse::<usize>() else { return false };
            if k >= p.choices.len() {
                return false;
            }
            let mut all = hub.0.votes.lock().unwrap();
            let v = all.entry(p.id).or_default();
            if v.len() < VOTERS || v.contains_key(*who) {
                v.insert(who.to_string(), k);
                hub.0.voted.store(true, Ordering::Relaxed);
            }
            true
        }
        _ => false,
    }
}

/// The remote: its page and script, where the talk is, and what it asks
/// for: a step (POST do/next, back, first, last, replay), or the pointer
/// (POST point/X/Y or tap/X/Y, fractions of the screen).
fn control(c: &mut (impl Read + Write), hub: &Hub, head: &str, method: &str, rest: &str) -> bool {
    let parts: Vec<&str> = rest.split('/').collect();
    match (method, parts.as_slice()) {
        ("get", [""]) => reply(c, "text/html", &remote_page(&hub.0.remote), true),
        ("get", ["remote.js"]) => reply(c, "text/javascript", REMOTE_JS, true),
        ("get", ["state"]) => {
            let s = hub.0.state.lock().unwrap().clone();
            reply(c, "application/json", &s, true)
        }
        // The phone asking to connect straight, to point with no wait.
        ("post", ["rtc"]) => match body(c, head).filter(|_| hub.0.public) {
            Some(offer) => match crate::rtc::answer(hub, &offer, hub.0.ip, crate::rtc::Side::Remote) {
                Ok(a) => reply(c, "application/json", &a, true),
                Err(_) => false,
            },
            None => false,
        },
        ("post", _) => act(hub, rest) && c.write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n").is_ok(),
        _ => false,
    }
}

/// A request's body, as much as its head says, 64 KiB at most.
fn body(c: &mut impl Read, head: &str) -> Option<String> {
    let len = head.lines().find_map(|l| l.strip_prefix("content-length:")).and_then(|n| n.trim().parse::<usize>().ok()).filter(|&n| n <= 65536)?;
    let mut b = vec![0; len];
    c.read_exact(&mut b).ok()?;
    Some(String::from_utf8_lossy(&b).into_owned())
}

/// What the remote asks for, however it came: a step (do/next, back,
/// first, last, replay), or the pointer (point/X/Y or tap/X/Y, fractions
/// of the screen, or lift). Whether it was one.
pub fn act(hub: &Hub, what: &str) -> bool {
    let parts: Vec<&str> = what.split('/').collect();
    let point = |p: (f64, f64, Touch)| {
        let mut q = hub.0.points.lock().unwrap();
        if q.len() >= 64 {
            q.remove(0);
        }
        q.push(p);
    };
    match parts.as_slice() {
        ["do", what] if ["next", "back", "first", "last", "replay"].contains(what) => {
            let mut q = hub.0.cmds.lock().unwrap();
            if q.len() < 16 {
                q.push(what.to_string());
            }
            true
        }
        ["lift"] => {
            point((0.0, 0.0, Touch::Lift));
            true
        }
        [kind @ ("point" | "tap"), x, y] => match (x.parse::<f64>(), y.parse::<f64>()) {
            (Ok(x), Ok(y)) if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) => {
                point((x, y, if *kind == "tap" { Touch::Tap } else { Touch::Point }));
                true
            }
            _ => false,
        },
        _ => false,
    }
}

/// The remote: where the talk is, the notes, next and back, and a
/// button to hold to point.
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
#laser{flex:none;height:min(34vh,260px);width:100%;border:2px solid #fb4934;border-radius:18px;touch-action:none;background:#2a2020;color:#fb4934;display:flex;flex-direction:column;align-items:center;justify-content:center;gap:6px;font-size:24px;font-weight:bold;transition:background .08s,box-shadow .08s}
#laser small{font-size:13px;font-weight:normal;color:#928374}
#laser.on{background:#fb4934;color:#1d2021;box-shadow:0 0 40px #fb4934aa}
#laser.on small{color:#1d2021}
#keys{display:flex;gap:10px}
#speed{flex:none;align-self:flex-end;font-size:13px;padding:6px 12px;border-radius:8px;background:none;color:#928374;border:1px solid #504945}
button{flex:1;font:inherit;font-size:22px;padding:22px 0;border:0;border-radius:12px;background:#3c3836;color:#ebdbb2;touch-action:manipulation}
button:active{filter:brightness(1.3)}
#go{flex:2;background:#fabd2f;color:#1d2021;font-weight:bold}
.small{font-size:16px;padding:12px 0}
#hint{color:#928374;font-size:13px;min-height:1em}
</style>
<div id="app">
<div id="top"><span id="where">…</span><span id="clock"></span></div>
<div id="title"></div>
<div id="notes"></div>
<div id="next"></div>
<div id="hint"></div>
<button id="speed"></button>
<div id="laser">hold to point<small>aim the phone at the screen</small></div>
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
// Straight to the presenter's machine, where the phone can reach it (the
// same network, through a public link): what it asks for goes at once,
// with no wait for an answer. Elsewhere, or till then, a request each.
let dc = null;
const fast = () => dc && dc.readyState === "open";
(async () => {
  if (!window.RTCPeerConnection) return;
  try {
    const pc = new RTCPeerConnection();
    const ch = pc.createDataChannel("remote");
    ch.onopen = () => { dc = ch; };
    ch.onclose = () => { dc = null; pc.close(); };
    await pc.setLocalDescription(await pc.createOffer());
    const r = await fetch(base + "rtc", {method: "POST", body: JSON.stringify(pc.localDescription)});
    if (!r.ok) throw 0;
    await pc.setRemoteDescription(await r.json());
  } catch (e) {}
})();
const post = (p, keep) => {
  if (fast()) { dc.send(p); return Promise.resolve(); }
  return fetch(base + p, {method: "POST", keepalive: !!keep}).catch(() => {});
};
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
    if (s.w && s.h) aspect = s.w / (s.h * 2);
  } catch (e) {
    if (++misses > 1) {
      $("where").classList.add("off");
      $("where").textContent = "reconnecting…";
    }
  }
  setTimeout(poll, 600);
}
setInterval(() => {
  const e = Math.floor((Date.now() - started) / 1000);
  $("clock").textContent = Math.floor(e / 60) + ":" + String(e % 60).padStart(2, "0");
}, 1000);

// The laser: held, the pointer's on the screen and moves as the phone
// turns, from the middle every time; let go, it's out. A quick press is a ring. Nothing to set up and nothing
// to drift: it goes by how the phone turns, not where it points, so if
// it's off, turn past the edge and it catches up, as a mouse does.
let pos = [0.5, 0.5], sending = false, lastSent = 0;
let holding = false, heldAt = 0, lastUp = 0, travelled = 0, ready = false;
function point(force) {
  const now = Date.now();
  // A request at a time, each waiting on the last; straight, as often as
  // the screen's drawn.
  if (!force && (fast() ? now - lastSent < 16 : sending || now - lastSent < 33)) return;
  lastSent = now;
  sending = true;
  post(`point/${pos[0].toFixed(4)}/${pos[1].toFixed(4)}`).finally(() => (sending = false));
}
const SPEEDS = [["slow", 50], ["medium", 32], ["fast", 20]];
let speed = Math.min(2, Math.max(0, parseInt(store.get("deque-speed") || "1", 10) || 0));
function label() { $("speed").textContent = "speed: " + SPEEDS[speed][0]; }
$("speed").onclick = () => { speed = (speed + 1) % SPEEDS.length; store.set("deque-speed", speed); label(); };

const laser = $("laser");
function press(e) {
  e.preventDefault();
  if (!ready || holding) return;
  holding = true;
  heldAt = Date.now();
  travelled = 0;
  pos = [0.5, 0.5];
  was = null;
  laser.classList.add("on");
  buzz();
  point(true);
}
function release(e) {
  e.preventDefault();
  if (!ready) { enable(); return; }
  if (!holding) return;
  holding = false;
  lastUp = Date.now();
  laser.classList.remove("on");
  if (lastUp - heldAt < 250 && travelled < 0.02) post(`tap/${pos[0].toFixed(4)}/${pos[1].toFixed(4)}`);
  post("lift", true);
}
laser.addEventListener("touchstart", press, {passive: false});
laser.addEventListener("touchend", release, {passive: false});
laser.addEventListener("touchcancel", release, {passive: false});
laser.addEventListener("mousedown", press);
addEventListener("mouseup", e => { if (holding) release(e); });
// Leaving the page, or the phone locking, lets go.
document.addEventListener("visibilitychange", () => { if (document.hidden && holding) release(new Event("x")); });
addEventListener("blur", () => { if (holding) release(new Event("x")); });
// Held still, the pointer's said again, so it stays lit on the screen.
setInterval(() => { if (holding) point(true); }, 400);

// Where the phone points, from which way it's turned (its orientation,
// which every phone reports the same way): its top, or its back when it's
// held upright, whichever's nearer level, as a heading and an elevation.
// Held, the pointer moves as those change; a roll of the wrist changes
// neither.
const D = Math.PI / 180;
let was = null, axis = null, seen = 0, rate = 0;
const debug = location.hash === "#debug";
function aimed(e) {
  const [a, b, g] = [e.alpha * D, e.beta * D, e.gamma * D];
  const [sa, ca, sb, cb, sg, cg] = [Math.sin(a), Math.cos(a), Math.sin(b), Math.cos(b), Math.sin(g), Math.cos(g)];
  const top = [-cb * sa, ca * cb, sb];
  const back = [-(cg * sa * sb + ca * sg), -(sa * sg - ca * cg * sb), -cb * cg];
  // Kept to one till the other's clearly nearer level, so it doesn't flip.
  let use = axis || "top";
  if (Math.abs(top[2]) + 0.15 < Math.abs(back[2])) use = "top";
  else if (Math.abs(back[2]) + 0.15 < Math.abs(top[2])) use = "back";
  const p = use === "top" ? top : back;
  return {use, heading: Math.atan2(p[0], p[1]) / D, up: Math.asin(Math.max(-1, Math.min(1, p[2]))) / D, steep: Math.abs(p[2]) > 0.95};
}
function tilt(e) {
  if (e.alpha == null || e.beta == null || e.gamma == null) return;
  seen++;
  const now = aimed(e);
  const last = was;
  was = now;
  const turned = axis !== now.use;
  axis = now.use;
  if (debug) show(now);
  if (!holding || !last || turned) return;
  let dh = now.heading - last.heading;
  dh = ((dh + 540) % 360) - 180;
  if (now.steep || last.steep) dh = 0;
  const du = now.up - last.up;
  const span = SPEEDS[speed][1];
  const dx = dh / span, dy = -du / span * aspect;
  if (!dx && !dy) return;
  pos = [clamp(pos[0] + dx), clamp(pos[1] + dy)];
  travelled += Math.abs(dx) + Math.abs(dy);
  point();
}
// With #debug on the link: what the phone says, to see what's wrong.
setInterval(() => { rate = seen; seen = 0; }, 1000);
function show(n) {
  $("hint").textContent = `${n.use} · heading ${n.heading.toFixed(1)} · up ${n.up.toFixed(1)} · ${rate}/s · ${holding ? "held" : "free"} · ${pos[0].toFixed(2)},${pos[1].toFixed(2)}`;
}

// Held, but the phone's said nothing of how it's turned: say so.
setInterval(() => {
  if (holding && ready && !rate && !debug) $("hint").textContent = "no motion from the phone: is Motion & Orientation Access on?";
}, 1000);

// The motion sensors: iPhones ask first, and only on a tap, so the first
// tap of the button asks; everywhere else they're just on.
function start() {
  addEventListener("deviceorientation", tilt);
  ready = true;
  $("hint").textContent = "";
}
function asks() {
  return [window.DeviceMotionEvent, window.DeviceOrientationEvent].some(E => E && typeof E.requestPermission === "function");
}
async function enable() {
  if (!window.isSecureContext) { $("hint").textContent = "pointing needs the https link"; return; }
  for (const E of [window.DeviceMotionEvent, window.DeviceOrientationEvent]) {
    if (E && typeof E.requestPermission === "function") {
      try {
        if ((await E.requestPermission()) !== "granted") throw 0;
      } catch (e) {
        $("hint").textContent = "motion wasn't allowed: Settings › Apps › Safari › Motion & Orientation Access, then reload";
        return;
      }
    }
  }
  start();
  $("hint").textContent = "ready: hold the red button and aim";
  setTimeout(() => { if ($("hint").textContent.startsWith("ready")) $("hint").textContent = ""; }, 3000);
}
if (!window.isSecureContext) $("hint").textContent = "pointing needs the https link";
else if (!window.DeviceOrientationEvent) $("hint").textContent = "this browser can't tell how the phone moves";
else if (asks()) $("hint").textContent = "tap the red button once to let it use the phone's motion";
else start();

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
    let ok = "HTTP/1.1 200 OK\r\nContent-Type: text/plain; charset=utf-8\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n";
    if c.write_all(ok.as_bytes()).and_then(|_| c.write_all(b"\x1b[0m\x1b[?25l\x1b[2J\x1b[H")).is_err() {
        return;
    }
    for b in hub.viewer() {
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
    let t = hub.0.theme.lock().unwrap().clone();
    let (bg, fg, accent, muted) = (hex(t.bg), hex(t.fg), hex(t.accent), hex(t.muted));
    let token = &hub.0.token;
    let face = match hub.0.font {
        Some(_) => format!(r#"@font-face{{font-family:"deque";src:url("/{}/font")}}"#, hub.0.token),
        None => String::new(),
    };
    format!(
        r##"<!doctype html>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover">
<title>deque · live</title>
<link rel="stylesheet" href="{XTERM}/css/xterm.css" integrity="{XTERM_CSS}" crossorigin="anonymous">
<style>
html,body{{margin:0;height:100%;background:{bg};overflow:hidden}}
body{{display:flex;flex-direction:column;font:16px ui-monospace,Menlo,monospace;color:{fg}}}
#stage{{flex:1;min-height:0;display:flex;align-items:center;justify-content:center;overflow:hidden}}
#t{{transform-origin:center}}
#poll{{display:none;flex-direction:column;gap:8px;padding:10px 12px 0;max-width:640px;width:100%;margin:0 auto;box-sizing:border-box}}
#poll.on{{display:flex}}
#poll b{{color:{accent}}}
#poll button{{text-align:left;font:inherit;padding:12px 14px;border-radius:10px;border:1px solid {muted};background:none;color:inherit;cursor:pointer}}
#poll button.mine{{border-color:{accent};background:{accent};color:{bg};font-weight:bold}}
#react{{display:flex;justify-content:center;gap:6px;padding:10px 8px calc(10px + env(safe-area-inset-bottom))}}
#react button{{font-size:26px;line-height:1;padding:8px 10px;border-radius:12px;border:0;background:none;cursor:pointer;touch-action:manipulation;transition:transform .08s}}
#react button:active{{transform:scale(1.35)}}
.pop{{position:fixed;font-size:30px;pointer-events:none;animation:up 1.2s ease-out forwards}}
@keyframes up{{to{{transform:translateY(-120px);opacity:0}}}}
{face}
</style>
<div id="stage"><div id="t"></div></div>
<div id="poll"></div>
<div id="react"></div>
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
    let public = hub.0.public;
    let reactions = serde_json::to_string(&REACTIONS.map(String::from)).unwrap();
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
  const e = document.querySelector("#t .xterm-screen"), stage = document.getElementById("stage");
  if (!e) return;
  const k = Math.min(stage.clientWidth / e.offsetWidth, stage.clientHeight / e.offsetHeight) * 0.98;
  document.getElementById("t").style.transform = "scale(" + k + ")";
}}
addEventListener("resize", fit);
setTimeout(fit, 100);

// Reacting: a tap sends which, and it pops up here too.
const post = p => fetch("/{token}/" + p, {{method: "POST"}}).catch(() => {{}});
{reactions}.forEach((r, k) => {{
  const b = document.createElement("button");
  b.textContent = r;
  b.onclick = () => {{
    post("react/" + k);
    const box = b.getBoundingClientRect(), p = document.createElement("span");
    p.className = "pop";
    p.textContent = r;
    p.style.left = box.left + box.width / 2 - 15 + (Math.random() - 0.5) * 20 + "px";
    p.style.top = box.top - 10 + "px";
    document.body.appendChild(p);
    setTimeout(() => p.remove(), 1300);
  }};
  document.getElementById("react").appendChild(b);
}});

// Voting: this browser's own random name, so voting again changes the
// vote rather than adding one; the choice it made, for each poll.
const who = (() => {{
  try {{
    let w = localStorage.getItem("deque-voter");
    if (!w) {{ w = [...crypto.getRandomValues(new Uint8Array(16))].map(b => b.toString(16).padStart(2, "0")).join(""); localStorage.setItem("deque-voter", w); }}
    return w;
  }} catch (e) {{
    return [...crypto.getRandomValues(new Uint8Array(16))].map(b => b.toString(16).padStart(2, "0")).join("");
  }}
}})();
const mine = {{}};
let shown = "";
async function poll() {{
  try {{
    const p = await (await fetch("/{token}/poll", {{cache: "no-store"}})).json();
    const box = document.getElementById("poll");
    const key = p.id ? p.id + "\n" + mine[p.id] : "";
    if (key !== shown) {{
      shown = key;
      box.replaceChildren();
      box.classList.toggle("on", !!p.id);
      if (p.id) {{
        const q = document.createElement("b");
        q.textContent = p.question;
        box.appendChild(q);
        p.choices.forEach((c, k) => {{
          const b = document.createElement("button");
          b.textContent = c;
          if (mine[p.id] === k) b.className = "mine";
          b.onclick = () => {{ mine[p.id] = k; post("vote/" + who + "/" + k); shown = ""; poll(); }};
          box.appendChild(b);
        }});
      }}
      setTimeout(fit, 0);
    }}
  }} catch (e) {{}}
}}
setInterval(poll, 1500);
poll();
// What's drawn, as it's drawn, over a WebSocket; where that can't be
// had, asked for again and again, each answer what's been drawn since the
// last, as soon as there's any.
// Through a tunnel, and able to reach the presenter's machine: straight
// from it, and the rest waits while that's open.
let direct = false, ws = null;
const pause = ms => new Promise(r => setTimeout(r, ms));
const socket = () => new Promise(done => {{
  let opened = false;
  ws = new WebSocket((location.protocol === "https:" ? "wss://" : "ws://") + location.host + "/{token}/ws");
  ws.binaryType = "arraybuffer";
  ws.onopen = () => {{ opened = true; }};
  ws.onmessage = e => {{ if (!direct) term.write(new Uint8Array(e.data)); }};
  ws.onclose = () => done(opened);
}});
async function straight() {{
  try {{
    const pc = new RTCPeerConnection();
    const dc = pc.createDataChannel("talk");
    dc.binaryType = "arraybuffer";
    dc.onmessage = e => term.write(new Uint8Array(e.data));
    dc.onopen = () => {{ direct = true; if (ws) ws.close(); }};
    dc.onclose = () => {{ direct = false; pc.close(); }};
    await pc.setLocalDescription(await pc.createOffer());
    const r = await fetch("/{token}/rtc", {{method: "POST", body: JSON.stringify(pc.localDescription)}});
    if (!r.ok) throw 0;
    await pc.setRemoteDescription(await r.json());
  }} catch (e) {{}}
}}
if ({public} && window.RTCPeerConnection) straight();
(async () => {{
  // Open again when it closes, having opened; if it never does, ask.
  for (;;) {{
    if (direct) {{ await pause(500); continue; }}
    if (await socket()) {{ await pause(1000); continue; }}
    if (!direct) break;
  }}
  let next = "new";
  for (;;) {{
    if (direct) {{ next = "new"; await pause(500); continue; }}
    try {{
      const r = await fetch("/{token}/frames/" + next, {{cache: "no-store"}});
      if (!r.ok) throw 0;
      if (r.headers.get("x-reset")) term.write("\x1b[0m\x1b[?25l\x1b[2J\x1b[H");
      next = r.headers.get("x-next");
      const b = new Uint8Array(await r.arrayBuffer());
      if (b.length) term.write(b);
    }} catch (e) {{
      next = "new";
      await new Promise(r => setTimeout(r, 1000));
    }}
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
        let share = start(&Theme::default(), false, Some(face), false).unwrap();
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
                let head = request(&mut c).unwrap_or_default().to_lowercase();
                let len = head.lines().find_map(|l| l.strip_prefix("content-length: ")).and_then(|n| n.trim().parse().ok()).unwrap_or(0);
                let mut body = vec![0u8; len];
                c.read_exact(&mut body).unwrap();
                head.lines().next().unwrap_or("").to_string()
            })
            .collect()
    }

    #[test]
    fn only_the_remote_drives() {
        let share = start(&Theme::default(), false, None, false).unwrap();
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
                post(&format!("/{remote}/lift")),
            ],
        );
        assert_eq!(got, ["http/1.1 204 no content"; 4]);
        assert_eq!(tls(&port, &[post(&format!("/{watch}/do/next"))]), ["http/1.1 404 not found"]);
        assert_eq!(share.hub.take().as_deref(), Some("next"));
        assert_eq!(share.hub.take().as_deref(), Some("back"));
        assert_eq!(share.hub.points(), [(0.5, 0.25, Touch::Point), (0.0, 0.0, Touch::Lift)]);
        assert_eq!(tls(&port, &[post(&format!("/{remote}/tap/2/0.25"))]), ["http/1.1 404 not found"]);
        // Watching works either way.
        assert!(all(get(&port, &format!("/{watch}"), "Mozilla")).starts_with("HTTP/1.1 200"));
        let page = format!("GET /{watch} HTTP/1.1\r\n\r\n");
        assert_eq!(tls(&port, &[page]), ["http/1.1 200 ok"]);
    }

    #[test]
    fn watchers_react_and_vote_by_number_only() {
        let share = start(&Theme::default(), false, None, false).unwrap();
        let port = share.url.rsplit(':').next().unwrap().split('/').next().unwrap().to_string();
        let (watch, remote) = (share.url.rsplit('/').next().unwrap(), share.remote.rsplit('/').next().unwrap());
        let ok = |p: &str| post(&port, &format!("/{watch}/{p}")).starts_with("HTTP/1.1 204");
        assert!(ok("react/0") && ok("react/5"));
        assert!(!ok("react/6") && !ok("react/x") && !ok("say/hello"));
        assert!(!post(&port, "/nope/react/0").starts_with("HTTP/1.1 204"));
        assert_eq!(share.hub.reactions(), [0, 5]);
        // No poll open: nothing to vote on.
        assert!(!ok("vote/0123456789abcdef/0"));
        let p = Poll { id: "q".into(), question: "which?".into(), choices: vec!["a".into(), "b".into()] };
        share.hub.poll(Some(p.clone()));
        let got = all(get(&port, &format!("/{watch}/poll"), "Mozilla"));
        assert!(got.ends_with(r#"{"choices":["a","b"],"id":"q","question":"which?"}"#), "{got}");
        assert!(ok("vote/0123456789abcdef/0") && ok("vote/fedcba9876543210/1"));
        // A second vote replaces the first; a voter's name is hex, a choice
        // one there is.
        assert!(ok("vote/0123456789abcdef/1"));
        assert!(!ok("vote/not-hex-at-all!/0") && !ok("vote/0123456789abcdef/2") && !ok("vote/abc/0"));
        assert!(share.hub.voted() && !share.hub.voted());
        assert_eq!(share.hub.tally(&p), [0, 2]);
        // The remote's link isn't the watchers'.
        assert!(!post(&port, &format!("/{remote}/react/0")).starts_with("HTTP/1.1 204"));
        // No asking to connect straight but through a tunnel.
        let offer = format!("POST /{watch}/rtc HTTP/1.1\r\nContent-Length: 2\r\n\r\n{{}}");
        let mut c = TcpStream::connect(format!("127.0.0.1:{port}")).unwrap();
        c.write_all(offer.as_bytes()).unwrap();
        assert!(all(c).starts_with("HTTP/1.1 404"));
    }

    #[test]
    fn pages_ask_for_what_they_have_not_had() {
        let share = start(&Theme::default(), false, None, false).unwrap();
        let port = share.url.rsplit(':').next().unwrap().split('/').next().unwrap().to_string();
        let watch = share.url.rsplit('/').next().unwrap().to_string();
        let ask = |after: &str| {
            let r = all(get(&port, &format!("/{watch}/frames/{after}"), "Mozilla"));
            let next = r.lines().find_map(|l| l.strip_prefix("X-Next: ")).unwrap().to_string();
            (next, r.contains("X-Reset: 1"), r.split("\r\n\r\n").nth(1).unwrap_or("").to_string())
        };
        // New: start from here, the slide drawn again for it.
        assert_eq!(ask("new"), ("0".into(), true, String::new()));
        assert!(share.hub.joined());
        share.hub.send(b"one", 80, 24);
        share.hub.send(b"two", 80, 24);
        let (next, reset, got) = ask("0");
        assert!(!reset && got.ends_with("onetwo") && next == "2", "{got:?}");
        assert_eq!(ask("1").2, "two");
        // Nothing new yet: it waits for it.
        let h = share.hub.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            h.send(b"three", 80, 24);
        });
        assert_eq!(ask("2"), ("3".into(), false, "three".into()));
        // Asked for what's no longer kept, or never was: start again.
        assert!(ask("99").1);
    }

    #[test]
    fn a_watcher_too_far_behind_is_let_go() {
        let share = start(&Theme::default(), false, None, false).unwrap();
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
