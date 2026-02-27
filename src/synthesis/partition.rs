// Grid partitioning + sub-grid operations for ARC-AGI.
//
// Many ARC tasks split the grid at separator lines (rows/columns of
// a single color), then compare, select, or recombine the sub-regions.
//
// Operations:
// 1. Detect separator lines (horizontal/vertical)
// 2. Split grid into sub-grids
// 3. Compare sub-grids (XOR, AND, difference marking)
// 4. Select sub-grid by predicate (unique color, max objects, etc.)
// 5. Overlay/merge sub-grids

use super::dsl::{Grid, unique_colors, connected_components};

#[derive(Debug, Clone)]
pub struct GridPartition {
    pub sub_grids: Vec<Grid>,
    pub layout: PartitionLayout,
}

#[derive(Debug, Clone)]
pub enum PartitionLayout {
    Horizontal(Vec<usize>), // row indices of separators
    Vertical(Vec<usize>),   // col indices of separators
    Grid2D(Vec<usize>, Vec<usize>), // both row + col separators
}

pub fn detect_h_separators(grid: &Grid) -> Vec<usize> {
    if grid.is_empty() { return Vec::new(); }
    let mut seps = Vec::new();
    for r in 0..grid.len() {
        let c0 = grid[r][0];
        if c0 != 0 && grid[r].iter().all(|&c| c == c0) {
            // Check it's not the only row color (separator should differ from content)
            let is_sep = if r > 0 { grid[r - 1].iter().any(|&c| c != c0) } else { true };
            let is_sep2 = if r + 1 < grid.len() { grid[r + 1].iter().any(|&c| c != c0) } else { true };
            if is_sep || is_sep2 { seps.push(r); }
        }
    }
    seps
}

pub fn detect_v_separators(grid: &Grid) -> Vec<usize> {
    if grid.is_empty() { return Vec::new(); }
    let rows = grid.len();
    let cols = grid[0].len();
    let mut seps = Vec::new();
    for c in 0..cols {
        let c0 = grid[0][c];
        if c0 != 0 && (0..rows).all(|r| grid[r][c] == c0) {
            let is_sep = if c > 0 { (0..rows).any(|r| grid[r][c - 1] != c0) } else { true };
            let is_sep2 = if c + 1 < cols { (0..rows).any(|r| grid[r][c + 1] != c0) } else { true };
            if is_sep || is_sep2 { seps.push(c); }
        }
    }
    seps
}

pub fn split_at_h_separators(grid: &Grid, seps: &[usize]) -> Vec<Grid> {
    if seps.is_empty() { return vec![grid.clone()]; }
    let mut result = Vec::new();
    let mut start = 0;
    for &sep in seps {
        if sep > start {
            let sub: Grid = grid[start..sep].to_vec();
            if !sub.is_empty() { result.push(sub); }
        }
        start = sep + 1;
    }
    if start < grid.len() {
        result.push(grid[start..].to_vec());
    }
    result
}

pub fn split_at_v_separators(grid: &Grid, seps: &[usize]) -> Vec<Grid> {
    if grid.is_empty() || seps.is_empty() { return vec![grid.clone()]; }
    let cols = grid[0].len();
    let mut result = Vec::new();
    let mut start = 0;
    for &sep in seps {
        if sep > start {
            let sub: Grid = grid.iter()
                .map(|row| row[start..sep].to_vec())
                .collect();
            result.push(sub);
        }
        start = sep + 1;
    }
    if start < cols {
        let sub: Grid = grid.iter()
            .map(|row| row[start..].to_vec())
            .collect();
        result.push(sub);
    }
    result
}

pub fn split_grid_2d(grid: &Grid, h_seps: &[usize], v_seps: &[usize]) -> Vec<Grid> {
    let h_strips = split_at_h_separators(grid, h_seps);
    let mut result = Vec::new();
    for strip in &h_strips {
        let cells = split_at_v_separators(strip, v_seps);
        result.extend(cells);
    }
    result
}

pub fn partition_grid(grid: &Grid) -> Option<GridPartition> {
    let h_seps = detect_h_separators(grid);
    let v_seps = detect_v_separators(grid);

    if !h_seps.is_empty() && !v_seps.is_empty() {
        let subs = split_grid_2d(grid, &h_seps, &v_seps);
        if subs.len() >= 2 {
            return Some(GridPartition {
                sub_grids: subs,
                layout: PartitionLayout::Grid2D(h_seps, v_seps),
            });
        }
    }
    if !h_seps.is_empty() {
        let subs = split_at_h_separators(grid, &h_seps);
        if subs.len() >= 2 {
            return Some(GridPartition {
                sub_grids: subs,
                layout: PartitionLayout::Horizontal(h_seps),
            });
        }
    }
    if !v_seps.is_empty() {
        let subs = split_at_v_separators(grid, &v_seps);
        if subs.len() >= 2 {
            return Some(GridPartition {
                sub_grids: subs,
                layout: PartitionLayout::Vertical(v_seps),
            });
        }
    }
    None
}

