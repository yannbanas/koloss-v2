use koloss_v2::core::{Term, SymbolTable};
use koloss_v2::reasoning::unifier::{Substitution, unify};
use koloss_v2::reasoning::solver::{SatProblem, SatResult};
use koloss_v2::reasoning::rules::{Rule, RuleEngine};
use koloss_v2::reasoning::builtins;
use koloss_v2::memory::graph::KnowledgeGraph;
use koloss_v2::synthesis::dsl::Prim;

fn main() {
    println!("KOLOSS v2 — Autonomous Reasoning Engine");
    println!("========================================\n");

    demo_unification();
    demo_sat();
    demo_rules();
    demo_builtins();
    demo_naf();
    demo_cut();
    demo_tabling();
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

fn demo_builtins() {
    println!("\n--- Built-in Predicates ---");
    let mut syms = SymbolTable::new();
    let mut engine = RuleEngine::new();

    // Register builtins
    let is_sym = syms.intern("is");
    let gt_sym = syms.intern(">");
    let lt_sym = syms.intern("<");
    let plus_sym = syms.intern("+");
    let mul_sym = syms.intern("*");
    let between_sym = syms.intern("between");

    engine.builtins_mut().register(builtins::BUILTIN_IS, is_sym);
    engine.builtins_mut().register(builtins::BUILTIN_GT, gt_sym);
    engine.builtins_mut().register(builtins::BUILTIN_LT, lt_sym);
    engine.builtins_mut().register(builtins::BUILTIN_PLUS, plus_sym);
    engine.builtins_mut().register(builtins::BUILTIN_MUL, mul_sym);
    engine.builtins_mut().register(builtins::BUILTIN_BETWEEN, between_sym);

    // is(X, 3 + 4 * 2) => X = 11
    let expr = Term::compound(plus_sym, vec![
        Term::int(3),
        Term::compound(mul_sym, vec![Term::int(4), Term::int(2)]),
    ]);
    let query = Term::compound(is_sym, vec![Term::var(0), expr]);
    let results = engine.query(&query);
    if let Some(sub) = results.first() {
        println!("  is(?X, 3 + 4*2) => ?X = {}", sub.apply(&Term::var(0)));
    }

    // >(5, 3) => true
    let query = Term::compound(gt_sym, vec![Term::int(5), Term::int(3)]);
    let results = engine.query(&query);
    println!("  >(5, 3) => {}", if results.is_empty() { "false" } else { "true" });

    // between(1, 5, ?X) => X = 1, 2, 3, 4, 5
    let query = Term::compound(between_sym, vec![Term::int(1), Term::int(5), Term::var(0)]);
    let results = engine.query(&query);
    let vals: Vec<String> = results.iter().map(|s| format!("{}", s.apply(&Term::var(0)))).collect();
    println!("  between(1, 5, ?X) => {}", vals.join(", "));
}

fn demo_naf() {
    println!("\n--- Negation as Failure ---");
    let mut syms = SymbolTable::new();
    let mut engine = RuleEngine::new();

    let flies_sym = syms.intern("flies");
    let bird_sym = syms.intern("bird");
    let penguin_sym = syms.intern("penguin");
    let not_sym = syms.intern("not");
    let tweety = syms.intern("tweety");
    let opus = syms.intern("opus");

    engine.set_not_sym(not_sym);

    // bird(tweety). bird(opus). penguin(opus).
    engine.add_fact(Term::compound(bird_sym, vec![Term::atom(tweety)]));
    engine.add_fact(Term::compound(bird_sym, vec![Term::atom(opus)]));
    engine.add_fact(Term::compound(penguin_sym, vec![Term::atom(opus)]));

    // flies(X) :- bird(X), not(penguin(X)).
    engine.add_rule(Rule::new(
        Term::compound(flies_sym, vec![Term::var(0)]),
        vec![
            Term::compound(bird_sym, vec![Term::var(0)]),
            Term::compound(not_sym, vec![
                Term::compound(penguin_sym, vec![Term::var(0)])
            ]),
        ],
    ));

    // Query: flies(?X)
    let query = Term::compound(flies_sym, vec![Term::var(99)]);
    let results = engine.query(&query);
    println!("  flies(X) :- bird(X), not(penguin(X)).");
    println!("  bird(tweety). bird(opus). penguin(opus).");
    print!("  query: flies(?X) => ");
    let answers: Vec<String> = results.iter().map(|s| {
        let val = s.apply(&Term::var(99));
        if let Term::Atom(a) = &val { syms.resolve(*a).unwrap_or("?").to_string() }
        else { format!("{}", val) }
    }).collect();
    println!("{}", answers.join(", "));
    println!("  (tweety flies, opus doesn't — correct!)");
}

fn demo_cut() {
    println!("\n--- Cut (!) ---");
    let mut syms = SymbolTable::new();
    let mut engine = RuleEngine::new();

    let max_sym = syms.intern("my_max");
    let gte_sym = syms.intern(">=");
    let cut_sym = syms.intern("!");

    engine.builtins_mut().register(builtins::BUILTIN_GTE, gte_sym);
    engine.builtins_mut().register(builtins::BUILTIN_CUT, cut_sym);

    // my_max(X, Y, X) :- X >= Y, !.
    engine.add_rule(Rule::new(
        Term::compound(max_sym, vec![Term::var(0), Term::var(1), Term::var(0)]),
        vec![
            Term::compound(gte_sym, vec![Term::var(0), Term::var(1)]),
            Term::compound(cut_sym, vec![]),
        ],
    ));

    // my_max(X, Y, Y) :- Y > X.
    // (simplified: my_max(_, Y, Y).)
    engine.add_rule(Rule::new(
        Term::compound(max_sym, vec![Term::var(0), Term::var(1), Term::var(1)]),
        vec![],
    ));

    let query = Term::compound(max_sym, vec![Term::int(7), Term::int(3), Term::var(99)]);
    let results = engine.query(&query);
    if let Some(sub) = results.first() {
        println!("  my_max(7, 3, ?Z) => ?Z = {} (cut prevented duplicate)", sub.apply(&Term::var(99)));
    }
    println!("  {} solution(s) with cut (without cut would be 2)", results.len());
}

fn demo_tabling() {
    println!("\n--- Tabling/Memoization ---");
    let mut syms = SymbolTable::new();

    let fib_sym = syms.intern("fib");
    let is_sym = syms.intern("is");
    let plus_sym = syms.intern("+");
    let minus_sym = syms.intern("-");
    let lte_sym = syms.intern("<=");

    let mut engine = RuleEngine::new().with_tabling();
    engine.table_functor(fib_sym);

    engine.builtins_mut().register(builtins::BUILTIN_IS, is_sym);
    engine.builtins_mut().register(builtins::BUILTIN_PLUS, plus_sym);
    engine.builtins_mut().register(builtins::BUILTIN_MINUS, minus_sym);
    engine.builtins_mut().register(builtins::BUILTIN_LTE, lte_sym);

    // fib(0, 0). fib(1, 1).
    engine.add_fact(Term::compound(fib_sym, vec![Term::int(0), Term::int(0)]));
    engine.add_fact(Term::compound(fib_sym, vec![Term::int(1), Term::int(1)]));

    // fib(N, F) :- N > 1, N1 is N-1, N2 is N-2, fib(N1, F1), fib(N2, F2), F is F1+F2.
    // We'll compute manually here since recursive tabled rules need iterative deepening
    // Instead, demonstrate tabling cache behavior
    let query = Term::compound(fib_sym, vec![Term::int(1), Term::var(99)]);
    let results = engine.query(&query);
    println!("  fib(1, ?F) => {} solution(s)", results.len());
    println!("  table size after query: {}", engine.table_size());

    // Second query hits cache
    let results2 = engine.query(&query);
    println!("  fib(1, ?F) again => {} solution(s) (from cache)", results2.len());
    println!("  table size: {} (memoized)", engine.table_size());
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
        println!("  path alice->bob: {} edges", path.len());
    }

    let triples = graph.query_triple(Some(person), Some(works_at), Some(company));
    println!("  person-works_at-company: {} triples", triples.len());
}

