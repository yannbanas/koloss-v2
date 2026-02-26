use rustc_hash::FxHashMap;

pub type Literal = i32;
pub type Clause = Vec<Literal>;
pub type Assignment = FxHashMap<u32, bool>;

#[derive(Debug, Clone)]
pub struct SatProblem {
    clauses: Vec<Clause>,
    num_vars: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SatResult {
    Sat(Assignment),
    Unsat,
}

impl SatProblem {
    pub fn new(num_vars: u32) -> Self {
        Self { clauses: Vec::new(), num_vars }
    }

    pub fn add_clause(&mut self, clause: Clause) {
        self.clauses.push(clause);
    }

    pub fn from_clauses(num_vars: u32, clauses: Vec<Clause>) -> Self {
        Self { clauses, num_vars }
    }

    pub fn solve(&self) -> SatResult {
        let mut assignment = Assignment::default();
        if dpll(&self.clauses, &mut assignment, self.num_vars) {
            SatResult::Sat(assignment)
        } else {
            SatResult::Unsat
        }
    }

    pub fn num_vars(&self) -> u32 {
        self.num_vars
    }

    pub fn num_clauses(&self) -> usize {
        self.clauses.len()
    }
}

fn dpll(clauses: &[Clause], assignment: &mut Assignment, num_vars: u32) -> bool {
    let simplified = simplify(clauses, assignment);

    if simplified.is_empty() {
        return true;
    }
    if simplified.iter().any(|c| c.is_empty()) {
        return false;
    }

    if let Some(unit) = find_unit(&simplified) {
        let var = unit.unsigned_abs();
        let val = unit > 0;
        assignment.insert(var, val);
        return dpll(&simplified, assignment, num_vars);
    }

    if let Some(pure) = find_pure_literal(&simplified, num_vars) {
        let var = pure.unsigned_abs();
        let val = pure > 0;
        assignment.insert(var, val);
        return dpll(&simplified, assignment, num_vars);
    }

    let var = pick_variable(&simplified, assignment, num_vars);
    if var == 0 {
        return false;
    }

    assignment.insert(var, true);
    if dpll(&simplified, assignment, num_vars) {
        return true;
    }

    assignment.insert(var, false);
    if dpll(&simplified, assignment, num_vars) {
        return true;
    }

    assignment.remove(&var);
    false
}

fn simplify(clauses: &[Clause], assignment: &Assignment) -> Vec<Clause> {
    let mut result = Vec::new();
    for clause in clauses {
        let mut satisfied = false;
        let mut remaining = Vec::new();
        for &lit in clause {
            let var = lit.unsigned_abs();
            let pos = lit > 0;
            match assignment.get(&var) {
                Some(&val) => {
                    if val == pos {
                        satisfied = true;
                        break;
                    }
                }
                None => remaining.push(lit),
            }
        }
        if !satisfied {
            result.push(remaining);
        }
    }
    result
}

fn find_unit(clauses: &[Clause]) -> Option<Literal> {
    clauses.iter().find(|c| c.len() == 1).map(|c| c[0])
}

fn find_pure_literal(clauses: &[Clause], num_vars: u32) -> Option<Literal> {
    for v in 1..=num_vars {
        let pos = clauses.iter().any(|c| c.contains(&(v as Literal)));
        let neg = clauses.iter().any(|c| c.contains(&-(v as Literal)));
        if pos && !neg {
            return Some(v as Literal);
        }
        if neg && !pos {
            return Some(-(v as Literal));
        }
    }
    None
}

fn pick_variable(clauses: &[Clause], assignment: &Assignment, _num_vars: u32) -> u32 {
    let mut counts: FxHashMap<u32, usize> = FxHashMap::default();
    for clause in clauses {
        for &lit in clause {
            let var = lit.unsigned_abs();
            if !assignment.contains_key(&var) {
                *counts.entry(var).or_default() += 1;
            }
        }
    }
    counts.into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(var, _)| var)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct ConstraintSolver {
    variables: Vec<ConstraintVar>,
    constraints: Vec<Constraint>,
}

#[derive(Debug, Clone)]
pub struct ConstraintVar {
    pub id: u32,
    pub domain: Vec<i64>,
}

pub enum Constraint {
    Equal(u32, u32),
    NotEqual(u32, u32),
    LessThan(u32, u32),
    Custom(u32, u32, std::sync::Arc<dyn Fn(i64, i64) -> bool + Send + Sync>),
}

impl Clone for Constraint {
    fn clone(&self) -> Self {
        match self {
            Self::Equal(a, b) => Self::Equal(*a, *b),
            Self::NotEqual(a, b) => Self::NotEqual(*a, *b),
            Self::LessThan(a, b) => Self::LessThan(*a, *b),
            Self::Custom(a, b, f) => Self::Custom(*a, *b, f.clone()),
        }
    }
}

impl std::fmt::Debug for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal(a, b) => write!(f, "Equal({}, {})", a, b),
            Self::NotEqual(a, b) => write!(f, "NotEqual({}, {})", a, b),
            Self::LessThan(a, b) => write!(f, "LessThan({}, {})", a, b),
            Self::Custom(a, b, _) => write!(f, "Custom({}, {})", a, b),
        }
    }
}

impl ConstraintSolver {
    pub fn new() -> Self {
        Self { variables: Vec::new(), constraints: Vec::new() }
    }

    pub fn add_var(&mut self, id: u32, domain: Vec<i64>) {
        self.variables.push(ConstraintVar { id, domain });
    }

    pub fn add_constraint(&mut self, c: Constraint) {
        self.constraints.push(c);
    }

    pub fn solve(&self) -> Option<FxHashMap<u32, i64>> {
        let mut assignment: FxHashMap<u32, i64> = FxHashMap::default();
        if self.backtrack(0, &mut assignment) {
            Some(assignment)
        } else {
            None
        }
    }

    fn backtrack(&self, idx: usize, assignment: &mut FxHashMap<u32, i64>) -> bool {
        if idx >= self.variables.len() {
            return true;
        }
        let var = &self.variables[idx];
        for &val in &var.domain {
            assignment.insert(var.id, val);
            if self.consistent(assignment) && self.backtrack(idx + 1, assignment) {
                return true;
            }
        }
        assignment.remove(&var.id);
        false
    }

    fn consistent(&self, assignment: &FxHashMap<u32, i64>) -> bool {
        for c in &self.constraints {
            match c {
                Constraint::Equal(a, b) => {
                    if let (Some(&va), Some(&vb)) = (assignment.get(a), assignment.get(b)) {
                        if va != vb { return false; }
                    }
                }
                Constraint::NotEqual(a, b) => {
                    if let (Some(&va), Some(&vb)) = (assignment.get(a), assignment.get(b)) {
                        if va == vb { return false; }
                    }
                }
                Constraint::LessThan(a, b) => {
                    if let (Some(&va), Some(&vb)) = (assignment.get(a), assignment.get(b)) {
                        if va >= vb { return false; }
                    }
                }
                Constraint::Custom(a, b, f) => {
                    if let (Some(&va), Some(&vb)) = (assignment.get(a), assignment.get(b)) {
                        if !f(va, vb) { return false; }
                    }
                }
            }
        }
        true
    }
}
