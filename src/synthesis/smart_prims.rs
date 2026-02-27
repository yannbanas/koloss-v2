// Smart primitives: operations that LEARN from input/output examples.
//
// Unlike static primitives (FlipH, RotateCW), these analyze the
// training examples to infer parameters, then apply the learned
// transformation to new inputs.
//
// This is the key insight from SOTA ARC solvers:
// Don't just enumerate fixed operations — infer the operation from data.

use super::dsl::Grid;
use rustc_hash::FxHashMap;

/// Learn a color mapping from one example pair.
/// Returns None if no consistent mapping exists.
pub fn learn_color_map(input: &Grid, output: &Grid) -> Option<FxHashMap<u8, u8>> {
    if input.len() != output.len() { return None; }
    if input.is_empty() { return Some(FxHashMap::default()); }
    if input[0].len() != output[0].len() { return None; }

    let mut mapping: FxHashMap<u8, u8> = FxHashMap::default();

    for (ir, or) in input.iter().zip(output.iter()) {
        for (&ic, &oc) in ir.iter().zip(or.iter()) {
            if let Some(&existing) = mapping.get(&ic) {
                if existing != oc { return None; } // inconsistent
            } else {
                mapping.insert(ic, oc);
            }
        }
    }

    Some(mapping)
}

/// Verify a color map works for all training examples.
pub fn verify_color_map(map: &FxHashMap<u8, u8>, examples: &[(Grid, Grid)]) -> bool {
    examples.iter().all(|(input, output)| {
        if input.len() != output.len() { return false; }
        if input.is_empty() { return true; }
        if input[0].len() != output[0].len() { return false; }
        input.iter().zip(output.iter()).all(|(ir, or)| {
            ir.iter().zip(or.iter()).all(|(ic, oc)| {
                map.get(ic).map(|m| m == oc).unwrap_or(false)
            })
        })
    })
}

/// Apply a color mapping to a grid.
pub fn apply_color_map(grid: &Grid, map: &FxHashMap<u8, u8>) -> Grid {
    grid.iter().map(|row| {
        row.iter().map(|&c| *map.get(&c).unwrap_or(&c)).collect()
    }).collect()
}

/// Self-tiling: each non-zero cell in the grid gets replaced by a copy
/// of the grid itself. Zero cells become all-zero blocks.
/// Output size = input_rows * input_rows × input_cols * input_cols.
pub fn tile_with_self(grid: &Grid) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let rows = grid.len();
    let cols = grid[0].len();
    let out_rows = rows * rows;
    let out_cols = cols * cols;
    let mut result = vec![vec![0u8; out_cols]; out_rows];

    for r in 0..rows {
        for c in 0..cols {
            if grid[r][c] != 0 {
                // Place a copy of the grid at block position (r, c)
                for br in 0..rows {
                    for bc in 0..cols {
                        result[r * rows + br][c * cols + bc] = grid[br][bc];
                    }
                }
            }
        }
    }
    result
}

/// Tile a grid n_r × n_c times.
pub fn tile_grid(grid: &Grid, n_r: usize, n_c: usize) -> Grid {
    if grid.is_empty() || n_r == 0 || n_c == 0 { return Vec::new(); }
    let rows = grid.len();
    let cols = grid[0].len();
    let mut result = vec![vec![0u8; cols * n_c]; rows * n_r];
    for tr in 0..n_r {
        for tc in 0..n_c {
            for r in 0..rows {
                for c in 0..cols {
                    result[tr * rows + r][tc * cols + c] = grid[r][c];
                }
            }
        }
    }
    result
}

/// Detect if output = input tiled n×m times. Returns (n_r, n_c) if so.
pub fn detect_tiling(input: &Grid, output: &Grid) -> Option<(usize, usize)> {
    if input.is_empty() || output.is_empty() { return None; }
    let in_r = input.len();
    let in_c = input[0].len();
    let out_r = output.len();
    let out_c = output[0].len();

    if out_r % in_r != 0 || out_c % in_c != 0 { return None; }
    let n_r = out_r / in_r;
    let n_c = out_c / in_c;
    if n_r == 0 || n_c == 0 { return None; }

    let tiled = tile_grid(input, n_r, n_c);
    if tiled == *output { Some((n_r, n_c)) } else { None }
}