// --- Sub-grid comparison operations ---

pub fn xor_grids(a: &Grid, b: &Grid) -> Grid {
    if a.is_empty() || b.is_empty() { return Vec::new(); }
    let rows = a.len().min(b.len());
    let cols = a[0].len().min(b[0].len());
    (0..rows).map(|r| {
        (0..cols).map(|c| {
            if a[r][c] != b[r][c] { a[r][c].max(b[r][c]) } else { 0 }
        }).collect()
    }).collect()
}

pub fn and_grids(a: &Grid, b: &Grid) -> Grid {
    if a.is_empty() || b.is_empty() { return Vec::new(); }
    let rows = a.len().min(b.len());
    let cols = a[0].len().min(b[0].len());
    (0..rows).map(|r| {
        (0..cols).map(|c| {
            if a[r][c] != 0 && b[r][c] != 0 { a[r][c] } else { 0 }
        }).collect()
    }).collect()
}

pub fn or_grids(a: &Grid, b: &Grid) -> Grid {
    if a.is_empty() || b.is_empty() { return Vec::new(); }
    let rows = a.len().min(b.len());
    let cols = a[0].len().min(b[0].len());
    (0..rows).map(|r| {
        (0..cols).map(|c| {
            if a[r][c] != 0 { a[r][c] } else { b[r][c] }
        }).collect()
    }).collect()
}

pub fn diff_grids(a: &Grid, b: &Grid, mark_color: u8) -> Grid {
    if a.is_empty() || b.is_empty() { return Vec::new(); }
    let rows = a.len().min(b.len());
    let cols = a[0].len().min(b[0].len());
    (0..rows).map(|r| {
        (0..cols).map(|c| {
            if a[r][c] != b[r][c] { mark_color } else { 0 }
        }).collect()
    }).collect()
}

// --- Sub-grid selection predicates ---

pub fn select_most_colorful(subs: &[Grid]) -> Option<&Grid> {
    subs.iter().max_by_key(|g| {
        unique_colors(g).iter().filter(|&&c| c != 0).count()
    })
}

pub fn select_most_objects(subs: &[Grid]) -> Option<&Grid> {
    subs.iter().max_by_key(|g| connected_components(g, true).len())
}

pub fn select_unique_pattern(subs: &[Grid]) -> Option<&Grid> {
    if subs.len() < 2 { return subs.first(); }
    // Find the sub-grid that differs most from the others
    let mut best_idx = 0;
    let mut best_diff = 0usize;
    for i in 0..subs.len() {
        let diff: usize = (0..subs.len())
            .filter(|&j| j != i)
            .map(|j| grid_diff_count(&subs[i], &subs[j]))
            .sum();
        if diff > best_diff {
            best_diff = diff;
            best_idx = i;
        }
    }
    Some(&subs[best_idx])
}

fn grid_diff_count(a: &Grid, b: &Grid) -> usize {
    if a.len() != b.len() { return usize::MAX; }
    if a.is_empty() { return 0; }
    if a[0].len() != b[0].len() { return usize::MAX; }
    a.iter().zip(b.iter())
        .flat_map(|(ar, br)| ar.iter().zip(br.iter()))
        .filter(|(&ac, &bc)| ac != bc)
        .count()
}

// --- Smart partition solver: try all partition-based approaches ---

pub fn try_partition_solve(examples: &[(Grid, Grid)]) -> Option<PartitionSolution> {
    if examples.is_empty() { return None; }

    // 1. Try: output = one of the input's sub-grids
    if let Some(sol) = try_select_subgrid(examples) {
        return Some(sol);
    }

    // 2. Try: output = XOR/AND/OR of input sub-grids
    if let Some(sol) = try_combine_subgrids(examples) {
        return Some(sol);
    }

    // 3. Try: output = diff of two halves, marked with a color
    if let Some(sol) = try_diff_subgrids(examples) {
        return Some(sol);
    }

    None
}

