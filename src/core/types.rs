use std::fmt;

pub type Sym = u32;

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Term {
    Var(Sym),
    Atom(Sym),
    Int(i64),
    Float(OrderedFloat),
    Str(Box<str>),
    Bool(bool),
    Compound(Sym, Vec<Term>),
    List(Vec<Term>),
    Nil,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct OrderedFloat(pub u64);

impl OrderedFloat {
    pub fn new(f: f64) -> Self {
        Self(f.to_bits())
    }
    pub fn val(self) -> f64 {
        f64::from_bits(self.0)
    }
}

impl fmt::Debug for OrderedFloat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.val())
    }
}

impl Term {
    pub fn var(id: Sym) -> Self {
        Term::Var(id)
    }

    pub fn atom(id: Sym) -> Self {
        Term::Atom(id)
    }

    pub fn int(n: i64) -> Self {
        Term::Int(n)
    }

    pub fn float(f: f64) -> Self {
        Term::Float(OrderedFloat::new(f))
    }

    pub fn compound(functor: Sym, args: Vec<Term>) -> Self {
        Term::Compound(functor, args)
    }

    pub fn list(items: Vec<Term>) -> Self {
        Term::List(items)
    }

    pub fn is_ground(&self) -> bool {
        match self {
            Term::Var(_) => false,
            Term::Atom(_) | Term::Int(_) | Term::Float(_) | Term::Str(_)
            | Term::Bool(_) | Term::Nil => true,
            Term::Compound(_, args) | Term::List(args) => args.iter().all(|a| a.is_ground()),
        }
    }

    pub fn vars(&self) -> Vec<Sym> {
        let mut out = Vec::new();
        self.collect_vars(&mut out);
        out
    }

    fn collect_vars(&self, out: &mut Vec<Sym>) {
        match self {
            Term::Var(v) => {
                if !out.contains(v) {
                    out.push(*v);
                }
            }
            Term::Compound(_, args) | Term::List(args) => {
                for a in args {
                    a.collect_vars(out);
                }
            }
            _ => {}
        }
    }

    pub fn substitute(&self, var: Sym, replacement: &Term) -> Term {
        match self {
            Term::Var(v) if *v == var => replacement.clone(),
            Term::Compound(f, args) => {
                Term::Compound(*f, args.iter().map(|a| a.substitute(var, replacement)).collect())
            }
            Term::List(items) => {
                Term::List(items.iter().map(|a| a.substitute(var, replacement)).collect())
            }
            other => other.clone(),
        }
    }

    pub fn size(&self) -> usize {
        match self {
            Term::Compound(_, args) | Term::List(args) => {
                1 + args.iter().map(|a| a.size()).sum::<usize>()
            }
            _ => 1,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    symbols: Vec<Box<str>>,
    index: rustc_hash::FxHashMap<Box<str>, Sym>,
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, name: &str) -> Sym {
        if let Some(&id) = self.index.get(name) {
            return id;
        }
        let id = self.symbols.len() as Sym;
        let boxed: Box<str> = name.into();
        self.index.insert(boxed.clone(), id);
        self.symbols.push(boxed);
        id
    }

    pub fn resolve(&self, id: Sym) -> Option<&str> {
        self.symbols.get(id as usize).map(|s| &**s)
    }

    pub fn len(&self) -> usize {
        self.symbols.len()
    }

    pub fn is_empty(&self) -> bool {
        self.symbols.is_empty()
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Term::Var(v) => write!(f, "?{}", v),
            Term::Atom(a) => write!(f, ":{}", a),
            Term::Int(n) => write!(f, "{}", n),
            Term::Float(fl) => write!(f, "{}", fl.val()),
            Term::Str(s) => write!(f, "\"{}\"", s),
            Term::Bool(b) => write!(f, "{}", b),
            Term::Nil => write!(f, "nil"),
            Term::Compound(func, args) => {
                write!(f, "{}(", func)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
            Term::List(items) => {
                write!(f, "[")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
        }
    }
}
