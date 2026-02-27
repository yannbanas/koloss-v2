use serde::{Serialize, Deserialize};

pub type Grid = Vec<Vec<u8>>;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Object {
    pub cells: Vec<(usize, usize)>,
    pub color: u8,
    pub min_r: usize,
    pub min_c: usize,
    pub max_r: usize,
    pub max_c: usize,
}

impl Object {
    pub fn from_cells(cells: Vec<(usize, usize)>, color: u8) -> Self {
        let min_r = cells.iter().map(|&(r, _)| r).min().unwrap_or(0);
        let min_c = cells.iter().map(|&(_, c)| c).min().unwrap_or(0);
        let max_r = cells.iter().map(|&(r, _)| r).max().unwrap_or(0);
        let max_c = cells.iter().map(|&(_, c)| c).max().unwrap_or(0);
        Self { cells, color, min_r, min_c, max_r, max_c }
    }

    pub fn width(&self) -> usize { self.max_c - self.min_c + 1 }
    pub fn height(&self) -> usize { self.max_r - self.min_r + 1 }
    pub fn area(&self) -> usize { self.cells.len() }

    pub fn to_grid(&self) -> Grid {
        let h = self.height();
        let w = self.width();
        let mut g = vec![vec![0u8; w]; h];
        for &(r, c) in &self.cells {
            g[r - self.min_r][c - self.min_c] = self.color;
        }
        g
    }

    pub fn center(&self) -> (usize, usize) {
        ((self.min_r + self.max_r) / 2, (self.min_c + self.max_c) / 2)
    }

    pub fn bounding_box(&self) -> (usize, usize, usize, usize) {
        (self.min_r, self.min_c, self.height(), self.width())
    }
}

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
    FloodFill(usize, usize, u8),
    ExtractObject(usize),
    Overlay,
    MirrorH,
    MirrorV,
    RepeatH(usize),
    RepeatV(usize),
    Invert,
    SortRowsByColor,
    SortColsByColor,
    RemoveColor(u8),
    KeepLargestObject,
    KeepSmallestObject,
    OutlineObjects(u8),
    FillInsideObjects(u8),
    // New: translate, crop-to-bbox, line extension, diagonal
    Translate(i32, i32),         // shift non-zero cells by (dr, dc)
    CropToBBox,                  // tight crop around non-zero cells
    ExtendHLines,                // extend each non-zero pixel into full row
    ExtendVLines,                // extend each non-zero pixel into full column
    ExtendCross,                 // extend each non-zero pixel into full row + column
    DiagFillTL,                  // fill diagonal stripes top-left
    DiagFillTR,                  // fill diagonal stripes top-right
    FillEnclosed(u8),            // fill regions enclosed by a specific wall color
    UpscaleObjects(usize),       // upscale each object to fill its bounding box × factor
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
            Prim::FloodFill(r, c, color) => flood_fill(grid, *r, *c, *color),
            Prim::ExtractObject(idx) => extract_object(grid, *idx),
            Prim::Overlay => grid.clone(), // Overlay needs two grids, handled separately
            Prim::MirrorH => mirror_h(grid),
            Prim::MirrorV => mirror_v(grid),
            Prim::RepeatH(n) => repeat_h(grid, *n),
            Prim::RepeatV(n) => repeat_v(grid, *n),
            Prim::Invert => invert(grid),
            Prim::SortRowsByColor => sort_rows_by_color(grid),
            Prim::SortColsByColor => sort_cols_by_color(grid),
            Prim::RemoveColor(c) => replace_color(grid, *c, 0),
            Prim::KeepLargestObject => keep_largest_object(grid),
            Prim::KeepSmallestObject => keep_smallest_object(grid),
            Prim::OutlineObjects(c) => outline_objects(grid, *c),
            Prim::FillInsideObjects(c) => fill_inside_objects(grid, *c),
            Prim::Translate(dr, dc) => translate(grid, *dr, *dc),
            Prim::CropToBBox => crop_to_bbox(grid),
            Prim::ExtendHLines => extend_h_lines(grid),
            Prim::ExtendVLines => extend_v_lines(grid),
            Prim::ExtendCross => extend_cross(grid),
            Prim::DiagFillTL => diag_fill_tl(grid),
            Prim::DiagFillTR => diag_fill_tr(grid),
            Prim::FillEnclosed(wall) => fill_enclosed(grid, *wall),
            Prim::UpscaleObjects(f) => upscale_objects(grid, *f),
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
            Prim::MirrorH, Prim::MirrorV,
            Prim::Invert, Prim::SortRowsByColor, Prim::SortColsByColor,
            Prim::KeepLargestObject, Prim::KeepSmallestObject,
            Prim::CropToBBox, Prim::ExtendHLines, Prim::ExtendVLines, Prim::ExtendCross,
            Prim::DiagFillTL, Prim::DiagFillTR,
        ];
        for c in 0..=9 {
            prims.push(Prim::FillColor(c));
            prims.push(Prim::FilterColor(c));
            prims.push(Prim::BorderFill(c));
            prims.push(Prim::RemoveColor(c));
            prims.push(Prim::OutlineObjects(c));
            prims.push(Prim::FillInsideObjects(c));
            prims.push(Prim::FillEnclosed(c));
            for c2 in 0..=9 {
                if c != c2 {
                    prims.push(Prim::ReplaceColor(c, c2));
                }
            }
        }
        for s in 2..=4 {
            prims.push(Prim::Scale(s));
            prims.push(Prim::RepeatH(s));
            prims.push(Prim::RepeatV(s));
            prims.push(Prim::UpscaleObjects(s));
        }
        // Translation offsets: common shifts ±1..3
        for d in [-3i32, -2, -1, 1, 2, 3] {
            prims.push(Prim::Translate(d, 0));
            prims.push(Prim::Translate(0, d));
        }
        prims
    }
}

