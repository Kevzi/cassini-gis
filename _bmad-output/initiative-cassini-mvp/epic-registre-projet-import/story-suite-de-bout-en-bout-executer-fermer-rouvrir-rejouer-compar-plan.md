---
title: 'Suite de bout en bout : exécuter, fermer, rouvrir, rejouer, comparer'
type: 'feature'
ticket: '10'
created: '2026-10-15'
status: 'built'
baseline_revision: 'e30262bb26fd2a7e19d05cb946d03b638f548da8'
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

**Problem:** le journal s'écrit (2.5) et se rejoue (2.6), mais aucun test ne parcourt le cycle entier sur le vrai binaire : R4 et R5 (rejeu identique après fermeture et réouverture, journal en texte lisible) ne sont vérifiés qu'en pièces détachées.

**Approach:** un test d'intégration lance le binaire `cassini` sur un projet de test : une suite de commandes par `run`, puis par `serve` (la session précédente est fermée, la suivante la rouvre et continue le journal), puis `replay`, et compare chaque résultat rejoué à celui d'origine. Il vérifie que le journal est du texte JSON lisible, à côté du projet, sans chemin absolu, et qu'un rejeu ne le modifie pas. Aucun QGIS.

</frozen-after-approval>

## Tasks & Acceptance

**Execution:**
- [ ] `cli/tests/end_to_end.rs` -- scénario sur le binaire (`CARGO_BIN_EXE_cassini`) : `run` ×2, fermeture, `serve` (rouvre, étapes 3 et 4), fermeture, `replay` ; compare le bilan et les résultats rejoués -- R4, R5
- [ ] `cli/tests/end_to_end.rs` -- contrôles du journal : texte UTF-8, une ligne = un objet JSON, `step` 1..n, posé dans `<projet>.cassini/`, sans chemin absolu, inchangé par le rejeu -- R4
- [ ] `cli/tests/end_to_end.rs` -- tests négatifs : un résultat altéré dans le journal fait échouer le rejeu (`replay.diverged`) ; un journal binaire est refusé par le contrôle de texte

**Acceptance Criteria:**
- Given un projet de test et quatre appels répartis sur deux sessions fermées puis rouvertes, when `cassini replay`, then `{"steps":4,"identical":true}`, résultats rejoués égaux à ceux d'origine.
- Given un résultat modifié dans le journal, when on rejoue, then échec `replay.diverged`.
- Given un journal non textuel, when on le contrôle, then le test échoue.

## Implementation Notes

Oneshot : un seul fichier de test, aucun code de production. Les commandes `test.*` n'existent que sous `cfg(test)` du noyau et donc pas dans le binaire : la suite emploie `system.about`, seule commande du binaire, et la couverture des entrées et sorties de fichiers reste celle de 2.5 et 2.6 dans `core`. La commande de recette est `cargo test -p cassini --test end_to_end --locked` ; la recette de l'intégration continue lance déjà `cargo test -p cassini`.

## Review Triage Log

Relecture par le développeur seul (aucun sous-agent lancé : la session n'en autorise pas sans demande explicite). Rien de relevé : les trois tests passent, clippy et fmt sont propres, aucun code de production touché. Limite connue : la suite n'emploie que `system.about`, donc les entrées et sorties de fichiers restent couvertes par les tests de `core` (2.5, 2.6).

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement
