---
title: 'Champ de vision cumulé depuis plusieurs points'
type: 'feature'
ticket: '7'
created: '2026-11-06'
status: done
baseline_revision: 'ae1eeaf565b97daa4e523e9fa96ad4a5320d503d'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le noyau sait ce qu'un seul observateur voit (`terrain.viewshed`), mais pas combien d'observateurs voient chaque cellule ; le catalogue a figé `terrain.viewshed_cumulative` (fiche §6, planifiée par l'entrée 7).

**Approach:** ajouter la commande exactement comme sa fiche la décrit : le calcul de `terrain.viewshed` répété pour chaque observateur, mêmes hauteurs, rayon, courbure et réfraction, et les grilles 0/1 additionnées sur une grille commune ; passer la fiche en `faite`.

## Boundaries & Constraints

**Always:** paramètres, défauts et résultats exactement ceux de la fiche ; `observers` borné de 1 à 32 par le schéma (`minItems`/`maxItems`, AD-8) **et** par la désérialisation, hors bornes → `registry.invalid_params` dont le message nomme `observers`, rien d'écrit ; chaque observateur contrôlé comme dans `terrain.viewshed` (`terrain.crs_mismatch`, `terrain.outside_dem`, `terrain.no_elevation`), le message nommant son rang (« observateur n° 2 ») ; rayon trop grand → `terrain.viewshed_too_large`, rien d'écrit ; tous les observateurs contrôlés avant d'écrire quoi que ce soit ; valeur d'une cellule = nombre d'observateurs pour lesquels `terrain.viewshed` rend 1 ; -9999 si aucun observateur ne la couvre (hors de tous les rayons, ou trou) ; grille de sortie alignée sur le MNT, réduite à l'enveloppe des fenêtres individuelles, même EPSG, GeoTIFF Float32 ; un observateur répété compte deux fois ; sortie déterministe ; tests et octets de `terrain.viewshed` inchangés.

**Never:** aucune dépendance nouvelle ; pas de parallélisme (ordre et octets figés) ; pas de paramètre par observateur (hauteurs, rayons distincts) ; aucun changement de l'entraînement ni du générateur (entrées 12 et 16) ; pas de geste QGIS ni de ligne dans la table fermée ; aucun code de GDAL, QGIS, GRASS ou Whitebox repris.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Deux observateurs de part et d'autre | colline, un observateur à l'ouest et un à l'est du sommet | comptes 0, 1 et 2 tous présents ; sommet 2 ; chaque versant éloigné 1 ; `observer_count` 2, `max_count` 2 | — |
| Accord avec le simple | même entrée | chaque cellule = somme des deux `terrain.viewshed` ; -9999 là où les deux le sont | — |
| Un seul observateur | liste d'un point | mêmes cellules visibles et mêmes valeurs que `terrain.viewshed` | — |
| Rayons disjoints | deux observateurs à petit rayon, éloignés | enveloppe des deux fenêtres ; entre elles -9999 | — |
| Liste vide / 33 points | `observers: []` ou 33 points | rien d'écrit | `registry.invalid_params` nommant `observers` |
| Observateur fautif | 2ᵉ point hors du MNT, ou autre EPSG, ou sur un trou | rien d'écrit | `terrain.outside_dem` / `crs_mismatch` / `no_elevation`, rang 2 nommé |
| Rayon trop grand | rayon > 500 cellules | rien d'écrit | `terrain.viewshed_too_large` |
| Rejeu | deux exécutions identiques | mêmes octets | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/terrain_viewshed.rs` l. 140-240 `viewshed(dem, params)` -- contrôles puis grille rognée ; le nom `"observateur"` est en dur dans `check_crs`, `check_extent`, `endpoint_elevation` (l. 147-149). Extraire `pub(crate) fn viewshed_of(dem, params, observer: &Point, name: &str)` que `viewshed` appelle avec `"observateur"` (aucun octet ni message changé). Réutiliser `TerrainViewshedParams` (cloner avec l'observateur remplacé, ou passer l'observateur à part), `VIEWSHED_NODATA`, `given_radius`, `DEFAULT_RADIUS`.
- Recalage d'une fenêtre dans la grille commune : décalage entier `((sub.origin_x - dem.origin_x) / cell_size_x).round()`, comme le helper de test `at` (l. ~370).
- `core/src/commands/terrain_viewshed_cumulative.rs` (nouveau, découvert par `build.rs`) -- calqué sur `terrain_viewshed.rs` : `Params` `deny_unknown_fields`, `dem: RasterPath`, `observers: Vec<Point>` avec `deserialize_with` borné + `#[schemars(length(min = 1, max = 32))]` (modèle : `forms` dans `coord_convert.rs` l. 41-65), `pub const MAX_OBSERVERS: usize = 32` ; mêmes défauts que le simple (`default_observer_height`, etc., de `terrain_line_of_sight.rs`) ; `CommandSpec` `runs_in(Core)`, `cancellable(false)`, `path_params(&["dem"])`, `output_params(&["out"])`, `output_ext(".tif")`, descriptions FR/EN qui disent la borne de 32 et de 500 cellules, exemples sur `mnt-colline-2154.tif`, `.tests(&[..])` nommant les tests réels ; `inventory::submit!`.
- Fixture : `fixtures/mnt-colline-2154.tif`, 50 × 50 cellules de 10 m, coin (652000, 6850500), sommet (652250, 6850250). Si aucune position d'observateurs ne fait apparaître de 0 sur la colline, écrire dans le test un petit MNT synthétique par `Dem::write_geotiff` (deux crêtes séparées d'un creux).
- `core/src/contract.rs` l. 299-315 -- refuse un tableau de géométries sans `maxItems` : le schéma doit passer, rien à changer.
- `ai/src/plan.rs` l. ~1029 -- test du long prompt qui vérifie `dem` en chaîne : y ajouter `terrain.viewshed_cumulative`.
- `_bmad-output/.../spec-cassini-mvp/catalogue-des-commandes.md` l. 265 (tableau `planifiée` → `faite`), l. 946-986 (fiche `"status": "faite"`, déplacée au besoin avec les faites ; phrase sur le refus au-delà de 32) ; `core/tests/catalogue.rs` confronte, rien à y changer.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/terrain_viewshed.rs` -- extraire `viewshed_of` ; tests existants inchangés et verts.
- [x] `core/src/commands/terrain_viewshed_cumulative.rs` -- commande et tests de la matrice (comptes 0/1/2, accord cellule par cellule avec la somme des simples, un seul observateur, rayons disjoints, bornes de la liste, observateur fautif par rang, rayon trop grand, octets identiques, contrat accepté avec `maxItems` 32).
- [x] `ai/src/plan.rs` -- la nouvelle commande dans le test du long prompt.
- [x] `catalogue-des-commandes.md` -- fiche en `faite`, dans le même commit.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when `cargo test --workspace`, then tout passe, dont `catalogue` avec la fiche `faite` et les tests de `terrain.viewshed` inchangés.
- Given le registre, when on liste les commandes servies au client MCP et au modèle, then `terrain.viewshed_cumulative` y figure, avec `dem` montré comme chaîne et `observers` borné à 32.

## Implementation Notes

- `terrain_viewshed.rs` : `viewshed` appelle `viewshed_of(dem, params, &params.observer, "observateur")` ; les trois contrôles de l'observateur sont regroupés dans `check_observer` (même ordre, mêmes messages). `given_radius` passe en `pub(crate)`.
- La commande cumulée contrôle d'abord tous les observateurs (`check_observer`, rang nommé), puis calcule toutes les fenêtres : un observateur fautif l'emporte sur un rayon trop grand, comme dans `terrain.viewshed`.
- La colline du jeu d'essai est bombée : son sommet est caché à tout observateur plus bas (aucun compte 2). Le test des comptes 0/1/2 utilise donc un MNT synthétique (pic à flancs concaves, murs, creux derrière les murs), comme le prévoit la Code Map ; les autres tests restent sur `mnt-colline-2154.tif`.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 0, medium 1, low 2, false 1, maybe-false 0 ; 2 patchs, 1 renvoi au propriétaire, 1 rejet.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| `cargo fmt --check` échouerait (signature de `given_radius` à 106 caractères, tuple de test à 105) | false | rejet | `cargo fmt --check` lancé sous Linux, sortie 0, aucun écart |
| Pire cas 32 × ~5 s, non annulable, et 32 fenêtres gardées en mémoire (~128 Mo) | medium | patch (mémoire) + renvoi au propriétaire (durée, annulation) | `terrain_viewshed.rs` l. 21-23 donne 5 s au pire pour un observateur ; `.cancellable(false)` ; borne 32 et 500 cellules figées par la fiche, donc plafond de travail total ou annulation à décider par le propriétaire |
| `length(max = 32)` en dur, à côté de `MAX_OBSERVERS` | low | patch | deux sources de vérité ; seul le test de contrat les relie |
| Le test 0/1/2 n'est pas sur la colline nommée par la matrice (« sommet 2 ») | low | rejet | le corriger reviendrait à modifier le plan ; la colline bombée ne peut donner 2 (Implementation Notes), la Code Map prévoyait le MNT synthétique, le ticket dit « colline synthétique » |

## Design Notes

Grille commune : enveloppe (min des colonnes/lignes de départ, max des fins) des fenêtres rendues par `viewshed_of`, chaque fenêtre étant déjà alignée sur le MNT. Pour chaque cellule de chaque fenêtre : 1 → +1, 0 → la cellule devient définie (0 si encore sans donnée), -9999 → rien. `visible_cells` = cellules ≥ 1 ; `nodata_cells` = cellules restées à -9999 ; `max_count` = maximum (0 si rien n'est vu, mais la cellule de l'observateur l'est toujours, donc ≥ 1). Calculer toutes les fenêtres avant d'écrire : la première erreur arrête tout.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `cargo test --workspace` -- expected: tout vert