// --- Grid analysis functions (public for use by other modules) ---

pub fn connected_components(grid: &Grid, ignore_bg: bool) -> Vec<Object> {
    if grid.is_empty() { return Vec::new(); }
    let rows = grid.len();
    let cols = grid[0].len();
    let mut visited = vec![vec![false; cols]; rows];
    let mut objects = Vec::new();

    for r in 0..rows {
        for c in 0..cols {
            if visited[r][c] { continue; }
            let color = grid[r][c];
            if ignore_bg && color == 0 { continue; }

            let mut cells = Vec::new();
            let mut stack = vec![(r, c)];
            visited[r][c] = true;

            while let Some((cr, cc)) = stack.pop() {
                cells.push((cr, cc));
                for (dr, dc) in &[(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
                    let nr = cr as i32 + dr;
                    let nc = cc as i32 + dc;
                    if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                        let (nr, nc) = (nr as usize, nc as usize);
                        if !visited[nr][nc] && grid[nr][nc] == color {
                            visited[nr][nc] = true;
                            stack.push((nr, nc));
                        }
                    }
                }
            }
            objects.push(Object::from_cells(cells, color));
        }
    }
    objects
}

pub fn connected_components_8(grid: &Grid, ignore_bg: bool) -> Vec<Object> {
    if grid.is_empty() { return Vec::new(); }
    let rows = grid.len();
    let cols = grid[0].len();
    let mut visited = vec![vec![false; cols]; rows];
    let mut objects = Vec::new();

    for r in 0..rows {
        for c in 0..cols {
            if visited[r][c] { continue; }
            let color = grid[r][c];
            if ignore_bg && color == 0 { continue; }

            let mut cells = Vec::new();
            let mut stack = vec![(r, c)];
            visited[r][c] = true;

            while let Some((cr, cc)) = stack.pop() {
                cells.push((cr, cc));
                for dr in -1i32..=1 {
                    for dc in -1i32..=1 {
                        if dr == 0 && dc == 0 { continue; }
                        let nr = cr as i32 + dr;
                        let nc = cc as i32 + dc;
                        if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                            let (nr, nc) = (nr as usize, nc as usize);
                            if !visited[nr][nc] && grid[nr][nc] == color {
                                visited[nr][nc] = true;
                                stack.push((nr, nc));
                            }
                        }
                    }
                }
            }
            objects.push(Object::from_cells(cells, color));
        }
    }
    objects
}

pub fn count_objects(grid: &Grid) -> usize {
    connected_components(grid, true).len()
}

pub fn unique_colors(grid: &Grid) -> Vec<u8> {
    let mut seen = [false; 256];
    let mut result = Vec::new();
    for row in grid {
        for &c in row {
            if !seen[c as usize] {
                seen[c as usize] = true;
                result.push(c);
            }
        }
    }
    result
}

pub fn grid_dimensions(grid: &Grid) -> (usize, usize) {
    if grid.is_empty() { (0, 0) } else { (grid.len(), grid[0].len()) }
}

