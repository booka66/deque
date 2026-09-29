//! tr: life. The slide going becomes Conway's Game of Life: every letter
//! a few living pixels, four to a cell, each block all four, then the
//! rules run on them, the letters breaking up, spreading and dying out, in
//! the colors they were. What's born takes a neighbor's color.

use crate::markup::{Cell, Rgb};
use crate::screen::Screen;
use crate::sky::{QUAD, quad};

/// Which of a cell's four pixels a letter lights: a block, the ones it
/// fills; anything else, a pattern of two or three picked by the letter, so
/// the same word seeds the same way each time.
fn seed(ch: char) -> u32 {
    if let Some(m) = QUAD.iter().position(|&q| q == ch) {
        return m as u32;
    }
    match ch {
        ' ' => 0,
        _ => [0b0110, 0b1001, 0b0111, 0b1011, 0b1101, 0b1110, 0b0101, 0b1010][ch as usize % 8],
    }
}

pub fn play(s: &mut Screen, cells: &[Option<Cell>]) {
    let (w, h) = (s.w as usize, s.h as usize);
    let (pw, ph) = (w * 2, h * 2);
    let mut on = vec![false; pw * ph];
    let mut col = vec![Rgb::default(); pw * ph];
    for (i, c) in cells.iter().enumerate().take(w * h) {
        let Some(c) = c else { continue };
        let m = seed(c.ch);
        let (x, y) = ((i % w) * 2, (i / w) * 2);
        for (k, (dx, dy)) in [(0, 0), (1, 0), (0, 1), (1, 1)].into_iter().enumerate() {
            if m & (1 << k) != 0 {
                on[(y + dy) * pw + x + dx] = true;
                col[(y + dy) * pw + x + dx] = c.st.fg.unwrap_or(s.theme.fg);
            }
        }
    }
    let (bg, gens) = (s.theme.bg, 24);
    let mut was: Vec<Option<Cell>> = vec![None; w * h];
    for g in 0..=gens {
        if s.hurry {
            break;
        }
        // Brightest to start, going out as it goes on.
        let a = 1.0 - (g as f64 / gens as f64).powi(2);
        for r in 0..h - 1 {
            for c in 0..w {
                let px = |dx: usize, dy: usize| {
                    let i = (2 * r + dy) * pw + 2 * c + dx;
                    if on[i] { bg.mix(col[i], a) } else { bg }
                };
                let cell = quad([px(0, 0), px(1, 0), px(0, 1), px(1, 1)]);
                // Only what changed goes out.
                if was[r * w + c] != Some(cell) {
                    was[r * w + c] = Some(cell);
                    s.put(r as i32 + 1, c as i32 + 1, &[cell]);
                }
            }
        }
        // The first frame held a moment, the letters as pixels, then on.
        s.tick(if g == 0 { 0.18 } else { 0.055 });
        let mut next = vec![false; pw * ph];
        let mut born = col.clone();
        for y in 0..ph {
            for x in 0..pw {
                let mut n = 0;
                let mut from = None;
                for (dx, dy) in [(-1, -1), (0, -1), (1, -1), (-1, 0), (1, 0), (-1, 1), (0, 1), (1, 1)] {
                    let (nx, ny) = (x as i32 + dx, y as i32 + dy);
                    if nx >= 0 && ny >= 0 && (nx as usize) < pw && (ny as usize) < ph && on[ny as usize * pw + nx as usize] {
                        n += 1;
                        from = Some(ny as usize * pw + nx as usize);
                    }
                }
                let i = y * pw + x;
                next[i] = matches!((on[i], n), (true, 2 | 3) | (false, 3));
                if next[i] && !on[i] {
                    born[i] = col[from.unwrap()];
                }
            }
        }
        (on, col) = (next, born);
    }
    s.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_seed_the_same_each_time() {
        assert_eq!(seed('█'), 15);
        assert_eq!(seed(' '), 0);
        assert_eq!(seed('a'), seed('a'));
        assert!(seed('x').count_ones() >= 2);
    }
}
