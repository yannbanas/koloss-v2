use super::dsl::{Prim, Grid};
use super::enumerate::bottom_up_enumerate;

#[derive(Debug, Clone)]
pub struct Individual {
    pub program: Prim,
    pub fitness: f64,
    pub generation: usize,
}

pub fn evolve(
    examples: &[(Grid, Grid)],
    population_size: usize,
    generations: usize,
) -> Option<Individual> {
    let seeds = bottom_up_enumerate(examples, population_size / 2);
    let mut population: Vec<Individual> = seeds.into_iter()
        .map(|(program, fitness)| Individual { program, fitness, generation: 0 })
        .collect();

    while population.len() < population_size {
        let prims = Prim::all_primitives();
        let idx = population.len() % prims.len();
        let fitness = eval_fitness(&prims[idx], examples);
        population.push(Individual { program: prims[idx].clone(), fitness, generation: 0 });
    }

    for gen in 0..generations {
        population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));

        if population[0].fitness >= 1.0 - f64::EPSILON {
            return Some(population.remove(0));
        }

        let elite_count = population_size / 4;
        let mut next_gen: Vec<Individual> = population.iter().take(elite_count).cloned().collect();

        while next_gen.len() < population_size {
            let parent_a = &population[next_gen.len() % elite_count];
            let parent_b = &population[(next_gen.len() + 1) % elite_count];

            let child_prog = crossover(&parent_a.program, &parent_b.program);
            let mutated = mutate(&child_prog);
            let fitness = eval_fitness(&mutated, examples);
            next_gen.push(Individual { program: mutated, fitness, generation: gen + 1 });
        }

        population = next_gen;
    }

    population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));
    population.into_iter().next()
}

fn eval_fitness(program: &Prim, examples: &[(Grid, Grid)]) -> f64 {
    if examples.is_empty() { return 0.0; }
    let total: f64 = examples.iter().map(|(input, expected)| {
        let result = program.apply(input);
        grid_similarity(&result, expected)
    }).sum();
    let accuracy = total / examples.len() as f64;
    let size_penalty = 1.0 / (1.0 + program.size() as f64 * 0.01);
    accuracy * 0.95 + size_penalty * 0.05
}

fn grid_similarity(a: &Grid, b: &Grid) -> f64 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    if a[0].len() != b[0].len() { return 0.0; }
    let total = a.len() * a[0].len();
    if total == 0 { return 1.0; }
    let matching: usize = a.iter().zip(b.iter())
        .flat_map(|(ar, br)| ar.iter().zip(br.iter()))
        .filter(|(&ac, &bc)| ac == bc)
        .count();
    matching as f64 / total as f64
}

fn crossover(a: &Prim, b: &Prim) -> Prim {
    Prim::Compose(Box::new(a.clone()), Box::new(b.clone()))
}

fn mutate(p: &Prim) -> Prim {
    let prims = Prim::all_primitives();
    let idx = (p.size() * 7 + 13) % prims.len();

    match p {
        Prim::Compose(a, _) => Prim::Compose(a.clone(), Box::new(prims[idx].clone())),
        _ => {
            if p.size() < 3 {
                Prim::Compose(Box::new(p.clone()), Box::new(prims[idx].clone()))
            } else {
                prims[idx].clone()
            }
        }
    }
}
