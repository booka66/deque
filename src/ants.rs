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
//! A colony begins with a queen alone and her first eggs, and lives by a
//! clock of its own: days and nights, four seasons. By night, in winter and
//! in rain the foragers stay in. What an ant does goes by what it is and
//! how old: the small and the young tend the brood, which they carry up to
//! the nursery by day and down to the queen at night; the middle-aged dig
//! and clear the dead; the old go out; the big keep watch. Those that meet
//! underground stop to share food. One that finds plenty leads another
//! back to it. Once a year, in summer, the winged are raised, and fly.
//!
//! A queen has her years, and when she's gone and her last worker with
//! her, another comes and begins again in the same nest. Honeypot ants
//! keep their food in workers hanging from the store's roof; army ants dig
//! nothing, and move on every evening.
//!
//! Its keeper can follow one ant, name the chambers, stop its time or
//! hurry it, and read what's happened to it. It's kept on disk, and lives
//! through the time deque wasn't running. Piles of food can stand for
//! tasks: there as long as the task is.
//!
//! The ground has its own life. Stones lie in the soil, and tunnels go
//! round them; a seed from a full midden sprouts. A ladybird comes for the
//! aphids till an ant drives it off. Rain stops up the door and washes the
//! trails away. Garden ants milk the aphids on the plants. Another colony
//! across the way sends its own foragers, and where they meet ours, they
//! fight. Twigs fall, and lie in the way till
//! they've rotted. Now and then a spider comes hunting: it eats a few of
//! those that are out and goes, unless the ants that fight, majors and
//! fire ants, bring it down, and then it's food.
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
    Honeypot,
    Army,
}

/// What a species looks like and does: its color, how fast it walks,
/// whether it cuts leaves, and how many in a hundred are majors, the big
/// ones.
struct Habit {
    body: Rgb,
    pace: f64,
    leaf: bool,
    majors: i32,
    minims: i32,
}

fn habit(s: Species) -> Habit {
    match s {
        Species::Leafcutter => Habit { body: Rgb(176, 84, 44), pace: 9.0, leaf: true, majors: 20, minims: 30 },
        Species::Black => Habit { body: Rgb(34, 32, 38), pace: 10.0, leaf: false, majors: 0, minims: 0 },
        Species::Fire => Habit { body: Rgb(214, 70, 38), pace: 14.0, leaf: false, majors: 12, minims: 20 },
        Species::Honeypot => Habit { body: Rgb(176, 112, 52), pace: 10.0, leaf: false, majors: 0, minims: 0 },
        Species::Army => Habit { body: Rgb(150, 74, 34), pace: 13.0, leaf: false, majors: 15, minims: 0 },
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
const TWIG: Rgb = Rgb(134, 100, 62);
const SPIDER: Rgb = Rgb(70, 56, 50);
const APHID: Rgb = Rgb(196, 224, 140);
const RAIN: Rgb = Rgb(128, 168, 214);
const HONEY: Rgb = Rgb(236, 172, 58);
const STONE: Rgb = Rgb(126, 124, 118);
const LADYBIRD: Rgb = Rgb(222, 44, 34);

/// What's happened to a colony, said: by its number, and how many.
fn tell(code: u8, n: f64) -> String {
    let room = |n: f64| ["a tunnel", "the store", "the nursery", "the midden", "the queen's chamber", "another chamber"][(n as usize).min(5)];
    match code {
        0 => "a queen founded it".into(),
        1 => "the first worker hatched".into(),
        2 => format!("{} was dug", room(n)),
        3 => "a spider was brought down, and eaten".into(),
        4 => format!("a spider took {n:.0}"),
        5 => format!("{n:.0} winged flew"),
        6 => format!("winter came, {n:.0} strong"),
        7 => format!("spring: {n:.0} came through the winter"),
        8 => "the queen died".into(),
        9 => "a new queen came, and began again".into(),
        10 if n >= 120.0 => format!("left alone {:.0} hours", n / 60.0),
        10 => format!("left alone {n:.0} minutes"),
        11 => "a seed from the midden sprouted".into(),
        12 => "the colony moved on".into(),
        13 => "a plant died".into(),
        _ => "a seedling came up".into(),
    }
}

/// How far a point is from the stretch between two others.
fn off(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let u = (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy).max(1e-9)).clamp(0.0, 1.0);
    ((p.0 - a.0 - dx * u).powi(2) + (p.1 - a.1 - dy * u).powi(2)).sqrt()
}

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
/// gaster, darker. A minim (size 0) is its head alone; a major (2) a pixel
/// longer. What it carries goes over its head, or before it.
fn ant(px: &mut [Rgb], pw: usize, at: (f64, f64), dir: (f64, f64), body: Rgb, size: u8, load: Option<(Rgb, bool)>) {
    let across = dir.0.abs() >= dir.1.abs();
    let back = if across { (-dir.0.signum(), 0.0) } else { (0.0, -2.0 * dir.1.signum()) };
    put(px, pw, at.0, at.1, body, 1.0);
    if size > 0 && (across || size > 1) {
        put(px, pw, at.0 + back.0, at.1 + back.1, body.mix(Rgb(0, 0, 0), 0.35), 1.0);
    }
    if across && size > 1 {
        put(px, pw, at.0 + 2.0 * back.0, at.1, body.mix(Rgb(0, 0, 0), 0.35), 1.0);
    }
    if let Some((c, over)) = load {
        match over {
            true => {
                put(px, pw, at.0, at.1 - 2.0, c, 1.0);
                put(px, pw, at.0 - back.0, at.1 - 2.0, c, if size > 1 { 1.0 } else { 0.6 });
            }
            false => put(px, pw, at.0 - back.0, at.1 - back.1, c, 1.0),
        }
    }
}

/// Numbers in a row, written out and read back: a colony as it is, for
/// another deque to show.
struct Out(Vec<u8>);

impl Out {
    fn n(&mut self, v: f64) {
        self.0.extend(v.to_le_bytes());
    }
    fn all<const N: usize>(&mut self, v: [f64; N]) {
        v.into_iter().for_each(|x| self.n(x));
    }
    fn loc(&mut self, l: Loc) {
        self.all(match l {
            Loc::Surface(x, z) => [0.0, x, z, 0.0],
            Loc::Edge(e, s) => [1.0, e as f64, s, 0.0],
            Loc::Room(n, ox, oy) => [2.0, n as f64, ox, oy],
            Loc::Air(x, z, up) => [3.0, x, z, up],
        });
    }
}

struct In<'a>(&'a [u8]);

impl In<'_> {
    fn n(&mut self) -> Option<f64> {
        let (a, rest) = self.0.split_first_chunk::<8>()?;
        self.0 = rest;
        Some(f64::from_le_bytes(*a)).filter(|v| v.is_finite())
    }
    fn all<const N: usize>(&mut self) -> Option<[f64; N]> {
        let mut v = [0.0; N];
        for x in &mut v {
            *x = self.n()?;
        }
        Some(v)
    }
    /// A count, or a place in a list: a whole number less than `under`.
    fn u(&mut self, under: usize) -> Option<usize> {
        self.n().filter(|v| *v >= 0.0 && *v < under as f64).map(|v| v as usize)
    }
    /// A place, in a nest of `nodes` places.
    fn loc(&mut self, nodes: usize) -> Option<Loc> {
        let [tag, a, b, c] = self.all()?;
        let node = (a >= 0.0 && a < nodes as f64).then_some(a as usize);
        Some(match tag as u8 {
            0 => Loc::Surface(a, b),
            1 => Loc::Edge(node?, b),
            2 => Loc::Room(node?, b, c),
            _ => Loc::Air(a, b, c),
        })
    }
}

/// What a snapshot starts with: another, when what's in one changes.
const SNAPSHOT: f64 = 8.0;
const ROOMS: [Room; 6] = [Room::None, Room::Store, Room::Nursery, Room::Midden, Room::Queen, Room::Spare];
const JOBS: [Job; 7] = [Job::Dig, Job::Forage, Job::Nurse, Job::Bury, Job::Rest, Job::Guard, Job::Fly];
const CASTES: [Caste; 4] = [Caste::Minim, Caste::Media, Caste::Major, Caste::Alate];

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
    /// On the wing: over this place on the ground, this high.
    Air(f64, f64, f64),
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
    /// After another ant, by its number, that's leading it to food.
    Follow(usize),
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Job {
    Dig,
    Forage,
    Nurse,
    Bury,
    Rest,
    /// Out by the door, keeping watch.
    Guard,
    /// Winged, waiting for the day, then off.
    Fly,
}

/// What an ant was raised as: the smallest, who tend the brood and the
/// garden; the workers; the big ones, who guard and fight; and the winged,
/// who leave.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Caste {
    Minim,
    Media,
    Major,
    Alate,
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
    caste: Caste,
    /// Which side of a tunnel it keeps to.
    side: f64,
    ph: f64,
    /// Its number, for one that follows it.
    id: usize,
    /// Stopped head to head with another, sharing food, this much longer;
    /// and how long before it will again.
    greet: f64,
    met: f64,
    /// A minim riding the leaf it carries, keeping flies off it.
    rider: bool,
    /// Where it last found plenty, to go back to, and lead another to.
    knows: Option<(f64, f64)>,
    /// How many times it's brought food home, and when it hatched, by
    /// the colony's clock.
    trips: usize,
    born: f64,
}

struct Plant {
    x: f64,
    z: f64,
    h: f64,
    lean: f64,
    /// Each leaf: how high up, which side, and how much of it is left.
    leaves: Vec<(f64, f64, f64)>,
    /// Aphids on it, for garden ants to milk; and how much longer a
    /// ladybird's on it, eating them.
    aphids: f64,
    bug: f64,
    /// How well it is, 1 down to 0, when it's dead; how much longer it
    /// stands there dead, before it's gone; and how old it is.
    health: f64,
    wither: f64,
    age: f64,
}

impl Plant {
    fn alive(&self) -> bool {
        self.health > 0.0
    }
    /// Dead and fallen: nothing there, till something grows there again.
    fn gone(&self) -> bool {
        !self.alive() && self.wither <= 0.0
    }
}

/// One of another colony's, out on the same ground: where, which way,
/// whether it's carrying something home, and how long it's locked in a
/// fight.
struct Rival {
    x: f64,
    z: f64,
    dir: f64,
    has: bool,
    grip: f64,
}

/// A twig fallen on the ground, from one end to the other: nothing walks
/// through it, till it's rotted away. How much of it is left, 1 to 0.
struct Twig {
    a: (f64, f64),
    b: (f64, f64),
    left: f64,
}

/// A spider come hunting: where it is and which way it's going, how much
/// fight is left in it, how many it's eaten, how long it's busy eating,
/// and how long before it gives up and goes.
struct Spider {
    x: f64,
    z: f64,
    dir: f64,
    hp: f64,
    fed: usize,
    bite: f64,
    stay: f64,
}

