use super::dsl::{Prim, Grid};

#[derive(Debug, Clone)]
pub struct SynthesisResult {
    pub program: Prim,
    pub size: usize,
    pub checked: usize,
}

pub fn synthesize(examples: &[(Grid, Grid)], max_size: usize) -> Option<SynthesisResult> {
    let mut checked = 0usize;

    let prims = Prim::all_primitives();
    for p in &prims {
        checked += 1;
        if matches_all(p, examples) {
            return Some(SynthesisResult { program: p.clone(), size: p.size(), checked });
        }
    }

    if max_size >= 2 {
        for a in &prims {
            for b in &prims {
                checked += 1;
                let composed = Prim::Compose(Box::new(a.clone()), Box::new(b.clone()));
                if matches_all(&composed, examples) {
                    return Some(SynthesisResult { program: composed.clone(), size: composed.size(), checked });
                }
            }
            if checked > 100_000 {
                break;
            }
        }
    }

    if max_size >= 3 {
        let top_singles: Vec<&Prim> = prims.iter()
            .filter(|p| partial_match_score(p, examples) > 0.3)
            .take(20)
            .collect();

        for a in &top_singles {
            for b in &top_singles {
                for c in &top_singles {
                    checked += 1;
                    let prog = Prim::Compose(
                        Box::new((*a).clone()),
                        Box::new(Prim::Compose(Box::new((*b).clone()), Box::new((*c).clone()))),
                    );
                    if matches_all(&prog, examples) {
                        return Some(SynthesisResult { program: prog.clone(), size: prog.size(), checked });
                    }
                    if checked > 500_000 {
                        return None;
                    }
                }
            }
        }
    }

    None
}

fn matches_all(program: &Prim, examples: &[(Grid, Grid)]) -> bool {
    examples.iter().all(|(input, expected)| {
        let result = program.apply(input);
        result == *expected
    })
}

fn partial_match_score(program: &Prim, examples: &[(Grid, Grid)]) -> f64 {
    if examples.is_empty() { return 0.0; }
    let total: f64 = examples.iter().map(|(input, expected)| {
        let result = program.apply(input);
        grid_similarity(&result, expected)
    }).sum();
    total / examples.len() as f64
}

fn grid_similarity(a: &Grid, b: &Grid) -> f64 {
    if a.len() != b.len() { return 0.0; }
    if a.is_empty() { return 1.0; }
    if a[0].len() != b[0].len() { return 0.0; }

    let total = a.len() * a[0].len();
    if total == 0 { return 1.0; }

    let matching: usize = a.iter().zip(b.iter())
        .flat_map(|(ar, br)| ar.iter().zip(br.iter()))
        .filter(|(&ac, &bc)| ac == bc)
        .count();

    matching as f64 / total as f64
}

pub fn bottom_up_enumerate(examples: &[(Grid, Grid)], max_programs: usize) -> Vec<(Prim, f64)> {
    let prims = Prim::all_primitives();
    let mut ranked: Vec<(Prim, f64)> = prims.iter()
        .map(|p| (p.clone(), partial_match_score(p, examples)))
        .filter(|(_, score)| *score > 0.0)
        .collect();

    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.truncate(max_programs);
    ranked
}
