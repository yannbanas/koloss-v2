// Compression-based reasoning for program synthesis (MDL principle).
//
// Key insight from Kolmogorov complexity:
// The best program to describe a transformation is the SHORTEST one.
// We don't just find ANY correct program — we find the simplest.
//
// Approach:
// 1. Estimate the "description length" of input→output mapping
// 2. Search programs by increasing description length
// 3. Prefer programs with lower total description cost
//
// This naturally avoids overfitting (a program that memorizes examples
// has high description length, while one that captures the pattern is short).
//
// Also implements: delta encoding between grids (for efficient caching)
// and run-length encoding for grid storage.

use super::dsl::{Grid, Prim};

/// Compute description length of a grid transformation.
/// Lower = simpler, more compressible.
pub fn description_length(program: &Prim) -> f64 {
    // Cost model: each primitive costs log2(num_variants) bits
    // Compositions cost extra for the combiner node
    match program {
        Prim::Identity => 0.0,
        Prim::Compose(a, b) => {
            1.0 + description_length(a) + description_length(b)
        }
        Prim::Conditional(a, b, c) => {
            2.0 + description_length(a) + description_length(b) + description_length(c)
        }
        // Simple transforms: ~4 bits (16 basic ops)
        Prim::RotateCW | Prim::RotateCCW | Prim::Rotate180
        | Prim::FlipH | Prim::FlipV | Prim::Transpose
        | Prim::GravityDown | Prim::GravityUp
        | Prim::GravityLeft | Prim::GravityRight
        | Prim::Invert | Prim::SortRowsByColor | Prim::SortColsByColor
        | Prim::KeepLargestObject | Prim::KeepSmallestObject
        | Prim::MirrorH | Prim::MirrorV | Prim::Overlay
        | Prim::MostFrequentColor => 4.0,

        // Parameterized transforms: op cost + param cost
        Prim::FillColor(_) | Prim::FilterColor(_)
        | Prim::RemoveColor(_) | Prim::BorderFill(_) => 4.0 + 3.3, // ~log2(10)

        Prim::ReplaceColor(_, _) => 4.0 + 6.6, // 2 color params
        Prim::OutlineObjects(_) | Prim::FillInsideObjects(_) => 4.0 + 3.3,

        Prim::Crop(_, _, _, _) => 4.0 + 12.0, // 4 params
        Prim::Pad(_, _) => 4.0 + 6.0,
        Prim::Scale(_) | Prim::RepeatH(_) | Prim::RepeatV(_) => 4.0 + 2.0,
        Prim::FloodFill(_, _, _) => 4.0 + 9.0,
        Prim::ExtractObject(_) => 4.0 + 3.0,
    }
}

/// MDL score: balance program simplicity with accuracy.
/// `mdl_score = -log P(examples | program) + description_length(program)`
/// Lower MDL = better program.
pub fn mdl_score(program: &Prim, examples: &[(Grid, Grid)]) -> f64 {
    let dl = description_length(program);
    let fit = data_fit(program, examples);
    dl + fit
}

/// Data fit: how well does the program explain the examples?
/// Returns 0 for perfect fit, positive for errors.
fn data_fit(program: &Prim, examples: &[(Grid, Grid)]) -> f64 {
    let mut total_error = 0.0;
    for (input, expected) in examples {
        let result = program.apply(input);
        total_error += grid_error(&result, expected);
    }
    total_error
}

/// Error between two grids in bits.
fn grid_error(actual: &Grid, expected: &Grid) -> f64 {
    if actual == expected { return 0.0; }

    // Dimension mismatch: heavy penalty
    if actual.len() != expected.len() {
        return 100.0;
    }
    if actual.is_empty() { return 0.0; }
    if actual[0].len() != expected[0].len() {
        return 100.0;
    }

    // Per-cell error (each wrong cell costs log2(10) ≈ 3.3 bits)
    let wrong = actual.iter().zip(expected.iter())
        .flat_map(|(ar, er)| ar.iter().zip(er.iter()))
        .filter(|(a, e)| a != e)
        .count();

    wrong as f64 * 3.3
}

// --- Grid compression utilities ---

/// Run-length encode a grid row. Good for ARC grids (lots of repeats).
pub fn rle_encode(row: &[u8]) -> Vec<(u8, u16)> {
    if row.is_empty() { return Vec::new(); }
    let mut runs = Vec::new();
    let mut current = row[0];
    let mut count: u16 = 1;

    for &val in &row[1..] {
        if val == current && count < u16::MAX {
            count += 1;
        } else {
            runs.push((current, count));
            current = val;
            count = 1;
        }
    }
    runs.push((current, count));
    runs
}

pub fn rle_decode(runs: &[(u8, u16)]) -> Vec<u8> {
    let mut row = Vec::new();
    for &(val, count) in runs {
        for _ in 0..count {
            row.push(val);
        }
    }
    row
}

/// Delta-encode: represent one grid as diff from another.
/// Useful for caching DAG search states compactly.
pub fn delta_encode(base: &Grid, target: &Grid) -> Vec<(u16, u16, u8)> {
    let mut diffs = Vec::new();
    for (r, (br, tr)) in base.iter().zip(target.iter()).enumerate() {
        for (c, (&bv, &tv)) in br.iter().zip(tr.iter()).enumerate() {
            if bv != tv {
                diffs.push((r as u16, c as u16, tv));
            }
        }
    }
    diffs
}

