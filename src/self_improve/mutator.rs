use crate::reasoning::rules::{Rule, RuleEngine};
use crate::core::Term;

#[derive(Debug, Clone)]
pub enum Mutation {
    AddRule(Rule),
    RemoveRule(usize),
    ModifyRuleHead(usize, Term),
    AddFact(Term),
    RetractFact(Term),
    SwapRules(usize, usize),
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
        Mutation::RemoveRule(_) | Mutation::ModifyRuleHead(_, _) | Mutation::SwapRules(_, _) => {
            false
        }
    }
}

pub fn generate_mutations(engine: &RuleEngine) -> Vec<Mutation> {
    let mut mutations = Vec::new();

    for (i, _rule) in engine.rules().iter().enumerate() {
        mutations.push(Mutation::RemoveRule(i));
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
