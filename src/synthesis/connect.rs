// Connect: draw lines between same-color markers.
//
// One of the most frequent ARC patterns (~15% of tasks):
// Single-pixel markers of the same color → draw H/V/diagonal lines between them.
// The fill color is learned from training examples.

use super::dsl::{Grid, connected_components, grid_dimensions};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct ConnectSolution {
    pub rules: Vec<ConnectRule>,
    pub method: String,
}

#[derive(Debug, Clone)]
pub struct ConnectRule {
    pub marker_color: u8,
    pub fill_color: u8,
    pub mode: ConnectMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectMode {
    HLine,    // horizontal between markers on same row
    VLine,    // vertical between markers on same column
    HVLine,   // both H and V
    Diagonal, // diagonal between markers
    FullRow,  // extend marker to fill entire row
    FullCol,  // extend marker to fill entire column
}

pub fn try_connect_solve(examples: &[(Grid, Grid)]) -> Option<ConnectSolution> {
    if examples.is_empty() { return None; }

    // Strategy 1: Connect pairs of same-color markers
    if let Some(sol) = try_connect_pairs(examples) {
        return Some(sol);
    }

    // Strategy 2: Extend markers to fill row/col
    if let Some(sol) = try_extend_to_fill(examples) {
        return Some(sol);
    }

    // Strategy 3: Fill between adjacent same-color markers on same row
    if let Some(sol) = try_fill_between(examples) {
        return Some(sol);
    }

    None
}

fn try_connect_pairs(examples: &[(Grid, Grid)]) -> Option<ConnectSolution> {
    let (input, output) = &examples[0];
    if input.len() != output.len() || input.is_empty() || input[0].len() != output[0].len() {
        return None;
    }
    let (rows, cols) = grid_dimensions(input);

    // Find single-pixel markers in input
    let objects = connected_components(input, true);
    let markers: Vec<_> = objects.iter()
        .filter(|o| o.area() <= 2)
        .collect();
    if markers.len() < 2 { return None; }

    // Group markers by color
    let mut by_color: FxHashMap<u8, Vec<(usize, usize)>> = FxHashMap::default();
    for m in &markers {
        if m.area() == 1 {
            by_color.entry(m.color).or_default().push(m.cells[0]);
        }
    }

    let mut rules = Vec::new();

    for (&color, positions) in &by_color {
        if positions.len() < 2 { continue; }

        // Find new cells in output that weren't in input
        // These are the lines drawn
        let mut new_cells: Vec<(usize, usize, u8)> = Vec::new();
        for r in 0..rows {
            for c in 0..cols {
                if input[r][c] == 0 && output[r][c] != 0 {
                    new_cells.push((r, c, output[r][c]));
                }
            }
        }
        if new_cells.is_empty() { continue; }

        // Determine fill color (most common new color)
        let mut color_counts: FxHashMap<u8, usize> = FxHashMap::default();
        for &(_, _, c) in &new_cells {
            *color_counts.entry(c).or_default() += 1;
        }
        let fill_color = color_counts.iter()
            .max_by_key(|(_, &cnt)| cnt)
            .map(|(&c, _)| c)?;

        // Try HLine: connect pairs on same row
        let test_h = apply_connect_pairs(input, color, fill_color, ConnectMode::HLine);
        if grid_matches_new_cells(&test_h, output) {
            rules.push(ConnectRule { marker_color: color, fill_color, mode: ConnectMode::HLine });
            continue;
        }

        // Try VLine: connect pairs on same column
        let test_v = apply_connect_pairs(input, color, fill_color, ConnectMode::VLine);
        if grid_matches_new_cells(&test_v, output) {
            rules.push(ConnectRule { marker_color: color, fill_color, mode: ConnectMode::VLine });
            continue;
        }

        // Try both H+V
        let test_hv = apply_connect_pairs(input, color, fill_color, ConnectMode::HVLine);
        if grid_matches_new_cells(&test_hv, output) {
            rules.push(ConnectRule { marker_color: color, fill_color, mode: ConnectMode::HVLine });
            continue;
        }

        // Try diagonal
        let test_d = apply_connect_pairs(input, color, fill_color, ConnectMode::Diagonal);
        if grid_matches_new_cells(&test_d, output) {
            rules.push(ConnectRule { marker_color: color, fill_color, mode: ConnectMode::Diagonal });
        }
    }

    if rules.is_empty() { return None; }

    // Verify on all examples
    let result = apply_all_rules(input, &rules);
    if result != *output { return None; }

    let all_ok = examples[1..].iter().all(|(inp, out)| {
        apply_all_rules(inp, &rules) == *out
    });
    if !all_ok { return None; }

    Some(ConnectSolution {
        rules,
        method: "connect_pairs".into(),
    })
}

fn try_extend_to_fill(examples: &[(Grid, Grid)]) -> Option<ConnectSolution> {
    let (input, output) = &examples[0];
    if input.len() != output.len() || input.is_empty() || input[0].len() != output[0].len() {
        return None;
    }

    let objects = connected_components(input, true);
    let markers: Vec<_> = objects.iter().filter(|o| o.area() == 1).collect();
    if markers.is_empty() { return None; }

    // Try: each marker fills its entire row with its color
    let test_full_row = apply_extend_markers(input, ConnectMode::FullRow);
    if test_full_row == *output {
        let all_ok = examples[1..].iter().all(|(inp, out)| {
            apply_extend_markers(inp, ConnectMode::FullRow) == *out
        });
        if all_ok {
            return Some(ConnectSolution {
                rules: vec![],
                method: "extend_full_row".into(),
            });
        }
    }

    // Try: each marker fills its entire column
    let test_full_col = apply_extend_markers(input, ConnectMode::FullCol);
    if test_full_col == *output {
        let all_ok = examples[1..].iter().all(|(inp, out)| {
            apply_extend_markers(inp, ConnectMode::FullCol) == *out
        });
        if all_ok {
            return Some(ConnectSolution {
                rules: vec![],
                method: "extend_full_col".into(),
            });
        }
    }

    None
}

fn try_fill_between(examples: &[(Grid, Grid)]) -> Option<ConnectSolution> {
    let (input, output) = &examples[0];
    if input.len() != output.len() || input.is_empty() || input[0].len() != output[0].len() {
        return None;
    }
    let (rows, cols) = grid_dimensions(input);

    // Try: for each row, find pairs of same-color cells and fill between them
    let mut test = input.clone();
    for r in 0..rows {
        let mut colored: Vec<(usize, u8)> = Vec::new();
        for c in 0..cols {
            if input[r][c] != 0 {
                colored.push((c, input[r][c]));
            }
        }
        // Fill between same-color pairs
        for i in 0..colored.len() {
            for j in (i+1)..colored.len() {
                if colored[i].1 == colored[j].1 {
                    let (c1, c2) = (colored[i].0, colored[j].0);
                    let fill = colored[i].1;
                    for c in c1..=c2 {
                        if test[r][c] == 0 { test[r][c] = fill; }
                    }
                }
            }
        }
    }

    if test == *output {
        let all_ok = examples[1..].iter().all(|(inp, out)| {
            let mut t = inp.clone();
            let (rows, cols) = grid_dimensions(inp);
            for r in 0..rows {
                let mut colored: Vec<(usize, u8)> = Vec::new();
                for c in 0..cols {
                    if inp[r][c] != 0 { colored.push((c, inp[r][c])); }
                }
                for i in 0..colored.len() {
                    for j in (i+1)..colored.len() {
                        if colored[i].1 == colored[j].1 {
                            let (c1, c2) = (colored[i].0, colored[j].0);
                            for c in c1..=c2 {
                                if t[r][c] == 0 { t[r][c] = colored[i].1; }
                            }
                        }
                    }
                }
            }
            t == *out
        });
        if all_ok {
            return Some(ConnectSolution {
                rules: vec![],
                method: "fill_between_same_row".into(),
            });
        }
    }

    // Also try column-wise
    let mut test = input.clone();
    for c in 0..cols {
        let mut colored: Vec<(usize, u8)> = Vec::new();
        for r in 0..rows {
            if input[r][c] != 0 { colored.push((r, input[r][c])); }
        }
        for i in 0..colored.len() {
            for j in (i+1)..colored.len() {
                if colored[i].1 == colored[j].1 {
                    let (r1, r2) = (colored[i].0, colored[j].0);
                    for r in r1..=r2 {
                        if test[r][c] == 0 { test[r][c] = colored[i].1; }
                    }
                }
            }
        }
    }

    if test == *output {
        let all_ok = examples[1..].iter().all(|(inp, out)| {
            let mut t = inp.clone();
            let (rows, cols) = grid_dimensions(inp);
            for c in 0..cols {
                let mut colored: Vec<(usize, u8)> = Vec::new();
                for r in 0..rows { if inp[r][c] != 0 { colored.push((r, inp[r][c])); } }
                for i in 0..colored.len() {
                    for j in (i+1)..colored.len() {
                        if colored[i].1 == colored[j].1 {
                            let (r1, r2) = (colored[i].0, colored[j].0);
                            for r in r1..=r2 { if t[r][c] == 0 { t[r][c] = colored[i].1; } }
                        }
                    }
                }
            }
            t == *out
        });
        if all_ok {
            return Some(ConnectSolution {
                rules: vec![],
                method: "fill_between_same_col".into(),
            });
        }
    }

    None
}

fn apply_connect_pairs(grid: &Grid, marker_color: u8, fill_color: u8, mode: ConnectMode) -> Grid {
    let (rows, cols) = grid_dimensions(grid);
    let mut result = grid.clone();

    // Find marker positions
    let objects = connected_components(grid, true);
    let positions: Vec<(usize, usize)> = objects.iter()
        .filter(|o| o.color == marker_color && o.area() == 1)
        .map(|o| o.cells[0])
        .collect();

    for i in 0..positions.len() {
        for j in (i+1)..positions.len() {
            let (r1, c1) = positions[i];
            let (r2, c2) = positions[j];

            match mode {
                ConnectMode::HLine => {
                    if r1 == r2 {
                        let (min_c, max_c) = (c1.min(c2), c1.max(c2));
                        for c in min_c..=max_c {
                            if result[r1][c] == 0 { result[r1][c] = fill_color; }
                        }
                    }
                }
                ConnectMode::VLine => {
                    if c1 == c2 {
                        let (min_r, max_r) = (r1.min(r2), r1.max(r2));
                        for r in min_r..=max_r {
                            if result[r][c1] == 0 { result[r][c1] = fill_color; }
                        }
                    }
                }
                ConnectMode::HVLine => {
                    if r1 == r2 {
                        let (min_c, max_c) = (c1.min(c2), c1.max(c2));
                        for c in min_c..=max_c {
                            if result[r1][c] == 0 { result[r1][c] = fill_color; }
                        }
                    }
                    if c1 == c2 {
                        let (min_r, max_r) = (r1.min(r2), r1.max(r2));
                        for r in min_r..=max_r {
                            if result[r][c1] == 0 { result[r][c1] = fill_color; }
                        }
                    }
                }
                ConnectMode::Diagonal => {
                    let dr = (r2 as i32 - r1 as i32).signum();
                    let dc = (c2 as i32 - c1 as i32).signum();
                    if dr.abs() == dc.abs() || dr == 0 || dc == 0 {
                        let mut r = r1 as i32;
                        let mut c = c1 as i32;
                        while r != r2 as i32 || c != c2 as i32 {
                            if r >= 0 && (r as usize) < rows && c >= 0 && (c as usize) < cols {
                                if result[r as usize][c as usize] == 0 {
                                    result[r as usize][c as usize] = fill_color;
                                }
                            }
                            r += dr; c += dc;
                        }
                    }
                }
                _ => {}
            }
        }
    }
    result
}

fn apply_extend_markers(grid: &Grid, mode: ConnectMode) -> Grid {
    let (rows, cols) = grid_dimensions(grid);
    let mut result = grid.clone();
    let objects = connected_components(grid, true);

    for obj in &objects {
        if obj.area() != 1 { continue; }
        let (r, c) = obj.cells[0];
        let color = obj.color;
        match mode {
            ConnectMode::FullRow => {
                for cc in 0..cols { if result[r][cc] == 0 { result[r][cc] = color; } }
            }
            ConnectMode::FullCol => {
                for rr in 0..rows { if result[rr][c] == 0 { result[rr][c] = color; } }
            }
            _ => {}
        }
    }
    result
}

fn apply_all_rules(grid: &Grid, rules: &[ConnectRule]) -> Grid {
    let mut result = grid.clone();
    for rule in rules {
        result = apply_connect_pairs(&result, rule.marker_color, rule.fill_color, rule.mode);
    }
    result
}

fn grid_matches_new_cells(candidate: &Grid, expected: &Grid) -> bool {
    if candidate.len() != expected.len() { return false; }
    if candidate.is_empty() { return true; }
    if candidate[0].len() != expected[0].len() { return false; }
    candidate.iter().zip(expected.iter()).all(|(cr, er)| {
        cr.iter().zip(er.iter()).all(|(cv, ev)| cv == ev)
    })
}

impl ConnectSolution {
    pub fn apply(&self, grid: &Grid) -> Grid {
        match self.method.as_str() {
            "connect_pairs" => apply_all_rules(grid, &self.rules),
            "extend_full_row" => apply_extend_markers(grid, ConnectMode::FullRow),
            "extend_full_col" => apply_extend_markers(grid, ConnectMode::FullCol),
            "fill_between_same_row" => {
                let (rows, cols) = grid_dimensions(grid);
                let mut t = grid.clone();
                for r in 0..rows {
                    let mut colored: Vec<(usize, u8)> = Vec::new();
                    for c in 0..cols { if grid[r][c] != 0 { colored.push((c, grid[r][c])); } }
                    for i in 0..colored.len() {
                        for j in (i+1)..colored.len() {
                            if colored[i].1 == colored[j].1 {
                                let (c1, c2) = (colored[i].0, colored[j].0);
                                for c in c1..=c2 { if t[r][c] == 0 { t[r][c] = colored[i].1; } }
                            }
                        }
                    }
                }
                t
            }
            "fill_between_same_col" => {
                let (rows, cols) = grid_dimensions(grid);
                let mut t = grid.clone();
                for c in 0..cols {
                    let mut colored: Vec<(usize, u8)> = Vec::new();
                    for r in 0..rows { if grid[r][c] != 0 { colored.push((r, grid[r][c])); } }
                    for i in 0..colored.len() {
                        for j in (i+1)..colored.len() {
                            if colored[i].1 == colored[j].1 {
                                let (r1, r2) = (colored[i].0, colored[j].0);
                                for r in r1..=r2 { if t[r][c] == 0 { t[r][c] = colored[i].1; } }
                            }
                        }
                    }
                }
                t
            }
            _ => grid.clone(),
        }
    }

