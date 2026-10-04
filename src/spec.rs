//! Every name a talk file can use, with what it does: the parser checks
//! against these, and the language server offers and explains them.

pub type Named = (&'static str, &'static str);

pub const FX: &[Named] = &[
    ("wipe", "from the left, a column at a time (the default)"),
    ("iris", "from the middle out"),
    ("slide", "in from off the left edge"),
    ("zip", "the rows in from both edges at once, alternating"),
    ("drop", "the rows fall from the top, the bottom one first, and stack"),
    ("fade", "up out of the background"),
    ("scramble", "noise that settles into the letters, a cell at a time"),
    ("assemble", "every cell flies in from somewhere on the screen"),
    ("glitch", "wipes in, tears and breaks up, goes dark a moment, and comes back"),
    ("typewriter", "typed a column at a time with a block cursor riding the edge"),
    ("random", "one of the others, the same one for a slide every time"),
    ("none", "there at once"),
];

pub const LINES: &[Named] = &[
    ("type", "typed out, a few letters a frame (the default for lines)"),
    ("glide", "in from the right edge, easing into place (the default for steps)"),
    ("fade", "up out of the background"),
    ("scramble", "noise settling into the letters, left to right"),
    ("count", "its numbers counting up from 0"),
    ("none", "there at once"),
];

pub const THEN: &[Named] = &[
    ("shine", "a glint running across the headline"),
    ("pulse", "the headline brightening and back"),
    ("shake", "the headline jolted side to side, settling"),
    ("rainbow", "colors running through the headline"),
    ("sparkle", "glints coming and going around the headline"),
    ("confetti", "confetti falling the height of the screen"),
];

pub const TR: &[Named] = &[
    ("dissolve", "the slide before falls away a cell at a time"),
    ("sweep", "a bar crosses the screen, leaving it empty"),
    ("curtain", "bars from both sides, meeting in the middle"),
    ("morph", "the slide before turns into this one: its words, code and headline swing to their new places"),
    ("life", "the slide before becomes Conway's Game of Life and dies out"),
    ("sand", "the slide before crumbles to sand, falls in a heap and runs out through the floor"),
    ("focus", "the slide before melts into a blur, and this one comes into focus out of it"),
    ("none", "straight to the next slide (the default)"),
];

pub const SKY: &[Named] = &[
    ("stars", "stars twinkling and drifting, now and then one shooting across"),
    ("snow", "snow falling, swaying as it goes, lying a while on the text it lands on"),
    ("rain", "rain streaking down"),
    ("embers", "sparks rising from the bottom, flickering out"),
    ("life", "Conway's Game of Life, faint, never settling"),
    ("boids", "a flock of birds wheeling about, swerving round the text, the pointer, and the two hawks hunting them, now and then one perching on a line"),
    ("fireflies", "a few warm lights drifting, now and then lit"),
    ("sand", "sand pouring in, heaping on the text and the floor, and running out when it's deep"),
    ("koi", "a pond: koi swimming under lily pads, light crossing the water; a click scatters food, and they come and eat it"),
    ("ants", "an ant colony, kept: seen from the side, the nest, dug a grain at a time; from above, the same ants out on the ground, on their trails. After it, any of a view, a species and what it's in, and how many: sky: ants ground fire sand 80. A second number is how many piles of food there are to be, each there till the number drops: sky: ants 60 ${sh: gh issue list | wc -l} is a pile a task. A click drops food, or on an ant, follows it; v is the other view; i says how it's doing; H, what's happened. It's kept between runs, and lives on between them"),
    ("none", "the terminal's own background (the default)"),
];

/// What `sky: ants` can say after it, one of each: the view, the species,
/// and what the nest is in.
pub const ANTS: [&[Named]; 4] = [
    &[
        ("farm", "the colony from the side: a shaft, galleries and chambers, dug as you watch; the queen, her brood, the store, the midden; days and nights, seasons, rain (the default)"),
        ("ground", "the same colony from above: those that are out, finding food by scent, trails forming round the words and round the twigs that fall; a spider comes hunting now and then"),
    ],
    &[
        ("leafcutter", "rust-brown, with majors: they climb the plants, cut leaf and carry it overhead to the fungus garden (the default)"),
        ("black", "garden ants: they milk the aphids on the plants and take what's fallen, and run from a spider"),
        ("fire", "red, quick, and more of them; all of them fight a spider"),
        ("honeypot", "they milk aphids, and keep what they gather in themselves: workers hanging from the store's roof, full"),
        ("army", "no nest but a hollow for the night: they raid in a column by day, and every evening the whole colony moves on"),
    ],
    &[("soil", "brown earth (the default)"), ("sand", "pale sand"), ("gel", "blue gel, as in the farms you can see through")],
    &[
        ("founding", "a colony just begun: a queen alone with her first eggs, and everything still to do (the default)"),
        ("grown", "a colony well under way when the slide comes: workers about, the store dug"),
    ],
];

