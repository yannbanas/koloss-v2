use std::time::Instant;
use crate::core::Term;
use crate::reasoning::rules::RuleEngine;

#[derive(Debug, Clone)]
pub struct FitnessScore {
    pub accuracy: f64,
    pub code_size: usize,
    pub latency_ms: u64,
    pub memory_bytes: usize,
    pub composite: f64,
}

impl FitnessScore {
    pub fn compute(accuracy: f64, code_size: usize, latency_ms: u64, memory_bytes: usize) -> Self {
        let size_factor = 1.0 / (1.0 + code_size as f64 / 10000.0);
        let speed_factor = 1.0 / (1.0 + latency_ms as f64 / 1000.0);
        let mem_factor = 1.0 / (1.0 + memory_bytes as f64 / 100_000_000.0);
        let composite = accuracy * 0.60 + size_factor * 0.20 + speed_factor * 0.10 + mem_factor * 0.10;
        Self { accuracy, code_size, latency_ms, memory_bytes, composite }
    }

    pub fn is_improvement_over(&self, other: &FitnessScore) -> bool {
        self.composite > other.composite + 0.001
    }
}

#[derive(Debug, Clone)]
pub struct TestCase {
    pub query: Term,
    pub expected_var: u32,
    pub expected_values: Vec<Term>,
}

pub fn evaluate_engine(engine: &mut RuleEngine, test_cases: &[TestCase]) -> f64 {
    if test_cases.is_empty() { return 0.0; }
    let mut correct = 0;
    for tc in test_cases {
        let results = engine.query(&tc.query);
        let actual: Vec<Term> = results.iter()
            .map(|s| s.apply(&Term::var(tc.expected_var)))
            .collect();
        let matches = tc.expected_values.iter().all(|ev| actual.contains(ev))
            && actual.len() == tc.expected_values.len();
        if matches { correct += 1; }
    }
    correct as f64 / test_cases.len() as f64
}

pub fn evaluate_engine_partial(engine: &mut RuleEngine, test_cases: &[TestCase]) -> f64 {
    if test_cases.is_empty() { return 0.0; }
    let mut score = 0.0;
    for tc in test_cases {
        let results = engine.query(&tc.query);
        let actual: Vec<Term> = results.iter()
            .map(|s| s.apply(&Term::var(tc.expected_var)))
            .collect();
        let found = tc.expected_values.iter().filter(|ev| actual.contains(ev)).count();
        let expected_count = tc.expected_values.len().max(1);
        let precision = if actual.is_empty() { 0.0 } else { found as f64 / actual.len() as f64 };
        let recall = found as f64 / expected_count as f64;
        score += if precision + recall > 0.0 { 2.0 * precision * recall / (precision + recall) } else { 0.0 };
    }
    score / test_cases.len() as f64
}

pub fn measure_accuracy<F: Fn(&[u8]) -> Vec<u8>>(
    f: &F,
    test_cases: &[(Vec<u8>, Vec<u8>)],
) -> f64 {
    if test_cases.is_empty() { return 0.0; }
    let passed = test_cases.iter()
        .filter(|(input, expected)| f(input) == *expected)
        .count();
    passed as f64 / test_cases.len() as f64
}

pub fn measure_latency<F: Fn()>(f: &F, iterations: usize) -> u64 {
    let start = Instant::now();
    for _ in 0..iterations {
        f();
    }
    start.elapsed().as_millis() as u64 / iterations.max(1) as u64
}

pub fn benchmark_engine(engine: &mut RuleEngine, test_cases: &[TestCase], iterations: usize) -> FitnessScore {
    let accuracy = evaluate_engine(engine, test_cases);
    let code_size = engine.num_rules() + engine.num_facts();
    let start = Instant::now();
    for _ in 0..iterations {
        for tc in test_cases {
            let _ = engine.query(&tc.query);
        }
    }
    let latency_ms = start.elapsed().as_millis() as u64 / iterations.max(1) as u64;
    FitnessScore::compute(accuracy, code_size, latency_ms, 0)
}
