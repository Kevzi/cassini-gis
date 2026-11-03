---
title: "Second rejeu dans la même session QGIS : l'export PDF voit les couches du rejeu précédent"
type: 'bugfix'
ticket: '36'
created: '2026-11-03'
status: 'built'
baseline_revision: '92e9d19cb544c655d1e61c5db983f68fecbf7935'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Depuis 5.35, un second rejeu sans rouvrir le projet passe les étapes 1 à 4 sous `replay-2/`, puis rend `replay.diverged` à `layout.export_pdf` : le geste dessine et compte toutes les couches visibles du projet, dont celles posées par le rejeu précédent, et rend donc un `layer_count` plus grand que celui du journal.

**Approach:** Au rejeu, la carte exportée ne montre que ce que l'exécution d'origine montrait : les couches posées par une exécution précédente du même journal (session d'origine ou rejeu antérieur) sont laissées hors de la carte, sur décision du noyau ; le résultat de l'export reste comparé tel quel, `layer_count` et `sources` compris.

**Decision (2026-11-03, propriétaire) :** remède (a) — au rejeu seulement, le noyau passe à `layout.export_pdf` un paramètre facultatif `leave_out` (ids de couche à laisser hors de la carte) : ids du journal et ids créés par les rejeux précédents de ce processus du noyau (mémoire perdue si le noyau est relancé, limite acceptée). Écartés : (b) commande de retrait de couche (modifie le projet, nouvelle commande), (c) ne plus comparer `layer_count` (contraire au ticket).

**Decision (2026-11-03, propriétaire) :** les couches de la session d'origine encore présentes (ids `layer_id` du journal) sont incluses dans ce ticket.

## Boundaries & Constraints

**Always:** le noyau décide quelles couches laisser hors de la carte, l'extension exécute (aucune logique métier côté extension) ; un export hors rejeu (agent, MCP, `command.run`) reste inchangé ; le journal n'est jamais écrit par le rejeu ; une couche en trop ou en moins par rapport au journal fait toujours diverger le rejeu ; les couches laissées hors de la carte restent dans le projet, intactes et visibles.

**Never:** décharger, masquer ou modifier une couche du projet de l'utilisateur ; assouplir la comparaison de `layer_count` ou `sources` ; nouvelle dépendance ; nouvelle commande du registre.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Premier rejeu, projet rouvert | aucune couche d'une exécution précédente | export identique à aujourd'hui, `identical: true` | — |
| Second rejeu, même session | couches du rejeu 1 visibles dans le projet | couches du rejeu 1 hors de la carte, `identical: true` | — |
| Rejeu après la session d'origine, sans rouvrir | couches créées par la session d'origine (ids du journal) encore présentes | laissées hors de la carte, `identical: true` | — |
| Couche ajoutée à la main par l'utilisateur | une couche visible de plus, inconnue du journal | elle est dessinée et comptée | `replay.diverged` à l'export |
| Couche d'origine retirée par l'utilisateur | une couche du journal absente | une couche de moins | `replay.diverged` à l'export |

</frozen-after-approval>

## Code Map

- `core/src/commands/layout_export_pdf.rs:14-23` -- `LayoutExportPdfParams` : ajouter `#[serde(default)] leave_out: Vec<String>` (ids de couche, fourni par le noyau au rejeu, comme `out`) ; doc FR/EN de la spec à compléter d'une ligne.
- `core/src/registry.rs:30-115` -- `CommandSpec` : drapeau générique `leave_out_param(&str)` (comme `output_params`) pour que le rejeu ne nomme aucune commande en dur.
- `core/src/journal/mod.rs:517-522` -- `ReplayReport` : garder sérialisés `steps`/`identical` seulement ; rendre aussi (champ `#[serde(skip)]`) les ids créés (`bound` values).
- `core/src/journal/mod.rs:671-750` -- `Replay::run` : prendre l'ensemble des ids à laisser hors (ids `layer_id` des lignes du journal ∪ ids passés par l'appelant) ; après `substitute`, si la spec a le paramètre, y mettre cet ensemble trié (jamais les ids de ce rejeu).
- `core/src/serve.rs:235-254` -- `State` : `replayed_layers: HashSet<String>` (mémoire du processus) ; `journal_replay` (l.962) le passe à `run` et y ajoute les ids créés, rejeu réussi ou non.
- Appelants de `Replay::run` hors serve (`cli` `replay`, tests `serve.rs:1881`, `journal/tests.rs`) : passer un ensemble vide.
- `qgis-plugin/cassini_qgis/layout_export.py:157-198` -- `_read` : retirer de `layers` ceux dont `layer.id()` est dans `params["leave_out"]` avant tout calcul (emprise, sources, compte) ; passer par le fond (`backend.layer_id(layer)`) pour rester testable sans QGIS.
- `qgis-plugin/tests/test_layout_export.py` -- modèle de faux fond pour les tests.
- Ne pas changer : `same_result`, `summarize`, `output_file`, `fresh_replay_dir`.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/registry.rs`, `core/src/commands/layout_export_pdf.rs` -- paramètre `leave_out` et drapeau de spec ; test de schéma (facultatif, tableau de chaînes).
- [x] `core/src/journal/mod.rs` -- `run(router, leave_out)` remplit le paramètre déclaré ; `ReplayReport` porte les ids créés hors sérialisation.
- [x] `core/src/serve.rs` -- mémoire `replayed_layers` dans `State`, branchée sur `journal_replay`.
- [x] `qgis-plugin/cassini_qgis/layout_export.py` -- filtre par id via le fond.
- [x] Tests -- `journal/tests.rs` : journal QGIS factice dont l'export compte les couches visibles d'un faux projet ; deux rejeux de suite → deux `identical: true` ; couche ajoutée « à la main » → `replay.diverged` ; `serve.rs` : deux `journal.replay` de suite dans une même session, le second reçoit au geste les ids du premier ; `test_layout_export.py` : `leave_out` retire couche, source et compte, sans `leave_out` rien ne change.

**Acceptance Criteria:**
- Given un projet qui porte les couches d'un rejeu précédent, when le journal est rejoué jusqu'à l'export PDF, then le rejeu rend `identical: true` et le PDF ne dessine pas ces couches.
- Given le journal de démo dans une session QGIS ouverte (noyau et extension installés), when Claude lance `rejeu.bat` (deux rejeux de suite sans rouvrir le projet), then les deux rendent `identical: true`.

## Implementation Notes

- `Replay::run(router, replayed_layers: &mut HashSet<String>)` : un seul paramètre en entrée et
  sortie au lieu d'un champ caché de `ReplayReport`, pour que les ids posés par un rejeu qui
  **diverge** soient eux aussi retenus (« rejeu réussi ou non ») ; `ReplayReport` est inchangé.
  L'ensemble `leave_out` est figé avant la première étape (ids `layer_id` du journal ∪ mémoire),
  trié, et posé après `substitute` : jamais un id de ce rejeu.
- Le fond de l'extension avait déjà `id_of(layer)` (`inventory.QgisBackend`) : repris au lieu
  d'un nouveau `layer_id(layer)`. Un `leave_out` qui n'est pas une liste de chaînes rend
  `layout.bad_params` ; toutes les couches laissées hors → `layout.empty`, comme aujourd'hui.
- `leave_out` est ôté du catalogue montré au modèle (`ai/src/plan.rs`, comme `out`) ; les
  descriptions FR/EN de la commande sont inchangées : le prompt du modèle ajusté ne bouge pas.
  Seul le commentaire du champ porte la doc.
- Recette : la sonde hors dépôt `probe_replay_5_9.py` relançait le noyau entre les deux rejeux,
  ce qui efface la mémoire (limite acceptée). Elle rejoue désormais par le même noyau, comme le
  panneau ; `CASSINI_PROBE_RESTART=1` garde l'ancien comportement (le second rejeu y diverge
  encore à l'export). Ancienne version gardée en `probe_replay_5_9.py.avant-5-36`.

## Plan Change Log

## Review Triage Log

Relecture `thorough` (4 lentilles), itération 0.

| # | Lentille | Constat | Verdict | Route | Preuve |
|---|---|---|---|---|---|
| 1 | blind, edge, verif, intent | Un `leave_out` fourni par l'appelant hors rejeu (`command.run`, MCP, agent) passe à QGIS et au journal | medium | patch (groupe A) | `Session::call` → `resolve_call` (journal/mod.rs:256, 400-420) ne réécrit que `output_params` ; contredit « export hors rejeu inchangé ». |
| 2 | verif | La vérification du plan (ai/src/plan.rs:489) ne refuse pas un plan qui écrit `leave_out` | medium | patch (groupe A) | La boucle ne parcourt que `spec.output_params` ; même cause racine que 1. |
| 3 | blind, edge | L'outil MCP publie `leave_out` dans son schéma | low | rejeté | Même traitement que `out`, déjà publié puis réécrit par le noyau ; une fois 1 corrigé, la valeur est ignorée. |
| 4 | blind, verif, edge | `check_contract` ne vérifie pas `leave_out_param` | low | rejeté | Une seule commande, figée par son test ; le correctif ajoute une garde. |
| 5 | edge, blind | Un export journalisé après un rejeu, dans la même session, compte les couches du rejeu ; le rejeu suivant les laisse hors et diverge | medium | defer | Le cas divergeait déjà avant 5.36 (couches comptées des deux côtés différemment), donc antérieur au changement et hors de l'intention. |
| 6 | edge | Une couche posée par une étape en erreur n'est pas retenue | low | rejeté | Sur une erreur du geste, rien ne montre qu'une couche reste ; pas d'id disponible. |
| 7 | edge | Mémoire perdue à la relance du noyau ; CLI `replay` sans mémoire | false | rejeté | Limite acceptée par le propriétaire (Intent, décision a). |
| 8 | edge, blind | Mémoire commune à tout le processus, croît sans fin | low | rejeté | Les ids QGIS sont uniques ; un id d'un autre projet est absent de celui-ci, donc sans effet ; la taille reste négligeable. |
| 9 | edge | Une commande rendant le `layer_id` d'une couche existante serait laissée hors | false | rejeté | Seules les commandes qui créent une couche rendent un `layer_id` de tête (layer.add, analysis.*) ; describe et inventory n'en rendent pas. |
| 10 | edge | `ReplayReport` sans champ caché, signature de `run` différente du Code Map | false | rejeté | Écart noté et justifié dans Implementation Notes ; aucun mauvais résultat. |
| 11 | blind | Ids du journal pris sur toutes les lignes, même après l'export | false | rejeté | Ces couches d'origine sont présentes dans le projet à l'export du rejeu : il est juste de les laisser hors. |
| 12 | blind | Extension ancienne : pas de contrôle de version | low | rejeté | Le noyau et l'extension s'installent ensemble ; le rejeu diverge bruyamment. |
| 13 | blind | `registry_version` change, d'où un avertissement sur les anciens journaux | low | rejeté | Comportement prévu à chaque changement de schéma. |
| 14 | blind | Assertion d'emprise trop lâche (test_layout_export.py) | low | patch | Correctif direct : comparer à l'emprise attendue de « b ». |
| 15 | blind | Aucun AD cité pour le nouveau paramètre décidé par le noyau | low | rejeté | Documentation seule ; AD-10 couvre déjà les paramètres décidés par le noyau. |
| 16 | intent | La ligne « couche d'origine retirée » est ambiguë si c'est une couche créée par le journal | false | rejeté | Une couche créée par le journal est laissée hors de toute façon ; la lecture cohérente vise une couche d'entrée, testée avec `communes_0`. |
| 17 | intent | Le vrai QGIS n'est pas couvert par les tests | false | rejeté | Couvert par la recette manuelle (rejeu.bat, deux `identical: true`), rapportée par l'implémenteur. |

## Design Notes

Le paramètre plutôt qu'un réglage de session de l'extension : le canal ne porte que des paramètres à schéma (AD), et le rejeu reste lisible dans les échanges. Les ids du journal suffisent pour la session d'origine ; pour les rejeux précédents, seule la mémoire du noyau les connaît (même portée que la session QGIS, l'extension lançant le noyau).

## Verification

**Commands:**
- `cargo test --workspace` (Windows, `cargo.exe`, `CARGO_TARGET_DIR=C:\ct`) -- expected: vert.
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement.
- tests Python de l'extension (`qgis-plugin/tests`) -- expected: verts.

**Manual checks (if no CLI):**
- Recette QGIS : installer noyau et extension, `rejeu.bat` de `../donnees-test-cassini/mesure-5-9/` → deux `identical: true`.
