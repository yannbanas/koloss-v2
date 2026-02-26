// Bidirectional DAG search for ARC program synthesis.
//
// Key insight: Instead of searching only forward (input → ???),
// also search backward (output ← ???) using inverse primitives.
// When a forward state matches a backward state, we found a path.
//
// Complexity: O(2 * b^(d/2)) instead of O(b^d) — exponential speedup.
// For b=30, d=4: naive = 810,000 states, bidir = 1,800 states.
//
// Not all primitives are invertible, but many ARC-relevant ones are:
// - RotateCW ↔ RotateCCW
// - FlipH ↔ FlipH (self-inverse)
// - ReplaceColor(a,b) ↔ ReplaceColor(b,a)
//
// For non-invertible primitives, we only search forward.
// The backward frontier uses only invertible primitives.

use super::dsl::{Prim, Grid};
use rustc_hash::FxHashMap;

/// Get the inverse of a primitive, if it exists.
/// Returns None for non-invertible operations (lossy transforms).
pub fn inverse(prim: &Prim) -> Option<Prim> {
    match prim {
        // Rotations
        Prim::RotateCW => Some(Prim::RotateCCW),
        Prim::RotateCCW => Some(Prim::RotateCW),
        Prim::Rotate180 => Some(Prim::Rotate180),

        // Self-inverse transforms
        Prim::FlipH => Some(Prim::FlipH),
        Prim::FlipV => Some(Prim::FlipV),
        Prim::Transpose => Some(Prim::Transpose),
        Prim::Invert => Some(Prim::Invert),
        Prim::Identity => Some(Prim::Identity),

        // Color swaps (bijection)
        Prim::ReplaceColor(a, b) => Some(Prim::ReplaceColor(*b, *a)),

        // Scale 2 → can't truly invert (lossy), but for grid matching purposes
        // we know the expected dimensions, so it's informational

        // Everything else is lossy (gravity, filter, fill, etc.)
        _ => None,
    }
}

/// Collect all invertible primitives from a set.
pub fn invertible_subset(prims: &[Prim]) -> Vec<(Prim, Prim)> {
    prims.iter()
        .filter_map(|p| inverse(p).map(|inv| (p.clone(), inv)))
        .collect()
}

#[derive(Debug, Clone)]
struct BidirNode {
    grid: Grid,
    program: Prim,
    depth: usize,
}

#[derive(Debug)]
pub struct BidirSearch {
    max_nodes: usize,
}

#[derive(Debug, Clone)]
pub struct BidirResult {
    pub program: Prim,
    pub method: &'static str,
    pub forward_depth: usize,
    pub backward_depth: usize,
    pub nodes_explored: usize,
}

impl BidirSearch {
    pub fn new(max_nodes: usize) -> Self {
        Self { max_nodes }
    }

    /// Bidirectional search: expand forward from input AND backward from output.
    /// Meet in the middle when grids match.
    pub fn search(
        &self,
        input: &Grid,
        target: &Grid,
        forward_prims: &[Prim],
        max_depth: usize,
    ) -> Option<BidirResult> {
        // Identity check
        if input == target {
            return Some(BidirResult {
                program: Prim::Identity,
                method: "identity",
                forward_depth: 0,
                backward_depth: 0,
                nodes_explored: 0,
            });
        }

        // Separate invertible primitives for backward search
        let inv_pairs = invertible_subset(forward_prims);
        let backward_prims: Vec<(Prim, Prim)> = inv_pairs; // (forward, inverse)

        // Forward frontier: grid → (program, depth)
        let mut forward: FxHashMap<u64, BidirNode> = FxHashMap::default();
        let mut backward: FxHashMap<u64, BidirNode> = FxHashMap::default();

        let input_fp = grid_hash(input);
        let target_fp = grid_hash(target);

        forward.insert(input_fp, BidirNode {
            grid: input.clone(),
            program: Prim::Identity,
            depth: 0,
        });

        backward.insert(target_fp, BidirNode {
            grid: target.clone(),
            program: Prim::Identity,
            depth: 0,
        });

        let mut total_nodes = 2;
        let half_depth = (max_depth + 1) / 2;

        // Alternate forward and backward expansion
        for depth in 0..half_depth {
            // Forward expansion
            if let Some(result) = self.expand_forward(
                &mut forward, &backward, forward_prims, depth, &mut total_nodes,
            ) {
                return Some(result);
            }

            // Backward expansion (using inverse primitives)
            if !backward_prims.is_empty() {
                if let Some(result) = self.expand_backward(
                    &forward, &mut backward, &backward_prims, depth, &mut total_nodes,
                ) {
                    return Some(result);
                }
            }

            if total_nodes >= self.max_nodes {
                break;
            }
        }

        None
    }

