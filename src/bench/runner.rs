// ARC-AGI benchmark runner.
// Loads tasks from the official dataset, runs the multi-strategy solver,
// produces detailed scoring and per-task reports.

use std::path::Path;
use std::time::Instant;
use crate::perception::grid::load_arc_task;
use super::arc::{solve_arc_task, ArcResult};

#[derive(Debug)]
pub struct BenchmarkReport {
    pub total_tasks: usize,
    pub solved: usize,
    pub score: f64,
    pub avg_mdl: f64,
    pub elapsed_ms: u64,
    pub by_method: Vec<(String, usize)>,
    pub per_task: Vec<TaskReport>,
}

#[derive(Debug, Clone)]
pub struct TaskReport {
    pub task_id: String,
    pub solved: bool,
    pub method: String,
    pub program_size: usize,
    pub checked: usize,
    pub mdl: f64,
    pub elapsed_ms: u64,
}

/// Run benchmark on a directory of ARC tasks.
pub fn run_benchmark(data_dir: &str, max_tasks: Option<usize>, max_size: usize) -> BenchmarkReport {
    let dir = Path::new(data_dir);
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .expect("cannot read ARC data dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "json").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());

    if let Some(max) = max_tasks {
        entries.truncate(max);
    }

    let total_start = Instant::now();
    let mut per_task = Vec::new();
    let mut method_counts: rustc_hash::FxHashMap<String, usize> = Default::default();

    for entry in &entries {
        let path = entry.path();
        let task = match load_arc_task(path.to_str().unwrap_or("")) {
            Ok(t) => t,
            Err(_) => continue,
        };

        let start = Instant::now();
        let result: ArcResult = solve_arc_task(&task, max_size);
        let elapsed = start.elapsed().as_millis() as u64;

        if result.solved {
            *method_counts.entry(result.method.clone()).or_default() += 1;
        }

        per_task.push(TaskReport {
            task_id: result.task_id,
            solved: result.solved,
            method: result.method,
            program_size: result.program_size,
            checked: result.checked,
            mdl: result.mdl,
            elapsed_ms: elapsed,
        });
    }

    let total_elapsed = total_start.elapsed().as_millis() as u64;
    let solved = per_task.iter().filter(|t| t.solved).count();
    let avg_mdl = per_task.iter()
        .filter(|t| t.solved)
        .map(|t| t.mdl)
        .sum::<f64>()
        / solved.max(1) as f64;

    let mut by_method: Vec<(String, usize)> = method_counts.into_iter().collect();
    by_method.sort_by(|a, b| b.1.cmp(&a.1));

    BenchmarkReport {
        total_tasks: per_task.len(),
        solved,
        score: if per_task.is_empty() { 0.0 } else { solved as f64 / per_task.len() as f64 },
        avg_mdl,
        elapsed_ms: total_elapsed,
        by_method,
        per_task,
    }
}

impl BenchmarkReport {
    pub fn print_summary(&self) {
        println!("=== ARC-AGI Benchmark Results ===");
        println!("Tasks: {} | Solved: {} | Score: {:.1}%",
            self.total_tasks, self.solved, self.score * 100.0);
        println!("Time: {}ms | Avg MDL: {:.1}", self.elapsed_ms, self.avg_mdl);
        println!("\nBy method:");
        for (method, count) in &self.by_method {
            println!("  {}: {} ({:.1}%)", method, count,
                *count as f64 / self.solved.max(1) as f64 * 100.0);
        }
    }

    pub fn print_detail(&self) {
        self.print_summary();
        println!("\nPer-task detail:");
        for t in &self.per_task {
            let status = if t.solved { "OK" } else { "--" };
            println!("  [{}] {} | method={} size={} checked={} mdl={:.1} time={}ms",
                status, t.task_id, t.method, t.program_size, t.checked, t.mdl, t.elapsed_ms);
        }
    }
}
