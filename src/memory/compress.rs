use crate::core::Term;

#[derive(Debug, Clone)]
pub struct GeneralizedRule {
    pub pattern: Term,
    pub examples: Vec<Term>,
    pub confidence: f64,
    pub support: usize,
}

pub fn anti_unify(t1: &Term, t2: &Term, next_var: &mut u32) -> Term {
    match (t1, t2) {
        (a, b) if a == b => a.clone(),

        (Term::Compound(f1, args1), Term::Compound(f2, args2)) if f1 == f2 && args1.len() == args2.len() => {
            let args: Vec<Term> = args1.iter().zip(args2.iter())
                .map(|(a, b)| anti_unify(a, b, next_var))
                .collect();
            Term::Compound(*f1, args)
        }

        (Term::List(l1), Term::List(l2)) if l1.len() == l2.len() => {
            let items: Vec<Term> = l1.iter().zip(l2.iter())
                .map(|(a, b)| anti_unify(a, b, next_var))
                .collect();
            Term::List(items)
        }

        _ => {
            let v = *next_var;
            *next_var += 1;
            Term::var(v)
        }
    }
}

pub fn generalize_terms(terms: &[Term]) -> Option<GeneralizedRule> {
    if terms.len() < 2 {
        return None;
    }
    let mut var_counter = 50000u32;
    let mut pattern = anti_unify(&terms[0], &terms[1], &mut var_counter);

    for t in &terms[2..] {
        pattern = anti_unify(&pattern, t, &mut var_counter);
    }

    if pattern.vars().is_empty() && pattern == terms[0] {
        return None;
    }

    let support = terms.len();
    let specificity = 1.0 - (pattern.vars().len() as f64 / pattern.size() as f64).min(1.0);

    Some(GeneralizedRule {
        pattern,
        examples: terms.to_vec(),
        confidence: specificity,
        support,
    })
}

pub fn compress_facts(facts: &[Term], min_support: usize) -> Vec<GeneralizedRule> {
    let mut groups: rustc_hash::FxHashMap<String, Vec<&Term>> = rustc_hash::FxHashMap::default();

    for fact in facts {
        let key = match fact {
            Term::Compound(f, args) => format!("{}/{}", f, args.len()),
            _ => "leaf".into(),
        };
        groups.entry(key).or_default().push(fact);
    }

    let mut rules = Vec::new();
    for (_key, group) in &groups {
        if group.len() >= min_support {
            let owned: Vec<Term> = group.iter().map(|&t| t.clone()).collect();
            if let Some(rule) = generalize_terms(&owned) {
                if rule.confidence > 0.1 {
                    rules.push(rule);
                }
            }
        }
    }

    rules.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal));
    rules
}