    fn expand_forward(
        &self,
        forward: &mut FxHashMap<u64, BidirNode>,
        backward: &FxHashMap<u64, BidirNode>,
        prims: &[Prim],
        depth: usize,
        total_nodes: &mut usize,
    ) -> Option<BidirResult> {
        let current: Vec<(u64, Grid, Prim)> = forward.iter()
            .filter(|(_, n)| n.depth == depth)
            .map(|(k, n)| (*k, n.grid.clone(), n.program.clone()))
            .collect();

        for (_fp, grid, prog) in &current {
            for prim in prims {
                let result = prim.apply(grid);
                let result_fp = grid_hash(&result);

                // Check if backward frontier reached this state
                if let Some(back_node) = backward.get(&result_fp) {
                    // Verify actual grid equality (hash collision check)
                    if result == back_node.grid {
                        let forward_prog = compose_programs(prog, prim);
                        let full_prog = if back_node.depth == 0 {
                            forward_prog
                        } else {
                            // Compose forward path with inverse of backward path
                            Prim::Compose(
                                Box::new(forward_prog),
                                Box::new(invert_program(&back_node.program)),
                            )
                        };
                        return Some(BidirResult {
                            program: full_prog,
                            method: "bidirectional",
                            forward_depth: depth + 1,
                            backward_depth: back_node.depth,
                            nodes_explored: *total_nodes,
                        });
                    }
                }

                // Skip duplicates in forward set
                if forward.contains_key(&result_fp) { continue; }

                // Skip if grid unchanged
                if result == *grid { continue; }

                let new_prog = compose_programs(prog, prim);
                forward.insert(result_fp, BidirNode {
                    grid: result,
                    program: new_prog,
                    depth: depth + 1,
                });
                *total_nodes += 1;

                if *total_nodes >= self.max_nodes {
                    return None;
                }
            }
        }
        None
    }

    fn expand_backward(
        &self,
        forward: &FxHashMap<u64, BidirNode>,
        backward: &mut FxHashMap<u64, BidirNode>,
        inv_prims: &[(Prim, Prim)],
        depth: usize,
        total_nodes: &mut usize,
    ) -> Option<BidirResult> {
        let current: Vec<(u64, Grid, Prim)> = backward.iter()
            .filter(|(_, n)| n.depth == depth)
            .map(|(k, n)| (*k, n.grid.clone(), n.program.clone()))
            .collect();

        for (_fp, grid, back_prog) in &current {
            for (forward_prim, inv_prim) in inv_prims {
                // Apply inverse to go backward from target
                let result = inv_prim.apply(grid);
                let result_fp = grid_hash(&result);

                // Check if forward frontier reached this state
                if let Some(fwd_node) = forward.get(&result_fp) {
                    if result == fwd_node.grid {
                        // Build the forward primitive path
                        let back_forward = compose_programs(back_prog, forward_prim);
                        let full_prog = if fwd_node.depth == 0 {
                            invert_program(&back_forward)
                        } else {
                            Prim::Compose(
                                Box::new(fwd_node.program.clone()),
                                Box::new(invert_program(&back_forward)),
                            )
                        };
                        return Some(BidirResult {
                            program: full_prog,
                            method: "bidirectional",
                            forward_depth: fwd_node.depth,
                            backward_depth: depth + 1,
                            nodes_explored: *total_nodes,
                        });
                    }
                }

                if backward.contains_key(&result_fp) { continue; }
                if result == *grid { continue; }

                // Track which forward primitive was used (for reconstruction)
                let new_back_prog = compose_programs(back_prog, forward_prim);
                backward.insert(result_fp, BidirNode {
                    grid: result,
                    program: new_back_prog,
                    depth: depth + 1,
                });
                *total_nodes += 1;

                if *total_nodes >= self.max_nodes {
                    return None;
                }
            }
        }
        None
    }

    /// Multi-example search: find a program that works for all examples.
    pub fn search_all(
        &self,
        examples: &[(Grid, Grid)],
        prims: &[Prim],
        max_depth: usize,
    ) -> Option<BidirResult> {
        if examples.is_empty() { return None; }
        if examples.len() == 1 {
            return self.search(&examples[0].0, &examples[0].1, prims, max_depth);
        }

        // Strategy: solve first example, verify against rest
        let result = self.search(&examples[0].0, &examples[0].1, prims, max_depth)?;

        // Verify on all other examples
        let all_match = examples[1..].iter().all(|(input, output)| {
            result.program.apply(input) == *output
        });

        if all_match { Some(result) } else { None }
    }
}