pub const COLORS: &[Named] = &[
    ("accent", "headlines, and what should stand out"),
    ("muted", "asides and labels, what should recede"),
    ("good", "success, and what to press"),
    ("bad", "failure"),
    ("warm", "a second highlight; arrows in drawn slides"),
    ("link", "code and commands; what `backticks` use"),
];

pub const ON_OFF: &[Named] = &[("on", ""), ("off", "")];

pub struct Opt {
    pub key: &'static str,
    pub doc: &'static str,
    /// The values it takes, when they are a fixed set.
    pub values: &'static [Named],
}

const fn opt(key: &'static str, doc: &'static str, values: &'static [Named]) -> Opt {
    Opt { key, doc, values }
}

/// Options a slide sets in the lines right after its `---`.
pub const SLIDE: &[Opt] = &[
    opt("fx", "how the headline arrives", FX),
    opt("lines", "how the lines arrive", LINES),
    opt("reveal", "how a step (`> line`) arrives", LINES),
    opt("then", "flourishes once the slide is all there, space-separated", THEN),
    opt("tr", "how the slide before this one leaves", TR),
    opt("sky", "what moves behind the slide, the whole time it's up. A number after it is how many: sky: rain 200; for boids, birds then hawks: sky: boids 12 3", SKY),
    opt("glow", "the headline lighting what's around it, breathing", ON_OFF),
    opt("enter", "a command enter runs instead of going on: a TUI, a shell, a demo. Run by sh (cmd on Windows) in the talk's folder", &[]),
    opt("play", "a recording (an asciinema .cast) enter plays instead of running something live: space pauses, → skips to its next marker, q stops. With `enter` too, enter cuts to that, live", &[]),
    opt("cols", "with --tv, how many columns wide the screen is while `enter` runs", &[]),
    opt("keys", "while `enter` runs, the keys you press shown on a row under it, the newest lit, so the room can follow a TUI", ON_OFF),
    opt("draw", "a drawn slide, WIDTHxHEIGHT: its lines are drawing commands, not text", &[]),
    opt("time", "how long the slide's meant to be up, for pacing in deque notes, and how long --loop keeps it up: 90s, 2m, 1m30s", &[]),
];

/// Settings for the whole talk, before the first `---`.
pub const TALK: &[Opt] = &[
    opt("fx", "how headlines arrive, unless a slide says otherwise", FX),
    opt("lines", "how lines arrive, unless a slide says otherwise", LINES),
    opt("reveal", "how steps arrive, unless a slide says otherwise", LINES),
    opt("then", "flourishes on every slide, unless a slide says otherwise", THEN),
    opt("tr", "how slides leave, unless a slide says otherwise", TR),
    opt("sky", "what moves behind every slide, unless a slide says otherwise; one sky goes on from slide to slide. A number after it is how many: sky: rain 200", SKY),
    opt("glow", "headlines lighting what's around them, unless a slide says otherwise", ON_OFF),
    opt("cursor", "whether the cursor shows; --cursor and --no-cursor win over it", ON_OFF),
    opt("keys", "the keys you press shown under every `enter`, unless a slide says otherwise", ON_OFF),
    opt("calm", "nothing moves that needn't: no skies, glow, flourishes, transitions or morphs; what arrives fades in. --calm does the same", ON_OFF),
    opt("accent", "color: headlines, and what should stand out. #rrggbb or 38;2;r;g;b", &[]),
    opt("muted", "color: asides and labels. #rrggbb or 38;2;r;g;b", &[]),
    opt("good", "color: success, and what to press. #rrggbb or 38;2;r;g;b", &[]),
    opt("bad", "color: failure. #rrggbb or 38;2;r;g;b", &[]),
    opt("warm", "color: a second highlight; arrows. #rrggbb or 38;2;r;g;b", &[]),
    opt("link", "color: code and `backticks`. #rrggbb or 38;2;r;g;b", &[]),
    opt("fg", "the text color fades end at. #rrggbb or 38;2;r;g;b", &[]),
    opt("bg", "the background color fades start from. #rrggbb or 38;2;r;g;b", &[]),
];