pub fn overlay_grids(base: &Grid, top: &Grid) -> Grid {
    if base.is_empty() { return top.clone(); }
    let rows = base.len().max(top.len());
    let cols = base[0].len().max(if top.is_empty() { 0 } else { top[0].len() });
    let mut result = vec![vec![0u8; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            let base_val = if r < base.len() && c < base[0].len() { base[r][c] } else { 0 };
            let top_val = if r < top.len() && c < top[0].len() { top[r][c] } else { 0 };
            result[r][c] = if top_val != 0 { top_val } else { base_val };
        }
    }
    result
}

pub fn is_symmetric_h(grid: &Grid) -> bool {
    grid.iter().all(|row| {
        let n = row.len();
        (0..n / 2).all(|i| row[i] == row[n - 1 - i])
    })
}

pub fn is_symmetric_v(grid: &Grid) -> bool {
    let n = grid.len();
    (0..n / 2).all(|i| grid[i] == grid[n - 1 - i])
}

pub fn is_symmetric_diag(grid: &Grid) -> bool {
    let (rows, cols) = grid_dimensions(grid);
    if rows != cols { return false; }
    (0..rows).all(|r| (0..cols).all(|c| grid[r][c] == grid[c][r]))
}

pub fn detect_period_h(grid: &Grid) -> Option<usize> {
    if grid.is_empty() { return None; }
    let cols = grid[0].len();
    for period in 1..=cols / 2 {
        if cols % period != 0 { continue; }
        let valid = grid.iter().all(|row| {
            (period..cols).all(|c| row[c] == row[c % period])
        });
        if valid { return Some(period); }
    }
    None
}

pub fn detect_period_v(grid: &Grid) -> Option<usize> {
    let rows = grid.len();
    for period in 1..=rows / 2 {
        if rows % period != 0 { continue; }
        let valid = (period..rows).all(|r| grid[r] == grid[r % period]);
        if valid { return Some(period); }
    }
    None
}

// Spatial reasoning queries
pub fn is_above(a: &Object, b: &Object) -> bool { a.max_r < b.min_r }
pub fn is_below(a: &Object, b: &Object) -> bool { a.min_r > b.max_r }
pub fn is_left_of(a: &Object, b: &Object) -> bool { a.max_c < b.min_c }
pub fn is_right_of(a: &Object, b: &Object) -> bool { a.min_c > b.max_c }

pub fn is_adjacent(a: &Object, b: &Object) -> bool {
    for &(ar, ac) in &a.cells {
        for &(br, bc) in &b.cells {
            let dr = (ar as i32 - br as i32).unsigned_abs();
            let dc = (ac as i32 - bc as i32).unsigned_abs();
            if (dr == 1 && dc == 0) || (dr == 0 && dc == 1) {
                return true;
            }
        }
    }
    false
}

pub fn is_inside(inner: &Object, outer: &Object) -> bool {
    inner.min_r > outer.min_r && inner.max_r < outer.max_r
        && inner.min_c > outer.min_c && inner.max_c < outer.max_c
}

pub fn objects_overlap(a: &Object, b: &Object) -> bool {
    for &(ar, ac) in &a.cells {
        for &(br, bc) in &b.cells {
            if ar == br && ac == bc { return true; }
        }
    }
    false
}

pub fn distance_between(a: &Object, b: &Object) -> f64 {
    let (ar, ac) = a.center();
    let (br, bc) = b.center();
    (((ar as f64 - br as f64).powi(2) + (ac as f64 - bc as f64).powi(2))).sqrt()
}

// --- Internal primitive implementations ---

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

