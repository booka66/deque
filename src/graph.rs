//! ```graph: boxes and arrows laid out from lines like `a -> b -> c`, for a
//! drawn slide without placing anything by hand. In ranks, left to right
//! (or, ```graph down, top to bottom): a box goes one rank past the
//! furthest box with an arrow to it. An arrow across more than one rank
//! goes through a place kept for it in each it crosses, so it never runs
//! through a box; an arrow back (in a loop) goes round outside everything,
//! under the graph or to its right.
//!
//! It's worked out along the ranks (x) and across them (y), then turned
//! the right way: left to right, x is the column; top to bottom, the row.

/// Where a box goes, from the top left of the graph: row, column, width.
/// Every box is three rows tall.
pub type Place = (i32, i32, i32);

/// An arrow to lay out: its ends, and its label's width, 0 for none.
pub struct Edge {
    pub from: usize,
    pub to: usize,
    pub dotted: bool,
    pub label: i32,
}

/// The graph laid out: each box's place, each arrow's points (straight
/// across or down between them, its head at the last), where each arrow's
/// label starts (row, column), and the graph's size.
pub struct Layout {
    pub boxes: Vec<Place>,
    pub arrows: Vec<Vec<(i32, i32)>>,
    pub labels: Vec<Option<(i32, i32)>>,
    pub w: i32,
    pub h: i32,
}

const BOX_H: i32 = 3;

/// What's in a rank: a box, or a place an arrow passes through.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Slot {
    Box(usize),
    Pass(usize, usize),
}