/// How wide a tunnel is, half across.
const BORE: f64 = 2.1;
/// How long a day is, in seconds, and how many days a year: five of
/// spring, six of summer, four of autumn, and one of winter.
const DAY: f64 = 480.0;
const YEAR: f64 = 16.0;
const SEASONS: [&str; 4] = ["spring", "summer", "autumn", "winter"];
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
    /// What's fallen on the ground, and how long till the next falls; the
    /// spider, when one's about, and how long till the next comes.
    twigs: Vec<Twig>,
    fall: f64,
    hunter: Option<Spider>,
    prowl: f64,
    /// The colony across the way: where its nest is, those of it that are
    /// out, and how long till it sends another.
    camp: (f64, f64),
    rivals: Vec<Rival>,
    muster: f64,
    /// Leaf brought in and not yet turned to fungus.
    leaf: f64,
    /// The winged: the year they were last raised, how many are still to
    /// raise this year and how long till the next, and how many have flown.
    flew: f64,
    raise: usize,
    wing: f64,
    flown: usize,
    /// How long till those underground next stop to share food.
    social: f64,
    /// The ant its keeper's following, by its number.
    follow: Option<usize>,
    /// How fast its time goes, for its keeper: stopped, as it is, or
    /// several times over.
    speed: usize,
    /// What's happened: the day, what, and how many. And how many workers
    /// there were, four times a day, for a year back.
    events: Vec<(f64, u8, f64)>,
    pops: Vec<f64>,
    sampled: f64,
    season_was: usize,
    /// The queen: whether she's alive, how long she's reigned, and how
    /// long she has.
    queen: bool,
    reign: f64,
    span: f64,
    /// When it was last written out, in seconds since 1970.
    stamp: f64,
    /// Stones in the soil, each where it is and how big: tunnels go round.
    stones: Vec<(f64, f64, f64, f64)>,
    /// The piles of food that stand for tasks, by their numbers: there as
    /// long as the task is.
    tasks: Vec<usize>,
    /// Those flown in the flight going on; and for army ants, where
    /// they're moving to, since when, and the day they last moved.
    flock: usize,
    trek: Option<(f64, f64, f64)>,
    moved: f64,
    /// Eggs, larvae and pupae: the chamber, where in it, and how old.
    brood: Vec<(usize, f64, f64, f64)>,
    text: Vec<bool>,
    count: usize,
    stock: f64,
    refuse: usize,
    lay: f64,
    /// How long the colony's lived, in seconds: the time of day and of
    /// year go by it.
    clock: f64,
    /// How much longer it's raining, how long till it next does, and how
    /// far the nest's door is stopped up against it, 0 to 1.
    rain: f64,
    cloud: f64,
    plug: f64,
    /// The background, kept till something changes it, with the color, the
    /// view, and the light and weather it was made for.
    cache: Vec<Rgb>,
    made: Option<(Rgb, bool, u8)>,
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
        } else if has("honeypot") {
            Species::Honeypot
        } else if has("army") {
            Species::Army
        } else {
            Species::Leafcutter
        };
        let (w, h) = (pw as f64, ph as f64 * 2.0);
        let usual = Self::usual(pw, ph, species);
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
            twigs: vec![],
            fall: 40.0,
            hunter: None,
            prowl: 70.0,
            camp: (0.0, 0.0),
            rivals: vec![],
            muster: 30.0,
            leaf: 0.0,
            flew: -1.0,
            raise: 0,
            wing: 0.0,
            flown: 0,
            social: 1.0,
            follow: None,
            speed: 1,
            events: vec![],
            pops: vec![],
            sampled: 0.0,
            season_was: 0,
            queen: true,
            reign: 0.0,
            span: DAY * YEAR * (2.5 + (seed % 100) as f64 / 100.0),
            stamp: 0.0,
            stones: vec![],
            tasks: vec![],
            flock: 0,
            trek: None,
            moved: -1.0,
            brood: vec![],
            text: vec![],
            count: 0,
            stock: 10.0,
            refuse: 0,
            lay: 4.0,
            // A spring morning.
            clock: DAY * 8.0 / 24.0,
            rain: 0.0,
            cloud: 420.0,
            plug: 0.0,
            cache: vec![],
            made: None,
        };
        c.plan();
        match has("grown") {
            true => c.settle(),
            false => c.found(),
        }
        c.note(0, 0.0);
        c
    }

    /// How many workers at most suit a screen, when the talk doesn't say.
    fn usual(pw: usize, ph: usize, species: Species) -> usize {
        (pw * ph / 220).clamp(16, 110) * if species == Species::Fire { 3 } else { 2 } / 2
    }

    /// Which colony it is: the same number, the same colony.
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn seed(&self) -> u32 {
        self.seed
    }

    /// The screen it's on, in pixels.
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

    /// How big the screen is beside the one the colony's paces and
    /// distances were set for, 110 cells by 32: on a bigger one they walk
    /// faster, see further, and dig more at a time, in proportion, so a
    /// trip to a plant, or down the shaft, takes as long on any.
    fn scale(&self) -> f64 {
        ((self.w / 220.0) * (self.h / 128.0)).sqrt().clamp(0.3, 4.0)
    }

    /// Whether its kind milks aphids.
    fn milks(&self) -> bool {
        matches!(self.species, Species::Black | Species::Honeypot)
    }

    /// Something happened, to be remembered: the last sixty are.
    fn note(&mut self, code: u8, n: f64) {
        self.events.push((self.clock / DAY, code, n));
        if self.events.len() > 60 {
            self.events.remove(0);
        }
    }

    /// As many piles of food as there are tasks, each there as long as
    /// its task is: more put down, or the last taken up.
    pub fn tasks(&mut self, n: Option<usize>) {
        let n = n.unwrap_or(0).min(40);
        while self.tasks.len() > n {
            let id = self.tasks.pop().unwrap();
            self.piles.retain(|p| p.3 != id);
        }
        while self.tasks.len() < n {
            self.count += 1;
            let at = self.somewhere(26.0 * self.scale());
            self.piles.push((at.0, at.1, 12.0, self.count));
            self.tasks.push(self.count);
        }
    }

    /// A key its keeper pressed: f follows an ant, or lets it go; [ and ]
    /// slow its time and quicken it.
    pub fn key(&mut self, ch: char) {
        match ch {
            'f' if self.follow.is_some() => self.follow = None,
            'f' => {
                // One that can be seen from here, if there's one.
                let seen = |a: &Ant| !self.top || matches!(a.loc, Loc::Surface(..));
                let all: Vec<usize> = self.ants.iter().filter(|a| seen(a)).map(|a| a.id).collect();
                self.follow = (!all.is_empty()).then(|| all[self.rng.below(all.len() as i32) as usize]);
            }
            '[' => self.speed = self.speed.saturating_sub(1),
            ']' => self.speed = (self.speed + 1).min(3),
            _ => {}
        }
    }

    /// Its time as it goes, neither stopped nor hurried: for a pane that
    /// only shows what another keeps.
    pub fn unhurried(&mut self) {
        self.speed = 1;
    }

    /// How many moments of its time go by in one of its keeper's.
    pub fn speed(&self) -> usize {
        [0, 1, 4, 16][self.speed]
    }

    /// The ant at a place on the screen, in pixels, taken to follow, if
    /// there's one there.
    pub fn pick(&mut self, x: f64, y: f64) -> bool {
        let near = |a: &Ant| {
            let at = match (self.top, a.loc) {
                (true, Loc::Surface(ax, az)) => (ax, az),
                (true, _) => return f64::MAX,
                _ => a.at,
            };
            (at.0 - x).powi(2) + (at.1 - y * 2.0).powi(2)
        };
        let best = self.ants.iter().map(|a| (near(a), a.id)).min_by(|a, b| a.0.total_cmp(&b.0));
        match best {
            Some((d, id)) if d < 9.0 => {
                self.follow = Some(id);
                true
            }
            _ => false,
        }
    }

    /// What its chambers are, and where, in cells from the top left, for
    /// naming them on the screen: from the side only.
    pub fn labels(&self) -> Vec<(i32, i32, &'static str)> {
        if self.top {
            return vec![];
        }
        let leaf = habit(self.species).leaf;
        self.nodes
            .iter()
            .filter(|n| n.room != Room::None && n.done())
            .map(|n| {
                let name = match n.room {
                    Room::Store if leaf => "garden",
                    Room::Store => "store",
                    Room::Nursery => "nursery",
                    Room::Midden => "midden",
                    Room::Queen if self.species == Species::Army => "bivouac",
                    Room::Queen => "queen",
                    _ => "chamber",
                };
                let c = n.end();
                (((c.1 - n.ry) / 4.0) as i32, (c.0 / 2.0) as i32 - name.len() as i32 / 2 + 1, name)
            })
            .collect()
    }

    /// What's happened to it, for its keeper: how many workers it's had,
    /// day by day, drawn small; then the last things, the newest last.
    pub fn history(&self) -> Vec<String> {
        const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let most = self.pops.iter().copied().fold(1.0, f64::max);
        let graph: String = self.pops.iter().map(|p| BARS[((p / most * 7.0).round() as usize).min(7)]).collect();
        let mut out = vec![format!("workers, the last {} days: most {most:.0}", self.pops.len().div_ceil(4)), graph, String::new()];
        out.extend(self.events.iter().rev().take(14).rev().map(|(day, code, n)| format!("day {:>3}  {}", *day as usize + 1, tell(*code, *n))));
        out
    }

    /// The time it was away made up: as much of it lived through, quickly,
    /// as a moment allows, and the rest only gone by on its clock.
    pub fn catch_up(&mut self, now: f64) {
        let away = now - self.stamp;
        if self.stamp <= 0.0 || away < 30.0 {
            return;
        }
        let lived = away.min(1200.0);
        let mut t = 0.0;
        while t < lived {
            self.step(0.1, t, &[], &[]);
            t += 0.1;
        }
        self.clock += (away - lived).min(DAY * YEAR * 20.0);
        self.season_was = self.season();
        self.note(10, away / 60.0);
    }

    /// The hour of the colony's day, 0 to 24.
    fn hour(&self) -> f64 {
        (self.clock / DAY).fract() * 24.0
    }

    /// How light it is, 0 at night to 1 by day: dawn's from five to seven,
    /// dusk from seven to nine.
    fn light(&self) -> f64 {
        let h = self.hour();
        ((h - 5.0) / 2.0).min((21.0 - h) / 2.0).clamp(0.0, 1.0)
    }

    /// Which season it is, from spring, 0.
    fn season(&self) -> usize {
        match (self.clock / DAY) % YEAR {
            d if d < 5.0 => 0,
            d if d < 11.0 => 1,
            d if d < 15.0 => 2,
            _ => 3,
        }
    }

    /// Whether it's a time to be out: by day, not in winter, not in rain.
    fn abroad(&self) -> bool {
        self.light() > 0.3 && self.season() != 3 && self.rain <= 0.0
    }

    /// The light and the weather, as far as they change what the ground
    /// looks like.
    fn mood(&self) -> u8 {
        (self.light() * 6.0).round() as u8 + 8 * (self.season() == 3) as u8 + 16 * (self.rain > 0.0) as u8
    }

    /// How the colony's doing, in a line, for its keeper.
    pub fn status(&self) -> String {
        // The ant being followed, if one is: its life in a line.
        if let Some(a) = self.follow.and_then(|id| self.ants.iter().find(|a| a.id == id)) {
            let what = match (a.job, a.carry) {
                _ if a.greet > 0.0 && matches!(a.loc, Loc::Surface(..)) => "in a fight",
                _ if a.greet > 0.0 => "sharing food",
                (_, Carry::Leaf) => "carrying leaf home",
                (_, Carry::Food) => "carrying food home",
                (_, Carry::Grain) => "carrying out a grain",
                (_, Carry::Egg(_)) => "carrying brood",
                (_, Carry::Dead) => "carrying off the dead",
                (Job::Dig, _) => "digging",
                (Job::Forage, _) if matches!(a.goal, Goal::Follow(_)) => "following another to food",
                (Job::Forage, _) if a.wait > 0.0 => "at a plant",
                (Job::Forage, _) => "out looking for food",
                (Job::Nurse, _) => "tending brood",
                (Job::Bury, _) => "going for the dead",
                (Job::Guard, _) => "on watch",
                (Job::Fly, _) => "waiting to fly",
                (Job::Rest, _) => "resting",
            };
            let kind = match a.caste {
                Caste::Minim => "a minim",
                Caste::Media => "a worker",
                Caste::Major => "a major",
                Caste::Alate => "winged",
            };
            let days = a.age / DAY;
            return format!("ant {} · {kind} · {days:.1} days old, of {:.1} · {what} · {} trips home", a.id, a.life / DAY, a.trips);
        }
        let h = self.hour();
        let stage = |lo: f64, hi: f64| self.brood.iter().filter(|b| b.3 >= lo && b.3 < hi).count();
        let mut s = format!("day {} · {} · {:02}:{:02}", (self.clock / DAY) as usize + 1, SEASONS[self.season()], h as usize, (h.fract() * 60.0) as usize);
        if self.rain > 0.0 {
            s += " · rain";
        }
        let of = |c: Caste| self.ants.iter().filter(|a| a.caste == c).count();
        s += &format!(" · {} workers", self.ants.len() - of(Caste::Alate));
        if of(Caste::Major) + of(Caste::Minim) > 0 {
            s += &format!(" ({} minims, {} majors)", of(Caste::Minim), of(Caste::Major));
        }
        if of(Caste::Alate) > 0 {
            s += &format!(" · {} winged", of(Caste::Alate));
        }
        if self.flown > 0 {
            s += &format!(" · {} flown", self.flown);
        }
        s += &format!(" · {} eggs {} larvae {} pupae", stage(0.0, LARVA), stage(LARVA, PUPA), stage(PUPA, f64::MAX));
        s += &format!(" · {} {:.0}", if habit(self.species).leaf { "fungus" } else { "food" }, self.stock);
        if !self.dead.is_empty() {
            s += &format!(" · {} dead", self.dead.len());
        }
        if self.hunter.is_some() {
            s += " · a spider";
        }
        if !self.rivals.is_empty() {
            s += &format!(" · {} rivals", self.rivals.len());
        }
        if !self.queen {
            s += " · no queen";
        }
        match self.speed() {
            0 => s += " · stopped",
            1 => {}
            n => s += &format!(" · ×{n}"),
        }
        s
    }

    fn r(&mut self) -> f64 {
        self.rng.below(1 << 20) as f64 / (1 << 20) as f64
    }

    /// The colony on a screen of another size: everything where it was,
    /// in proportion, the trails on the ground with it.
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
            Loc::Air(x, z, up) => Loc::Air(on(x, z).0, on(x, z).1, up * sz),
        };
        for a in &mut self.ants {
            a.loc = moved(a.loc);
            a.goal = match a.goal {
                Goal::Point(x, z) => Goal::Point(on(x, z).0, on(x, z).1),
                Goal::Spot(e, s) => Goal::Spot(e, if lens[e] > 0.0 { s / lens[e] * now[e] } else { 0.0 }),
                g => g,
            };
            a.to = (a.to.0 * sx, a.to.1 * sy);
            a.knows = a.knows.map(|k| on(k.0, k.1));
        }
        for r in &mut self.rivals {
            (r.x, r.z) = on(r.x, r.z);
        }
        for t in &mut self.stones {
            let at = under((t.0, t.1));
            *t = (at.0, at.1, t.2 * sx, t.3 * sy);
        }
        self.trek = self.trek.map(|t| (on(t.0, t.1).0, on(t.0, t.1).1, t.2));
        self.camp = on(self.camp.0, self.camp.1);
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
        for t in &mut self.twigs {
            (t.a, t.b) = (on(t.a.0, t.a.1), on(t.b.0, t.b.1));
        }
        if let Some(s) = &mut self.hunter {
            (s.x, s.z) = (s.x * sx, s.z * sz);
        }
        self.mound = (0..pw).map(|k| self.mound[((k as f64 / sx) as usize).min(self.pw - 1)] * sy.min(sx)).collect();
        self.nest = (self.nest.0 * sx, self.nest.1 * sz);
        // The scent on the ground, where it was: each pixel now takes the
        // strongest of those it covers, or the one it's a part of, so a
        // trail a pixel wide isn't lost on a smaller screen.
        let (opw, oph) = (self.pw, self.ph);
        let most = |k: usize, of: &dyn Fn(usize) -> f32| {
            let (x0, y0) = (k % pw * opw / pw, k / pw * oph / ph);
            let (x1, y1) = (((k % pw + 1) * opw / pw).max(x0 + 1).min(opw), ((k / pw + 1) * oph / ph).max(y0 + 1).min(oph));
            (y0.min(oph - 1)..y1).flat_map(|y| (x0.min(opw - 1)..x1).map(move |x| y * opw + x)).map(of).fold(0.0, f32::max)
        };
        let scent: Vec<f32> = (0..pw * ph).map(|k| most(k, &|i| self.scent[i])).collect();
        let shown: Vec<u8> = (0..pw * ph).map(|k| most(k, &|i| self.shown[i] as f32) as u8).collect();
        (self.pw, self.ph, self.w, self.h, self.gy) = (pw, ph, w, h, gy);
        (self.hole, self.scent, self.shown) = (vec![false; pw * ph], scent, shown);
        // As many as suit this screen, where that's what it was going by.
        let usual = Self::usual(pw, ph, self.species);
        if self.cap == self.usual {
            self.cap = usual;
        }
        self.usual = usual;
        self.carve();
        for i in 0..self.ants.len() {
            self.ants[i].at = self.spot(self.ants[i].loc, self.ants[i].side);
        }
    }

    /// The colony as it is, in numbers, for another deque to show: all of
    /// it but what's its own to each, the view and the text.
    pub fn snapshot(&self) -> Vec<u8> {
        let mut o = Out(vec![]);
        o.all([SNAPSHOT, self.pw as f64, self.ph as f64, self.stock, self.refuse as f64, self.lay, self.count as f64, self.nest.0, self.nest.1]);
        o.all([self.leaf, self.flew, self.raise as f64, self.flown as f64, self.camp.0, self.camp.1, self.muster]);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64());
        let trek = self.trek.unwrap_or((0.0, 0.0, 0.0));
        o.all([self.reign, self.queen as u8 as f64, self.span, now, self.season_was as f64, self.flock as f64, self.moved]);
        o.all([self.trek.is_some() as u8 as f64, trek.0, trek.1, trek.2]);
        o.n(self.nodes.len() as f64);
        for n in &self.nodes {
            o.all([n.parent as f64, ROOMS.iter().position(|r| *r == n.room).unwrap() as f64, n.rx, n.ry, n.open, n.dug, n.path.len() as f64]);
            n.path.iter().for_each(|p| o.all([p.0, p.1]));
        }
        self.mound.iter().for_each(|m| o.n(*m));
        o.n(self.ants.len() as f64);
        for a in &self.ants {
            o.loc(a.loc);
            o.all(match a.goal {
                Goal::Seek => [0.0, 0.0, 0.0],
                Goal::Point(x, z) => [1.0, x, z],
                Goal::Spot(e, s) => [2.0, e as f64, s],
                Goal::Tip(e) => [3.0, e as f64, 0.0],
                Goal::Follow(id) => [4.0, id as f64, 0.0],
            });
            o.all(match a.found {
                None => [0.0, 0.0],
                Some(Found::Plant(p)) => [1.0, p as f64],
                Some(Found::Pile(id)) => [2.0, id as f64],
            });
            o.all(match a.carry {
                Carry::None => [0.0, 0.0],
                Carry::Grain => [1.0, 0.0],
                Carry::Food => [2.0, 0.0],
                Carry::Leaf => [3.0, 0.0],
                Carry::Egg(age) => [4.0, age],
                Carry::Dead => [5.0, 0.0],
            });
            o.all([JOBS.iter().position(|j| *j == a.job).unwrap() as f64, a.stage as f64, a.aim.min(1 << 40) as f64, a.wait, a.rest, a.to.0, a.to.1]);
            o.all([a.dir.0, a.dir.1, a.heading, a.scent, a.dodge, a.age, a.life, CASTES.iter().position(|c| *c == a.caste).unwrap() as f64, a.side, a.ph]);
            let knows = a.knows.unwrap_or((0.0, 0.0));
            o.all([a.id as f64, a.greet, a.met, a.rider as u8 as f64, a.knows.is_some() as u8 as f64, knows.0, knows.1, a.trips as f64, a.born]);
        }
        o.n(self.plants.len() as f64);
        for p in &self.plants {
            o.all([p.x, p.z, p.h, p.lean, p.aphids, p.bug, p.health, p.wither, p.age, p.leaves.len() as f64]);
            p.leaves.iter().for_each(|l| o.all([l.0, l.1, l.2]));
        }
        o.n(self.piles.len() as f64);
        self.piles.iter().for_each(|p| o.all([p.0, p.1, p.2, p.3 as f64]));
        o.n(self.dead.len() as f64);
        for d in &self.dead {
            o.loc(d.0);
            o.all([d.1 as f64, d.2 as u8 as f64]);
        }
        o.n(self.brood.len() as f64);
        self.brood.iter().for_each(|b| o.all([b.0 as f64, b.1, b.2, b.3]));
        o.n(self.twigs.len() as f64);
        self.twigs.iter().for_each(|t| o.all([t.a.0, t.a.1, t.b.0, t.b.1, t.left]));
        match &self.hunter {
            Some(s) => o.all([1.0, s.x, s.z, s.dir, s.hp, s.fed as f64, s.bite, s.stay]),
            None => o.all([0.0; 8]),
        }
        o.all([self.clock, self.rain, self.cloud, self.plug]);
        o.n(self.rivals.len() as f64);
        self.rivals.iter().for_each(|r| o.all([r.x, r.z, r.dir, r.has as u8 as f64, r.grip]));
        o.n(self.stones.len() as f64);
        self.stones.iter().for_each(|t| o.all([t.0, t.1, t.2, t.3]));
        o.n(self.events.len() as f64);
        self.events.iter().for_each(|e| o.all([e.0, e.1 as f64, e.2]));
        o.n(self.pops.len() as f64);
        self.pops.iter().for_each(|v| o.n(*v));
        o.n(self.tasks.len() as f64);
        self.tasks.iter().for_each(|v| o.n(*v as f64));
        o.0.extend(&self.shown);
        o.0
    }

    /// The colony another deque wrote out, taken as this one's, on this
    /// one's screen and in its own view. Whether it could be: one that's
    /// cut short or makes no sense is left alone.
    pub fn restore(&mut self, bytes: &[u8]) -> bool {
        self.read(bytes).is_some()
    }

    fn read(&mut self, bytes: &[u8]) -> Option<()> {
        const MOST: usize = 100_000;
        let mut i = In(bytes);
        let [version, pw, ph, stock, refuse, lay, count, nx, nz] = i.all()?;
        let (pw, ph) = (pw as usize, ph as usize);
        if version != SNAPSHOT || pw == 0 || ph == 0 || pw.checked_mul(ph).is_none_or(|n| n > 4_000_000) {
            return None;
        }
        let [leaf, flew, raise, flown, cx, cz, muster] = i.all()?;
        let [reign, queen, span, stamp, season_was, flock, moved] = i.all()?;
        let [trekking, tx, tz, since] = i.all()?;
        let many = i.u(64)?;
        let mut nodes = vec![];
        for _ in 0..many {
            let [parent, room, rx, ry, open, dug, points] = i.all()?;
            let path = (0..(points as usize).clamp(1, 4096)).map(|_| i.all().map(|[x, y]| (x, y))).collect::<Option<Vec<_>>>()?;
            // Dug from a place before it, and no bigger than a screen.
            let from = (parent >= 0.0 && (parent as usize) <= nodes.len().saturating_sub(1)).then_some(parent as usize)?;
            if !(0.0..=2000.0).contains(&rx) || !(0.0..=2000.0).contains(&ry) {
                return None;
            }
            let mut t = Tunnel::new(from, path, *ROOMS.get(room as usize)?, rx, ry);
            // Dug through is dug through, however the length rounds.
            (t.open, t.dug) = (open, if t.len - dug < 1e-6 { t.len } else { dug });
            nodes.push(t);
        }
        // A nest has its way in, and a queen, or it isn't one.
        let has = |r: Room| nodes.iter().any(|n: &Tunnel| n.room == r && n.done());
        if nodes.len() < 2 || nodes[1].parent != 0 || !has(Room::Queen) {
            return None;
        }
        let mound = (0..pw).map(|_| i.n()).collect::<Option<Vec<_>>>()?;
        let mut ants = vec![];
        for _ in 0..i.u(MOST)? {
            let loc = i.loc(many)?;
            let [g, ga, gb] = i.all()?;
            let node = (ga >= 0.0 && ga < many as f64).then_some(ga as usize);
            let goal = match g as u8 {
                0 => Goal::Seek,
                1 => Goal::Point(ga, gb),
                2 => Goal::Spot(node?, gb),
                3 => Goal::Tip(node?),
                _ => Goal::Follow(ga as usize),
            };
            let [f, fv] = i.all()?;
            let found = match f as u8 {
                0 => None,
                1 => Some(Found::Plant(fv as usize)),
                _ => Some(Found::Pile(fv as usize)),
            };
            let [c, cv] = i.all()?;
            let carry = [Carry::None, Carry::Grain, Carry::Food, Carry::Leaf, Carry::Egg(cv), Carry::Dead].get(c as usize).copied()?;
            let [job, stage, aim, wait, rest, tx, ty] = i.all()?;
            let [dx, dy, heading, scent, dodge, age, life, caste, side, ph] = i.all()?;
            let [id, greet, met, rider, known, kx, kz, trips, born] = i.all()?;
            ants.push(Ant {
                loc,
                goal,
                job: *JOBS.get(job as usize)?,
                stage: stage as u8,
                aim: aim as usize,
                found,
                carry,
                wait,
                rest,
                to: (tx, ty),
                at: (0.0, 0.0),
                dir: (dx, dy),
                heading,
                scent,
                dodge,
                age,
                life,
                caste: *CASTES.get(caste as usize)?,
                side,
                ph,
                id: id as usize,
                greet,
                met,
                rider: rider != 0.0,
                knows: (known != 0.0).then_some((kx, kz)),
                trips: trips as usize,
                born,
            });
        }
        let mut plants = vec![];
        for _ in 0..i.u(64)? {
            let [x, z, h, lean, aphids, bug, health, wither, age, leaves] = i.all()?;
            let leaves = (0..(leaves as usize).clamp(1, 64)).map(|_| i.all().map(|[a, b, c]| (a, b, c))).collect::<Option<Vec<_>>>()?;
            plants.push(Plant { x, z, h: h.max(1.0), lean, leaves, aphids, bug, health, wither, age });
        }
        // What it says of a plant, a leaf or a tunnel has to be there.
        for a in &mut ants {
            let leaf = |p: usize, l: usize| plants.get(p).is_some_and(|pl: &Plant| l < pl.leaves.len());
            match a.found {
                Some(Found::Plant(p)) if !leaf(p, if a.wait > 0.0 { a.aim } else { 0 }) => (a.found, a.wait) = (None, 0.0),
                _ => {}
            }
            if a.job == Job::Dig && a.aim >= many {
                return None;
            }
        }
        let piles = (0..i.u(MOST)?).map(|_| i.all().map(|[x, z, left, id]| (x, z, left, id as usize))).collect::<Option<Vec<_>>>()?;
        let mut dead = vec![];
        for _ in 0..i.u(MOST)? {
            let loc = i.loc(many)?;
            let [id, claimed] = i.all()?;
            dead.push((loc, id as usize, claimed != 0.0));
        }
        let mut brood = vec![];
        for _ in 0..i.u(MOST)? {
            let [n, x, y, age] = i.all()?;
            brood.push(((n >= 0.0 && n < many as f64).then_some(n as usize)?, x, y, age));
        }
        let twigs = (0..i.u(64)?).map(|_| i.all().map(|[ax, az, bx, bz, left]| Twig { a: (ax, az), b: (bx, bz), left })).collect::<Option<Vec<_>>>()?;
        let [about, x, z, dir, hp, fed, bite, stay] = i.all()?;
        let hunter = (about != 0.0).then_some(Spider { x, z, dir, hp, fed: fed as usize, bite, stay });
        let [clock, rain, cloud, plug] = i.all()?;
        let rivals = (0..i.u(4096)?).map(|_| i.all().map(|[x, z, dir, has, grip]| Rival { x, z, dir, has: has != 0.0, grip })).collect::<Option<Vec<_>>>()?;
        let stones = (0..i.u(256)?).map(|_| i.all().map(|[x, y, rx, ry]| (x, y, rx, ry))).collect::<Option<Vec<_>>>()?;
        let events = (0..i.u(256)?).map(|_| i.all().map(|[day, code, n]| (day, code as u8, n))).collect::<Option<Vec<_>>>()?;
        let pops = (0..i.u(4096)?).map(|_| i.n()).collect::<Option<Vec<_>>>()?;
        let tasks = (0..i.u(256)?).map(|_| i.n().map(|v| v as usize)).collect::<Option<Vec<_>>>()?;
        if i.0.len() != pw * ph {
            return None;
        }
        // All there: it's this colony now, as big as it was written, then
        // made the size of this screen.
        let mine = (self.pw, self.ph);
        (self.nodes, self.mound, self.ants, self.plants, self.piles, self.dead, self.brood) = (nodes, mound, ants, plants, piles, dead, brood);
        (self.stock, self.refuse, self.lay, self.count, self.nest) = (stock, refuse as usize, lay, count as usize, (nx, nz));
        (self.twigs, self.hunter, self.rivals) = (twigs, hunter, rivals);
        (self.leaf, self.flew, self.raise, self.flown, self.camp, self.muster) = (leaf, flew, raise as usize, flown as usize, (cx, cz), muster);
        (self.reign, self.queen, self.span, self.stamp, self.season_was, self.flock, self.moved) = (reign, queen != 0.0, span.max(DAY), stamp, (season_was as usize).min(3), flock as usize, moved);
        self.trek = (trekking != 0.0).then_some((tx, tz, since));
        (self.stones, self.events, self.pops, self.tasks) = (stones, events, pops, tasks);
        self.sampled = self.clock;
        (self.clock, self.rain, self.cloud, self.plug) = (clock.max(0.0), rain, cloud, plug.clamp(0.0, 1.0));
        (self.pw, self.ph, self.w, self.h) = (pw, ph, pw as f64, ph as f64 * 2.0);
        self.gy = (self.h * 0.26).round();
        self.hole = vec![false; pw * ph];
        // The trails as they showed there, and scent to match, for when
        // this one's keeping it: the resize brings them here.
        self.shown = i.0.iter().map(|s| (*s).min(3)).collect();
        self.scent = self.shown.iter().map(|s| *s as f32 / 4.5).collect();
        self.carve();
        self.resize(mine.0, mine.1);
        for k in 0..self.ants.len() {
            self.ants[k].at = self.spot(self.ants[k].loc, self.ants[k].side);
        }
        self.made = None;
        Some(())
    }

    /// What there is of it, whatever screen it's on: for telling one
    /// colony's state from another's.
    #[cfg(test)]
    pub fn snapshot_of(&self) -> (usize, usize, usize, usize, usize, String, Vec<String>) {
        let ants = self.ants.iter().map(|a| format!("{:?} {:?} {:?}", a.job, a.carry, std::mem::discriminant(&a.loc))).collect();
        (self.ants.len(), self.brood.len(), self.dead.len(), self.piles.len(), self.nodes.iter().filter(|n| n.done()).count(), format!("{:.6} {}", self.stock, self.refuse), ants)
    }

    /// Food dropped: where, as parts of the screen across and down, seen
    /// from above or not. From above it falls there; from the side, that
    /// far across, as far away as the nest is, more or less.
    pub fn feed(&mut self, x: f64, y: f64, top: bool) {
        self.count += 1;
        let z = if top { y * self.h } else { self.nest.1 + 24.0 * self.scale() * (self.r() - 0.5) };
        let pile = ((x * self.w).min(self.w - 2.0).max(1.0), z.min(self.h - 2.0).max(1.0), if top { 20.0 } else { 9.0 }, self.count);
        self.piles.push(pile);
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
        // Round the stones, if a few tries can find a way.
        let mut path = self.course(at, to);
        for _ in 0..8 {
            let hits = |p: &(f64, f64)| self.stones.iter().any(|s| ((p.0 - s.0) / (s.2 + BORE)).powi(2) + ((p.1 - s.1) / (s.3 + BORE)).powi(2) < 1.0);
            if !path.iter().any(hits) {
                break;
            }
            path = self.course(at, to);
        }
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
        // Army ants dig nothing: a hollow just under the ground, for the
        // night, is all their nest.
        if self.species == Species::Army {
            let to = (self.nest.0, (self.gy + 9.0).min(self.h - 7.0));
            let path = self.course((self.nest.0, self.gy), to);
            self.nodes.push(Tunnel::new(0, path, Room::Queen, 14.0, 4.5));
            return;
        }
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
        // Stones in the soil, where no tunnel's to go: the nest winds
        // between them.
        for _ in 0..40 {
            if self.stones.len() >= (self.pw * self.ph / 2600).clamp(2, 9) {
                break;
            }
            let (rx, ry) = (4.0 + 6.0 * self.r(), 2.5 + 2.5 * self.r());
            let at = (rx + (self.w - 2.0 * rx).max(1.0) * self.r(), self.gy + 6.0 + (self.h - self.gy - 10.0).max(1.0) * self.r());
            if !self.stony(at, rx, ry) && !self.stones.iter().any(|s| (s.0 - at.0).abs() < s.2 + rx + 3.0 && (s.1 - at.1).abs() < s.3 + ry + 3.0) {
                self.stones.push((at.0, at.1, rx, ry));
            }
        }
    }

    /// Whether a stone of this size, here, would be in the way of the
    /// nest as it's planned: on a tunnel, or in a chamber.
    fn stony(&self, at: (f64, f64), rx: f64, ry: f64) -> bool {
        self.nodes.iter().any(|n| {
            let near = |p: &(f64, f64), wide: f64, tall: f64| ((p.0 - at.0) / (rx + wide)).powi(2) + ((p.1 - at.1) / (ry + tall)).powi(2) < 1.0;
            n.path.iter().any(|p| near(p, BORE + 2.0, BORE + 2.0)) || (n.room != Room::None && near(&n.end(), n.rx + 2.0, n.ry + 2.0))
        })
    }

    /// A new ant at a place, `age` of the way through its life. The first
    /// few a queen raises are small; after them, as its kind has them.
    fn hatch(&mut self, loc: Loc, age: f64) -> Ant {
        let life = 300.0 + 400.0 * self.r();
        let (h, roll) = (habit(self.species), self.rng.below(100));
        let caste = if self.ants.len() < 8 || roll < h.minims {
            Caste::Minim
        } else if roll < h.minims + h.majors {
            Caste::Major
        } else {
            Caste::Media
        };
        self.count += 1;
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
            caste,
            side: if self.rng.below(2) == 0 { 1.0 } else { -1.0 },
            ph: TAU * self.r(),
            id: self.count,
            greet: 0.0,
            met: 0.0,
            rider: false,
            knows: None,
            trips: 0,
            born: self.clock - age * life,
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

    /// A twig falls, somewhere clear of the nest, lying any way round.
    fn twig(&mut self) {
        // Not on a plant, if a few tries can help it.
        let mut mid = self.somewhere(20.0);
        for _ in 0..8 {
            if !self.plants.iter().any(|p| (p.x - mid.0).powi(2) + (p.z - mid.1).powi(2) < 18.0 * 18.0) {
                break;
            }
            mid = self.somewhere(20.0);
        }
        let (ang, half) = (PI * self.r(), (6.0 + 9.0 * self.r()) * self.scale());
        self.twigs.push(Twig { a: (mid.0 - half * ang.cos(), mid.1 - half * ang.sin()), b: (mid.0 + half * ang.cos(), mid.1 + half * ang.sin()), left: 1.0 });
    }

    /// The spider a moment on. It makes for the nearest ant out on the
    /// ground and eats it, which takes it a while; three eaten, or its
    /// time up, it goes. The ants that fight, majors and fire ants, wear it
    /// down while they're on it, and when it's dead it's food.
    fn hunt(&mut self, dt: f64) {
        // It comes by day, and leaves a colony just begun alone.
        if self.abroad() && self.ants.len() >= 12 {
            self.prowl -= dt;
        }
        if self.hunter.is_none() && self.prowl <= 0.0 {
            // In from an edge, heading across.
            let (x, z) = match self.rng.below(4) {
                0 => (1.0, self.h * self.r()),
                1 => (self.w - 2.0, self.h * self.r()),
                2 => (self.w * self.r(), 1.0),
                _ => (self.w * self.r(), self.h - 2.0),
            };
            self.hunter = Some(Spider { x, z, dir: (self.h / 2.0 - z).atan2(self.w / 2.0 - x), hp: 9.0, fed: 0, bite: 0.0, stay: 45.0 });
        }
        let Some(mut s) = self.hunter.take() else { return };
        let fights = habit(self.species).majors > 0;
        let out = |a: &Ant| if let Loc::Surface(x, z) = a.loc { Some(((x - s.x).powi(2) + (z - s.z).powi(2)).sqrt()) } else { None };
        let on_it = self.ants.iter().filter(|a| fights && (a.caste == Caste::Major || self.species == Species::Fire) && out(a).is_some_and(|d| d < 3.5)).count();
        s.hp -= on_it as f64 * dt * 1.0;
        (s.bite, s.stay) = ((s.bite - dt).max(0.0), s.stay - dt);
        let gone = || 90.0 + 120.0;
        if s.hp <= 0.0 {
            // Dead: a prize to carry home.
            self.count += 1;
            self.piles.push((s.x.min(self.w - 2.0).max(1.0), s.z.min(self.h - 2.0).max(1.0), 14.0, self.count));
            self.prowl = gone();
            self.note(3, 0.0);
            return;
        }
        let leaving = s.stay <= 0.0 || s.fed >= 3;
        let prey = (0..self.ants.len()).filter_map(|k| Some((k, out(&self.ants[k])?))).min_by(|a, b| a.1.total_cmp(&b.1));
        if s.bite < 1.5 {
            let want = match (leaving, prey) {
                // Off the nearest side.
                (true, _) => (s.z - self.h / 2.0).atan2(s.x - self.w / 2.0),
                (false, Some((k, _))) => match self.ants[k].loc {
                    Loc::Surface(x, z) => (z - s.z).atan2(x - s.x),
                    _ => s.dir,
                },
                _ => s.dir + 0.5 * (s.x * 0.2 + s.z * 0.13).sin(),
            };
            s.dir += veer(s.dir, want).clamp(-3.0 * dt, 3.0 * dt);
            // It closes slowly, and pounces.
            let close = prey.is_some_and(|p| p.1 < 9.0);
            let v = if leaving { 10.0 } else if close { 17.0 } else { 8.0 } * self.scale() * dt;
            (s.x, s.z) = (s.x + s.dir.cos() * v, s.z + s.dir.sin() * v);
            if let Some((k, _)) = prey.filter(|p| !leaving && s.bite <= 0.0 && p.1 < 2.5) {
                self.ants.swap_remove(k);
                (s.fed, s.bite) = (s.fed + 1, 3.0);
            }
        }
        if s.x < -3.0 || s.z < -3.0 || s.x > self.w + 3.0 || s.z > self.h + 3.0 {
            self.prowl = gone();
            if s.fed > 0 {
                self.note(4, s.fed as f64);
            }
            return;
        }
        // Not leaving, it keeps to the ground.
        if !leaving {
            (s.x, s.z) = (s.x.min(self.w - 2.0).max(1.0), s.z.min(self.h - 2.0).max(1.0));
        }
        self.hunter = Some(s);
    }

    /// What's up top: the mound round the door, as high as `heap`; a few
    /// plants, a way off; a twig; and across the way, another colony's nest.
    fn landscape(&mut self, heap: f64) {
        for k in 0..self.pw {
            let d = (k as f64 - self.nest.0).abs();
            self.mound[k] = if d < 2.0 { 0.0 } else { (heap * (3.0 - (d - 6.0).abs() * 0.45)).max(0.0) };
        }
        for _ in 0..3 {
            let at = self.plant_site();
            self.grow(at, 1.0);
        }
        // A twig lying about already, part rotted.
        self.twig();
        self.twigs.iter_mut().for_each(|t| t.left = 0.6);
        // The other colony, in the corner furthest off.
        let corners = [(10.0, 10.0), (self.w - 10.0, 10.0), (10.0, self.h - 10.0), (self.w - 10.0, self.h - 10.0)];
        let far = |c: &(f64, f64)| (c.0 - self.nest.0).powi(2) + (c.1 - self.nest.1).powi(2);
        self.camp = *corners.iter().max_by(|a, b| far(a).total_cmp(&far(b))).unwrap();
    }

    /// Somewhere for a plant: a way off from the nest, and from the
    /// others, the best of a few places tried. Far across counts for more,
    /// that being all that shows from the side.
    fn plant_site(&mut self) -> (f64, f64) {
        let mut best = (f64::MIN, 0.0, 0.0);
        for _ in 0..16 {
            let (x, z) = (8.0 + (self.w - 16.0).max(0.0) * self.r(), self.h * (0.12 + 0.76 * self.r()));
            let from = |p: (f64, f64)| ((p.0 - x).powi(2) + (p.1 - z).powi(2)).sqrt();
            let apart = self.plants.iter().filter(|p| !p.gone()).map(|p| from((p.x, p.z)).min((p.x - x).abs() * 1.5)).fold(f64::MAX, f64::min);
            let score = from(self.nest).min((self.nest.0 - x).abs() * 1.6).min(apart);
            if score > best.0 {
                best = (score, x, z);
            }
        }
        (best.1, best.2)
    }

    /// A plant at a place, its leaves `leaf` grown: where one's died and
    /// gone, if one has, so those that knew the others still do.
    fn grow(&mut self, at: (f64, f64), leaf: f64) {
        let h = self.gy * (0.62 + 0.25 * self.r());
        let n = ((h / 5.0) as usize).clamp(2, 6);
        let leaves = (0..n).map(|k| (h * (0.35 + 0.65 * (k + 1) as f64 / n as f64), if k % 2 == 0 { 1.0 } else { -1.0 }, leaf)).collect();
        let lean = (self.r() - 0.5) * 0.5;
        let aphids = if self.milks() && leaf >= 1.0 { 6.0 } else { 0.0 };
        let plant = Plant { x: at.0, z: at.1, h, lean, leaves, aphids, bug: 0.0, health: 1.0, wither: 0.0, age: 0.0 };
        match self.plants.iter().position(|p| p.gone()) {
            Some(k) => self.plants[k] = plant,
            None => self.plants.push(plant),
        }
        self.made = None;
    }

    /// The plants a moment on. One whose leaves are kept cut short, or
    /// that's old, sickens, and in the end dies; it stands dead a while,
    /// then it's gone. Some die at the start of each winter. When few are
    /// left, a seedling comes up somewhere, in the warm half of the year.
    fn garden(&mut self, dt: f64, bare: bool, winter_came: bool) {
        for k in 0..self.plants.len() {
            let kill = winter_came && self.plants[k].alive() && self.r() < 0.2;
            let p = &mut self.plants[k];
            if !p.alive() {
                p.wither -= dt;
                p.leaves.iter_mut().for_each(|l| l.2 = (l.2 - dt / 20.0).max(0.0));
                (p.aphids, p.bug) = (0.0, 0.0);
                if p.wither <= 0.0 && p.wither > -dt {
                    self.made = None;
                }
                continue;
            }
            p.age += dt;
            let leaf = p.leaves.iter().map(|l| l.2).sum::<f64>() / p.leaves.len().max(1) as f64;
            // Cut hard, past its first days; or old.
            let sick = (!bare && p.age > 200.0 && leaf < 0.4) || p.age > DAY * YEAR * 2.5;
            p.health = if kill { 0.0 } else if sick { p.health - dt / 150.0 } else { (p.health + dt / 400.0).min(1.0) };
            if !p.alive() {
                p.wither = 90.0;
                self.made = None;
                self.note(13, 0.0);
            }
        }
        let living = self.plants.iter().filter(|p| p.alive()).count();
        if living < 2 && self.season() < 2 && self.r() < dt / 45.0 {
            let at = self.plant_site();
            self.grow(at, 0.05);
            self.note(14, 0.0);
        }
    }

    /// A colony well under way, as `grown` asks for: the shaft, the store
    /// and the queen's chamber dug already, the rest to do; workers about.
    fn settle(&mut self) {
        for n in &mut self.nodes {
            if matches!(n.room, Room::None | Room::Store | Room::Queen) {
                (n.dug, n.open) = (n.len, 1.0);
            }
        }
        self.carve();
        self.landscape(1.0);
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
            // Not the small ones a queen starts with: those are long dead.
            if self.ants.len() < 8 {
                let roll = self.rng.below(100);
                let h = habit(self.species);
                a.caste = if roll < h.minims { Caste::Minim } else if roll < h.minims + h.majors { Caste::Major } else { Caste::Media };
            }
            self.assign(&mut a);
            self.ants.push(a);
        }
        let q = self.room(Room::Queen).unwrap();
        for _ in 0..4 {
            let egg = (q, 5.0 * (self.r() - 0.5), 1.0 + self.r(), HATCH * self.r());
            self.brood.push(egg);
        }
    }

    /// A colony just begun: a queen alone at the bottom of the shaft she
    /// dug, with her first eggs, living off herself till they hatch. The
    /// first workers come small, soon, and one after another; everything
    /// else is theirs to do.
    fn found(&mut self) {
        for n in &mut self.nodes {
            if matches!(n.room, Room::None | Room::Queen) {
                (n.dug, n.open) = (n.len, 1.0);
            }
        }
        self.carve();
        self.landscape(0.35);
        let q = self.room(Room::Queen).unwrap();
        for k in 0..6 {
            let egg = (q, 6.0 * (self.r() - 0.5), 1.0 + self.r(), HATCH - 12.0 - 9.0 * k as f64);
            self.brood.push(egg);
        }
        self.stock = 9.0;
    }

    /// The colony across the way. Its foragers come out by day, once this
    /// one's big enough to be worth robbing, take what food they find, and
    /// carry it home. Where one meets one of ours they fight, and one of
    /// the two dies: a major mostly wins, a minim mostly loses.
    fn feud(&mut self, dt: f64) {
        let abroad = self.abroad();
        self.muster -= dt;
        if abroad && self.ants.len() >= (self.cap / 2).max(15) && self.rivals.len() < (self.cap / 6).clamp(3, 14) && self.muster <= 0.0 {
            self.muster = 5.0;
            let dir = TAU * self.r();
            self.rivals.push(Rival { x: self.camp.0, z: self.camp.1, dir, has: false, grip: 0.0 });
        }
        let pace = habit(self.species).pace * 1.1 * self.scale();
        for mut r in std::mem::take(&mut self.rivals) {
            if r.grip > 0.0 {
                r.grip -= dt;
                self.rivals.push(r);
                continue;
            }
            let mut turn = 1.6 * (self.r() - 0.5);
            let home = (self.camp.0 - r.x).powi(2) + (self.camp.1 - r.z).powi(2);
            if r.has || !abroad {
                turn += 3.0 * veer(r.dir, (self.camp.1 - r.z).atan2(self.camp.0 - r.x));
                if home < 9.0 {
                    // Home: with what it took, or for the night.
                    r.has = false;
                    if !abroad {
                        continue;
                    }
                }
            } else if let Some((f, fx, fz)) = self.food(r.x, r.z, 16.0 * self.scale()) {
                turn += 3.0 * veer(r.dir, (fz - r.z).atan2(fx - r.x));
                if (fx - r.x).powi(2) + (fz - r.z).powi(2) < 12.0 {
                    match f {
                        Found::Pile(id) => self.piles.iter_mut().filter(|p| p.3 == id).for_each(|p| p.2 -= 1.0),
                        Found::Plant(p) if habit(self.species).leaf => {
                            if let Some(l) = self.plants[p].leaves.iter_mut().find(|l| l.2 > 0.3) {
                                l.2 -= 0.2;
                            }
                        }
                        Found::Plant(p) => self.plants[p].aphids = (self.plants[p].aphids - 1.0).max(0.0),
                    }
                    r.has = true;
                }
            }
            r.dir += turn * dt;
            let (nx, nz) = (r.x + r.dir.cos() * pace * dt, r.z + r.dir.sin() * pace * dt);
            if self.blocked(nx, nz) && !self.blocked(r.x, r.z) {
                r.dir += (0.9 + self.r()) * if self.rng.below(2) == 0 { 1.0 } else { -1.0 };
            } else {
                (r.x, r.z) = (nx.min(self.w - 0.01).max(0.0), nz.min(self.h - 0.01).max(0.0));
            }
            // One of ours within reach, and not in a fight already: they
            // close, and it's settled.
            // One that's fought lately keeps clear a while; and half the
            // time they only square up, and part.
            let met = (0..self.ants.len()).find(|&k| self.ants[k].met <= 0.0 && matches!(self.ants[k].loc, Loc::Surface(x, z) if (x - r.x).powi(2) + (z - r.z).powi(2) < 6.0));
            if let Some(k) = met {
                (self.ants[k].greet, self.ants[k].met, r.grip) = (1.5, 30.0, 1.5);
                if self.rng.below(2) == 0 {
                    self.rivals.push(r);
                    continue;
                }
                let odds = 50
                    + match self.ants[k].caste {
                        Caste::Major => 30,
                        Caste::Minim => -20,
                        _ => 0,
                    }
                    + if self.species == Species::Fire { 15 } else { 0 };
                if self.rng.below(100) < odds {
                    // Theirs dead: something to carry home.
                    self.count += 1;
                    self.piles.push((r.x.min(self.w - 2.0).max(1.0), r.z.min(self.h - 2.0).max(1.0), 1.0, self.count));
                    continue;
                }
                self.ants[k].age = self.ants[k].life;
            }
            self.rivals.push(r);
        }
    }

    /// The winged. Once a year, in summer, a colony that's grown and fed
    /// raises some, one after another; they wait in the nest till they're
    /// all there and the day's bright, then come out and fly.
    fn swarm(&mut self, dt: f64) {
        let year = (self.clock / DAY / YEAR).floor();
        if self.season() == 1 && year > self.flew && self.ants.len() * 10 >= self.cap * 6 && self.stock >= 6.0 {
            (self.flew, self.raise) = (year, (self.cap / 6).clamp(5, 14));
        }
        self.wing -= dt;
        if self.raise > 0 && self.wing <= 0.0 {
            (self.wing, self.raise) = (4.0, self.raise - 1);
            let q = self.room(Room::Queen).unwrap();
            let at = Loc::Room(q, 5.0 * (self.r() - 0.5), self.r());
            let mut a = self.hatch(at, 0.2);
            (a.caste, a.job, a.rest, a.goal) = (Caste::Alate, Job::Fly, 6.0, Goal::Spot(q, self.nodes[q].len));
            self.ants.push(a);
        }
    }

    /// Army ants on the move. Toward evening, once a day, the whole colony
    /// leaves its hollow and goes in a column to another place; when most
    /// of them are there, or it's taken long enough, that's the nest.
    fn march(&mut self, dt: f64) {
        if self.species != Species::Army {
            return;
        }
        let day = (self.clock / DAY).floor();
        if self.trek.is_none() && day > self.moved && self.hour() >= 17.5 && self.hour() < 19.0 && self.rain <= 0.0 && self.ants.len() >= 6 {
            let to = self.somewhere(45.0 * self.scale());
            self.trek = Some((to.0, to.1, 0.0));
        }
        let Some((x, z, since)) = self.trek else { return };
        self.trek = Some((x, z, since + dt));
        let there = self.ants.iter().filter(|a| matches!(a.loc, Loc::Surface(ax, az) if (ax - x).powi(2) + (az - z).powi(2) < 64.0)).count();
        if there * 5 < self.ants.len() * 4 && since < 70.0 {
            return;
        }
        // There: the hollow's here now, everyone in it or by it.
        let dx = x - self.nest.0;
        for n in &mut self.nodes {
            let through = n.dug >= n.len;
            n.path.iter_mut().for_each(|p| p.0 += dx);
            n.measure();
            if through {
                n.dug = n.len;
            }
        }
        self.nest = (x, z);
        self.mound.fill(0.0);
        self.carve();
        for a in &mut self.ants {
            if !matches!(a.loc, Loc::Surface(..) | Loc::Air(..)) {
                a.loc = Loc::Surface(x, z);
            }
            a.goal = Goal::Seek;
        }
        (self.trek, self.moved) = (None, day);
        self.scent.fill(0.0);
        self.note(12, 0.0);
    }

    /// Whether it's the day to fly: they're all raised, and it's bright
    /// and dry.
    fn launch(&self) -> bool {
        self.raise == 0 && self.abroad() && self.light() > 0.8
    }

    /// Where food's kept: the store, or till that's dug, with the queen.
    fn larder(&self) -> usize {
        self.room(Room::Store).or(self.room(Room::Queen)).unwrap()
    }

    /// Where the brood's kept just now: up in the nursery by day, where
    /// it's warm, and down with the queen by night and in winter.
    fn cradle(&self) -> usize {
        match self.room(Room::Nursery) {
            Some(n) if self.light() > 0.4 && self.season() != 3 => n,
            _ => self.room(Room::Queen).unwrap(),
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
            Loc::Air(x, _, up) => (x, self.ground(x) - 1.0 - up),
        }
    }

    /// Whether an ant on the ground can't be at a place: it's off the
    /// screen, there's a twig lying there, or, seen from above, it's under
    /// text. Round the nest's hole it always can, and round food.
    fn blocked(&self, x: f64, z: f64) -> bool {
        if x < 1.0 || z < 1.0 || x >= self.w - 1.0 || z >= self.h - 1.0 {
            return true;
        }
        let near = (x - self.nest.0).powi(2) + (z - self.nest.1).powi(2) < 100.0;
        if !near && self.twigs.iter().any(|t| t.left > 0.12 && off((x, z), t.a, t.b) < 1.6) {
            return true;
        }
        if !self.top || near || !self.text.get(z as usize / 4 * (self.pw / 2) + x as usize / 2).is_some_and(|t| *t) {
            return false;
        }
        // Under text; but food that's under text can still be got to, from
        // as far off as it can be seen.
        let far = (16.0 * self.scale()).powi(2);
        let by = |fx: f64, fz: f64| (fx - x).powi(2) + (fz - z).powi(2) < far;
        !(self.piles.iter().any(|p| by(p.0, p.1)) || self.plants.iter().any(|p| p.alive() && by(p.x, p.z)))
    }

    fn cell(&self, x: f64, z: f64) -> Option<usize> {
        (x >= 0.0 && z >= 0.0 && x < self.w && z < self.h).then(|| (z as usize / 2).min(self.ph - 1) * self.pw + x as usize)
    }

    /// The food within `reach` of a place, the nearest: a plant with leaf
    /// on it, for those that cut leaves, or aphids, for those that milk
    /// them, or a pile.
    fn food(&self, x: f64, z: f64, reach: f64) -> Option<(Found, f64, f64)> {
        let leaf = habit(self.species).leaf;
        // Garden ants go to a plant for its aphids.
        let milk = self.milks();
        let plants = self.plants.iter().enumerate().filter(|(_, p)| p.alive() && ((leaf && p.leaves.iter().any(|l| l.2 > 0.3)) || (milk && p.aphids >= 1.0))).map(|(k, p)| (Found::Plant(k), p.x, p.z));
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
            // After its leader, where that's out; by the door till it is.
            Goal::Follow(id) => {
                let led = self.ants.iter().find(|o| o.id == id).and_then(|o| if let Loc::Surface(lx, lz) = o.loc { Some((lx, lz)) } else { None });
                (Some(led.unwrap_or(nest)), 0.0)
            }
            _ => (Some(nest), 2.5),
        };
        let mut turn = 1.6 * (self.r() - 0.5) + 0.6 * (a.ph + x * 0.3 + z * 0.2).sin();
        a.dodge = (a.dodge - dt).max(0.0);
        match to {
            // It knows which way it's going, as ants do, counting steps.
            Some(p) => turn += if a.dodge > 0.0 { 0.4 } else { 4.0 } * veer(a.heading, (p.1 - z).atan2(p.0 - x)),
            None => match self.food(x, z, 16.0 * self.scale()) {
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
        // A spider about: those that fight go for it from a long way off,
        // the alarm reaching them, and from anywhere once it's killed one;
        // the rest run when it's near.
        let fights = a.caste == Caste::Major || self.species == Species::Fire;
        // Once it's killed, the alarm's everywhere.
        let alarm = match (fights, self.hunter.as_ref().is_some_and(|s| s.fed > 0)) {
            (true, true) => f64::MAX.sqrt(),
            (true, false) => 50.0 * self.scale(),
            _ => 12.0,
        };
        if let Some(s) = self.hunter.as_ref().filter(|s| (s.x - x).powi(2) + (s.z - z).powi(2) < alarm * alarm) {
            let at = (s.z - z).atan2(s.x - x);
            turn = 5.0 * veer(a.heading, if fights { at } else { at + PI }) + 2.0 * (self.r() - 0.5);
        }
        a.heading += turn * dt;
        let v = h.pace * self.scale() * 1.2 * if a.carry == Carry::None { 1.0 } else { 0.8 } * dt;
        let (nx, nz) = (x + a.heading.cos() * v, z + a.heading.sin() * v);
        // Something in the way: it turns along it a while. In among text
        // already, it walks out.
        let (mut x, mut z) = (x, z);
        if self.blocked(nx, nz) && !self.blocked(x, z) {
            a.heading += (0.9 + self.r()) * if self.rng.below(2) == 0 { 1.0 } else { -1.0 };
            a.dodge = 0.7;
        } else {
            (x, z) = (nx.min(self.w - 0.01).max(0.0), nz.min(self.h - 0.01).max(0.0));
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
        let v = h.pace * self.scale() * dt * if a.carry == Carry::None { 1.0 } else { 0.8 } * if a.age < 30.0 { 0.6 } else { 1.0 };
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
                                // The door's stopped up: it waits by it.
                                _ if p == 0 && under.is_none() && self.plug > 0.5 => Loc::Edge(e, ns),
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
            Loc::Air(..) => {}
        }
        false
    }

    /// What it does next, by what it is and how old: the small and the
    /// young stay in, with the brood and the queen; the middle-aged dig and
    /// clear the dead; the old go out for food; the big keep watch. But
    /// when too few are digging or foraging, whoever's free does that.
    fn assign(&mut self, a: &mut Ant) {
        (a.stage, a.wait, a.carry, a.found, a.rider) = (0, 0.0, Carry::None, None, false);
        let pop = self.ants.len().max(1);
        let doing = |j: Job| self.ants.iter().filter(|o| o.job == j).count();
        let (young, old) = (a.age < a.life * 0.3, a.age > a.life * 0.6);
        let mut wants = match a.caste {
            Caste::Alate => vec![Job::Fly],
            Caste::Major => vec![Job::Guard, Job::Forage, Job::Rest],
            Caste::Minim => vec![Job::Nurse, Job::Rest],
            _ if young => vec![Job::Nurse, Job::Rest],
            _ if old => vec![Job::Forage, Job::Bury, Job::Dig, Job::Rest],
            _ => vec![Job::Dig, Job::Bury, Job::Nurse, Job::Forage, Job::Rest],
        };
        if a.caste != Caste::Alate {
            if doing(Job::Forage) * 4 < pop {
                wants.insert(0, Job::Forage);
            }
            if doing(Job::Dig) * 4 < pop {
                wants.insert(0, Job::Dig);
            }
            // The dead aren't left lying.
            if a.caste != Caste::Minim && !self.dead.is_empty() && doing(Job::Bury) * 6 < pop {
                wants.insert(0, Job::Bury);
            }
        }
        for j in wants {
            if self.take(a, j) {
                return;
            }
        }
    }

    /// A job taken up, if there's any of it to do: where to go for it.
    fn take(&mut self, a: &mut Ant, job: Job) -> bool {
        let pop = self.ants.len().max(1);
        let (digging, nursing) = (self.ants.iter().filter(|o| o.job == Job::Dig).count(), self.ants.iter().filter(|o| o.job == Job::Nurse).count());
        let queen = self.room(Room::Queen).unwrap();
        let near = |c: &mut Colony, lo: f64, hi: f64| {
            let (ang, out) = (TAU * c.r(), lo + (hi - lo) * c.r());
            Goal::Point((c.nest.0 + out * ang.cos()).min(c.w - 2.0).max(1.0), (c.nest.1 + out * ang.sin()).min(c.h - 2.0).max(1.0))
        };
        let goal = match job {
            // No digging in winter, or carrying grains out in the rain.
            Job::Dig => match self.site() {
                Some(e) if digging * 3 < pop + 3 && self.season() != 3 && self.rain <= 0.0 => {
                    a.aim = e;
                    Goal::Tip(e)
                }
                _ => return false,
            },
            // One nobody alive is on the way to: whoever was may have been
            // eaten, or called off.
            Job::Bury => match self.dead.iter().position(|d| !self.ants.iter().any(|o| o.id != a.id && o.job == Job::Bury && o.stage == 0 && o.aim == d.1)) {
                Some(k) => {
                    a.aim = self.dead[k].1;
                    match self.dead[k].0 {
                        Loc::Surface(x, z) | Loc::Air(x, z, _) => Goal::Point(x, z),
                        Loc::Edge(e, s) => Goal::Spot(e, s),
                        Loc::Room(n, ..) => Goal::Spot(n, self.nodes[n].len),
                    }
                }
                None => return false,
            },
            // Brood that isn't where brood's kept just now: to carry there.
            Job::Nurse => {
                let to = self.cradle();
                match self.brood.iter().find(|b| b.0 != to) {
                    Some(b) if nursing * 5 < pop + 5 => {
                        a.aim = b.0;
                        Goal::Spot(b.0, self.nodes[b.0].len)
                    }
                    _ => return false,
                }
            }
            // Back to where it found plenty, if it did; or out looking.
            Job::Forage => {
                if !self.abroad() || self.rng.below(100) >= 85 {
                    return false;
                }
                a.knows.map_or(Goal::Seek, |k| Goal::Point(k.0, k.1))
            }
            Job::Guard => {
                if !self.abroad() {
                    return false;
                }
                near(self, 5.0, 18.0 * self.scale())
            }
            // The day come, out by the door; till then, in with the queen.
            Job::Fly => match self.launch() {
                true => {
                    a.stage = 1;
                    near(self, 3.0, 9.0)
                }
                false => Goal::Spot(queen, self.nodes[queen].len),
            },
            Job::Rest => {
                let rooms: Vec<usize> = (1..self.nodes.len()).filter(|&i| self.nodes[i].room != Room::None && self.nodes[i].done()).collect();
                // In winter, all together round the queen; a minim, by the
                // garden; the young, by the brood.
                let n = if self.season() == 3 {
                    queen
                } else if a.caste == Caste::Minim && habit(self.species).leaf {
                    self.larder()
                } else if a.age < a.life * 0.3 {
                    self.cradle()
                } else {
                    rooms[self.rng.below(rooms.len() as i32) as usize]
                };
                Goal::Spot(n, self.nodes[n].len)
            }
        };
        (a.job, a.goal) = (job, goal);
        true
    }

    /// One that's resting, called out to follow a forager back to food it
    /// found: a few at a time, no more.
    fn recruit(&mut self, leader: usize) {
        if self.ants.iter().filter(|o| matches!(o.goal, Goal::Follow(_))).count() >= 3 {
            return;
        }
        let free = |o: &Ant| o.job == Job::Rest && o.caste == Caste::Media && matches!(o.loc, Loc::Room(..));
        if let Some(k) = self.ants.iter().position(free)
            && let Loc::Room(n, ..) = self.ants[k].loc
        {
            let o = &mut self.ants[k];
            (o.loc, o.job, o.goal, o.stage, o.rest) = (Loc::Edge(n, self.nodes[n].len), Job::Forage, Goal::Follow(leader), 0, 0.0);
        }
    }

    /// At its goal: what it came to do.
    fn arrive(&mut self, a: &mut Ant) {
        let larder = self.larder();
        let home = Goal::Spot(larder, self.nodes[larder].len);
        let into = |a: &mut Ant, c: &mut Colony, long: f64| {
            if let Goal::Spot(n, _) = a.goal {
                (a.loc, a.rest, a.to) = (Loc::Room(n, 0.0, 0.0), long * (0.3 + 0.7 * c.r()), (0.0, 0.0));
            }
        };
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
                Some(Found::Plant(p)) => {
                    // A leaf with enough on it, cut, or its aphids milked:
                    // either takes a moment.
                    let leaves = &self.plants[p].leaves;
                    let at = match habit(self.species).leaf {
                        true => (0..leaves.len()).find(|&l| leaves[l].2 > 0.3),
                        false => (self.plants[p].aphids >= 1.0 && !leaves.is_empty()).then(|| a.id % leaves.len()),
                    };
                    // An ant at the plant: the ladybird's off.
                    self.plants[p].bug = 0.0;
                    match at {
                        Some(l) => (a.wait, a.stage, a.aim) = (1.0 + self.r(), 1, l),
                        None => a.found = None,
                    }
                }
                Some(Found::Pile(id)) => match self.piles.iter_mut().find(|p| p.3 == id && p.2 > 0.0) {
                    Some(p) => {
                        p.2 -= 1.0;
                        // Plenty left: somewhere to come back to.
                        a.knows = (p.2 > 3.0).then_some((p.0, p.1));
                        (a.carry, a.goal, a.stage, a.scent) = (Carry::Food, home, 2, 1.0);
                    }
                    None => a.found = None,
                },
                // Where it was going back to, or was led: now it looks.
                None => (a.goal, a.knows) = (Goal::Seek, None),
            },
            (Job::Forage, _) => {
                match a.carry {
                    Carry::Leaf => self.leaf += 1.0,
                    _ => self.stock += 1.0,
                }
                a.trips += 1;
                let knows = a.knows;
                self.assign(a);
                // Straight back for more, with another in tow.
                if a.job == Job::Forage && knows.is_some() {
                    self.recruit(a.id);
                }
            }
            (Job::Nurse, 0) => {
                let to = self.cradle();
                match self.brood.iter().position(|b| b.0 == a.aim && b.0 != to) {
                    Some(k) => {
                        let egg = self.brood.remove(k);
                        (a.carry, a.goal, a.stage) = (Carry::Egg(egg.3), Goal::Spot(to, self.nodes[to].len), 2);
                    }
                    None => self.assign(a),
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
                            Goal::Point((self.nest.0 + 26.0 * self.scale() * ang.cos()).min(self.w - 3.0).max(3.0), (self.nest.1 + 26.0 * self.scale() * ang.sin()).min(self.h - 3.0).max(3.0))
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
            // On watch a while, then somewhere else.
            (Job::Guard, _) => (a.wait, a.stage) = (2.0 + 3.0 * self.r(), 1),
            (Job::Fly, 0) => into(a, self, 5.0),
            (Job::Fly, _) => {
                if let Loc::Surface(x, z) = a.loc {
                    a.loc = Loc::Air(x, z, 0.0);
                }
            }
            (Job::Rest, _) => into(a, self, 10.0),
        }
    }

    /// Done with what it was busy at: a grain dug out, a piece of leaf
    /// cut, an aphid milked, a watch kept.
    fn finish(&mut self, a: &mut Ant) {
        match a.job {
            Job::Dig => {
                let bite = 2.0 * self.scale();
                let n = &mut self.nodes[a.aim];
                if n.dug < n.len {
                    n.dug = (n.dug + bite).min(n.len);
                } else {
                    n.open = (n.open + 1.2 / (n.rx * n.ry)).min(1.0);
                }
                if n.done() && n.room != Room::None {
                    let which = ROOMS.iter().position(|r| *r == n.room).unwrap();
                    self.note(2, which as f64);
                }
                self.carve();
                // Out, and a little way from the hole, any way round it.
                let (ang, out) = (TAU * self.r(), 4.0 + 7.0 * self.r());
                let to = Goal::Point((self.nest.0 + out * ang.cos()).min(self.w - 2.0).max(1.0), (self.nest.1 + out * ang.sin()).min(self.h - 2.0).max(1.0));
                (a.carry, a.goal, a.stage) = (Carry::Grain, to, 2);
            }
            Job::Forage => {
                let larder = self.larder();
                a.carry = Carry::Food;
                if let Some(Found::Plant(p)) = a.found {
                    match habit(self.species).leaf {
                        true => {
                            let leaf = &mut self.plants[p].leaves[a.aim].2;
                            *leaf = (*leaf - 0.2).max(0.0);
                            // A minim rides some of them home, on the leaf.
                            (a.carry, a.rider) = (Carry::Leaf, a.caste != Caste::Minim && self.rng.below(100) < 40);
                        }
                        false => self.plants[p].aphids = (self.plants[p].aphids - 1.0).max(0.0),
                    }
                }
                (a.goal, a.stage, a.scent) = (Goal::Spot(larder, self.nodes[larder].len), 2, 1.0);
            }
            _ => self.assign(a),
        }
    }

    fn live(&mut self, a: &mut Ant, dt: f64) {
        // In winter they're all but asleep, and hardly age: a colony comes
        // through it.
        a.age += dt * if self.season() == 3 { 0.1 } else { 1.0 };
        // Army ants moving: everyone goes where the colony's going.
        if let Some((x, z, _)) = self.trek
            && !matches!(a.goal, Goal::Point(gx, gz) if (gx, gz) == (x, z))
        {
            (a.job, a.goal, a.stage, a.wait) = (Job::Guard, Goal::Point(x, z), 0, 0.0);
            if let Loc::Room(n, ..) = a.loc {
                a.loc = Loc::Edge(n, self.nodes[n].len);
            }
        }
        // Night, winter or rain coming on: those out looking come home.
        if a.goal == Goal::Seek && !self.abroad() {
            let n = self.room(Room::Queen).unwrap();
            (a.job, a.goal, a.stage) = (Job::Rest, Goal::Spot(n, self.nodes[n].len), 0);
        }
        // On the wing: up and away.
        if let Loc::Air(x, z, up) = a.loc {
            a.loc = Loc::Air(x + 5.0 * (a.ph + up * 0.2).sin() * dt, z, up + 13.0 * self.scale() * dt);
            return;
        }
        // Head to head with another, or in a fight: it stays where it is.
        a.met = (a.met - dt).max(0.0);
        if a.greet > 0.0 {
            a.greet -= dt;
            return;
        }
        // Its leader there, or gone: it looks for itself.
        if let Goal::Follow(id) = a.goal
            && !self.ants.iter().any(|o| o.id == id && matches!(o.goal, Goal::Point(..)))
        {
            a.goal = Goal::Seek;
        }
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
        self.clock += dt;
        let winter = self.season() == 3;
        // Seen from above, food that's under the slide's text is moved out
        // from under it, to the nearest clear ground: nothing could see it
        // there.
        if self.top {
            let under = |c: &Colony, x: f64, z: f64| c.text.get(z as usize / 4 * (c.pw / 2) + x as usize / 2).is_some_and(|t| *t);
            for k in 0..self.piles.len() {
                let (x, z) = (self.piles[k].0, self.piles[k].1);
                if !under(self, x, z) {
                    continue;
                }
                let clear = (1..60).flat_map(|ring| (0..16).map(move |i| (ring as f64 * 4.0, i as f64 / 16.0 * TAU))).map(|(out, ang)| (x + out * ang.cos(), z + out * ang.sin())).find(|&(nx, nz)| nx >= 2.0 && nz >= 2.0 && nx < self.w - 2.0 && nz < self.h - 2.0 && !under(self, nx, nz));
                if let Some((nx, nz)) = clear {
                    (self.piles[k].0, self.piles[k].1) = (nx, nz);
                }
            }
        }
        // A click on an ant follows it; anywhere else, drops food.
        for &(x, y) in clicks {
            if !self.pick(x, y) {
                self.feed(x / self.w, y * 2.0 / self.h, self.top);
            }
        }
        // Those that cut no leaves find what's fallen: there's always a
        // pile or two somewhere.
        self.piles.retain(|p| p.2 > 0.0);
        if !habit(self.species).leaf && self.piles.len() < 2 {
            self.count += 1;
            let (at, much) = (self.somewhere(40.0), 20.0 + 25.0 * self.r());
            self.piles.push((at.0, at.1, much, self.count));
        }
        // Leaves grow back, but for the end of autumn, when they fall, and
        // winter, when there are none.
        let bare = winter || (self.clock / DAY) % YEAR > 14.0;
        for l in self.plants.iter_mut().filter(|p| p.alive()).flat_map(|p| &mut p.leaves) {
            l.2 = if bare { (l.2 - dt / 40.0).max(0.0) } else { (l.2 + dt / 90.0).min(1.0) };
        }
        // Rain: it comes now and then, but not in winter. While it falls
        // the door's stopped up, the trails wash away, twigs rot the
        // faster and leaves grow; after, the door's opened again.
        if self.rain > 0.0 {
            self.rain -= dt;
            self.plug = (self.plug + dt / 6.0).min(1.0);
            self.twigs.iter_mut().for_each(|t| t.left -= dt / 100.0);
            self.plants.iter_mut().filter(|p| p.alive()).flat_map(|p| &mut p.leaves).filter(|_| !bare).for_each(|l| l.2 = (l.2 + dt / 45.0).min(1.0));
            if self.rain <= 0.0 {
                self.cloud = 480.0 + 480.0 * self.r();
            }
        } else {
            self.plug = (self.plug - dt / 12.0).max(0.0);
            if !winter {
                self.cloud -= dt;
            }
            if self.cloud <= 0.0 {
                self.rain = 50.0 + 40.0 * self.r();
            }
        }
        // Aphids breed on a plant in leaf.
        // A ladybird comes for them now and then, and stays till it's
        // driven off, or has had enough.
        if self.milks() {
            for k in 0..self.plants.len() {
                if !self.plants[k].alive() {
                    continue;
                }
                let come = self.plants[k].bug <= 0.0 && self.plants[k].aphids >= 3.0 && self.r() < dt / 150.0;
                let p = &mut self.plants[k];
                p.bug = if come { 40.0 } else { (p.bug - dt).max(0.0) };
                p.aphids = if bare { 0.0 } else { (p.aphids + if p.bug > 0.0 { -dt / 2.5 } else { dt / 9.0 }).clamp(0.0, 6.0) };
            }
        }
        // Tasks' piles never run out; a seed from a full midden sprouts,
        // in the warm half of the year, near the door.
        for p in self.piles.iter_mut().filter(|p| self.tasks.contains(&p.3)) {
            p.2 = p.2.max(6.0);
        }
        if self.refuse >= 12 && self.plants.iter().filter(|p| !p.gone()).count() < 5 && self.season() < 2 && self.r() < dt / 60.0 {
            self.refuse -= 10;
            let (ang, out) = (TAU * self.r(), (20.0 + 15.0 * self.r()) * self.scale());
            let at = ((self.nest.0 + out * ang.cos()).min(self.w - 6.0).max(6.0), (self.nest.1 + out * ang.sin()).min(self.h - 6.0).max(6.0));
            self.grow(at, 0.05);
            self.note(11, 0.0);
        }
        // How many there are, four times a day; the seasons turning; the
        // queen's years.
        if self.clock - self.sampled >= DAY / 4.0 {
            self.sampled = self.clock;
            self.pops.push(self.ants.len() as f64);
            if self.pops.len() > 64 {
                self.pops.remove(0);
            }
        }
        let winter_came = self.season() == 3 && self.season_was != 3;
        self.garden(dt, bare, winter_came);
        if self.season() != self.season_was {
            self.season_was = self.season();
            match self.season_was {
                3 => self.note(6, self.ants.len() as f64),
                0 => self.note(7, self.ants.len() as f64),
                _ => {}
            }
        }
        self.reign += dt;
        if self.queen && self.reign > self.span {
            self.queen = false;
            self.note(8, 0.0);
        }
        // Nobody left, and nothing to hatch: one of those that flew comes
        // down here, and it begins again.
        if self.ants.is_empty() && self.brood.is_empty() && (!self.queen || self.stock < 1.0) {
            if !self.queen {
                self.note(9, 0.0);
            }
            (self.queen, self.reign, self.stock) = (true, 0.0, 9.0);
            let q = self.room(Room::Queen).unwrap();
            for k in 0..6 {
                let egg = (q, 6.0 * (self.r() - 0.5), 1.0 + self.r(), HATCH - 12.0 - 9.0 * k as f64);
                self.brood.push(egg);
            }
        }
        if self.follow.is_some_and(|id| !self.ants.iter().any(|a| a.id == id)) {
            self.follow = None;
        }
        self.march(dt);
        // Leaf brought in is worked into the garden, which is what they
        // eat; and a garden not fed dwindles.
        if habit(self.species).leaf {
            let worked = self.leaf.min(0.25 * dt);
            (self.leaf, self.stock) = (self.leaf - worked, self.stock + worked - self.stock * 0.0015 * dt);
        }
        // Those underground that meet stop head to head a moment, one
        // feeding the other.
        self.social -= dt;
        if self.social <= 0.0 {
            self.social = 1.0;
            let free: Vec<usize> = (0..self.ants.len())
                .filter(|&k| {
                    let a = &self.ants[k];
                    matches!(a.loc, Loc::Edge(..) | Loc::Room(..)) && a.wait <= 0.0 && a.greet <= 0.0 && a.met <= 0.0 && a.carry == Carry::None
                })
                .collect();
            for (n, &i) in free.iter().enumerate() {
                let at = self.ants[i].at;
                let other = free[n + 1..].iter().find(|&&j| self.ants[j].greet <= 0.0 && (self.ants[j].at.0 - at.0).powi(2) + (self.ants[j].at.1 - at.1).powi(2) < 6.0);
                if let Some(&j) = other.filter(|_| self.ants[i].greet <= 0.0) {
                    let (long, to) = (1.0 + 0.6 * self.r(), self.ants[j].at);
                    let d = ((to.0 - at.0).powi(2) + (to.1 - at.1).powi(2)).sqrt().max(0.1);
                    (self.ants[i].greet, self.ants[i].met, self.ants[i].dir) = (long, 25.0, ((to.0 - at.0) / d, (to.1 - at.1) / d));
                    (self.ants[j].greet, self.ants[j].met, self.ants[j].dir) = (long, 25.0, ((at.0 - to.0) / d, (at.1 - to.1) / d));
                }
            }
        }
        // A trail to food that's gone is soon forgotten, and washed away
        // by rain. What shows of it is looked at again only now and then:
        // less to draw.
        let fade = (-dt / if self.rain > 0.0 { 3.0 } else { 22.0 }).exp() as f32;
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
        // In winter they're still, and eat little, and she doesn't lay.
        self.stock = (self.stock - self.ants.len() as f64 * if winter { 0.001 } else { 0.003 } * dt).max(0.0);
        self.stock = self.stock.min(self.cap as f64 * 2.0 + 20.0);
        if !winter {
            self.lay -= dt;
        }
        let queen = self.room(Room::Queen).unwrap();
        // An egg on its way to the nursery is still one to come.
        let carried = self.ants.iter().filter(|a| matches!(a.carry, Carry::Egg(_))).count();
        if self.queen && self.lay <= 0.0 && self.stock >= 1.0 && self.ants.len() + self.brood.len() + carried < self.cap {
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
            if self.ants.is_empty() && !self.events.iter().rev().take_while(|e| e.1 != 0 && e.1 != 9).any(|e| e.1 == 1) {
                self.note(1, 0.0);
            }
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
        // The winged, high enough, are gone for good.
        for a in std::mem::take(&mut self.ants) {
            if let Loc::Air(_, _, up) = a.loc {
                match up > self.gy + 4.0 {
                    true => (self.flown, self.flock) = (self.flown + 1, self.flock + 1),
                    false => self.ants.push(a),
                }
                continue;
            }
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
        // A twig falls now and then, and lies there rotting, in the way.
        self.fall -= dt;
        if self.fall <= 0.0 && self.twigs.len() < 3 {
            self.fall = 50.0 + 70.0 * self.r();
            self.twig();
        }
        self.twigs.iter_mut().for_each(|t| t.left -= dt / 200.0);
        self.twigs.retain(|t| t.left > 0.0);
        // The last of a flight gone: that's the flight.
        if self.flock > 0 && self.raise == 0 && !self.ants.iter().any(|a| a.caste == Caste::Alate) {
            self.note(5, self.flock as f64);
            self.flock = 0;
        }
        self.hunt(dt);
        self.feud(dt);
        self.swarm(dt);
        // All dug, and the colony grown into it: another chamber, off one
        // of the bends.
        if self.site().is_none() && self.nodes.len() < 14 && self.ants.len() * 5 >= self.cap * 3 && self.species != Species::Army {
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
            // Drawn again now and then, as it does.
            if (self.clock / 5.0).floor() != ((self.clock - dt) / 5.0).floor() {
                self.made = None;
            }
        }
        let door = (self.nest.0 as usize).min(self.pw - 1);
        for k in door.saturating_sub(1)..=(door + 1).min(self.pw - 1) {
            self.mound[k] = 0.0;
        }
    }

    /// What's behind the text: from the side, the soil and what's dug in
    /// it; from above, the ground and the scent on it.
    pub fn back(&mut self, field: &mut [Rgb], th: &Theme) {
        let mood = self.mood();
        if self.made != Some((th.bg, self.top, mood)) || self.cache.len() != field.len() {
            self.made = Some((th.bg, self.top, mood));
            let (a, b, much) = self.tones;
            // Darker by night, pale under snow, dark again when it's wet.
            let (dark, snow, wet) = (0.45 * (1.0 - (mood % 8) as f64 / 6.0), mood & 8 != 0, mood & 16 != 0);
            let night = Rgb(4, 6, 18);
            let sky = th.bg.mix(night, dark);
            let (void, mut earth) = (th.bg.mix(a, 0.1), th.bg.mix(a, much * 0.3));
            if snow {
                earth = earth.mix(BROOD, 0.3);
            }
            earth = earth.mix(night, dark * 0.7).mix(Rgb(0, 0, 0), if wet { 0.2 } else { 0.0 });
            self.cache = (0..self.pw * self.ph)
                .map(|i| {
                    if self.top {
                        // The trail to food shows, faintly.
                        return earth.mix(th.accent, 0.14 * self.shown[i] as f64 / 3.0);
                    }
                    let (x, y) = (i % self.pw, i / self.pw);
                    let deep = y as f64 * 2.0 + 1.0 - self.ground(x as f64);
                    if deep < 0.0 {
                        return sky;
                    }
                    // Snow lying on the ground.
                    if snow && deep < 2.5 {
                        return th.bg.mix(BROOD, 0.75).mix(night, dark * 0.5);
                    }
                    // What's dug, and the way in, through the mound.
                    if self.hole[i] || (x as f64 + 0.5 - self.nest.0).abs() < 1.5 && (y as f64 * 2.0 + 1.0) < self.gy {
                        // In the rain the top of the shaft has water in it.
                        return if wet && deep < 9.0 { void.mix(RAIN, 0.4) } else { void };
                    }
                    // A stone; or a root, going down from a plant.
                    let at = (x as f64 + 0.5, y as f64 * 2.0 + 1.0);
                    if let Some(st) = self.stones.iter().find(|s| ((at.0 - s.0) / s.2).powi(2) + ((at.1 - s.1) / s.3).powi(2) < 1.0) {
                        return th.bg.mix(if at.1 < st.1 - st.3 * 0.3 { STONE.mix(BROOD, 0.25) } else { STONE }, 0.7);
                    }
                    if self.plants.iter().any(|p| !p.gone() && deep < p.h * 0.7 && (at.0 - p.x - 2.5 * (deep * 0.35 + p.x).sin()).abs() < 0.8) {
                        return th.bg.mix(b.mix(Rgb(0, 0, 0), 0.35), much);
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

    /// The ant being followed, marked: a point over it, beating.
    fn marker(px: &mut [Rgb], pw: usize, at: (f64, f64), t: f64, th: &Theme) {
        put(px, pw, at.0, at.1 - 4.0, th.accent, 1.0);
        if (t * 5.0).sin() > 0.0 {
            put(px, pw, at.0 - 1.0, at.1 - 6.0, th.accent, 0.9);
            put(px, pw, at.0 + 1.0, at.1 - 6.0, th.accent, 0.9);
        }
    }

    /// How big an ant's drawn: a minim, a worker, a major.
    fn build(a: &Ant) -> u8 {
        match a.caste {
            Caste::Minim => 0,
            Caste::Major => 2,
            _ => 1,
        }
    }

    /// What's drawn with an ant: its wings, or what it's carrying.
    fn load(&self, a: &Ant, body: Rgb, th: &Theme) -> Option<(Rgb, bool)> {
        if a.caste == Caste::Alate {
            return Some((BROOD, true));
        }
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
        // The other colony's: dark, or where ours are, red.
        let theirs = if self.species == Species::Black { Rgb(196, 62, 40) } else { Rgb(30, 30, 36) };
        let theirs = if far(theirs, under) < 150 { theirs.mix(th.fg, 0.75) } else { theirs };
        match self.top {
            true => self.above(px, t, body, theirs, th),
            false => self.beside(px, t, body, theirs, th),
        }
    }

    /// From above: the plants, the food, the nest's hole in its ring of
    /// what was dug out, and those that are out.
    fn above(&self, px: &mut [Rgb], t: f64, body: Rgb, theirs: Rgb, th: &Theme) {
        let pw = self.pw;
        for p in self.plants.iter().filter(|p| !p.gone()) {
            for (k, &(_, _, left)) in p.leaves.iter().enumerate() {
                let ang = k as f64 * 2.4;
                if left > 0.05 {
                    self.leaf(px, (p.x + 4.0 * ang.cos(), p.z + 4.0 * ang.sin()), 4.0 * left.sqrt(), 3.0 * left.sqrt());
                }
            }
            // Dead, what's left of it is brown.
            put(px, pw, p.x, p.z, if p.alive() { STEM } else { TWIG }, 1.0);
            if !p.alive() {
                for k in 0..4 {
                    let ang = k as f64 * 1.6 + p.x;
                    put(px, pw, p.x + 2.0 * ang.cos(), p.z + 2.0 * ang.sin(), TWIG, 0.7);
                }
            }
            for k in 0..p.aphids.ceil() as usize {
                let ang = k as f64 * 1.9 + 0.7;
                put(px, pw, p.x + 3.5 * ang.cos(), p.z + 3.5 * ang.sin(), APHID, 1.0);
            }
            if p.bug > 0.0 {
                put(px, pw, p.x + 1.0, p.z - 2.0, LADYBIRD, 1.0);
                put(px, pw, p.x + 2.0, p.z - 2.0, LADYBIRD.mix(Rgb(0, 0, 0), 0.5), 1.0);
            }
        }
        // The other colony's nest, in its corner.
        for k in 0..24 {
            let ang = k as f64 / 24.0 * TAU;
            put(px, pw, self.camp.0 + 3.0 * ang.cos(), self.camp.1 + 3.0 * ang.sin(), self.tones.1, 0.6);
        }
        put(px, pw, self.camp.0, self.camp.1, th.bg.mix(Rgb(0, 0, 0), 0.5), 1.0);
        // A twig, in bits as it rots.
        for t in &self.twigs {
            let len = ((t.b.0 - t.a.0).powi(2) + (t.b.1 - t.a.1).powi(2)).sqrt();
            for k in 0..(len * 2.0) as usize {
                if (k * 7919 % 97) as f64 / 97.0 < t.left + 0.15 {
                    let u = k as f64 / (len * 2.0);
                    put(px, pw, t.a.0 + (t.b.0 - t.a.0) * u, t.a.1 + (t.b.1 - t.a.1) * u, TWIG, 0.5 + 0.5 * t.left);
                }
            }
        }
        if let Some(s) = &self.hunter {
            // Its body, long the way it's going, and its legs out each
            // side, each pair stepping in turn.
            let c = if far(SPIDER, th.bg) < 150 { SPIDER.mix(th.fg, 0.55) } else { SPIDER };
            let (dx, dz) = (s.dir.cos(), s.dir.sin());
            for k in [-1.0, 0.0, 1.0] {
                put(px, pw, s.x + dx * k, s.z + dz * k, c, 1.0);
            }
            let step = if ((s.x + s.z) * 0.6).sin() > 0.0 { 0.6 } else { -0.6 };
            for (along, side) in [(1.0 + step, 1.0), (-1.0 - step, 1.0), (1.0 - step, -1.0), (-1.0 + step, -1.0)] {
                put(px, pw, s.x + dx * along - dz * 2.4 * side, s.z + dz * along + dx * 2.4 * side, c, 0.8);
            }
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
        // The one being followed is down the hole: marked there.
        if self.follow.is_some_and(|id| self.ants.iter().any(|a| a.id == id && !matches!(a.loc, Loc::Surface(..) | Loc::Air(..)))) {
            Self::marker(px, pw, n, t, th);
        }
        for d in &self.dead {
            if let Loc::Surface(x, z) = d.0 {
                put(px, pw, x, z, body.mix(Rgb(0, 0, 0), 0.55), 1.0);
            }
        }
        for r in &self.rivals {
            ant(px, pw, (r.x, r.z), (r.dir.cos(), r.dir.sin()), theirs, 1, r.has.then_some((CRUMB, false)));
        }
        for a in &self.ants {
            match a.loc {
                Loc::Surface(x, z) => {
                    ant(px, pw, (x, z), (a.heading.cos(), a.heading.sin()), body, Self::build(a), self.load(a, body, th));
                    if self.follow == Some(a.id) {
                        Self::marker(px, pw, (x, z), t, th);
                    }
                }
                // On the wing: off, any way, the further for being higher.
                Loc::Air(x, z, up) => {
                    let at = (x + up * 1.6 * a.ph.cos(), z + up * 1.6 * a.ph.sin());
                    put(px, pw, at.0, at.1, body, 1.0);
                    let beat = if (t * 14.0 + a.ph).sin() > 0.0 { 1.0 } else { 0.4 };
                    put(px, pw, at.0 - 1.0, at.1, BROOD, beat);
                    put(px, pw, at.0 + 1.0, at.1, BROOD, beat);
                }
                _ => {}
            }
        }
        // Rain: where each drop lands, a ring going out from it and
        // fading, the next drop somewhere else.
        if self.rain > 0.0 {
            for k in 0..(self.pw * self.ph / 500).max(4) {
                let beat = t / (0.7 + (k % 5) as f64 * 0.09) + k as f64 * 0.37;
                let (drop, age) = (beat.floor() as usize, beat.fract());
                let n = (k * 7919 + drop * 104729).wrapping_mul(2654435761) >> 5;
                let (x, z) = ((n % self.pw) as f64, ((n / self.pw) % (self.ph * 2)) as f64);
                if age < 0.15 {
                    put(px, pw, x, z, RAIN, 0.8);
                }
                let (wide, points) = (0.8 + 3.6 * age, 12);
                for i in 0..points {
                    let ang = i as f64 / points as f64 * TAU;
                    put(px, pw, x + wide * ang.cos(), z + wide * ang.sin(), RAIN, 0.5 * (1.0 - age).powi(2));
                }
            }
        }
    }

    /// From the side: the plants, the nest and what's in it, and everyone.
    fn beside(&self, px: &mut [Rgb], t: f64, body: Rgb, theirs: Rgb, th: &Theme) {
        let pw = self.pw;
        // By night, stars, and a moon.
        let dark = 1.0 - self.light();
        if dark > 0.4 {
            for k in 0..self.pw / 6 {
                let (x, y) = ((k * 97 % self.pw) as f64, (k * 61 % 23) as f64 / 23.0 * (self.gy - 6.0).max(1.0));
                put(px, pw, x, y, BROOD, 0.55 * (dark - 0.4) / 0.6 * (0.6 + 0.4 * (t * 0.7 + k as f64).sin()));
            }
            for (dx, dy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 2.0), (1.0, 2.0)] {
                put(px, pw, self.w * 0.8 + dx, self.gy * 0.2 + dy, BROOD, 0.9 * (dark - 0.4) / 0.6);
            }
        }
        // Grass, where the ground's bare; the plants, their leaves as
        // much as is left of them.
        for k in 0..self.pw / 3 {
            let x = (k * 53 % self.pw) as f64;
            if (x - self.nest.0).abs() > 14.0 && !self.gel && self.season() != 3 {
                put(px, pw, x, self.ground(x) - 1.0, STEM, 0.7);
            }
        }
        for (p, pl) in self.plants.iter().enumerate().filter(|(_, pl)| !pl.gone()) {
            // Dead, it's a brown stalk, bent over, shorter as it rots.
            let (tone, tall) = if pl.alive() { (STEM, pl.h) } else { (TWIG, pl.h * (0.35 + 0.5 * pl.wither / 90.0)) };
            let mut up = 0.0;
            while up <= tall {
                let s = self.stem(p, up);
                put(px, pw, s.0, s.1, tone, 1.0);
                up += 0.5;
            }
            for &(up, side, left) in &pl.leaves {
                let (c, rx, ry) = (self.stem(p, up), 5.5 * left.sqrt(), 3.4 * left.sqrt());
                if left > 0.05 {
                    self.leaf(px, (c.0 + side * (rx + 0.5), c.1 + 0.6 * (t * 0.8 + up).sin()), rx, ry);
                }
            }
            // Its aphids, along the stem and out on the leaves.
            for k in 0..pl.aphids.ceil() as usize {
                let (up, side, _) = pl.leaves[k % pl.leaves.len().max(1)];
                let c = self.stem(p, up);
                put(px, pw, c.0 + side * (1.0 + (k / pl.leaves.len().max(1)) as f64 * 2.0), c.1 - 2.0, APHID, 1.0);
            }
            if pl.bug > 0.0 {
                let c = self.stem(p, pl.leaves.last().map_or(pl.h, |l| l.0));
                put(px, pw, c.0 + 2.0, c.1 - 2.0, LADYBIRD, 1.0);
                put(px, pw, c.0 + 3.0, c.1 - 2.0, LADYBIRD.mix(Rgb(0, 0, 0), 0.5), 1.0);
            }
        }
        // Rain, slanting down; and the door stopped up against it.
        if self.rain > 0.0 {
            for k in 0..self.pw / 3 {
                let fall = (t * 46.0 + (k * 37 % 101) as f64 * 3.0) % self.gy.max(1.0);
                let x = (k * 61 % self.pw) as f64 - fall * 0.25;
                put(px, pw, x, fall, RAIN, 0.5);
                put(px, pw, x + 0.5, fall - 2.0, RAIN, 0.25);
            }
        }
        if self.plug > 0.2 {
            for dx in [-1.0, 0.0, 1.0] {
                for row in 0..(self.plug * 2.0).ceil() as usize {
                    put(px, pw, self.nest.0 + dx, self.gy - 1.0 - 2.0 * row as f64, self.tones.1.mix(th.bg, 0.2), 1.0);
                }
            }
        }
        for r in &self.rivals {
            ant(px, pw, (r.x, self.ground(r.x) - 1.0), (r.dir.cos().signum(), 0.0), theirs, 1, r.has.then_some((CRUMB, false)));
        }
        for p in &self.piles {
            for k in 0..(p.2 / 3.0).ceil() as usize {
                put(px, pw, p.0 + k as f64 - 1.0, self.ground(p.0) - 1.0, CRUMB, 1.0);
            }
        }
        // A twig lying on the ground, as much across as shows from here.
        for tw in &self.twigs {
            let (lo, hi) = (tw.a.0.min(tw.b.0), tw.a.0.max(tw.b.0).max(tw.a.0.min(tw.b.0) + 2.0));
            for k in 0..(hi - lo) as usize {
                if (k * 7919 % 97) as f64 / 97.0 < tw.left + 0.15 {
                    put(px, pw, lo + k as f64, self.ground(lo + k as f64) - 1.0, TWIG, 0.5 + 0.5 * tw.left);
                }
            }
        }
        if let Some(s) = &self.hunter {
            let c = if far(SPIDER, th.bg) < 150 { SPIDER.mix(th.fg, 0.55) } else { SPIDER };
            let y = self.ground(s.x) - 1.0;
            for (dx, dy, a) in [(-1.0, 0.0, 1.0), (0.0, 0.0, 1.0), (1.0, 0.0, 1.0), (0.0, -2.0, 1.0), (-2.0, 0.0, 0.7), (2.0, 0.0, 0.7), (-2.0, -2.0, 0.5), (2.0, -2.0, 0.5)] {
                put(px, pw, s.x + dx, y + dy, c, a);
            }
        }
        // The store: leaf turned to fungus, or what was gathered. The
        // midden's dark.
        let leaf = habit(self.species).leaf;
        // Fresh leaf lying by the garden it'll become.
        let s = self.larder();
        match self.species {
            // Honeypot ants keep it in themselves: workers hanging from
            // the chamber's roof, full.
            Species::Honeypot => {
                let (c, n) = (self.nodes[s].end(), ((self.stock / 2.0) as usize).min(26));
                for k in 0..n {
                    let u = (k as f64 + 0.5) / n as f64 * 2.0 - 1.0;
                    let (x, top) = (c.0 + u * self.nodes[s].rx * 0.85, c.1 - self.nodes[s].ry * (1.0 - u * u).max(0.0).sqrt());
                    put(px, pw, x, top + 1.0, body, 1.0);
                    put(px, pw, x, top + 3.0, HONEY, 1.0);
                }
            }
            _ => self.pile(px, s, (self.stock * 1.5) as usize, if leaf { FUNGUS } else { CRUMB }, 3),
        }
        self.pile(px, s, (self.leaf * 1.5) as usize, LEAF, 23);
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
        if let Some(q) = self.room(Room::Queen).filter(|_| self.queen) {
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
            ant(px, pw, at, dir, c, Self::build(a), self.load(a, body, th));
            if a.rider {
                put(px, pw, at.0, at.1 - 4.0, BROOD.mix(body, 0.4), 1.0);
            }
            if self.follow == Some(a.id) {
                Self::marker(px, pw, at, t, th);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colony(said: &str, secs: usize) -> Colony {
        let mut c = Colony::new(160, 60, 5, &format!("grown {said}"), Some(30));
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
        let mut c = Colony::new(160, 60, 9, "grown black sand", Some(40));
        (c.cloud, c.muster) = (f64::MAX, f64::MAX);
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
                    let near = (x - c.nest.0).powi(2) + (z - c.nest.1).powi(2) < 100.0;
                    assert!(k < 90 || near || !text[z as usize / 4 * 80 + x as usize / 2], "frame {k}");
                }
            }
        }
        assert!(stored > 8.0 && trail && down, "{stored} {trail} {down}");
        all_in_holes(&c);
    }

    #[test]
    fn plants_are_far_and_twigs_are_in_the_way_till_they_rot() {
        let mut c = colony("", 0);
        // Plants a way off from the nest, across as well.
        assert_eq!(c.plants.len(), 3);
        for p in &c.plants {
            assert!((p.x - c.nest.0).abs() > 25.0, "{} {}", p.x, c.nest.0);
        }
        // The twig that's lying there: nothing's on it, once those it fell
        // on have walked off.
        let t = (c.twigs[0].a, c.twigs[0].b);
        let mid = ((t.0.0 + t.1.0) / 2.0, (t.0.1 + t.1.1) / 2.0);
        assert!(c.blocked(mid.0, mid.1) && !c.blocked(c.nest.0, c.nest.1));
        c.prowl = f64::MAX;
        for k in 0..60 * 30 {
            c.step(1.0 / 30.0, k as f64 / 30.0, &[], &[]);
            for a in &c.ants {
                if let Loc::Surface(x, z) = a.loc {
                    assert!(k < 90 || !c.twigs.iter().any(|t| t.left > 0.12 && off((x, z), t.a, t.b) < 1.0) || (x - c.nest.0).powi(2) + (z - c.nest.1).powi(2) < 100.0, "frame {k}");
                }
            }
        }
        // Rotted away, in time, and the way's clear.
        c.twigs.truncate(1);
        c.fall = f64::MAX;
        for k in 0..150 * 30 {
            c.step(1.0 / 30.0, 60.0 + k as f64 / 30.0, &[], &[]);
        }
        assert!(c.twigs.is_empty() && !c.blocked(mid.0, mid.1));
    }

    #[test]
    fn a_spider_eats_some_and_goes_or_is_brought_down() {
        // Black ants have none that fight: it eats, and leaves.
        let mut c = Colony::new(160, 60, 5, "grown black", Some(40));
        (c.prowl, c.muster, c.cloud) = (0.0, f64::MAX, f64::MAX);
        let (mut came, mut most) = (false, 0);
        for k in 0..120 * 30 {
            c.step(1.0 / 30.0, k as f64 / 30.0, &[], &[]);
            came |= c.hunter.is_some();
            most = most.max(c.hunter.as_ref().map_or(0, |s| s.fed));
        }
        assert!(came && c.hunter.is_none() && (1..=3).contains(&most), "{came} {most}");
        // Fire ants all fight: it's brought down, and carried home.
        let mut c = Colony::new(160, 60, 5, "grown fire", Some(60));
        (c.prowl, c.muster, c.cloud) = (0.0, f64::MAX, f64::MAX);
        let (mut prize, mut stored) = (false, 0.0);
        for k in 0..240 * 30 {
            let was = c.stock;
            c.step(1.0 / 30.0, k as f64 / 30.0, &[], &[]);
            prize |= c.piles.iter().any(|p| p.2 == 14.0);
            stored += (c.stock - was).max(0.0);
        }
        assert!(prize && stored > 5.0, "{prize} {stored}");
    }

    #[test]
    fn a_trip_takes_as_long_on_any_screen() {
        // How long the first leaf takes to reach the store, on a small
        // screen and on a big one.
        let first = |pw: usize, ph: usize| {
            let mut c = Colony::new(pw, ph, 5, "grown", Some(30));
            (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
            (0..300 * 30).find(|&k| {
                c.step(1.0 / 30.0, k as f64 / 30.0, &[], &[]);
                c.leaf > 0.0
            })
        };
        let (small, big) = (first(120, 40).unwrap() as f64, first(360, 120).unwrap() as f64);
        assert!(big < small * 2.2 && small < big * 2.2, "{small} {big}");
        let c = Colony::new(360, 120, 5, "", None);
        assert!((c.scale() - 3.0 * (120.0 / 220.0 * 80.0 / 128.0f64).sqrt() * 1.0).abs() < 1.0 && c.scale() > 1.5);
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

    /// A colony run for `secs`, looked at each frame.
    fn watch(c: &mut Colony, from: f64, secs: usize, mut see: impl FnMut(&Colony)) {
        for k in 0..secs * 30 {
            c.step(1.0 / 30.0, from + k as f64 / 30.0, &[], &[]);
            see(c);
        }
    }

    #[test]
    fn a_queen_alone_founds_it() {
        let mut c = Colony::new(160, 60, 5, "", Some(30));
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        // Her, her eggs, and the shaft she dug; no store yet, no workers.
        assert!(c.ants.is_empty() && c.brood.len() == 6 && c.room(Room::Store).is_none() && c.room(Room::Queen).is_some());
        watch(&mut c, 0.0, 70, |_| {});
        // The first are small, and soon.
        assert!(c.ants.len() >= 5 && c.ants.iter().all(|a| a.caste == Caste::Minim), "{}", c.ants.len());
        // They dig, they forage, she goes on laying: it grows.
        let mut seen = [false; 2];
        watch(&mut c, 70.0, 420, |c| {
            seen[0] |= c.ants.iter().any(|a| a.carry == Carry::Leaf);
            seen[1] |= c.ants.iter().any(|a| a.carry == Carry::Grain);
        });
        assert_eq!(seen, [true; 2]);
        assert!(c.ants.len() > 8 && c.room(Room::Store).is_some(), "{} {}", c.ants.len(), c.status());
        all_in_holes(&c);
    }

    #[test]
    fn night_winter_and_rain_keep_them_in() {
        let out = |c: &Colony| c.ants.iter().filter(|a| a.goal == Goal::Seek).count();
        let mut c = colony("", 30);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        assert!(c.abroad() && out(&c) > 0);
        // Night: none out looking, and none sent.
        c.clock = DAY * 23.0 / 24.0;
        watch(&mut c, 30.0, 20, |_| {});
        assert!(!c.abroad() && out(&c) == 0 && c.light() == 0.0);
        // Rain: the door stopped up, the trails washed off, and after it,
        // opened again.
        c.clock = DAY * 12.0 / 24.0;
        watch(&mut c, 50.0, 60, |_| {});
        assert!(out(&c) > 0);
        c.scent.fill(2.0);
        c.rain = 30.0;
        watch(&mut c, 110.0, 25, |_| {});
        assert!(c.plug == 1.0 && out(&c) == 0 && c.scent.iter().all(|v| *v < 0.01), "{}", c.plug);
        watch(&mut c, 135.0, 40, |_| {});
        assert!(c.rain <= 0.0 && c.plug == 0.0 && out(&c) > 0 && c.status().contains("workers"));
        // Winter: the leaves gone, no eggs laid, everyone in.
        c.clock = DAY * (15.2 + 12.0 / 24.0);
        let eggs = |c: &Colony| c.brood.iter().filter(|b| b.3 < 5.0).count();
        watch(&mut c, 175.0, 60, |_| {});
        assert!(c.season() == 3 && out(&c) == 0 && eggs(&c) == 0 && c.status().contains("winter"));
        assert!(c.plants.iter().flat_map(|p| &p.leaves).all(|l| l.2 == 0.0));
    }

    #[test]
    fn they_share_food_keep_watch_move_the_brood_and_lead_each_other() {
        // Leafcutters: all three sizes, the big on watch, minims riding
        // leaves; underground, they stop and share.
        let mut c = colony("", 0);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        let mut seen = [false; 5];
        watch(&mut c, 0.0, 240, |c| {
            seen[0] |= c.ants.iter().any(|a| a.greet > 0.0);
            seen[1] |= c.ants.iter().any(|a| a.job == Job::Guard && a.caste == Caste::Major);
            seen[2] |= c.ants.iter().any(|a| a.rider);
            seen[3] |= c.leaf > 0.0;
            // Out looking: the old, or anyone when too few are.
            seen[4] |= c.ants.iter().any(|a| a.job == Job::Forage && a.age > a.life * 0.6);
        });
        assert_eq!(seen, [true; 5]);
        assert!([Caste::Minim, Caste::Media, Caste::Major].iter().all(|k| c.ants.iter().any(|a| a.caste == *k)));
        // The brood goes up to the nursery by day, once there's one, and
        // down to the queen at night.
        let (queen, nursery) = (c.room(Room::Queen).unwrap(), c.nodes.iter().position(|n| n.room == Room::Nursery).unwrap());
        (c.nodes[nursery].dug, c.nodes[nursery].open) = (c.nodes[nursery].len, 1.0);
        let way = c.nodes[nursery].parent;
        c.nodes[way].dug = c.nodes[way].len;
        c.carve();
        c.clock = DAY * 9.0 / 24.0;
        watch(&mut c, 240.0, 90, |_| {});
        assert!(c.brood.iter().any(|b| b.0 == nursery), "{:?}", c.brood.iter().map(|b| b.0).collect::<Vec<_>>());
        c.clock = DAY * 23.0 / 24.0;
        watch(&mut c, 330.0, 90, |_| {});
        assert!(c.brood.iter().filter(|b| b.0 == queen).count() * 2 >= c.brood.len());
        // Garden ants: one that finds plenty leads another to it, and
        // they milk the aphids on the plants.
        let mut c = colony("black", 0);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        let (mut led, mut milked) = (false, false);
        watch(&mut c, 0.0, 300, |c| {
            led |= c.ants.iter().any(|a| matches!(a.goal, Goal::Follow(_)) && matches!(a.loc, Loc::Surface(..)));
            milked |= c.plants.iter().any(|p| p.aphids < 5.0);
        });
        assert!(led && milked, "{led} {milked}");
    }

    #[test]
    fn rivals_come_and_the_winged_fly() {
        let mut c = colony("fire", 0);
        (c.cloud, c.prowl) = (f64::MAX, f64::MAX);
        let (mut most, mut robbed, mut fought, mut said) = (0, false, false, false);
        watch(&mut c, 0.0, 240, |c| {
            most = most.max(c.rivals.len());
            robbed |= c.rivals.iter().any(|r| r.has);
            fought |= c.rivals.iter().any(|r| r.grip > 0.0);
            said |= c.status().contains("rivals");
        });
        assert!(most >= 3 && robbed && fought && said, "{most} {robbed} {fought}");
        // Summer, the colony grown and fed: the winged are raised, wait
        // for a bright day, and are gone.
        c.muster = f64::MAX;
        c.rivals.clear();
        c.clock = DAY * (6.0 + 10.0 / 24.0);
        while c.ants.len() * 10 < c.cap * 6 {
            let a = c.hatch(Loc::Surface(c.nest.0, c.nest.1), 0.3);
            c.ants.push(a);
        }
        c.stock = 30.0;
        let (mut winged, mut up) = (0, false);
        watch(&mut c, 240.0, 150, |c| {
            winged = winged.max(c.ants.iter().filter(|a| a.caste == Caste::Alate).count());
            up |= c.ants.iter().any(|a| matches!(a.loc, Loc::Air(..)));
        });
        // (One may have died waiting, or missed the light.)
        let left = |c: &Colony| c.ants.iter().filter(|a| a.caste == Caste::Alate).count();
        assert!(winged >= 5 && up && c.flown >= 4 && c.raise == 0, "{winged} {up} {}", c.flown);
        assert!(left(&c) <= 1 && c.status().contains("flown"));
        // Not again this year.
        let flown = c.flown;
        watch(&mut c, 390.0, 30, |_| {});
        assert!(left(&c) <= 1 && c.flown <= flown + 1 && c.raise == 0);
    }

    #[test]
    fn a_colony_written_out_comes_back_the_same() {
        let mut c = colony("black", 200);
        c.rain = 20.0;
        c.step(1.0 / 30.0, 200.0, &[], &[]);
        let mut d = Colony::new(160, 60, 77, "black", Some(30));
        assert!(d.restore(&c.snapshot()));
        assert_eq!((d.status(), d.snapshot_of(), d.history()), (c.status(), c.snapshot_of(), c.history()));
        assert_eq!(d.snapshot().len(), c.snapshot().len());
        // One just begun, too, on a screen of another size.
        let mut young = Colony::new(220, 60, 3, "", None);
        watch(&mut young, 0.0, 12, |_| {});
        let mut again = Colony::new(200, 80, 4, "", None);
        assert!(again.restore(&young.snapshot()));
        assert_eq!((again.status(), again.size()), (young.status(), (200, 80)));
        // Cut short, or of another making: left alone.
        let whole = c.snapshot();
        assert!(!d.restore(&whole[..whole.len() / 2]) && !d.restore(&[0; 64]));
        assert_eq!(d.status(), c.status());
    }

    #[test]
    fn its_keeper_follows_one_names_the_chambers_and_reads_what_happened() {
        let mut c = colony("", 30);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        // f follows one; its line says who it is; f lets it go.
        c.key('f');
        let id = c.follow.unwrap();
        assert!(c.status().starts_with(&format!("ant {id} · ")) && c.status().contains("days old"), "{}", c.status());
        c.key('f');
        assert!(c.follow.is_none() && c.status().starts_with("day 1"));
        // A click on an ant follows it, and isn't food; a click on nothing is.
        let a = c.ants.iter().find(|a| matches!(a.loc, Loc::Edge(..))).unwrap();
        let (at, id, piles) = (a.at, a.id, c.piles.len());
        c.step(1.0 / 30.0, 30.0, &[], &[(at.0, at.1 / 2.0)]);
        assert_eq!((c.follow, c.piles.len()), (Some(id), piles));
        c.step(1.0 / 30.0, 30.1, &[], &[(3.0, 1.0)]);
        assert_eq!(c.piles.len(), piles + 1);
        // Its time stopped, and quickened.
        c.follow = None;
        c.key('[');
        assert_eq!((c.speed(), c.status().contains("stopped")), (0, true));
        (0..3).for_each(|_| c.key(']'));
        assert_eq!(c.speed(), 16);
        c.key('[');
        c.key('[');
        // The chambers that are dug, named, from the side only.
        let names: Vec<&str> = c.labels().iter().map(|l| l.2).collect();
        assert!(names.contains(&"garden") && names.contains(&"queen"), "{names:?}");
        c.view(true);
        assert!(c.labels().is_empty());
        c.view(false);
        // What's happened: how it began, and after a winter, that.
        c.clock = DAY * 14.9;
        watch(&mut c, 31.0, 80, |_| {});
        let said = c.history().join("\n");
        assert!(said.contains("a queen founded it") && said.contains("winter came") && said.contains("workers, the last"), "{said}");
    }

    #[test]
    fn it_lives_on_while_it_is_away_and_after_its_queen() {
        // Left an hour: it's an hour older, and says so.
        let mut c = colony("", 10);
        let day = c.clock;
        c.stamp = 1000.0;
        c.catch_up(1000.0 + 3600.0);
        assert!((c.clock - day - 3600.0).abs() < 1.0 && c.history().join(" ").contains("left alone 60 minutes"));
        // Left a moment: nothing.
        let at = c.clock;
        c.stamp = 5000.0;
        c.catch_up(5010.0);
        assert_eq!(c.clock, at);
        // The queen's years run out: no more eggs, and when the last
        // worker's gone, another queen begins again in the same nest.
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        c.reign = c.span;
        c.clock = DAY * 2.4;
        watch(&mut c, 0.0, 2, |_| {});
        assert!(!c.queen && c.status().contains("no queen"));
        c.ants.clear();
        c.brood.clear();
        let dug = c.nodes.iter().filter(|n| n.done()).count();
        watch(&mut c, 2.0, 60, |_| {});
        assert!(c.queen && !c.ants.is_empty() && c.nodes.iter().filter(|n| n.done()).count() >= dug);
        let said = c.history().join(" ");
        assert!(said.contains("the queen died") && said.contains("a new queen came"), "{said}");
    }

    #[test]
    fn tasks_are_piles_stones_are_gone_round_and_seeds_sprout() {
        let mut c = colony("black", 0);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        // A pile a task: there while the task is, however much is taken.
        let had = c.piles.len();
        c.tasks(Some(4));
        assert_eq!((c.piles.len(), c.tasks.len()), (had + 4, 4));
        watch(&mut c, 0.0, 120, |c| assert!(c.tasks.iter().all(|id| c.piles.iter().any(|p| p.3 == *id && p.2 >= 5.0))));
        c.tasks(Some(1));
        assert_eq!(c.tasks.len(), 1);
        c.tasks(None);
        assert!(c.tasks.is_empty());
        // Stones, and no tunnel through one, however much is dug.
        let mut c = colony("", 0);
        assert!(c.stones.len() >= 2);
        for n in &mut c.nodes {
            (n.dug, n.open) = (n.len, 1.0);
        }
        c.carve();
        for s in &c.stones {
            let (x, y) = (s.0 as usize, (s.1 / 2.0) as usize);
            assert!(!c.hole[y.min(c.ph - 1) * c.pw + x.min(c.pw - 1)], "{s:?}");
        }
        // A full midden, in spring: a plant comes up.
        (c.cloud, c.prowl, c.muster, c.refuse) = (f64::MAX, f64::MAX, f64::MAX, 30);
        let plants = c.plants.len();
        watch(&mut c, 0.0, 200, |_| {});
        assert!(c.plants.len() > plants && c.history().join(" ").contains("sprouted"));
    }

    #[test]
    fn honeypots_hang_ladybirds_are_driven_off_and_army_ants_move_on() {
        // Honeypot ants milk aphids; a ladybird on a plant goes when an
        // ant gets there.
        let mut c = colony("honeypot", 0);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        assert!(c.milks() && c.plants.iter().all(|p| p.aphids == 6.0));
        c.plants.iter_mut().for_each(|p| p.bug = 1e9);
        watch(&mut c, 0.0, 200, |_| {});
        assert!(c.plants.iter().any(|p| p.bug == 0.0) && c.stock > 0.0);
        // Army ants: one hollow, nothing dug; toward evening they go, all
        // of them, and the hollow's where they went.
        let mut c = colony("army", 0);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        assert_eq!((c.nodes.len(), c.site(), c.labels()[0].2), (2, None, "bivouac"));
        let was = c.nest;
        c.clock = DAY * 17.6 / 24.0;
        let mut out = 0;
        watch(&mut c, 0.0, 100, |c| out = out.max(c.ants.iter().filter(|a| c.trek.is_some() && matches!(a.loc, Loc::Surface(..))).count()));
        assert!(c.nest != was && c.trek.is_none() && out * 5 >= c.ants.len() * 3, "{out} of {}", c.ants.len());
        assert!((c.nodes[1].end().0 - c.nest.0).abs() < 1.0 && c.history().join(" ").contains("moved on"));
        watch(&mut c, 100.0, 30, |_| {});
        all_in_holes(&c);
    }

    #[test]
    fn a_resize_keeps_the_trails() {
        let mut c = colony("black", 0);
        // A trail a pixel wide, across, and one down.
        for x in 20..120 {
            (c.scent[30 * c.pw + x], c.shown[30 * c.pw + x]) = (3.0, 3);
        }
        for y in 5..50 {
            (c.scent[y * c.pw + 80], c.shown[y * c.pw + 80]) = (3.0, 3);
        }
        let lit = |c: &Colony| c.shown.iter().filter(|s| **s == 3).count();
        // Bigger, it's still there, as long in proportion; and smaller, a
        // thin one isn't lost between the pixels.
        for (pw, ph) in [(240, 90), (100, 36), (61, 23), (160, 60)] {
            c.resize(pw, ph);
            let (across, down) = ((0..pw).filter(|&x| (0..ph).any(|y| c.shown[y * pw + x] == 3)).count(), (0..ph).filter(|&y| (0..pw).any(|x| c.shown[y * pw + x] == 3)).count());
            assert!(across * 160 >= pw * 95 && down * 60 >= ph * 40, "{pw}x{ph}: {across} {down} {}", lit(&c));
            assert!(c.scent.iter().zip(&c.shown).all(|(v, s)| (*v > 0.0) == (*s > 0)) && c.scent.len() == pw * ph);
            // And it's what's drawn, from above.
            c.view(true);
            let th = Theme::default();
            let mut field = vec![Rgb::default(); pw * ph];
            c.back(&mut field, &th);
            assert!(field.iter().any(|p| *p != field[0]));
            c.view(false);
        }
    }

    #[test]
    fn plants_die_and_others_come_up() {
        let mut c = colony("", 0);
        (c.cloud, c.prowl, c.muster, c.fall) = (f64::MAX, f64::MAX, f64::MAX, f64::MAX);
        c.ants.clear();
        c.brood.clear();
        c.queen = true;
        c.stock = 0.5;
        // One kept cut short, past its first days: it sickens and dies,
        // stands dead a while, and is gone; nothing goes to it for leaf.
        c.plants.iter_mut().for_each(|p| p.age = 500.0);
        let mut states = [false; 2];
        for k in 0..300 * 30 {
            c.plants[0].leaves.iter_mut().for_each(|l| l.2 = l.2.min(0.2));
            c.step(1.0 / 30.0, k as f64 / 30.0, &[], &[]);
            states[0] |= !c.plants[0].alive() && !c.plants[0].gone();
            states[1] |= c.plants[0].gone();
            c.ants.clear();
            c.brood.clear();
        }
        assert_eq!(states, [true; 2]);
        assert!(c.plants[1].alive() && c.plants[2].alive() && c.history().join(" ").contains("a plant died"));
        assert!(c.food(c.plants[0].x, c.plants[0].z, 3.0).is_none());
        // The rest dead too: a seedling comes up, where one was, so those
        // that knew the others by number still do.
        c.plants.iter_mut().for_each(|p| (p.health, p.wither) = (0.0, 0.0));
        c.clock = DAY * 1.4;
        watch(&mut c, 300.0, 240, |c| {
            c.plants.iter().for_each(|p| assert!(p.alive() || p.gone() || p.wither > 0.0));
        });
        assert!(c.plants.iter().filter(|p| p.alive()).count() >= 2 && c.plants.len() == 3, "{}", c.plants.len());
        assert!(c.history().join(" ").contains("a seedling came up"));
        // Each winter takes some: over a few, not none, and not all.
        let mut c = colony("", 0);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        let mut died = 0;
        for year in 0..6 {
            c.plants.iter_mut().for_each(|p| (p.health, p.age) = (1.0, 0.0));
            c.clock = DAY * (YEAR * year as f64 + 14.999);
            c.season_was = 2;
            watch(&mut c, 0.0, 3, |_| {});
            died += c.plants.iter().filter(|p| !p.alive()).count();
        }
        assert!((1..15).contains(&died), "{died}");
    }

    #[test]
    fn what_a_reader_found_wrong_stays_right() {
        // Army ants moved: what's written out still comes back.
        for seed in 0..16 {
            let mut c = Colony::new(160, 60, seed, "grown army", Some(30));
            (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
            c.clock = DAY * 17.6 / 24.0;
            watch(&mut c, 0.0, 90, |_| {});
            assert!(c.history().join(" ").contains("moved on") && c.nodes.iter().all(|n| n.done()), "seed {seed}");
            let mut d = Colony::new(160, 60, 99, "grown army", Some(30));
            assert!(d.restore(&c.snapshot()), "seed {seed}");
        }
        // The dead whose bearer was taken on the way are fetched by another.
        let mut c = colony("", 20);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        c.count += 1;
        let q = c.room(Room::Queen).unwrap();
        c.dead.push((Loc::Room(q, 0.0, 0.0), c.count, true));
        watch(&mut c, 20.0, 120, |_| {});
        assert!(c.dead.iter().all(|d| d.1 != c.count), "{}", c.dead.len());
        // Food under text, seen from above, is got to all the same.
        let mut c = colony("black", 0);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        c.view(true);
        let mut text = vec![false; 80 * 30];
        for r in 4..12 {
            text[r * 80 + 10..r * 80 + 40].fill(true);
        }
        c.piles.clear();
        c.plants.iter_mut().for_each(|p| p.aphids = 0.0);
        c.count += 1;
        c.piles.push((50.0, 32.0, 6.0, c.count));
        let id = c.count;
        for k in 0..240 * 30 {
            c.plants.iter_mut().for_each(|p| p.aphids = 0.0);
            c.step(1.0 / 30.0, k as f64 / 30.0, &text, &[]);
        }
        assert!(c.piles.iter().all(|p| p.3 != id || p.2 < 6.0), "{:?}", c.piles);
        // A screen a cell wide, with a spider and a rival's dead: no panic.
        let mut c = Colony::new(2, 12, 1, "grown fire", None);
        c.hunter = Some(Spider { x: 0.5, z: 3.0, dir: 0.0, hp: 0.0, fed: 0, bite: 0.0, stay: 9.0 });
        c.rivals.push(Rival { x: 0.5, z: 3.0, dir: 0.0, has: false, grip: 0.0 });
        watch(&mut c, 0.0, 20, |_| {});
        // A colony comes through the winter.
        let mut c = colony("", 60);
        (c.cloud, c.prowl, c.muster) = (f64::MAX, f64::MAX, f64::MAX);
        let before = c.ants.len();
        c.clock = DAY * 15.0;
        c.stock = 40.0;
        watch(&mut c, 60.0, 490, |_| {});
        assert!(c.season() == 0 && c.ants.len() * 2 >= before, "{} of {before}", c.ants.len());
        // A file that makes no sense, in ways that would have hung or
        // panicked, is left alone.
        let whole = c.snapshot();
        let mut bad = whole.clone();
        bad[8..16].copy_from_slice(&1e300f64.to_le_bytes());
        assert!(!c.restore(&bad));
        assert!(c.restore(&whole));
    }
}
