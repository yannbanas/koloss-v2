use crate::core::{Term, SymbolTable};

pub fn parse_rust_signature(sig: &str, syms: &mut SymbolTable) -> Option<Term> {
    let sig = sig.trim();
    if !sig.starts_with("fn ") { return None; }
    let rest = &sig[3..];
    let paren = rest.find('(')?;
    let name = rest[..paren].trim();
    let name_sym = syms.intern(name);

    let args_end = rest.find(')')?;
    let args_str = &rest[paren + 1..args_end];
    let mut args = Vec::new();

    for arg in args_str.split(',') {
        let arg = arg.trim();
        if arg.is_empty() { continue; }
        if let Some(colon) = arg.find(':') {
            let param_name = arg[..colon].trim();
            let param_type = arg[colon + 1..].trim();
            let pn = syms.intern(param_name);
            let pt = syms.intern(param_type);
            args.push(Term::compound(syms.intern("param"), vec![Term::atom(pn), Term::atom(pt)]));
        }
    }

    let ret_type = if let Some(arrow) = rest.find("->") {
        let rt = rest[arrow + 2..].trim().trim_end_matches('{').trim();
        Some(Term::atom(syms.intern(rt)))
    } else {
        None
    };

    let mut fn_args = vec![Term::atom(name_sym), Term::list(args)];
    if let Some(rt) = ret_type {
        fn_args.push(rt);
    }

    Some(Term::compound(syms.intern("fn_sig"), fn_args))
}

pub fn parse_python_signature(sig: &str, syms: &mut SymbolTable) -> Option<Term> {
    let sig = sig.trim();
    if !sig.starts_with("def ") { return None; }
    let rest = &sig[4..];
    let paren = rest.find('(')?;
    let name = rest[..paren].trim();
    let name_sym = syms.intern(name);

    let args_end = rest.find(')')?;
    let args_str = &rest[paren + 1..args_end];
    let mut args = Vec::new();

    for arg in args_str.split(',') {
        let arg = arg.trim();
        if arg.is_empty() || arg == "self" { continue; }
        let param_name = arg.split(':').next().unwrap_or(arg).trim();
        let pn = syms.intern(param_name);
        args.push(Term::atom(pn));
    }

    let ret_type = if let Some(arrow) = rest.find("->") {
        let rt = rest[arrow + 2..].trim().trim_end_matches(':').trim();
        Some(Term::atom(syms.intern(rt)))
    } else {
        None
    };

    let mut fn_args = vec![Term::atom(name_sym), Term::list(args)];
    if let Some(rt) = ret_type {
        fn_args.push(rt);
    }

    Some(Term::compound(syms.intern("fn_sig"), fn_args))
}
