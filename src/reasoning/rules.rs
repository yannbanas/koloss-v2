use crate::core::{Term, Sym, Result, KolossError};
use super::unifier::{Substitution, unify, rename_vars};
use super::builtins::{BuiltinRegistry, BuiltinResult, eval_builtin};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct Rule {
    pub head: Term,
    pub body: Vec<Term>,
    pub id: usize,
}

impl Rule {
    pub fn fact(head: Term) -> Self {
        Self { head, body: Vec::new(), id: 0 }
    }

    pub fn new(head: Term, body: Vec<Term>) -> Self {
        Self { head, body, id: 0 }
    }

    pub fn with_id(mut self, id: usize) -> Self {
        self.id = id;
        self
    }

    pub fn is_fact(&self) -> bool {
        self.body.is_empty()
    }

    pub fn rename(&self, offset: Sym) -> Rule {
        Rule {
            head: rename_vars(&self.head, offset),
            body: self.body.iter().map(|t| rename_vars(t, offset)).collect(),
            id: self.id,
        }
    }
}

// Tabling: cache for memoized query results
#[derive(Debug, Clone, Default)]
struct Table {
    entries: FxHashMap<u64, Vec<Substitution>>,
}

impl Table {
    fn key(goal: &Term) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = rustc_hash::FxHasher::default();
        goal.hash(&mut hasher);
        hasher.finish()
    }

    fn get(&self, goal: &Term) -> Option<&Vec<Substitution>> {
        self.entries.get(&Self::key(goal))
    }

    fn insert(&mut self, goal: &Term, results: Vec<Substitution>) {
        self.entries.insert(Self::key(goal), results);
    }

    fn clear(&mut self) {
        self.entries.clear();
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

// Signal for cut propagation
struct CutSignal;

#[derive(Debug, Clone)]
pub struct RuleEngine {
    rules: Vec<Rule>,
    facts: Vec<Term>,
    max_depth: usize,
    var_counter: Sym,
    builtins: BuiltinRegistry,
    table: Table,
    tabling_enabled: bool,
    tabled_functors: Vec<Sym>,
    not_sym: Option<Sym>,
    naf_sym: Option<Sym>,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            facts: Vec::new(),
            max_depth: 64,
            var_counter: 10000,
            builtins: BuiltinRegistry::new(),
            table: Table::default(),
            tabling_enabled: false,
            tabled_functors: Vec::new(),
            not_sym: None,
            naf_sym: None,
        }
    }

    pub fn with_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_tabling(mut self) -> Self {
        self.tabling_enabled = true;
        self
    }

    pub fn table_functor(&mut self, functor: Sym) {
        if !self.tabled_functors.contains(&functor) {
            self.tabled_functors.push(functor);
        }
        self.tabling_enabled = true;
    }

    pub fn set_not_sym(&mut self, sym: Sym) {
        self.not_sym = Some(sym);
    }

    pub fn set_naf_sym(&mut self, sym: Sym) {
        self.naf_sym = Some(sym);
    }

    pub fn builtins_mut(&mut self) -> &mut BuiltinRegistry {
        &mut self.builtins
    }

    pub fn builtins(&self) -> &BuiltinRegistry {
        &self.builtins
    }

    pub fn clear_tables(&mut self) {
        self.table.clear();
    }

    pub fn table_size(&self) -> usize {
        self.table.len()
    }

    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    pub fn add_fact(&mut self, fact: Term) {
        self.facts.push(fact);
    }

    pub fn num_rules(&self) -> usize {
        self.rules.len()
    }

    pub fn num_facts(&self) -> usize {
        self.facts.len()
    }

    pub fn query(&mut self, goal: &Term) -> Vec<Substitution> {
        let sub = Substitution::new();
        self.solve(goal, &sub, 0).unwrap_or_default()
    }

    pub fn query_first(&mut self, goal: &Term) -> Option<Substitution> {
        let sub = Substitution::new();
        self.solve_first(goal, &sub, 0)
    }

    pub fn query_all(&mut self, goals: &[Term]) -> Vec<Substitution> {
        let sub = Substitution::new();
        self.solve_conjunction(goals, &sub, 0).unwrap_or_default()
    }

    // Core solver — returns Err(CutSignal) if cut encountered
    fn solve(&mut self, goal: &Term, sub: &Substitution, depth: usize) -> std::result::Result<Vec<Substitution>, CutSignal> {
        if depth > self.max_depth {
            return Ok(Vec::new());
        }

        let resolved = sub.apply(goal);

        // Check NAF: \+(Goal) or not(Goal)
        if let Term::Compound(f, args) = &resolved {
            if args.len() == 1 {
                let is_not = self.not_sym.map_or(false, |s| *f == s);
                let is_naf = self.naf_sym.map_or(false, |s| *f == s);
                if is_not || is_naf {
                    return Ok(self.solve_naf(&args[0], sub, depth));
                }
            }
        }

        // Check builtins
        if let Term::Compound(f, args) = &resolved {
            if self.builtins.is_builtin(*f) {
                return self.solve_builtin(*f, args, sub);
            }
        }

        // Check tabling
        if self.tabling_enabled {
            if let Term::Compound(f, _) = &resolved {
                if self.tabled_functors.contains(f) {
                    if let Some(cached) = self.table.get(&resolved) {
                        return Ok(cached.clone());
                    }
                }
            }
        }

        let mut results = Vec::new();

        // Facts
        for fact in self.facts.clone() {
            if let Ok(s) = unify(&resolved, &fact, sub) {
                results.push(s);
            }
        }

        // Rules
        let rules: Vec<Rule> = self.rules.clone();
        let mut cut = false;
        for rule in &rules {
            if cut { break; }
            self.var_counter += 100;
            let renamed = rule.rename(self.var_counter);

            if let Ok(s) = unify(&resolved, &renamed.head, sub) {
                if renamed.body.is_empty() {
                    results.push(s);
                } else {
                    match self.solve_conjunction(&renamed.body, &s, depth + 1) {
                        Ok(body_results) => results.extend(body_results),
                        Err(CutSignal) => {
                            // Cut propagates: stop trying more rules, keep results found so far
                            // But we need to also get results from the cut branch
                            // Re-run but capture partial results up to cut
                            let partial = self.solve_conjunction_with_cut(&renamed.body, &s, depth + 1);
                            results.extend(partial);
                            cut = true;
                        }
                    }
                }
            }
        }

        // Cache if tabled
        if self.tabling_enabled {
            if let Term::Compound(f, _) = &resolved {
                if self.tabled_functors.contains(f) {
                    self.table.insert(&resolved, results.clone());
                }
            }
        }

        Ok(results)
    }

    fn solve_first(&mut self, goal: &Term, sub: &Substitution, depth: usize) -> Option<Substitution> {
        if depth > self.max_depth {
            return None;
        }

        let resolved = sub.apply(goal);

        // NAF
        if let Term::Compound(f, args) = &resolved {
            if args.len() == 1 {
                let is_not = self.not_sym.map_or(false, |s| *f == s);
                let is_naf = self.naf_sym.map_or(false, |s| *f == s);
                if is_not || is_naf {
                    let naf_results = self.solve_naf(&args[0], sub, depth);
                    return naf_results.into_iter().next();
                }
            }
        }

        // Builtins
        if let Term::Compound(f, args) = &resolved {
            if self.builtins.is_builtin(*f) {
                if let Ok(results) = self.solve_builtin(*f, args, sub) {
                    return results.into_iter().next();
                }
                return None;
            }
        }

        // Facts
        for fact in self.facts.clone() {
            if let Ok(s) = unify(&resolved, &fact, sub) {
                return Some(s);
            }
        }

        // Rules
        let rules: Vec<Rule> = self.rules.clone();
        for rule in &rules {
            self.var_counter += 100;
            let renamed = rule.rename(self.var_counter);

            if let Ok(s) = unify(&resolved, &renamed.head, sub) {
                if renamed.body.is_empty() {
                    return Some(s);
                }
                if let Some(result) = self.solve_conjunction_first(&renamed.body, &s, depth + 1) {
                    return Some(result);
                }
            }
        }

        None
    }

    // Negation as Failure: \+(Goal) succeeds iff Goal has no solutions
    fn solve_naf(&mut self, inner_goal: &Term, sub: &Substitution, depth: usize) -> Vec<Substitution> {
        let results = self.solve(inner_goal, sub, depth + 1).unwrap_or_default();
        if results.is_empty() {
            // Goal failed → negation succeeds (with original substitution, no new bindings)
            vec![sub.clone()]
        } else {
            // Goal succeeded → negation fails
            Vec::new()
        }
    }

    fn solve_builtin(&mut self, functor: Sym, args: &[Term], sub: &Substitution) -> std::result::Result<Vec<Substitution>, CutSignal> {
        match eval_builtin(functor, args, sub, &self.builtins) {
            Some(BuiltinResult::Success(s)) => Ok(vec![s]),
            Some(BuiltinResult::Fail) => Ok(Vec::new()),
            Some(BuiltinResult::Cut) => Err(CutSignal),
            Some(BuiltinResult::Multi(subs)) => Ok(subs),
            None => Ok(Vec::new()),
        }
    }

    fn solve_conjunction(&mut self, goals: &[Term], sub: &Substitution, depth: usize) -> std::result::Result<Vec<Substitution>, CutSignal> {
        if goals.is_empty() {
            return Ok(vec![sub.clone()]);
        }
        let first = sub.apply(&goals[0]);
        let rest = &goals[1..];
        let mut results = Vec::new();

        // Check if first goal is a cut
        if let Term::Compound(f, args) = &first {
            if args.is_empty() && self.builtins.name_of(*f) == Some("!") {
                // Cut: succeed once, then signal cut to parent
                let rest_results = self.solve_conjunction(rest, sub, depth)?;
                results.extend(rest_results);
                return Err(CutSignal);
            }
        }

        for s in self.solve(&first, sub, depth)? {
            match self.solve_conjunction(rest, &s, depth) {
                Ok(rest_results) => results.extend(rest_results),
                Err(CutSignal) => return Err(CutSignal),
            }
        }

        Ok(results)
    }

    // Variant that catches cut and returns partial results
    fn solve_conjunction_with_cut(&mut self, goals: &[Term], sub: &Substitution, depth: usize) -> Vec<Substitution> {
        if goals.is_empty() {
            return vec![sub.clone()];
        }
        let first = sub.apply(&goals[0]);
        let rest = &goals[1..];
        let mut results = Vec::new();

        // Handle cut goal
        if let Term::Compound(f, args) = &first {
            if args.is_empty() && self.builtins.name_of(*f) == Some("!") {
                results.extend(self.solve_conjunction_with_cut(rest, sub, depth));
                return results;
            }
        }

        let first_results = self.solve(&first, sub, depth).unwrap_or_default();
        for s in first_results {
            results.extend(self.solve_conjunction_with_cut(rest, &s, depth));
        }

        results
    }

    fn solve_conjunction_first(&mut self, goals: &[Term], sub: &Substitution, depth: usize) -> Option<Substitution> {
        if goals.is_empty() {
            return Some(sub.clone());
        }
        let first = sub.apply(&goals[0]);
        let rest = &goals[1..];

        // Handle cut goal
        if let Term::Compound(f, args) = &first {
            if args.is_empty() && self.builtins.name_of(*f) == Some("!") {
                return self.solve_conjunction_first(rest, sub, depth);
            }
        }

        for s in self.solve(&first, sub, depth).unwrap_or_default() {
            if let Some(result) = self.solve_conjunction_first(rest, &s, depth) {
                return Some(result);
            }
        }

        None
    }

    pub fn forward_chain(&mut self, max_iterations: usize) -> usize {
        let mut new_facts = 0;
        for _ in 0..max_iterations {
            let mut added = false;
            let rules: Vec<Rule> = self.rules.clone();

            for rule in &rules {
                if rule.body.is_empty() {
                    continue;
                }

                self.var_counter += 100;
                let renamed = rule.rename(self.var_counter);
                let sub = Substitution::new();
                let solutions = self.solve_conjunction(&renamed.body, &sub, 0).unwrap_or_default();

                for s in solutions {
                    let new_fact = s.apply(&renamed.head);
                    if new_fact.is_ground() && !self.facts.contains(&new_fact) {
                        self.facts.push(new_fact);
                        new_facts += 1;
                        added = true;
                    }
                }
            }

            if !added {
                break;
            }
        }
        new_facts
    }

    pub fn assert_fact(&mut self, fact: Term) -> Result<()> {
        if !fact.is_ground() {
            return Err(KolossError::InvalidTerm("fact must be ground".into()));
        }
        if !self.facts.contains(&fact) {
            self.facts.push(fact);
        }
        Ok(())
    }

    pub fn retract(&mut self, fact: &Term) -> bool {
        let before = self.facts.len();
        self.facts.retain(|f| f != fact);
        self.facts.len() < before
    }

    pub fn facts(&self) -> &[Term] {
        &self.facts
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
}