/// Detect if output is the input with self-tiling applied.
pub fn detect_self_tiling(input: &Grid, output: &Grid) -> bool {
    tile_with_self(input) == *output
}

/// Extract a subgrid from the grid at position (r, c) with size (h, w).
pub fn extract_subgrid(grid: &Grid, r: usize, c: usize, h: usize, w: usize) -> Grid {
    grid.iter().skip(r).take(h)
        .map(|row| row.iter().skip(c).take(w).cloned().collect())
        .collect()
}

/// Detect if output is a subgrid of input. Returns (r, c, h, w) if so.
pub fn detect_subgrid(input: &Grid, output: &Grid) -> Option<(usize, usize, usize, usize)> {
    if output.is_empty() { return None; }
    let out_r = output.len();
    let out_c = output[0].len();

    for r in 0..=input.len().saturating_sub(out_r) {
        for c in 0..=input.get(0).map(|row| row.len()).unwrap_or(0).saturating_sub(out_c) {
            let sub = extract_subgrid(input, r, c, out_r, out_c);
            if sub == *output {
                return Some((r, c, out_r, out_c));
            }
        }
    }
    None
}

/// Deduplicate consecutive identical rows.
pub fn dedup_rows(grid: &Grid) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let mut result = vec![grid[0].clone()];
    for row in &grid[1..] {
        if *row != *result.last().unwrap() {
            result.push(row.clone());
        }
    }
    result
}

/// Deduplicate consecutive identical columns.
pub fn dedup_cols(grid: &Grid) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let cols = grid[0].len();
    if cols == 0 { return grid.clone(); }

    let mut keep = vec![true; cols];
    for c in 1..cols {
        let same = grid.iter().all(|row| row[c] == row[c - 1]);
        if same { keep[c] = false; }
    }

    grid.iter().map(|row| {
        row.iter().enumerate()
            .filter(|(i, _)| keep[*i])
            .map(|(_, &v)| v)
            .collect()
    }).collect()
}

/// Majority vote per cell across multiple grids.
/// Useful for consensus when multiple strategies produce partial results.
pub fn majority_vote(grids: &[Grid]) -> Grid {
    if grids.is_empty() { return Vec::new(); }
    let rows = grids[0].len();
    if rows == 0 { return Vec::new(); }
    let cols = grids[0][0].len();

    let mut result = vec![vec![0u8; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            let mut counts = [0u32; 10];
            for g in grids {
                if r < g.len() && c < g[r].len() {
                    let v = g[r][c] as usize;
                    if v < 10 { counts[v] += 1; }
                }
            }
            result[r][c] = counts.iter().enumerate()
                .max_by_key(|(_, &cnt)| cnt)
                .map(|(i, _)| i as u8)
                .unwrap_or(0);
        }
    }
    result
}

/// Try all smart/learned transforms and return the first that works.
pub fn try_smart_transforms(examples: &[(Grid, Grid)]) -> Option<SmartTransform> {
    if examples.is_empty() { return None; }

    // 1. Try color mapping
    if let Some(map) = learn_color_map(&examples[0].0, &examples[0].1) {
        if verify_color_map(&map, examples) {
            return Some(SmartTransform::ColorMap(map));
        }
    }

    // 2. Try self-tiling
    if detect_self_tiling(&examples[0].0, &examples[0].1) {
        let all_match = examples.iter().all(|(i, o)| detect_self_tiling(i, o));
        if all_match {
            return Some(SmartTransform::SelfTile);
        }
    }

    // 3. Try regular tiling
    if let Some((nr, nc)) = detect_tiling(&examples[0].0, &examples[0].1) {
        let all_match = examples.iter().all(|(i, o)| {
            detect_tiling(i, o) == Some((nr, nc))
        });
        if all_match {
            return Some(SmartTransform::Tile(nr, nc));
        }
    }

    // 4. Try subgrid extraction with fixed offset
    if let Some((r, c, h, w)) = detect_subgrid(&examples[0].0, &examples[0].1) {
        // Check if same (r,c) works for all examples (fixed crop)
        let all_match = examples.iter().all(|(input, output)| {
            let sub = extract_subgrid(input, r, c, h, w);
            sub == *output
        });
        if all_match {
            return Some(SmartTransform::Subgrid(r, c, h, w));
        }
    }

    // 5. Try row dedup
    {
        let all_match = examples.iter().all(|(i, o)| dedup_rows(i) == *o);
        if all_match {
            return Some(SmartTransform::DedupRows);
        }
    }

    // 6. Try column dedup
    {
        let all_match = examples.iter().all(|(i, o)| dedup_cols(i) == *o);
        if all_match {
            return Some(SmartTransform::DedupCols);
        }
    }

    // 7. Try periodic pattern repair (fill 0-holes in tiled grid)
    if let Some((pr, pc)) = detect_damaged_period(&examples[0].0, &examples[0].1) {
        let all_match = examples.iter().all(|(i, o)| {
            repair_period(i, pr, pc) == *o
        });
        if all_match {
            return Some(SmartTransform::RepairPeriod(pr, pc));
        }
    }

    None
}

