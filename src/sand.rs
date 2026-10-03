//! tr: sand. The slide going crumbles: every letter a few grains, four to a
//! cell, as tr: life has them, each letting go in its own time and falling,
//! in the color it was, into a heap along the bottom that slides where it's
//! steep. Then the floor opens and the heap runs out.

use crate::life::seed;
use crate::markup::{Cell, Rgb};
use crate::screen::Screen;
use crate::sky::quad;

pub fn play(s: &mut Screen, cells: &[Option<Cell>]) {
    let (w, h) = (s.w as usize, s.h as usize);
    // The last row but one is the floor: the slides' dots stay.
    let (pw, ph) = (w * 2, (h - 1) * 2);
    let mut on: Vec<Option<Rgb>> = vec![None; pw * ph];
    for (i, c) in cells.iter().enumerate().take(w * (h - 1)) {
        let Some(c) = c else { continue };
        let m = seed(c.ch);
        let (x, y) = ((i % w) * 2, (i / w) * 2);
        for (k, (dx, dy)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
            if m & (1 << k) != 0 {
                on[(y + dy) * pw + x + dx] = Some(c.st.fg.unwrap_or(s.theme.fg));
            }
        }
    }
    // When each grain lets go, by where it starts: no two neighbors together.
    let mut held: Vec<u8> = (0..pw * ph).map(|i| ((i % pw * 7 + i / pw * 13) % 10) as u8).collect();
    let (bg, frames) = (s.theme.bg, 44);
    let mut was: Vec<Option<Cell>> = vec![None; w * h];
    for f in 0..=frames {
        if s.hurry {
            break;
        }
        for r in 0..h - 1 {
            for c in 0..w {
                let px = |dx: usize, dy: usize| on[(2 * r + dy) * pw + 2 * c + dx].unwrap_or(bg);
                let cell = quad([px(0, 0), px(1, 0), px(0, 1), px(1, 1)]);
                // Only what changed goes out.
                if was[r * w + c] != Some(cell) {
                    was[r * w + c] = Some(cell);
                    s.put(r as i32 + 1, c as i32 + 1, &[cell]);
                }
            }
        }
        // The first frame held a moment, the letters as grains, then on,
        // faster as it goes, as things fall.
        s.tick(if f == 0 { 0.18 } else { 0.03 });
        for _ in 0..2 + f / 3 {
            for y in (0..ph - 1).rev() {
                for x in 0..pw {
                    let i = y * pw + x;
                    if on[i].is_none() || held[i] as usize > f {
                        continue;
                    }
                    let side = if s.rng.below(2) == 0 { 1 } else { -1 };
                    let to = [0, side, -side].into_iter().map(|d| x as i32 + d).find(|&nx| nx >= 0 && (nx as usize) < pw && on[(y + 1) * pw + nx as usize].is_none());
                    if let Some(nx) = to {
                        let j = (y + 1) * pw + nx as usize;
                        (on[j], held[j]) = (on[i].take(), 0);
                    }
                }
            }
            // The floor opens once it's all down.
            if f > frames * 2 / 3 {
                on[(ph - 1) * pw..].fill(None);
            }
        }
    }
    s.clear();
}
