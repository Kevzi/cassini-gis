---
title: 'Annonce fidèle : chaîne présente, fournisseurs chargés, versions journalisées'
type: 'feature'
ticket: '4'
created: '2026-11-19'
status: 'built'
baseline_revision: '055c25d6c041b0bc538b2ecb91fbea0abfabb9f0'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, edge-cases, adversarial]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** L'extension annonce tous ses gestes sans vérifier que les algorithmes de traitement existent dans le QGIS hôte, et ni la version de QGIS ni la version de chaîne ne sont annoncées ni journalisées : un rejeu sur un autre QGIS peut diverger sans explication (AD-25, AD-11).

**Approach:** L'extension envoie `session.hello` une fois les fournisseurs chargés, n'y annonce une commande que si toute sa chaîne existe, et joint `qgis_version` et `chain_versions`. Le noyau les journalise dans `versions` et le rejeu signale une version de chaîne changée (avertissement, pas échec).

## Boundaries & Constraints

**Always:** ajout compatible dans `session.hello` (champs facultatifs, `PROTOCOL_VERSION` reste 2) ; règle unique pour toute ligne de la table fermée, `data.reproject` comprise ; aucune logique métier ajoutée dans l'extension au-delà de l'existence des algorithmes ; les étapes `when` comptent comme requises ; décision : le rejeu remonte un champ `warnings` dans `ReplayReport`.