fn try_select_subgrid(examples: &[(Grid, Grid)]) -> Option<PartitionSolution> {
    let (input, output) = &examples[0];
    let part = partition_grid(input)?;

    // Check if output matches any sub-grid directly
    for (idx, sub) in part.sub_grids.iter().enumerate() {
        if sub == output {
            // Verify on all examples
            let all_match = examples.iter().all(|(inp, out)| {
                if let Some(p) = partition_grid(inp) {
                    p.sub_grids.get(idx).map(|s| s == out).unwrap_or(false)
                } else { false }
            });
            if all_match {
                return Some(PartitionSolution {
                    method: format!("select_sub_{}", idx),
                    apply: PartitionOp::SelectIndex(idx),
                });
            }
        }
    }

    // Check: output = most colorful sub-grid
    if let Some(best) = select_most_colorful(&part.sub_grids) {
        if best == output {
            let all_match = examples.iter().all(|(inp, out)| {
                partition_grid(inp)
                    .and_then(|p| select_most_colorful(&p.sub_grids).cloned())
                    .map(|s| s == *out)
                    .unwrap_or(false)
            });
            if all_match {
                return Some(PartitionSolution {
                    method: "select_most_colorful".into(),
                    apply: PartitionOp::SelectMostColorful,
                });
            }
        }
    }

    // Check: output = unique pattern sub-grid
    if let Some(best) = select_unique_pattern(&part.sub_grids) {
        if best == output {
            let all_match = examples.iter().all(|(inp, out)| {
                partition_grid(inp)
                    .and_then(|p| select_unique_pattern(&p.sub_grids).cloned())
                    .map(|s| s == *out)
                    .unwrap_or(false)
            });
            if all_match {
                return Some(PartitionSolution {
                    method: "select_unique_pattern".into(),
                    apply: PartitionOp::SelectUniquePattern,
                });
            }
        }
    }

    None
}

fn try_combine_subgrids(examples: &[(Grid, Grid)]) -> Option<PartitionSolution> {
    let (input, output) = &examples[0];
    let part = partition_grid(input)?;
    if part.sub_grids.len() < 2 { return None; }

    // Try pairwise XOR, AND, OR
    for i in 0..part.sub_grids.len() {
        for j in (i+1)..part.sub_grids.len() {
            let a = &part.sub_grids[i];
            let b = &part.sub_grids[j];

            for (op_name, result) in [
                ("xor", xor_grids(a, b)),
                ("and", and_grids(a, b)),
                ("or", or_grids(a, b)),
            ] {
                if result == *output {
                    let all_match = examples.iter().all(|(inp, out)| {
                        if let Some(p) = partition_grid(inp) {
                            if let (Some(sa), Some(sb)) = (p.sub_grids.get(i), p.sub_grids.get(j)) {
                                let r = match op_name {
                                    "xor" => xor_grids(sa, sb),
                                    "and" => and_grids(sa, sb),
                                    "or" => or_grids(sa, sb),
                                    _ => return false,
                                };
                                r == *out
                            } else { false }
                        } else { false }
                    });
                    if all_match {
                        return Some(PartitionSolution {
                            method: format!("{}_{}{}", op_name, i, j),
                            apply: PartitionOp::Combine(i, j, op_name.to_string()),
                        });
                    }
                }
            }
        }
    }
    None
}

fn try_diff_subgrids(examples: &[(Grid, Grid)]) -> Option<PartitionSolution> {
    let (input, output) = &examples[0];
    let part = partition_grid(input)?;
    if part.sub_grids.len() < 2 { return None; }

    let out_colors = unique_colors(output);
    for &mark in &out_colors {
        if mark == 0 { continue; }
        for i in 0..part.sub_grids.len() {
            for j in 0..part.sub_grids.len() {
                if i == j { continue; }
                let diff = diff_grids(&part.sub_grids[i], &part.sub_grids[j], mark);
                if diff == *output {
                    let all_match = examples.iter().all(|(inp, out)| {
                        if let Some(p) = partition_grid(inp) {
                            if let (Some(sa), Some(sb)) = (p.sub_grids.get(i), p.sub_grids.get(j)) {
                                diff_grids(sa, sb, mark) == *out
                            } else { false }
                        } else { false }
                    });
                    if all_match {
                        return Some(PartitionSolution {
                            method: format!("diff_{}_{}_c{}", i, j, mark),
                            apply: PartitionOp::Diff(i, j, mark),
                        });
                    }
                }
            }
        }
    }
    None
}

#[derive(Debug, Clone)]
pub struct PartitionSolution {
    pub method: String,
    pub apply: PartitionOp,
}

#[derive(Debug, Clone)]
pub enum PartitionOp {
    SelectIndex(usize),
    SelectMostColorful,
    SelectUniquePattern,
    Combine(usize, usize, String),
    Diff(usize, usize, u8),
}

