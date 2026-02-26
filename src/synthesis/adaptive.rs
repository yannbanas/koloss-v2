// Adaptive Meta-Solver: polymorphic program synthesis that learns
// which strategies work best and dynamically adapts its search.
//
// Novel concepts:
// 1. Strategy Portfolio: maintain performance stats per strategy,
//    allocate more time to strategies that solve more tasks
// 2. Polymorphic Transforms: detect transform "type" and dispatch
//    to the most specific solver
// 3. Transfer Learning: use solutions from previous tasks to
//    seed the search for new ones
// 4. Autonomous Primitive Discovery: detect recurring patterns
//    in failed tasks and propose new primitives

use super::dsl::{Grid, Prim};
use rustc_hash::FxHashMap;

/// Transform type classification — what kind of problem is this?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransformType {
    ColorRemap,      // Pure color mapping
    Geometric,       // Rotation, flip, transpose
    ObjectManip,     // Extract/move/remove objects
    Tiling,          // Grid repetition/tiling
    Resizing,        // Output dimensions differ
    PatternFill,     // Fill based on a pattern
    Conditional,     // Different action per region/color
    Unknown,
}

/// Classify a task based on input/output analysis.
pub fn classify_transform(examples: &[(Grid, Grid)]) -> TransformType {
    if examples.is_empty() { return TransformType::Unknown; }

    let (input, output) = &examples[0];
    let in_dims = (input.len(), if input.is_empty() { 0 } else { input[0].len() });
    let out_dims = (output.len(), if output.is_empty() { 0 } else { output[0].len() });

    // Check for color remap (same dims, all cells changed by a function)
    if in_dims == out_dims {
        if let Some(_) = super::smart_prims::learn_color_map(input, output) {
            return TransformType::ColorRemap;
        }
    }

    // Check for tiling
    if out_dims.0 > in_dims.0 || out_dims.1 > in_dims.1 {
        if out_dims.0 % in_dims.0 == 0 && out_dims.1 % in_dims.1 == 0 {
            return TransformType::Tiling;
        }
        return TransformType::Resizing;
    }

    // Check for resizing (smaller output)
    if out_dims != in_dims {
        return TransformType::Resizing;
    }

    // Same dimensions — check for geometric (rotation invariant hash)
    let in_cells: Vec<u8> = input.iter().flat_map(|r| r.iter()).cloned().collect();
    let out_cells: Vec<u8> = output.iter().flat_map(|r| r.iter()).cloned().collect();
    let mut in_sorted = in_cells.clone();
    let mut out_sorted = out_cells.clone();
    in_sorted.sort();
    out_sorted.sort();
    if in_sorted == out_sorted {
        return TransformType::Geometric;
    }

    // Check for object manipulation (different object counts)
    let in_objs = super::dsl::connected_components(input, true).len();
    let out_objs = super::dsl::connected_components(output, true).len();
    if in_objs != out_objs {
        return TransformType::ObjectManip;
    }

    TransformType::Unknown
}

/// Strategy performance tracker — learns which strategies work.
#[derive(Debug, Clone)]
pub struct StrategyTracker {
    stats: FxHashMap<String, StrategyStats>,
    type_affinity: FxHashMap<TransformType, Vec<(String, f64)>>,
}

#[derive(Debug, Clone, Default)]
pub struct StrategyStats {
    pub attempts: usize,
    pub successes: usize,
    pub total_time_ms: u64,
}

impl StrategyStats {
    pub fn success_rate(&self) -> f64 {
        if self.attempts == 0 { 0.0 } else { self.successes as f64 / self.attempts as f64 }
    }

    pub fn avg_time_ms(&self) -> f64 {
        if self.attempts == 0 { 0.0 } else { self.total_time_ms as f64 / self.attempts as f64 }
    }
}

impl StrategyTracker {
    pub fn new() -> Self {
        Self {
            stats: FxHashMap::default(),
            type_affinity: FxHashMap::default(),
        }
    }

    pub fn record(&mut self, strategy: &str, transform_type: TransformType,
                   success: bool, time_ms: u64) {
        let stats = self.stats.entry(strategy.to_string()).or_default();
        stats.attempts += 1;
        if success { stats.successes += 1; }
        stats.total_time_ms += time_ms;

        // Update type affinity
        let affinity = self.type_affinity.entry(transform_type).or_default();
        let score = if success { 1.0 } else { -0.1 };
        if let Some(entry) = affinity.iter_mut().find(|(s, _)| s == strategy) {
            entry.1 += score;
        } else {
            affinity.push((strategy.to_string(), score));
        }
    }

    /// Get strategies ranked by expected success for this transform type.
    pub fn ranked_strategies(&self, transform_type: TransformType) -> Vec<(String, f64)> {
        let mut strategies: Vec<(String, f64)> = self.stats.iter()
            .map(|(name, stats)| {
                let base_score = stats.success_rate();
                // Boost by type affinity
                let affinity_bonus = self.type_affinity.get(&transform_type)
                    .and_then(|aff| aff.iter().find(|(s, _)| s == name))
                    .map(|(_, score)| *score * 0.1)
                    .unwrap_or(0.0);
                (name.clone(), base_score + affinity_bonus)
            })
            .collect();
        strategies.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        strategies
    }

    pub fn stats(&self) -> &FxHashMap<String, StrategyStats> {
        &self.stats
    }
}

