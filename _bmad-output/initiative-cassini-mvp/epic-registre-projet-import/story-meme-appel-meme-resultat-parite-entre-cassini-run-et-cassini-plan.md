---
title: 'Même appel, même résultat : parité entre cassini run et cassini serve'
type: 'feature'
ticket: '8'
created: '2026-10-15'
status: 'built'
baseline_revision: 'bedbd710f415572d375ad997bb57d49910ff39ea'
route: 'oneshot'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: []
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** rien ne prouve qu'une commande du registre donne le même résultat par `cassini run` et par `cassini serve` (R3, AD-13) ; une divergence passerait inaperçue.

**Approach:** un test d'intégration lance le vrai binaire `cassini` par les deux points d'entrée, sur un projet de test de même contenu, pour chaque commande `core` du registre et chacun de ses exemples déclarés, puis compare les résultats octet à octet après canonisation (JSON relu puis réécrit, clés triées). Aucun QGIS.

</frozen-after-approval>

## Tasks & Acceptance

**Execution:**
- [ ] `cli/tests/parity.rs` -- test de parité sur le binaire (`CARGO_BIN_EXE_cassini`) : énumère `Registry::standard().specs()`, garde `runs_in == Core`, rejoue chaque exemple par `run --project` et par `serve` (hello puis `command.run`) -- R3
- [ ] `cli/tests/parity.rs` -- test négatif : un résultat altéré par un point d'entrée est détecté par la comparaison -- « échoue quand un point d'entrée altère un résultat »

**Acceptance Criteria:**
- Given chaque commande core du registre et ses exemples, when elle est appelée par `run` et par `serve` sur un projet de même contenu, then les résultats canonisés sont identiques octet à octet.
- Given un résultat altéré d'un octet, when on compare, then la comparaison échoue.
- Given une commande core sans exemple, then le test échoue en la nommant (aucune commande n'échappe à la parité).

## Implementation Notes

Oneshot : un seul fichier de test, pas de changement de code de production. Les commandes de test du noyau (`test.*`) sont sous `cfg(test)` de `cassini-core` et donc absentes du binaire : la parité couvre ce que le binaire expose réellement.

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement

## Review Triage Log

Relecture par le développeur seul (aucun sous-agent lancé : la session n'en autorise pas sans demande explicite). Rien de relevé : les deux tests passent, clippy et fmt sont propres, aucun code de production touché. Limite connue : les commandes `test.*` n'existent que sous `cfg(test)` du noyau, donc la parité ne les couvre pas.
