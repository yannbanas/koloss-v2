use std::time::Instant;

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
    start.elapsed().as_millis() as u64 / iterations as u64
}
