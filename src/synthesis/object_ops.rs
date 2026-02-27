// Per-object operations for ARC-AGI.
//
// Key insight: many ARC tasks require iterating over detected objects
// (connected components) and applying transformations to each one
// independently based on its properties (color, size, shape, position).
//
// This module provides:
// 1. Object-centric transforms (stamp patterns around markers)
// 2. Object property analysis (bounding box completion, shape detection)
// 3. Per-object conditional dispatch

use super::dsl::{Grid, Object, connected_components, grid_dimensions};

// --- Marker-based line extension ---

pub fn extend_markers_to_lines(grid: &Grid, direction: LineDir) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let rows = grid.len();
    let cols = grid[0].len();
    let mut result = grid.clone();
    let objects = connected_components(grid, true);

    for obj in &objects {
        if obj.area() == 1 {
            let (r, c) = obj.cells[0];
            let color = obj.color;
            match direction {
                LineDir::Horizontal => {
                    for cc in 0..cols { if result[r][cc] == 0 { result[r][cc] = color; } }
                }
                LineDir::Vertical => {
                    for rr in 0..rows { if result[rr][c] == 0 { result[rr][c] = color; } }
                }
                LineDir::Both => {
                    for cc in 0..cols { if result[r][cc] == 0 { result[r][cc] = color; } }
                    for rr in 0..rows { if result[rr][c] == 0 { result[rr][c] = color; } }
                }
            }
        }
    }
    result
}

#[derive(Debug, Clone, Copy)]
pub enum LineDir { Horizontal, Vertical, Both }

// --- Pattern stamping around markers ---

pub fn stamp_plus(grid: &Grid, target_color: u8, stamp_color: u8, radius: usize) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let rows = grid.len();
    let cols = grid[0].len();
    let mut result = grid.clone();
    for r in 0..rows {
        for c in 0..cols {
            if grid[r][c] == target_color {
                for d in 1..=radius {
                    if r >= d { result[r - d][c] = stamp_color; }
                    if r + d < rows { result[r + d][c] = stamp_color; }
                    if c >= d { result[r][c - d] = stamp_color; }
                    if c + d < cols { result[r][c + d] = stamp_color; }
                }
            }
        }
    }
    result
}

pub fn stamp_x(grid: &Grid, target_color: u8, stamp_color: u8, radius: usize) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let rows = grid.len();
    let cols = grid[0].len();
    let mut result = grid.clone();
    for r in 0..rows {
        for c in 0..cols {
            if grid[r][c] == target_color {
                for d in 1..=radius {
                    if r >= d && c >= d { result[r - d][c - d] = stamp_color; }
                    if r >= d && c + d < cols { result[r - d][c + d] = stamp_color; }
                    if r + d < rows && c >= d { result[r + d][c - d] = stamp_color; }
                    if r + d < rows && c + d < cols { result[r + d][c + d] = stamp_color; }
                }
            }
        }
    }
    result
}

pub fn stamp_box(grid: &Grid, target_color: u8, stamp_color: u8, radius: usize) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let (rows, cols) = grid_dimensions(grid);
    let mut result = grid.clone();
    for r in 0..rows {
        for c in 0..cols {
            if grid[r][c] == target_color {
                for dr in -(radius as i32)..=(radius as i32) {
                    for dc in -(radius as i32)..=(radius as i32) {
                        if dr == 0 && dc == 0 { continue; }
                        let nr = r as i32 + dr;
                        let nc = c as i32 + dc;
                        if nr >= 0 && (nr as usize) < rows && nc >= 0 && (nc as usize) < cols {
                            if result[nr as usize][nc as usize] == 0 {
                                result[nr as usize][nc as usize] = stamp_color;
                            }
                        }
                    }
                }
            }
        }
    }
    result
}

// --- Object bounding-box operations ---

pub fn complete_bbox(grid: &Grid) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let mut result = grid.clone();
    let objects = connected_components(grid, true);

    for obj in &objects {
        // Fill the bounding box of each object with its color
        for r in obj.min_r..=obj.max_r {
            for c in obj.min_c..=obj.max_c {
                if result[r][c] == 0 {
                    result[r][c] = obj.color;
                }
            }
        }
    }
    result
}

pub fn draw_bboxes(grid: &Grid, outline_color: u8) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let mut result = grid.clone();
    let objects = connected_components(grid, true);

    for obj in &objects {
        if obj.height() < 2 || obj.width() < 2 { continue; }
        for c in obj.min_c..=obj.max_c {
            result[obj.min_r][c] = outline_color;
            result[obj.max_r][c] = outline_color;
        }
        for r in obj.min_r..=obj.max_r {
            result[r][obj.min_c] = outline_color;
            result[r][obj.max_c] = outline_color;
        }
    }
    result
}

// --- Per-object sorting/alignment ---

