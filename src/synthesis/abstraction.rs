// DreamCoder-style library learning for program synthesis.
// Identifies recurring sub-programs and compresses them into reusable abstractions.
//
// Based on: Ellis et al., "DreamCoder: bootstrapping inductive program synthesis
// with wake-sleep library learning" (PLDI 2021)
// And: AbstractBeam (2024) — library learning for bottom-up synthesis.
//
// The approach:
// 1. Solve a batch of tasks → collect solution programs
// 2. Find common sub-expressions (anti-unification on program trees)
// 3. Extract frequent sub-programs as new library primitives
// 4. Re-index the DSL with compressed programs
// 5. Repeat — the library grows, search space shrinks

use super::dsl::{Prim, Grid};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Library {
    pub entries: Vec<LibEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LibEntry {
    pub name: String,
    pub program: Prim,
    pub usage_count: usize,
    pub compression: usize, // how many nodes it saves vs inline
}

impl Library {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    pub fn add(&mut self, name: String, program: Prim) {
        let compression = program.size();
        self.entries.push(LibEntry {
            name,
            program,
            usage_count: 0,
            compression,
        });
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&LibEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    pub fn total_compression(&self) -> usize {
        self.entries.iter().map(|e| e.usage_count * e.compression.saturating_sub(1)).sum()
    }
}

// Extract sub-programs from a program tree
fn extract_subprograms(prog: &Prim, min_size: usize) -> Vec<Prim> {
    let mut subs = Vec::new();
    if prog.size() >= min_size {
        subs.push(prog.clone());
    }
    match prog {
        Prim::Compose(a, b) => {
            subs.extend(extract_subprograms(a, min_size));
            subs.extend(extract_subprograms(b, min_size));
        }
        Prim::Conditional(a, b, c) => {
            subs.extend(extract_subprograms(a, min_size));
            subs.extend(extract_subprograms(b, min_size));
            subs.extend(extract_subprograms(c, min_size));
        }
        _ => {}
    }
    subs
}

// Count frequency of each sub-program across a corpus
fn count_subprogram_frequency(programs: &[Prim], min_size: usize) -> Vec<(Prim, usize)> {
    let mut counts: FxHashMap<u64, (Prim, usize)> = FxHashMap::default();

    for prog in programs {
        let subs = extract_subprograms(prog, min_size);
        for sub in subs {
            let key = hash_prim(&sub);
            counts.entry(key).or_insert_with(|| (sub, 0)).1 += 1;
        }
    }

    let mut freqs: Vec<(Prim, usize)> = counts.into_values().collect();
    freqs.sort_by(|a, b| b.1.cmp(&a.1));
    freqs
}

fn hash_prim(p: &Prim) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = rustc_hash::FxHasher::default();
    p.hash(&mut hasher);
    hasher.finish()
}

// Wake phase: extract library from solved programs
pub fn wake_extract(solved_programs: &[Prim], min_freq: usize, min_size: usize, max_entries: usize) -> Library {
    let mut lib = Library::new();
    let freqs = count_subprogram_frequency(solved_programs, min_size);

    for (i, (prog, count)) in freqs.iter().enumerate() {
        if *count < min_freq { break; }
        if i >= max_entries { break; }

        // Don't add trivial single primitives
        if prog.size() <= 1 { continue; }

        lib.add(format!("lib_{}", i), prog.clone());
        if let Some(entry) = lib.entries.last_mut() {
            entry.usage_count = *count;
        }
    }

    lib
}

// Sleep phase: compress existing programs using the library
pub fn sleep_compress(program: &Prim, library: &Library) -> Prim {
    // Try to match each library entry against the program
    for entry in &library.entries {
        if *program == entry.program {
            // In a full implementation, this would return a LibRef
            // For now, we just track that it was matched
            return program.clone();
        }
    }

    match program {
        Prim::Compose(a, b) => {
            let ca = sleep_compress(a, library);
            let cb = sleep_compress(b, library);
            Prim::Compose(Box::new(ca), Box::new(cb))
        }
        Prim::Conditional(a, b, c) => {
            let ca = sleep_compress(a, library);
            let cb = sleep_compress(b, library);
            let cc = sleep_compress(c, library);
            Prim::Conditional(Box::new(ca), Box::new(cb), Box::new(cc))
        }
        other => other.clone(),
    }
}

// DAG-based search (Icecuber-style)
// Store intermediate grid results in a DAG, greedily compose primitives
#[derive(Debug)]
pub struct SearchDag {
    nodes: Vec<DagNode>,
    max_nodes: usize,
}

#[derive(Debug, Clone)]
struct DagNode {
    grid: Grid,
    program: Prim,
    depth: usize,
}

impl SearchDag {
    pub fn new(max_nodes: usize) -> Self {
        Self { nodes: Vec::new(), max_nodes }
    }

    pub fn search(&mut self, input: &Grid, target: &Grid, primitives: &[Prim], max_depth: usize) -> Option<Prim> {
        self.nodes.clear();
        self.nodes.push(DagNode {
            grid: input.clone(),
            program: Prim::Identity,
            depth: 0,
        });

        // Check identity
        if input == target {
            return Some(Prim::Identity);
        }

        for depth in 0..max_depth {
            let current_count = self.nodes.len();
            let mut new_nodes = Vec::new();

            for node_idx in 0..current_count {
                if self.nodes[node_idx].depth != depth { continue; }
                let grid = self.nodes[node_idx].grid.clone();
                let prog = self.nodes[node_idx].program.clone();

                for prim in primitives {
                    let result = prim.apply(&grid);

                    // Check if we found the target
                    if result == *target {
                        if depth == 0 {
                            return Some(prim.clone());
                        } else {
                            return Some(Prim::Compose(Box::new(prog.clone()), Box::new(prim.clone())));
                        }
                    }

                    // Avoid duplicates: check if this grid already exists
                    let is_dup = self.nodes.iter().any(|n| n.grid == result)
                        || new_nodes.iter().any(|n: &DagNode| n.grid == result);
                    if is_dup { continue; }

                    // Only keep if it changes something (avoid identity loops)
                    if result == grid { continue; }

                    let new_prog = if depth == 0 {
                        prim.clone()
                    } else {
                        Prim::Compose(Box::new(prog.clone()), Box::new(prim.clone()))
                    };

                    new_nodes.push(DagNode {
                        grid: result,
                        program: new_prog,
                        depth: depth + 1,
                    });

                    if self.nodes.len() + new_nodes.len() >= self.max_nodes {
                        break;
                    }
                }

                if self.nodes.len() + new_nodes.len() >= self.max_nodes {
                    break;
                }
            }

            self.nodes.extend(new_nodes);
        }

        None
    }

    pub fn search_scored(&mut self, input: &Grid, target: &Grid, primitives: &[Prim], max_depth: usize) -> Vec<(Prim, f64)> {
        self.nodes.clear();
        self.nodes.push(DagNode {
            grid: input.clone(),
            program: Prim::Identity,
            depth: 0,
        });

        let mut scored = Vec::new();

        for depth in 0..max_depth {
            let current_count = self.nodes.len();
            let mut new_nodes = Vec::new();

            for node_idx in 0..current_count {
                if self.nodes[node_idx].depth != depth { continue; }
                let grid = self.nodes[node_idx].grid.clone();
                let prog = self.nodes[node_idx].program.clone();

                for prim in primitives {
                    let result = prim.apply(&grid);

                    let new_prog = if depth == 0 {
                        prim.clone()
                    } else {
                        Prim::Compose(Box::new(prog.clone()), Box::new(prim.clone()))
                    };

                    if result == *target {
                        return vec![(new_prog, 1.0)];
                    }

                    let sim = grid_similarity(&result, target);
                    if sim > 0.0 {
                        scored.push((new_prog.clone(), sim));
                    }

                    let is_dup = self.nodes.iter().any(|n| n.grid == result)
                        || new_nodes.iter().any(|n: &DagNode| n.grid == result);
                    if !is_dup && result != grid {
                        new_nodes.push(DagNode {
                            grid: result,
                            program: new_prog,
                            depth: depth + 1,
                        });
                    }

                    if self.nodes.len() + new_nodes.len() >= self.max_nodes {
                        break;
                    }
                }
            }

            self.nodes.extend(new_nodes);
        }

        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(10);
        scored
    }

    pub fn nodes_explored(&self) -> usize {
        self.nodes.len()
    }
}

fn grid_similarity(a: &Grid, b: &Grid) -> f64 {
    if a.is_empty() || b.is_empty() { return 0.0; }
    if a.len() != b.len() || a[0].len() != b[0].len() { return 0.0; }
    let total = a.len() * a[0].len();
    if total == 0 { return 0.0; }
    let matching = a.iter().zip(b.iter())
        .flat_map(|(ra, rb)| ra.iter().zip(rb.iter()))
        .filter(|(ca, cb)| ca == cb)
        .count();
    matching as f64 / total as f64
}

// Full wake-sleep cycle
pub fn wake_sleep_cycle(
    tasks: &[(Grid, Grid)],
    primitives: &[Prim],
    max_dag_nodes: usize,
    max_depth: usize,
    min_freq: usize,
) -> (Library, Vec<Option<Prim>>) {
    let mut dag = SearchDag::new(max_dag_nodes);
    let mut solutions = Vec::new();
    let mut solved_programs = Vec::new();

    // Wake: solve tasks
    for (input, output) in tasks {
        let result = dag.search(input, output, primitives, max_depth);
        if let Some(ref prog) = result {
            solved_programs.push(prog.clone());
        }
        solutions.push(result);
    }

    // Sleep: extract library
    let library = wake_extract(&solved_programs, min_freq, 2, 20);

    (library, solutions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn library_basic_ops() {
        let mut lib = Library::new();
        assert!(lib.is_empty());
        lib.add("test".into(), Prim::Compose(Box::new(Prim::FlipH), Box::new(Prim::RotateCW)));
        assert_eq!(lib.len(), 1);
        assert!(!lib.is_empty());
        assert!(lib.get("test").is_some());
        assert!(lib.get("nope").is_none());
    }

    #[test]
    fn library_compression_tracking() {
        let mut lib = Library::new();
        let prog = Prim::Compose(Box::new(Prim::FlipH), Box::new(Prim::RotateCW));
        lib.add("comp".into(), prog);
        assert_eq!(lib.entries[0].compression, 3); // Compose(FlipH, RotateCW) = size 3
    }

    #[test]
    fn wake_extract_finds_common() {
        // Create 5 identical composed programs
        let prog = Prim::Compose(Box::new(Prim::FlipH), Box::new(Prim::RotateCW));
        let programs = vec![prog.clone(); 5];
        let lib = wake_extract(&programs, 2, 2, 10);
        assert!(lib.len() > 0);
    }

    #[test]
    fn wake_extract_filters_low_freq() {
        let prog = Prim::Compose(Box::new(Prim::FlipH), Box::new(Prim::RotateCW));
        let programs = vec![prog]; // only 1 occurrence
        let lib = wake_extract(&programs, 3, 2, 10); // min_freq=3
        assert_eq!(lib.len(), 0);
    }

    #[test]
    fn search_dag_identity() {
        let grid = vec![vec![1, 2], vec![3, 4]];
        let mut dag = SearchDag::new(100);
        let result = dag.search(&grid, &grid, &[Prim::FlipH], 3);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), Prim::Identity);
    }

    #[test]
    fn search_dag_single_step() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let target = Prim::FlipH.apply(&input);
        let prims = vec![Prim::FlipH, Prim::FlipV, Prim::RotateCW];
        let mut dag = SearchDag::new(1000);
        let result = dag.search(&input, &target, &prims, 3);
        assert!(result.is_some());
        assert_eq!(result.unwrap().apply(&input), target);
    }

    #[test]
    fn search_dag_two_step() {
        let input = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let mid = Prim::FlipH.apply(&input);
        let target = Prim::FlipV.apply(&mid);
        let prims = vec![Prim::FlipH, Prim::FlipV, Prim::RotateCW, Prim::RotateCCW];
        let mut dag = SearchDag::new(5000);
        let result = dag.search(&input, &target, &prims, 3);
        assert!(result.is_some());
        assert_eq!(result.unwrap().apply(&input), target);
    }

    #[test]
    fn search_dag_scored() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let target = vec![vec![4, 3], vec![2, 1]];
        let prims = vec![Prim::FlipH, Prim::FlipV, Prim::RotateCW];
        let mut dag = SearchDag::new(1000);
        let scored = dag.search_scored(&input, &target, &prims, 2);
        assert!(!scored.is_empty());
        // Scores should be sorted descending
        for w in scored.windows(2) {
            assert!(w[0].1 >= w[1].1);
        }
    }

    #[test]
    fn sleep_compress_preserves() {
        let prog = Prim::FlipH;
        let lib = Library::new();
        let compressed = sleep_compress(&prog, &lib);
        assert_eq!(compressed, prog);
    }

    #[test]
    fn grid_similarity_identical() {
        let g = vec![vec![1, 2], vec![3, 4]];
        assert_eq!(grid_similarity(&g, &g), 1.0);
    }

    #[test]
    fn grid_similarity_different_dims() {
        let a = vec![vec![1, 2]];
        let b = vec![vec![1, 2], vec![3, 4]];
        assert_eq!(grid_similarity(&a, &b), 0.0);
    }

    #[test]
    fn wake_sleep_cycle_basic() {
        let input = vec![vec![1, 2], vec![3, 4]];
        let target = Prim::FlipH.apply(&input);
        let tasks = vec![(input, target)];
        let prims = vec![Prim::FlipH, Prim::FlipV, Prim::RotateCW];
        let (lib, solutions) = wake_sleep_cycle(&tasks, &prims, 1000, 3, 2);
        assert_eq!(solutions.len(), 1);
        assert!(solutions[0].is_some());
        // Library may or may not have entries (depends on min_freq)
        let _ = lib;
    }
}