/// Compose two programs into a sequence.
fn compose_programs(existing: &Prim, next: &Prim) -> Prim {
    match existing {
        Prim::Identity => next.clone(),
        _ => Prim::Compose(Box::new(existing.clone()), Box::new(next.clone())),
    }
}

/// Invert a program by reversing composition order and inverting each step.
fn invert_program(prog: &Prim) -> Prim {
    match prog {
        Prim::Compose(a, b) => {
            let inv_a = invert_program(a);
            let inv_b = invert_program(b);
            Prim::Compose(Box::new(inv_b), Box::new(inv_a))
        }
        other => inverse(other).unwrap_or_else(|| other.clone()),
    }
}

fn grid_hash(grid: &Grid) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for (r, row) in grid.iter().enumerate() {
        for (c, &val) in row.iter().enumerate() {
            let cell = (r as u64).wrapping_mul(0x517cc1b727220a95)
                ^ (c as u64).wrapping_mul(0x6c62272e07bb0142)
                ^ (val as u64);
            h = h.wrapping_mul(0x100000001b3) ^ cell;
        }
    }
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse_rotate() {
        assert_eq!(inverse(&Prim::RotateCW), Some(Prim::RotateCCW));
        assert_eq!(inverse(&Prim::RotateCCW), Some(Prim::RotateCW));
        assert_eq!(inverse(&Prim::Rotate180), Some(Prim::Rotate180));
    }

    #[test]
    fn inverse_self_inverse() {
        assert_eq!(inverse(&Prim::FlipH), Some(Prim::FlipH));
        assert_eq!(inverse(&Prim::FlipV), Some(Prim::FlipV));
        assert_eq!(inverse(&Prim::Transpose), Some(Prim::Transpose));
        assert_eq!(inverse(&Prim::Invert), Some(Prim::Invert));
    }

    #[test]
    fn inverse_color_swap() {
        assert_eq!(inverse(&Prim::ReplaceColor(1, 2)), Some(Prim::ReplaceColor(2, 1)));
    }

    #[test]
    fn inverse_lossy_returns_none() {
        assert_eq!(inverse(&Prim::GravityDown), None);
        assert_eq!(inverse(&Prim::KeepLargestObject), None);
        assert_eq!(inverse(&Prim::FillColor(1)), None);
    }

    #[test]
    fn bidir_finds_identity() {
        let grid = vec![vec![1, 2], vec![3, 4]];
        let bidir = BidirSearch::new(1000);
        let prims = vec![Prim::RotateCW, Prim::FlipH];
        let result = bidir.search(&grid, &grid, &prims, 4);
        assert!(result.is_some());
        assert_eq!(result.unwrap().program, Prim::Identity);
    }

    #[test]
    fn bidir_finds_single_step() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let target = Prim::FlipH.apply(&input);
        let bidir = BidirSearch::new(1000);
        let prims = vec![Prim::RotateCW, Prim::FlipH, Prim::FlipV, Prim::Transpose];
        let result = bidir.search(&input, &target, &prims, 4);
        assert!(result.is_some());
        let res = result.unwrap();
        assert_eq!(res.program.apply(&input), target);
    }

    #[test]
    fn bidir_finds_two_step() {
        let input = vec![vec![1, 2, 3], vec![4, 5, 6]];
        // FlipH then FlipV
        let mid = Prim::FlipH.apply(&input);
        let target = Prim::FlipV.apply(&mid);
        let bidir = BidirSearch::new(5000);
        let prims = vec![Prim::RotateCW, Prim::RotateCCW, Prim::FlipH, Prim::FlipV,
                         Prim::Transpose, Prim::Rotate180];
        let result = bidir.search(&input, &target, &prims, 4);
        assert!(result.is_some());
        let res = result.unwrap();
        assert_eq!(res.program.apply(&input), target);
    }

    #[test]
    fn bidir_multi_example() {
        // FlipH should work on both
        let ex1_in = vec![vec![1, 2], vec![3, 4]];
        let ex1_out = Prim::FlipH.apply(&ex1_in);
        let ex2_in = vec![vec![5, 6], vec![7, 8]];
        let ex2_out = Prim::FlipH.apply(&ex2_in);

        let bidir = BidirSearch::new(5000);
        let prims = vec![Prim::FlipH, Prim::FlipV, Prim::RotateCW];
        let result = bidir.search_all(&[(ex1_in, ex1_out), (ex2_in, ex2_out)], &prims, 4);
        assert!(result.is_some());
    }

    #[test]
    fn invertible_subset_filters() {
        let prims = vec![Prim::RotateCW, Prim::GravityDown, Prim::FlipH, Prim::FillColor(1)];
        let inv = invertible_subset(&prims);
        assert_eq!(inv.len(), 2); // RotateCW and FlipH
    }
}
