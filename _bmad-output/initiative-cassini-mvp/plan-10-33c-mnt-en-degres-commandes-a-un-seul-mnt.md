---
title: 'MNT en degrés : champ de vision, ligne de vue et points hauts'
type: 'feature'
ticket: ''
created: '2026-11-21'
status: 'built'
baseline_revision: '71e968e0a6ed43eee5634c5f97d579f0f8e13dc2'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, edge-cases]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/plan-10-33b-geste-qgis-reprojection-du-mnt.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/story-mnt-en-degres-calcul-dans-un-systeme-en-metres-choisi-par-le-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `terrain.viewshed`, `viewshed_cumulative`, `line_of_sight` et `peaks` refusent encore un MNT en degrés (`dem.not_projected`), et un point WGS84 est refusé (`terrain.crs_mismatch`) : la phrase de démo sur un MNT en degrés échoue.

**Approach:** Appliquer à ces quatre commandes le mécanisme de 10.33 : `work_crs_param`, `check_dem_system`, reprojection dictée par le noyau avant l'étape, rééchantillonnage bilinéaire. Le noyau convertit en plus, à l'exécution, juste après la reprojection dictée, les points littéraux dans un autre système vers `work_epsg` par une étape `data.transform_point`, journalisée et rejouée telle quelle.

## Boundaries & Constraints

**Always:** Les sorties (champ de vision, points hauts, obstruction) sont dans `work_epsg`, en mètres, sans reprojection en retour. Un MNT métrique et un point déjà dans le bon système : résultat inchangé, aucune étape ajoutée. Rejeu identique, y compris après changement du système du projet. Mécanisme de `ef13d4b` et `71e968e` réutilisé tel quel ; noyau sans GDAL ; fiches du catalogue à jour.

**Never:** Pas `terrain.cost_distance`, `least_cost_path` ni `raster.weighted_overlay` (ticket B en `deferred-work.md`). Pas de reprojection en retour des résultats. Aucun choix laissé au modèle.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Champ de vision, MNT en degrés, observateur WGS84 | `terrain.viewshed` | reprojection dictée, observateur converti vers `work_epsg`, résultat en mètres | — |
| Observateur déjà dans `work_epsg` | point en 2154 | aucune étape de conversion ajoutée | — |
| MNT métrique | EPSG:2154 | inchangé | — |
| Points hauts | `terrain.peaks` sur MNT en degrés | points en `work_epsg` | — |
| Aucun système métrique | MNT et projet en degrés, étendue hors fuseau | refus `dem.not_projected` | message qui dit pourquoi |
| Étape refusée après dictée | observateur hors du MNT | refus propre, journal cohérent | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/terrain_viewshed.rs` l.60-62, 147, 271, 305 -- `dem`, `observer`, `check_observer`, `path_params`, `Dem::open`
- `core/src/commands/terrain_viewshed_cumulative.rs` l.58-62, 265 -- `dem`, `observers`
- `core/src/commands/terrain_line_of_sight.rs` l.122-126, 206, 498 -- `observer`, `target`, `check_crs`
- `core/src/commands/terrain_peaks.rs` l.89, 339 -- `dem`, sortie GeoJSON en `dem.epsg`
- Modèle à imiter : `core/src/commands/terrain_slope.rs` l.128-155 (`work_crs_param`, `check_dem_system`), `terrain_aspect.rs`
- `core/src/journal/mod.rs` l.548-620 `dictate_reprojection`, `resampling_for` (`raster_reproject_work.rs:61`) -- ajouter bilinéaire pour les quatre commandes et la conversion des points après la reprojection
- `core/src/plan_completion.rs` `sites_to_convert` -- à réutiliser pour repérer les points à convertir ; `core/src/serve/plans.rs` ~l.274 `dem::epsg_of` : ne doit plus bloquer la complétion pour un MNT en degrés
- `core/src/point.rs` -- `Point{x,y,epsg}`
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- fiches des quatre commandes et de `raster.reproject_work` (liste des commandes)
- `qgis-plugin/tests/probe_dem_degrees.py` -- à étendre ; ne pas toucher à `analysis.py` sauf nécessité

## Tasks & Acceptance

**Execution:**
- [ ] quatre fichiers `core/src/commands/terrain_*.rs` -- `work_epsg`, `work_crs_param`, `check_dem_system`, tests de déclaration -- brancher le mécanisme
- [ ] `core/src/journal/mod.rs`, `raster_reproject_work.rs` -- bilinéaire pour ces commandes ; conversion des points littéraux vers `work_epsg` après la reprojection -- comportement
- [ ] `core/src/plan_completion.rs`, `core/src/serve/plans.rs` -- ne pas refuser ni convertir à tort au moment de la proposition quand le MNT est en degrés -- cohérence
- [ ] `core/src/journal/tests.rs` -- ordre dicté, conversion du point, MNT métrique inchangé, rejeu, refus propre -- preuves de la matrice
- [ ] catalogue -- quatre fiches, liste de `raster.reproject_work`
- [ ] `qgis-plugin/tests/probe_dem_degrees.py` -- cas : champ de vision avec observateur WGS84, ligne de vue, points hauts ; comparaison avec la reprojection faite à la main

**Acceptance Criteria:**
- Given un MNT en EPSG:4326 et un observateur WGS84, when `terrain.viewshed` est planifié, then le noyau dicte la reprojection puis la conversion du point, et le résultat égale celui du MNT reprojeté à la main avec l'observateur converti.
- Given un MNT métrique, then les quatre commandes donnent un résultat inchangé et aucune étape n'est ajoutée.
- Given `cargo test -p cassini-core`, `python -I -m unittest discover -s qgis-plugin/tests` et la sonde, then tout est vert et `VERDICT OK`.

## Implementation Notes

Quatre commandes branchées (`work_epsg`, `work_crs_param`, `check_dem_system`). `dictate_reprojection` dicte, après la reprojection, une étape `data.transform_point` par point littéral hors `work_epsg` (via `plan_completion::foreign_points`), sans quoi rien n'est dicté (ancien refus). `current_plan` reconnaît la conversion dictée. Rééchantillonnage : `resampling_for` donnait déjà bilinéaire. `plans.rs` : aucun changement de logique (`epsg_of` refuse déjà le MNT en degrés, donc pas de conversion à la proposition). Sonde étendue (viewshed, cumulé, ligne de vue, points hauts) : VERDICT OK.

## Plan Change Log

## Review Triage Log

Passe 1 (quick + cas limites) : corrigés — message de conversion aux espaces parasites, système du point converti non vérifié, tests manquants (échec de conversion, geste absent, aucun système métrique, point hors MNT, MNT métrique avec point WGS84, rééchantillonnage des quatre commandes), textes obsolètes lus par le modèle. Consignés en `deferred-work.md` : conversions avant la reprojection, contrôle d'ordre longitude/latitude.

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: vert, `core/tests/catalogue.rs` compris
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_dem_degrees.py` -- expected: `VERDICT OK` (reconstruire `C:\ct-5-57g` avec `--features gpu` si le noyau change)
