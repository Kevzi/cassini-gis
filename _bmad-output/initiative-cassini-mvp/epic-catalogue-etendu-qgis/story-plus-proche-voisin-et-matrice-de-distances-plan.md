---
title: 'Plus proche voisin (analysis.nearest)'
type: 'feature'
ticket: '10'
created: '2026-11-18'
status: 'ready-for-dev'
baseline_revision: 'a0a3abf3c8e522728af66c68fe1001bcb156badc'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-distance-en-metres-sur-couche-en-degres/change-distance-en-metres-sur-couche-en-degres.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le catalogue annonce `analysis.nearest` (jointure au plus proche voisin) mais la commande n'existe pas. Sa sœur `analysis.distance_matrix` exige la sortie `table` (entrées 10.1 et 10.3, non construites).

**Approach:** ajouter `analysis.nearest` seule, sur le modèle d'`analysis.buffer` : déclaration dans le noyau, ligne de la table fermée de l'extension (`native:joinbynearest`), distance calculée dans le système `work_epsg` du noyau. `analysis.distance_matrix` est reportée (décision du propriétaire, 2026-11-18) jusqu'à 10.1 et 10.3.

## Boundaries & Constraints

**Always:** fiche du catalogue reprise telle quelle (params `layer_id`, `target_layer_id`, `max_distance` facultative, `out`; résultat `crs`, `distance_field`, `feature_count`, `layer_id`, `name`, `unit`, `unmatched_count`), seul son statut passe à `faite` ; `work_epsg` déclaré par `.work_crs_param` comme `analysis.buffer` ; distance du champ en mètres même sur couche en degrés ; aucun nom d'algorithme ni de champ QGIS dans le noyau (AD-22) ; `layer_id` ≠ `target_layer_id`, refusé par le noyau ; textes en français et en anglais ; un test par ligne de la matrice.

**Never:** `analysis.distance_matrix`, gabarit `SummaryRow`, sortie `table` ou chaîne fixe (10.1, 10.3) ; modifier le catalogue au-delà du statut ; ré-entraîner le modèle ; changer `analysis.buffer` ; toucher à `.claude/worktrees` ni à `_bmad-output/party-mode/`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Couche en mètres | points et cibles en Lambert-93 | chaque entité reçoit les attributs de sa cible la plus proche et la distance en m | — |
| Couche en degrés | `layer_id` en WGS84 | le noyau remplit `work_epsg`, la chaîne reprojette, distance en m | — |
| Limite | `max_distance` 500 m | entités sans cible à moins de 500 m gardées sans voisin ; `unmatched_count` les compte | — |
| Aucune limite | `max_distance` absente | `unmatched_count` = 0 | — |
| Même couche | `layer_id` = `target_layer_id` | refus | erreur du noyau, rien exécuté |
| Cible vide | couche cible sans entité | toutes les entités sans voisin | `unmatched_count` = `feature_count` |
| Annulation | arrêt demandé | chaîne interrompue, sortie retirée | comme `analysis.buffer` |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_buffer.rs` -- modèle à suivre : `Delegated`, `CommandSpec`, `work_crs_param`, `Distance`/`DistanceUnit` (l. 13-45), `inventory::submit!`. Rien d'autre à enregistrer (`mod.rs` inclut `build.rs`).
- `core/src/contract.rs` -- contrôle `.tests(...)` et `work_crs_param` ; `core/src/journal/mod.rs` `fill_work_crs` (l. 427-460) remplit seul `work_epsg`.
- `core/tests/catalogue.rs` -- confronte la commande à sa fiche ; `catalogue-des-commandes.md` (fiche l. 1943-1969, tableau l. 375).
- `qgis-plugin/cassini_qgis/analysis.py` -- `ALGORITHMS` (l. ~327-400), `translate_buffer` (l. 53), `_distance_in_meters` (l. 42), clé `work_crs`, `summary`.
- `qgis-plugin/cassini_qgis/texts.py`, `summary.py` (`_SENTENCES`, l. ~897) -- textes `sum_analysis_nearest…`.
- `qgis-plugin/tests/test_analysis.py` (`BufferTest`, fixtures synthétiques), `test_summary.py`, `test_delegated.py` (ids de l'extension = fiches `faite`).
- `training/src/families.rs`, `docs/entrainement-du-modele.md`, `ai/tests/par_commande/phrases.json`, `README.md` (l. 116-120) -- une famille de phrases, une ligne de doc.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_nearest.rs` -- commande déléguée, `max_distance: Option<Distance>`, `work_epsg`, refus de la même couche, exemple et noms de tests -- suit `analysis_buffer.rs`
- [ ] `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- statuts `analysis.nearest` en `faite` (fiche et tableau) -- seul changement permis
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- ligne `analysis.nearest` : `native:joinbynearest`, un voisin, limite en mètres, champ `distance`, sans entité écartée, comptage des non appariées
- [ ] `qgis-plugin/cassini_qgis/texts.py`, `summary.py` -- phrases de résumé fr et en
- [ ] `qgis-plugin/tests/test_analysis.py`, `test_summary.py`, `core/src/journal/tests.rs` -- une épreuve par ligne de la matrice ; `work_epsg` rempli pour nearest
- [ ] `training/src/families.rs`, `docs/entrainement-du-modele.md`, `ai/tests/par_commande/phrases.json`, `README.md` -- famille d'exemples, barre par commande, ligne de table
- [ ] `qgis-plugin/tests/probe_nearest.py` -- sonde QGIS réelle sur fixture synthétique (`native:joinbynearest`, couche en degrés)

**Acceptance Criteria:**
- Given une couche et une couche cible, when l'agent compose un plan avec `analysis.nearest`, then le plan est valide pour le noyau.
- Given `cargo test --workspace` et les tests Python, when on les lance, then ils réussissent, y compris `catalogue.rs` et `test_delegated.py`.

## Implementation Notes

## Plan Change Log

## Review Triage Log

## Design Notes

`native:joinbynearest` mesure dans le système de la couche d'entrée : la chaîne ouvre par la reprojection vers `work_epsg` (comme le buffer), donc le champ `distance` est en mètres. À vérifier par la sonde : la couche cible est-elle transformée par l'algorithme ? Sinon, la reprojeter aussi.

## Verification

**Commands:**
- `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` -- expected: vert (première compilation longue : `C:\ct` a été vidé)
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert

**Manual checks (if no CLI):**
- Sonde `probe_nearest.py` sous QGIS : voisins et distances en mètres sur une couche en degrés.
