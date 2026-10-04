//! What's behind a slide, alive the whole time it's up: `sky:` stars,
//! snow, rain, embers, life, boids, fireflies, sand, koi or ants, and `glow:`, a light the headline's letters
//! give off onto what's around them, breathing.
//!
//! It's drawn in pixels four to a cell, two across and two down: a cell's
//! four become one of the quadrant blocks (▘ ▚ ▟ …) in the two colors
//! nearest them, so a star is a quarter of a cell and a glow fades in
//! quarters. Text goes over it, the glow behind it; a space put with no
//! color of its own is see-through.

use crate::markup::{Cell, Rgb, Style, Theme};
use crate::screen::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    None,
    Stars,
    Snow,
    Rain,
    Embers,
    Life,
    Boids,
    Fireflies,
    Sand,
    Koi,
    Ants,
}

impl Kind {
    /// A sky by its name, the first word of what `sky:` says.
    pub fn from(name: &str) -> Kind {
        match name.split_whitespace().next().unwrap_or("") {
            "stars" => Kind::Stars,
            "snow" => Kind::Snow,
            "rain" => Kind::Rain,
            "embers" => Kind::Embers,
            "life" => Kind::Life,
            "boids" => Kind::Boids,
            "fireflies" => Kind::Fireflies,
            "sand" => Kind::Sand,
            "koi" => Kind::Koi,
            "ants" => Kind::Ants,
            _ => Kind::None,
        }
    }
}

/// How many `sky:` says, the numbers after its name: `boids 12 3` is twelve
/// birds and three hawks.
pub fn many(name: &str) -> [Option<usize>; 2] {
    let mut n = name.split_whitespace().skip(1).filter_map(|w| w.parse().ok());
    [n.next(), n.next()]
}

/// What else `sky:` says, the words after its name that aren't numbers:
/// `ants ground fire 80` says `ground fire`.
pub fn said(name: &str) -> String {
    name.split_whitespace().skip(1).filter(|w| w.parse::<usize>().is_err()).collect::<Vec<_>>().join(" ")
}

/// The most of anything, however many the talk says.
const MOST: usize = 600;

/// A koi, in a space where a step down is as long as a step across: x in
/// pixels, y in half pixels, a pixel being twice as tall as wide.
#[derive(Clone)]
struct Koi {
    /// Its head, then each joint of its back, then its tail fin's two.
    spine: Vec<(f64, f64)>,
    x: f64,
    y: f64,
    /// Where it's heading, how fast it's turning, and its speed.
    dir: f64,
    turn: f64,
    v: f64,
    /// How far through a stroke of its tail it is.
    beat: f64,
    size: f64,
    ph: f64,
    /// Its markings, and which way it turns off from text.
    look: u8,
    side: f64,
}

/// How wide a koi is at each joint from its head, half across, and then
/// its tail fin's.
const GIRTH: [f64; 10] = [2.5, 3.9, 4.4, 4.3, 3.8, 3.1, 2.4, 1.7, 1.2, 0.8];
const FIN: [f64; 2] = [2.4, 4.4];
/// From one joint to the next.
const JOINT: f64 = 3.6;

/// A koi's two colors and how much of it the second covers: white with
/// red, all orange, white with orange, gold.
const COATS: [(Rgb, Rgb, f64); 4] = [
    (Rgb(244, 238, 226), Rgb(214, 58, 36), 0.1),
    (Rgb(240, 120, 34), Rgb(246, 160, 60), 0.3),
    (Rgb(244, 238, 226), Rgb(240, 120, 34), -0.1),
    (Rgb(236, 180, 70), Rgb(244, 238, 226), 0.6),
];
const DEEP: Rgb = Rgb(14, 62, 78);
const LIGHT: Rgb = Rgb(120, 200, 205);
const PAD: Rgb = Rgb(58, 132, 76);
const FOOD: Rgb = Rgb(214, 168, 96);
/// How long food floats before it's gone, in seconds, and how much can be
/// on the water at once.
const FLOATS_FOR: f64 = 30.0;
const MOST_FOOD: usize = 80;

/// From a to b turning the short way, -π to π.
fn veer(a: f64, b: f64) -> f64 {
    (b - a + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU) - std::f64::consts::PI
}

/// Something moving, in pixels: where, how fast, and how it looks.
#[derive(Clone, Copy)]
struct Mote {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    /// How bright at most, 0 to 1.
    a: f64,
    /// Its own phase, for twinkling and swaying.
    ph: f64,
    /// Seconds since it came, and how long it lasts.
    age: f64,
    life: f64,
    tint: u8,
}

pub struct Sky {
    pub kind: Kind,
    pub glow: bool,
    /// In pixels, twice the cells each way.
    pw: usize,
    ph: usize,
    /// The background and glow, which text sits on.
    field: Vec<Rgb>,
    /// The field with the motes over it.
    px: Vec<Rgb>,
    motes: Vec<Mote>,
    /// Life's cells, and how lit each is, fading in and out.
    cells: Vec<bool>,
    lit: Vec<f64>,
    /// When life last stepped, and how many times it has.
    beat: f64,
    gens: u64,
    /// A shooting star, when one is crossing.
    streak: Option<Mote>,
    next_streak: f64,
    rng: Rng,
    t: f64,
    /// The glow's reach, as (dx, dy, weight) from a cell's top left pixel.
    kernel: Vec<(i32, i32, f64)>,
    /// The cells near text, this frame.
    quiet: Vec<bool>,
    /// A screen frozen and blurred, a color and how much of it each pixel
    /// takes, behind whatever's drawn now.
    frost: Option<Vec<(Rgb, f64)>>,
    /// The mouse, in pixels, and when it last moved: a laser pointer's dot
    /// and the light around it, fading once it's still. Each click, a ring
    /// going out from where it was, and when.
    pub pointer: Option<(f64, f64, f64)>,
    pub ripples: Vec<(f64, f64, f64)>,
    /// Where the pointer's been lately, and when, for its trail.
    pub trail: Vec<(f64, f64, f64)>,
    /// With boids: hawks, bigger and faster, after the nearest bird.
    hawks: Vec<Mote>,
    /// How many the talk asked for, and of hawks; and what else it said
    /// of the sky, which a sky of another sort wouldn't be.
    pub many: [Option<usize>; 2],
    pub said: String,
    /// The ant colony, made when it's first needed, knowing by then what
    /// the talk said of it.
    ants: Option<crate::ants::Colony>,
    seed: u32,
    /// The colony's link with another deque showing it, looked for when
    /// it's first needed: none, when no talk's being presented.
    twin: Option<Option<crate::twin::Twin>>,
    /// The cells with text in them, this frame: what snow and sand lie on
    /// and birds perch on.
    text: Vec<bool>,
    /// Snow that's settled, a pixel each, melting from 1 to 0.
    drift: Vec<f64>,
    /// Sand's floor is open: it's running out.
    draining: bool,
    /// The pond: its fish, its lily pads (where, how big, and where the
    /// notch is), the food on the water (where each bit is, and when it
    /// fell), when a click last dropped some, and the rings where a bit
    /// was taken (where, and when).
    koi: Vec<Koi>,
    pads: Vec<(f64, f64, f64, f64)>,
    food: Vec<(f64, f64, f64)>,
    fed: f64,
    rings: Vec<(f64, f64, f64)>,
    /// Watchers' reactions, rising up the right of the screen.
    pub floats: Vec<Float>,
}

/// A reaction on its way up: what, from which column, when it came, its
/// own sway, and how fast it rises, in rows a second.
#[derive(Clone, Copy)]
pub struct Float {
    ch: char,
    x: f64,
    born: f64,
    ph: f64,
    rise: f64,
}

/// How long a reaction's on the screen, and how many can be at once.
const FLOAT_LIFE: f64 = 3.5;
const FLOATS: usize = 24;

pub const QUAD: [char; 16] = [' ', '▘', '▝', '▀', '▖', '▌', '▞', '▛', '▗', '▚', '▐', '▜', '▄', '▙', '▟', '█'];

/// How far the glow reaches, in pixels across; a pixel is twice as tall
/// as it is wide, so it reaches half as many down.
const REACH: f64 = 15.0;

fn luma(c: Rgb) -> f64 {
    0.2126 * c.0 as f64 + 0.7152 * c.1 as f64 + 0.0722 * c.2 as f64
}

fn far(a: Rgb, b: Rgb) -> i32 {
    (a.0 as i32 - b.0 as i32).abs() + (a.1 as i32 - b.1 as i32).abs() + (a.2 as i32 - b.2 as i32).abs()
}

fn mean(cs: &[Rgb]) -> Rgb {
    let n = cs.len().max(1) as u32;
    let s = cs.iter().fold((0u32, 0u32, 0u32), |s, c| (s.0 + c.0 as u32, s.1 + c.1 as u32, s.2 + c.2 as u32));
    Rgb((s.0 / n) as u8, (s.1 / n) as u8, (s.2 / n) as u8)
}