pub fn delta_apply(base: &Grid, diffs: &[(u16, u16, u8)]) -> Grid {
    let mut result = base.clone();
    for &(r, c, v) in diffs {
        if (r as usize) < result.len() {
            if let Some(row) = result.get_mut(r as usize) {
                if (c as usize) < row.len() {
                    row[c as usize] = v;
                }
            }
        }
    }
    result
}

/// Compute compression ratio of a grid (RLE bytes vs raw bytes).
pub fn compression_ratio(grid: &Grid) -> f64 {
    if grid.is_empty() { return 1.0; }
    let raw_size: usize = grid.iter().map(|r| r.len()).sum();
    let rle_size: usize = grid.iter().map(|r| rle_encode(r).len() * 3).sum(); // 3 bytes per run
    if raw_size == 0 { return 1.0; }
    rle_size as f64 / raw_size as f64
}

/// Information content of a grid (Shannon entropy in bits per cell).
pub fn grid_entropy(grid: &Grid) -> f64 {
    let mut counts = [0u64; 256];
    let mut total = 0u64;
    for row in grid {
        for &c in row {
            counts[c as usize] += 1;
            total += 1;
        }
    }
    if total == 0 { return 0.0; }

    let mut entropy = 0.0;
    for &count in &counts {
        if count > 0 {
            let p = count as f64 / total as f64;
            entropy -= p * p.log2();
        }
    }
    entropy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_zero_description_length() {
        assert_eq!(description_length(&Prim::Identity), 0.0);
    }

    #[test]
    fn compose_longer_than_parts() {
        let a = Prim::FlipH;
        let b = Prim::RotateCW;
        let composed = Prim::Compose(Box::new(a.clone()), Box::new(b.clone()));
        assert!(description_length(&composed) > description_length(&a));
        assert!(description_length(&composed) > description_length(&b));
    }

    #[test]
    fn parameterized_more_expensive() {
        let simple = Prim::FlipH;
        let param = Prim::ReplaceColor(1, 2);
        assert!(description_length(&param) > description_length(&simple));
    }

    #[test]
    fn mdl_prefers_simpler() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let output = Prim::FlipH.apply(&input);
        let examples = vec![(input, output)];

        let simple = Prim::FlipH;
        let complex = Prim::Compose(Box::new(Prim::FlipH), Box::new(Prim::Identity));
        assert!(mdl_score(&simple, &examples) <= mdl_score(&complex, &examples));
    }

    #[test]
    fn rle_roundtrip() {
        let row = vec![1, 1, 1, 2, 2, 3, 3, 3, 3];
        let encoded = rle_encode(&row);
        let decoded = rle_decode(&encoded);
        assert_eq!(row, decoded);
    }

    #[test]
    fn rle_single_values() {
        let row = vec![1, 2, 3, 4, 5];
        let encoded = rle_encode(&row);
        assert_eq!(encoded.len(), 5); // no compression
        assert_eq!(rle_decode(&encoded), row);
    }

    #[test]
    fn rle_uniform() {
        let row = vec![7; 100];
        let encoded = rle_encode(&row);
        assert_eq!(encoded.len(), 1); // single run
        assert_eq!(rle_decode(&encoded), row);
    }

    #[test]
    fn delta_encode_roundtrip() {
        let base = vec![vec![1, 2], vec![3, 4]];
        let target = vec![vec![1, 5], vec![3, 4]]; // one cell changed
        let diffs = delta_encode(&base, &target);
        assert_eq!(diffs.len(), 1);
        assert_eq!(delta_apply(&base, &diffs), target);
    }

    #[test]
    fn delta_identical_no_diffs() {
        let g = vec![vec![1, 2], vec![3, 4]];
        let diffs = delta_encode(&g, &g);
        assert!(diffs.is_empty());
    }

    #[test]
    fn compression_ratio_uniform() {
        let grid = vec![vec![0; 10]; 10]; // all zeros
        let ratio = compression_ratio(&grid);
        assert!(ratio < 0.5); // should compress well
    }

    #[test]
    fn entropy_uniform() {
        let grid = vec![vec![5; 10]; 10]; // all same color
        let e = grid_entropy(&grid);
        assert!(e < 0.01); // near zero entropy
    }

    #[test]
    fn entropy_mixed() {
        let grid = vec![vec![1, 2, 3, 4]]; // 4 distinct colors
        let e = grid_entropy(&grid);
        assert!(e > 1.0); // should have significant entropy
    }

    #[test]
    fn grid_error_identical() {
        let g = vec![vec![1, 2], vec![3, 4]];
        assert_eq!(grid_error(&g, &g), 0.0);
    }

    #[test]
    fn grid_error_dimension_mismatch() {
        let a = vec![vec![1, 2]];
        let b = vec![vec![1, 2], vec![3, 4]];
        assert!(grid_error(&a, &b) > 50.0); // heavy penalty
    }
}