fn flood_fill(g: &Grid, sr: usize, sc: usize, new_color: u8) -> Grid {
    if g.is_empty() || sr >= g.len() || sc >= g[0].len() { return g.clone(); }
    let old_color = g[sr][sc];
    if old_color == new_color { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = g.clone();
    let mut stack = vec![(sr, sc)];
    result[sr][sc] = new_color;

    while let Some((r, c)) = stack.pop() {
        for (dr, dc) in &[(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                let (nr, nc) = (nr as usize, nc as usize);
                if result[nr][nc] == old_color {
                    result[nr][nc] = new_color;
                    stack.push((nr, nc));
                }
            }
        }
    }
    result
}

fn extract_object(g: &Grid, idx: usize) -> Grid {
    let objects = connected_components(g, true);
    if idx >= objects.len() { return g.clone(); }
    let obj = &objects[idx];
    obj.to_grid()
}

fn mirror_h(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let cols = g[0].len();
    g.iter().map(|row| {
        let mut new_row = row.clone();
        new_row.extend(row.iter().rev());
        new_row.truncate(cols * 2);
        new_row
    }).collect()
}

fn mirror_v(g: &Grid) -> Grid {
    let mut result = g.clone();
    let reversed: Vec<Vec<u8>> = g.iter().rev().cloned().collect();
    result.extend(reversed);
    result
}

fn repeat_h(g: &Grid, n: usize) -> Grid {
    g.iter().map(|row| {
        let mut new_row = Vec::new();
        for _ in 0..n { new_row.extend(row.iter()); }
        new_row
    }).collect()
}

fn repeat_v(g: &Grid, n: usize) -> Grid {
    let mut result = Vec::new();
    for _ in 0..n { result.extend(g.iter().cloned()); }
    result
}

fn invert(g: &Grid) -> Grid {
    let max_color = g.iter().flat_map(|r| r.iter()).max().copied().unwrap_or(1);
    g.iter().map(|row| {
        row.iter().map(|&c| if c == 0 { max_color } else { 0 }).collect()
    }).collect()
}

fn sort_rows_by_color(g: &Grid) -> Grid {
    let mut result = g.clone();
    result.sort_by_key(|row| {
        row.iter().filter(|&&c| c != 0).next().copied().unwrap_or(255)
    });
    result
}

fn sort_cols_by_color(g: &Grid) -> Grid {
    transpose(&sort_rows_by_color(&transpose(g)))
}

fn keep_largest_object(g: &Grid) -> Grid {
    let objects = connected_components(g, true);
    let largest = objects.iter().max_by_key(|o| o.area());
    match largest {
        Some(obj) => {
            let (rows, cols) = grid_dimensions(g);
            let mut result = vec![vec![0u8; cols]; rows];
            for &(r, c) in &obj.cells {
                result[r][c] = obj.color;
            }
            result
        }
        None => g.clone(),
    }
}

fn keep_smallest_object(g: &Grid) -> Grid {
    let objects = connected_components(g, true);
    let smallest = objects.iter().min_by_key(|o| o.area());
    match smallest {
        Some(obj) => {
            let (rows, cols) = grid_dimensions(g);
            let mut result = vec![vec![0u8; cols]; rows];
            for &(r, c) in &obj.cells {
                result[r][c] = obj.color;
            }
            result
        }
        None => g.clone(),
    }
}

fn outline_objects(g: &Grid, outline_color: u8) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = g.clone();
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] != 0 {
                let on_border = [(0i32, 1i32), (0, -1), (1, 0), (-1, 0)].iter().any(|&(dr, dc)| {
                    let nr = r as i32 + dr;
                    let nc = c as i32 + dc;
                    nr < 0 || nr >= rows as i32 || nc < 0 || nc >= cols as i32
                        || g[nr as usize][nc as usize] == 0
                });
                if on_border { result[r][c] = outline_color; }
            }
        }
    }
    result
}

fn translate(g: &Grid, dr: i32, dc: i32) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = vec![vec![0u8; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] != 0 {
                let nr = r as i32 + dr;
                let nc = c as i32 + dc;
                if nr >= 0 && (nr as usize) < rows && nc >= 0 && (nc as usize) < cols {
                    result[nr as usize][nc as usize] = g[r][c];
                }
            }
        }
    }
    result
}

fn crop_to_bbox(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut min_r = rows;
    let mut max_r = 0;
    let mut min_c = cols;
    let mut max_c = 0;
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] != 0 {
                min_r = min_r.min(r);
                max_r = max_r.max(r);
                min_c = min_c.min(c);
                max_c = max_c.max(c);
            }
        }
    }
    if min_r > max_r { return vec![vec![0]]; }
    crop(g, min_r, min_c, max_r - min_r + 1, max_c - min_c + 1)
}

fn extend_h_lines(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = vec![vec![0u8; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] != 0 {
                for cc in 0..cols { result[r][cc] = g[r][c]; }
            }
        }
    }
    result
}

fn extend_v_lines(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = vec![vec![0u8; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] != 0 {
                for rr in 0..rows { result[rr][c] = g[r][c]; }
            }
        }
    }
    result
}

fn extend_cross(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = g.clone();
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] != 0 {
                for cc in 0..cols {
                    if result[r][cc] == 0 { result[r][cc] = g[r][c]; }
                }
                for rr in 0..rows {
                    if result[rr][c] == 0 { result[rr][c] = g[r][c]; }
                }
            }
        }
    }
    result
}

