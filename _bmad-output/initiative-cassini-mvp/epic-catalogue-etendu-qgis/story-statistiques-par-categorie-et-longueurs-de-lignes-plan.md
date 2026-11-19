---
title: 'Statistiques par catégorie et longueurs de lignes'
type: 'feature'
ticket: '11'
created: '2026-11-19'
status: 'built'
baseline_revision: 'db7f5b028af01bfe8105d2beb88ac56ce3f829a3'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le catalogue étendu n'a pas encore `analysis.statistics_by_category` (comptes et sommes par catégorie) ni `analysis.sum_line_lengths` (longueur de lignes par polygone), planifiées dans le catalogue des commandes.

**Approach:** Deux commandes déléguées à QGIS, au gabarit `SummaryRow` de 10.1 (`rows`, `truncated`), nées avec la règle des mètres sur couche en degrés : `sum_line_lengths` déclare `work_epsg`, rempli par le noyau comme pour `analysis.nearest` (10.10). Chaque commande a son id, ses paramètres et son résultat tels que fixés par les fiches du catalogue, plus `work_epsg` (changement de cap du 2026-11-12).

## Boundaries & Constraints

**Always:** une ligne de la table fermée par commande (chaîne fixe, `chain_version` 1, annulable déduite) ; `statistics_by_category` écrit un tableau sans géométrie dans `out` et rend `file`, `rows`, `truncated` ; `sum_line_lengths` rend une couche (`layer_id`, `name`, `feature_count`, `crs`) plus `rows` valeur `length` avec unité (`m` par défaut, `km`) ; chaque commande vient avec ses tests ; les noms de champs des algorithmes QGIS sont confirmés par une sonde sous vrai QGIS, jamais supposés ; le noyau ne contient aucun nom d'algorithme QGIS (AD-22) ; ids et paramètres du catalogue inchangés, sauf l'ajout `work_epsg`.

