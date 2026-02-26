use crate::core::{Term, Sym, OrderedFloat};
use super::unifier::Substitution;

pub const BUILTIN_IS: &str = "is";
pub const BUILTIN_GT: &str = ">";
pub const BUILTIN_LT: &str = "<";
pub const BUILTIN_GTE: &str = ">=";
pub const BUILTIN_LTE: &str = "<=";
pub const BUILTIN_EQ: &str = "=:=";
pub const BUILTIN_NEQ: &str = "=\\=";
pub const BUILTIN_PLUS: &str = "+";
pub const BUILTIN_MINUS: &str = "-";
pub const BUILTIN_MUL: &str = "*";
pub const BUILTIN_DIV: &str = "/";
pub const BUILTIN_MOD: &str = "mod";
pub const BUILTIN_ABS: &str = "abs";
pub const BUILTIN_MAX: &str = "max";
pub const BUILTIN_MIN: &str = "min";
pub const BUILTIN_NOT: &str = "not";
pub const BUILTIN_CUT: &str = "!";
pub const BUILTIN_TRUE: &str = "true";
pub const BUILTIN_FAIL: &str = "fail";
pub const BUILTIN_VAR: &str = "var";
pub const BUILTIN_NONVAR: &str = "nonvar";
pub const BUILTIN_ATOM: &str = "atom";
pub const BUILTIN_INTEGER: &str = "integer";
pub const BUILTIN_IS_LIST: &str = "is_list";
pub const BUILTIN_LENGTH: &str = "length";
pub const BUILTIN_APPEND: &str = "append";
pub const BUILTIN_MEMBER: &str = "member";
pub const BUILTIN_BETWEEN: &str = "between";
pub const BUILTIN_SUCC: &str = "succ";
pub const BUILTIN_PLUS_OP: &str = "plus";
pub const BUILTIN_WRITE: &str = "write";
pub const BUILTIN_NL: &str = "nl";
pub const BUILTIN_GROUND: &str = "ground";
pub const BUILTIN_COPY_TERM: &str = "copy_term";
pub const BUILTIN_FUNCTOR: &str = "functor";
pub const BUILTIN_ARG: &str = "arg";
pub const BUILTIN_FINDALL: &str = "findall";

#[derive(Debug, Clone)]
pub struct BuiltinRegistry {
    symbols: Vec<(String, Sym)>,
}

impl BuiltinRegistry {
    pub fn new() -> Self {
        Self { symbols: Vec::new() }
    }

    pub fn register(&mut self, name: &str, sym: Sym) {
        self.symbols.push((name.to_string(), sym));
    }

    pub fn is_builtin(&self, functor: Sym) -> bool {
        self.symbols.iter().any(|(_, s)| *s == functor)
    }

    pub fn name_of(&self, functor: Sym) -> Option<&str> {
        self.symbols.iter().find(|(_, s)| *s == functor).map(|(n, _)| n.as_str())
    }

    pub fn sym_of(&self, name: &str) -> Option<Sym> {
        self.symbols.iter().find(|(n, _)| n == name).map(|(_, s)| *s)
    }
}

pub fn eval_arithmetic(term: &Term, sub: &Substitution, builtins: &BuiltinRegistry) -> Option<f64> {
    let resolved = sub.apply(term);
    match &resolved {
        Term::Int(n) => Some(*n as f64),
        Term::Float(f) => Some(f.val()),
        Term::Compound(func, args) => {
            let name = builtins.name_of(*func)?;
            match (name, args.len()) {
                (BUILTIN_PLUS, 2) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    let b = eval_arithmetic(&args[1], sub, builtins)?;
                    Some(a + b)
                }
                (BUILTIN_MINUS, 2) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    let b = eval_arithmetic(&args[1], sub, builtins)?;
                    Some(a - b)
                }
                (BUILTIN_MINUS, 1) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    Some(-a)
                }
                (BUILTIN_MUL, 2) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    let b = eval_arithmetic(&args[1], sub, builtins)?;
                    Some(a * b)
                }
                (BUILTIN_DIV, 2) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    let b = eval_arithmetic(&args[1], sub, builtins)?;
                    if b == 0.0 { None } else { Some(a / b) }
                }
                (BUILTIN_MOD, 2) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)? as i64;
                    let b = eval_arithmetic(&args[1], sub, builtins)? as i64;
                    if b == 0 { None } else { Some((a % b) as f64) }
                }
                (BUILTIN_ABS, 1) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    Some(a.abs())
                }
                (BUILTIN_MAX, 2) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    let b = eval_arithmetic(&args[1], sub, builtins)?;
                    Some(a.max(b))
                }
                (BUILTIN_MIN, 2) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    let b = eval_arithmetic(&args[1], sub, builtins)?;
                    Some(a.min(b))
                }
                (BUILTIN_SUCC, 1) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    Some(a + 1.0)
                }
                (BUILTIN_PLUS_OP, 2) => {
                    let a = eval_arithmetic(&args[0], sub, builtins)?;
                    let b = eval_arithmetic(&args[1], sub, builtins)?;
                    Some(a + b)
                }
                _ => None,
            }
        }
        _ => None,
    }
}