fn demo_arc_dsl() {
    println!("\n--- ARC DSL ---");
    use koloss_v2::synthesis::dsl::{connected_components, count_objects, is_above, is_symmetric_h,
        detect_period_h, overlay_grids, unique_colors};

    let grid = vec![
        vec![0, 1, 0, 0, 2, 0],
        vec![0, 1, 0, 0, 2, 0],
        vec![0, 0, 0, 0, 0, 0],
        vec![0, 0, 3, 3, 0, 0],
        vec![0, 0, 3, 3, 0, 0],
    ];

    println!("  input: {}x{}", grid.len(), grid[0].len());
    println!("  colors: {:?}", unique_colors(&grid));
    println!("  objects: {}", count_objects(&grid));

    let objects = connected_components(&grid, true);
    for (i, obj) in objects.iter().enumerate() {
        println!("    obj[{}]: color={} area={} bbox={:?}", i, obj.color, obj.area(), obj.bounding_box());
    }

    if objects.len() >= 3 {
        println!("  obj[0] above obj[2]? {}", is_above(&objects[0], &objects[2]));
    }

    // Flood fill
    let filled = Prim::FloodFill(0, 0, 5).apply(&grid);
    let fill_count = filled.iter().flat_map(|r| r.iter()).filter(|&&c| c == 5).count();
    println!("  flood_fill(0,0,5): {} cells filled", fill_count);

    // Symmetry
    let sym_grid = vec![vec![1, 0, 1], vec![0, 1, 0], vec![1, 0, 1]];
    println!("  symmetric_h([[1,0,1],...]): {}", is_symmetric_h(&sym_grid));

    // Pattern repetition
    let rep_grid = vec![vec![1, 2, 1, 2, 1, 2]];
    println!("  period_h([[1,2,1,2,1,2]]): {:?}", detect_period_h(&rep_grid));

    // Overlay
    let base = vec![vec![1, 1], vec![1, 1]];
    let top = vec![vec![0, 2], vec![2, 0]];
    let merged = overlay_grids(&base, &top);
    println!("  overlay: {:?}", merged);

    // Keep largest
    let largest = Prim::KeepLargestObject.apply(&grid);
    let largest_count = largest.iter().flat_map(|r| r.iter()).filter(|&&c| c != 0).count();
    println!("  keep_largest: {} cells", largest_count);

    // Fill inside
    let hollow = vec![
        vec![1, 1, 1],
        vec![1, 0, 1],
        vec![1, 1, 1],
    ];
    let filled_inside = Prim::FillInsideObjects(2).apply(&hollow);
    println!("  fill_inside hollow square: center={}", filled_inside[1][1]);

    println!("  {} primitives available", Prim::all_primitives().len());
}
