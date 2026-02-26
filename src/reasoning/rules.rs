use crate::core::{Term, Sym, Result, KolossError};
use super::unifier::{Substitution, unify, rename_vars};

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

#[derive(Debug, Clone)]
pub struct RuleEngine {
    rules: Vec<Rule>,
    facts: Vec<Term>,
    max_depth: usize,
    var_counter: Sym,
}

impl RuleEngine {
    pub fn new() -> Self {
        Self { rules: Vec::new(), facts: Vec::new(), max_depth: 64, var_counter: 10000 }
    }

    pub fn with_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
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
        self.solve(goal, &sub, 0)
    }

    pub fn query_first(&mut self, goal: &Term) -> Option<Substitution> {
        let sub = Substitution::new();
        self.solve_first(goal, &sub, 0)
    }

    pub fn query_all(&mut self, goals: &[Term]) -> Vec<Substitution> {
        let sub = Substitution::new();
        self.solve_conjunction(goals, &sub, 0)
    }

    fn solve(&mut self, goal: &Term, sub: &Substitution, depth: usize) -> Vec<Substitution> {
        if depth > self.max_depth {
            return Vec::new();
        }
        let mut results = Vec::new();

        for fact in self.facts.clone() {
            if let Ok(s) = unify(goal, &fact, sub) {
                results.push(s);
            }
        }

        let rules: Vec<Rule> = self.rules.clone();
        for rule in &rules {
            self.var_counter += 100;
            let renamed = rule.rename(self.var_counter);

            if let Ok(s) = unify(goal, &renamed.head, sub) {
                if renamed.body.is_empty() {
                    results.push(s);
                } else {
                    let body_results = self.solve_conjunction(&renamed.body, &s, depth + 1);
                    results.extend(body_results);
                }
            }
        }

        results
    }

    fn solve_first(&mut self, goal: &Term, sub: &Substitution, depth: usize) -> Option<Substitution> {
        if depth > self.max_depth {
            return None;
        }

        for fact in self.facts.clone() {
            if let Ok(s) = unify(goal, &fact, sub) {
                return Some(s);
            }
        }

        let rules: Vec<Rule> = self.rules.clone();
        for rule in &rules {
            self.var_counter += 100;
            let renamed = rule.rename(self.var_counter);

            if let Ok(s) = unify(goal, &renamed.head, sub) {
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

    fn solve_conjunction(&mut self, goals: &[Term], sub: &Substitution, depth: usize) -> Vec<Substitution> {
        if goals.is_empty() {
            return vec![sub.clone()];
        }
        let first = sub.apply(&goals[0]);
        let rest = &goals[1..];
        let mut results = Vec::new();

        for s in self.solve(&first, sub, depth) {
            results.extend(self.solve_conjunction(rest, &s, depth));
        }

        results
    }

    fn solve_conjunction_first(&mut self, goals: &[Term], sub: &Substitution, depth: usize) -> Option<Substitution> {
        if goals.is_empty() {
            return Some(sub.clone());
        }
        let first = sub.apply(&goals[0]);
        let rest = &goals[1..];

        for s in self.solve(&first, sub, depth) {
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
                let solutions = self.solve_conjunction(&renamed.body, &sub, 0);

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