impl PartitionSolution {
    pub fn apply(&self, grid: &Grid) -> Grid {
        let part = match partition_grid(grid) {
            Some(p) => p,
            None => return grid.clone(),
        };
        match &self.apply {
            PartitionOp::SelectIndex(i) => {
                part.sub_grids.get(*i).cloned().unwrap_or_else(|| grid.clone())
            }
            PartitionOp::SelectMostColorful => {
                select_most_colorful(&part.sub_grids).cloned().unwrap_or_else(|| grid.clone())
            }
            PartitionOp::SelectUniquePattern => {
                select_unique_pattern(&part.sub_grids).cloned().unwrap_or_else(|| grid.clone())
            }
            PartitionOp::Combine(i, j, op) => {
                if let (Some(a), Some(b)) = (part.sub_grids.get(*i), part.sub_grids.get(*j)) {
                    match op.as_str() {
                        "xor" => xor_grids(a, b),
                        "and" => and_grids(a, b),
                        "or" => or_grids(a, b),
                        _ => grid.clone(),
                    }
                } else { grid.clone() }
            }
            PartitionOp::Diff(i, j, mark) => {
                if let (Some(a), Some(b)) = (part.sub_grids.get(*i), part.sub_grids.get(*j)) {
                    diff_grids(a, b, *mark)
                } else { grid.clone() }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_h_separator() {
        let grid = vec![
            vec![1, 2, 3],
            vec![5, 5, 5], // separator
            vec![4, 6, 7],
        ];
        let seps = detect_h_separators(&grid);
        assert_eq!(seps, vec![1]);
    }

    #[test]
    fn detect_v_separator() {
        let grid = vec![
            vec![1, 5, 3],
            vec![2, 5, 4],
            vec![6, 5, 7],
        ];
        let seps = detect_v_separators(&grid);
        assert_eq!(seps, vec![1]);
    }

    #[test]
    fn split_h_basic() {
        let grid = vec![
            vec![1, 2],
            vec![5, 5],
            vec![3, 4],
        ];
        let seps = detect_h_separators(&grid);
        let parts = split_at_h_separators(&grid, &seps);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], vec![vec![1, 2]]);
        assert_eq!(parts[1], vec![vec![3, 4]]);
    }

    #[test]
    fn split_v_basic() {
        let grid = vec![
            vec![1, 5, 3],
            vec![2, 5, 4],
        ];
        let seps = detect_v_separators(&grid);
        let parts = split_at_v_separators(&grid, &seps);
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], vec![vec![1], vec![2]]);
        assert_eq!(parts[1], vec![vec![3], vec![4]]);
    }

    #[test]
    fn xor_grids_basic() {
        let a = vec![vec![1, 0], vec![0, 1]];
        let b = vec![vec![0, 1], vec![1, 0]];
        let result = xor_grids(&a, &b);
        assert_eq!(result, vec![vec![1, 1], vec![1, 1]]);
    }

    #[test]
    fn and_grids_basic() {
        let a = vec![vec![1, 0], vec![3, 1]];
        let b = vec![vec![2, 1], vec![0, 1]];
        let result = and_grids(&a, &b);
        assert_eq!(result, vec![vec![1, 0], vec![0, 1]]);
    }

    #[test]
    fn diff_grids_basic() {
        let a = vec![vec![1, 2], vec![3, 4]];
        let b = vec![vec![1, 5], vec![3, 4]];
        let result = diff_grids(&a, &b, 7);
        assert_eq!(result, vec![vec![0, 7], vec![0, 0]]);
    }

    #[test]
    fn partition_select_subgrid() {
        // Grid split by separator, output = left half
        let input = vec![
            vec![1, 2, 5, 3, 4],
            vec![6, 7, 5, 8, 9],
        ];
        let output = vec![
            vec![1, 2],
            vec![6, 7],
        ];
        let examples = vec![(input, output)];
        let sol = try_partition_solve(&examples);
        assert!(sol.is_some());
        assert!(sol.unwrap().method.starts_with("select_sub"));
    }

    #[test]
    fn partition_xor() {
        let input = vec![
            vec![1, 0, 5, 0, 1],
            vec![0, 1, 5, 1, 0],
        ];
        let output = vec![
            vec![1, 1],
            vec![1, 1],
        ];
        let examples = vec![(input, output)];
        let sol = try_partition_solve(&examples);
        assert!(sol.is_some());
    }

    #[test]
    fn partition_2d() {
        let grid = vec![
            vec![1, 5, 2],
            vec![5, 5, 5],
            vec![3, 5, 4],
        ];
        let h = detect_h_separators(&grid);
        let v = detect_v_separators(&grid);
        let subs = split_grid_2d(&grid, &h, &v);
        assert_eq!(subs.len(), 4);
        assert_eq!(subs[0], vec![vec![1]]);
        assert_eq!(subs[1], vec![vec![2]]);
        assert_eq!(subs[2], vec![vec![3]]);
        assert_eq!(subs[3], vec![vec![4]]);
    }
}
