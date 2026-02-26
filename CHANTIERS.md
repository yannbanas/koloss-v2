# KOLOSS v2 — Chantiers Detailles

## Chantier 1 : Moteur de Raisonnement Symbolique

### Sous-composants
1. **Unificateur** (FAIT) : Robinson algorithm, occurs check, substitutions
2. **SAT solver** (FAIT) : DPLL avec unit propagation + pure literal
3. **Constraint solver** (FAIT) : CSP backtracking avec domaines finis
4. **Rule Engine** (FAIT) : Prolog-like, backward + forward chaining
5. **Search** (FAIT) : DFS, BFS, Beam, MCTS, Iterative Deepening

### A faire
- Negation as failure (CWA)
- Built-in predicates : `is(X, Y+Z)`, `>(X,Y)`, `<(X,Y)`, arithmetique
- Cut (!) pour pruning
- Tabling (memoization des sous-requetes)
- Mode declaration (input/output typing)

### Fichiers concernes
- `src/reasoning/unifier.rs`
- `src/reasoning/solver.rs`
- `src/reasoning/rules.rs`
- `src/reasoning/search.rs`

---

## Chantier 2 : Program Synthesis pour ARC-AGI

### Approche
ARC-AGI = decouvrir une transformation a partir de 3 exemples input→output.
LLMs plafonnent a ~30%. Le SOTA utilise DSL + search.

### Strategie
1. **Enumeration bottom-up** : tester toutes les primitives simples, puis compositions
2. **Evolution** : GA sur l'espace des programmes
3. **Abstraction** : DreamCoder-style, comprimer les programmes recurrants en bibliotheques
4. **Object detection** : segmenter les grilles en objets (composantes connexes, couleurs)

### DSL Actuel (134 primitives)
- Geometrique : rotate_cw, rotate_ccw, rotate_180, flip_h, flip_v, transpose
- Couleur : fill_color(c), replace_color(from, to), filter_color(c), most_frequent_color
- Spatial : crop(r,c,h,w), pad(n,c), scale(s), border_fill(c)
- Physique : gravity_down, gravity_up, gravity_left, gravity_right
- Composition : compose(a, b), conditional(cond, then, else)

### Prochaines primitives
- `connected_components(color)` : segmenter par composantes connexes
- `flood_fill(r, c, color)` : remplissage a partir d'un point
- `extract_object(id)` : isoler un objet
- `count_objects()` : compter les composantes
- `mirror_along(axis)` : miroir le long d'un axe detecte
- `repeat_pattern(dir)` : repeter un motif
- `overlay(grid_a, grid_b)` : superposer deux grilles

### Fichiers concernes
- `src/synthesis/dsl.rs`
- `src/synthesis/enumerate.rs`
- `src/synthesis/evolve.rs`
- `src/bench/arc.rs`
- `src/perception/grid.rs`

---

## Chantier 3 : Graphe de Connaissances

### Actuel
- KnowledgeGraph : nodes + edges + index
- BFS pathfinding
- Triple query
- Anti-unification (LGG)
- Structure Mapping (analogie)

### A faire
- **Persistence** : sauvegarder/charger le graphe (bincode ou SQLite)
- **Decay** : les noeuds non accedes perdent du poids, finissent supprimes
- **Inference** : extraire des regles depuis le graphe (graph mining)
- **Embedding symbolique** : convertir sous-graphes en vecteurs pour recherche rapide
- **Conflits** : detecter et resoudre les contradictions dans le graphe

### Fichiers concernes
- `src/memory/graph.rs`
- `src/memory/compress.rs`
- `src/memory/analogy.rs`

---

## Chantier 4 : Auto-Amelioration

### Principe
1. Mesurer la fitness actuelle (accuracy + taille + vitesse + memoire)
2. Generer des mutations (ajouter/supprimer regles, changer parametres)
3. Appliquer la mutation, re-mesurer
4. Si mieux → garder. Si pire → rollback.

### Types de mutations
- `AddRule` : ajouter une regle au RuleEngine
- `RemoveRule` : supprimer une regle
- `ModifyRuleHead` : changer la conclusion d'une regle
- `AddFact` : ajouter un fait
- `RetractFact` : supprimer un fait
- `SwapRules` : changer l'ordre de priorite

### A terme
- Auto-modification du code source Rust
- Auto-compilation + test + rollback
- Genetic programming sur l'arbre syntaxique du RuleEngine
- Self-replication (generer un nouveau projet complet)

### Fichiers concernes
- `src/self_improve/fitness.rs`
- `src/self_improve/mutator.rs`

---

## Chantier 5 : Minimisation du Code

### Objectif
< 100K lignes, zero test inline, zero commentaire, code ultra-dense.

### Strategie
1. Supprimer tous les dead code warnings (cargo fix)
2. Merge les modules redondants
3. Inline les abstractions a usage unique
4. Eliminer les allocations inutiles
5. Benchmark perf pour chaque changement

---

## Chantier 6 : NLU/NLG Bridge

### Principe
Le LLM ne raisonne JAMAIS. Il parse et genere du texte.

```
User: "Quelle est la capitale de la France ?"
  → NLU (LLM 3B) → query(capital_of, france, ?X)
  → Reasoning (Rust) → backward chain → ?X = paris
  → NLG (LLM 3B) → "La capitale de la France est Paris."
```

### Implementation
- Ollama/llama.cpp pour le modele 3B
- Prompt structure : "Parse cette question en terme logique: ..."
- Validation : le Term genere doit etre syntaxiquement valide

---

## Chantier 7 : Benchmarks

### ARC-AGI
- 400 taches d'entrainement + 400 d'evaluation
- Chaque tache : 3 exemples train + 1 test
- Score = % de taches resolues (grille identique)

### HumanEval (via synthesis)
- 164 exercices Python
- Au lieu d'un LLM qui genere du code, synthesiser le programme
- Limiter aux exercices reductibles a un DSL (filtrage, transformation)

### Metriques internes
- Nombre de regles apprises par le RuleEngine
- Profondeur moyenne des preuves backward chain
- Taux de succes du forward chaining
- Vitesse de resolution (ms par query)