/// Four pixels (top left, top right, bottom left, bottom right) as one
/// cell: split into the brighter and the darker by the two most unlike, a
/// quadrant block in the first over the second.
pub fn quad(p: [Rgb; 4]) -> Cell {
    let mut best = (0, 0, 1);
    for i in 0..4 {
        for j in i + 1..4 {
            let d = far(p[i], p[j]);
            if d > best.0 {
                best = (d, i, j);
            }
        }
    }
    if best.0 < 6 {
        return Cell { ch: ' ', st: Style { bg: Some(mean(&p)), ..Style::default() } };
    }
    let (a, b) = if luma(p[best.1]) >= luma(p[best.2]) { (p[best.1], p[best.2]) } else { (p[best.2], p[best.1]) };
    let mut mask = 0;
    let (mut hi, mut lo) = (vec![], vec![]);
    for (k, &c) in p.iter().enumerate() {
        if far(c, a) <= far(c, b) {
            mask |= 1 << k;
            hi.push(c);
        } else {
            lo.push(c);
        }
    }
    Cell { ch: QUAD[mask], st: Style { fg: Some(mean(&hi)), bg: Some(mean(&lo)), bold: false } }
}

impl Sky {
    pub fn new(kind: Kind, glow: bool, w: i32, h: i32, seed: u32) -> Sky {
        let (pw, ph) = (w.max(1) as usize * 2, h.max(1) as usize * 2);
        let mut kernel = vec![];
        let r = REACH as i32;
        for dy in -r / 2 - 1..=r / 2 + 2 {
            for dx in -r..=r + 1 {
                // From the middle of the cell's four pixels.
                let d = ((dx as f64 - 0.5).powi(2) + (2.0 * (dy as f64 - 0.5)).powi(2)).sqrt();
                if d < REACH {
                    kernel.push((dx, dy, (1.0 - d / REACH).powi(2)));
                }
            }
        }
        let mut s = Sky {
            kind,
            glow,
            pw,
            ph,
            field: vec![Rgb::default(); pw * ph],
            px: vec![Rgb::default(); pw * ph],
            motes: vec![],
            cells: vec![false; pw * ph],
            lit: vec![0.0; pw * ph],
            beat: 0.0,
            gens: 0,
            streak: None,
            next_streak: 3.0,
            rng: Rng::new(seed),
            t: 0.0,
            kernel,
            quiet: vec![],
            frost: None,
            pointer: None,
            ripples: vec![],
            trail: vec![],
            hawks: vec![],
            many: [None; 2],
            said: String::new(),
            ants: None,
            seed,
            twin: None,
            text: vec![],
            drift: vec![0.0; pw * ph],
            draining: false,
            koi: vec![],
            pads: vec![],
            food: vec![],
            fed: f64::NEG_INFINITY,
            rings: vec![],
            floats: vec![],
        };
        s.fill();
        s
    }

    fn rand(&mut self) -> f64 {
        self.rng.below(1 << 20) as f64 / (1 << 20) as f64
    }

    /// What it starts with, spread over the whole screen, so it's already
    /// going when the slide comes.
    fn fill(&mut self) {
        if self.kind == Kind::Life {
            for i in 0..self.cells.len() {
                self.cells[i] = self.rand() < 0.16;
            }
        }
        if self.kind == Kind::Koi {
            let (w, h) = (self.pw as f64, self.ph as f64 * 2.0);
            for _ in 0..(self.pw * self.ph / 4500).clamp(2, 6) {
                let pad = (w * self.rand(), h * self.rand(), 6.0 + 5.0 * self.rand(), 6.3 * self.rand());
                self.pads.push(pad);
            }
        }
        self.crowd([None; 2]);
    }

    /// Its ant colony, taken out, to be kept while another sky's up; and
    /// one kept put back, to go on from where it was.
    pub fn take_ants(&mut self) -> Option<crate::ants::Colony> {
        self.ants.take()
    }

    /// Its ant colony, when it has one, for its keeper's keys and for
    /// saying how it's doing.
    pub fn colony(&mut self) -> Option<&mut crate::ants::Colony> {
        self.ants.as_mut()
    }

    pub fn give_ants(&mut self, mut a: crate::ants::Colony) {
        a.resize(self.pw, self.ph);
        a.cap(self.many[0]);
        a.tasks(self.many[1]);
        self.ants = Some(a);
    }

    /// How many there are when the talk doesn't say: by the screen's size.
    fn usual(&self) -> usize {
        let area = (self.pw * self.ph) as f64;
        (match self.kind {
            Kind::Stars => area / 110.0,
            Kind::Snow => self.pw as f64 / 2.5,
            Kind::Rain => self.pw as f64 / 3.0,
            Kind::Embers => self.pw as f64 / 2.5,
            Kind::Boids => (area / 160.0).max(40.0),
            Kind::Fireflies => 18.0,
            Kind::Koi => (area / 2400.0).clamp(3.0, 9.0),
            _ => 0.0,
        }) as usize
    }

    /// As many as the talk says (`sky: boids 12 3`: twelve birds, three
    /// hawks), or as many as suit the screen: more come, or the last go.
    pub fn crowd(&mut self, many: [Option<usize>; 2]) {
        self.many = many;
        if let Some(a) = self.ants.as_mut() {
            a.cap(many[0]);
            a.tasks(many[1]);
        }
        let n = many[0].filter(|_| self.usual() > 0).map_or(self.usual(), |n| n.min(MOST));
        let (fish, n) = if self.kind == Kind::Koi { (n.min(40), 0) } else { (0, n) };
        self.motes.truncate(n);
        while self.motes.len() < n {
            let m = self.spawn(true);
            self.motes.push(m);
        }
        let hawks = if self.kind == Kind::Boids { many[1].unwrap_or(2).min(MOST) } else { 0 };
        self.hawks.truncate(hawks);
        while self.hawks.len() < hawks {
            let m = self.spawn(true);
            self.hawks.push(Mote { vx: m.vx * 1.4, vy: m.vy * 1.4, ..m });
        }
        self.koi.truncate(fish);
        while self.koi.len() < fish {
            let (x, y, dir) = (self.rand() * self.pw as f64, self.rand() * self.ph as f64 * 2.0, self.rand() * 6.283);
            let (size, ph, look, side) = (0.75 + 0.45 * self.rand(), self.rand() * 6.3, self.rng.below(4) as u8, if self.rng.below(2) == 0 { 1.0 } else { -1.0 });
            // Straight out behind its head, to start.
            let spine = (0..GIRTH.len() + FIN.len()).map(|k| (x - dir.cos() * JOINT * size * k as f64, y - dir.sin() * JOINT * size * k as f64)).collect();
            self.koi.push(Koi { spine, x, y, dir, turn: 0.0, v: 9.0, beat: ph, size, ph, look, side });
        }
    }

    /// A new mote: anywhere, when filling, or where its kind comes from.
    fn spawn(&mut self, anywhere: bool) -> Mote {
        let (w, h) = (self.pw as f64, self.ph as f64);
        let (x, y, ph, tint) = (self.rand() * w, self.rand() * h, self.rand() * 6.3, self.rng.below(7) as u8);
        let r = self.rand();
        let mut m = Mote { x, y, vx: 0.0, vy: 0.0, a: 0.0, ph, age: 0.0, life: f64::INFINITY, tint };
        match self.kind {
            Kind::Stars => {
                // Three depths: the far ones fainter and slower.
                let z = [0.35, 0.6, 1.0][self.rng.below(3) as usize];
                (m.vx, m.a) = (-0.9 * z, 0.18 + 0.5 * z);
                if !anywhere {
                    m.x = w + 1.0;
                }
            }
            Kind::Snow => {
                (m.vy, m.a) = (2.5 + 5.0 * r, 0.25 + 0.4 * r);
                if !anywhere {
                    m.y = -1.0;
                }
            }
            Kind::Rain => {
                (m.vy, m.a) = (28.0 + 30.0 * r, 0.3 + 0.35 * r);
                if !anywhere {
                    m.y = -self.rand() * h * 0.5;
                }
            }
            Kind::Boids => {
                let (ang, v) = (self.rand() * 6.283, 10.0 + 5.0 * r);
                (m.vx, m.vy, m.a) = (v * ang.cos(), v * ang.sin() / 2.0, 0.55 + 0.25 * r);
            }
            Kind::Fireflies => m.a = 0.7 + 0.3 * r,
            Kind::Embers => {
                (m.vy, m.a, m.life) = (-(3.0 + 7.0 * r), 0.55 + 0.45 * r, 4.0 + 6.0 * self.rand());
                m.age = if anywhere { self.rand() * m.life } else { 0.0 };
                m.y = if anywhere { h - m.age * -m.vy } else { h + 1.0 };
            }
            _ => {}
        }
        m
    }

    /// A reaction, rising from the bottom right; past as many as there's
    /// room for, it's let go.
    pub fn float(&mut self, ch: char) {
        let cols = (self.pw / 2) as f64;
        if self.floats.len() >= FLOATS || cols < 20.0 {
            return;
        }
        let (x, ph, r) = (cols - 5.0 - self.rand() * 12.0, self.rand() * 6.3, self.rand());
        let rise = (self.ph / 2) as f64 * (0.45 + 0.2 * r) / FLOAT_LIFE;
        self.floats.push(Float { ch, x, born: self.t, ph, rise });
    }