pub fn sort_objects_by_size(grid: &Grid) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let (rows, cols) = grid_dimensions(grid);
    let mut objects = connected_components(grid, true);
    objects.sort_by_key(|o| o.area());

    let mut result = vec![vec![0u8; cols]; rows];
    let mut cur_c = 0;
    for obj in &objects {
        let og = obj.to_grid();
        for r in 0..og.len() {
            for c in 0..og[r].len() {
                if og[r][c] != 0 && r < rows && cur_c + c < cols {
                    result[r][cur_c + c] = og[r][c];
                }
            }
        }
        cur_c += obj.width() + 1;
    }
    result
}

// --- Color-conditional per-pixel stamping (learned) ---

#[derive(Debug)]
pub struct StampRule {
    pub trigger_color: u8,
    pub pattern: StampPattern,
    pub stamp_color: u8,
    pub radius: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StampPattern { Plus, X, Box, HLine, VLine }

pub fn try_learn_stamp_rules(examples: &[(Grid, Grid)]) -> Option<Vec<StampRule>> {
    if examples.is_empty() { return None; }
    let (input, output) = &examples[0];
    if input.len() != output.len() || input.is_empty() || input[0].len() != output[0].len() {
        return None;
    }
    let rows = input.len();
    let cols = input[0].len();

    // Find single-pixel markers in input
    let objects = connected_components(input, true);
    let markers: Vec<&Object> = objects.iter().filter(|o| o.area() == 1).collect();
    if markers.is_empty() { return None; }

    let mut rules = Vec::new();
    for marker in &markers {
        let (mr, mc) = marker.cells[0];
        let mc_color = marker.color;

        // Check what pattern appears around this marker in the output
        for &(pattern, _pat_name) in &[
            (StampPattern::Plus, "plus"),
            (StampPattern::X, "x"),
        ] {
            for radius in 1..=5 {
                let _test = match pattern {
                    StampPattern::Plus => stamp_plus(input, mc_color, 0, radius),
                    StampPattern::X => stamp_x(input, mc_color, 0, radius),
                    _ => continue,
                };
                // Find the stamp_color from the output
                let mut stamp_colors = Vec::new();
                match pattern {
                    StampPattern::Plus => {
                        for d in 1..=radius {
                            if mr >= d && output[mr - d][mc] != 0 && output[mr - d][mc] != mc_color {
                                stamp_colors.push(output[mr - d][mc]);
                            }
                            if mr + d < rows && output[mr + d][mc] != 0 && output[mr + d][mc] != mc_color {
                                stamp_colors.push(output[mr + d][mc]);
                            }
                            if mc >= d && output[mr][mc - d] != 0 && output[mr][mc - d] != mc_color {
                                stamp_colors.push(output[mr][mc - d]);
                            }
                            if mc + d < cols && output[mr][mc + d] != 0 && output[mr][mc + d] != mc_color {
                                stamp_colors.push(output[mr][mc + d]);
                            }
                        }
                    }
                    StampPattern::X => {
                        for d in 1..=radius {
                            for &(dr, dc) in &[(-1i32, -1i32), (-1, 1), (1, -1), (1, 1)] {
                                let nr = mr as i32 + dr * d as i32;
                                let nc = mc as i32 + dc * d as i32;
                                if nr >= 0 && (nr as usize) < rows && nc >= 0 && (nc as usize) < cols {
                                    let c = output[nr as usize][nc as usize];
                                    if c != 0 && c != mc_color { stamp_colors.push(c); }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                if !stamp_colors.is_empty() {
                    let sc = stamp_colors[0];
                    if stamp_colors.iter().all(|&c| c == sc) {
                        rules.push(StampRule {
                            trigger_color: mc_color,
                            pattern,
                            stamp_color: sc,
                            radius,
                        });
                        break; // found radius for this pattern
                    }
                }
            }
        }
    }

    if rules.is_empty() { return None; }

    // Verify: apply rules to input and compare with output
    let result = apply_stamp_rules(input, &rules);
    if result == *output {
        // Verify on remaining examples
        let all_ok = examples[1..].iter().all(|(inp, out)| {
            apply_stamp_rules(inp, &rules) == *out
        });
        if all_ok { return Some(rules); }
    }

    None
}

pub fn apply_stamp_rules(grid: &Grid, rules: &[StampRule]) -> Grid {
    let mut result = grid.clone();
    for rule in rules {
        result = match rule.pattern {
            StampPattern::Plus => stamp_plus(&result, rule.trigger_color, rule.stamp_color, rule.radius),
            StampPattern::X => stamp_x(&result, rule.trigger_color, rule.stamp_color, rule.radius),
            StampPattern::Box => stamp_box(&result, rule.trigger_color, rule.stamp_color, rule.radius),
            StampPattern::HLine => extend_markers_to_lines(&result, LineDir::Horizontal),
            StampPattern::VLine => extend_markers_to_lines(&result, LineDir::Vertical),
        };
    }
    result
}

// --- Smart object solver: try all object-based approaches ---

pub fn try_object_solve(examples: &[(Grid, Grid)]) -> Option<ObjectSolution> {
    if examples.is_empty() { return None; }

    // 1. Try stamp rules
    if let Some(rules) = try_learn_stamp_rules(examples) {
        return Some(ObjectSolution::StampRules(rules));
    }

    // 2. Try bbox completion
    {
        let all_ok = examples.iter().all(|(inp, out)| complete_bbox(inp) == *out);
        if all_ok {
            return Some(ObjectSolution::CompleteBBox);
        }
    }

    // 3. Try marker line extension (all directions)
    for dir in [LineDir::Both, LineDir::Horizontal, LineDir::Vertical] {
        let all_ok = examples.iter().all(|(inp, out)| {
            extend_markers_to_lines(inp, dir) == *out
        });
        if all_ok {
            return Some(ObjectSolution::ExtendMarkers(dir));
        }
    }

    None
}

#[derive(Debug)]
pub enum ObjectSolution {
    StampRules(Vec<StampRule>),
    CompleteBBox,
    ExtendMarkers(LineDir),
}

impl ObjectSolution {
    pub fn apply(&self, grid: &Grid) -> Grid {
        match self {
            ObjectSolution::StampRules(rules) => apply_stamp_rules(grid, rules),
            ObjectSolution::CompleteBBox => complete_bbox(grid),
            ObjectSolution::ExtendMarkers(dir) => extend_markers_to_lines(grid, *dir),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            ObjectSolution::StampRules(_) => "stamp_rules",
            ObjectSolution::CompleteBBox => "complete_bbox",
            ObjectSolution::ExtendMarkers(_) => "extend_markers",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_markers_h() {
        let grid = vec![
            vec![0, 0, 0],
            vec![0, 3, 0],
            vec![0, 0, 0],
        ];
        let result = extend_markers_to_lines(&grid, LineDir::Horizontal);
        assert_eq!(result[1], vec![3, 3, 3]);
        assert_eq!(result[0], vec![0, 0, 0]);
    }

    #[test]
    fn extend_markers_v() {
        let grid = vec![
            vec![0, 0, 0],
            vec![0, 3, 0],
            vec![0, 0, 0],
        ];
        let result = extend_markers_to_lines(&grid, LineDir::Vertical);
        assert_eq!(result[0][1], 3);
        assert_eq!(result[1][1], 3);
        assert_eq!(result[2][1], 3);
    }

    #[test]
    fn stamp_plus_basic() {
        let grid = vec![
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 2, 0, 0],
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0],
        ];
        let result = stamp_plus(&grid, 2, 4, 1);
        assert_eq!(result[1][2], 4); // up
        assert_eq!(result[3][2], 4); // down
        assert_eq!(result[2][1], 4); // left
        assert_eq!(result[2][3], 4); // right
        assert_eq!(result[2][2], 2); // center unchanged
    }

    #[test]
    fn stamp_x_basic() {
        let grid = vec![
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 1, 0, 0],
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0],
        ];
        let result = stamp_x(&grid, 1, 7, 1);
        assert_eq!(result[1][1], 7);
        assert_eq!(result[1][3], 7);
        assert_eq!(result[3][1], 7);
        assert_eq!(result[3][3], 7);
    }

    #[test]
    fn complete_bbox_basic() {
        let grid = vec![
            vec![0, 0, 0, 0],
            vec![0, 1, 0, 0],
            vec![0, 0, 0, 0],
            vec![0, 0, 0, 1],
        ];
        let result = complete_bbox(&grid);
        // Object 1 at (1,1), object 2 at (3,3) — separate objects, fill each bbox
        assert_eq!(result[1][1], 1);
        assert_eq!(result[3][3], 1);
    }

    #[test]
    fn draw_bbox_outlines() {
        // Two separate objects with clear bounding boxes
        let grid = vec![
            vec![1, 1, 0, 0, 0],
            vec![1, 1, 0, 0, 0],
            vec![0, 0, 0, 0, 0],
            vec![0, 0, 0, 2, 2],
            vec![0, 0, 0, 2, 2],
        ];
        let result = draw_bboxes(&grid, 5);
        // Object 1 bbox: (0,0)-(1,1), Object 2 bbox: (3,3)-(4,4)
        assert_eq!(result[0][0], 5);
        assert_eq!(result[1][1], 5);
        assert_eq!(result[3][3], 5);
        assert_eq!(result[4][4], 5);
    }

    #[test]
    fn object_solver_finds_bbox() {
        let input = vec![
            vec![0, 0, 0],
            vec![0, 3, 0],
            vec![0, 0, 3],
        ];
        let output = complete_bbox(&input);
        let examples = vec![(input, output)];
        let sol = try_object_solve(&examples);
        assert!(sol.is_some());
        assert_eq!(sol.unwrap().name(), "complete_bbox");
    }
}
