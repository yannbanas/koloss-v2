# KOLOSS v2 — Roadmap

## Etat Actuel
- ~3500 lignes Rust
- 0 erreurs, 0 warnings, compile et execute
- Unifier, SAT solver, Rule Engine (NAF + builtins + cut + tabling), Knowledge Graph, ARC DSL, Search, Evolution
- Aucune dependance LLM pour le raisonnement

## Chantier 1 — Moteur de Raisonnement (COMPLET)
- [x] Term types + SymbolTable
- [x] Unification (Robinson algorithm)
- [x] Substitution (walk, walk_deep, compose)
- [x] Occurs check
- [x] SAT solver (DPLL)
- [x] Constraint solver (CSP backtracking)
- [x] Rule Engine (backward chaining)
- [x] Forward chaining
- [x] Search: DFS, BFS, Beam, Iterative Deepening, MCTS
- [x] Negation as failure (CWA, not/\+ predicates)
- [x] Built-in predicates (is, >, <, >=, <=, =:=, =\=, +, -, *, /, mod, abs, between, member, append, length, functor, arg, var, nonvar, atom, integer, ground, write, nl)
- [x] Cut (!) pour optimiser la recherche + propagation correcte
- [x] Tabling/memoization (per-functor, hash-based cache)

## Chantier 2 — ARC-AGI Program Synthesis (EN COURS)
- [x] DSL de 177 primitives (rotate, flip, filter, gravity, flood_fill, mirror, repeat, invert, etc.)
- [x] Enumeration bottom-up (taille 1, 2, 3)
- [x] Evolution genetique (crossover, mutation, selection)
- [x] ARC task loader (JSON)
- [x] Connected components (4-conn + 8-conn), flood fill
- [x] Object detection (extract, count, bounding box, center, area)
- [x] Raisonnement spatial (above, below, left_of, right_of, adjacent, inside, overlap, distance)
- [x] Symmetry detection (horizontal, vertical, diagonal) + period detection (H/V)
- [x] Grid overlay, keep largest/smallest, outline, fill inside holes
- [x] Rule Engine ↔ DSL bridge (GridReasoner: grid → logical facts)
- [ ] Abstraction (DreamCoder-style): comprimer les programmes en bibliotheques

## Chantier 3 — Graphe de Connaissances
- [x] KnowledgeGraph (nodes, edges, index)
- [x] BFS pathfinding
- [x] Triple query (source, relation, target)
- [x] Anti-unification (generalization)
- [x] Structure Mapping (analogie)
- [ ] Serialization/persistence (bincode ou SQLite)
- [ ] Decay temporel (oubli actif)
- [ ] Inference : graph → rules (apprentissage de regles depuis le graphe)
- [ ] Embedding symbolique (graph → vector pour search rapide)

## Chantier 4 — Auto-Amelioration
- [x] Fitness score composite
- [x] Mutation framework (add/remove rules, facts)
- [x] Mutation log (track improvements/regressions)
- [ ] Hill climbing sur le RuleEngine
- [ ] Genetic programming du RuleEngine entier
- [ ] Auto-compilation + test + rollback
- [ ] Self-replication (generate son propre Cargo.toml + src/)

## Chantier 5 — Minimisation
- [x] Supprimer dead code (warnings → 0)
- [ ] Fusionner modules redondants
- [ ] Inline les abstractions inutiles
- [ ] Benchmark perf (latence, memoire)
- [ ] Target < 10K lignes pour le core reasoning

## Chantier 6 — NLU/NLG Bridge (Futur)
- [ ] Petit LLM (3B) pour texte → Term
- [ ] Term → texte naturel
- [ ] Zero LLM dans la boucle de raisonnement (le LLM parse, le Rust raisonne)

## Chantier 7 — Benchmark Reel
- [ ] ARC-AGI evaluation dataset (400 tasks)
- [ ] HumanEval via program synthesis (pas LLM)
- [ ] Mesure score vs baseline
- [ ] Dashboard minimal (CLI)

## Metriques Cibles

| Metrique | Actuel | Cible v2.1 | Cible v2.5 | Cible v3.0 |
|----------|--------|------------|------------|------------|
| Lignes code | 4500 | 8000 | 15000 | <100K |
| ARC-AGI score | 0% | 15% | 40% | 60% |
| Warnings | 0 | 0 | 0 | 0 |
| LLM dependency | 0% | 5% (NLU) | 5% | 5% |
| Self-improvement | non | basique | mesurable | autonome |
