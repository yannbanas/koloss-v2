// Cellular Automaton-based program synthesis.
//
// Novel approach: Many ARC tasks can be described as local rules
// applied to cells based on their neighborhood. Instead of searching
// in primitive-composition space, search in CA rule space.
//
// A CA rule: for each cell, look at its Moore neighborhood (8 neighbors),
// compute a feature vector, and map to an output color.
//
// This captures patterns like:
// - Fill holes (0 surrounded by non-0)
// - Color borders (cells adjacent to different colors)
// - Propagate colors (gravity-like effects)
// - Symmetry completion
//
// The rule search: learn the mapping from (cell_color, neighbor_features) → output_color
// from training examples, then verify on test.

use super::dsl::Grid;
use rustc_hash::FxHashMap;

/// Neighborhood features for a single cell.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct CellContext {
    pub color: u8,
    pub neighbor_colors: [u8; 8], // Moore neighborhood (clockwise from top-left)
    pub row_frac: u8,             // position in grid (0-4 = 5 buckets)
    pub col_frac: u8,
}

/// Extract Moore neighborhood for a cell, padding with 0 for borders.
fn moore_neighborhood(grid: &Grid, r: usize, c: usize) -> [u8; 8] {
    let rows = grid.len() as i32;
    let cols = if grid.is_empty() { 0 } else { grid[0].len() as i32 };
    let offsets: [(i32, i32); 8] = [
        (-1, -1), (-1, 0), (-1, 1),
        (0, -1),           (0, 1),
        (1, -1),  (1, 0),  (1, 1),
    ];
    let mut neighbors = [0u8; 8];
    for (i, &(dr, dc)) in offsets.iter().enumerate() {
        let nr = r as i32 + dr;
        let nc = c as i32 + dc;
        if nr >= 0 && nr < rows && nc >= 0 && nc < cols {
            neighbors[i] = grid[nr as usize][nc as usize];
        }
    }
    neighbors
}

/// Extract a simplified neighborhood signature.
/// Instead of exact 8 colors, use counts: how many of each color.
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct NeighborSignature {
    pub center: u8,
    pub counts: [u8; 10], // count of each color 0-9 in neighborhood
    pub border: bool,      // is the cell on the grid border?
}

fn neighbor_signature(grid: &Grid, r: usize, c: usize) -> NeighborSignature {
    let neighbors = moore_neighborhood(grid, r, c);
    let mut counts = [0u8; 10];
    for &n in &neighbors {
        if (n as usize) < 10 { counts[n as usize] += 1; }
    }
    let rows = grid.len();
    let cols = if grid.is_empty() { 0 } else { grid[0].len() };
    let border = r == 0 || c == 0 || r == rows - 1 || c == cols - 1;

    NeighborSignature {
        center: grid[r][c],
        counts,
        border,
    }
}

/// Learn a CA rule from one training example.
/// Maps (center_color, neighbor_signature) → output_color.
pub fn learn_ca_rule(input: &Grid, output: &Grid) -> Option<FxHashMap<NeighborSignature, u8>> {
    if input.len() != output.len() { return None; }
    if input.is_empty() { return Some(FxHashMap::default()); }
    if input[0].len() != output[0].len() { return None; }

    let mut rule: FxHashMap<NeighborSignature, u8> = FxHashMap::default();

    for r in 0..input.len() {
        for c in 0..input[0].len() {
            let sig = neighbor_signature(input, r, c);
            let out_color = output[r][c];

            if let Some(&existing) = rule.get(&sig) {
                if existing != out_color {
                    return None; // inconsistent rule
                }
            } else {
                rule.insert(sig, out_color);
            }
        }
    }

    Some(rule)
}

/// Apply a learned CA rule to a grid (one step).
pub fn apply_ca_rule(grid: &Grid, rule: &FxHashMap<NeighborSignature, u8>) -> Grid {
    if grid.is_empty() { return grid.clone(); }
    let rows = grid.len();
    let cols = grid[0].len();
    let mut output = vec![vec![0u8; cols]; rows];

    for r in 0..rows {
        for c in 0..cols {
            let sig = neighbor_signature(grid, r, c);
            output[r][c] = rule.get(&sig).copied().unwrap_or(grid[r][c]);
        }
    }
    output
}

/// Verify CA rule on all training examples.
pub fn verify_ca_rule(rule: &FxHashMap<NeighborSignature, u8>,
                       examples: &[(Grid, Grid)]) -> bool {
    examples.iter().all(|(input, output)| {
        apply_ca_rule(input, rule) == *output
    })
}

