//! deque lsp: a language server for talk files, over stdin and stdout.
//! Diagnostics are the parser's own, so the editor flags exactly what deque
//! would refuse; completion and hover come from spec, so they offer exactly
//! what deque takes.

use crate::code;
use crate::figlet;
use crate::spec::{self, Named};
use crate::talk;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::PathBuf;

pub fn run() {
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    let mut docs: HashMap<String, String> = HashMap::new();
    while let Some(msg) = read(&mut input) {
        let method = msg["method"].as_str().unwrap_or_default();
        let id = msg.get("id").cloned();
        let p = &msg["params"];
        let uri = p["textDocument"]["uri"].as_str().unwrap_or_default().to_string();
        let result = match method {
            "initialize" => json!({
                "capabilities": {
                    "textDocumentSync": 1,
                    "completionProvider": { "triggerCharacters": ["{", ":", " ", "(", "/"] },
                    "hoverProvider": true,
                    "documentSymbolProvider": true,
                },
                "serverInfo": { "name": "deque", "version": env!("CARGO_PKG_VERSION") },
            }),
            "textDocument/didOpen" => {
                docs.insert(uri.clone(), p["textDocument"]["text"].as_str().unwrap_or_default().into());
                publish(&uri, &docs[&uri]);
                continue;
            }
            "textDocument/didChange" => {
                if let Some(t) = p["contentChanges"].as_array().and_then(|c| c.last()).and_then(|c| c["text"].as_str()) {
                    docs.insert(uri.clone(), t.into());
                    publish(&uri, t);
                }
                continue;
            }
            "textDocument/didClose" => {
                docs.remove(&uri);
                send(&json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                    "params": { "uri": uri, "diagnostics": [] } }));
                continue;
            }
            "textDocument/completion" | "textDocument/hover" => {
                let text = docs.get(&uri).map(String::as_str).unwrap_or_default();
                let lines: Vec<&str> = text.lines().collect();
                let row = p["position"]["line"].as_u64().unwrap_or(0) as usize;
                let line = lines.get(row).copied().unwrap_or_default();
                let col = from_utf16(line, p["position"]["character"].as_u64().unwrap_or(0) as usize);
                let at = At { place: place(&lines, row), line, col, dir: dir_of(&uri) };
                if method == "textDocument/hover" { hover(&at) } else { json!(complete(&at)) }
            }
            "textDocument/documentSymbol" => {
                let text = docs.get(&uri).map(String::as_str).unwrap_or_default();
                outline(text, &dir_of(&uri))
            }
            "shutdown" => Value::Null,
            "exit" => return,
            _ if id.is_none() => continue,
            _ => {
                send(&json!({ "jsonrpc": "2.0", "id": id, "error": { "code": -32601, "message": format!("no method {method}") } }));
                continue;
            }
        };
        if id.is_some() {
            send(&json!({ "jsonrpc": "2.0", "id": id, "result": result }));
        }
    }
}

