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
    ("focus", "the slide before melts into a blur, and this one comes into focus out of it"),
    ("none", "straight to the next slide (the default)"),
];

pub const SKY: &[Named] = &[
    ("stars", "stars twinkling and drifting, now and then one shooting across"),
    ("snow", "snow falling, swaying as it goes"),
    ("rain", "rain streaking down"),
    ("embers", "sparks rising from the bottom, flickering out"),
    ("life", "Conway's Game of Life, faint, never settling"),
    ("boids", "a flock of birds wheeling about, swerving round the text, the pointer, and the two hawks hunting them"),
    ("fireflies", "a few warm lights drifting, now and then lit"),
    ("none", "the terminal's own background (the default)"),
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
    opt("sky", "what moves behind the slide, the whole time it's up", SKY),
    opt("glow", "the headline lighting what's around it, breathing", ON_OFF),
    opt("enter", "a command enter runs instead of going on: a TUI, a shell, a demo. Run by sh (cmd on Windows) in the talk's folder", &[]),
    opt("cols", "with --tv, how many columns wide the screen is while `enter` runs", &[]),
    opt("draw", "a drawn slide, WIDTHxHEIGHT: its lines are drawing commands, not text", &[]),
];

/// Settings for the whole talk, before the first `---`.
pub const TALK: &[Opt] = &[
    opt("fx", "how headlines arrive, unless a slide says otherwise", FX),
    opt("lines", "how lines arrive, unless a slide says otherwise", LINES),
    opt("reveal", "how steps arrive, unless a slide says otherwise", LINES),
    opt("then", "flourishes on every slide, unless a slide says otherwise", THEN),
    opt("tr", "how slides leave, unless a slide says otherwise", TR),
    opt("sky", "what moves behind every slide, unless a slide says otherwise; one sky goes on from slide to slide", SKY),
    opt("glow", "headlines lighting what's around them, unless a slide says otherwise", ON_OFF),
    opt("cursor", "whether the cursor shows; --cursor and --no-cursor win over it", ON_OFF),
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
    ("```", "lines between two of these line up in a column instead of each being centered. With a language (```ts), they're code, highlighted, and taken as they are; code in the same language on the next slide morphs into its code. ```sh run runs it on a step (sh bash zsh fish py js rb), its output under the slide"),
    ("![", "![label](file.png): a picture. Several on one line go side by side, labelled; on lines of their own, stacked. Text lines become the caption"),
    ("//", "in a slide, a speaker note, shown by `deque notes`; before the first `---`, a comment"),
];

pub fn find<'a>(opts: &'a [Opt], key: &str) -> Option<&'a Opt> {
    opts.iter().find(|o| o.key == key)
}

pub fn names(v: &[Named]) -> String {
    v.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
}
