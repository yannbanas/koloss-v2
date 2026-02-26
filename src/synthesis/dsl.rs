use serde::{Serialize, Deserialize};

pub type Grid = Vec<Vec<u8>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Prim {
    Identity,
    RotateCW,
    RotateCCW,
    Rotate180,
    FlipH,
    FlipV,
    Transpose,
    FillColor(u8),
    ReplaceColor(u8, u8),
    Crop(usize, usize, usize, usize),
    Pad(usize, u8),
    Scale(usize),
    FilterColor(u8),
    GravityDown,
    GravityUp,
    GravityLeft,
    GravityRight,
    MostFrequentColor,
    BorderFill(u8),
    Compose(Box<Prim>, Box<Prim>),
    Conditional(Box<Prim>, Box<Prim>, Box<Prim>),
}

impl Prim {
    pub fn apply(&self, grid: &Grid) -> Grid {
        match self {
            Prim::Identity => grid.clone(),
            Prim::RotateCW => rotate_cw(grid),
            Prim::RotateCCW => rotate_ccw(grid),
            Prim::Rotate180 => rotate_cw(&rotate_cw(grid)),
            Prim::FlipH => flip_h(grid),
            Prim::FlipV => flip_v(grid),
            Prim::Transpose => transpose(grid),
            Prim::FillColor(c) => fill_color(grid, *c),
            Prim::ReplaceColor(from, to) => replace_color(grid, *from, *to),
            Prim::Crop(r, c, h, w) => crop(grid, *r, *c, *h, *w),
            Prim::Pad(n, c) => pad(grid, *n, *c),
            Prim::Scale(s) => scale(grid, *s),
            Prim::FilterColor(c) => filter_color(grid, *c),
            Prim::GravityDown => gravity_down(grid),
            Prim::GravityUp => flip_v(&gravity_down(&flip_v(grid))),
            Prim::GravityLeft => transpose(&gravity_down(&transpose(grid))),
            Prim::GravityRight => transpose(&flip_v(&gravity_down(&flip_v(&transpose(grid))))),
            Prim::MostFrequentColor => most_frequent_fill(grid),
            Prim::BorderFill(c) => border_fill(grid, *c),
            Prim::Compose(a, b) => b.apply(&a.apply(grid)),
            Prim::Conditional(cond, then_p, else_p) => {
                let result = cond.apply(grid);
                if result != *grid { then_p.apply(grid) } else { else_p.apply(grid) }
            }
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Prim::Compose(a, b) => 1 + a.size() + b.size(),
            Prim::Conditional(a, b, c) => 1 + a.size() + b.size() + c.size(),
            _ => 1,
        }
    }

    pub fn all_primitives() -> Vec<Prim> {
        let mut prims = vec![
            Prim::Identity, Prim::RotateCW, Prim::RotateCCW, Prim::Rotate180,
            Prim::FlipH, Prim::FlipV, Prim::Transpose,
            Prim::GravityDown, Prim::GravityUp, Prim::GravityLeft, Prim::GravityRight,
        ];
        for c in 0..=9 {
            prims.push(Prim::FillColor(c));
            prims.push(Prim::FilterColor(c));
            prims.push(Prim::BorderFill(c));
            for c2 in 0..=9 {
                if c != c2 {
                    prims.push(Prim::ReplaceColor(c, c2));
                }
            }
        }
        for s in 2..=4 {
            prims.push(Prim::Scale(s));
        }
        prims
    }
}

fn rotate_cw(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    (0..cols).map(|c| (0..rows).rev().map(|r| g[r][c]).collect()).collect()
}

fn rotate_ccw(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    (0..cols).rev().map(|c| (0..rows).map(|r| g[r][c]).collect()).collect()
}

fn flip_h(g: &Grid) -> Grid {
    g.iter().map(|row| row.iter().rev().cloned().collect()).collect()
}

fn flip_v(g: &Grid) -> Grid {
    g.iter().rev().cloned().collect()
}

fn transpose(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let cols = g[0].len();
    (0..cols).map(|c| g.iter().map(|row| row[c]).collect()).collect()
}

fn fill_color(g: &Grid, color: u8) -> Grid {
    g.iter().map(|row| row.iter().map(|&c| if c != 0 { color } else { 0 }).collect()).collect()
}

fn replace_color(g: &Grid, from: u8, to: u8) -> Grid {
    g.iter().map(|row| row.iter().map(|&c| if c == from { to } else { c }).collect()).collect()
}

fn crop(g: &Grid, r: usize, c: usize, h: usize, w: usize) -> Grid {
    g.iter().skip(r).take(h).map(|row| row.iter().skip(c).take(w).cloned().collect()).collect()
}

fn pad(g: &Grid, n: usize, color: u8) -> Grid {
    if g.is_empty() { return g.clone(); }
    let new_cols = g[0].len() + 2 * n;
    let mut result = Vec::new();
    for _ in 0..n {
        result.push(vec![color; new_cols]);
    }
    for row in g {
        let mut new_row = vec![color; n];
        new_row.extend(row);
        new_row.extend(vec![color; n]);
        result.push(new_row);
    }
    for _ in 0..n {
        result.push(vec![color; new_cols]);
    }
    result
}

fn scale(g: &Grid, s: usize) -> Grid {
    let mut result = Vec::new();
    for row in g {
        let scaled_row: Vec<u8> = row.iter().flat_map(|&c| std::iter::repeat(c).take(s)).collect();
        for _ in 0..s {
            result.push(scaled_row.clone());
        }
    }
    result
}

fn filter_color(g: &Grid, color: u8) -> Grid {
    g.iter().map(|row| row.iter().map(|&c| if c == color { c } else { 0 }).collect()).collect()
}

fn gravity_down(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = vec![vec![0u8; cols]; rows];
    for c in 0..cols {
        let non_zero: Vec<u8> = (0..rows).filter_map(|r| {
            if g[r][c] != 0 { Some(g[r][c]) } else { None }
        }).collect();
        let offset = rows - non_zero.len();
        for (i, &val) in non_zero.iter().enumerate() {
            result[offset + i][c] = val;
        }
    }
    result
}

fn most_frequent_fill(g: &Grid) -> Grid {
    let mut counts = [0u32; 10];
    for row in g {
        for &c in row {
            if (c as usize) < 10 { counts[c as usize] += 1; }
        }
    }
    counts[0] = 0;
    let mfc = counts.iter().enumerate().max_by_key(|(_, &c)| c).map(|(i, _)| i as u8).unwrap_or(0);
    fill_color(g, mfc)
}

fn border_fill(g: &Grid, color: u8) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = g.clone();
    for c in 0..cols { result[0][c] = color; result[rows - 1][c] = color; }
    for r in 0..rows { result[r][0] = color; result[r][cols - 1] = color; }
    result
}
