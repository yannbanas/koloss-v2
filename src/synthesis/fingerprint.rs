// Fast grid fingerprinting via polynomial rolling hash.
// Replaces O(rows*cols) grid comparison with O(1) hash lookup.
// Uses FxHash internally for speed, with collision resistance from
// mixing row/col position into the hash.
//
// Novel approach: multi-resolution fingerprints for approximate matching.
// Level 0 = full grid hash, Level 1 = quadrant hashes, Level 2 = color histogram.
// This enables "fuzzy dedup" in DAG search — skip states that are
// structurally similar even if not pixel-identical.

use super::dsl::Grid;

const MIX_A: u64 = 0x517cc1b727220a95;
const MIX_B: u64 = 0x6c62272e07bb0142;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GridFingerprint {
    pub full: u64,
    pub shape: u32,     // (rows << 16) | cols
    pub color_sig: u32, // color histogram signature
}

impl GridFingerprint {
    pub fn compute(grid: &Grid) -> Self {
        let full = hash_grid(grid);
        let shape = grid_shape(grid);
        let color_sig = color_signature(grid);
        Self { full, shape, color_sig }
    }

    pub fn same_shape(&self, other: &GridFingerprint) -> bool {
        self.shape == other.shape
    }

    pub fn same_colors(&self, other: &GridFingerprint) -> bool {
        self.color_sig == other.color_sig
    }

    /// Approximate structural similarity without full grid comparison.
    /// Returns true if grids have same shape AND similar color distribution.
    pub fn structurally_similar(&self, other: &GridFingerprint) -> bool {
        self.shape == other.shape && self.color_sig == other.color_sig
    }
}

fn hash_grid(grid: &Grid) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV offset basis
    for (r, row) in grid.iter().enumerate() {
        for (c, &val) in row.iter().enumerate() {
            // Mix position + value into hash
            let cell = (r as u64).wrapping_mul(MIX_A)
                ^ (c as u64).wrapping_mul(MIX_B)
                ^ (val as u64);
            h = h.wrapping_mul(0x100000001b3) ^ cell; // FNV prime
        }
    }
    h
}

fn grid_shape(grid: &Grid) -> u32 {
    let rows = grid.len() as u32;
    let cols = if grid.is_empty() { 0 } else { grid[0].len() as u32 };
    (rows << 16) | cols
}

/// Color histogram compressed to 32 bits.
/// Each of colors 0-9 gets 3 bits (0-7 = log2 bucket of count).
/// Remaining 2 bits = total unique color count (mod 4).
fn color_signature(grid: &Grid) -> u32 {
    let mut counts = [0u32; 10];
    let mut unique = 0u8;
    for row in grid {
        for &c in row {
            if (c as usize) < 10 {
                if counts[c as usize] == 0 { unique += 1; }
                counts[c as usize] += 1;
            }
        }
    }

    let mut sig: u32 = 0;
    for i in 0..10 {
        let bucket = if counts[i] == 0 { 0 }
            else { (counts[i] as f64).log2().min(7.0) as u32 };
        sig |= bucket << (i * 3);
    }
    sig |= ((unique & 3) as u32) << 30;
    sig
}

/// Multi-resolution fingerprint for hierarchical matching.
/// Computes fingerprints at different spatial resolutions.
#[derive(Debug, Clone)]
pub struct MultiResFingerprint {
    pub full: GridFingerprint,
    pub quadrants: [u64; 4], // TL, TR, BL, BR
}

impl MultiResFingerprint {
    pub fn compute(grid: &Grid) -> Self {
        let full = GridFingerprint::compute(grid);
        let quadrants = quadrant_hashes(grid);
        Self { full, quadrants }
    }

    /// Similarity score [0, 1] based on quadrant matching.
    pub fn similarity(&self, other: &MultiResFingerprint) -> f64 {
        if self.full.full == other.full.full { return 1.0; }
        if !self.full.same_shape(&other.full) { return 0.0; }

        let matching = self.quadrants.iter().zip(other.quadrants.iter())
            .filter(|(a, b)| a == b)
            .count();
        matching as f64 / 4.0
    }
}

