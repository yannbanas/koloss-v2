# KOLOSS v2 — Vision

## Principe
Tout raisonnement se fait en Rust pur. Aucun LLM dans la boucle de raisonnement.
Un petit LLM (3B) sert uniquement de NLU/NLG : texte → structure, structure → texte.

## Architecture
```
Perception (NLU) → Working Memory (Graph) → Reasoning (Symbolic) → Action
     ↑                    ↑                        |
     └──── NLG ───────────┴────── Feedback ────────┘
```

## Modules
- core/       : Types fondamentaux, erreurs
- reasoning/  : Unifier, SAT solver, regles, recherche
- synthesis/  : DSL, enumeration, evolution de programmes
- memory/     : Graphe de connaissances, compression, analogie
- perception/ : NLU bridge, parsing grilles (ARC), parsing code
- self_improve/ : Auto-mutation, fitness, replication
- bench/      : ARC-AGI, HumanEval, fitness globale
- net/        : API minimale, git ops

## Contraintes
- < 100K lignes total
- 0 commentaire, 0 test inline
- Chaque module doit fonctionner sans les autres
- Le raisonnement ne depend JAMAIS d'un LLM
