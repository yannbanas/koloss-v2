use std::fmt;

#[derive(Debug)]
pub enum KolossError {
    UnificationFail(String),
    Unsatisfiable,
    NoRuleMatch(String),
    CyclicDependency,
    DepthExceeded(usize),
    SynthesisFail(String),
    MemoryFull,
    InvalidTerm(String),
}

impl fmt::Display for KolossError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnificationFail(msg) => write!(f, "unification failed: {}", msg),
            Self::Unsatisfiable => write!(f, "unsatisfiable"),
            Self::NoRuleMatch(msg) => write!(f, "no rule matches: {}", msg),
            Self::CyclicDependency => write!(f, "cyclic dependency detected"),
            Self::DepthExceeded(d) => write!(f, "depth exceeded: {}", d),
            Self::SynthesisFail(msg) => write!(f, "synthesis failed: {}", msg),
            Self::MemoryFull => write!(f, "memory full"),
            Self::InvalidTerm(msg) => write!(f, "invalid term: {}", msg),
        }
    }
}

impl std::error::Error for KolossError {}

pub type Result<T> = std::result::Result<T, KolossError>;
