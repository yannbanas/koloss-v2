use crate::core::{Term, Sym, Result, KolossError};
use rustc_hash::FxHashMap;

#[derive(Debug, Clone, Default)]
pub struct Substitution {
    bindings: FxHashMap<Sym, Term>,
}

impl Substitution {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn bind(&mut self, var: Sym, term: Term) {
        self.bindings.insert(var, term);
    }

    pub fn lookup(&self, var: Sym) -> Option<&Term> {
        self.bindings.get(&var)
    }

    pub fn walk(&self, term: &Term) -> Term {
        match term {
            Term::Var(v) => {
                match self.bindings.get(v) {
                    Some(bound) => self.walk(bound),
                    None => term.clone(),
                }
            }
            _ => term.clone(),
        }
    }

    pub fn walk_deep(&self, term: &Term) -> Term {
        let walked = self.walk(term);
        match walked {
            Term::Compound(f, args) => {
                Term::Compound(f, args.iter().map(|a| self.walk_deep(a)).collect())
            }
            Term::List(items) => {
                Term::List(items.iter().map(|a| self.walk_deep(a)).collect())
            }
            other => other,
        }
    }

    pub fn apply(&self, term: &Term) -> Term {
        self.walk_deep(term)
    }

    pub fn compose(&self, other: &Substitution) -> Substitution {
        let mut result = Substitution::new();
        for (&var, term) in &self.bindings {
            result.bind(var, other.apply(term));
        }
        for (&var, term) in &other.bindings {
            if !result.bindings.contains_key(&var) {
                result.bind(var, term.clone());
            }
        }
        result
    }

    pub fn bindings(&self) -> &FxHashMap<Sym, Term> {
        &self.bindings
    }

    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

pub fn unify(t1: &Term, t2: &Term, sub: &Substitution) -> Result<Substitution> {
    let w1 = sub.walk(t1);
    let w2 = sub.walk(t2);

    match (&w1, &w2) {
        _ if w1 == w2 => Ok(sub.clone()),

        (Term::Var(v), _) => {
            if occurs_check(*v, &w2, sub) {
                return Err(KolossError::UnificationFail(
                    format!("occurs check: ?{} in {}", v, w2)
                ));
            }
            let mut s = sub.clone();
            s.bind(*v, w2);
            Ok(s)
        }

        (_, Term::Var(v)) => {
            if occurs_check(*v, &w1, sub) {
                return Err(KolossError::UnificationFail(
                    format!("occurs check: ?{} in {}", v, w1)
                ));
            }
            let mut s = sub.clone();
            s.bind(*v, w1);
            Ok(s)
        }

        (Term::Compound(f1, args1), Term::Compound(f2, args2)) => {
            if f1 != f2 || args1.len() != args2.len() {
                return Err(KolossError::UnificationFail(
                    format!("functor mismatch: {} vs {}", f1, f2)
                ));
            }
            let mut s = sub.clone();
            for (a1, a2) in args1.iter().zip(args2.iter()) {
                s = unify(a1, a2, &s)?;
            }
            Ok(s)
        }

        (Term::List(l1), Term::List(l2)) => {
            if l1.len() != l2.len() {
                return Err(KolossError::UnificationFail(
                    format!("list length mismatch: {} vs {}", l1.len(), l2.len())
                ));
            }
            let mut s = sub.clone();
            for (a, b) in l1.iter().zip(l2.iter()) {
                s = unify(a, b, &s)?;
            }
            Ok(s)
        }

        _ => Err(KolossError::UnificationFail(
            format!("cannot unify {} with {}", w1, w2)
        )),
    }
}

fn occurs_check(var: Sym, term: &Term, sub: &Substitution) -> bool {
    let walked = sub.walk(term);
    match &walked {
        Term::Var(v) => *v == var,
        Term::Compound(_, args) | Term::List(args) => {
            args.iter().any(|a| occurs_check(var, a, sub))
        }
        _ => false,
    }
}

pub fn unify_lists(pairs: &[(Term, Term)]) -> Result<Substitution> {
    let mut sub = Substitution::new();
    for (t1, t2) in pairs {
        sub = unify(t1, t2, &sub)?;
    }
    Ok(sub)
}

pub fn rename_vars(term: &Term, offset: Sym) -> Term {
    match term {
        Term::Var(v) => Term::Var(*v + offset),
        Term::Compound(f, args) => {
            Term::Compound(*f, args.iter().map(|a| rename_vars(a, offset)).collect())
        }
        Term::List(items) => {
            Term::List(items.iter().map(|a| rename_vars(a, offset)).collect())
        }
        other => other.clone(),
    }
}