fn quadrant_hashes(grid: &Grid) -> [u64; 4] {
    if grid.is_empty() { return [0; 4]; }
    let rows = grid.len();
    let cols = grid[0].len();
    let mid_r = rows / 2;
    let mid_c = cols / 2;

    let hash_region = |r_start: usize, r_end: usize, c_start: usize, c_end: usize| -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;
        for r in r_start..r_end.min(rows) {
            for c in c_start..c_end.min(cols) {
                let cell = (r as u64).wrapping_mul(MIX_A)
                    ^ (c as u64).wrapping_mul(MIX_B)
                    ^ (grid[r][c] as u64);
                h = h.wrapping_mul(0x100000001b3) ^ cell;
            }
        }
        h
    };

    [
        hash_region(0, mid_r, 0, mid_c),         // TL
        hash_region(0, mid_r, mid_c, cols),       // TR
        hash_region(mid_r, rows, 0, mid_c),       // BL
        hash_region(mid_r, rows, mid_c, cols),    // BR
    ]
}

/// Deduplication set using fingerprints instead of full grid comparison.
/// O(1) insert + lookup vs O(n * rows * cols) for naive approach.
pub struct FingerprintSet {
    seen: rustc_hash::FxHashSet<u64>,
}

impl FingerprintSet {
    pub fn new() -> Self {
        Self { seen: rustc_hash::FxHashSet::default() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self { seen: rustc_hash::FxHashSet::with_capacity_and_hasher(cap, Default::default()) }
    }

    /// Returns true if this is a new grid (not seen before).
    pub fn insert(&mut self, grid: &Grid) -> bool {
        let fp = hash_grid(grid);
        self.seen.insert(fp)
    }

    pub fn contains(&self, grid: &Grid) -> bool {
        let fp = hash_grid(grid);
        self.seen.contains(&fp)
    }

    pub fn len(&self) -> usize {
        self.seen.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_identical_grids() {
        let g1 = vec![vec![1, 2], vec![3, 4]];
        let g2 = vec![vec![1, 2], vec![3, 4]];
        let fp1 = GridFingerprint::compute(&g1);
        let fp2 = GridFingerprint::compute(&g2);
        assert_eq!(fp1.full, fp2.full);
        assert_eq!(fp1.shape, fp2.shape);
    }

    #[test]
    fn fingerprint_different_grids() {
        let g1 = vec![vec![1, 2], vec![3, 4]];
        let g2 = vec![vec![1, 2], vec![3, 5]];
        let fp1 = GridFingerprint::compute(&g1);
        let fp2 = GridFingerprint::compute(&g2);
        assert_ne!(fp1.full, fp2.full);
        assert_eq!(fp1.shape, fp2.shape); // same dimensions
    }

    #[test]
    fn fingerprint_different_shapes() {
        let g1 = vec![vec![1, 2, 3]];
        let g2 = vec![vec![1], vec![2], vec![3]];
        let fp1 = GridFingerprint::compute(&g1);
        let fp2 = GridFingerprint::compute(&g2);
        assert!(!fp1.same_shape(&fp2));
    }

    #[test]
    fn color_signature_same_histogram() {
        let g1 = vec![vec![1, 2, 1], vec![2, 1, 2]];
        let g2 = vec![vec![2, 1, 2], vec![1, 2, 1]];
        let fp1 = GridFingerprint::compute(&g1);
        let fp2 = GridFingerprint::compute(&g2);
        assert_eq!(fp1.color_sig, fp2.color_sig);
    }

    #[test]
    fn fingerprint_set_dedup() {
        let g1 = vec![vec![1, 2], vec![3, 4]];
        let g2 = vec![vec![1, 2], vec![3, 4]];
        let g3 = vec![vec![5, 6], vec![7, 8]];

        let mut set = FingerprintSet::new();
        assert!(set.insert(&g1));   // new
        assert!(!set.insert(&g2));  // duplicate
        assert!(set.insert(&g3));   // new
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn multi_res_self_similarity() {
        let g = vec![vec![1, 2, 3], vec![4, 5, 6], vec![7, 8, 9]];
        let mr = MultiResFingerprint::compute(&g);
        assert_eq!(mr.similarity(&mr), 1.0);
    }

    #[test]
    fn multi_res_different() {
        let g1 = vec![vec![1, 2], vec![3, 4]];
        let g2 = vec![vec![5, 6], vec![7, 8]];
        let mr1 = MultiResFingerprint::compute(&g1);
        let mr2 = MultiResFingerprint::compute(&g2);
        assert!(mr1.similarity(&mr2) < 1.0);
    }

    #[test]
    fn empty_grid_fingerprint() {
        let g: Grid = Vec::new();
        let fp = GridFingerprint::compute(&g);
        assert_eq!(fp.shape, 0);
    }
}