fn read(r: &mut impl BufRead) -> Option<Value> {
    let mut len = 0;
    loop {
        let mut h = String::new();
        if r.read_line(&mut h).ok()? == 0 {
            return None;
        }
        let h = h.trim();
        if h.is_empty() {
            break;
        }
        if let Some(v) = h.strip_prefix("Content-Length:") {
            len = v.trim().parse().ok()?;
        }
    }
    let mut body = vec![0; len];
    r.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn send(v: &Value) {
    let body = v.to_string();
    let mut o = std::io::stdout().lock();
    let _ = write!(o, "Content-Length: {}\r\n\r\n{body}", body.len());
    let _ = o.flush();
}

fn dir_of(uri: &str) -> PathBuf {
    let path = uri.strip_prefix("file://").unwrap_or(uri);
    // Percent-decoded: %20 and the like.
    let b = path.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(x) = u8::from_str_radix(&path[i + 1..i + 3], 16) {
                out.push(x);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    let p = PathBuf::from(String::from_utf8_lossy(&out).into_owned());
    p.parent().map(|d| d.to_path_buf()).unwrap_or_default()
}

/// Columns: the parser counts chars, LSP UTF-16 units.
fn to_utf16(line: &str, col: usize) -> usize {
    line.chars().take(col).map(char::len_utf16).sum()
}

fn from_utf16(line: &str, units: usize) -> usize {
    let mut n = 0;
    for (i, c) in line.chars().enumerate() {
        if n >= units {
            return i;
        }
        n += c.len_utf16();
    }
    line.chars().count()
}

fn publish(uri: &str, text: &str) {
    let (_, diags) = talk::parse(text, &dir_of(uri), true);
    let lines: Vec<&str> = text.lines().collect();
    let ds: Vec<Value> = diags
        .iter()
        .map(|d| {
            let l = lines.get(d.line).copied().unwrap_or_default();
            let end = d.end.max(d.start + 1);
            json!({
                "range": {
                    "start": { "line": d.line, "character": to_utf16(l, d.start) },
                    "end": { "line": d.line, "character": to_utf16(l, end) },
                },
                "severity": if d.warn { 2 } else { 1 },
                "source": "deque",
                "message": d.msg,
            })
        })
        .collect();
    send(&json!({ "jsonrpc": "2.0", "method": "textDocument/publishDiagnostics", "params": { "uri": uri, "diagnostics": ds } }));
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Place {
    /// Before the first `---`.
    Settings,
    /// Among a slide's options, right after its `---`.
    Options { draw: bool },
    Body { draw: bool, fence: bool },
}

/// Where in the talk a line is, from the lines before it.
fn place(lines: &[&str], row: usize) -> Place {
    let mut at = Place::Settings;
    for l in lines.iter().take(row) {
        let sep = l.trim_end() == "---";
        at = match at {
            Place::Body { fence: true, draw } => Place::Body { draw, fence: !l.trim_start().starts_with("```") },
            _ if sep => Place::Options { draw: false },
            Place::Settings => Place::Settings,
            Place::Options { draw } => match talk::key_line(l) {
                Some((k, _)) if spec::find(spec::SLIDE, k).is_some() => Place::Options { draw: draw || k == "draw" },
                _ => Place::Body { draw, fence: l.trim_start().starts_with("```") },
            },
            Place::Body { draw, .. } => Place::Body { draw, fence: l.trim_start().starts_with("```") },
        };
    }
    at
}

struct At<'a> {
    place: Place,
    line: &'a str,
    /// The cursor, in chars.
    col: usize,
    dir: PathBuf,
}

impl At<'_> {
    fn before(&self) -> String {
        self.line.chars().take(self.col).collect()
    }
}

/// A completion. The detail shows beside it; documentation, when it says
/// more than the detail, under it.
fn item(label: &str, insert: &str, kind: u32, detail: &str, doc: &str) -> Value {
    let mut v = json!({ "label": label, "insertText": insert, "kind": kind, "detail": detail });
    if !doc.is_empty() && doc != detail {
        v["documentation"] = json!({ "kind": "markdown", "value": doc });
    }
    v
}

// Completion item kinds, as LSP numbers them.
const PROPERTY: u32 = 10;
const VALUE: u32 = 12;
const KEYWORD: u32 = 14;
const COLOR: u32 = 16;
const FILE: u32 = 17;

fn values_doc(v: &[Named]) -> String {
    v.iter().filter(|(_, d)| !d.is_empty()).map(|(n, d)| format!("- `{n}`: {d}")).collect::<Vec<_>>().join("\n")
}

fn opt_doc(o: &spec::Opt) -> String {
    let vals = values_doc(o.values);
    if vals.is_empty() { o.doc.to_string() } else { format!("{}\n\n{vals}", o.doc) }
}

fn complete(at: &At) -> Vec<Value> {
    let before = at.before();
    // A color, inside {.
    if let Some(i) = before.rfind('{') {
        let typed = &before[i + 1..];
        if typed.starts_with('/') {
            return vec![];
        }
        if !typed.contains('}') && at.place != Place::Settings {
            // The } too, unless an editor's autopairs already put one there.
            let close = if at.line.chars().nth(at.col) == Some('}') { "" } else { "}" };
            let mut v: Vec<Value> = spec::COLORS
                .iter()
                .map(|(n, d)| item(n, &format!("{n}{close}"), COLOR, d, ""))
                .collect();
            v.push(item("/", &format!("/{close}"), COLOR, "closes the color opened last", ""));
            return v;
        }
    }
    // A picture's file.
    if let Some(i) = before.rfind("](") {
        if before.contains("![") && !before[i..].contains(')') {
            return files(at, &before[i + 2..]);
        }
    }
    // A code block's language, opening a ```.
    if let (Some(lang), Place::Body { fence: false, .. } | Place::Options { draw: false }) = (before.trim_start().strip_prefix("```"), at.place) {
        if !lang.contains(' ') {
            let mut seen = std::collections::HashSet::new();
            return code::languages()
                .into_iter()
                .filter(|(tok, _)| seen.insert(tok.clone()))
                .map(|(tok, name)| item(&tok, &tok, VALUE, &name, ""))
                .collect();
        }
    }
    let opts = match at.place {
        Place::Settings => Some(spec::TALK),
        Place::Options { .. } => Some(spec::SLIDE),
        Place::Body { .. } => None,
    };
    // An option's value, after its colon.
    if let (Some(opts), Some((k, v))) = (opts, before.split_once(':')) {
        // After `sky: ants`: what it can say of the colony, one of each,
        // those not said yet.
        if let Some(said) = v.trim_start().strip_prefix("ants ").filter(|_| k == "sky" && spec::find(opts, k).is_some()) {
            let unsaid = |set: &&&[Named]| !said.split_whitespace().any(|w| set.iter().any(|(n, _)| *n == w));
            return spec::ANTS.iter().filter(unsaid).flat_map(|set| set.iter()).map(|(n, d)| item(n, n, VALUE, d, d)).collect();
        }
        return match spec::find(opts, k) {
            Some(o) => o.values.iter().map(|(n, d)| item(n, n, VALUE, d, d)).collect(),
            None => vec![],
        };
    }
    if before.contains(' ') && !matches!(at.place, Place::Body { fence: false, .. }) {
        return vec![];
    }
    let keys = |opts: &[spec::Opt]| -> Vec<Value> {
        opts.iter().map(|o| item(o.key, &format!("{}: ", o.key), PROPERTY, o.doc, &values_doc(o.values))).collect()
    };
    let syntax = || -> Vec<Value> {
        spec::SYNTAX.iter().map(|(s, d)| item(s.trim(), s, KEYWORD, d, d)).collect()
    };
    let commands = || -> Vec<Value> {
        let mut v: Vec<Value> = spec::DRAW.iter().map(|(n, u, d)| item(n, &format!("{n} "), KEYWORD, u, d)).collect();
        v.extend(syntax().into_iter().filter(|i| i["label"] == "##" || i["label"] == "//"));
        v
    };
    match at.place {
        Place::Settings => keys(spec::TALK),
        Place::Options { draw } => {
            let mut v = keys(spec::SLIDE);
            v.extend(if draw { commands() } else { syntax() });
            v
        }
        Place::Body { draw: true, .. } if !before.contains(' ') => commands(),
        Place::Body { draw: false, fence } if !before.contains(' ') => {
            if fence { vec![item(">", "> ", KEYWORD, spec::SYNTAX[3].1, spec::SYNTAX[3].1)] } else { syntax() }
        }
        _ => vec![],
    }
}

/// The pictures and folders beside the talk, under what has been typed.
fn files(at: &At, typed: &str) -> Vec<Value> {
    let (sub, _) = typed.rsplit_once('/').unwrap_or(("", typed));
    let Ok(rd) = std::fs::read_dir(at.dir.join(sub)) else { return vec![] };
    let mut out = vec![];
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let dir = e.file_type().is_ok_and(|t| t.is_dir());
        let img = ["png", "jpg", "jpeg"].iter().any(|x| name.to_lowercase().ends_with(&format!(".{x}")));
        if dir || img {
            let label = if dir { format!("{name}/") } else { name };
            out.push(item(&label, &label, FILE, "", ""));
        }
    }
    out
}