/// Solution cache for transfer learning.
/// Maps transform type → successful programs.
#[derive(Debug, Clone)]
pub struct SolutionCache {
    by_type: FxHashMap<TransformType, Vec<CachedSolution>>,
}

#[derive(Debug, Clone)]
pub struct CachedSolution {
    pub program: Prim,
    pub task_id: String,
    pub transform_type: TransformType,
}

impl SolutionCache {
    pub fn new() -> Self {
        Self { by_type: FxHashMap::default() }
    }

    pub fn add(&mut self, program: Prim, task_id: String, tt: TransformType) {
        self.by_type.entry(tt).or_default().push(CachedSolution {
            program, task_id, transform_type: tt,
        });
    }

    /// Try cached solutions of the same type on new examples.
    pub fn try_cached(&self, tt: TransformType, examples: &[(Grid, Grid)]) -> Option<&CachedSolution> {
        let cached = self.by_type.get(&tt)?;
        cached.iter().find(|sol| {
            examples.iter().all(|(input, expected)| {
                sol.program.apply(input) == *expected
            })
        })
    }

    pub fn total_cached(&self) -> usize {
        self.by_type.values().map(|v| v.len()).sum()
    }
}

/// Pattern detector for autonomous primitive discovery.
/// Analyzes failed tasks to find common patterns that current
/// primitives can't handle.
#[derive(Debug, Clone)]
pub struct PatternGap {
    pub description: String,
    pub frequency: usize,
    pub transform_type: TransformType,
}

pub fn detect_gaps(failed_tasks: &[(TransformType, usize)]) -> Vec<PatternGap> {
    let mut type_counts: FxHashMap<TransformType, usize> = FxHashMap::default();
    for (tt, _) in failed_tasks {
        *type_counts.entry(*tt).or_default() += 1;
    }

    let mut gaps: Vec<PatternGap> = type_counts.iter()
        .filter(|(_, &count)| count >= 2) // At least 2 failures of same type
        .map(|(&tt, &count)| PatternGap {
            description: format!("{:?} transforms fail ({} tasks)", tt, count),
            frequency: count,
            transform_type: tt,
        })
        .collect();

    gaps.sort_by(|a, b| b.frequency.cmp(&a.frequency));
    gaps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_color_remap() {
        let examples = vec![
            (vec![vec![1, 2]], vec![vec![3, 4]]),
        ];
        assert_eq!(classify_transform(&examples), TransformType::ColorRemap);
    }

    #[test]
    fn classify_geometric() {
        // Use repeated colors so color map is inconsistent (1 at (0,0)→1, 1 at (1,1)→1)
        // but cell positions change (geometric transform)
        let input = vec![vec![1, 2, 1], vec![3, 1, 3]];
        let output = vec![vec![1, 2, 1], vec![3, 1, 3]]; // FlipH of symmetric = same
        // Better: FlipV
        let input2 = vec![vec![1, 2], vec![3, 1]];
        let output2 = vec![vec![3, 1], vec![1, 2]]; // FlipV
        // color_map would need 1→3 AND 1→1 → inconsistent
        assert_eq!(classify_transform(&[(input2, output2)]), TransformType::Geometric);
    }

    #[test]
    fn classify_tiling() {
        let input = vec![vec![1, 2], vec![3, 4]]; // 2x2
        let output = vec![
            vec![1, 2, 1, 2], vec![3, 4, 3, 4],
            vec![1, 2, 1, 2], vec![3, 4, 3, 4],
        ]; // 4x4
        assert_eq!(classify_transform(&[(input, output)]), TransformType::Tiling);
    }

    #[test]
    fn classify_resizing() {
        let input = vec![vec![1, 2, 3], vec![4, 5, 6]]; // 2x3
        let output = vec![vec![1, 2]]; // 1x2
        assert_eq!(classify_transform(&[(input, output)]), TransformType::Resizing);
    }

    #[test]
    fn strategy_tracker_learns() {
        let mut tracker = StrategyTracker::new();
        tracker.record("heuristic", TransformType::Geometric, true, 10);
        tracker.record("heuristic", TransformType::Geometric, true, 5);
        tracker.record("bidir", TransformType::Geometric, false, 100);
        tracker.record("bidir", TransformType::ColorRemap, true, 50);

        let ranked = tracker.ranked_strategies(TransformType::Geometric);
        assert!(ranked[0].0 == "heuristic"); // higher success rate for geometric
    }

    #[test]
    fn solution_cache_transfer() {
        let mut cache = SolutionCache::new();
        cache.add(Prim::FlipH, "task1".into(), TransformType::Geometric);

        let examples = vec![
            (vec![vec![1, 2], vec![3, 4]], vec![vec![2, 1], vec![4, 3]]),
        ];
        let found = cache.try_cached(TransformType::Geometric, &examples);
        assert!(found.is_some());
    }

    #[test]
    fn gap_detection() {
        let failed = vec![
            (TransformType::Unknown, 5),
            (TransformType::Unknown, 3),
            (TransformType::PatternFill, 1),
            (TransformType::Conditional, 7),
            (TransformType::Conditional, 2),
        ];
        let gaps = detect_gaps(&failed);
        assert!(gaps.len() >= 2);
        assert_eq!(gaps[0].transform_type, TransformType::Unknown); // most frequent
    }
}