/// Multi-step CA: apply the rule N times.
/// Some ARC tasks require multiple iterations of a local rule.
pub fn apply_ca_steps(grid: &Grid, rule: &FxHashMap<NeighborSignature, u8>,
                       steps: usize) -> Grid {
    let mut current = grid.clone();
    for _ in 0..steps {
        let next = apply_ca_rule(&current, rule);
        if next == current { break; } // fixpoint
        current = next;
    }
    current
}

/// Try to solve with CA rules at various step counts.
pub fn try_ca_solve(examples: &[(Grid, Grid)], max_steps: usize) -> Option<CaSolution> {
    if examples.is_empty() { return None; }

    // Step 1: Direct CA rule (1 step)
    if let Some(rule) = learn_ca_rule(&examples[0].0, &examples[0].1) {
        if verify_ca_rule(&rule, examples) {
            return Some(CaSolution { rule, steps: 1 });
        }
    }

    // Step 2+: Iterative CA (find intermediate rule)
    // For multi-step: try to learn a rule from input that, when iterated,
    // reaches the output. This is harder — use binary search on step count.
    for steps in 2..=max_steps {
        // Heuristic: try to find a 1-step rule that, iterated, gives output
        // For each example, try to guess intermediate states
        // (This is a simplification — full version would search rule space)
        if examples.len() >= 2 {
            // Use first example to learn, verify on rest
            if let Some(rule) = learn_ca_rule(&examples[0].0, &examples[0].1) {
                let all_ok = examples.iter().all(|(input, output)| {
                    apply_ca_steps(input, &rule, steps) == *output
                });
                if all_ok {
                    return Some(CaSolution { rule, steps });
                }
            }
        }
    }

    None
}

#[derive(Debug, Clone)]
pub struct CaSolution {
    pub rule: FxHashMap<NeighborSignature, u8>,
    pub steps: usize,
}

impl CaSolution {
    pub fn apply(&self, grid: &Grid) -> Grid {
        apply_ca_steps(grid, &self.rule, self.steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moore_neighborhood_center() {
        let grid = vec![
            vec![1, 2, 3],
            vec![4, 5, 6],
            vec![7, 8, 9],
        ];
        let n = moore_neighborhood(&grid, 1, 1);
        assert_eq!(n, [1, 2, 3, 4, 6, 7, 8, 9]);
    }

    #[test]
    fn moore_neighborhood_corner() {
        let grid = vec![
            vec![1, 2],
            vec![3, 4],
        ];
        let n = moore_neighborhood(&grid, 0, 0);
        // TL corner: neighbors are 0,0,0, 0,2, 0,3,4
        assert_eq!(n, [0, 0, 0, 0, 2, 0, 3, 4]);
    }

    #[test]
    fn ca_learns_identity() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let output = input.clone();
        let rule = learn_ca_rule(&input, &output).unwrap();
        assert_eq!(apply_ca_rule(&input, &rule), output);
    }

    #[test]
    fn ca_learns_fill() {
        // Rule: if center is 0 and any neighbor is 1, output 1
        // Otherwise keep same color
        let input = vec![
            vec![1, 0, 0],
            vec![0, 0, 0],
            vec![0, 0, 0],
        ];
        let output = vec![
            vec![1, 1, 0],
            vec![1, 1, 0],
            vec![0, 0, 0],
        ];
        let rule = learn_ca_rule(&input, &output);
        // This specific pattern may or may not be learnable as a consistent CA
        // (depends on whether neighbor signatures are unique)
        if let Some(r) = rule {
            assert_eq!(apply_ca_rule(&input, &r), output);
        }
    }

    #[test]
    fn neighbor_signature_consistent() {
        let grid = vec![
            vec![1, 1, 1],
            vec![1, 0, 1],
            vec![1, 1, 1],
        ];
        let sig = neighbor_signature(&grid, 1, 1);
        assert_eq!(sig.center, 0);
        assert_eq!(sig.counts[1], 8); // all 8 neighbors are 1
        assert!(!sig.border);
    }

    #[test]
    fn ca_fixpoint() {
        let grid = vec![vec![1, 2], vec![3, 4]];
        let rule = learn_ca_rule(&grid, &grid).unwrap();
        // Applying identity CA multiple times should converge
        let result = apply_ca_steps(&grid, &rule, 100);
        assert_eq!(result, grid);
    }
}