/// `widths[k]` box k's width. An edge from a box to itself is left out.
pub fn layout(widths: &[i32], edges: &[Edge], down: bool) -> Layout {
    let n = widths.len();
    // Down a screen, boxes an odd width, so any two have the same middle
    // column and an arrow straight from one to the next doesn't jog.
    let widths: Vec<i32> = widths.iter().map(|&w| if down { w | 1 } else { w }).collect();
    // Sizes along the ranks and across them, and the room between: the
    // spaces between boxes side by side, and between lanes in a gap (a
    // column apart across a screen, a row apart down it).
    let len = |k: usize| if down { BOX_H } else { widths[k] };
    let span = |k: usize| if down { widths[k] } else { BOX_H };
    let (sep, step) = if down { (2, 1) } else { (1, 2) };
    let pairs: Vec<(usize, usize)> = edges.iter().map(|e| (e.from, e.to)).collect();
    // Arrows back, in a loop, found by walking forward from each box in
    // turn: left out of the ranks, and routed outside everything.
    let back = backs(n, &pairs);
    let fwd_i: Vec<usize> = (0..edges.len()).filter(|&e| !back[e] && edges[e].from != edges[e].to).collect();
    let loop_i: Vec<usize> = (0..edges.len()).filter(|&e| back[e] && edges[e].from != edges[e].to).collect();
    let fwd: Vec<(usize, usize)> = fwd_i.iter().map(|&e| pairs[e]).collect();
    let mut rank = vec![0usize; n];
    for _ in 0..n {
        for &(a, b) in &fwd {
            rank[b] = rank[b].max(rank[a] + 1);
        }
    }
    let ranks = rank.iter().max().map_or(0, |r| r + 1);
    let mut slots: Vec<Vec<Slot>> = vec![vec![]; ranks];
    for (k, &r) in rank.iter().enumerate() {
        slots[r].push(Slot::Box(k));
    }
    for (e, &(a, b)) in fwd.iter().enumerate() {
        for r in rank[a] + 1..rank[b] {
            slots[r].push(Slot::Pass(e, r));
        }
    }
    // Each rank ordered by where what leads into it sits in the rank
    // before: fewer arrows crossing.
    for c in 1..ranks {
        let before = slots[c - 1].clone();
        let at = |s: &Slot| before.iter().position(|x| x == s).map(|i| i as f64);
        let into = |s: &Slot| -> Option<f64> {
            let ins: Vec<f64> = match *s {
                Slot::Box(k) => fwd
                    .iter()
                    .enumerate()
                    .filter(|(_, e)| e.1 == k)
                    .filter_map(|(e, &(a, _))| if rank[a] + 1 == c { at(&Slot::Box(a)) } else { at(&Slot::Pass(e, c - 1)) })
                    .collect(),
                Slot::Pass(e, r) => {
                    let a = fwd[e].0;
                    if rank[a] + 1 == r { at(&Slot::Box(a)).into_iter().collect() } else { at(&Slot::Pass(e, r - 1)).into_iter().collect() }
                }
            };
            (!ins.is_empty()).then(|| ins.iter().sum::<f64>() / ins.len() as f64)
        };
        let mut keyed: Vec<(f64, usize, Slot)> = slots[c].iter().enumerate().map(|(i, s)| (into(s).unwrap_or(i as f64), i, *s)).collect();
        keyed.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        slots[c] = keyed.into_iter().map(|k| k.2).collect();
    }
    let across = |s: &Slot| if let Slot::Box(k) = s { span(*k) } else { 1 };
    let extent = |col: &[Slot]| col.iter().map(across).sum::<i32>() + sep * (col.len() as i32 - 1).max(0);
    let breadth = slots.iter().map(|c| extent(c)).max().unwrap_or(0);
    // Each slot's first place across, each rank centered on the broadest.
    let mut top = std::collections::HashMap::new();
    for col in &slots {
        let mut y = (breadth - extent(col)) / 2;
        for s in col {
            top.insert(*s, y);
            y += across(s) + sep;
        }
    }
    let mid = |s: &Slot| top[s] + across(s) / 2;
    let cw: Vec<i32> = slots.iter().map(|col| col.iter().map(|s| if let Slot::Box(k) = s { len(*k) } else { 1 }).max().unwrap_or(1)).collect();
    // Each arrow's stops: out of its first box, through its passes, into
    // its last.
    let stops: Vec<Vec<Slot>> = fwd
        .iter()
        .enumerate()
        .map(|(e, &(a, b))| std::iter::once(Slot::Box(a)).chain((rank[a] + 1..rank[b]).map(|r| Slot::Pass(e, r))).chain([Slot::Box(b)]).collect())
        .collect();
    // The gaps: before the first rank, between each two, after the last.
    // In each, a lane of its own for each box an arrow turns out of, so
    // arrows from different boxes don't run together (those from one box,
    // drawn the same, share it, branching); then one for each arrow back
    // that turns there.
    let mut turns: Vec<Vec<(i32, Slot, bool)>> = vec![vec![]; ranks + 1];
    for (st, &e) in stops.iter().zip(&fwd_i) {
        let (a, d) = (edges[e].from, edges[e].dotted);
        for (g, w) in st.windows(2).enumerate() {
            let gap = rank[a] + g + 1;
            if mid(&w[0]) != mid(&w[1]) && !turns[gap].iter().any(|t| t.1 == w[0] && t.2 == d) {
                turns[gap].push((mid(&w[0]), w[0], d));
            }
        }
    }
    for t in &mut turns {
        t.sort_by_key(|t| t.0);
    }
    let mut downs = vec![0i32; ranks + 1];
    for &e in &loop_i {
        downs[rank[edges[e].from] + 1] += 1;
        downs[rank[edges[e].to]] += 1;
    }
    // Across a screen, a labelled arrow's gap has room for its label.
    let mut room = vec![0i32; ranks + 1];
    if !down {
        for &e in &fwd_i {
            let g = rank[edges[e].from] + 1;
            room[g] = room[g].max(if edges[e].label > 0 { edges[e].label + 2 } else { 0 });
        }
    }
    let gap_w: Vec<i32> = (0..=ranks)
        .map(|g| {
            let k = turns[g].len() as i32 + downs[g];
            if g == 0 || g == ranks {
                if k > 0 { 2 + step * k } else { 0 }
            } else {
                (2 + step * (k + 1)).max(if down { 3 } else { 6 }) + room[g]
            }
        })
        .collect();
    let mut x = vec![gap_w[0]; ranks];
    for c in 1..ranks {
        x[c] = x[c - 1] + cw[c - 1] + gap_w[c];
    }
    let length = x.last().map_or(0, |l| l + cw[ranks - 1] + gap_w[ranks]);
    // Where gap g starts, and its k-th lane.
    let gap_at = |g: usize| if g == 0 { 0 } else { x[g - 1] + cw[g - 1] };
    let lane = |g: usize, k: i32| if g == 0 { step * k } else { gap_at(g) + 2 + step * k - if g == ranks { 1 } else { 0 } };
    // A box centered in its rank.
    let first = |s: &Slot, c: usize| match s {
        Slot::Box(k) => x[c] + (cw[c] - len(*k)) / 2,
        Slot::Pass(..) => x[c],
    };
    let last = |s: &Slot, c: usize| match s {
        Slot::Box(k) => first(s, c) + len(*k) - 1,
        Slot::Pass(..) => x[c] + cw[c] - 1,
    };
    let mut routes: Vec<(usize, Vec<(i32, i32)>)> = vec![];
    // Each label's place, (across, along), still to be turned.
    let mut labels: Vec<Option<(i32, i32)>> = vec![None; edges.len()];
    for (st, &e) in stops.iter().zip(&fwd_i) {
        let c0 = rank[edges[e].from];
        let mut pts = vec![(mid(&st[0]), last(&st[0], c0) + 1)];
        for (g, w) in st.windows(2).enumerate() {
            let (c, y) = (c0 + g + 1, mid(&w[1]));
            let here = pts.last().unwrap().0;
            if here != y {
                let k = turns[c].iter().position(|t| t.1 == w[0] && t.2 == edges[e].dotted).unwrap_or(0) as i32;
                pts.push((here, lane(c, k)));
                pts.push((y, lane(c, k)));
            }
            let end = if matches!(w[1], Slot::Box(_)) { first(&w[1], c) - 1 } else { last(&w[1], c) };
            // The label on the first stretch into the rank after its box,
            // past every lane in the gap, so none crosses it: in it,
            // across a screen; beside it, down one.
            if g == 0 && edges[e].label > 0 {
                let from = pts.last().unwrap().1;
                let (a, b) = ((from + 1).max(lane(c, turns[c].len() as i32 + downs[c])), first(&w[1], c) - 2);
                labels[e] = Some(if down { (y + 2, (a + b) / 2) } else { (y, a + ((b - a + 1 - edges[e].label) / 2).max(0)) });
            }
            pts.push((y, end));
        }
        routes.push((e, pts));
    }
    // Arrows back: out of the end of their box, round a lane of their own
    // outside everything, and into the start of the box they go to.
    let mut used = vec![0i32; ranks + 1];
    let mut outer = breadth;
    for (j, &e) in loop_i.iter().enumerate() {
        let (a, b) = (edges[e].from, edges[e].to);
        let (ga, gb) = (rank[a] + 1, rank[b]);
        let xr = lane(ga, turns[ga].len() as i32 + used[ga]);
        used[ga] += 1;
        let xl = lane(gb, turns[gb].len() as i32 + used[gb]);
        used[gb] += 1;
        let (sa, sb) = (Slot::Box(a), Slot::Box(b));
        let out = breadth + 1 + sep * j as i32;
        outer = out + 1;
        if edges[e].label > 0 {
            labels[e] = Some(if down { (out + 2, (xl + xr) / 2) } else { (out, xl + 1 + ((xr - xl - 1 - edges[e].label) / 2).max(0)) });
        }
        routes.push((e, vec![(mid(&sa), last(&sa, rank[a]) + 1), (mid(&sa), xr), (out, xr), (out, xl), (mid(&sb), xl), (mid(&sb), first(&sb, rank[b]) - 1)]));
    }
    // Straight on through a point: it's not a corner.
    let mut arrows = vec![vec![]; edges.len()];
    for (e, pts) in routes {
        let mut p: Vec<(i32, i32)> = vec![];
        for q in pts {
            if p.last() == Some(&q) {
                continue;
            }
            if p.len() >= 2 {
                let (a, b) = (p[p.len() - 2], p[p.len() - 1]);
                if (a.0 == b.0 && b.0 == q.0) || (a.1 == b.1 && b.1 == q.1) {
                    p.pop();
                }
            }
            p.push(q);
        }
        arrows[e] = p;
    }
    // Turned the right way: (across, along) to (row, column).
    let turn = |(y, x): (i32, i32)| if down { (x, y) } else { (y, x) };
    let boxes = (0..n).map(|k| {
        let (r, c) = turn((top[&Slot::Box(k)], first(&Slot::Box(k), rank[k])));
        (r, c, widths[k])
    });
    let labels: Vec<Option<(i32, i32)>> = labels.into_iter().map(|l| l.map(turn)).collect();
    let (mut w, h) = if down { (outer, length) } else { (length, outer) };
    // Down a screen, labels go beside their arrows: room for them.
    if down {
        for (l, e) in labels.iter().zip(edges) {
            if let Some((_, c)) = l {
                w = w.max(c + e.label);
            }
        }
    }
    Layout { boxes: boxes.collect(), arrows: arrows.into_iter().map(|a| a.into_iter().map(turn).collect()).collect(), labels, w, h }
}