    /// Where a reaction is at time t, its row and column from 0, or None
    /// when it's gone.
    fn at(&self, f: &Float) -> Option<(usize, usize)> {
        let age = self.t - f.born;
        let (rows, cols) = ((self.ph / 2) as f64, (self.pw / 2) as f64);
        let (y, x) = ((rows - 2.0 - age * f.rise).round(), (f.x + 1.5 * (age * 2.5 + f.ph).sin()).round());
        (age < FLOAT_LIFE && y >= 0.0 && x >= 0.0 && x + 1.0 < cols).then_some((y as usize, x as usize))
    }

    /// On to time t, in seconds: everything moved, then the pixels drawn
    /// again, the glow from the headline's letters among `front`.
    pub fn frame(&mut self, t: f64, theme: &Theme, front: &[Option<Cell>], w: usize) {
        let dt = (t - self.t).clamp(0.0, 0.1);
        self.t = t;
        self.floats.retain(|f| t - f.born < FLOAT_LIFE);
        // Where text is first, for what steers around it.
        self.hush(front, w);
        self.step(dt, t);
        let base = if self.kind == Kind::Koi { theme.bg.mix(DEEP, 0.45) } else { theme.bg };
        self.field.fill(base);
        if self.glow {
            self.shine(t, base, theme.accent, front, w);
        }
        if self.kind == Kind::Koi {
            self.water(t);
        }
        if self.kind == Kind::Ants {
            let (pw, ph, seed, said, cap) = (self.pw, self.ph, self.seed, self.said.clone(), self.many[0]);
            let colony = said.split_whitespace().filter(|w| *w != "farm" && *w != "ground").collect::<Vec<_>>().join(" ");
            let a = self.ants.get_or_insert_with(|| {
                // As it was left last time, if it was kept.
                let mut c = crate::ants::Colony::new(pw, ph, seed, &said, cap);
                if let Some(b) = crate::twin::saved(&colony)
                    && c.restore(&b)
                {
                    // It's lived on while deque wasn't running.
                    c.cap(cap);
                    c.catch_up(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64()));
                }
                c
            });
            a.tasks(self.many[1]);
            let top = said.split_whitespace().any(|w| w == "ground");
            a.view(top);
            // A click drops food.
            let clicks: Vec<(f64, f64)> = self.ripples.iter().filter(|r| r.2 > self.fed).map(|r| (r.0, r.1)).collect();
            self.fed = self.ripples.iter().fold(self.fed, |m, r| m.max(r.2));
            // Another deque showing this talk has the same colony: one of
            // them keeps it, and the other shows what it's told, and passes
            // on what's dropped in it.
            let tw = self.twin.get_or_insert_with(|| crate::twin::Twin::new(&colony));
            if tw.as_mut().is_some_and(|tw| !tw.owns()) {
                let (tw, (w, h)) = (tw.as_mut().unwrap(), a.size());
                tw.follow(a);
                // A click on an ant follows it, here; elsewhere it's food,
                // for the keeper to drop.
                for c in &clicks {
                    if !a.pick(c.0, c.1) {
                        tw.feed(c.0 / w as f64, c.1 / h as f64, top);
                    }
                }
            } else {
                for (x, y, top) in tw.as_ref().map(|tw| tw.fed()).unwrap_or_default() {
                    a.feed(x, y, top);
                }
                // As fast as its keeper has its time going.
                for k in 0..a.speed() {
                    a.step(dt, t, &self.text, if k == 0 { &clicks } else { &[] });
                }
                if let Some(tw) = tw {
                    tw.publish(a);
                    tw.save(a);
                }
            }
            a.back(&mut self.field, theme);
        }
        if let Some(f) = &self.frost {
            for (p, (c, k)) in self.field.iter_mut().zip(f) {
                *p = p.mix(*c, *k);
            }
        }
        self.point(t, theme);
        if self.kind == Kind::Fireflies {
            self.lanterns(t, theme);
        }
        self.px.copy_from_slice(&self.field);
        self.draw(t, theme);
        self.laser(t, theme);
        self.quiet.clear();
        if self.frost.is_some() {
            self.soften();
        }
    }

    /// Behind frost, what moves is blurred too: what the motes added to
    /// the field, spread out, and brighter for it, so they don't vanish.
    fn soften(&mut self) {
        let mut d: Vec<[f64; 4]> = self
            .px
            .iter()
            .zip(&self.field)
            .map(|(p, f)| [0.0, p.0 as f64 - f.0 as f64, p.1 as f64 - f.1 as f64, p.2 as f64 - f.2 as f64])
            .collect();
        for _ in 0..2 {
            blur(&mut d, self.pw, self.ph, 1, 1);
        }
        blur(&mut d, self.pw, self.ph, 1, self.pw);
        for (p, (f, d)) in self.px.iter_mut().zip(self.field.iter().zip(&d)) {
            let c = |f: u8, d: f64| (f as f64 + d * 2.6).clamp(0.0, 255.0) as u8;
            *p = Rgb(c(f.0, d[1]), c(f.1, d[2]), c(f.2, d[3]));
        }
    }

    /// What's on the screen now, frozen and blurred, to stay behind what's
    /// drawn next: blocks at full strength, letters as a haze, each in its
    /// own color.
    pub fn freeze(&mut self, front: &[Option<Cell>], w: usize, th: &Theme) {
        let mut ink = vec![[0.0f64; 4]; self.pw * self.ph];
        for (i, c) in front.iter().enumerate() {
            let Some(c) = c else { continue };
            let k = match QUAD.iter().position(|&q| q == c.ch) {
                Some(m) => m.count_ones() as f64 / 4.0,
                None if c.ch == ' ' => 0.0,
                None if ('\u{2500}'..='\u{257f}').contains(&c.ch) => 0.15,
                None => 0.3,
            };
            let col = c.st.fg.unwrap_or(th.fg);
            let (x, y) = ((i % w) * 2, (i / w) * 2);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                if x + dx < self.pw && y + dy < self.ph {
                    ink[(y + dy) * self.pw + x + dx] = [k, k * col.0 as f64, k * col.1 as f64, k * col.2 as f64];
                }
            }
        }
        // Twice a box across and down, near enough a gaussian; half as far
        // down, a pixel being twice as tall as wide.
        for _ in 0..2 {
            blur(&mut ink, self.pw, self.ph, 5, 1);
            blur(&mut ink, self.pw, self.ph, 3, self.pw);
        }
        self.frost = Some(
            ink.iter()
                .map(|a| match a[0] > 1e-3 {
                    true => (Rgb((a[1] / a[0]) as u8, (a[2] / a[0]) as u8, (a[3] / a[0]) as u8), (a[0] * 1.3).min(1.0) * 0.42),
                    false => (Rgb::default(), 0.0),
                })
                .collect(),
        );
    }

    pub fn thaw(&mut self) {
        self.frost = None;
    }

    /// The frost now, taken off.
    pub fn take_frost(&mut self) -> Option<Vec<(Rgb, f64)>> {
        self.frost.take()
    }

    /// Frost of its own, as take_frost gave it, maybe mixed.
    pub fn set_frost(&mut self, f: Vec<(Rgb, f64)>) {
        self.frost = Some(f);
    }

    /// Where text is, and a cell round it: the sky goes faint there, so
    /// nothing moving crosses a letter.
    fn hush(&mut self, front: &[Option<Cell>], w: usize) {
        let h = front.len() / w.max(1);
        self.quiet = vec![false; front.len()];
        self.text = front.iter().map(Option::is_some).collect();
        for (i, _) in front.iter().enumerate().filter(|(_, f)| f.is_some()) {
            let (r, c) = ((i / w) as i32, (i % w) as i32);
            for dr in -1..=1 {
                for dc in -1..=1 {
                    let (r, c) = (r + dr, c + dc);
                    if r >= 0 && c >= 0 && (r as usize) < h && (c as usize) < w {
                        self.quiet[r as usize * w + c as usize] = true;
                    }
                }
            }
        }
    }

    fn step(&mut self, dt: f64, t: f64) {
        let (w, h) = (self.pw as f64, self.ph as f64);
        for i in 0..self.motes.len() {
            let mut m = self.motes[i];
            m.x += m.vx * dt;
            m.y += m.vy * dt;
            m.age += dt;
            let gone = match self.kind {
                Kind::Stars => m.x < -1.0,
                Kind::Snow => m.y > h + 1.0,
                Kind::Rain => m.y > h + 8.0,
                Kind::Embers => m.age > m.life || m.y < -1.0,
                _ => false,
            };
            // Snow comes to rest on the text it falls onto, where it's drawn.
            let landed = self.kind == Kind::Snow && self.lands(m.x + 1.6 * (t * 0.9 + m.ph).sin(), m.y);
            if gone || landed {
                m = self.spawn(false);
            }
            self.motes[i] = m;
        }
        match self.kind {
            Kind::Boids => self.flock(dt),
            Kind::Koi => self.swim(dt, t),
            Kind::Snow => {
                // What's settled melts in its time, and goes at once when
                // what it lay on does.
                for y in (0..self.ph).rev() {
                    for x in 0..self.pw {
                        let i = y * self.pw + x;
                        if self.drift[i] > 0.0 {
                            let (x, y) = (x as i32, y as i32);
                            let held = !self.solid(x, y) && (self.solid(x, y + 1) || self.drift.get(i + self.pw).is_some_and(|d| *d > 0.0));
                            self.drift[i] = if held { (self.drift[i] - dt / 30.0).max(0.0) } else { 0.0 };
                        }
                    }
                }
            }
            Kind::Sand => {
                while t - self.beat >= 0.04 {
                    self.beat = if t - self.beat > 1.0 { t } else { self.beat + 0.04 };
                    self.pour(t);
                }
            }
            Kind::Fireflies => {
                // Each wanders on its own slow curve, round the edges.
                for m in &mut self.motes {
                    let ang = m.ph + 1.4 * (t * 0.31 + m.ph * 2.0).sin() + 0.9 * (t * 0.67 + m.ph).sin();
                    m.x = (m.x + 3.0 * ang.cos() * dt).rem_euclid(w);
                    m.y = (m.y + 1.5 * ang.sin() * dt).rem_euclid(h);
                }
            }
            _ => {}
        }
        if self.kind == Kind::Stars {
            self.next_streak -= dt;
            if self.streak.is_none() && self.next_streak <= 0.0 {
                let (x, y) = (w * (0.3 + 0.7 * self.rand()), h * 0.4 * self.rand());
                self.streak = Some(Mote { x, y, vx: -95.0, vy: 26.0, a: 0.9, ph: 0.0, age: 0.0, life: 0.8, tint: 0 });
                self.next_streak = 5.0 + 7.0 * self.rand();
            }
            if let Some(s) = self.streak.as_mut() {
                (s.x, s.y, s.age) = (s.x + s.vx * dt, s.y + s.vy * dt, s.age + dt);
                if s.age > s.life {
                    self.streak = None;
                }
            }
        }
        if self.kind == Kind::Life {
            while t - self.beat >= 0.16 {
                self.beat = if t - self.beat > 1.0 { t } else { self.beat + 0.16 };
                self.generation();
            }
            let k = (dt * 7.0).min(1.0);
            for i in 0..self.lit.len() {
                let to = if self.cells[i] { 1.0 } else { 0.0 };
                self.lit[i] += (to - self.lit[i]) * k;
            }
        }
    }

    /// Whether the pixel's in a cell with text in it.
    fn solid(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.pw && self.text.get(y as usize / 2 * (self.pw / 2) + x as usize / 2).is_some_and(|t| *t)
    }

    /// A snowflake at (x, y) settling there, if it's just over text, or
    /// over a flake that is: two deep at most.
    fn lands(&mut self, x: f64, y: f64) -> bool {
        let (x, y) = (x.round() as i32, y.round() as i32);
        if x < 0 || y < 0 || x as usize >= self.pw || y as usize + 2 >= self.ph || self.solid(x, y) {
            return false;
        }
        let i = y as usize * self.pw + x as usize;
        let on = self.solid(x, y + 1) || (self.drift[i + self.pw] > 0.0 && self.solid(x, y + 2));
        if on && self.drift[i] == 0.0 {
            self.drift[i] = 1.0;
        }
        on && self.drift[i] == 1.0
    }

    /// Whether a grain of sand could be at the pixel.
    fn free(&self, x: i32, y: i32) -> bool {
        x >= 0 && (x as usize) < self.pw && (y as usize) < self.ph && !self.cells[y as usize * self.pw + x as usize] && !self.solid(x, y)
    }

    /// Sand a step on: more poured in at the top from three spouts that
    /// wander, each grain falling till it lies on text, the floor or other
    /// sand, and sliding off a heap too steep. Deep enough, the floor
    /// opens and it runs out.
    fn pour(&mut self, t: f64) {
        let (w, h) = (self.pw, self.ph);
        for y in (0..h - 1).rev() {
            for x in 0..w {
                let i = y * w + x;
                if !self.cells[i] {
                    continue;
                }
                // Text came where it lay.
                if self.solid(x as i32, y as i32) {
                    self.cells[i] = false;
                    continue;
                }
                let side = if self.rng.below(2) == 0 { 1 } else { -1 };
                let to = [0, side, -side].into_iter().map(|d| x as i32 + d).find(|&nx| self.free(nx, y as i32 + 1));
                if let Some(nx) = to {
                    self.cells[i] = false;
                    self.cells[(y + 1) * w + nx as usize] = true;
                }
            }
        }
        let floor = (h - 1) * w;
        if self.cells.iter().filter(|c| **c).count() > w * h / 7 {
            self.draining = true;
        }
        if self.draining {
            self.draining = self.cells[floor..].iter().any(|c| *c);
            self.cells[floor..].fill(false);
            return;
        }
        for k in 0..3 {
            let k = k as f64;
            let x = (w as f64 * (0.5 + 0.45 * (t * (0.05 + 0.03 * k) + 2.1 * k).sin())) as i32 + self.rng.below(3);
            if self.free(x, 0) {
                self.cells[x as usize] = true;
            }
        }
    }

    /// The pond's fish a moment on. Each wanders, turning off from the
    /// edges and from text. A click scatters food on the water: each fish
    /// makes for the bit nearest it, slowing as it comes up to it, and
    /// takes it, a ring going out from where it was. Its head sways as its tail beats, and each joint of its back
    /// follows the one before, so the sway runs down it.
    fn swim(&mut self, dt: f64, t: f64) {
        let (w, h) = (self.pw as f64, self.ph as f64 * 2.0);
        for (x, y, at) in self.ripples.clone() {
            if at > self.fed {
                self.fed = at;
                for _ in 0..6 {
                    let bit = (x + 9.0 * (self.rand() - 0.5), y * 2.0 + 9.0 * (self.rand() - 0.5), t);
                    if self.food.len() < MOST_FOOD {
                        self.food.push(bit);
                    }
                }
            }
        }
        self.food.retain(|f| t - f.2 < FLOATS_FOR);
        self.rings.retain(|r| t - r.2 < 1.0);
        let (cols, quiet) = (self.pw / 2, &self.quiet);
        let near_text = |x: f64, y: f64| x >= 0.0 && y >= 0.0 && x < w && y < h && quiet.get(y as usize / 4 * cols + x as usize / 2).is_some_and(|q| *q);
        let heads: Vec<(f64, f64)> = self.koi.iter().map(|k| (k.x, k.y)).collect();
        for (i, k) in self.koi.iter_mut().enumerate() {
            let mut want = 0.25 * (t * 0.23 + k.ph).sin() + 0.15 * (t * 0.61 + k.ph * 2.3).sin();
            let mut speed = (8.5 + 3.0 * (t * 0.3 + k.ph).sin()) * k.size;
            // Near an edge, or past it: round toward the middle.
            let edge = 30.0;
            if k.x < edge || k.y < edge || k.x > w - edge || k.y > h - edge {
                want += 0.8 * veer(k.dir, (h / 2.0 - k.y).atan2(w / 2.0 - k.x));
            }
            // Text ahead, near or further: off to its own side of it.
            if [22.0, 44.0].iter().any(|d| near_text(k.x + k.dir.cos() * d, k.y + k.dir.sin() * d)) {
                want += 0.9 * k.side;
            }
            // Another's head close by: away from it.
            for (j, o) in heads.iter().enumerate() {
                if i != j && (o.0 - k.x).powi(2) + (o.1 - k.y).powi(2) < 28.0 * 28.0 {
                    want -= 0.4 * veer(k.dir, (o.1 - k.y).atan2(o.0 - k.x)).signum();
                }
            }
            let far = |f: &(f64, f64, f64)| (f.0 - k.x).powi(2) + (f.1 - k.y).powi(2);
            let mut most = 0.8;
            if let Some(b) = (0..self.food.len()).min_by(|&a, &b| far(&self.food[a]).total_cmp(&far(&self.food[b]))) {
                let (fx, fy, _) = self.food[b];
                let d = far(&self.food[b]).sqrt();
                if d < 5.0 * k.size {
                    self.food.swap_remove(b);
                    self.rings.push((fx, fy, t));
                } else {
                    // Quick to it from far off; close, slow, and turning
                    // tighter, so it doesn't go round and round it.
                    want += 1.6 * veer(k.dir, (fy - k.y).atan2(fx - k.x));
                    speed *= 0.6 + (d / 25.0).min(1.2);
                    most = if d < 30.0 { 1.8 } else { 0.8 };
                }
            }
            // Eased, so it never jerks round or lurches off.
            k.turn += (want.clamp(-most, most) - k.turn) * (dt * 2.0).min(1.0);
            k.v += (speed - k.v) * (dt * 1.5).min(1.0);
            k.dir += k.turn * dt;
            k.x += k.dir.cos() * k.v * dt;
            k.y += k.dir.sin() * k.v * dt;
            k.beat += (2.0 + k.v * 0.3) * dt;
            let sway = 1.5 * k.size * k.beat.sin();
            k.spine[0] = (k.x - k.dir.sin() * sway, k.y + k.dir.cos() * sway);
            for i in 1..k.spine.len() {
                let (a, b) = (k.spine[i - 1], k.spine[i]);
                let d = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt().max(1e-6);
                let len = JOINT * k.size;
                k.spine[i] = (a.0 + (b.0 - a.0) / d * len, a.1 + (b.1 - a.1) / d * len);
            }
        }
    }

    /// Light on the water, into the field: slow bands crossing, in a few
    /// steps of brightness, so the cells they light change now and then,
    /// not every frame.
    fn water(&mut self, t: f64) {
        for y in 0..self.ph {
            let yy = y as f64 * 2.0;
            for x in 0..self.pw {
                let xx = x as f64;
                let v = (xx * 0.11 + t * 0.35 + 1.7 * (yy * 0.07 + t * 0.21).sin()).sin() + (yy * 0.09 - t * 0.27 + 1.3 * (xx * 0.05 - t * 0.17).sin()).sin();
                let k = ((v * 0.5).max(0.0).powi(2) * 4.0).round() / 4.0;
                if k > 0.0 {
                    let p = &mut self.field[y * self.pw + x];
                    *p = p.mix(LIGHT, 0.1 * k);
                }
            }
        }
    }

    /// The koi and the lily pads over them. A fish is every pixel near
    /// enough its spine, as near as it's wide there, its edge shared with
    /// the water where it only half covers a pixel; rounder for being
    /// darker toward its sides. Its fins are see-through: one each side
    /// behind its head, swept back, and a tail that fans out and forks.
    fn pond(&mut self, t: f64) {
        // How far a point is outside a stretch from a to b, as wide as ra
        // at a and rb at b, with how far along it is, and how far out to
        // which side, 1 at its edge. Past its ends there's none of it,
        // unless it's `round` there.
        let stretch = |q: (f64, f64), a: (f64, f64), b: (f64, f64), ra: f64, rb: f64, round: bool| {
            let (dx, dy) = (b.0 - a.0, b.1 - a.1);
            let u = ((q.0 - a.0) * dx + (q.1 - a.1) * dy) / (dx * dx + dy * dy).max(1e-9);
            if !round && !(0.0..=1.0).contains(&u) {
                return (f64::MAX, u, 0.0);
            }
            let u = u.clamp(0.0, 1.0);
            let (ox, oy) = (q.0 - a.0 - dx * u, q.1 - a.1 - dy * u);
            let (d, r) = ((ox * ox + oy * oy).sqrt(), (ra + (rb - ra) * u).max(0.1));
            (d - r, u, (dx * oy - dy * ox).signum() * d / r)
        };
        let fish = std::mem::take(&mut self.koi);
        for k in &fish {
            let girth: Vec<f64> = GIRTH.iter().chain(&FIN).map(|g| g * k.size).collect();
            let body = GIRTH.len() - 1;
            let (c1, c2, much) = COATS[k.look as usize % COATS.len()];
            // Its side fins: from its sides out and back.
            let (s2, s3) = (k.spine[2], k.spine[3]);
            let (bx, by) = ((s3.0 - s2.0) / (JOINT * k.size), (s3.1 - s2.1) / (JOINT * k.size));
            let g = girth[2];
            let fins: Vec<((f64, f64), (f64, f64))> = [1.0, -1.0]
                .iter()
                .map(|sd| {
                    let (ox, oy) = (-by * sd, bx * sd);
                    ((s2.0 + ox * g * 0.8, s2.1 + oy * g * 0.8), (s2.0 + (ox * 1.7 + bx * 1.3) * g, s2.1 + (oy * 1.7 + by * 1.3) * g))
                })
                .collect();
            let reach = g * 3.0 + 1.0;
            let (x0, x1) = k.spine.iter().fold((f64::MAX, f64::MIN), |m, p| (m.0.min(p.0), m.1.max(p.0)));
            let (y0, y1) = k.spine.iter().fold((f64::MAX, f64::MIN), |m, p| (m.0.min(p.1), m.1.max(p.1)));
            for py in ((y0 - reach) / 2.0).floor().max(0.0) as usize..=(((y1 + reach) / 2.0).ceil().max(0.0) as usize).min(self.ph.saturating_sub(1)) {
                for px in (x0 - reach).floor().max(0.0) as usize..=((x1 + reach).ceil().max(0.0) as usize).min(self.pw.saturating_sub(1)) {
                    // Looked at in two places, upper and lower, a pixel
                    // being tall.
                    let (mut cover, mut sheer, mut col, mut shade) = (0.0, 0.0, c1, 0.0);
                    for sy in [0.5, 1.5] {
                        let q = (px as f64 + 0.5, py as f64 * 2.0 + sy);
                        let mut best = (f64::MAX, 0.0, 0.0);
                        for i in 0..body {
                            let (d, u, side) = stretch(q, k.spine[i], k.spine[i + 1], girth[i], girth[i + 1], true);
                            if d < best.0 {
                                best = (d, (i as f64 + u) / body as f64, side);
                            }
                        }
                        let c = (0.6 - best.0).clamp(0.0, 1.0);
                        if c > 0.0 {
                            cover += c * 0.5;
                            // Its markings: patches along it and across.
                            let v = (best.1 * 5.0 + k.ph * 3.0).sin() * (best.2 * 1.3 + k.ph * 5.0).cos() + 0.3 * (best.1 * 11.0 + k.ph * 7.0).sin();
                            col = if v > much { c2 } else { c1 };
                            shade = best.2.abs().min(1.0);
                        }
                        let mut fin: f64 = 0.0;
                        for i in body..k.spine.len() - 1 {
                            let (d, u, side) = stretch(q, k.spine[i], k.spine[i + 1], girth[i], girth[i + 1], false);
                            // The fork: a notch up the middle of its end.
                            let fork = i + 2 == k.spine.len() && side.abs() < 0.45 * u;
                            if !fork {
                                fin = fin.max((0.6 - d).clamp(0.0, 1.0));
                            }
                        }
                        for &(a, b) in &fins {
                            fin = fin.max((0.6 - stretch(q, a, b, g * 0.5, g * 0.2, true).0).clamp(0.0, 1.0));
                        }
                        sheer += fin * 0.5;
                    }
                    let p = &mut self.px[py * self.pw + px];
                    *p = p.mix(c1.mix(c2, 0.3), sheer * 0.45);
                    *p = p.mix(col.mix(DEEP, 0.35 * shade * shade), cover * 0.95);
                }
            }
        }
        self.koi = fish;
        // Food, afloat, each bit bobbing, fading as it's about to go; and
        // a ring widening where one was taken.
        for (x, y, at) in self.food.clone() {
            let fade = ((FLOATS_FOR - (t - at)) / 3.0).clamp(0.0, 1.0) * (0.8 + 0.2 * (t * 2.0 + x).sin());
            self.mark(x, y / 2.0, FOOD, fade);
        }
        for (x, y, at) in self.rings.clone() {
            let (age, n) = (t - at, 28);
            for k in 0..n {
                let a = k as f64 / n as f64 * std::f64::consts::TAU;
                let r = 2.0 + 9.0 * age;
                self.mark(x + r * a.cos(), (y + r * a.sin()) / 2.0, LIGHT, 0.5 * (1.0 - age));
            }
        }
        for &(x, y, r, notch) in &self.pads.clone() {
            // Each rides the water a little, in its own time.
            let (x, y) = (x + 0.8 * (t * 0.21 + notch).sin(), y + 0.8 * (t * 0.17 + notch * 2.0).cos());
            for py in ((y - r) / 2.0).floor().max(0.0) as usize..=(((y + r) / 2.0).ceil().max(0.0) as usize).min(self.ph.saturating_sub(1)) {
                for px in (x - r).floor().max(0.0) as usize..=((x + r).ceil().max(0.0) as usize).min(self.pw.saturating_sub(1)) {
                    let mut cover = 0.0;
                    let mut rim = 0.0;
                    for sy in [0.5, 1.5] {
                        let (dx, dy) = (px as f64 + 0.5 - x, py as f64 * 2.0 + sy - y);
                        let d = (dx * dx + dy * dy).sqrt();
                        // The notch: a wedge cut in to near the middle.
                        if d > r * 0.25 && veer(notch, dy.atan2(dx)).abs() < 0.3 {
                            continue;
                        }
                        cover += (r - d + 0.5).clamp(0.0, 1.0) * 0.5;
                        rim += if d > r - 1.2 { 0.5 } else { 0.0 };
                    }
                    let p = &mut self.px[py * self.pw + px];
                    *p = p.mix(PAD.mix(DEEP, 0.35 * rim), cover * 0.9);
                }
            }
        }
    }

    /// Conway's rules, the edges wrapping round; a few new cells scattered
    /// in now and then so it never settles.
    fn generation(&mut self) {
        let (w, h) = (self.pw, self.ph);
        let mut next = vec![false; w * h];
        let mut alive = 0;
        for y in 0..h {
            for x in 0..w {
                let mut n = 0;
                for (dx, dy) in [(w - 1, h - 1), (0, h - 1), (1, h - 1), (w - 1, 0), (1, 0), (w - 1, 1), (0, 1), (1, 1)] {
                    n += self.cells[(y + dy) % h * w + (x + dx) % w] as u8;
                }
                let on = matches!((self.cells[y * w + x], n), (true, 2 | 3) | (false, 3));
                next[y * w + x] = on;
                alive += on as usize;
            }
        }
        self.cells = next;
        self.gens += 1;
        if self.gens.is_multiple_of(40) || alive < w * h / 60 {
            let (cx, cy) = (self.rng.below(w as i32) as usize, self.rng.below(h as i32) as usize);
            for _ in 0..60 {
                let (x, y) = ((cx + self.rng.below(12) as usize) % w, (cy + self.rng.below(8) as usize) % h);
                self.cells[y * w + x] = true;
            }
        }
    }

    /// The glow: each headline block lights the pixels around it in its
    /// own color, the light adding up and easing off, breathing slowly.
    fn shine(&mut self, t: f64, base: Rgb, accent: Rgb, front: &[Option<Cell>], w: usize) {
        let mut sum = vec![(0.0f64, 0.0f64, 0.0f64, 0.0f64); self.field.len()];
        let mut any = false;
        for (i, c) in front.iter().enumerate() {
            // Blocks give off light as much of the cell as they fill.
            let Some((c, lit)) = c.and_then(|c| Some((c, QUAD.iter().position(|&q| q == c.ch)?))) else { continue };
            if lit == 0 {
                continue;
            }
            any = true;
            let cover = lit.count_ones() as f64 / 4.0;
            let col = c.st.fg.unwrap_or(accent);
            let (px, py) = ((i % w) as i32 * 2, (i / w) as i32 * 2);
            for &(dx, dy, k) in &self.kernel {
                let (x, y) = (px + dx, py + dy);
                if x < 0 || y < 0 || x >= self.pw as i32 || y >= self.ph as i32 {
                    continue;
                }
                let s = &mut sum[y as usize * self.pw + x as usize];
                let k = k * cover;
                s.0 += k;
                s.1 += k * col.0 as f64;
                s.2 += k * col.1 as f64;
                s.3 += k * col.2 as f64;
            }
        }
        if !any {
            return;
        }
        // In steps, so the cells it lights change a few times a second, not
        // every frame.
        let breath = ((0.8 + 0.2 * (t * 1.6).sin()) * 32.0).round() / 32.0;
        for (p, s) in self.field.iter_mut().zip(&sum) {
            if s.0 > 0.0 {
                let c = Rgb((s.1 / s.0) as u8, (s.2 / s.0) as u8, (s.3 / s.0) as u8);
                *p = base.mix(c, (1.0 - (-s.0 * 0.06).exp()) * 0.75 * breath);
            }
        }
    }

    /// Boids: each keeps near the others, goes their way, keeps its
    /// distance, and turns off before it reaches text.
    fn flock(&mut self, dt: f64) {
        let (w, h) = (self.pw as f64, self.ph as f64);
        let all = self.motes.clone();
        // The hunt: each hawk turns toward the bird nearest it, faster than
        // a bird cruises, slower than one fleeing.
        for k in 0..self.hawks.len() {
            let mut h2 = self.hawks[k];
            let near = all.iter().min_by(|a, b| {
                let d = |o: &Mote| (o.x - h2.x).powi(2) + ((o.y - h2.y) * 2.0).powi(2);
                d(a).total_cmp(&d(b))
            });
            if let Some(o) = near {
                h2.vx += (o.x - h2.x) * 1.6 * dt;
                h2.vy += (o.y - h2.y) * 1.6 * dt;
            }
            let v = (h2.vx * h2.vx + h2.vy * h2.vy * 4.0).sqrt().max(0.1);
            let k2 = v.clamp(12.0, 24.0) / v;
            (h2.vx, h2.vy) = (h2.vx * k2, h2.vy * k2);
            h2.x = (h2.x + h2.vx * dt).rem_euclid(w);
            h2.y = (h2.y + h2.vy * dt).rem_euclid(h);
            self.hawks[k] = h2;
        }
        // What to scatter from: the hawks, and the pointer while it's in use.
        let mut threats: Vec<(f64, f64)> = self.hawks.iter().map(|m| (m.x, m.y)).collect();
        threats.extend(self.pointer.filter(|p| self.t - p.2 < 2.0).map(|p| (p.0, p.1)));
        let hawk = !threats.is_empty();
        let (cols, text) = (self.pw / 2, &self.text);
        let on_text = |x: f64, y: f64| x >= 0.0 && y >= 0.0 && x < w && y < h && text.get(y as usize / 2 * cols + x as usize / 2).is_some_and(|t| *t);
        for (i, m) in self.motes.iter_mut().enumerate() {
            // Perched: still, till it's had enough, something comes at it,
            // or the text it's on goes.
            if m.life.is_finite() {
                let scared = threats.iter().any(|&(px, py)| ((m.x - px).powi(2) + ((m.y - py) * 2.0).powi(2)).sqrt() < 14.0);
                if m.age > m.life || scared || !on_text(m.x, m.y + 1.0) {
                    (m.vx, m.vy, m.life, m.age) = (9.0 * (m.ph * 7.0 + m.x).cos(), -5.0, f64::INFINITY, 0.0);
                }
                continue;
            }
            let (mut cx, mut cy, mut ax, mut ay, mut sx, mut sy, mut n) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            for (j, o) in all.iter().enumerate() {
                let (dx, dy) = (o.x - m.x, (o.y - m.y) * 2.0);
                let d = (dx * dx + dy * dy).sqrt();
                if i == j || d > 16.0 || o.life.is_finite() {
                    continue;
                }
                (cx, cy, ax, ay, n) = (cx + dx, cy + dy, ax + o.vx, ay + o.vy, n + 1.0);
                if d < 5.0 {
                    (sx, sy) = (sx - dx / d.max(0.5), sy - dy / d.max(0.5));
                }
            }
            if n > 0.0 {
                m.vx += (cx / n * 0.5 + (ax / n - m.vx) * 0.9 + sx * 6.0) * dt;
                m.vy += (cy / n * 0.25 + (ay / n - m.vy) * 0.9 + sy * 3.0) * dt;
            }
            let mut fleeing = false;
            for &(px, py) in &threats {
                let (dx, dy) = (m.x - px, (m.y - py) * 2.0);
                let d = (dx * dx + dy * dy).sqrt();
                if d < 22.0 {
                    let push = 260.0 * (1.0 - d / 22.0) / d.max(1.0);
                    m.vx += dx * push * dt;
                    m.vy += dy / 2.0 * push * dt;
                    fleeing = true;
                }
            }
            // Just over a line, and nothing after it: now and then it
            // lands, for a few seconds.
            let ledge = [0.0, 2.0].into_iter().find(|d| !on_text(m.x, m.y + d) && on_text(m.x, m.y + d + 2.0));
            if let Some(d) = ledge.filter(|_| !fleeing && m.age > 5.0)
                && self.rng.below(1000) < (dt * 500.0) as i32
            {
                (m.vx, m.vy, m.y, m.life, m.age) = (0.0, 0.0, ((m.y + d) / 2.0).floor() * 2.0 + 1.0, 3.0 + m.ph, 0.0);
                continue;
            }
            // Text ahead: turn to the left of the way it's going.
            let (fx, fy) = (m.x + m.vx * 0.6, m.y + m.vy * 0.6);
            let cell = |x: f64, y: f64| (y.rem_euclid(h) as usize / 2) * (self.pw / 2) + x.rem_euclid(w) as usize / 2;
            if self.quiet.get(cell(fx, fy)).is_some_and(|q| *q) {
                (m.vx, m.vy) = (m.vx * 0.85 + m.vy * 2.0 * 0.5, m.vy * 0.85 - m.vx / 2.0 * 0.5);
            }
            let v = (m.vx * m.vx + m.vy * m.vy * 4.0).sqrt().max(0.1);
            // Faster when scattering, back to a cruise after.
            let k = v.clamp(8.0, if hawk && fleeing { 34.0 } else { 18.0 }) / v;
            (m.vx, m.vy) = (m.vx * k, m.vy * k);
            m.x = (m.x + m.vx * dt).rem_euclid(w);
            m.y = (m.y + m.vy * dt).rem_euclid(h);
        }
    }

    /// How lit a firefly is: mostly dim, now and then bright.
    fn lit(m: &Mote, t: f64) -> f64 {
        (0.5 + 0.5 * (t * (0.6 + m.ph * 0.15) + m.ph * 3.0).sin()).powi(3)
    }

    /// Each firefly's light on what's around it, into the field.
    fn lanterns(&mut self, t: f64, th: &Theme) {
        let c = th.warm.mix(th.accent, 0.5);
        let r = 10.0;
        for m in self.motes.clone() {
            let b = Self::lit(&m, t) * m.a;
            if b < 0.05 {
                continue;
            }
            for py in (m.y - r / 2.0).floor() as i32..=(m.y + r / 2.0).ceil() as i32 {
                for px in (m.x - r).floor() as i32..=(m.x + r).ceil() as i32 {
                    if px < 0 || py < 0 || px >= self.pw as i32 || py >= self.ph as i32 {
                        continue;
                    }
                    let d = ((px as f64 - m.x).powi(2) + (2.0 * (py as f64 - m.y)).powi(2)).sqrt() / r;
                    if d < 1.0 {
                        let p = &mut self.field[py as usize * self.pw + px as usize];
                        *p = p.mix(c, (1.0 - d).powi(2) * 0.42 * b);
                    }
                }
            }
        }
    }

    /// The light around the pointer, into the field, so the text it's
    /// over is lit too; gone a second after it stops.
    fn point(&mut self, t: f64, th: &Theme) {
        let Some((x, y, at)) = self.pointer else { return };
        let fade = 1.0 - ((t - at - 1.2) / 0.8).clamp(0.0, 1.0);
        if fade <= 0.0 {
            return;
        }
        let r = 10.0;
        for py in (y - r / 2.0).floor() as i32..=(y + r / 2.0).ceil() as i32 {
            for px in (x - r).floor() as i32..=(x + r).ceil() as i32 {
                if px < 0 || py < 0 || px >= self.pw as i32 || py >= self.ph as i32 {
                    continue;
                }
                let d = ((px as f64 - x).powi(2) + (2.0 * (py as f64 - y)).powi(2)).sqrt() / r;
                if d < 1.0 {
                    let p = &mut self.field[py as usize * self.pw + px as usize];
                    *p = p.mix(th.bad, (1.0 - d).powi(2) * 0.35 * fade);
                }
            }
        }
    }

    /// The pointer's dot, and the rings clicks send out.
    fn laser(&mut self, t: f64, th: &Theme) {
        self.ripples.retain(|r| t - r.2 < 0.9);
        for (x, y, at) in self.ripples.clone() {
            let age = t - at;
            let rad = 4.0 + 30.0 * age;
            let n = (rad * 4.0) as usize;
            for k in 0..n {
                let a = k as f64 / n as f64 * std::f64::consts::TAU;
                let (px, py) = (x + rad * a.cos(), y + rad * a.sin() / 2.0);
                if px >= 0.0 && py >= 0.0 && px < self.pw as f64 && py < self.ph as f64 {
                    let p = &mut self.px[py as usize * self.pw + px as usize];
                    *p = p.mix(th.bad, 0.7 * (1.0 - age / 0.9));
                }
            }
        }
        // The trail: along each stretch it moved, a pixel at a time, dimmer
        // the longer ago.
        const TRAIL: f64 = 0.35;
        self.trail.retain(|p| t - p.2 < TRAIL);
        for w in self.trail.clone().windows(2) {
            let ((x0, y0, t0), (x1, y1, _)) = (w[0], w[1]);
            let steps = ((x1 - x0).abs().max((y1 - y0).abs() * 2.0)).ceil().max(1.0) as usize;
            for k in 0..steps {
                let f = k as f64 / steps as f64;
                let (px, py) = (x0 + (x1 - x0) * f, y0 + (y1 - y0) * f);
                let age = (t - t0) / TRAIL;
                if px >= 0.0 && py >= 0.0 && px < self.pw as f64 && py < self.ph as f64 {
                    let p = &mut self.px[py as usize * self.pw + px as usize];
                    *p = p.mix(th.bad, 0.75 * (1.0 - age).max(0.0).powi(2));
                }
            }
        }
        let Some((x, y, at)) = self.pointer else { return };
        let fade = 1.0 - ((t - at - 1.2) / 0.8).clamp(0.0, 1.0);
        for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
            let (px, py) = ((x + dx - 0.5) as i32, (y + dy - 0.5) as i32);
            if fade > 0.0 && px >= 0 && py >= 0 && (px as usize) < self.pw && (py as usize) < self.ph {
                let p = &mut self.px[py as usize * self.pw + px as usize];
                *p = p.mix(th.bad, fade);
            }
        }
    }

    /// A pixel lit, whatever's near it.
    fn mark(&mut self, x: f64, y: f64, c: Rgb, a: f64) {
        let (x, y) = (x.round(), y.round());
        if x < 0.0 || y < 0.0 || x >= self.pw as f64 || y >= self.ph as f64 || a <= 0.0 {
            return;
        }
        let p = &mut self.px[y as usize * self.pw + x as usize];
        *p = p.mix(c, a.min(1.0));
    }

    /// A pixel lit, faint near text.
    fn dot(&mut self, x: f64, y: f64, c: Rgb, a: f64) {
        let hushed = x.round() >= 0.0 && y.round() >= 0.0 && self.quiet.get(y.round() as usize / 2 * (self.pw / 2) + x.round() as usize / 2).is_some_and(|q| *q);
        self.mark(x, y, c, a.min(1.0) * if hushed { 0.12 } else { 1.0 });
    }

    fn draw(&mut self, t: f64, th: &Theme) {
        let motes = std::mem::take(&mut self.motes);
        for m in &motes {
            match self.kind {
                Kind::Stars => {
                    let tw = 0.55 + 0.45 * (t * (0.8 + m.ph * 0.3) + m.ph).sin();
                    let c = if m.tint == 0 { th.accent } else if m.tint == 1 { th.link } else { th.fg };
                    self.dot(m.x, m.y, c, m.a * tw);
                }
                Kind::Snow => {
                    let sway = 1.6 * (t * 0.9 + m.ph).sin();
                    self.dot(m.x + sway, m.y, th.fg, m.a);
                }
                Kind::Rain => {
                    for k in 0..6 {
                        self.dot(m.x, m.y - k as f64, th.link, m.a * (1.0 - k as f64 / 6.0));
                    }
                }
                Kind::Boids => {
                    // A head and a fainter tail behind it.
                    let v = (m.vx * m.vx + m.vy * m.vy * 4.0).sqrt().max(1.0);
                    let c = if m.tint < 2 { th.accent } else { th.fg };
                    if m.life.is_finite() {
                        // Perched, on the text's edge, where the rest go faint.
                        self.mark(m.x, m.y, c, m.a);
                        continue;
                    }
                    self.dot(m.x, m.y, c, m.a);
                    self.dot(m.x - 1.5 * m.vx / v, m.y - 0.75 * m.vy / v * 2.0, c, m.a * 0.4);
                }
                Kind::Fireflies => {
                    // The bright point; its light is in the field.
                    let b = Self::lit(m, t);
                    self.dot(m.x, m.y, th.warm.mix(th.fg, 0.4), m.a * (0.2 + 0.8 * b));
                }
                Kind::Embers => {
                    let k = m.age / m.life;
                    let c = th.warm.mix(th.bad, k.min(1.0));
                    let flicker = 0.7 + 0.3 * (t * 11.0 + m.ph * 5.0).sin();
                    let fade = (1.0 - k).max(0.0) * (k * 8.0).min(1.0);
                    let sway = 1.2 * (t * 1.3 + m.ph).sin();
                    self.dot(m.x + sway, m.y, c, m.a * flicker * fade);
                }
                _ => {}
            }
        }
        self.motes = motes;
        match self.kind {
            Kind::Snow => {
                // Settled, fading as it melts, in a few steps.
                for i in 0..self.drift.len() {
                    let lit = ((self.drift[i] * 4.0).min(1.0) * 4.0).round() / 4.0;
                    if lit > 0.0 {
                        self.px[i] = self.px[i].mix(th.fg, 0.7 * lit);
                    }
                }
            }
            Kind::Sand => {
                for i in 0..self.cells.len() {
                    if self.cells[i] {
                        // Grains a little unlike each other, by where they are.
                        let (x, y) = (i % self.pw, i / self.pw);
                        self.mark(x as f64, y as f64, th.warm, [0.5, 0.62, 0.42][(x * 7 + y * 13) % 3]);
                    }
                }
            }
            Kind::Koi => self.pond(t),
            Kind::Ants => {
                if let Some(a) = &self.ants {
                    a.draw(&mut self.px, t, th);
                }
            }
            _ => {}
        }
        // Hawks: a head a whole block, a longer tail, in the warm color.
        for m in self.hawks.clone() {
            let v = (m.vx * m.vx + m.vy * m.vy * 4.0).sqrt().max(1.0);
            let (ux, uy) = (m.vx / v, m.vy / v);
            for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                self.dot(m.x + dx, m.y + dy, th.warm, 0.85);
            }
            for k in 1..=3 {
                let k = k as f64 * 1.5;
                self.dot(m.x - ux * k, m.y - uy * k, th.warm, 0.5 / k);
            }
        }
        if let Some(s) = self.streak {
            let fade = 1.0 - s.age / s.life;
            for k in 0..14 {
                let b = k as f64 * 0.012;
                self.dot(s.x - s.vx * b, s.y - s.vy * b, th.fg, s.a * fade * (1.0 - k as f64 / 14.0));
            }
        }
        if self.kind == Kind::Life {
            for i in 0..self.lit.len() {
                // In a few steps, so a fading cell changes a few times, not
                // every frame.
                let lit = (self.lit[i] * 5.0).round() / 5.0;
                if lit > 0.0 {
                    let (x, y) = ((i % self.pw) as f64, (i / self.pw) as f64);
                    self.dot(x, y, th.accent, lit * 0.13);
                }
            }
        }
    }

    /// The cell at (row, col), from 0, with `front` over it: text on the
    /// glow, or the sky in quadrant blocks.
    pub fn look(&self, r: usize, c: usize, front: Option<Cell>) -> Cell {
        let i = |dy: usize, dx: usize| (2 * r + dy) * self.pw + 2 * c + dx;
        if 2 * r + 1 >= self.ph || 2 * c + 1 >= self.pw {
            return front.unwrap_or(Cell { ch: ' ', st: Style::default() });
        }
        // A reaction, over whatever's there: its two cells, the right one
        // drawn with the left.
        for f in &self.floats {
            match self.at(f) {
                Some((fr, fc)) if fr == r && (fc == c || fc + 1 == c) => {
                    let bg = mean(&[self.field[i(0, 0)], self.field[i(0, 1)], self.field[i(1, 0)], self.field[i(1, 1)]]);
                    let ch = if fc == c { f.ch } else { '\0' };
                    return Cell { ch, st: Style { bg: Some(bg), ..Style::default() } };
                }
                _ => {}
            }
        }
        match front {
            Some(f) => {
                let bg = f.st.bg.unwrap_or_else(|| mean(&[self.field[i(0, 0)], self.field[i(0, 1)], self.field[i(1, 0)], self.field[i(1, 1)]]));
                Cell { ch: f.ch, st: Style { bg: Some(bg), ..f.st } }
            }
            None => quad([self.px[i(0, 0)], self.px[i(0, 1)], self.px[i(1, 0)], self.px[i(1, 1)]]),
        }
    }
}

