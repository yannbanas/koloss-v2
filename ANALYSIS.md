# KOLOSS v1 → v2 : Analyse Critique

## Pourquoi v2

KOLOSS v1 est un orchestrateur LLM sophistique (~65K lignes, 1103 tests, 50 phases).
Toute l'intelligence vient d'Ollama. Si on coupe Ollama, KOLOSS a l'intelligence d'un echo.

| Capacite | v1 pretend | Realite v1 | v2 |
|----------|-----------|------------|-----|
| Raisonnement | lobes cerebraux | prompt engineering | Unifier + Rules + SAT |
| Memoire | Mnemos + StigMem | BM25 keyword match | Knowledge Graph structure |
| Auto-amelioration | Autopoiesis | LLM reecrit du code | Mutation + Fitness + Selection |
| Conscience | ThoughtStream | Logs JSON | N/A (pas un objectif) |
| Apprentissage | benchmark loop | LLM resout, on stocke | Generalisation symbolique |
| Goals | auto-goals | LLM genere des phrases | Rules-based planning |

## Les 7 Murs Vers l'AGI

### 1. Aucun raisonnement propre (v1)
v1 ne peut pas additionner 2+2 sans LLM.
v2: Unifier, SAT solver, Rule Engine — raisonnement Rust pur.

### 2. Memoire = stockage, pas comprehension (v1)
v1 stocke des strings et fait du BM25.
v2: Knowledge Graph + anti-unification + analogie structurelle.

### 3. ARC-AGI = program synthesis, pas LLM
Les LLMs plafonnent a ~30% sur ARC. Le SOTA (55-60%) utilise DSL + search.
v2: 134 primitives DSL + enumeration + evolution genetique.

### 4. Remplacer les LLMs
La seule voie realiste : hybrid neuro-symbolique.
Petit LLM (3B) pour NLU/NLG, tout le raisonnement en Rust pur.

### 5. Auto-amelioration reelle
v1: LLM genere du code Rust et commit. Aucune garantie de correction.
v2: Fitness mesurable + mutation + selection. Garder seulement ce qui ameliore.

### 6. < 100K lignes optimisees
v1: 65K lignes dont 40% plomberie (Axum, SSE, dashboard, serde, SQL).
v2: 2500 lignes, cible < 100K avec tout le raisonnement.

### 7. Self-replication
v1: ne peut pas se compiler, deployer, debugger seul.
v2: objectif futur, framework de mutation deja en place.

## Score Honnete par Objectif

| Objectif | v1 | v2 actuel | v2 cible | Impossible ? |
|----------|-----|-----------|----------|-------------|
| AGI 100% autonome | 2/100 | 5/100 | 15/100 | Oui (non-resolu) |
| Auto-replication | 0/100 | 2/100 | 40/100 | Non, faisable |
| Auto-amelioration | 1/100 | 5/100 | 30/100 | Partiellement |
| 99% ARC-AGI | 0/100 | 1/100 | 15/100 | Oui (record ~60%) |
| Remplacer LLMs | 0/100 | 30/100 | 50/100 | Pour NLU non, pour raisonnement oui |
| < 100K optimise | 65/100 | 95/100 | 90/100 | Faisable |