fn hover(at: &At) -> Value {
    let md = |s: String| json!({ "contents": { "kind": "markdown", "value": s } });
    let chars: Vec<char> = at.line.chars().collect();
    let word_at = |c: usize| -> (usize, usize) {
        let isw = |x: char| x.is_alphanumeric() || x == '-' || x == '/';
        let mut a = c.min(chars.len());
        while a > 0 && isw(chars[a - 1]) {
            a -= 1;
        }
        let mut b = c.min(chars.len());
        while b < chars.len() && isw(chars[b]) {
            b += 1;
        }
        (a, b)
    };
    let (a, b) = word_at(at.col);
    let word: String = chars[a..b].iter().collect();
    // A color, inside {}.
    if a > 0 && chars[a - 1] == '{' && chars.get(b) == Some(&'}') {
        if let Some((n, d)) = spec::COLORS.iter().find(|(n, _)| *n == word) {
            return md(format!("`{{{n}}}`: {d}. `{{/}}` ends it"));
        }
        if word == "/" {
            return md("`{/}` goes back to the color before".into());
        }
    }
    let opts = match at.place {
        Place::Settings => Some(spec::TALK),
        Place::Options { .. } => Some(spec::SLIDE),
        Place::Body { .. } => None,
    };
    if let (Some(opts), Some((k, _))) = (opts, talk::key_line(at.line)) {
        if let Some(o) = spec::find(opts, k) {
            if b <= k.len() {
                return md(format!("**{k}**: {}", opt_doc(o)));
            }
            // What `sky: ants` says of the colony: before the skies, one
            // of which, sand, is also what a colony can be in.
            let ants = at.line.split_once(':').is_some_and(|(_, v)| v.split_whitespace().next() == Some("ants"));
            if let Some((n, d)) = spec::ANTS.iter().flat_map(|set| set.iter()).find(|(n, _)| k == "sky" && ants && *n == word) {
                return md(format!("`sky: ants {n}`: {d}"));
            }
            if let Some((n, d)) = o.values.iter().find(|(n, _)| *n == word) {
                return md(format!("`{k}: {n}`: {d}"));
            }
        }
    }
    if let Place::Body { draw: true, .. } = at.place {
        if a == 0 {
            if let Some((_, u, d)) = spec::DRAW.iter().find(|(n, ..)| *n == word) {
                return md(format!("`{u}`\n\n{d}"));
            }
        }
    }
    // A headline, as it will look.
    if let Some(h) = at.line.strip_prefix("# ").filter(|_| matches!(at.place, Place::Body { fence: false, .. } | Place::Options { .. })) {
        let art = figlet::render(&h.trim().to_uppercase());
        let w = art.first().map_or(0, Vec::len);
        let rows: Vec<String> = art.iter().map(|r| r.iter().collect()).collect();
        return md(format!("```\n{}\n```\n{w} columns wide", rows.join("\n")));
    }
    let t = at.line.trim_end();
    for (s, d) in spec::SYNTAX {
        if t.starts_with(s.trim_end()) && at.col <= s.len() {
            return md(format!("`{}`: {d}", s.trim()));
        }
    }
    Value::Null
}

