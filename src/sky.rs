//! What's behind a slide, alive the whole time it's up: `sky:` stars,
//! snow, rain, embers or life, and `glow:`, a light the headline's letters
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
}

impl Kind {
    pub fn from(name: &str) -> Kind {
        match name {
            "stars" => Kind::Stars,
            "snow" => Kind::Snow,
            "rain" => Kind::Rain,
            "embers" => Kind::Embers,
            "life" => Kind::Life,
            "boids" => Kind::Boids,
            "fireflies" => Kind::Fireflies,
            _ => Kind::None,
        }
    }
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
}

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
        };
        s.fill();
        s
    }

    fn rand(&mut self) -> f64 {
        self.rng.below(1 << 20) as f64 / (1 << 20) as f64
    }

    /// The motes it starts with, spread over the whole screen, so it's
    /// already going when the slide comes.
    fn fill(&mut self) {
        let area = (self.pw * self.ph) as f64;
        let n = match self.kind {
            Kind::Stars => area / 110.0,
            Kind::Snow => self.pw as f64 / 2.5,
            Kind::Rain => self.pw as f64 / 3.0,
            Kind::Embers => self.pw as f64 / 2.5,
            Kind::Boids => (area / 160.0).max(40.0),
            Kind::Fireflies => 18.0,
            Kind::Life => {
                for i in 0..self.cells.len() {
                    self.cells[i] = self.rand() < 0.16;
                }
                0.0
            }
            Kind::None => 0.0,
        } as usize;
        for _ in 0..n {
            let m = self.spawn(true);
            self.motes.push(m);
        }
        if self.kind == Kind::Boids {
            for _ in 0..2 {
                let m = self.spawn(true);
                self.hawks.push(Mote { vx: m.vx * 1.4, vy: m.vy * 1.4, ..m });
            }
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

    /// On to time t, in seconds: everything moved, then the pixels drawn
    /// again, the glow from the headline's letters among `front`.
    pub fn frame(&mut self, t: f64, theme: &Theme, front: &[Option<Cell>], w: usize) {
        let dt = (t - self.t).clamp(0.0, 0.1);
        self.t = t;
        // Where text is first, for what steers around it.
        self.hush(front, w);
        self.step(dt, t);
        let base = theme.bg;
        self.field.fill(base);
        if self.glow {
            self.shine(t, base, theme.accent, front, w);
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
            if gone {
                m = self.spawn(false);
            }
            self.motes[i] = m;
        }
        match self.kind {
            Kind::Boids => self.flock(dt),
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
        for (i, m) in self.motes.iter_mut().enumerate() {
            let (mut cx, mut cy, mut ax, mut ay, mut sx, mut sy, mut n) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
            for (j, o) in all.iter().enumerate() {
                let (dx, dy) = (o.x - m.x, (o.y - m.y) * 2.0);
                let d = (dx * dx + dy * dy).sqrt();
                if i == j || d > 16.0 {
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

    fn dot(&mut self, x: f64, y: f64, c: Rgb, a: f64) {
        let (x, y) = (x.round(), y.round());
        if x < 0.0 || y < 0.0 || x >= self.pw as f64 || y >= self.ph as f64 || a <= 0.0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        let hushed = self.quiet.get(y / 2 * (self.pw / 2) + x / 2).is_some_and(|q| *q);
        let p = &mut self.px[y * self.pw + x];
        *p = p.mix(c, a.min(1.0) * if hushed { 0.12 } else { 1.0 });
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
    fn every_kind_runs() {
        let th = Theme::default();
        for k in [Kind::Stars, Kind::Snow, Kind::Rain, Kind::Embers, Kind::Life] {
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
