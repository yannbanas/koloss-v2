# KOLOSS v2 — Roadmap

## Etat Actuel
- ~7200 lignes Rust, 36 fichiers, 51 tests
- 0 erreurs, 0 warnings, compile et execute
- Unifier, SAT solver, Rule Engine (NAF + builtins + cut + tabling), Knowledge Graph
- ARC DSL (177 primitives), multi-strategy solver, real ARC-AGI benchmark
- **ARC-AGI score: 10% sur 50 taches (5 resolues)**
- Approches innovantes: bidirectional search, heuristic selection, grid fingerprinting, MDL compression
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

## Chantier 2 — ARC-AGI Program Synthesis (COMPLET)
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
- [x] Abstraction (DreamCoder-style): Library learning, wake/sleep cycle, sub-program extraction
- [x] DAG search (Icecuber-style): greedy composition with deduplication
- [x] **Bidirectional search**: forward + backward with inverse primitives (exponential speedup)
- [x] **Heuristic primitive selection**: feature analysis → filtered search space (177→20-60 prims)
- [x] **Grid fingerprinting**: O(1) dedup via polynomial rolling hash + multi-resolution
- [x] **MDL compression**: description length scoring, program simplicity preference
- [x] **Multi-strategy pipeline**: heuristic → bidir → DAG → enumerate → evolve (with 10s timeout)
- [x] RLE encoding, delta encoding, Shannon entropy, compression ratio

## Chantier 3 — Graphe de Connaissances (COMPLET)
- [x] KnowledgeGraph (nodes, edges, index)
- [x] BFS pathfinding
- [x] Triple query (source, relation, target)
- [x] Anti-unification (generalization)
- [x] Structure Mapping (analogie)
- [x] JSON persistence (save/load GraphSnapshot, TermSer for serializable terms)
- [x] Temporal decay (weight decay by age, access boost, configurable DecayConfig)
- [x] Auto-pruning (remove nodes/edges below weight threshold)
- [x] Graph inference (extract chain + shared-target patterns, infer rules with confidence)
- [x] Symbolic embedding (node → vector: label, degree, weight, access, relation distribution)
- [x] Subgraph embedding (BFS radius aggregation)
- [x] Cosine similarity + find_similar_nodes(top_k)

## Chantier 4 — Auto-Amelioration (COMPLET)
- [x] Fitness score composite (accuracy/size/speed/memory weighted)
- [x] Mutation framework (add/remove/swap/duplicate rules, add/retract facts)
- [x] Mutation log (track improvements/regressions, best_improvement)
- [x] Hill climbing (mutate→eval loop, greedy ascent with plateau detection)
- [x] Genetic programming (evolve population of RuleEngines, tournament selection)
- [x] Auto-compilation (generate_rust_source, try_compile_check via rustc)
- [x] Self-replication (generate_project: Cargo.toml + src/main.rs, write_project)
- [x] Binary serialization (BinaryWriter/Reader: terms, symbol tables, grids)
- [x] Grid packing (4-bit nibble packing for ARC grids, 50% compression)

## Chantier 5 — Minimisation
- [x] Supprimer dead code (warnings → 0)
- [ ] Fusionner modules redondants
- [ ] Inline les abstractions inutiles
- [x] Benchmark perf (ARC-AGI: 50 taches en 33s release)
- [x] Target < 10K lignes pour le core reasoning (7200 lignes actuellement)

## Chantier 6 — NLU/NLG Bridge (Futur)
- [ ] Petit LLM (3B) pour texte → Term
- [ ] Term → texte naturel
- [ ] Zero LLM dans la boucle de raisonnement (le LLM parse, le Rust raisonne)

## Chantier 7 — Benchmark Reel (EN COURS)
- [x] ARC-AGI training dataset (400 tasks) telecharge et integre
- [x] ARC-AGI evaluation dataset (400 tasks)
- [x] Benchmark runner avec scoring detaille + rapport
- [x] Score mesure: **10% sur 50 taches** (5 resolues)
- [x] Mesure par methode: heuristic_single (60%), heuristic_compose2 (40%)
- [ ] HumanEval via program synthesis (pas LLM)
- [ ] Dashboard minimal (CLI) — en cours

## Metriques

| Metrique | Actuel | Cible v2.1 | Cible v2.5 | Cible v3.0 |
|----------|--------|------------|------------|------------|
| Lignes code | 7200 | 10000 | 15000 | <100K |
| Fichiers | 36 | 50 | 80 | 150 |
| Tests | 51 | 100 | 300 | 1000 |
| ARC-AGI score | 10% | 20% | 40% | 60% |
| Warnings | 0 | 0 | 0 | 0 |
| LLM dependency | 0% | 5% (NLU) | 5% | 5% |
| Self-improvement | basique | mesurable | autonome | recursif |

## Approches Innovantes Implementees

### Bidirectional Search (bidir.rs)
- Recherche simultanée input→ et ←output via primitives inverses
- Complexite O(2*b^(d/2)) vs O(b^d) = speedup exponentiel
- Primitives invertibles: rotations, flips, transpose, color swaps

### Heuristic Primitive Selection (heuristics.rs)
- Analyse de features: dimension change, color mapping, object count, symmetry
- Filtre les 177 primitives en 20-60 pertinentes par tache
- Branching factor reduit de 5-10x

### Grid Fingerprinting (fingerprint.rs)
- Hash polynomial FNV-1a avec position mixing
- Multi-resolution: full + quadrants + color signature
- FingerprintSet pour dedup O(1) au lieu de O(rows*cols)

### MDL Compression (compression.rs)
- Description Length = cout du programme + cout des erreurs
- Prefere programmes simples (rasoir d'Occam automatique)
- RLE, delta encoding, Shannon entropy pour analyse de grilles
