---
title: 'Matrice de distances (analysis.distance_matrix)'
type: 'feature'
ticket: '36'
created: '2026-11-23'
status: done
baseline_revision: '37ca584ab6b40e9ca15a1da7fc5750487177ab8f'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Cassini sait donner le plus proche voisin (`analysis.nearest`) mais pas la matrice des distances entre deux couches de points : « donne-moi les distances entre chaque école et chaque gare » échoue. La commande est promise par l'epic 10 et le catalogue (statut `planifiee`) sans avoir été construite.

**Approach:** Ajouter `analysis.distance_matrix`, déléguée à `qgis:distancematrix` (l'identifiant réel dans QGIS 3.44.15, lu dans `PointDistance.py` ; `native:distancematrix` n'existe pas), sur le modèle d'`analysis.nearest` (couche en degrés calculée en mètres par le noyau, `work_epsg`) et d'`analysis.statistics_by_category` (sortie table CSV, `rows` bornées à 100).

## Boundaries & Constraints

**Always:** exécution par QGIS, aucune logique métier dans l'extension (CLAUDE.md §2.7) ; table nouvelle, jamais de modification d'une couche d'origine ; commande non annoncée si `qgis:distancematrix` manque (AD-25) ; textes en français et en anglais ; une famille d'entraînement (`training/tests/couverture.rs` l'exige) ; unité `m` portée par chaque distance.

**Never:** ré-entraîner le modèle (un seul passage après 10.38) ; toucher au geste `ProcessingGesture`, à `run_algorithm`, à `analysis.buffer` ou `analysis.nearest` ; utiliser le type de matrice « standard » ou « résumé » de QGIS (faux pour k < nombre de cibles, division par zéro) ; convertir en silence des lignes ou polygones en points ; ajouter une dépendance.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Toutes les paires | deux couches de points, `neighbors` absent, N×M ≤ 100 000 | un CSV, une ligne par paire (départ, cible, distance en m) ; `rows` ≤ 100, `truncated` vrai si plus | — |
| Voisins limités | `neighbors` = 2 | au plus 2 cibles par départ, les plus proches, triées par distance | égalité de distance : au plus k lignes gardées |
| Couche en degrés | WGS84 | distances en mètres, `work_epsg` journalisé | — |
| Trop de paires | `neighbors` absent, N×M > 100 000 | refus avant lancement, message qui demande `neighbors` | erreur nommée |
| Non-point | couche de lignes ou polygones, ou MultiPoint | refus avant lancement, type nommé | erreur nommée |
| Même couche des deux côtés | `layer_id` = `target_layer_id` | refus du noyau (comme `analysis.nearest`) | erreur nommée |
| Algorithme absent | `qgis:distancematrix` manquant | commande non annoncée | AD-25 |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_distance_matrix.rs` (nouveau) -- paramètres d'après `analysis_nearest.rs` (`RawParams`/`try_from`, `.work_crs_param("work_epsg","layer_id")`, `.output_params(&["out"])`, `.output_ext(".csv")`, `inventory::submit!`), résultat d'après `analysis_statistics_by_category.rs` (`file`, `rows` ≤ 100, `truncated`) ; vérifier la ligne `mod` de `core/src/commands/mod.rs`.
- `core/src/journal/tests.rs` (~4867, ~5022) -- cas du faux exécutant, rejeu identique, `work_epsg` journalisé.
- `qgis-plugin/cassini_qgis/analysis.py` -- traductions et entrée de `ALGORITHMS` : copier de `analysis.nearest` (~1075) `metric`, `work_crs`, `overlay` et l'étape `_REPROJECT_TO_WORK`, de `analysis.statistics_by_category` (~1124) `table` ; contrôle de géométrie (`_GEOMETRY_LABELS` ~1875, ~2024) ; lecture par `backend.table_rows` (~1715). Si le geste ne gère pas `table` + `overlay` + `work_crs` ensemble, le dire avant de le modifier.
- `qgis-plugin/cassini_qgis/summary.py` (~951, `_SENTENCES`), `texts.py` (clés `sum_distance_matrix*`, FR et EN).
- Tests : `qgis-plugin/tests/test_analysis.py` (classe sur le modèle de `NearestTest` l.951), `test_summary.py`, `test_result_conformance.py`, `fixtures/resultats-extension.json`, `table_annulable.json`, sonde `probe_distance_matrix.py` (nouvelle, d'après `probe_nearest.py`).
- `training/src/families.rs` (~3620 `plus_proche_voisin`, ~197 enregistrement) ; retirer `distance_matrix` de `ABSENT_FUNCTIONS` (l.10492, garder `travel_time`) ; `ai/tests/echelle_utm_reel/phrases.json` (~719, EU36 : `missing` → `["travel_time"]`), `ai/tests/echelle_utm_reel.rs` l.20, `docs/entrainement-du-modele.md` l.2613 et 2631.
- `ai/tests/par_commande/phrases.json` (phrase DM1+) et `training/src/barres.rs` (compteur `par_commande`, 102 relevé du nombre ajouté).
- `README.md`, `qgis-plugin/README.md`, `catalogue-des-commandes.md` (statut `faite`, fiche l.2029-2055).

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_distance_matrix.rs` -- commande, schéma, exemples, tests -- registre.
- [ ] `core/src/journal/tests.rs` -- sortie journalisée, rejeu identique, `work_epsg` -- reproductibilité.
- [ ] `qgis-plugin/cassini_qgis/{analysis,summary,texts}.py` -- geste, plafond de paires, refus des non-points, résumé FR/EN -- exécution QGIS.
- [ ] `qgis-plugin/tests/*`, `fixtures/resultats-extension.json` -- tests de la matrice ci-dessus, AD-25, sonde sous QGIS 3.44.
- [ ] `training/src/families.rs`, `ai/tests/**`, `docs/entrainement-du-modele.md` -- famille, refus corrigé, phrase DM1+.
- [ ] README (deux), catalogue -- documentation.

**Acceptance Criteria:**
- Given deux couches de points synthétiques, when la commande tourne, then les distances de la table égalent celles de `QgsDistanceArea` à la tolérance de `probe_nearest.py`.
- Given « donne-moi les distances entre chaque école et chaque gare », when l'agent compose, then le plan est valide et contient `analysis.distance_matrix`.
- Given tout le catalogue, when les tests de l'extension et du noyau tournent, then ils sont verts et `cargo clippy` sans avertissement.

## Design Notes

Décisions du plan (le propriétaire les confirme à l'approbation) : `label_field` et `target_label_field` deviennent **obligatoires** et se choisissent dans une liste (`qgis:distancematrix` exige un champ d'identifiant, règle « choisir, pas saisir ») ; `neighbors` est un entier ≥ 1, facultatif ; `rows` : une ligne par paire, libellé « départ → cible », valeur `distance` en `m`, `truncated` si > 100 ; sortie en table seulement. Ces trois points modifient la fiche figée du catalogue (paramètres `neighbors`, champs d'identifiant obligatoires, id de l'algorithme) : un **changement de cap léger**, comme pour `analysis.nearest`.

## Verification

**Commands:**
- `cargo test -p cassini-core` et `cargo test -p cassini-training` (CARGO_TARGET_DIR=C:/ct) -- expected: vert
- `cargo clippy --all-targets` -- expected: aucun avertissement
- Python de QGIS 3.44 : `qgis-plugin/tests/test_analysis.py`, sonde `probe_distance_matrix.py` -- expected: verts, VERDICT : OK

**Manual checks (if no CLI):**
- Ouvrir la table CSV de la sonde : colonnes départ, cible, distance en mètres.
