use crate::reasoning::rules::{Rule, RuleEngine};
use crate::core::Term;
use super::fitness::{TestCase, evaluate_engine};

#[derive(Debug, Clone)]
pub enum Mutation {
    AddRule(Rule),
    RemoveRule(usize),
    ModifyRuleHead(usize, Term),
    AddFact(Term),
    RetractFact(Term),
    SwapRules(usize, usize),
    DuplicateRule(usize),
    SimplifyRule(usize),
}

#[derive(Debug)]
pub struct MutationLog {
    pub mutations: Vec<(Mutation, f64, f64)>,
}

impl MutationLog {
    pub fn new() -> Self {
        Self { mutations: Vec::new() }
    }

    pub fn record(&mut self, mutation: Mutation, fitness_before: f64, fitness_after: f64) {
        self.mutations.push((mutation, fitness_before, fitness_after));
    }

    pub fn improvements(&self) -> Vec<&(Mutation, f64, f64)> {
        self.mutations.iter().filter(|(_, before, after)| after > before).collect()
    }

    pub fn regressions(&self) -> Vec<&(Mutation, f64, f64)> {
        self.mutations.iter().filter(|(_, before, after)| after < before).collect()
    }

    pub fn best_improvement(&self) -> Option<&(Mutation, f64, f64)> {
        self.improvements().into_iter().max_by(|a, b| {
            (a.2 - a.1).partial_cmp(&(b.2 - b.1)).unwrap_or(std::cmp::Ordering::Equal)
        })
    }
}

pub fn apply_mutation(engine: &mut RuleEngine, mutation: &Mutation) -> bool {
    match mutation {
        Mutation::AddRule(rule) => {
            engine.add_rule(rule.clone());
            true
        }
        Mutation::AddFact(fact) => {
            engine.add_fact(fact.clone());
            true
        }
        Mutation::RetractFact(fact) => {
            engine.retract(fact)
        }
        Mutation::RemoveRule(_) | Mutation::ModifyRuleHead(_, _)
        | Mutation::SwapRules(_, _) | Mutation::DuplicateRule(_)
        | Mutation::SimplifyRule(_) => {
            false
        }
    }
}

pub fn generate_mutations(engine: &RuleEngine) -> Vec<Mutation> {
    let mut mutations = Vec::new();

    for (i, _rule) in engine.rules().iter().enumerate() {
        mutations.push(Mutation::RemoveRule(i));
        mutations.push(Mutation::DuplicateRule(i));
    }

    for fact in engine.facts().iter() {
        mutations.push(Mutation::RetractFact((*fact).clone()));
    }

    if engine.num_rules() >= 2 {
        for i in 0..engine.num_rules() - 1 {
            mutations.push(Mutation::SwapRules(i, i + 1));
        }
    }

    mutations
}

// --- Hill Climbing ---

#[derive(Debug)]
pub struct HillClimbResult {
    pub iterations: usize,
    pub initial_fitness: f64,
    pub final_fitness: f64,
    pub improvements: usize,
    pub log: MutationLog,
}

pub fn hill_climb(
    engine: &mut RuleEngine,
    test_cases: &[TestCase],
    max_iterations: usize,
) -> HillClimbResult {
    let mut log = MutationLog::new();
    let mut current_fitness = evaluate_engine(engine, test_cases);
    let initial_fitness = current_fitness;
    let mut improvements = 0;

    for iter in 0..max_iterations {
        let mutations = generate_mutations(engine);
        if mutations.is_empty() { break; }

        let mut best_mutation = None;
        let mut best_fitness = current_fitness;

        for mutation in &mutations {
            let mut candidate = engine.clone();
            if apply_mutation(&mut candidate, mutation) {
                let fitness = evaluate_engine(&mut candidate, test_cases);
                if fitness > best_fitness + 0.001 {
                    best_fitness = fitness;
                    best_mutation = Some(mutation.clone());
                }
            }
        }

        if let Some(mutation) = best_mutation {
            apply_mutation(engine, &mutation);
            log.record(mutation, current_fitness, best_fitness);
            current_fitness = best_fitness;
            improvements += 1;
        } else {
            return HillClimbResult {
                iterations: iter + 1,
                initial_fitness,
                final_fitness: current_fitness,
                improvements,
                log,
            };
        }
    }

    HillClimbResult {
        iterations: max_iterations,
        initial_fitness,
        final_fitness: current_fitness,
        improvements,
        log,
    }
}

// --- Genetic Programming on RuleEngine ---

#[derive(Debug, Clone)]
pub struct EngineIndividual {
    pub engine: RuleEngine,
    pub fitness: f64,
}

