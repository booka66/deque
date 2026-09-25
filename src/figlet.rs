//! Headlines in ANSI Shadow, read from the FIGlet font built into deque.

use std::sync::OnceLock;

const FONT: &str = include_str!("ansi-shadow.flf");

/// The glyph rows of every printable ASCII character, 32 to 126.
fn glyphs() -> &'static Vec<Vec<String>> {
    static G: OnceLock<Vec<Vec<String>>> = OnceLock::new();
    G.get_or_init(|| {
        let lines: Vec<&str> = FONT.lines().collect();
        let h: Vec<&str> = lines[0].split_whitespace().collect();
        let hardblank = h[0].chars().nth(5).unwrap();
        let height: usize = h[1].parse().unwrap();
        let skip = h[5].parse::<usize>().unwrap() + 1;
        (0..95)
            .map(|c| {
                (0..height)
                    .map(|i| {
                        let row = lines[skip + c * height + i].trim_end_matches('@');
                        row.replace(hardblank, " ")
                    })
                    .collect()
            })
            .collect()
    })
}

/// Whether the font can draw the character.
pub fn has(c: char) -> bool {
    (' '..='~').contains(&c)
}

/// The text in block letters: rows of one width, in chars.
pub fn render(t: &str) -> Vec<Vec<char>> {
    let g = glyphs();
    let height = g[0].len();
    let mut rows: Vec<String> = (0..height)
        .map(|i| {
            let row: String = t.chars().filter(|&c| has(c)).map(|c| g[c as usize - 32][i].as_str()).collect();
            row.trim_end().to_string()
        })
        .collect();
    while rows.last().is_some_and(|r| r.is_empty()) {
        rows.pop();
    }
    let w = rows.iter().map(|r| r.chars().count()).max().unwrap_or(0);
    rows.iter()
        .map(|r| {
            let mut v: Vec<char> = r.chars().collect();
            v.resize(w, ' ');
            v
        })
        .collect()
}