**Never:** calculer un CRS dans l'extension (AD-5, AD-24) ; modifier `SummaryRow`, `MAX_ROWS` ni `analysis.nearest` ; ajouter une famille d'entraînement du modèle (le ré-entraînement vient après le dernier ticket de l'epic) ; modifier les fichiers d'origine ; réseau.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Comptes par catégorie | couche, `category_field` | une ligne par catégorie avec `count`, `file` écrit | — |
| Avec somme | `sum_field` numérique | `sum` en plus, par catégorie | champ non numérique refusé avant écriture |
| Plus de 100 catégories | 150 catégories | 100 lignes, `truncated: true` | — |
| Longueurs, couche en degrés | polygones en EPSG:4326 | longueurs en mètres dans `work_epsg` rempli par le noyau | — |
| Longueurs en km | `unit: km` | valeurs divisées par 1000, unité `km` | — |
| Sans ligne dans un polygone | polygone sans ligne | `length` à 0, ligne présente | — |
| Un algorithme absent | chaîne incomplète | commande non annoncée (10.4) | refus avant affichage du plan |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_count_points_in_polygon.rs`, `analysis_nearest.rs` -- modèles : `SummaryRow`, `output_params`, `work_crs_param`, `tests(&[…])`. Nouveaux : `analysis_statistics_by_category.rs`, `analysis_sum_line_lengths.rs` (auto-déclarés par `core/build.rs`).
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- lignes 376-377 et fiches l.1976, l.2003 : `planifiée` -> `faite`, ajouter `work_epsg` à `sum_line_lengths` (modèle 10.10) ; lu par `core/tests/catalogue.rs`.
- `qgis-plugin/cassini_qgis/analysis.py` -- `ALGORITHMS` (l.442+), `CANCELLABLE_ALGORITHMS`, `_REPROJECT_TO_WORK`, `_UNIT_TO_METERS`, `_begin`/`_load` (l.1047-1314), `validate_table` ; `QgisBackend` et `FakeBackend` (`tests/test_analysis.py`) : lectures de lignes de catégories et de longueurs.
- `qgis-plugin/cassini_qgis/summary.py`, `texts.py` -- phrases FR/EN ; `tests/test_summary.py` (`SAMPLES`, complétude des commandes `faite`).
- `qgis-plugin/tests/table_annulable.json`, `fixtures/resultats-extension.json`, `tests/test_result_conformance.py`, `core/tests/conformite_resultats.rs` -- un exemple de résultat par commande déléguée.
- `core/src/journal/tests.rs` -- mock `CrsQgis` (~l.4702) : bras `analysis.sum_line_lengths` ; modèle `a_nearest_join_of_a_layer_in_degrees_logs_the_metric_work_system`.
- `ai/tests/par_commande/phrases.json`, `training/src/barres.rs` (compte 102) -- une phrase par commande ; compte relevé.
- `README.md` (l.119-120), `qgis-plugin/README.md`, nouvelles sondes `probe_statistics_by_category.py`, `probe_sum_line_lengths.py` (modèle `probe_nearest.py`).

## Tasks & Acceptance

**Execution:**
- [ ] sondes sous QGIS -- confirmer les noms de champs (`count`, `sum`, longueur) et l'unité de `native:statisticsbycategories` et `native:sumlinelengths` avant d'écrire les chaînes
- [ ] `core/src/commands/` (2 fichiers), catalogue des commandes -- commandes déléguées, fiches `faite`, `work_epsg`, tests du noyau et du journal
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` -- deux lignes de table, mode « tableau » pour `out`, résumés, phrases FR/EN
- [ ] tests Python, conformité, `table_annulable.json`, `par_commande`, READMEs -- tous les exemples et listes synchronisés

**Acceptance Criteria:**
- Given une couche et un champ de catégorie, when on lance `statistics_by_category`, then comptes et sommes par catégorie sont exacts et validés par le noyau.
- Given des polygones en degrés, when on lance `sum_line_lengths`, then le noyau remplit `work_epsg` et les longueurs sont en mètres, avec unité.
- Given une phrase de l'utilisateur, when l'agent compose, then le plan qui nomme la commande est valide.

## Implementation Notes

- La sonde sous QGIS 3.44.15 a montré que `native:statisticsbycategories` n'existe pas : l'algorithme est `qgis:statisticsbycategories`, du fournisseur du plugin Traitement. La commande n'est donc annoncée que si ce fournisseur est chargé (10.4). Les valeurs nulles faisaient perdre des comptes : la chaîne commence par `native:fieldcalculator` (seulement avec `sum_field`) qui les remplace par 0.
- Mot-clé d'un test de refus changé de `categor` en `categorized` (`training/src/families.rs`, `ai/tests/style_reel/phrases.json`), pour ne pas correspondre à la nouvelle commande ; aucune famille d'entraînement ajoutée.

## Plan Change Log

## Review Triage Log

Passe 1 (lentille : quick). Verdicts : 0 high, 1 medium, 1 low, 4 rejetés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch (fait) | `qgis-plugin/README.md` : deux chemins du lanceur contenaient un caractère retour arrière (`` lu comme ``), non copiables ; vérifié octet par octet, corrigé, plus aucun caractère de contrôle. |
| low | patch (fait) | `summary.py`, `_statistics_by_category` : `row.get("label")` évalué sans garde `isinstance(row, dict)` ; garde ajouté. |
| low | rejeté | Changement de `categor` en `categorized` (`training/src/families.rs`, `style_reel/phrases.json`) hors carte du code : nécessaire, l'ancien mot-clé de refus se mettait à correspondre à `statistics_by_category` ; aucune famille ajoutée. |
| false | rejeté | `qgis:statisticsbycategories` au lieu de `native:` : écart voulu, la sonde sous QGIS 3.44.15 a montré que `native:` n'existe pas ; noté dans Implementation Notes et le README. |
| low | rejeté | `label_field` égal à `length_m` ou `line_count` donne la valeur calculée : cas improbable, correctif = garde de plus. |
| false | rejeté | Seule la couche de lignes en degrés : `native:sumlinelengths` reprojette les lignes dans le SCR des polygones (sonde) ; `work_epsg` se déduit de `layer_id` comme en 10.10. |

## Design Notes

Le résultat de `statistics_by_category` n'a pas de `layer_id` : un tableau écrit dans `out` (CSV sans géométrie), non ajouté au projet. Cela demande un mode de geste « tableau » à côté du mode couche et du mode fichier existants.

## Verification

**Commands:**
- `cargo test -p cassini-core --locked` et `cargo test -p cassini --locked` -- expected: succès
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: succès

**Manual checks (if no CLI):**
- Les deux sondes sous vrai QGIS (`python-qgis-ltr.bat`) rendent un verdict OK, avec le cas en degrés.