**Never:** changer `registry_version`, `versions.core`, `versions.protocol`, `validate_table`, `NETWORK_BLACKLIST`, `qgis-plugin/tests/table_annulable.json` ; comparer une ligne de journal ancienne sans `versions.chain` ; rendre le rejeu fatal sur simple écart de version ; toucher aux gestes fixes sans algorithme.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Chaîne complète | tous les algorithmes présents | commande annoncée avec sa `chain_version` | — |
| Algorithme retiré | `native:buffer` absent du registre de traitement | `analysis.buffer` non annoncée, `available:false` | un plan qui la nomme est refusé avant affichage (`plan_invalid`) |
| Fournisseurs non chargés | hello demandé trop tôt | hello différé jusqu'au chargement | — |
| Journal | commande déléguée annoncée | `versions` porte `qgis` et `chain` | — |
| Rejeu, chaîne changée | `versions.chain` du journal ≠ version annoncée | avertissement dans `warnings` du rapport | rejeu poursuivi |
| Journal ancien | ligne sans `versions.chain` | aucune comparaison | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/client.py` -- `CoreClient.start()` (l.160-173) construit le hello ; réponse lue l.519-540.
- `qgis-plugin/cassini_qgis/plugin.py` -- `_launch()` (l.183-204) : attendre les fournisseurs avant `client.start`.
- `qgis-plugin/cassini_qgis/gestures.py` -- `GestureRegistry.ids()` : liste brute à filtrer.
- `qgis-plugin/cassini_qgis/analysis.py` -- `ALGORITHMS` (l.442+), `chain_of`, `QgisBackend.run_algorithm` (l.764, `algorithmById`) : ajouter `algorithm_exists`.
- `qgis-plugin/cassini_qgis/delegated.py` -- `table_rows()` : source des algorithmes requis et de `chain_version`.
- `qgis-plugin/tests/test_analysis.py` (`FakeBackend`), `test_chain.py` -- doublure et modèle de test.
- `core/src/serve.rs` -- `hello` (l.846-940), `state.announced`, motif `style_kinds` à imiter ; `journal_replay` (l.1077).
- `core/src/journal/mod.rs` -- `versions` (l.764-770), `Session` builders (l.246-260), `Replay::load/run`, `ReplayReport` (l.1353), `warnings()`.
- `core/src/serve/plans.rs` -- `verify` : refus déjà en place, à ne pas changer.
- `core/src/journal/tests.rs`, tests de `serve.rs` -- modèles de tests.
- `docs/canal-local.md` -- section `session.hello`.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- `algorithm_exists` sur le fond, et fonction qui rend les commandes dont toute la chaîne existe -- source unique de l'annonce
- [ ] `qgis-plugin/cassini_qgis/client.py`, `plugin.py` -- hello filtré, avec `qgis_version` et `chain_versions`, envoyé après chargement des fournisseurs -- AD-25
- [ ] `core/src/serve.rs`, `core/src/journal/mod.rs` -- lire les champs facultatifs, les porter dans `versions` (`qgis`, `chain`), comparer au rejeu, `ReplayReport.warnings` -- AD-11
- [ ] `docs/canal-local.md` -- documenter les champs
- [ ] tests Python (`test_analysis.py`/`test_chain.py`) et Rust (`journal/tests.rs`, `serve.rs`) couvrant la matrice

**Acceptance Criteria:**
- Given un algorithme retiré du registre de traitement, when l'extension annonce, then la commande est indisponible et un plan qui la nomme est refusé avant d'être montré.
- Given une commande déléguée exécutée, when on lit le journal, then `versions` porte la version de QGIS et la version de chaîne.
- Given une chaîne dont la version a changé, when on rejoue, then le rapport porte un avertissement et le rejeu va au bout.
- Given un journal ancien, when on rejoue, then aucun avertissement de version.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (lentilles : quick, edge-cases, adversarial). Verdicts : 3 medium, 6 low, 2 maybe-false, 9 false/rejetés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch (fait) | `available_chains` lisait `chain_version` hors du `try` : une ligne sans clé levait KeyError dans `_launch`. Corrigé, test ajouté. |
| medium | patch (fait) | `_launch` créait QProcess/CoreClient avant le calcul de l'annonce : une exception laissait un client figé que `_refresh` ne relançait plus. Calcul remonté avant. |
| medium | patch (fait) | `versions.qgis` n'était écrit que si la commande avait une `chain` : `layer.add`, `layout.export_pdf` etc. ne portaient jamais la version de QGIS. Écrit pour toute commande QGIS annoncée. |
| low | patch (fait) | Aucun test Rust ne fixait « commande non annoncée -> available:false, plan refusé » (critère 1). Test ajouté. |
| low | patch (fait) | Test `"C:\p.qgz"` contenait le caractère BEL ; chaîne brute et assertion du chemin à `/`. |
| low | patch (sans effet) | `State.qgis_version` jamais lu : clippy `-D warnings` ne le signale pas, laissé. |
| medium | defer | Les avertissements de version sont perdus quand le rejeu diverge (`replay.diverged`) : la cause probable n'est pas expliquée. Le correctif ajoute de la surface à l'erreur, hors du ticket. |
| medium | defer | `REPLAY_DONE` (client.py) ne relaie que `steps` et `identical` : le panneau n'affiche pas `warnings`. |
| low | defer | La version de QGIS est journalisée mais jamais comparée au rejeu ; seul le changement de `chain` avertit (conforme au ticket). |
| maybe-false | defer (medium, non vérifié) | Repli `mainWindow().isVisible()` : peut précéder `initializationCompleted`, ou attente sans fin si fenêtre masquée ; `gdal:warpreproject` dépend du plugin Traitement. À trancher par une sonde sous vrai QGIS 3.44 et 4.x. |
| maybe-false | défer (low) | `AttributeError` large dans `initGui` / `available_chains` : masque un bogue sans trace. |
| low | rejeté | Avertissements non portables en double (stderr de la CLI et JSON du bilan) : flux distincts, documenté. |
| low | rejeté | `chain_warnings` parcourt aussi les lignes annulées : le message dit déjà « le résultat peut différer ». |
| low | rejeté | Un seul champ invalide dans `chain_versions` refuse tout le hello (-32602) : même règle que `style_kinds`, seul producteur : l'extension. |
| false | rejeté | Commande retirée sans avertissement au rejeu : elle échoue à `Announced`, résultat attendu. |
| false | rejeté | `layer.remove` contradictoire : c'est une ligne de la table (`layer_remove.TABLE_ROW`). |
| false | rejeté | Tests tautologiques, hello à clé inconnue, état du plan « in-progress » dans le diff : ni défaut du code ni contraire au ticket. |

## Verification

**Commands:**
- `cargo test -p cassini-core --locked` -- expected: succès
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: succès
