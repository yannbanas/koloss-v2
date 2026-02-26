# KOLOSS v2 — Architecture

## Principe Fondamental
Le raisonnement se fait en Rust pur. Zero LLM dans la boucle de raisonnement.
Un petit LLM (3B) sert uniquement de NLU/NLG : texte → structure, structure → texte.

## Diagramme Global
```
Input (texte/grille/code)
    │
    ▼
┌──────────────┐
│  Perception  │  NLU parse → Term structures
└──────┬───────┘
       │
       ▼
┌──────────────────────────────────────────┐
│           Working Memory (Graph)          │
│  Nodes ←→ Edges ←→ Rules ←→ Facts        │
│  Compression (anti-unification)           │
│  Analogie structurelle (structure mapping)│
└──────────────────┬───────────────────────┘
                   │
       ┌───────────┼───────────┐
       ▼           ▼           ▼
┌──────────┐ ┌──────────┐ ┌──────────┐
│  Unifier │ │ SAT/CSP  │ │ Program  │
│ (Prolog) │ │  Solver  │ │ Synthesis│
└──────────┘ └──────────┘ └──────────┘
       │           │           │
       └───────────┼───────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│         Search (DFS/BFS/Beam/MCTS)        │
│  Explore l'espace des solutions           │
└──────────────────┬───────────────────────┘
                   │
                   ▼
┌──────────────────────────────────────────┐
│           Self-Improvement                │
│  Fitness → Mutation → Selection → Evolve  │
└──────────────────┬───────────────────────┘
                   │
                   ▼
              Output (solution)
```

## Modules

### core/ (~200 lignes)
Types fondamentaux partages par tous les modules.
- **Term** : representation universelle (Var, Atom, Int, Float, Str, Bool, Compound, List, Nil)
- **SymbolTable** : intern pool pour les symboles (zero allocation repetee)
- **KolossError** : erreurs typees sans panic
- **OrderedFloat** : f64 hashable pour les Term

### reasoning/ (~900 lignes)
Moteur de raisonnement symbolique. Le coeur de l'intelligence.
- **unifier.rs** : Algorithme d'unification (Robinson), substitutions, occurs check, walk/walk_deep
- **solver.rs** : SAT solver DPLL + constraint solver (CSP) avec backtracking
- **rules.rs** : Moteur de regles Prolog-like. Backward chaining (query), forward chaining (derive)
- **search.rs** : DFS, BFS, Beam search, Iterative deepening, MCTS (Monte Carlo Tree Search)

### synthesis/ (~500 lignes)
Generation de programmes a partir d'exemples (pour ARC-AGI).
- **dsl.rs** : 20+ primitives de transformation de grilles (rotate, flip, filter, gravity, etc.)
- **enumerate.rs** : Enumeration bottom-up de programmes, composition, scoring partiel
- **evolve.rs** : Evolution genetique de programmes (crossover, mutation, selection)

### memory/ (~400 lignes)
Graphe de connaissances structure.
- **graph.rs** : KnowledgeGraph (nodes, edges, index par label/relation, BFS pathfinding, triple query)
- **compress.rs** : Anti-unification (LGG), generalisation de faits, compression memoire
- **analogy.rs** : Structure Mapping Engine, similarite structurelle entre sous-graphes

### perception/ (~200 lignes)
Entree/sortie, parsing.
- **grid.rs** : Chargement/affichage de grilles ARC-AGI (JSON format)
- **code.rs** : Parsing de signatures Rust/Python → Term structures

### self_improve/ (~200 lignes)
Auto-amelioration mesurable.
- **fitness.rs** : Score composite (accuracy 60% + taille 20% + vitesse 10% + memoire 10%)
- **mutator.rs** : Mutations ciblees sur le RuleEngine, log des ameliorations/regressions

### bench/ (~100 lignes)
Benchmarks.
- **arc.rs** : Evaluateur ARC-AGI (synthesis + evolution, scoring par tache)

### net/ (stub)
API minimale et git ops (a implementer).

## Flux de Donnees

1. **Input** → `perception::parse_*()` → `Term` structures
2. **Term** → `memory::graph` (stockage) + `reasoning::rules` (facts)
3. **Query** → `reasoning::rules::query()` → backward chaining → `Substitution`
4. **Si echec** → `reasoning::solver` (SAT/CSP) ou `synthesis::enumerate` (program search)
5. **Recherche** → `reasoning::search::mcts()` guide l'exploration
6. **Resultat** → `memory::compress::generalize_terms()` (apprentissage)
7. **Fitness** → `self_improve::fitness::compute()` → decide si garder la mutation
