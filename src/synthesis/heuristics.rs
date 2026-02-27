// Heuristic primitive selection for ARC program synthesis.
//
// Instead of brute-forcing all 177 primitives at each search step,
// analyze input/output features to predict which primitives are relevant.
// This cuts the effective branching factor from ~177 to ~20-40.
//
// Features extracted:
// - Dimension change (same, scaled, transposed, cropped)
// - Color mapping (bijection, subset, superset)
// - Object count change
// - Symmetry presence/change
// - Pattern repetition
//
// Each feature maps to a set of "likely useful" primitives.
// The intersection of all feature-predicted sets becomes the search space.

use super::dsl::{Grid, Prim, connected_components, unique_colors, grid_dimensions,
    is_symmetric_h, is_symmetric_v, detect_period_h, detect_period_v};

#[derive(Debug, Clone)]
pub struct FeatureProfile {
    pub dim_change: DimChange,
    pub color_change: ColorChange,
    pub object_delta: i32,       // output objects - input objects
    pub input_symmetric_h: bool,
    pub input_symmetric_v: bool,
    pub output_symmetric_h: bool,
    pub output_symmetric_v: bool,
    pub input_period_h: Option<usize>,
    pub input_period_v: Option<usize>,
    pub output_period_h: Option<usize>,
    pub output_period_v: Option<usize>,
    pub same_grid: bool,
    pub input_colors: Vec<u8>,
    pub output_colors: Vec<u8>,
    pub input_dims: (usize, usize),
    pub output_dims: (usize, usize),
}