/// One entry a slide, for an editor's outline: its number and title.
fn outline(text: &str, dir: &std::path::Path) -> Value {
    let (talk, _) = talk::parse(text, dir, true);
    let last = text.lines().count().max(1) - 1;
    let starts: Vec<usize> = talk.slides.iter().map(|s| s.line).collect();
    let syms: Vec<Value> = talk
        .slides
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let end = starts.get(i + 1).map_or(last, |n| n.saturating_sub(1)).max(s.line);
            let range = json!({ "start": { "line": s.line, "character": 0 }, "end": { "line": end, "character": 0 } });
            json!({ "name": format!("{} · {}", i + 1, s.title()), "kind": 5, "range": range, "selectionRange": range })
        })
        .collect();
    json!(syms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(line: &str) -> At<'_> {
        At { place: Place::Options { draw: false }, line, col: line.chars().count(), dir: PathBuf::from(".") }
    }

    #[test]
    fn an_ant_colony_is_offered_and_explained() {
        let labels = |line: &str| -> Vec<String> { complete(&at(line)).iter().map(|i| i["label"].as_str().unwrap().to_string()).collect() };
        // A sky: the colony among them, and the newer ones.
        let skies = labels("sky: ");
        assert!(["ants", "koi", "sand"].iter().all(|k| skies.iter().any(|s| s == k)));
        // After it, a view, a species and what it's in; said, not again.
        assert_eq!(labels("sky: ants "), ["farm", "ground", "leafcutter", "black", "fire", "honeypot", "army", "soil", "sand", "gel", "founding", "grown"]);
        assert_eq!(labels("sky: ants ground fire "), ["soil", "sand", "gel", "founding", "grown"]);
        assert!(labels("sky: stars ").iter().all(|l| l != "ground"));
        let said = |line: &str, col: usize| hover(&At { col, ..at(line) })["contents"]["value"].as_str().unwrap_or("").to_string();
        assert!(said("sky: ants ground fire", 12).starts_with("`sky: ants ground`: the same colony from above"));
        assert!(said("sky: ants ground fire", 19).contains("red, quick"));
        assert!(said("sky: ants", 7).starts_with("`sky: ants`: an ant colony"));
        // sand is a sky of its own, and what a colony's in.
        assert!(said("sky: sand", 7).contains("pouring"));
        assert!(said("sky: ants sand", 12).contains("pale sand"));
    }
}