/// Which edges go back, closing a loop.
fn backs(n: usize, edges: &[(usize, usize)]) -> Vec<bool> {
    // 0 not seen, 1 on the way, 2 done.
    let mut state = vec![0u8; n];
    let mut back = vec![false; edges.len()];
    fn walk(v: usize, edges: &[(usize, usize)], state: &mut [u8], back: &mut [bool]) {
        state[v] = 1;
        for (e, &(a, b)) in edges.iter().enumerate() {
            if a != v || a == b {
                continue;
            }
            match state[b] {
                0 => walk(b, edges, state, back),
                1 => back[e] = true,
                _ => {}
            }
        }
        state[v] = 2;
    }
    for v in 0..n {
        if state[v] == 0 {
            walk(v, edges, &mut state, &mut back);
        }
    }
    back
}

const UP: u8 = 1;
const DOWN: u8 = 2;
const LEFT: u8 = 4;
const RIGHT: u8 = 8;

/// The ways a line character goes out of its cell; None for a head.
fn ways(ch: char) -> Option<u8> {
    Some(match ch {
        '─' | '┄' => LEFT | RIGHT,
        '│' | '┊' => UP | DOWN,
        '╭' => RIGHT | DOWN,
        '╮' => LEFT | DOWN,
        '╰' => UP | RIGHT,
        '╯' => UP | LEFT,
        _ => return None,
    })
}

