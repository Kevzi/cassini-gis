---
title: 'Le rejeu accepte une étape QGIS qui redonne la même erreur'
type: 'bugfix'
ticket: ''
created: '2026-11-07'
status: 'built'
baseline_revision: 'd4402238f9918c27653814dc9bddb7f1c2a4c689'
route: 'oneshot'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Un journal qui contient une étape QGIS échouée, par exemple `analysis.buffer` refusé en `analysis.crs_not_metric` ou `layout.export_pdf` en `layout.bad_params`, ne se rejoue pas. `Replay::run` (`core/src/journal/mod.rs`) accepte que l'étape redonne la même erreur, puis exige quand même son fichier de sortie et conclut à tort en `replay.diverged`. La sonde 11.18 sur Annecy l'a montré deux fois (2026-11-07).

**Approach:** Quand l'étape rejouée redonne la même erreur que le journal, ne pas contrôler ses fichiers de sortie : une étape échouée n'en a pas, ni à l'origine ni au rejeu. Tout le reste ne change pas : même résultat exigé pour une étape réussie, octets identiques pour une sortie du noyau, et fichier non vide exigé pour une sortie de QGIS réussie. Un test de journal le couvre : une étape QGIS refusée se rejoue à l'identique, et une étape refusée à l'origine mais réussie au rejeu diverge toujours.

</frozen-after-approval>

## Implementation Notes

Route courte : une condition dans `Replay::run` et deux tests dans `core/src/journal/tests.rs`, avec le faux QGIS `SessionQgis`, qui gagne un drapeau de refus du tampon (`RegistryError::Delegated`).

Fait : `continue` après la comparaison du résultat ou de l'erreur, quand `outcome` est une erreur et que la commande s'exécute dans QGIS (`core/src/journal/mod.rs`). Après la relecture, le saut est limité à QGIS : une commande du noyau garde exactement sa comparaison d'octets. Tests ajoutés : `a_qgis_step_refused_again_replays_identically_and_the_replay_goes_on`, qui échoue sans le correctif, et `a_qgis_step_refused_at_first_but_done_on_replay_diverges`. Suite du noyau : 551 tests verts ; `fmt` et `clippy` propres.

De bout en bout : noyau `--features gpu` reconstruit et installé, puis rejeu seul (`%TEMP%\sonde-11-18\rejeu-seul.py`) du vrai journal d'Annecy, qui a 10 étapes et finit par `layout.export_pdf` refusé en `layout.bad_params`. Résultat : `{"steps": 10, "identical": true}`. Avant le correctif, ce journal divergeait à l'étape 10.

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: tous verts, dont les deux nouveaux tests du journal
- `cargo fmt --check && cargo clippy -p cassini-core --all-targets -- -D warnings` -- expected: sans avertissement

## Review Triage Log

Lentilles : blind-hunter (8 constats), edge-case-hunter (0), verification-gap (0), intent-alignment (descriptif).

- medium, patch : le saut s'appliquait aussi aux commandes du noyau échouées, et l'on aurait perdu la comparaison d'octets d'un éventuel fichier partiel. Le saut est limité à `RunsIn::Qgis`, comme le dit le problème.
- medium, patch : aucune preuve sur un vrai journal QGIS, ni sur le cas `layout.export_pdf`. Le rejeu seul du journal réel d'Annecy (10 étapes, export refusé) est identique.
- low, patch : la vérification ne lançait que le filtre `journal` ; elle lance maintenant toute la suite.
- low, rejeté : fichier partiel laissé par QGIS lors d'un échec. Le rejeu écrit dans un dossier neuf (`fresh_replay_dir`) et aucun lecteur n'y reprend une étape échouée ; la vérifier n'apporterait rien.
- false : avec un export refusé, `replay_param` laisserait QGIS redécider. Le journal réel rejoue l'export à l'identique (même refus), et rien n'est décidé puisque rien n'est produit.
- low, rejeté : l'étape 3 n'est pas observée en détail. `identical: true` sur 3 étapes compare déjà son résultat, couche liée comprise.
- low, rejeté : deux codes d'erreur différents. La branche `(Err(e), None, Some(code))` n'a pas changé et rend toujours `false`, donc `replay.diverged`.
- false : le message de l'erreur n'est pas comparé. C'est le contrat existant : seul le code est stable (AD-21).
- false : frontmatter. `ticket` est vide parce que c'est un travail hors arbre ; `lenses_ran` est écrit par le workflow avant le tri.
