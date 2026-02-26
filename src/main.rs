use koloss_v2::core::{Term, SymbolTable};
use koloss_v2::reasoning::unifier::{Substitution, unify};
use koloss_v2::reasoning::solver::{SatProblem, SatResult};
use koloss_v2::reasoning::rules::{Rule, RuleEngine};
use koloss_v2::memory::graph::KnowledgeGraph;
use koloss_v2::synthesis::dsl::Prim;

fn main() {
    println!("KOLOSS v2 — Autonomous Reasoning Engine");
    println!("========================================\n");

    demo_unification();
    demo_sat();
    demo_rules();
    demo_knowledge_graph();
    demo_arc_dsl();

    println!("\n[v2] All systems operational. No LLM required.");
}

fn demo_unification() {
    println!("--- Unifier ---");
    let mut syms = SymbolTable::new();
    let parent = syms.intern("parent");
    let alice = syms.intern("alice");
    let bob = syms.intern("bob");

    let t1 = Term::compound(parent, vec![Term::atom(alice), Term::var(0)]);
    let t2 = Term::compound(parent, vec![Term::atom(alice), Term::atom(bob)]);

    let sub = Substitution::new();
    match unify(&t1, &t2, &sub) {
        Ok(result) => {
            let resolved = result.apply(&Term::var(0));
            println!("  unify({}, {}) => ?0 = {}", t1, t2, resolved);
        }
        Err(e) => println!("  unification failed: {}", e),
    }
}

fn demo_sat() {
    println!("\n--- SAT Solver ---");
    let mut problem = SatProblem::new(3);
    problem.add_clause(vec![1, 2]);
    problem.add_clause(vec![-1, 3]);
    problem.add_clause(vec![-2, -3]);
    problem.add_clause(vec![1]);

    match problem.solve() {
        SatResult::Sat(assignment) => {
            let mut sorted: Vec<_> = assignment.iter().collect();
            sorted.sort_by_key(|(&k, _)| k);
            let assigns: Vec<String> = sorted.iter().map(|(&v, &b)| format!("x{}={}", v, b)).collect();
            println!("  SAT! {}", assigns.join(", "));
        }
        SatResult::Unsat => println!("  UNSAT"),
    }
}

fn demo_rules() {
    println!("\n--- Rule Engine ---");
    let mut syms = SymbolTable::new();
    let parent_sym = syms.intern("parent");
    let ancestor_sym = syms.intern("ancestor");
    let alice = syms.intern("alice");
    let bob = syms.intern("bob");
    let charlie = syms.intern("charlie");

    let mut engine = RuleEngine::new();

    engine.add_fact(Term::compound(parent_sym, vec![Term::atom(alice), Term::atom(bob)]));
    engine.add_fact(Term::compound(parent_sym, vec![Term::atom(bob), Term::atom(charlie)]));

    engine.add_rule(Rule::new(
        Term::compound(ancestor_sym, vec![Term::var(0), Term::var(1)]),
        vec![Term::compound(parent_sym, vec![Term::var(0), Term::var(1)])],
    ));

    engine.add_rule(Rule::new(
        Term::compound(ancestor_sym, vec![Term::var(0), Term::var(2)]),
        vec![
            Term::compound(parent_sym, vec![Term::var(0), Term::var(1)]),
            Term::compound(ancestor_sym, vec![Term::var(1), Term::var(2)]),
        ],
    ));

    let query = Term::compound(ancestor_sym, vec![Term::atom(alice), Term::var(99)]);
    let results = engine.query(&query);
    println!("  query: ancestor(alice, ?X)");
    for sub in &results {
        let answer = sub.apply(&Term::var(99));
        if let Term::Atom(a) = &answer {
            println!("    ?X = {} ({})", a, syms.resolve(*a).unwrap_or("?"));
        } else {
            println!("    ?X = {}", answer);
        }
    }
    println!("  {} solutions found", results.len());

    let new_facts = engine.forward_chain(10);
    println!("  forward chaining derived {} new facts", new_facts);
}

fn demo_knowledge_graph() {
    println!("\n--- Knowledge Graph ---");
    let mut syms = SymbolTable::new();
    let mut graph = KnowledgeGraph::new();

    let person = syms.intern("person");
    let knows = syms.intern("knows");
    let works_at = syms.intern("works_at");
    let company = syms.intern("company");

    let alice = graph.add_node(person);
    let bob = graph.add_node(person);
    let acme = graph.add_node(company);

    graph.add_edge(alice, knows, bob);
    graph.add_edge(alice, works_at, acme);
    graph.add_edge(bob, works_at, acme);

    println!("  {} nodes, {} edges", graph.node_count(), graph.edge_count());
    println!("  alice neighbors: {:?}", graph.neighbors(alice));

    if let Some(path) = graph.find_path(alice, bob, 5) {
        println!("  path alice→bob: {} edges", path.len());
    }

    let triples = graph.query_triple(Some(person), Some(works_at), Some(company));
    println!("  person-works_at-company: {} triples", triples.len());
}

fn demo_arc_dsl() {
    println!("\n--- ARC DSL ---");
    let grid = vec![
        vec![0, 1, 0],
        vec![1, 0, 1],
        vec![0, 1, 0],
    ];

    let rotated = Prim::RotateCW.apply(&grid);
    let flipped = Prim::FlipH.apply(&grid);

    println!("  input: {:?}", grid);
    println!("  rotate_cw: {:?}", rotated);
    println!("  flip_h: {:?}", flipped);
    println!("  {} primitives available", Prim::all_primitives().len());
}
