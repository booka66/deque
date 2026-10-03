//! sky: ants. A colony, kept the way a keeper would know it: one colony,
//! seen two ways.
//!
//! From the side (`farm`) it's a formicarium: soil under a strip of
//! ground, a shaft going down from the entrance, galleries off it to
//! chambers. The workers dig it themselves, a grain at a time, each grain
//! carried up and dropped round the hole, where a mound grows. Foragers
//! bring food down to the store: leafcutters cut pieces of leaf from the
//! plants and carry them over their heads to the fungus garden; the others
//! take what's lying about. The queen lays while there's food; nurses
//! carry her eggs to the nursery, where they grow from egg to larva to pupa
//! and come out pale. The dead are carried to the midden.
//!
//! From above (`ground`) it's the same ants, those of them that are out:
//! the nest's hole, the plants and the food, and foragers finding it by the
//! scent the others left, so trails form, round the words on the slide. An
//! ant that goes down the hole is gone from this view, and in the other is
//! coming down the shaft.
//!
//! Places are in pixels across and half pixels down, a pixel being twice
//! as tall as wide, so a step is as long whichever way it goes. The ground
//! has the same measure across in both views; from the side, how far away
//! an ant on it is doesn't show.

use crate::markup::{Rgb, Theme};
use crate::screen::Rng;
use std::f64::consts::{PI, TAU};

#[derive(Clone, Copy, PartialEq)]
enum Species {
    Leafcutter,
    Black,
    Fire,
}

/// What a species looks like and does: its color, how fast it walks,
/// whether it cuts leaves, and how many in a hundred are majors, the big
/// ones.
struct Habit {
    body: Rgb,
    pace: f64,
    leaf: bool,
    majors: i32,
}

fn habit(s: Species) -> Habit {
    match s {
        Species::Leafcutter => Habit { body: Rgb(176, 84, 44), pace: 9.0, leaf: true, majors: 22 },
        Species::Black => Habit { body: Rgb(34, 32, 38), pace: 10.0, leaf: false, majors: 0 },
        Species::Fire => Habit { body: Rgb(214, 70, 38), pace: 14.0, leaf: false, majors: 8 },
    }
}

/// What the nest is in: its two tones, and how much of them over the
/// talk's background.
fn medium(said: &str) -> (Rgb, Rgb, f64) {
    if said.split_whitespace().any(|w| w == "sand") {
        (Rgb(206, 178, 128), Rgb(186, 156, 108), 0.6)
    } else if said.split_whitespace().any(|w| w == "gel") {
        (Rgb(56, 146, 190), Rgb(56, 146, 190), 0.45)
    } else {
        (Rgb(124, 86, 54), Rgb(98, 68, 42), 0.62)
    }
}

const LEAF: Rgb = Rgb(84, 168, 84);
const STEM: Rgb = Rgb(66, 118, 62);
const CRUMB: Rgb = Rgb(226, 190, 120);
const BROOD: Rgb = Rgb(238, 234, 220);
const FUNGUS: Rgb = Rgb(196, 204, 186);

fn far(a: Rgb, b: Rgb) -> i32 {
    (a.0 as i32 - b.0 as i32).abs() + (a.1 as i32 - b.1 as i32).abs() + (a.2 as i32 - b.2 as i32).abs()
}

/// From a to b turning the short way, -π to π.
fn veer(a: f64, b: f64) -> f64 {
    (b - a + PI).rem_euclid(TAU) - PI
}

/// A pixel lit, by where it is in the colony's own measure.
fn put(px: &mut [Rgb], pw: usize, x: f64, y: f64, c: Rgb, a: f64) {
    let (x, y) = (x.floor(), (y / 2.0).floor());
    if x < 0.0 || y < 0.0 || x as usize >= pw || y as usize * pw + x as usize >= px.len() {
        return;
    }
    let p = &mut px[y as usize * pw + x as usize];
    *p = p.mix(c, a);
}