fn diag_fill_tl(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = g.clone();
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] != 0 {
                let color = g[r][c];
                let mut nr = r as i32 + 1;
                let mut nc = c as i32 + 1;
                while nr < rows as i32 && nc < cols as i32 {
                    if result[nr as usize][nc as usize] == 0 {
                        result[nr as usize][nc as usize] = color;
                    }
                    nr += 1; nc += 1;
                }
                let mut nr = r as i32 - 1;
                let mut nc = c as i32 - 1;
                while nr >= 0 && nc >= 0 {
                    if result[nr as usize][nc as usize] == 0 {
                        result[nr as usize][nc as usize] = color;
                    }
                    nr -= 1; nc -= 1;
                }
            }
        }
    }
    result
}

fn diag_fill_tr(g: &Grid) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = g.clone();
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] != 0 {
                let color = g[r][c];
                let mut nr = r as i32 + 1;
                let mut nc = c as i32 - 1;
                while nr < rows as i32 && nc >= 0 {
                    if result[nr as usize][nc as usize] == 0 {
                        result[nr as usize][nc as usize] = color;
                    }
                    nr += 1; nc -= 1;
                }
                let mut nr = r as i32 - 1;
                let mut nc = c as i32 + 1;
                while nr >= 0 && nc < cols as i32 {
                    if result[nr as usize][nc as usize] == 0 {
                        result[nr as usize][nc as usize] = color;
                    }
                    nr -= 1; nc += 1;
                }
            }
        }
    }
    result
}

fn fill_enclosed(g: &Grid, wall_color: u8) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = g.clone();
    let mut reachable = vec![vec![false; cols]; rows];
    let mut stack: Vec<(usize, usize)> = Vec::new();
    for r in 0..rows {
        for c in 0..cols {
            if (r == 0 || r == rows - 1 || c == 0 || c == cols - 1) && g[r][c] != wall_color {
                reachable[r][c] = true;
                stack.push((r, c));
            }
        }
    }
    while let Some((r, c)) = stack.pop() {
        for (dr, dc) in &[(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                let (nr, nc) = (nr as usize, nc as usize);
                if !reachable[nr][nc] && g[nr][nc] != wall_color {
                    reachable[nr][nc] = true;
                    stack.push((nr, nc));
                }
            }
        }
    }
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] == 0 && !reachable[r][c] {
                result[r][c] = wall_color;
            }
        }
    }
    result
}

fn upscale_objects(g: &Grid, factor: usize) -> Grid {
    if g.is_empty() || factor == 0 { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = vec![vec![0u8; cols * factor]; rows * factor];
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] != 0 {
                for dr in 0..factor {
                    for dc in 0..factor {
                        result[r * factor + dr][c * factor + dc] = g[r][c];
                    }
                }
            }
        }
    }
    result
}

fn fill_inside_objects(g: &Grid, fill_color: u8) -> Grid {
    if g.is_empty() { return g.clone(); }
    let rows = g.len();
    let cols = g[0].len();
    let mut result = g.clone();

    // For each object, find enclosed holes (0s not reachable from border)
    let mut reachable = vec![vec![false; cols]; rows];
    let mut stack: Vec<(usize, usize)> = Vec::new();

    // Start BFS from all border 0s
    for r in 0..rows {
        for c in 0..cols {
            if (r == 0 || r == rows - 1 || c == 0 || c == cols - 1) && g[r][c] == 0 {
                reachable[r][c] = true;
                stack.push((r, c));
            }
        }
    }

    while let Some((r, c)) = stack.pop() {
        for (dr, dc) in &[(0i32, 1i32), (0, -1), (1, 0), (-1, 0)] {
            let nr = r as i32 + dr;
            let nc = c as i32 + dc;
            if nr >= 0 && nr < rows as i32 && nc >= 0 && nc < cols as i32 {
                let (nr, nc) = (nr as usize, nc as usize);
                if !reachable[nr][nc] && g[nr][nc] == 0 {
                    reachable[nr][nc] = true;
                    stack.push((nr, nc));
                }
            }
        }
    }

    // Fill unreachable 0s
    for r in 0..rows {
        for c in 0..cols {
            if g[r][c] == 0 && !reachable[r][c] {
                result[r][c] = fill_color;
            }
        }
    }
    result
}