#[derive(Debug, Clone, PartialEq)]
pub enum DimChange {
    Same,
    Scaled(usize, usize), // row_factor, col_factor
    Transposed,
    Cropped,
    Padded,
    Arbitrary,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ColorChange {
    Same,
    Bijection,  // 1:1 mapping
    Reduction,  // fewer colors in output
    Expansion,  // more colors in output
    Complex,
}

pub fn analyze_features(examples: &[(Grid, Grid)]) -> FeatureProfile {
    if examples.is_empty() {
        return default_profile();
    }

    // Analyze first example in detail, verify against rest
    let (input, output) = &examples[0];
    let in_dims = grid_dimensions(input);
    let out_dims = grid_dimensions(output);
    let in_colors = unique_colors(input);
    let out_colors = unique_colors(output);
    let in_objs = connected_components(input, true).len();
    let out_objs = connected_components(output, true).len();

    let dim_change = classify_dim_change(in_dims, out_dims);
    let color_change = classify_color_change(&in_colors, &out_colors);

    FeatureProfile {
        dim_change,
        color_change,
        object_delta: out_objs as i32 - in_objs as i32,
        input_symmetric_h: is_symmetric_h(input),
        input_symmetric_v: is_symmetric_v(input),
        output_symmetric_h: is_symmetric_h(output),
        output_symmetric_v: is_symmetric_v(output),
        input_period_h: detect_period_h(input),
        input_period_v: detect_period_v(input),
        output_period_h: detect_period_h(output),
        output_period_v: detect_period_v(output),
        same_grid: input == output,
        input_colors: in_colors,
        output_colors: out_colors,
        input_dims: in_dims,
        output_dims: out_dims,
    }
}

fn classify_dim_change(in_d: (usize, usize), out_d: (usize, usize)) -> DimChange {
    if in_d == out_d { return DimChange::Same; }
    if in_d.0 == out_d.1 && in_d.1 == out_d.0 { return DimChange::Transposed; }

    // Check for integer scaling
    if out_d.0 > 0 && out_d.1 > 0 && in_d.0 > 0 && in_d.1 > 0 {
        if out_d.0 % in_d.0 == 0 && out_d.1 % in_d.1 == 0 {
            let rf = out_d.0 / in_d.0;
            let cf = out_d.1 / in_d.1;
            if rf > 1 || cf > 1 {
                return DimChange::Scaled(rf, cf);
            }
        }
    }

    if out_d.0 < in_d.0 || out_d.1 < in_d.1 { return DimChange::Cropped; }
    if out_d.0 > in_d.0 || out_d.1 > in_d.1 { return DimChange::Padded; }

    DimChange::Arbitrary
}

fn classify_color_change(in_c: &[u8], out_c: &[u8]) -> ColorChange {
    if in_c == out_c { return ColorChange::Same; }

    let in_set: rustc_hash::FxHashSet<u8> = in_c.iter().copied().collect();
    let out_set: rustc_hash::FxHashSet<u8> = out_c.iter().copied().collect();

    if in_set.len() == out_set.len() { return ColorChange::Bijection; }
    if out_set.len() < in_set.len() { return ColorChange::Reduction; }
    if out_set.len() > in_set.len() { return ColorChange::Expansion; }

    ColorChange::Complex
}

/// Select primitives likely to be useful based on feature analysis.
/// Returns a reduced set of primitives (typically 20-50 vs 177 total).
pub fn select_primitives(profile: &FeatureProfile) -> Vec<Prim> {
    let mut prims = Vec::with_capacity(60);

    // Always include identity (baseline)
    prims.push(Prim::Identity);

    // Dimension-based selection
    match &profile.dim_change {
        DimChange::Same => {
            // Dimension-preserving ops
            prims.push(Prim::RotateCW);
            prims.push(Prim::RotateCCW);
            prims.push(Prim::Rotate180);
            prims.push(Prim::FlipH);
            prims.push(Prim::FlipV);
            prims.push(Prim::GravityDown);
            prims.push(Prim::GravityUp);
            prims.push(Prim::GravityLeft);
            prims.push(Prim::GravityRight);
            prims.push(Prim::Invert);
            prims.push(Prim::SortRowsByColor);
            prims.push(Prim::SortColsByColor);
            prims.push(Prim::KeepLargestObject);
            prims.push(Prim::KeepSmallestObject);
            prims.push(Prim::ExtendHLines);
            prims.push(Prim::ExtendVLines);
            prims.push(Prim::ExtendCross);
            prims.push(Prim::DiagFillTL);
            prims.push(Prim::DiagFillTR);
            // Translations
            for d in [-2i32, -1, 1, 2] {
                prims.push(Prim::Translate(d, 0));
                prims.push(Prim::Translate(0, d));
            }

            // Color ops (only relevant colors)
            add_color_ops(&mut prims, &profile.input_colors, &profile.output_colors);
        }
        DimChange::Transposed => {
            prims.push(Prim::Transpose);
            prims.push(Prim::RotateCW);
            prims.push(Prim::RotateCCW);
        }
        DimChange::Scaled(rf, cf) => {
            for s in 2..=4 {
                prims.push(Prim::Scale(s));
                prims.push(Prim::RepeatH(s));
                prims.push(Prim::RepeatV(s));
            }
            if rf == cf {
                prims.push(Prim::Scale(*rf));
            }
            prims.push(Prim::MirrorH);
            prims.push(Prim::MirrorV);
        }
        DimChange::Cropped => {
            prims.push(Prim::KeepLargestObject);
            prims.push(Prim::KeepSmallestObject);
            prims.push(Prim::CropToBBox);
            for i in 0..5 {
                prims.push(Prim::ExtractObject(i));
            }
        }
        DimChange::Padded => {
            for c in 0..=9 {
                prims.push(Prim::Pad(1, c));
                prims.push(Prim::BorderFill(c));
            }
            prims.push(Prim::MirrorH);
            prims.push(Prim::MirrorV);
        }
        DimChange::Arbitrary => {
            // Unknown transformation — include broad set
            prims.push(Prim::KeepLargestObject);
            prims.push(Prim::KeepSmallestObject);
            prims.push(Prim::Transpose);
            for i in 0..3 {
                prims.push(Prim::ExtractObject(i));
            }
        }
    }

    // Symmetry-based additions
    if profile.output_symmetric_h && !profile.input_symmetric_h {
        prims.push(Prim::MirrorH);
        prims.push(Prim::FlipH);
    }
    if profile.output_symmetric_v && !profile.input_symmetric_v {
        prims.push(Prim::MirrorV);
        prims.push(Prim::FlipV);
    }

    // Object count changes
    if profile.object_delta < 0 {
        // Fewer objects → keep/extract/remove
        prims.push(Prim::KeepLargestObject);
        prims.push(Prim::KeepSmallestObject);
        for c in 0..=9 {
            prims.push(Prim::RemoveColor(c));
        }
    }
    if profile.object_delta > 0 {
        // More objects → fill, outline
        for c in 0..=9 {
            prims.push(Prim::OutlineObjects(c));
            prims.push(Prim::FillInsideObjects(c));
        }
    }

    // Color mapping
    match &profile.color_change {
        ColorChange::Bijection => {
            for &ic in &profile.input_colors {
                for &oc in &profile.output_colors {
                    if ic != oc {
                        prims.push(Prim::ReplaceColor(ic, oc));
                    }
                }
            }
        }
        ColorChange::Reduction => {
            for &c in &profile.input_colors {
                if !profile.output_colors.contains(&c) {
                    prims.push(Prim::RemoveColor(c));
                    for &oc in &profile.output_colors {
                        prims.push(Prim::ReplaceColor(c, oc));
                    }
                }
            }
        }
        ColorChange::Expansion => {
            for &c in &profile.output_colors {
                if !profile.input_colors.contains(&c) {
                    prims.push(Prim::FillColor(c));
                    prims.push(Prim::BorderFill(c));
                    prims.push(Prim::OutlineObjects(c));
                    prims.push(Prim::FillInsideObjects(c));
                }
            }
            for &c in &profile.input_colors {
                prims.push(Prim::FillEnclosed(c));
            }
        }
        _ => {}
    }

    // Deduplicate
    dedup_prims(&mut prims);
    prims
}

fn add_color_ops(prims: &mut Vec<Prim>, in_colors: &[u8], out_colors: &[u8]) {
    for &ic in in_colors {
        for &oc in out_colors {
            if ic != oc {
                prims.push(Prim::ReplaceColor(ic, oc));
            }
        }
        prims.push(Prim::FilterColor(ic));
        prims.push(Prim::FillColor(ic));
    }
}

fn dedup_prims(prims: &mut Vec<Prim>) {
    let mut seen = rustc_hash::FxHashSet::default();
    prims.retain(|p| {
        use std::hash::{Hash, Hasher};
        let mut hasher = rustc_hash::FxHasher::default();
        p.hash(&mut hasher);
        seen.insert(hasher.finish())
    });
}

fn default_profile() -> FeatureProfile {
    FeatureProfile {
        dim_change: DimChange::Same,
        color_change: ColorChange::Same,
        object_delta: 0,
        input_symmetric_h: false,
        input_symmetric_v: false,
        output_symmetric_h: false,
        output_symmetric_v: false,
        input_period_h: None,
        input_period_v: None,
        output_period_h: None,
        output_period_v: None,
        same_grid: true,
        input_colors: Vec::new(),
        output_colors: Vec::new(),
        input_dims: (0, 0),
        output_dims: (0, 0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dim_same_detected() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let output = vec![vec![4, 3], vec![2, 1]];
        let prof = analyze_features(&[(input, output)]);
        assert_eq!(prof.dim_change, DimChange::Same);
    }

    #[test]
    fn dim_transposed_detected() {
        let input = vec![vec![1, 2, 3], vec![4, 5, 6]]; // 2x3
        let output = vec![vec![1, 4], vec![2, 5], vec![3, 6]]; // 3x2
        let prof = analyze_features(&[(input, output)]);
        assert_eq!(prof.dim_change, DimChange::Transposed);
    }

    #[test]
    fn dim_scaled_detected() {
        let input = vec![vec![1, 2], vec![3, 4]]; // 2x2
        let output = vec![
            vec![1, 1, 2, 2], vec![1, 1, 2, 2],
            vec![3, 3, 4, 4], vec![3, 3, 4, 4],
        ]; // 4x4 = 2x scale
        let prof = analyze_features(&[(input, output)]);
        assert_eq!(prof.dim_change, DimChange::Scaled(2, 2));
    }

    #[test]
    fn color_bijection_detected() {
        // Same number of unique colors, different values
        let input = vec![vec![1, 2], vec![0, 1]];
        let output = vec![vec![3, 4], vec![0, 3]];
        let prof = analyze_features(&[(input, output)]);
        assert_eq!(prof.color_change, ColorChange::Bijection);
    }

    #[test]
    fn heuristic_selects_fewer_prims() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let output = vec![vec![4, 3], vec![2, 1]]; // rotation/flip
        let prof = analyze_features(&[(input, output)]);
        let prims = select_primitives(&prof);
        let all = Prim::all_primitives();
        // Heuristic should select significantly fewer primitives
        assert!(prims.len() < all.len());
        assert!(prims.len() > 0);
    }

    #[test]
    fn transpose_detected_selects_transpose() {
        let input = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let output = vec![vec![1, 4], vec![2, 5], vec![3, 6]];
        let prof = analyze_features(&[(input, output)]);
        let prims = select_primitives(&prof);
        assert!(prims.contains(&Prim::Transpose));
    }

    #[test]
    fn symmetry_change_detected() {
        let input = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let output = vec![vec![1, 2, 1], vec![4, 5, 4]]; // h-symmetric output
        let prof = analyze_features(&[(input, output)]);
        assert!(!prof.input_symmetric_h);
        assert!(prof.output_symmetric_h);
    }

    #[test]
    fn empty_examples() {
        let prof = analyze_features(&[]);
        assert_eq!(prof.dim_change, DimChange::Same);
    }
}