    pub fn name(&self) -> &str {
        &self.method
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connect_h_pair() {
        let input = vec![
            vec![0, 0, 0, 0, 0],
            vec![0, 3, 0, 3, 0],
            vec![0, 0, 0, 0, 0],
        ];
        let output = vec![
            vec![0, 0, 0, 0, 0],
            vec![0, 3, 7, 3, 0],
            vec![0, 0, 0, 0, 0],
        ];
        let result = apply_connect_pairs(&input, 3, 7, ConnectMode::HLine);
        assert_eq!(result, output);
    }

    #[test]
    fn connect_v_pair() {
        let input = vec![
            vec![0, 3, 0],
            vec![0, 0, 0],
            vec![0, 3, 0],
        ];
        let expected = vec![
            vec![0, 3, 0],
            vec![0, 7, 0],
            vec![0, 3, 0],
        ];
        let result = apply_connect_pairs(&input, 3, 7, ConnectMode::VLine);
        assert_eq!(result, expected);
    }

    #[test]
    fn fill_between_row() {
        let input = vec![
            vec![0, 2, 0, 0, 2, 0],
            vec![0, 0, 0, 0, 0, 0],
        ];
        let expected = vec![
            vec![0, 2, 2, 2, 2, 0],
            vec![0, 0, 0, 0, 0, 0],
        ];
        let examples = vec![(input.clone(), expected.clone())];
        let sol = try_fill_between(&examples);
        assert!(sol.is_some());
        assert_eq!(sol.unwrap().apply(&input), expected);
    }

    #[test]
    fn extend_to_full_row() {
        let input = vec![
            vec![0, 0, 0],
            vec![0, 5, 0],
            vec![0, 0, 0],
        ];
        let expected = vec![
            vec![0, 0, 0],
            vec![5, 5, 5],
            vec![0, 0, 0],
        ];
        let result = apply_extend_markers(&input, ConnectMode::FullRow);
        assert_eq!(result, expected);
    }
}
