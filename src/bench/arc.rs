use crate::perception::grid::{ArcTask, ArcExample};
use crate::synthesis::dsl::Grid;
use crate::synthesis::enumerate::synthesize;
use crate::synthesis::evolve::evolve;

#[derive(Debug, Clone)]
pub struct ArcResult {
    pub task_id: String,
    pub solved: bool,
    pub method: String,
    pub program_size: usize,
    pub checked: usize,
}

pub fn solve_arc_task(task: &ArcTask, max_size: usize) -> ArcResult {
    let examples: Vec<(Grid, Grid)> = task.train.iter()
        .map(|ex| (ex.input.clone(), ex.output.clone()))
        .collect();

    if let Some(result) = synthesize(&examples, max_size) {
        let test_ok = task.test.iter().all(|ex| {
            result.program.apply(&ex.input) == ex.output
        });
        if test_ok {
            return ArcResult {
                task_id: task.id.clone(),
                solved: true,
                method: "synthesis".into(),
                program_size: result.size,
                checked: result.checked,
            };
        }
    }

    if let Some(individual) = evolve(&examples, 50, 100) {
        let test_ok = task.test.iter().all(|ex| {
            individual.program.apply(&ex.input) == ex.output
        });
        if test_ok {
            return ArcResult {
                task_id: task.id.clone(),
                solved: true,
                method: "evolution".into(),
                program_size: individual.program.size(),
                checked: 0,
            };
        }
    }

    ArcResult {
        task_id: task.id.clone(),
        solved: false,
        method: "none".into(),
        program_size: 0,
        checked: 0,
    }
}

pub fn benchmark_arc(tasks: &[ArcTask], max_size: usize) -> ArcBenchmarkResult {
    let mut results = Vec::new();
    for task in tasks {
        results.push(solve_arc_task(task, max_size));
    }
    let solved = results.iter().filter(|r| r.solved).count();
    ArcBenchmarkResult {
        total: tasks.len(),
        solved,
        score: if tasks.is_empty() { 0.0 } else { solved as f64 / tasks.len() as f64 },
        results,
    }
}

#[derive(Debug)]
pub struct ArcBenchmarkResult {
    pub total: usize,
    pub solved: usize,
    pub score: f64,
    pub results: Vec<ArcResult>,
}