fn glyph(ways: u8, dotted: bool) -> char {
    match ways {
        w if w == LEFT | RIGHT => if dotted { '┄' } else { '─' },
        w if w == UP | DOWN => if dotted { '┊' } else { '│' },
        w if w == RIGHT | DOWN => '╭',
        w if w == LEFT | DOWN => '╮',
        w if w == UP | RIGHT => '╰',
        w if w == UP | LEFT => '╯',
        w if w == UP | DOWN | RIGHT => '├',
        w if w == UP | DOWN | LEFT => '┤',
        w if w == LEFT | RIGHT | DOWN => '┬',
        w if w == LEFT | RIGHT | UP => '┴',
        _ => '┼',
    }
}

/// Where arrows run together, branch or join, the cell as all of them
/// make it (├ ┬ ┼ …), solid where any of them is: each arrow's points,
/// whether it's dotted, and whether it's `fresh`, only now drawn; only
/// cells a fresh one goes through. Where they only cross, or one ends in
/// its head, it's left as drawn.
pub fn joins(arrows: &[(Vec<(i32, i32)>, bool, bool)]) -> Vec<(i32, i32, char)> {
    let mut at: std::collections::BTreeMap<(i32, i32), Vec<(Option<u8>, bool, bool)>> = Default::default();
    for (pts, dotted, fresh) in arrows {
        if pts.len() < 2 {
            continue;
        }
        for (r, c, ch, _) in crate::render::arrow(pts, *dotted) {
            at.entry((r, c)).or_default().push((ways(ch), *dotted, *fresh));
        }
    }
    at.into_iter()
        .filter(|(_, v)| v.len() > 1 && v.iter().any(|x| x.2) && v.iter().all(|x| x.0.is_some()))
        .filter_map(|((r, c), v)| {
            let w: Vec<u8> = v.iter().map(|x| x.0.unwrap()).collect();
            let shared = w.iter().enumerate().any(|(i, a)| w[i + 1..].iter().any(|b| a & b != 0));
            shared.then(|| (r, c, glyph(w.iter().fold(0, |a, b| a | b), v.iter().all(|x| x.1))))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edges(e: &[(usize, usize)]) -> Vec<Edge> {
        e.iter().map(|&(from, to)| Edge { from, to, dotted: false, label: 0 }).collect()
    }

    /// Every arrow's segments straight, and no arrow cell inside a box.
    fn sound(l: &Layout, down: bool) {
        for a in &l.arrows {
            assert!(a.len() >= 2, "{a:?}");
            for w in a.windows(2) {
                assert!(w[0] != w[1] && (w[0].0 == w[1].0 || w[0].1 == w[1].1), "{a:?}");
                let (dr, dc) = ((w[1].0 - w[0].0).signum(), (w[1].1 - w[0].1).signum());
                let mut at = w[0];
                loop {
                    for &(r, c, bw) in &l.boxes {
                        assert!(!(at.0 >= r && at.0 < r + BOX_H && at.1 >= c && at.1 < c + bw), "{a:?} through box at {r},{c} (down: {down})");
                    }
                    if at == w[1] {
                        break;
                    }
                    at = (at.0 + dr, at.1 + dc);
                }
            }
        }
    }

    #[test]
    fn a_chain_is_a_row_or_a_column() {
        let l = layout(&[5, 5, 5], &edges(&[(0, 1), (1, 2)]), false);
        assert_eq!(l.boxes.iter().map(|b| b.0).collect::<Vec<_>>(), [0, 0, 0]);
        assert!(l.boxes[0].1 < l.boxes[1].1 && l.boxes[1].1 < l.boxes[2].1);
        // Straight from the first box's right to beside the second.
        assert_eq!(l.arrows[0], [(1, 5), (1, l.boxes[1].1 - 1)]);
        assert_eq!(l.h, 3);
        sound(&l, false);
        let l = layout(&[5, 7, 5], &edges(&[(0, 1), (1, 2)]), true);
        assert_eq!(l.boxes.iter().map(|b| b.1 + b.2 / 2).collect::<Vec<_>>(), [3, 3, 3]);
        assert!(l.boxes[0].0 < l.boxes[1].0 && l.boxes[1].0 < l.boxes[2].0);
        // Straight down from under the first to over the second.
        assert_eq!(l.arrows[0], [(3, 3), (l.boxes[1].0 - 1, 3)]);
        sound(&l, true);
    }

    #[test]
    fn long_arrows_go_round_and_loops_come_back() {
        for down in [false, true] {
            // a -> b -> c, a -> c across b's rank, and c -> a back.
            let l = layout(&[5, 5, 5], &edges(&[(0, 1), (1, 2), (0, 2), (2, 0)]), down);
            sound(&l, down);
            // The loop goes outside everything, and into a from its start.
            let back = l.arrows.last().unwrap();
            let a = l.boxes[0];
            let want = if down { (a.0 - 1, a.1 + 2) } else { (a.0 + 1, a.1 - 1) };
            assert_eq!(*back.last().unwrap(), want, "down: {down}");
        }
    }

    #[test]
    fn fans_out_and_in() {
        let l = layout(&[6, 4, 4, 4, 6], &edges(&[(0, 1), (0, 2), (0, 3), (1, 4), (2, 4), (3, 4)]), false);
        sound(&l, false);
        let rows: Vec<i32> = l.boxes.iter().map(|b| b.0).collect();
        assert_eq!((rows[1], rows[2], rows[3]), (0, 4, 8));
        assert_eq!(l.h, 11);
        // Where they branch up, on and down, a junction all four ways;
        // where one joins from above, and one from below, a T.
        let all: Vec<(Vec<(i32, i32)>, bool, bool)> = l.arrows.iter().map(|a| (a.clone(), false, true)).collect();
        let j: Vec<char> = joins(&all).iter().map(|x| x.2).collect();
        assert!(j.contains(&'┼') && j.contains(&'┴') && j.contains(&'┬'), "{j:?}");
    }

    #[test]
    fn labels_get_room() {
        let mut e = edges(&[(0, 1)]);
        e[0].label = 10;
        let l = layout(&[5, 5], &e, false);
        let (r, c) = l.labels[0].unwrap();
        assert_eq!(r, 1);
        assert!(c > 5 && c + 10 < l.boxes[1].1, "{c} {:?}", l.boxes);
        let l = layout(&[5, 5], &e, true);
        let (r, c) = l.labels[0].unwrap();
        assert!(r > 2 && r < l.boxes[1].0 && c == 4 && l.w >= 14, "{r},{c} {}", l.w);
    }

    #[test]
    fn solid_wins_and_crossings_stay() {
        // A solid and a dotted arrow down the same stretch: solid.
        let j = joins(&[(vec![(0, 0), (0, 5)], false, false), (vec![(0, 0), (0, 3), (3, 3)], true, true)]);
        assert!(j.contains(&(0, 1, '─')) && j.contains(&(0, 3, '┬')), "{j:?}");
        // Only crossing: left alone.
        assert!(joins(&[(vec![(2, 0), (2, 6)], false, true), (vec![(0, 3), (5, 3)], false, true)]).is_empty());
    }
}