/// An ant: its head where it is, and behind it, the way it came, its
/// gaster, darker; a major a pixel longer. What it carries goes over its
/// head, or before it.
fn ant(px: &mut [Rgb], pw: usize, at: (f64, f64), dir: (f64, f64), body: Rgb, big: bool, load: Option<(Rgb, bool)>) {
    let across = dir.0.abs() >= dir.1.abs();
    let back = if across { (-dir.0.signum(), 0.0) } else { (0.0, -2.0 * dir.1.signum()) };
    put(px, pw, at.0, at.1, body, 1.0);
    if across || big {
        put(px, pw, at.0 + back.0, at.1 + back.1, body.mix(Rgb(0, 0, 0), 0.35), 1.0);
    }
    if across && big {
        put(px, pw, at.0 + 2.0 * back.0, at.1, body.mix(Rgb(0, 0, 0), 0.35), 1.0);
    }
    if let Some((c, over)) = load {
        match over {
            true => {
                put(px, pw, at.0, at.1 - 2.0, c, 1.0);
                put(px, pw, at.0 - back.0, at.1 - 2.0, c, if big { 1.0 } else { 0.6 });
            }
            false => put(px, pw, at.0 - back.0, at.1 - back.1, c, 1.0),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Room {
    /// Where tunnels meet; not a chamber.
    None,
    Store,
    Nursery,
    Midden,
    Queen,
    Spare,
}

/// A place underground, and the tunnel to it from the place it's dug from.
struct Tunnel {
    parent: usize,
    /// From the parent's place to this one, and how far along each point is.
    path: Vec<(f64, f64)>,
    along: Vec<f64>,
    len: f64,
    /// How much of it is dug.
    dug: f64,
    room: Room,
    /// A chamber's half-width and half-height, and how much of it is open,
    /// 0 to 1.
    rx: f64,
    ry: f64,
    open: f64,
}

impl Tunnel {
    fn new(parent: usize, path: Vec<(f64, f64)>, room: Room, rx: f64, ry: f64) -> Tunnel {
        let mut t = Tunnel { parent, len: 0.0, path, along: vec![], dug: 0.0, room, rx, ry, open: 0.0 };
        t.measure();
        t
    }

    /// How far along its path each point is, and how long it is.
    fn measure(&mut self) {
        self.along = vec![0.0];
        for w in self.path.windows(2) {
            self.along.push(self.along.last().unwrap() + ((w[1].0 - w[0].0).powi(2) + (w[1].1 - w[0].1).powi(2)).sqrt());
        }
        self.len = *self.along.last().unwrap();
    }

    fn end(&self) -> (f64, f64) {
        *self.path.last().unwrap()
    }

    /// The point s along it, and which way it's going there.
    fn at(&self, s: f64) -> ((f64, f64), (f64, f64)) {
        if self.path.len() < 2 {
            return (self.path[0], (0.0, 1.0));
        }
        let s = s.clamp(0.0, self.len);
        let i = self.along.partition_point(|a| *a <= s).clamp(1, self.path.len() - 1);
        let (a, b) = (self.path[i - 1], self.path[i]);
        let d = (self.along[i] - self.along[i - 1]).max(1e-9);
        let k = (s - self.along[i - 1]) / d;
        ((a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k), ((b.0 - a.0) / d, (b.1 - a.1) / d))
    }

    fn done(&self) -> bool {
        self.dug >= self.len && (self.room == Room::None || self.open >= 1.0)
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Loc {
    /// On the ground: this far across, and this far from the top of it as
    /// it's seen from above.
    Surface(f64, f64),
    /// In the tunnel to a place, this far along it.
    Edge(usize, f64),
    /// In a chamber, this far from its middle.
    Room(usize, f64, f64),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Goal {
    /// Out looking for food, by scent and by sight.
    Seek,
    /// A place on the ground.
    Point(f64, f64),
    Spot(usize, f64),
    /// As far as a tunnel's dug.
    Tip(usize),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Job {
    Dig,
    Forage,
    Nurse,
    Bury,
    Rest,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Carry {
    None,
    Grain,
    Food,
    Leaf,
    /// An egg, as old as it is.
    Egg(f64),
    Dead,
}

/// Food it's come to: a plant, or a pile by its number.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Found {
    Plant(usize),
    Pile(usize),
}

#[derive(Clone)]
struct Ant {
    loc: Loc,
    goal: Goal,
    job: Job,
    /// How far through its job it is: on its way, at it, on its way back.
    stage: u8,
    /// Which corpse, tunnel or leaf the job's about, and what food it's
    /// found.
    aim: usize,
    found: Option<Found>,
    carry: Carry,
    /// Busy where it stands, this much longer; in a chamber, resting.
    wait: f64,
    rest: f64,
    /// In a chamber, where it's wandering to.
    to: (f64, f64),
    /// Where it is, and which way it's going there, as seen from the side.
    at: (f64, f64),
    dir: (f64, f64),
    /// On the ground: which way it's heading, how strong the scent it
    /// lays still is, and how much longer it's going round something.
    heading: f64,
    scent: f64,
    dodge: f64,
    age: f64,
    life: f64,
    big: bool,
    /// Which side of a tunnel it keeps to.
    side: f64,
    ph: f64,
}

struct Plant {
    x: f64,
    z: f64,
    h: f64,
    lean: f64,
    /// Each leaf: how high up, which side, and how much of it is left.
    leaves: Vec<(f64, f64, f64)>,
}

/// How wide a tunnel is, half across.
const BORE: f64 = 2.1;
/// How long from laid to hatched, and when it's a larva and a pupa.
const HATCH: f64 = 80.0;
const LARVA: f64 = 22.0;
const PUPA: f64 = 58.0;

pub struct Colony {
    seed: u32,
    pw: usize,
    ph: usize,
    w: f64,
    h: f64,
    species: Species,
    tones: (Rgb, Rgb, f64),
    gel: bool,
    cap: usize,
    usual: usize,
    rng: Rng,
    /// Seen from above, not from the side.
    top: bool,
    /// The ground's height from the side, and where the entrance is: across,
    /// and from the top of the ground seen from above.
    gy: f64,
    nest: (f64, f64),
    nodes: Vec<Tunnel>,
    /// What's dug, a pixel each, and the mound over the ground, a column
    /// each.
    hole: Vec<bool>,
    mound: Vec<f64>,
    /// Scent leading to food, a pixel of the ground each; and how much of
    /// it shows, in a few steps, looked at again a few times a second.
    scent: Vec<f32>,
    shown: Vec<u8>,
    looked: f64,
    ants: Vec<Ant>,
    plants: Vec<Plant>,
    /// Food lying on the ground: where, how much is left, and its number.
    piles: Vec<(f64, f64, f64, usize)>,
    /// The dead where they lie, each with its number and whether one's
    /// coming for it.
    dead: Vec<(Loc, usize, bool)>,
    /// Eggs, larvae and pupae: the chamber, where in it, and how old.
    brood: Vec<(usize, f64, f64, f64)>,
    text: Vec<bool>,
    count: usize,
    stock: f64,
    refuse: usize,
    lay: f64,
    /// The background, kept till something changes it, with the color and
    /// the view it was made for.
    cache: Vec<Rgb>,
    made: Option<(Rgb, bool)>,
}

impl Colony {
    /// The colony `sky: ants` asks for: `said` is the words after it, a
    /// species (leafcutter, black, fire) and what it's in (soil, sand,
    /// gel), either left out; `cap`, how many workers at most.
    pub fn new(pw: usize, ph: usize, seed: u32, said: &str, cap: Option<usize>) -> Colony {
        let has = |w: &str| said.split_whitespace().any(|x| x == w);
        let species = if has("black") {
            Species::Black
        } else if has("fire") {
            Species::Fire
        } else {
            Species::Leafcutter
        };
        let (w, h) = (pw as f64, ph as f64 * 2.0);
        let usual = (pw * ph / 220).clamp(16, 110) * if species == Species::Fire { 3 } else { 2 } / 2;
        let mut c = Colony {
            seed,
            pw,
            ph,
            w,
            h,
            species,
            tones: medium(said),
            gel: has("gel"),
            cap: cap.map_or(usual, |c| c.clamp(1, 400)),
            usual,
            rng: Rng::new(seed),
            top: false,
            gy: (h * 0.26).round(),
            nest: (0.0, 0.0),
            nodes: vec![],
            hole: vec![false; pw * ph],
            mound: vec![0.0; pw],
            scent: vec![0.0; pw * ph],
            shown: vec![0; pw * ph],
            looked: f64::NEG_INFINITY,
            ants: vec![],
            plants: vec![],
            piles: vec![],
            dead: vec![],
            brood: vec![],
            text: vec![],
            count: 0,
            stock: 10.0,
            refuse: 0,
            lay: 4.0,
            cache: vec![],
            made: None,
        };
        c.plan();
        c.settle();
        c
    }

    /// Which colony it is: the same number, the same colony.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// The screen it's on, in pixels.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn size(&self) -> (usize, usize) {
        (self.pw, self.ph)
    }

    pub fn cap(&mut self, cap: Option<usize>) {
        self.cap = cap.map_or(self.usual, |c| c.clamp(1, 400));
    }

    /// Seen from above, or from the side.
    pub fn view(&mut self, top: bool) {
        self.top = top;
    }

    fn r(&mut self) -> f64 {
        self.rng.below(1 << 20) as f64 / (1 << 20) as f64
    }

    /// The colony on a screen of another size: everything where it was,
    /// in proportion. The scent on the ground is laid again.
    pub fn resize(&mut self, pw: usize, ph: usize) {
        if (pw, ph) == (self.pw, self.ph) {
            return;
        }
        let (w, h) = (pw as f64, ph as f64 * 2.0);
        let gy = (h * 0.26).round();
        let (sx, sz) = (w / self.w, h / self.h);
        // Underground, down from the ground as it now is.
        let sy = (h - gy) / (self.h - self.gy);
        let old = self.gy;
        let under = |p: (f64, f64)| (p.0 * sx, gy + (p.1 - old) * sy);
        let lens: Vec<f64> = self.nodes.iter().map(|n| n.len).collect();
        for n in &mut self.nodes {
            let frac = if n.len > 0.0 { n.dug / n.len } else { 0.0 };
            n.path = n.path.iter().map(|p| under(*p)).collect();
            n.measure();
            (n.dug, n.rx, n.ry) = (frac * n.len, n.rx * sx, n.ry * sy);
        }
        let now: Vec<f64> = self.nodes.iter().map(|n| n.len).collect();
        let on = |x: f64, z: f64| ((x * sx).min(w - 1.0), (z * sz).min(h - 1.0));
        let moved = |loc: Loc| match loc {
            Loc::Surface(x, z) => Loc::Surface(on(x, z).0, on(x, z).1),
            Loc::Edge(e, s) => Loc::Edge(e, if lens[e] > 0.0 { s / lens[e] * now[e] } else { 0.0 }),
            Loc::Room(n, ox, oy) => Loc::Room(n, ox * sx, oy * sy),
        };
        for a in &mut self.ants {
            a.loc = moved(a.loc);
            a.goal = match a.goal {
                Goal::Point(x, z) => Goal::Point(on(x, z).0, on(x, z).1),
                Goal::Spot(e, s) => Goal::Spot(e, if lens[e] > 0.0 { s / lens[e] * now[e] } else { 0.0 }),
                g => g,
            };
            a.to = (a.to.0 * sx, a.to.1 * sy);
        }
        for d in &mut self.dead {
            d.0 = moved(d.0);
        }
        for b in &mut self.brood {
            (b.1, b.2) = (b.1 * sx, b.2 * sy);
        }
        let tall = gy / self.gy.max(1.0);
        for p in &mut self.plants {
            (p.x, p.z) = on(p.x, p.z);
            p.h *= tall;
            p.leaves.iter_mut().for_each(|l| l.0 *= tall);
        }
        for p in &mut self.piles {
            (p.0, p.1) = on(p.0, p.1);
        }
        self.mound = (0..pw).map(|k| self.mound[((k as f64 / sx) as usize).min(self.pw - 1)] * sy.min(sx)).collect();
        self.nest = (self.nest.0 * sx, self.nest.1 * sz);
        (self.pw, self.ph, self.w, self.h, self.gy) = (pw, ph, w, h, gy);
        (self.hole, self.scent, self.shown) = (vec![false; pw * ph], vec![0.0; pw * ph], vec![0; pw * ph]);
        self.carve();
        for i in 0..self.ants.len() {
            self.ants[i].at = self.spot(self.ants[i].loc, self.ants[i].side);
        }
    }

    /// A tunnel's course from a to b: not straight, bowed a little each
    /// way, the corners rounded off.
    fn course(&mut self, a: (f64, f64), b: (f64, f64)) -> Vec<(f64, f64)> {
        let (dx, dy) = (b.0 - a.0, b.1 - a.1);
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let (nx, ny) = (-dy / len, dx / len);
        let mut pts = vec![a];
        for k in [0.33, 0.66] {
            let bow = (self.r() - 0.5) * len * 0.22;
            pts.push((a.0 + dx * k + nx * bow, a.1 + dy * k + ny * bow));
        }
        pts.push(b);
        for _ in 0..3 {
            let mut next = vec![pts[0]];
            for w in pts.windows(2) {
                next.push((w[0].0 * 0.75 + w[1].0 * 0.25, w[0].1 * 0.75 + w[1].1 * 0.25));
                next.push((w[0].0 * 0.25 + w[1].0 * 0.75, w[0].1 * 0.25 + w[1].1 * 0.75));
            }
            next.push(*pts.last().unwrap());
            pts = next;
        }
        pts
    }

    /// A gallery off place `from` to a new chamber, to the side there's
    /// more room on, or the side asked for.
    fn gallery(&mut self, from: usize, room: Room, side: Option<f64>) {
        let at = self.nodes[from].end();
        let side = side.unwrap_or(if at.0 < self.w / 2.0 { 1.0 } else { -1.0 });
        let reach = self.w * (0.15 + 0.18 * self.r());
        let (rx, ry) = (8.0 + 5.0 * self.r(), 3.6 + 1.8 * self.r());
        // Kept in the soil, however small the screen.
        let to = ((at.0 + side * reach).min(self.w - rx - 3.0).max(rx + 3.0), (at.1 + 5.0 * (self.r() - 0.3)).min(self.h - ry - 3.0).max(self.gy + 8.0));
        let path = self.course(at, to);
        self.nodes.push(Tunnel::new(from, path, room, rx, ry));
    }

    /// The nest as it's going to be: the entrance, a shaft down from it in
    /// stretches, a gallery to a chamber off each bend, and the queen's
    /// chamber at the bottom.
    fn plan(&mut self) {
        // Low on the ground as it's seen from above, where a slide's text
        // mostly isn't.
        self.nest = (self.w * (0.3 + 0.4 * self.r()), self.h * (0.7 + 0.16 * self.r()));
        self.nodes.push(Tunnel::new(0, vec![(self.nest.0, self.gy)], Room::None, 0.0, 0.0));
        let deep = self.h - self.gy;
        let levels = ((deep / 16.0) as usize).clamp(2, 5);
        let drop = (deep - 6.0) / levels as f64;
        let rooms = [Room::Store, Room::Nursery, Room::Midden, Room::Spare];
        let (mut from, mut side) = (0, if self.r() < 0.5 { 1.0 } else { -1.0 });
        for l in 0..levels {
            let at = self.nodes[from].end();
            let sway = self.w * 0.1 * (self.r() - 0.5) * 2.0;
            let to = ((at.0 + sway).min(self.w - 12.0).max(12.0), self.gy + drop * (l + 1) as f64);
            let path = self.course(at, to);
            let last = l + 1 == levels;
            self.nodes.push(Tunnel::new(from, path, if last { Room::Queen } else { Room::None }, if last { 12.0 } else { 0.0 }, if last { 5.0 } else { 0.0 }));
            from = self.nodes.len() - 1;
            if !last {
                self.gallery(from, rooms[l.min(rooms.len() - 1)], Some(side));
                side = -side;
            }
        }
    }

    /// A new ant at a place, `age` of the way through its life.
    fn hatch(&mut self, loc: Loc, age: f64) -> Ant {
        let life = 300.0 + 400.0 * self.r();
        Ant {
            loc,
            goal: Goal::Seek,
            job: Job::Rest,
            stage: 0,
            aim: 0,
            found: None,
            carry: Carry::None,
            wait: 0.0,
            rest: 0.0,
            to: (0.0, 0.0),
            at: self.spot(loc, 1.0),
            dir: (1.0, 0.0),
            heading: TAU * self.r(),
            scent: 1.0,
            dodge: 0.0,
            age: age * life,
            life,
            big: self.rng.below(100) < habit(self.species).majors,
            side: if self.rng.below(2) == 0 { 1.0 } else { -1.0 },
            ph: TAU * self.r(),
        }
    }

    /// Somewhere on the ground clear of the nest by `clear`, and of text.
    fn somewhere(&mut self, clear: f64) -> (f64, f64) {
        let mut at = (self.w / 2.0, self.h / 2.0);
        for _ in 0..60 {
            at = (6.0 + (self.w - 12.0).max(0.0) * self.r(), 6.0 + (self.h - 12.0).max(0.0) * self.r());
            let near = (self.nest.0 - at.0).powi(2) + (self.nest.1 - at.1).powi(2) < clear * clear;
            let round = [(0.0, 0.0), (6.0, 0.0), (-6.0, 0.0), (0.0, 6.0), (0.0, -6.0)];
            if !near && !round.iter().any(|d| self.blocked(at.0 + d.0, at.1 + d.1)) {
                break;
            }
        }
        at
    }

    /// The colony as it is when the slide comes: the shaft, the store and
    /// the queen's chamber dug already, the rest to do; some workers about,
    /// a few plants up top.
    fn settle(&mut self) {
        for n in &mut self.nodes {
            if matches!(n.room, Room::None | Room::Store | Room::Queen) {
                (n.dug, n.open) = (n.len, 1.0);
            }
        }
        self.carve();
        for k in 0..self.pw {
            let d = (k as f64 - self.nest.0).abs();
            self.mound[k] = if d < 2.0 { 0.0 } else { (3.0 - (d - 6.0).abs() * 0.45).max(0.0) };
        }
        for spot in [0.12, 0.86, 0.5] {
            let x = self.w * spot + 6.0 * (self.r() - 0.5);
            if (x - self.nest.0).abs() < 22.0 {
                continue;
            }
            let h = self.gy * (0.62 + 0.25 * self.r());
            let n = ((h / 5.0) as usize).clamp(2, 6);
            let leaves = (0..n).map(|k| (h * (0.35 + 0.65 * (k + 1) as f64 / n as f64), if k % 2 == 0 { 1.0 } else { -1.0 }, 1.0)).collect();
            let (z, lean) = (self.h * (0.15 + 0.7 * self.r()), (self.r() - 0.5) * 0.5);
            self.plants.push(Plant { x, z, h, lean, leaves });
        }
        for _ in 0..(self.cap * 2 / 3).max(4).min(self.cap) {
            // Somewhere along the shaft, or out on the ground.
            let loc = match self.rng.below(3) {
                0 => Loc::Surface(self.w * self.r(), self.h * self.r()),
                _ => {
                    let e = 1 + self.rng.below(self.nodes.len() as i32 - 1) as usize;
                    Loc::Edge(e, self.nodes[e].dug * self.r())
                }
            };
            let age = 0.1 + 0.9 * self.r();
            let mut a = self.hatch(loc, age);
            self.assign(&mut a);
            self.ants.push(a);
        }
        let q = self.room(Room::Queen).unwrap();
        for _ in 0..4 {
            let egg = (q, 5.0 * (self.r() - 0.5), 1.0 + self.r(), HATCH * self.r());
            self.brood.push(egg);
        }
    }

    /// The pixels that are dug: along each tunnel as far as it's dug, and
    /// each chamber as far as it's open.
    fn carve(&mut self) {
        self.hole.fill(false);
        let pw = self.pw;
        let stamp = |hole: &mut [bool], c: (f64, f64), rx: f64, ry: f64| {
            for py in ((c.1 - ry) / 2.0).floor().max(0.0) as usize..=((c.1 + ry) / 2.0).ceil().max(0.0) as usize {
                for px in (c.0 - rx).floor().max(0.0) as usize..=(c.0 + rx).ceil().max(0.0) as usize {
                    let (dx, dy) = ((px as f64 + 0.5 - c.0) / rx, (py as f64 * 2.0 + 1.0 - c.1) / ry);
                    if px < pw && py * pw + px < hole.len() && dx * dx + dy * dy <= 1.0 {
                        hole[py * pw + px] = true;
                    }
                }
            }
        };
        for n in &self.nodes {
            let mut s = 0.0;
            while s <= n.dug.min(n.len) {
                stamp(&mut self.hole, n.at(s).0, BORE, BORE);
                s += 0.5;
            }
            if n.room != Room::None && n.dug >= n.len && n.open > 0.0 {
                let k = n.open.min(1.0).sqrt();
                stamp(&mut self.hole, n.end(), n.rx * k, n.ry * k);
            }
        }
        self.made = None;
    }

    fn room(&self, r: Room) -> Option<usize> {
        self.nodes.iter().position(|n| n.room == r && n.done())
    }

    /// The tunnel being dug: the first not done whose way in is.
    fn site(&self) -> Option<usize> {
        (1..self.nodes.len()).find(|&i| !self.nodes[i].done() && (self.nodes[i].parent == 0 || self.nodes[self.nodes[i].parent].done()))
    }

    /// From place `from`, the next place on the way down to `to`, when
    /// `to` is further in.
    fn toward(&self, from: usize, to: usize) -> Option<usize> {
        let mut at = to;
        while at != 0 && at != from {
            if self.nodes[at].parent == from {
                return Some(at);
            }
            at = self.nodes[at].parent;
        }
        None
    }

    /// How high the ground is at x, from the side: lower numbers are
    /// higher up.
    fn ground(&self, x: f64) -> f64 {
        self.gy - self.mound.get(x.max(0.0) as usize).copied().unwrap_or(0.0)
    }

    fn stem(&self, p: usize, up: f64) -> (f64, f64) {
        let pl = &self.plants[p];
        (pl.x + pl.lean * pl.h * (up / pl.h).powi(2), self.gy - up)
    }

    /// Where a place is, seen from the side.
    fn spot(&self, loc: Loc, side: f64) -> (f64, f64) {
        match loc {
            Loc::Surface(x, _) => (x, self.ground(x) - 1.0),
            Loc::Edge(e, s) => {
                let (p, d) = self.nodes[e].at(s);
                (p.0 - d.1 * 0.45 * side, p.1 + d.0 * 0.45 * side)
            }
            Loc::Room(n, ox, oy) => {
                let c = self.nodes[n].end();
                (c.0 + ox, c.1 + oy)
            }
        }
    }

    /// Whether an ant on the ground can't be at a place: it's off the
    /// screen, or, seen from above, under text. Round the nest's hole it
    /// always can.
    fn blocked(&self, x: f64, z: f64) -> bool {
        if x < 1.0 || z < 1.0 || x >= self.w - 1.0 || z >= self.h - 1.0 {
            return true;
        }
        let near = (x - self.nest.0).powi(2) + (z - self.nest.1).powi(2) < 100.0;
        self.top && !near && self.text.get(z as usize / 4 * (self.pw / 2) + x as usize / 2).is_some_and(|t| *t)
    }

    fn cell(&self, x: f64, z: f64) -> Option<usize> {
        (x >= 0.0 && z >= 0.0 && x < self.w && z < self.h).then(|| (z as usize / 2).min(self.ph - 1) * self.pw + x as usize)
    }

    /// The food within `reach` of a place, the nearest: a plant with leaf
    /// on it, for those that cut leaves, or a pile.
    fn food(&self, x: f64, z: f64, reach: f64) -> Option<(Found, f64, f64)> {
        let leaf = habit(self.species).leaf;
        let plants = self.plants.iter().enumerate().filter(|(_, p)| leaf && p.leaves.iter().any(|l| l.2 > 0.3)).map(|(k, p)| (Found::Plant(k), p.x, p.z));
        let piles = self.piles.iter().filter(|p| p.2 > 0.0).map(|p| (Found::Pile(p.3), p.0, p.1));
        let far = |f: &(Found, f64, f64)| (f.1 - x).powi(2) + (f.2 - z).powi(2);
        plants.chain(piles).filter(|f| far(f) < reach * reach).min_by(|a, b| far(a).total_cmp(&far(b)))
    }

    /// A step on the ground: toward where it's going, round what's in the
    /// way; or looking for food, by the scent of those that found some, and
    /// by sight when it's close. Whether it's there.
    fn roam(&mut self, a: &mut Ant, x: f64, z: f64, dt: f64) -> bool {
        let h = habit(self.species);
        let nest = self.nest;
        let (to, reach) = match a.goal {
            Goal::Point(px, pz) => (Some((px, pz)), 2.0),
            Goal::Seek => (None, 0.0),
            _ => (Some(nest), 2.5),
        };
        let mut turn = 1.6 * (self.r() - 0.5) + 0.6 * (a.ph + x * 0.3 + z * 0.2).sin();
        a.dodge = (a.dodge - dt).max(0.0);
        match to {
            // It knows which way it's going, as ants do, counting steps.
            Some(p) => turn += if a.dodge > 0.0 { 0.4 } else { 4.0 } * veer(a.heading, (p.1 - z).atan2(p.0 - x)),
            None => match self.food(x, z, 16.0) {
                Some((_, fx, fz)) => turn += 3.0 * veer(a.heading, (fz - z).atan2(fx - x)),
                None => {
                    // It smells ahead, to the left and to the right; a
                    // trail leads both ways, and it takes it away from home.
                    let out = (z - nest.1).atan2(x - nest.0);
                    let smell = |c: &Colony, by: f64| {
                        let d = a.heading + by;
                        let (sx, sz) = (x + 7.0 * d.cos(), z + 7.0 * d.sin());
                        if c.blocked(sx, sz) {
                            return -1.0;
                        }
                        c.cell(sx, sz).map_or(0.0, |i| c.scent[i] as f64) * if veer(d, out).abs() > 2.0 { 0.3 } else { 1.0 }
                    };
                    let (l, c, r) = (smell(self, -0.6), smell(self, 0.0), smell(self, 0.6));
                    if l > c && l > r {
                        turn -= 3.0;
                    } else if r > c && r > l {
                        turn += 3.0;
                    }
                }
            },
        }
        a.heading += turn * dt;
        let v = h.pace * 1.2 * if a.carry == Carry::None { 1.0 } else { 0.8 } * dt;
        let (nx, nz) = (x + a.heading.cos() * v, z + a.heading.sin() * v);
        // Something in the way: it turns along it a while. In among text
        // already, it walks out.
        let (mut x, mut z) = (x, z);
        if self.blocked(nx, nz) && !self.blocked(x, z) {
            a.heading += (0.9 + self.r()) * if self.rng.below(2) == 0 { 1.0 } else { -1.0 };
            a.dodge = 0.7;
        } else {
            (x, z) = (nx.clamp(0.0, self.w - 0.01), nz.clamp(0.0, self.h - 0.01));
        }
        a.loc = Loc::Surface(x, z);
        // Coming home with food, it lays the scent the others follow.
        a.scent *= (-dt / 18.0).exp();
        if matches!(a.carry, Carry::Food | Carry::Leaf)
            && let Some(i) = self.cell(x, z)
        {
            self.scent[i] = (self.scent[i] + (a.scent * dt * 6.0) as f32).min(4.0);
        }
        match to {
            Some(p) if (p.0 - x).powi(2) + (p.1 - z).powi(2) < reach * reach => match a.goal {
                Goal::Point(..) => return true,
                // Down the hole: the entrance has the one way in.
                _ => a.loc = Loc::Edge(1, 0.0),
            },
            None => {
                if let Some((f, ..)) = self.food(x, z, 3.5) {
                    a.found = Some(f);
                    return true;
                }
            }
            _ => {}
        }
        false
    }

    /// A step toward its goal; whether it's there.
    fn walk(&mut self, a: &mut Ant, dt: f64) -> bool {
        let h = habit(self.species);
        let v = h.pace * dt * if a.carry == Carry::None { 1.0 } else { 0.8 } * if a.age < 30.0 { 0.6 } else { 1.0 };
        let under = match a.goal {
            Goal::Spot(e, s) => Some((e, s)),
            Goal::Tip(e) => Some((e, (self.nodes[e].dug - 0.4).max(0.0))),
            _ => None,
        };
        match a.loc {
            Loc::Surface(x, z) => return self.roam(a, x, z, dt),
            Loc::Edge(e, s) => {
                let (len, dug) = (self.nodes[e].len, self.nodes[e].dug.min(self.nodes[e].len));
                let down = under.and_then(|(g, _)| self.toward(e, g));
                let to = match under {
                    Some((g, sg)) if g == e => sg,
                    _ if down.is_some() => len,
                    _ => 0.0,
                }
                .min(dug);
                let ns = s + (to - s).clamp(-v, v);
                a.loc = Loc::Edge(e, ns);
                if (ns - to).abs() < 1e-6 {
                    match (under, down) {
                        (Some((g, _)), _) if g == e => return true,
                        (_, Some(c)) if dug >= len => a.loc = Loc::Edge(c, 0.0),
                        // Not dug through to where it was going.
                        (_, Some(_)) => return true,
                        _ => {
                            let p = self.nodes[e].parent;
                            a.loc = match under.and_then(|(g, _)| self.toward(p, g)) {
                                // Out of the hole, onto the ground.
                                _ if p == 0 && under.is_none() => {
                                    (a.heading, a.scent) = (TAU * self.r(), 1.0);
                                    Loc::Surface(self.nest.0, self.nest.1)
                                }
                                Some(c) if c != e => Loc::Edge(c, 0.0),
                                // Back down the one tunnel there is.
                                _ if p == 0 => return true,
                                _ => Loc::Edge(p, self.nodes[p].len),
                            };
                        }
                    }
                }
            }
            Loc::Room(n, ..) => a.loc = Loc::Edge(n, self.nodes[n].len),
        }
        false
    }

    /// What it does next: dig while there's digging and too few at it,
    /// see to the dead, carry eggs, and otherwise forage, or rest a while.
    fn assign(&mut self, a: &mut Ant) {
        (a.stage, a.wait, a.carry, a.found) = (0, 0.0, Carry::None, None);
        let pop = self.ants.len().max(1);
        let doing = |j: Job| self.ants.iter().filter(|o| o.job == j).count();
        if let Some(e) = self.site().filter(|_| doing(Job::Dig) * 3 < pop + 3) {
            (a.job, a.goal, a.aim) = (Job::Dig, Goal::Tip(e), e);
            return;
        }
        if let Some(k) = self.dead.iter().position(|d| !d.2) {
            self.dead[k].2 = true;
            let goal = match self.dead[k].0 {
                Loc::Surface(x, z) => Goal::Point(x, z),
                Loc::Edge(e, s) => Goal::Spot(e, s),
                Loc::Room(n, ..) => Goal::Spot(n, self.nodes[n].len),
            };
            (a.job, a.goal, a.aim) = (Job::Bury, goal, self.dead[k].1);
            return;
        }
        let queen = self.room(Room::Queen).unwrap();
        if self.room(Room::Nursery).is_some() && self.brood.iter().any(|b| b.0 == queen) && doing(Job::Nurse) * 6 < pop + 6 {
            (a.job, a.goal) = (Job::Nurse, Goal::Spot(queen, self.nodes[queen].len));
            return;
        }
        if self.rng.below(100) < 72 {
            (a.job, a.goal) = (Job::Forage, Goal::Seek);
            return;
        }
        let rooms: Vec<usize> = (1..self.nodes.len()).filter(|&i| self.nodes[i].room != Room::None && self.nodes[i].done()).collect();
        let n = rooms[self.rng.below(rooms.len() as i32) as usize];
        (a.job, a.goal) = (Job::Rest, Goal::Spot(n, self.nodes[n].len));
    }

    /// At its goal: what it came to do.
    fn arrive(&mut self, a: &mut Ant) {
        let store = self.room(Room::Store).unwrap();
        let home = Goal::Spot(store, self.nodes[store].len);
        match (a.job, a.stage) {
            (Job::Dig, 0) => (a.wait, a.stage) = (0.7 + 0.5 * self.r(), 1),
            (Job::Dig, _) => {
                // The grain dropped: the mound's a little higher there.
                if let Loc::Surface(x, _) = a.loc {
                    let k = (x as usize).min(self.pw - 1);
                    self.mound[k] += 0.5;
                    self.made = None;
                }
                self.assign(a);
            }
            (Job::Forage, 0) => match a.found {
                // A leaf with enough on it, cut, which takes a moment.
                Some(Found::Plant(p)) => match (0..self.plants[p].leaves.len()).find(|&l| self.plants[p].leaves[l].2 > 0.3) {
                    Some(l) => (a.wait, a.stage, a.aim) = (1.0 + self.r(), 1, l),
                    None => a.found = None,
                },
                Some(Found::Pile(id)) => match self.piles.iter_mut().find(|p| p.3 == id && p.2 > 0.0) {
                    Some(p) => {
                        p.2 -= 1.0;
                        (a.carry, a.goal, a.stage, a.scent) = (Carry::Food, home, 2, 1.0);
                    }
                    None => a.found = None,
                },
                None => {}
            },
            (Job::Forage, _) => {
                self.stock += 1.0;
                self.assign(a);
            }
            (Job::Nurse, 0) => {
                let queen = self.room(Room::Queen).unwrap();
                match (self.brood.iter().position(|b| b.0 == queen), self.room(Room::Nursery)) {
                    (Some(k), Some(n)) => {
                        let egg = self.brood.remove(k);
                        (a.carry, a.goal, a.stage) = (Carry::Egg(egg.3), Goal::Spot(n, self.nodes[n].len), 2);
                    }
                    _ => self.assign(a),
                }
            }
            (Job::Nurse, _) => {
                if let (Carry::Egg(age), Goal::Spot(n, _)) = (a.carry, a.goal) {
                    let (rx, ry) = (self.nodes[n].rx, self.nodes[n].ry);
                    let egg = (n, rx * 1.4 * (self.r() - 0.5), ry * 0.6 * self.r(), age);
                    self.brood.push(egg);
                }
                self.assign(a);
            }
            (Job::Bury, 0) => match self.dead.iter().position(|d| d.1 == a.aim) {
                Some(k) => {
                    self.dead.remove(k);
                    // To the midden, or till there's one, out and away
                    // from the door.
                    let to = match self.room(Room::Midden) {
                        Some(m) => Goal::Spot(m, self.nodes[m].len),
                        None => {
                            let ang = TAU * self.r();
                            Goal::Point((self.nest.0 + 26.0 * ang.cos()).min(self.w - 3.0).max(3.0), (self.nest.1 + 26.0 * ang.sin()).min(self.h - 3.0).max(3.0))
                        }
                    };
                    (a.carry, a.goal, a.stage) = (Carry::Dead, to, 2);
                }
                None => self.assign(a),
            },
            (Job::Bury, _) => {
                self.refuse += 1;
                self.assign(a);
            }
            (Job::Rest, _) => {
                if let Goal::Spot(n, _) = a.goal {
                    (a.loc, a.rest, a.to) = (Loc::Room(n, 0.0, 0.0), 3.0 + 7.0 * self.r(), (0.0, 0.0));
                }
            }
        }
    }

    /// Done with what it was busy at: a grain dug out, or a piece of leaf
    /// cut.
    fn finish(&mut self, a: &mut Ant) {
        match a.job {
            Job::Dig => {
                let n = &mut self.nodes[a.aim];
                if n.dug < n.len {
                    n.dug = (n.dug + 1.3).min(n.len);
                } else {
                    n.open = (n.open + 0.7 / (n.rx * n.ry)).min(1.0);
                }
                self.carve();
                // Out, and a little way from the hole, any way round it.
                let (ang, out) = (TAU * self.r(), 4.0 + 7.0 * self.r());
                let to = Goal::Point((self.nest.0 + out * ang.cos()).min(self.w - 2.0).max(1.0), (self.nest.1 + out * ang.sin()).min(self.h - 2.0).max(1.0));
                (a.carry, a.goal, a.stage) = (Carry::Grain, to, 2);
            }
            Job::Forage => {
                if let Some(Found::Plant(p)) = a.found {
                    let leaf = &mut self.plants[p].leaves[a.aim].2;
                    *leaf = (*leaf - 0.2).max(0.0);
                }
                let store = self.room(Room::Store).unwrap();
                (a.carry, a.goal, a.stage, a.scent) = (Carry::Leaf, Goal::Spot(store, self.nodes[store].len), 2, 1.0);
            }
            _ => self.assign(a),
        }
    }

    fn live(&mut self, a: &mut Ant, dt: f64) {
        a.age += dt;
        if a.wait > 0.0 {
            a.wait -= dt;
            if a.wait <= 0.0 {
                self.finish(a);
            }
            return;
        }
        if let Loc::Room(n, ox, oy) = a.loc {
            // About the chamber, slowly, from one spot to another.
            a.rest -= dt;
            let (dx, dy) = (a.to.0 - ox, a.to.1 - oy);
            let d = (dx * dx + dy * dy).sqrt();
            if d < 0.3 {
                let (rx, ry) = (self.nodes[n].rx, self.nodes[n].ry);
                a.to = (rx * 1.5 * (self.r() - 0.5), ry * 1.2 * (self.r() - 0.5));
            } else {
                let v = (habit(self.species).pace * 0.3 * dt).min(d);
                a.loc = Loc::Room(n, ox + dx / d * v, oy + dy / d * v);
            }
            if a.rest <= 0.0 {
                a.loc = Loc::Edge(n, self.nodes[n].len);
                self.assign(a);
            }
            return;
        }
        if self.walk(a, dt) {
            self.arrive(a);
        }
    }

    /// A moment on. `text` is the cells with text in them, as wide as the
    /// screen is; `clicks`, where food's been dropped, in pixels.
    pub fn step(&mut self, dt: f64, t: f64, text: &[bool], clicks: &[(f64, f64)]) {
        if self.text != text {
            self.text = text.to_vec();
        }
        // Food dropped where a click was: from above, there; from the
        // side, that far across, as far away as the nest is, more or less.
        for &(x, y) in clicks {
            self.count += 1;
            let z = if self.top { y * 2.0 } else { self.nest.1 + 24.0 * (self.r() - 0.5) };
            let pile = (x.min(self.w - 2.0).max(1.0), z.min(self.h - 2.0).max(1.0), if self.top { 20.0 } else { 9.0 }, self.count);
            self.piles.push(pile);
        }
        // Those that cut no leaves find what's fallen: there's always a
        // pile or two somewhere.
        self.piles.retain(|p| p.2 > 0.0);
        if !habit(self.species).leaf && self.piles.len() < 2 {
            self.count += 1;
            let (at, much) = (self.somewhere(40.0), 20.0 + 25.0 * self.r());
            self.piles.push((at.0, at.1, much, self.count));
        }
        for l in self.plants.iter_mut().flat_map(|p| &mut p.leaves) {
            l.2 = (l.2 + dt / 90.0).min(1.0);
        }
        // A trail to food that's gone is soon forgotten. What shows of it
        // is looked at again only now and then: less to draw.
        let fade = (-dt / 22.0).exp() as f32;
        self.scent.iter_mut().for_each(|v| *v *= fade);
        if t - self.looked > 0.4 || t < self.looked {
            self.looked = t;
            let mut changed = false;
            for (s, v) in self.shown.iter_mut().zip(&self.scent) {
                let lit = ((*v * 1.5).min(1.0) * 3.0).round() as u8;
                changed |= *s != lit;
                *s = lit;
            }
            if changed && self.top {
                self.made = None;
            }
        }
        // The queen lays while there's food and room for more; the brood
        // grows, and hatches where it lies.
        self.stock = (self.stock - self.ants.len() as f64 * 0.003 * dt).max(0.0);
        self.lay -= dt;
        let queen = self.room(Room::Queen).unwrap();
        // An egg on its way to the nursery is still one to come.
        let carried = self.ants.iter().filter(|a| matches!(a.carry, Carry::Egg(_))).count();
        if self.lay <= 0.0 && self.stock >= 1.0 && self.ants.len() + self.brood.len() + carried < self.cap {
            self.lay = 7.0 + 5.0 * self.r();
            self.stock -= 1.0;
            let egg = (queen, 4.0 + 3.0 * self.r(), 1.0 + self.r(), 0.0);
            self.brood.push(egg);
        }
        for b in &mut self.brood {
            b.3 += dt;
        }
        for b in std::mem::take(&mut self.brood) {
            if b.3 < HATCH {
                self.brood.push(b);
                continue;
            }
            let mut a = self.hatch(Loc::Room(b.0, b.1, b.2), 0.0);
            (a.goal, a.rest) = (Goal::Spot(b.0, self.nodes[b.0].len), 12.0);
            self.ants.push(a);
        }
        for i in 0..self.ants.len() {
            let mut a = self.ants[i].clone();
            self.live(&mut a, dt);
            let at = self.spot(a.loc, a.side);
            let (dx, dy) = (at.0 - a.at.0, at.1 - a.at.1);
            let d = (dx * dx + dy * dy).sqrt();
            if d > 1e-3 {
                a.dir = (dx / d, dy / d);
            }
            a.at = at;
            self.ants[i] = a;
        }
        // The old die where they stand, and lie there till they're fetched.
        for a in std::mem::take(&mut self.ants) {
            if a.age < a.life {
                self.ants.push(a);
                continue;
            }
            // What it was coming for is anyone's again.
            if let Some(d) = self.dead.iter_mut().find(|d| a.job == Job::Bury && d.1 == a.aim) {
                d.2 = false;
            }
            self.count += 1;
            self.dead.push((a.loc, self.count, false));
        }
        // All dug, and the colony grown into it: another chamber, off one
        // of the bends.
        if self.site().is_none() && self.nodes.len() < 14 && self.ants.len() * 5 >= self.cap * 3 {
            let bends: Vec<usize> = (1..self.nodes.len()).filter(|&i| self.nodes[i].room == Room::None).collect();
            let from = bends[self.rng.below(bends.len() as i32) as usize];
            self.gallery(from, Room::Spare, None);
        }
        // The mound slumps where it's steep, and wears away a little, so
        // it never buries the plants.
        for k in 1..self.pw {
            let d = self.mound[k] - self.mound[k - 1];
            if d.abs() > 1.2 {
                let m = d.signum() * 0.25;
                self.mound[k] -= m;
                self.mound[k - 1] += m;
                self.made = None;
            }
        }
        let total: f64 = self.mound.iter().sum();
        if total > self.w * 0.9 {
            self.mound.iter_mut().for_each(|m| *m *= 1.0 - 0.01 * dt);
        }
        let door = (self.nest.0 as usize).min(self.pw - 1);
        for k in door.saturating_sub(1)..=(door + 1).min(self.pw - 1) {
            self.mound[k] = 0.0;
        }
    }

    /// What's behind the text: from the side, the soil and what's dug in
    /// it; from above, the ground and the scent on it.
    pub fn back(&mut self, field: &mut [Rgb], th: &Theme) {
        if self.made != Some((th.bg, self.top)) || self.cache.len() != field.len() {
            self.made = Some((th.bg, self.top));
            let (a, b, much) = self.tones;
            let (void, earth) = (th.bg.mix(a, 0.1), th.bg.mix(a, much * 0.3));
            self.cache = (0..self.pw * self.ph)
                .map(|i| {
                    if self.top {
                        // The trail to food shows, faintly.
                        return earth.mix(th.accent, 0.14 * self.shown[i] as f64 / 3.0);
                    }
                    let (x, y) = (i % self.pw, i / self.pw);
                    let deep = y as f64 * 2.0 + 1.0 - self.ground(x as f64);
                    if deep < 0.0 {
                        return th.bg;
                    }
                    // What's dug, and the way in, through the mound.
                    if self.hole[i] || (x as f64 + 0.5 - self.nest.0).abs() < 1.5 && (y as f64 * 2.0 + 1.0) < self.gy {
                        return void;
                    }
                    // Streaks of the darker tone, each a cell deep and a
                    // few across, no two rows of them lined up: a terminal
                    // draws a run of one color cheaply, which grains of two
                    // colors in every cell aren't.
                    let row = y / 2;
                    let patch = (x + row.wrapping_mul(2654435761) % 12) / 12;
                    let tone = if (patch.wrapping_mul(7919) ^ row.wrapping_mul(104729)) % 5 == 0 { b } else { a };
                    let top = if deep < 2.5 && !self.gel { 0.25 } else { 0.0 };
                    th.bg.mix(tone.mix(Rgb(40, 60, 30), top), much)
                })
                .collect();
        }
        field.copy_from_slice(&self.cache);
    }

    /// What's piled in a chamber: n bits, lying from its floor up.
    fn pile(&self, px: &mut [Rgb], node: usize, n: usize, c: Rgb, salt: usize) {
        let (at, t) = (self.nodes[node].end(), &self.nodes[node]);
        for k in 0..n {
            let (h1, h2) = (((k + salt) * 7919 % 97) as f64 / 97.0, ((k + salt) * 104729 % 89) as f64 / 89.0);
            let oy = t.ry * (0.95 - 1.5 * (k as f64 / 60.0).min(1.0) * h1);
            let ox = t.rx * 0.9 * (2.0 * h2 - 1.0) * (1.0 - (oy / t.ry).powi(2)).max(0.0).sqrt();
            put(px, self.pw, at.0 + ox, at.1 + oy - 1.0, c, 0.9);
        }
    }

    /// A leaf: pointed at its ends, as much of it as is left.
    fn leaf(&self, px: &mut [Rgb], c: (f64, f64), rx: f64, ry: f64) {
        for py in ((c.1 - ry) / 2.0).floor() as i32..=((c.1 + ry) / 2.0).ceil() as i32 {
            for qx in (c.0 - rx).floor() as i32..=(c.0 + rx).ceil() as i32 {
                let (dx, dy) = ((qx as f64 + 0.5 - c.0) / rx.max(0.1), (py as f64 * 2.0 + 1.0 - c.1) / ry.max(0.1));
                if dx * dx + dy * dy * (1.0 + 2.0 * dx.abs()) <= 1.0 {
                    put(px, self.pw, qx as f64, py as f64 * 2.0, LEAF, 0.95);
                }
            }
        }
    }

    fn load(&self, a: &Ant, body: Rgb, th: &Theme) -> Option<(Rgb, bool)> {
        match a.carry {
            Carry::None => None,
            Carry::Grain => Some((self.tones.0.mix(th.fg, 0.25), false)),
            Carry::Food => Some((CRUMB, false)),
            Carry::Leaf => Some((LEAF, true)),
            Carry::Egg(_) => Some((BROOD, false)),
            Carry::Dead => Some((body.mix(Rgb(0, 0, 0), 0.55), false)),
        }
    }

    pub fn draw(&self, px: &mut [Rgb], t: f64, th: &Theme) {
        let h = habit(self.species);
        // Seen against what they're on, whatever the talk's colors.
        let under = if self.top { th.bg.mix(self.tones.0, self.tones.2 * 0.3) } else { th.bg.mix(self.tones.0, 0.1) };
        let body = if far(h.body, under) < 150 { h.body.mix(th.fg, 0.7) } else { h.body };
        match self.top {
            true => self.above(px, body, th),
            false => self.beside(px, t, body, th),
        }
    }

    /// From above: the plants, the food, the nest's hole in its ring of
    /// what was dug out, and those that are out.
    fn above(&self, px: &mut [Rgb], body: Rgb, th: &Theme) {
        let pw = self.pw;
        for p in &self.plants {
            for (k, &(_, _, left)) in p.leaves.iter().enumerate() {
                let ang = k as f64 * 2.4;
                if left > 0.05 {
                    self.leaf(px, (p.x + 4.0 * ang.cos(), p.z + 4.0 * ang.sin()), 4.0 * left.sqrt(), 3.0 * left.sqrt());
                }
            }
            put(px, pw, p.x, p.z, STEM, 1.0);
        }
        // A pile: a scatter as wide as there's much of it.
        for p in &self.piles {
            for k in 0..p.2.max(0.0) as usize {
                let (ang, out) = (k as f64 * 2.4, 0.9 * (k as f64).sqrt());
                put(px, pw, p.0 + out * ang.cos(), p.1 + out * ang.sin(), CRUMB, 0.9);
            }
        }
        let n = self.nest;
        let wide = (3.0 + self.mound.iter().sum::<f64>().sqrt() * 0.3).min(9.0);
        for k in 0..48 {
            let ang = k as f64 / 48.0 * TAU;
            let mut out = 3.0;
            while out <= wide {
                put(px, pw, n.0 + out * ang.cos(), n.1 + out * ang.sin(), self.tones.0, 0.75 - 0.4 * (out - 3.0) / wide);
                out += 1.0;
            }
        }
        for (dx, dy) in [(0.0, 0.0), (-1.0, 0.0), (1.0, 0.0), (0.0, -1.5), (0.0, 1.5)] {
            put(px, pw, n.0 + dx, n.1 + dy, th.bg.mix(Rgb(0, 0, 0), 0.5), 1.0);
        }
        for d in &self.dead {
            if let Loc::Surface(x, z) = d.0 {
                put(px, pw, x, z, body.mix(Rgb(0, 0, 0), 0.55), 1.0);
            }
        }
        for a in &self.ants {
            if let Loc::Surface(x, z) = a.loc {
                ant(px, pw, (x, z), (a.heading.cos(), a.heading.sin()), body, a.big, self.load(a, body, th));
            }
        }
    }

    /// From the side: the plants, the nest and what's in it, and everyone.
    fn beside(&self, px: &mut [Rgb], t: f64, body: Rgb, th: &Theme) {
        let pw = self.pw;
        // Grass, where the ground's bare; the plants, their leaves as
        // much as is left of them.
        for k in 0..self.pw / 3 {
            let x = (k * 53 % self.pw) as f64;
            if (x - self.nest.0).abs() > 14.0 && !self.gel {
                put(px, pw, x, self.ground(x) - 1.0, STEM, 0.7);
            }
        }
        for (p, pl) in self.plants.iter().enumerate() {
            let mut up = 0.0;
            while up <= pl.h {
                let s = self.stem(p, up);
                put(px, pw, s.0, s.1, STEM, 1.0);
                up += 0.5;
            }
            for &(up, side, left) in &pl.leaves {
                let (c, rx, ry) = (self.stem(p, up), 5.5 * left.sqrt(), 3.4 * left.sqrt());
                if left > 0.05 {
                    self.leaf(px, (c.0 + side * (rx + 0.5), c.1 + 0.6 * (t * 0.8 + up).sin()), rx, ry);
                }
            }
        }
        for p in &self.piles {
            for k in 0..(p.2 / 3.0).ceil() as usize {
                put(px, pw, p.0 + k as f64 - 1.0, self.ground(p.0) - 1.0, CRUMB, 1.0);
            }
        }
        // The store: leaf turned to fungus, or what was gathered. The
        // midden's dark.
        let leaf = habit(self.species).leaf;
        if let Some(s) = self.room(Room::Store) {
            self.pile(px, s, (self.stock * 1.5) as usize, if leaf { FUNGUS } else { CRUMB }, 3);
        }
        if let Some(m) = self.room(Room::Midden) {
            self.pile(px, m, self.refuse.min(50), body.mix(Rgb(0, 0, 0), 0.6), 11);
        }
        for b in &self.brood {
            let c = self.nodes[b.0].end();
            let (lit, wide) = if b.3 < LARVA { (0.6, false) } else if b.3 < PUPA { (1.0, true) } else { (0.85, true) };
            let tone = if b.3 < PUPA { BROOD } else { BROOD.mix(CRUMB, 0.5) };
            put(px, pw, c.0 + b.1, c.1 + b.2, tone, lit);
            if wide {
                put(px, pw, c.0 + b.1 + 1.0, c.1 + b.2, tone, lit * 0.8);
            }
        }
        for d in &self.dead {
            let at = self.spot(d.0, 1.0);
            put(px, pw, at.0, at.1, body.mix(Rgb(0, 0, 0), 0.55), 1.0);
        }
        // The queen: long, dark behind, never still, never going anywhere.
        if let Some(q) = self.room(Room::Queen) {
            let c = self.nodes[q].end();
            let x = c.0 - 1.0 + 0.8 * (t * 0.4).sin();
            for k in 0..5 {
                put(px, pw, x - k as f64, c.1 + 0.5, if k == 0 { body } else { body.mix(Rgb(0, 0, 0), 0.25 + 0.05 * k as f64) }, 1.0);
            }
            put(px, pw, x - 2.0, c.1 - 1.5, body.mix(Rgb(0, 0, 0), 0.3), 1.0);
            put(px, pw, x - 3.0, c.1 - 1.5, body.mix(Rgb(0, 0, 0), 0.3), 1.0);
        }
        for a in &self.ants {
            // Just hatched, it's pale a while.
            let c = if a.age < 30.0 { body.mix(BROOD, 0.5 * (1.0 - a.age / 30.0)) } else { body };
            let (at, dir) = match (a.loc, a.found) {
                // Cutting: up the plant, at the leaf.
                (Loc::Surface(..), Some(Found::Plant(p))) if a.wait > 0.0 => (self.stem(p, self.plants[p].leaves[a.aim].0), (1.0, 0.0)),
                (Loc::Surface(..), _) => (a.at, (a.heading.cos().signum(), 0.0)),
                _ => (a.at, a.dir),
            };
            ant(px, pw, at, dir, c, a.big, self.load(a, body, th));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colony(said: &str, secs: usize) -> Colony {
        let mut c = Colony::new(160, 60, 5, said, Some(30));
        for k in 0..secs * 30 {
            c.step(1.0 / 30.0, k as f64 / 30.0, &[], &[]);
        }
        c
    }

    /// Everyone underground is in what's dug, or at its very edge.
    fn all_in_holes(c: &Colony) {
        for a in &c.ants {
            let (x, y) = ((a.at.0 as usize).min(c.pw - 1), ((a.at.1 / 2.0) as usize).min(c.ph - 1));
            let ok = match a.loc {
                Loc::Surface(..) => a.at.1 <= c.gy,
                _ => (-1..=1).any(|d| c.hole[(y * c.pw + x).saturating_add_signed(d)]) || c.hole[(y + 1).min(c.ph - 1) * c.pw + x] || c.hole[y.saturating_sub(1) * c.pw + x],
            };
            assert!(ok, "{:?} at {:?}", a.loc, a.at);
        }
    }

    #[test]
    fn the_nest_is_dug_by_those_in_it() {
        let c = colony("", 0);
        let holes = c.hole.iter().filter(|h| **h).count();
        assert!(c.site().is_some() && c.room(Room::Queen).is_some() && c.room(Room::Store).is_some());
        let c = colony("", 240);
        // More's dug, and what came out is on the mound.
        assert!(c.hole.iter().filter(|h| **h).count() > holes + 20);
        assert!(c.mound.iter().sum::<f64>() > 10.0);
        all_in_holes(&c);
    }

    #[test]
    fn leaves_are_cut_eggs_laid_and_the_dead_fetched() {
        let mut c = colony("", 0);
        let (leaf, ants) = (c.plants.iter().flat_map(|p| &p.leaves).map(|l| l.2).sum::<f64>(), c.ants.len());
        let mut carried = [false; 4];
        let (mut most, mut least) = (0, leaf);
        for k in 0..600 * 30 {
            c.step(1.0 / 30.0, k as f64 / 30.0, &[], &[]);
            for a in &c.ants {
                match a.carry {
                    Carry::Leaf => carried[0] = true,
                    Carry::Grain => carried[1] = true,
                    Carry::Egg(_) => carried[2] = true,
                    Carry::Dead => carried[3] = true,
                    _ => {}
                }
            }
            most = most.max(c.ants.len());
            least = least.min(c.plants.iter().flat_map(|p| &p.leaves).map(|l| l.2).sum::<f64>());
        }
        assert_eq!(carried, [true; 4]);
        assert!(least < leaf);
        // It grew, to no more than it's let; and the dead don't pile up.
        assert!(most > ants && most <= 30, "{most}");
        assert!(c.dead.len() < 6 && c.refuse > 0, "{} {}", c.dead.len(), c.refuse);
    }

    #[test]
    fn a_screen_too_small_for_a_nest_still_runs() {
        let th = Theme::default();
        for said in ["", "black", "fire gel"] {
            for (pw, ph) in [(16, 8), (40, 12), (6, 40)] {
                let mut c = Colony::new(pw, ph, 3, said, None);
                let (mut field, text) = (vec![Rgb::default(); pw * ph], vec![false; pw * ph / 4]);
                for k in 0..20 * 30 {
                    c.view(k % 200 < 100);
                    c.step(1.0 / 30.0, k as f64 / 30.0, &text, &[(2.0, 1.0)][..(k % 150 == 5) as usize]);
                    c.back(&mut field, &th);
                    c.draw(&mut field, k as f64 / 30.0, &th);
                }
            }
        }
    }

    #[test]
    fn one_colony_seen_two_ways() {
        // Food dropped seen from above is found, carried down the hole,
        // and in the store, seen from the side; and it left a trail.
        let mut c = Colony::new(160, 60, 9, "black sand", Some(40));
        let mut text = vec![false; 80 * 30];
        for r in 12..16 {
            text[r * 80 + 20..r * 80 + 60].fill(true);
        }
        c.view(true);
        let (mut stored, mut trail, mut down) = (0.0, false, false);
        for k in 0..240 * 30 {
            let was = c.stock;
            c.step(1.0 / 30.0, k as f64 / 30.0, &text, &[(30.0, 50.0)][..(k == 0) as usize]);
            stored += (c.stock - was).max(0.0);
            trail |= c.scent.iter().any(|v| *v > 0.3);
            down |= c.ants.iter().any(|a| a.carry == Carry::Food && !matches!(a.loc, Loc::Surface(..)));
            // None on the text, once it's had time to walk off it.
            for a in &c.ants {
                if let Loc::Surface(x, z) = a.loc {
                    assert!(k < 90 || !c.blocked(x, z) || x < 1.0 || z < 1.0 || x >= c.w - 1.0 || z >= c.h - 1.0, "frame {k}");
                }
            }
        }
        assert!(stored > 8.0 && trail && down, "{stored} {trail} {down}");
        all_in_holes(&c);
    }

    #[test]
    fn a_resize_keeps_the_colony() {
        let mut c = colony("", 120);
        let (ants, dug, brood, stock) = (c.ants.len(), c.nodes.iter().filter(|n| n.done()).count(), c.brood.len(), c.stock);
        let frac: Vec<f64> = c.nodes.iter().map(|n| if n.len > 0.0 { n.dug / n.len } else { 0.0 }).collect();
        c.resize(220, 90);
        assert_eq!((c.ants.len(), c.nodes.iter().filter(|n| n.done()).count(), c.brood.len(), c.stock), (ants, dug, brood, stock));
        for (n, f) in c.nodes.iter().zip(&frac) {
            assert!(n.len == 0.0 || (n.dug / n.len - f).abs() < 1e-9);
        }
        assert_eq!((c.hole.len(), c.mound.len(), c.size()), (220 * 90, 220, (220, 90)));
        all_in_holes(&c);
        // And it goes on, smaller too.
        c.resize(60, 20);
        for k in 0..60 * 30 {
            c.step(1.0 / 30.0, 120.0 + k as f64 / 30.0, &[], &[]);
        }
        all_in_holes(&c);
    }
}
