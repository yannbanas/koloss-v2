# KOLOSS v2 — Autonomous Reasoning Engine

A pure Rust symbolic reasoning engine designed to replace LLM-dependent architectures with deterministic, self-improving intelligence.

## Core Philosophy

> **Zero LLM for reasoning.** Every logical deduction, search, synthesis, and self-improvement step runs in pure Rust. LLMs are relegated to NLU/NLG interface only.

## Architecture

```
                    ┌──────────────────────────┐
                    │        main.rs            │
                    │   (CLI / orchestrator)    │
                    └────────────┬─────────────┘
                                 │
        ┌────────────────────────┼────────────────────────┐
        │                        │                        │
┌───────┴────────┐   ┌──────────┴──────────┐   ┌─────────┴────────┐
│   reasoning/   │   │    synthesis/        │   │     memory/      │
│  unifier.rs    │   │  dsl.rs (134 prims)  │   │  graph.rs        │
│  solver.rs     │   │  enumerate.rs        │   │  compress.rs     │
│  rules.rs      │   │  evolve.rs           │   │  analogy.rs      │
│  search.rs     │   └─────────────────────-┘   └──────────────────┘
└────────────────┘
        │                        │                        │
┌───────┴────────┐   ┌──────────┴──────────┐   ┌─────────┴────────┐
│  perception/   │   │  self_improve/       │   │     bench/       │
│  grid.rs       │   │  fitness.rs          │   │  arc.rs          │
│  code.rs       │   │  mutator.rs          │   └──────────────────┘
└────────────────┘   └─────────────────────-┘
```

## Modules

| Module | Purpose |
|--------|---------|
| `core` | Term algebra, symbol table, ordered floats, error types |
| `reasoning/unifier` | Robinson unification with occurs check |
| `reasoning/solver` | DPLL SAT solver + CSP constraint solver |
| `reasoning/rules` | Prolog-like rule engine (backward + forward chaining) |
| `reasoning/search` | DFS, BFS, beam search, iterative deepening, MCTS |
| `synthesis/dsl` | 134 ARC-AGI grid transformation primitives |
| `synthesis/enumerate` | Bottom-up program synthesis |
| `synthesis/evolve` | Genetic evolution of programs |
| `memory/graph` | Knowledge graph with pathfinding and triple queries |
| `memory/compress` | Anti-unification (LGG) for fact compression |
| `memory/analogy` | Structure Mapping Engine for analogical reasoning |
| `perception/grid` | ARC task JSON loader |
| `perception/code` | Rust/Python signature parser |
| `self_improve/fitness` | Composite fitness scoring |
| `self_improve/mutator` | Rule mutation framework |
| `bench/arc` | ARC-AGI evaluator (synthesis + evolution) |

## Quick Start

```bash
cargo run
```

Output:
```
KOLOSS v2 — Autonomous Reasoning Engine
========================================

--- Unifier ---
  unify(parent(alice, ?0), parent(alice, bob)) => ?0 = bob
--- SAT Solver ---
  SAT! x1=true, x2=false, x3=true
--- Rule Engine ---
  query: ancestor(alice, ?X)
    ?X = bob
    ?X = charlie
  2 solutions found
  forward chaining derived 3 new facts
--- Knowledge Graph ---
  3 nodes, 3 edges
  path alice->bob: 1 edges
  person-works_at-company: 2 triples
--- ARC DSL ---
  134 primitives available

[v2] All systems operational. No LLM required.
```

## Roadmap

| Chantier | Target | Status |
|----------|--------|--------|
| 1. Reasoning Engine | Negation, builtins, cut, tabling | In progress |
| 2. ARC-AGI Primitives | Connected components, spatial reasoning | Planned |
| 3. Knowledge Graph | Persistence, decay, inference | Planned |
| 4. Self-Improvement | Hill climbing, GP, auto-compilation | Planned |
| 5. Code Minimization | <100K lines of ultra-optimized Rust | Planned |
| 6. NLU/NLG Bridge | Small LLM (3B) for interface only | Planned |
| 7. ARC-AGI Benchmark | Real evaluation on 400 public tasks | Planned |

## License

MIT