/// Each pixel the average of those within r of it one way, `step` apart
/// in the buffer: 1 across a row, the row's length down a column.
fn blur(v: &mut [[f64; 4]], w: usize, h: usize, r: usize, step: usize) {
    let (lines, len, gap) = if step == 1 { (h, w, w) } else { (w, h, 1) };
    let mut buf = vec![[0.0; 4]; len];
    for l in 0..lines {
        let at = |i: usize| l * gap + i * step;
        for (i, b) in buf.iter_mut().enumerate() {
            let (lo, hi) = (i.saturating_sub(r), (i + r).min(len - 1));
            let mut s = [0.0; 4];
            for j in lo..=hi {
                for k in 0..4 {
                    s[k] += v[at(j)][k];
                }
            }
            *b = s.map(|x| x / (2 * r + 1) as f64);
        }
        for (i, b) in buf.iter().enumerate() {
            v[at(i)] = *b;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quadrants() {
        let (d, l) = (Rgb(10, 10, 10), Rgb(200, 200, 200));
        assert_eq!(quad([d, d, d, d]).ch, ' ');
        assert_eq!(quad([l, d, d, d]).ch, '▘');
        assert_eq!(quad([d, d, l, l]).ch, '▄');
        let c = quad([l, d, d, l]);
        assert_eq!((c.ch, c.st.fg, c.st.bg), ('▚', Some(l), Some(d)));
    }

    #[test]
    fn reactions_rise_and_go() {
        let th = Theme::default();
        let mut s = Sky::new(Kind::None, false, 60, 20, 3);
        let front = vec![None; 60 * 20];
        s.float('👏');
        s.frame(0.5, &th, &front, 60);
        let at = |s: &Sky| (0..20).flat_map(|r| (0..60).map(move |c| (r, c))).find(|&(r, c)| s.look(r, c, None).ch == '👏');
        let (r0, c0) = at(&s).unwrap();
        // Its right half goes with it.
        assert_eq!(s.look(r0, c0 + 1, None).ch, '\0');
        s.frame(2.0, &th, &front, 60);
        assert!(at(&s).unwrap().0 < r0);
        s.frame(4.0, &th, &front, 60);
        assert!(at(&s).is_none() && s.floats.is_empty());
    }

    /// A sky 60x20 with a line of text across row 10, run for `secs`.
    fn over_a_line(kind: Kind, many: [Option<usize>; 2], secs: usize) -> Sky {
        let th = Theme::default();
        let mut s = Sky::new(kind, false, 60, 20, 7);
        s.crowd(many);
        let mut front = vec![None; 60 * 20];
        front[10 * 60 + 10..10 * 60 + 50].fill(Some(Cell { ch: 'x', st: Style::default() }));
        for f in 0..secs * 30 {
            s.frame(f as f64 / 30.0, &th, &front, 60);
        }
        s
    }

    #[test]
    fn as_many_as_it_says() {
        assert_eq!((Kind::from("boids 12 3"), many("boids 12 3"), many("rain")), (Kind::Boids, [Some(12), Some(3)], [None, None]));
        assert_eq!((Kind::from("ants ground fire 80"), many("ants ground fire 80"), said("ants ground fire 80").as_str()), (Kind::Ants, [Some(80), None], "ground fire"));
        let mut s = Sky::new(Kind::Boids, false, 60, 20, 3);
        assert_eq!(s.hawks.len(), 2);
        s.crowd([Some(12), Some(3)]);
        assert_eq!((s.motes.len(), s.hawks.len()), (12, 3));
        s.crowd([Some(5), None]);
        assert_eq!((s.motes.len(), s.hawks.len()), (5, 2));
        let mut s = Sky::new(Kind::Koi, false, 60, 20, 3);
        s.crowd([Some(4), None]);
        assert_eq!((s.koi.len(), s.motes.len()), (4, 0));
    }

    #[test]
    fn snow_lies_on_text_and_birds_perch_on_it() {
        // Settled just over the line, nowhere else.
        let s = over_a_line(Kind::Snow, [None; 2], 40);
        let lying: Vec<usize> = (0..s.drift.len()).filter(|&i| s.drift[i] > 0.0).collect();
        assert!(lying.len() > 10, "{}", lying.len());
        assert!(lying.iter().all(|i| matches!(i / s.pw, 18 | 19) && (20..100).contains(&(i % s.pw))));
        // With no hawks about, some bird's sat on it, on the row over it.
        let mut s = over_a_line(Kind::Boids, [Some(80), Some(0)], 0);
        let (th, mut front) = (Theme::default(), vec![None; 60 * 20]);
        front[10 * 60 + 10..10 * 60 + 50].fill(Some(Cell { ch: 'x', st: Style::default() }));
        let mut sat = 0;
        for f in 0..900 {
            s.frame(f as f64 / 30.0, &th, &front, 60);
            let perched: Vec<&Mote> = s.motes.iter().filter(|m| m.life.is_finite()).collect();
            assert!(perched.iter().all(|m| m.y == 19.0 && (20.0..100.0).contains(&m.x)));
            sat = sat.max(perched.len());
        }
        assert!((1..40).contains(&sat), "{sat}");
        // The line gone, they're off.
        s.frame(30.1, &th, &vec![None; 60 * 20], 60);
        assert!(s.motes.iter().all(|m| !m.life.is_finite()));
    }

    #[test]
    fn sand_heaps_and_runs_out() {
        let s = over_a_line(Kind::Sand, [None; 2], 6);
        let grains = |s: &Sky| s.cells.iter().filter(|c| **c).count();
        // Some on the floor, some on the line, none in it.
        assert!(s.cells[39 * s.pw..].iter().any(|c| *c));
        assert!((20..100).any(|x| s.cells[19 * s.pw + x]));
        assert!((20..100).all(|x| !s.cells[20 * s.pw + x]));
        // Never deeper than it's let get.
        let s = over_a_line(Kind::Sand, [None; 2], 240);
        assert!(grains(&s) <= s.pw * s.ph / 7 + 3, "{}", grains(&s));
    }

    #[test]
    fn koi_keep_their_shape_and_stay_about() {
        let s = over_a_line(Kind::Koi, [Some(5), None], 120);
        for k in &s.koi {
            // In or near the pond, every joint as far from the last as ever.
            assert!((-40.0..160.0).contains(&k.x) && (-40.0..120.0).contains(&k.y), "{} {}", k.x, k.y);
            for w in k.spine.windows(2) {
                let d = ((w[0].0 - w[1].0).powi(2) + (w[0].1 - w[1].1).powi(2)).sqrt();
                assert!((d - JOINT * k.size).abs() < 1e-6);
            }
        }
        // Drawn: somewhere a pixel's nearer a koi's color than the water's.
        let th = Theme::default();
        let water = th.bg.mix(DEEP, 0.45);
        assert!(s.px.iter().any(|p| far(*p, water) > 150));
        // Fed with a click: six bits on the water, and in a while all eaten.
        let mut s = s;
        let front = vec![None; 60 * 20];
        s.ripples.push((60.0, 12.0, 120.0));
        s.frame(120.0, &th, &front, 60);
        // (One may be under a fish's nose already.)
        assert!((4..=6).contains(&s.food.len()));
        for f in 0..25 * 30 {
            s.frame(120.0 + f as f64 / 30.0, &th, &front, 60);
        }
        assert!(s.food.is_empty(), "{} left", s.food.len());
    }

    #[test]
    fn every_kind_runs() {
        let th = Theme::default();
        for k in [Kind::Stars, Kind::Snow, Kind::Rain, Kind::Embers, Kind::Life, Kind::Boids, Kind::Fireflies, Kind::Sand, Kind::Koi, Kind::Ants] {
            let mut s = Sky::new(k, true, 40, 12, 3);
            let mut front = vec![None; 40 * 12];
            front[5 * 40 + 20] = Some(Cell { ch: '█', st: Style::fg(th.accent) });
            for f in 0..40 {
                s.frame(f as f64 / 30.0, &th, &front, 40);
            }
            // The glow lights what's beside the letter.
            assert_ne!(s.look(5, 21, None).st.bg, Some(th.bg), "{k:?}");
        }
    }
}