/// Commands in a `draw:` slide. Positions are ROW,COL from the top left of
/// the drawing, which is centered on the screen.
pub const DRAW: &[(&str, &str, &str)] = &[
    ("step", "step", "what follows comes in on the next key, drawn in moving"),
    ("text", "text ROW,COL text", "text at a place, typed in"),
    ("center", "center ROW text", "text centered on a row"),
    ("box", "box ROW,COL WIDTHxHEIGHT title", "a rounded box, its border traced in, the title bold inside"),
    ("arrow", "arrow ROW,COL ROW,COL …", "an arrow through the points, straight lines between them, the head at the last"),
    ("dotted", "dotted ROW,COL ROW,COL …", "a dotted arrow, as arrow"),
    ("clear", "clear ROW-ROW", "the rows emptied, to replace what an earlier step drew"),
];

pub const SYNTAX: &[(&str, &str)] = &[
    ("---", "starts a slide. Options go on the lines right after it"),
    ("## ", "the slide's label, small and numbered above the headline"),
    ("# ", "the headline, in block letters"),
    ("> ", "a step: the line comes in on a key"),
    ("```", "lines between two of these line up in a column instead of each being centered. With a language (```ts), they're code, highlighted, and taken as they are; code in the same language on the next slide morphs into its code. After the language: run (sh bash zsh fish py js rb) runs it on a step; a file (src/a.ts, src/a.ts:10-24, src/a.ts#name) takes its lines from there, and @REV after it (src/a.ts#name@HEAD~2) as it was at that revision in git; focus: 2|4-5 lights those lines a step each. ```chart lines are a label and a number: bars. ```poll lines are choices: with --share, watchers vote on them, and the bars grow as they do; a `* ` choice is the answer, lit on a step of its own. A ```chart number can be a ${NAME}"),
    ("```graph", "a diagram: lines of boxes and arrows, a -> b -> c (..> dotted, `: label` after the last), laid out left to right for you (```graph down: top to bottom); a `> ` line comes in on a step. Text after the block goes under it. The slide has a label, no headline"),
    ("|", "a table row: | a | b |; a row of --- under the first makes it a header"),
    ("![", "![label](file.png): a picture. Several on one line go side by side, labelled; on lines of their own, stacked. Text lines become the caption"),
    ("${", "${NAME}: an environment variable's value. ${sh: command}: what the command prints, run in the talk's folder when the talk's read, and again each minute while it's up, what changed swinging to its new place: # ${sh: date +%H:%M} is a clock, sky: boids ${sh: gh issue list | wc -l} a bird a task. $${ is a ${ left as it's written"),
    ("//", "in a slide, a speaker note, shown by `deque notes`; before the first `---`, a comment"),
];

pub fn find<'a>(opts: &'a [Opt], key: &str) -> Option<&'a Opt> {
    opts.iter().find(|o| o.key == key)
}

pub fn names(v: &[Named]) -> String {
    v.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
}

/// For a word that's none of these: the one it's likely a slip for, as
/// the start of a diagnostic, or nothing when none is close.
pub fn near<'a>(word: &str, among: impl IntoIterator<Item = &'a str>) -> String {
    let w: Vec<char> = word.to_lowercase().chars().collect();
    // Letters in, out, changed or swapped with the next.
    let dist = |b: &str| {
        let b: Vec<char> = b.chars().collect();
        let mut d = vec![vec![0usize; b.len() + 1]; w.len() + 1];
        for (i, r) in d.iter_mut().enumerate() {
            r[0] = i;
        }
        for (j, v) in d[0].iter_mut().enumerate() {
            *v = j;
        }
        for i in 1..=w.len() {
            for j in 1..=b.len() {
                let mut v = (d[i - 1][j - 1] + usize::from(w[i - 1] != b[j - 1])).min(d[i - 1][j] + 1).min(d[i][j - 1] + 1);
                if i > 1 && j > 1 && w[i - 1] == b[j - 2] && w[i - 2] == b[j - 1] {
                    v = v.min(d[i - 2][j - 2] + 1);
                }
                d[i][j] = v;
            }
        }
        d[w.len()][b.len()]
    };
    // A slip: a letter or two off, fewer for short words.
    let most = if w.len() <= 4 { 1 } else { 2 };
    // Ties to the one the same length: a swap over a letter left out.
    match among.into_iter().map(|c| (dist(&c.to_lowercase()), c.chars().count().abs_diff(w.len()), c)).filter(|&(d, ..)| d <= most).min() {
        Some((.., c)) => format!("did you mean \"{}\"? ", c.to_lowercase()),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slips_are_named() {
        let keys = || SLIDE.iter().map(|o| o.key);
        assert_eq!(near("syk", keys()), "did you mean \"sky\"? ");
        assert_eq!(near("glwo", keys()), "did you mean \"glow\"? ");
        assert_eq!(near("Rust", ["rs", "rust", "rst"]), "did you mean \"rust\"? ");
        assert_eq!(near("banana", keys()), "");
    }
}