pub fn evolve_engines(
    base: &RuleEngine,
    test_cases: &[TestCase],
    population_size: usize,
    generations: usize,
) -> EngineIndividual {
    let mut rng_state: u64 = 12345;
    let mut lcg = || -> u64 {
        rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        rng_state >> 33
    };

    // Initialize population with mutations of base
    let mut population: Vec<EngineIndividual> = Vec::new();
    for _ in 0..population_size {
        let mut eng = base.clone();
        let mutations = generate_mutations(&eng);
        if !mutations.is_empty() {
            let idx = lcg() as usize % mutations.len();
            let _ = apply_mutation(&mut eng, &mutations[idx]);
        }
        let fitness = evaluate_engine(&mut eng, test_cases);
        population.push(EngineIndividual { engine: eng, fitness });
    }

    // Add base
    {
        let mut base_clone = base.clone();
        let fitness = evaluate_engine(&mut base_clone, test_cases);
        population.push(EngineIndividual { engine: base_clone, fitness });
    }

    for _ in 0..generations {
        population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));
        population.truncate(population_size);

        let top_half = population_size / 2;
        let mut children = Vec::new();

        for i in 0..top_half {
            let parent = &population[i];
            let mut child = parent.engine.clone();

            // Apply 1-3 random mutations
            let n_mutations = 1 + (lcg() % 3) as usize;
            for _ in 0..n_mutations {
                let mutations = generate_mutations(&child);
                if !mutations.is_empty() {
                    let idx = lcg() as usize % mutations.len();
                    let _ = apply_mutation(&mut child, &mutations[idx]);
                }
            }

            let fitness = evaluate_engine(&mut child, test_cases);
            children.push(EngineIndividual { engine: child, fitness });
        }

        population.extend(children);
    }

    population.sort_by(|a, b| b.fitness.partial_cmp(&a.fitness).unwrap_or(std::cmp::Ordering::Equal));
    population.into_iter().next().unwrap_or(EngineIndividual {
        engine: base.clone(),
        fitness: 0.0,
    })
}

// --- Auto-Compilation ---

pub fn generate_rust_source(engine: &RuleEngine) -> String {
    let mut src = String::new();
    src.push_str("// Auto-generated by KOLOSS v2 self-improvement\n");
    src.push_str("// Rules and facts snapshot\n\n");

    src.push_str(&format!("// {} rules, {} facts\n", engine.num_rules(), engine.num_facts()));

    for (i, fact) in engine.facts().iter().enumerate() {
        src.push_str(&format!("// fact[{}]: {}\n", i, fact));
    }

    for (i, rule) in engine.rules().iter().enumerate() {
        src.push_str(&format!("// rule[{}]: {} :- ", i, rule.head));
        let body: Vec<String> = rule.body.iter().map(|t| format!("{}", t)).collect();
        src.push_str(&body.join(", "));
        src.push_str(".\n");
    }

    src.push_str("\npub fn num_rules() -> usize { ");
    src.push_str(&format!("{}", engine.num_rules()));
    src.push_str(" }\n");
    src.push_str("pub fn num_facts() -> usize { ");
    src.push_str(&format!("{}", engine.num_facts()));
    src.push_str(" }\n");

    src
}

pub fn try_compile_check(source: &str) -> Result<(), String> {
    let tmp = std::env::temp_dir().join("koloss_v2_self_compile.rs");
    std::fs::write(&tmp, source).map_err(|e| e.to_string())?;

    let output = std::process::Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("-o")
        .arg("/dev/null")
        .arg(&tmp)
        .output()
        .map_err(|e| e.to_string())?;

    let _ = std::fs::remove_file(&tmp);

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}

// --- Self-Replication ---

pub fn generate_project(engine: &RuleEngine, project_name: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();

    // Cargo.toml
    files.push(("Cargo.toml".to_string(), format!(
        r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[profile.release]
opt-level = 3
lto = true
strip = true
"#, project_name)));

    // src/main.rs with embedded facts/rules
    let mut main_rs = String::new();
    main_rs.push_str("fn main() {\n");
    main_rs.push_str(&format!("    println!(\"{}  — Self-replicated engine\");\n", project_name));
    main_rs.push_str(&format!("    println!(\"Rules: {}, Facts: {}\");\n",
        engine.num_rules(), engine.num_facts()));
    main_rs.push_str("}\n");

    files.push(("src/main.rs".to_string(), main_rs));

    files
}

pub fn write_project(files: &[(String, String)], base_dir: &std::path::Path) -> Result<(), String> {
    for (path, content) in files {
        let full_path = base_dir.join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&full_path, content).map_err(|e| e.to_string())?;
    }
    Ok(())
}