#[derive(Debug, Clone)]
pub enum SmartTransform {
    ColorMap(FxHashMap<u8, u8>),
    SelfTile,
    Tile(usize, usize),
    Subgrid(usize, usize, usize, usize),
    DedupRows,
    DedupCols,
    RepairPeriod(usize, usize), // (period_r, period_c)
}

impl SmartTransform {
    pub fn apply(&self, grid: &Grid) -> Grid {
        match self {
            SmartTransform::ColorMap(map) => apply_color_map(grid, map),
            SmartTransform::SelfTile => tile_with_self(grid),
            SmartTransform::Tile(nr, nc) => tile_grid(grid, *nr, *nc),
            SmartTransform::Subgrid(r, c, h, w) => extract_subgrid(grid, *r, *c, *h, *w),
            SmartTransform::DedupRows => dedup_rows(grid),
            SmartTransform::DedupCols => dedup_cols(grid),
            SmartTransform::RepairPeriod(pr, pc) => repair_period(grid, *pr, *pc),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            SmartTransform::ColorMap(_) => "color_map",
            SmartTransform::SelfTile => "self_tile",
            SmartTransform::Tile(_, _) => "tile",
            SmartTransform::Subgrid(_, _, _, _) => "subgrid",
            SmartTransform::DedupRows => "dedup_rows",
            SmartTransform::DedupCols => "dedup_cols",
            SmartTransform::RepairPeriod(_, _) => "repair_period",
        }
    }
}

// --- Periodic pattern repair ---

pub fn detect_damaged_period(input: &Grid, output: &Grid) -> Option<(usize, usize)> {
    if input.len() != output.len() || input.is_empty() || input[0].len() != output[0].len() {
        return None;
    }
    let rows = input.len();
    let cols = input[0].len();

    // The output should be a perfectly periodic grid
    // The input should be the same but with some 0-holes
    // Try different period sizes
    for pr in 1..=rows / 2 {
        if rows % pr != 0 { continue; }
        for pc in 1..=cols / 2 {
            if cols % pc != 0 { continue; }
            // Check if output is periodic with this period
            let output_periodic = (0..rows).all(|r| {
                (0..cols).all(|c| output[r][c] == output[r % pr][c % pc])
            });
            if !output_periodic { continue; }

            // Check if input matches output except where input has 0
            let input_consistent = (0..rows).all(|r| {
                (0..cols).all(|c| input[r][c] == 0 || input[r][c] == output[r][c])
            });
            if !input_consistent { continue; }

            // Must have at least some damage (0s that get filled)
            let has_damage = (0..rows).any(|r| {
                (0..cols).any(|c| input[r][c] == 0 && output[r][c] != 0)
            });
            if has_damage {
                return Some((pr, pc));
            }
        }
    }
    None
}