pub fn term_from_number(n: f64) -> Term {
    if n.fract() == 0.0 && n.abs() < i64::MAX as f64 {
        Term::Int(n as i64)
    } else {
        Term::Float(OrderedFloat::new(n))
    }
}

pub enum BuiltinResult {
    Success(Substitution),
    Fail,
    Cut,
    Multi(Vec<Substitution>),
}

pub fn eval_builtin(
    functor: Sym,
    args: &[Term],
    sub: &Substitution,
    builtins: &BuiltinRegistry,
) -> Option<BuiltinResult> {
    let name = builtins.name_of(functor)?;

    match name {
        BUILTIN_TRUE => Some(BuiltinResult::Success(sub.clone())),
        BUILTIN_FAIL => Some(BuiltinResult::Fail),
        BUILTIN_CUT => Some(BuiltinResult::Cut),

        BUILTIN_IS => {
            if args.len() != 2 { return Some(BuiltinResult::Fail); }
            let val = eval_arithmetic(&args[1], sub, builtins)?;
            let result_term = term_from_number(val);
            let target = sub.apply(&args[0]);
            match &target {
                Term::Var(_) => {
                    let mut s = sub.clone();
                    if let Term::Var(v) = &args[0] {
                        s.bind(*v, result_term);
                    } else {
                        let walked = sub.walk(&args[0]);
                        if let Term::Var(v) = walked {
                            s.bind(v, result_term);
                        } else {
                            return Some(BuiltinResult::Fail);
                        }
                    }
                    Some(BuiltinResult::Success(s))
                }
                Term::Int(n) => {
                    if *n as f64 == val { Some(BuiltinResult::Success(sub.clone())) }
                    else { Some(BuiltinResult::Fail) }
                }
                Term::Float(f) => {
                    if f.val() == val { Some(BuiltinResult::Success(sub.clone())) }
                    else { Some(BuiltinResult::Fail) }
                }
                _ => Some(BuiltinResult::Fail),
            }
        }

        BUILTIN_GT => {
            if args.len() != 2 { return Some(BuiltinResult::Fail); }
            let a = eval_arithmetic(&args[0], sub, builtins)?;
            let b = eval_arithmetic(&args[1], sub, builtins)?;
            if a > b { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_LT => {
            if args.len() != 2 { return Some(BuiltinResult::Fail); }
            let a = eval_arithmetic(&args[0], sub, builtins)?;
            let b = eval_arithmetic(&args[1], sub, builtins)?;
            if a < b { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_GTE => {
            if args.len() != 2 { return Some(BuiltinResult::Fail); }
            let a = eval_arithmetic(&args[0], sub, builtins)?;
            let b = eval_arithmetic(&args[1], sub, builtins)?;
            if a >= b { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_LTE => {
            if args.len() != 2 { return Some(BuiltinResult::Fail); }
            let a = eval_arithmetic(&args[0], sub, builtins)?;
            let b = eval_arithmetic(&args[1], sub, builtins)?;
            if a <= b { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_EQ => {
            if args.len() != 2 { return Some(BuiltinResult::Fail); }
            let a = eval_arithmetic(&args[0], sub, builtins)?;
            let b = eval_arithmetic(&args[1], sub, builtins)?;
            if (a - b).abs() < f64::EPSILON { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_NEQ => {
            if args.len() != 2 { return Some(BuiltinResult::Fail); }
            let a = eval_arithmetic(&args[0], sub, builtins)?;
            let b = eval_arithmetic(&args[1], sub, builtins)?;
            if (a - b).abs() >= f64::EPSILON { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_VAR => {
            if args.len() != 1 { return Some(BuiltinResult::Fail); }
            let resolved = sub.apply(&args[0]);
            if matches!(resolved, Term::Var(_)) { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_NONVAR => {
            if args.len() != 1 { return Some(BuiltinResult::Fail); }
            let resolved = sub.apply(&args[0]);
            if !matches!(resolved, Term::Var(_)) { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_ATOM => {
            if args.len() != 1 { return Some(BuiltinResult::Fail); }
            let resolved = sub.apply(&args[0]);
            if matches!(resolved, Term::Atom(_)) { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_INTEGER => {
            if args.len() != 1 { return Some(BuiltinResult::Fail); }
            let resolved = sub.apply(&args[0]);
            if matches!(resolved, Term::Int(_)) { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_GROUND => {
            if args.len() != 1 { return Some(BuiltinResult::Fail); }
            let resolved = sub.apply(&args[0]);
            if resolved.is_ground() { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_IS_LIST => {
            if args.len() != 1 { return Some(BuiltinResult::Fail); }
            let resolved = sub.apply(&args[0]);
            if matches!(resolved, Term::List(_)) { Some(BuiltinResult::Success(sub.clone())) }
            else { Some(BuiltinResult::Fail) }
        }

        BUILTIN_LENGTH => {
            if args.len() != 2 { return Some(BuiltinResult::Fail); }
            let list = sub.apply(&args[0]);
            if let Term::List(items) = list {
                let len_term = Term::Int(items.len() as i64);
                let target = sub.apply(&args[1]);
                match target {
                    Term::Var(_) => {
                        let mut s = sub.clone();
                        if let Term::Var(v) = sub.walk(&args[1]) {
                            s.bind(v, len_term);
                        }
                        Some(BuiltinResult::Success(s))
                    }
                    Term::Int(n) if n == items.len() as i64 => {
                        Some(BuiltinResult::Success(sub.clone()))
                    }
                    _ => Some(BuiltinResult::Fail),
                }
            } else {
                Some(BuiltinResult::Fail)
            }
        }

        BUILTIN_MEMBER => {
            if args.len() != 2 { return Some(BuiltinResult::Fail); }
            let list = sub.apply(&args[1]);
            if let Term::List(items) = list {
                let mut results = Vec::new();
                for item in &items {
                    if let Ok(s) = super::unifier::unify(&args[0], item, sub) {
                        results.push(s);
                    }
                }
                if results.is_empty() {
                    Some(BuiltinResult::Fail)
                } else {
                    Some(BuiltinResult::Multi(results))
                }
            } else {
                Some(BuiltinResult::Fail)
            }
        }

        BUILTIN_APPEND => {
            if args.len() != 3 { return Some(BuiltinResult::Fail); }
            let l1 = sub.apply(&args[0]);
            let l2 = sub.apply(&args[1]);
            match (&l1, &l2) {
                (Term::List(a), Term::List(b)) => {
                    let mut merged = a.clone();
                    merged.extend(b.iter().cloned());
                    let result = Term::List(merged);
                    if let Ok(s) = super::unifier::unify(&args[2], &result, sub) {
                        Some(BuiltinResult::Success(s))
                    } else {
                        Some(BuiltinResult::Fail)
                    }
                }
                _ => Some(BuiltinResult::Fail),
            }
        }

        BUILTIN_BETWEEN => {
            if args.len() != 3 { return Some(BuiltinResult::Fail); }
            let lo = eval_arithmetic(&args[0], sub, builtins)? as i64;
            let hi = eval_arithmetic(&args[1], sub, builtins)? as i64;
            if lo > hi { return Some(BuiltinResult::Fail); }
            let target = sub.apply(&args[2]);
            match target {
                Term::Var(_) => {
                    let var_sym = if let Term::Var(v) = sub.walk(&args[2]) { v } else { return Some(BuiltinResult::Fail); };
                    let mut results = Vec::new();
                    for i in lo..=hi {
                        let mut s = sub.clone();
                        s.bind(var_sym, Term::Int(i));
                        results.push(s);
                    }
                    Some(BuiltinResult::Multi(results))
                }
                Term::Int(n) => {
                    if n >= lo && n <= hi { Some(BuiltinResult::Success(sub.clone())) }
                    else { Some(BuiltinResult::Fail) }
                }
                _ => Some(BuiltinResult::Fail),
            }
        }

        BUILTIN_WRITE => {
            if args.len() != 1 { return Some(BuiltinResult::Fail); }
            let resolved = sub.apply(&args[0]);
            print!("{}", resolved);
            Some(BuiltinResult::Success(sub.clone()))
        }

        BUILTIN_NL => {
            println!();
            Some(BuiltinResult::Success(sub.clone()))
        }

        BUILTIN_FUNCTOR => {
            if args.len() != 3 { return Some(BuiltinResult::Fail); }
            let term = sub.apply(&args[0]);
            match &term {
                Term::Compound(f, a) => {
                    let f_term = Term::Atom(*f);
                    let a_term = Term::Int(a.len() as i64);
                    let s = super::unifier::unify(&args[1], &f_term, sub).ok()?;
                    let s = super::unifier::unify(&args[2], &a_term, &s).ok()?;
                    Some(BuiltinResult::Success(s))
                }
                Term::Atom(a) => {
                    let f_term = Term::Atom(*a);
                    let a_term = Term::Int(0);
                    let s = super::unifier::unify(&args[1], &f_term, sub).ok()?;
                    let s = super::unifier::unify(&args[2], &a_term, &s).ok()?;
                    Some(BuiltinResult::Success(s))
                }
                _ => Some(BuiltinResult::Fail),
            }
        }

        BUILTIN_ARG => {
            if args.len() != 3 { return Some(BuiltinResult::Fail); }
            let n = eval_arithmetic(&args[0], sub, builtins)? as usize;
            let term = sub.apply(&args[1]);
            if let Term::Compound(_, a) = &term {
                if n >= 1 && n <= a.len() {
                    if let Ok(s) = super::unifier::unify(&args[2], &a[n - 1], sub) {
                        return Some(BuiltinResult::Success(s));
                    }
                }
            }
            Some(BuiltinResult::Fail)
        }

        _ => None,
    }
}