pub fn repair_period(grid: &Grid, pr: usize, pc: usize) -> Grid {
    if grid.is_empty() || pr == 0 || pc == 0 { return grid.clone(); }
    let rows = grid.len();
    let cols = grid[0].len();

    // Build tile by majority vote across all period positions
    let mut tile = vec![vec![0u8; pc]; pr];
    for tr in 0..pr {
        for tc in 0..pc {
            let mut counts = [0u32; 10];
            let mut r = tr;
            while r < rows {
                let mut c = tc;
                while c < cols {
                    let v = grid[r][c] as usize;
                    if v > 0 && v < 10 { counts[v] += 1; }
                    c += pc;
                }
                r += pr;
            }
            // Pick most common non-zero color
            tile[tr][tc] = counts.iter().enumerate()
                .skip(1) // skip color 0
                .max_by_key(|(_, &cnt)| cnt)
                .filter(|(_, &cnt)| cnt > 0)
                .map(|(i, _)| i as u8)
                .unwrap_or(0);
        }
    }

    // Apply tile to fill all cells
    let mut result = vec![vec![0u8; cols]; rows];
    for r in 0..rows {
        for c in 0..cols {
            result[r][c] = tile[r % pr][c % pc];
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_map_simple() {
        let input = vec![vec![1, 2], vec![3, 0]];
        let output = vec![vec![4, 5], vec![6, 0]];
        let map = learn_color_map(&input, &output).unwrap();
        assert_eq!(map[&1], 4);
        assert_eq!(map[&2], 5);
        assert_eq!(map[&3], 6);
        assert_eq!(map[&0], 0);
        assert_eq!(apply_color_map(&input, &map), output);
    }

    #[test]
    fn color_map_inconsistent() {
        let input = vec![vec![1, 1]];
        let output = vec![vec![2, 3]]; // 1→2 and 1→3 conflict
        assert!(learn_color_map(&input, &output).is_none());
    }

    #[test]
    fn self_tiling() {
        let input = vec![vec![0, 1], vec![1, 1]];
        let output = tile_with_self(&input);
        assert_eq!(output.len(), 4);
        assert_eq!(output[0].len(), 4);
        // Top-left block (input[0][0]=0) should be all zeros
        assert_eq!(output[0][0], 0);
        assert_eq!(output[0][1], 0);
        assert_eq!(output[1][0], 0);
        assert_eq!(output[1][1], 0);
        // Top-right block (input[0][1]=1) should be copy of input
        assert_eq!(output[0][2], 0);
        assert_eq!(output[0][3], 1);
        assert_eq!(output[1][2], 1);
        assert_eq!(output[1][3], 1);
    }

    #[test]
    fn detect_self_tiling_works() {
        let input = vec![vec![0, 7, 7], vec![7, 7, 7], vec![0, 7, 7]];
        let output = tile_with_self(&input);
        assert!(detect_self_tiling(&input, &output));
    }

    #[test]
    fn tiling_2x3() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let output = tile_grid(&input, 2, 3);
        assert_eq!(output.len(), 4);
        assert_eq!(output[0].len(), 6);
        assert_eq!(detect_tiling(&input, &output), Some((2, 3)));
    }

    #[test]
    fn subgrid_detection() {
        let input = vec![
            vec![1, 2, 3, 4],
            vec![5, 6, 7, 8],
            vec![9, 0, 1, 2],
        ];
        let output = vec![vec![6, 7], vec![0, 1]]; // rows 1-2, cols 1-2
        let result = detect_subgrid(&input, &output);
        assert_eq!(result, Some((1, 1, 2, 2)));
    }

    #[test]
    fn dedup_rows_basic() {
        let grid = vec![vec![1, 2], vec![1, 2], vec![3, 4], vec![3, 4]];
        let result = dedup_rows(&grid);
        assert_eq!(result, vec![vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn dedup_cols_basic() {
        let grid = vec![vec![1, 1, 2, 2], vec![3, 3, 4, 4]];
        let result = dedup_cols(&grid);
        assert_eq!(result, vec![vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn smart_finds_color_map() {
        let examples = vec![
            (vec![vec![1, 2]], vec![vec![3, 4]]),
            (vec![vec![2, 1]], vec![vec![4, 3]]),
        ];
        let result = try_smart_transforms(&examples);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "color_map");
    }

    #[test]
    fn smart_finds_self_tile() {
        let input = vec![vec![0, 1], vec![1, 1]];
        let output = tile_with_self(&input);
        let examples = vec![(input, output)];
        let result = try_smart_transforms(&examples);
        assert!(result.is_some());
        assert_eq!(result.unwrap().name(), "self_tile");
    }

    #[test]
    fn majority_vote_basic() {
        let g1 = vec![vec![1, 2], vec![3, 4]];
        let g2 = vec![vec![1, 5], vec![3, 4]];
        let g3 = vec![vec![1, 2], vec![6, 4]];
        let result = majority_vote(&[g1, g2, g3]);
        assert_eq!(result, vec![vec![1, 2], vec![3, 4]]); // majority wins
    }
}
