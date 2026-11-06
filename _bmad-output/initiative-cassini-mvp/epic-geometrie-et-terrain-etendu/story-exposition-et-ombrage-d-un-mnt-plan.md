---
title: "Exposition et ombrage d'un MNT"
type: 'feature'
ticket: '6'
created: '2026-11-06'
status: 'built'
baseline_revision: '153b17908bd349a0650ff106496dc22a90914306'
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

**Problem:** le noyau calcule la pente d'un MNT mais ni son exposition (vers où descend chaque cellule) ni son ombrage (le relief vu sous un soleil rasant), deux lectures courantes du terrain que le catalogue a figées (`terrain.aspect`, `terrain.hillshade`, §6, planifiées par l'entrée 6).

**Approach:** ajouter les deux commandes au noyau exactement comme leurs fiches les décrivent, sur la porte raster unique (`Dem`, AD-20) et `RasterPath`, par le voisinage 3 × 3 de Horn déjà employé par `terrain.slope` et mis en commun ; passer les deux fiches en `faite` ; consigner la comparaison à `native:aspect` et `native:hillshade` de QGIS dans la note de 4.7.

## Boundaries & Constraints

**Always:** paramètres, résultats et défauts exactement ceux des fiches (`sun_azimuth` 315°, `sun_altitude` 45°, `z_factor` 1) ; angles en `Angle` (deg ou gon), rapportés au **nord du quadrillage** du MNT, exposition rendue en degrés dans [0, 360), sens horaire (AD-31) ; cellule plate (gradient nul) en NoData pour l'exposition, comptée dans `flat_cells` et dans `nodata_cells` ; bord et voisines d'un trou en NoData, comme la pente ; sortie GeoTIFF Float32, sans-donnée -9999, même grille et même EPSG que le MNT, aucune reprojection ; fonctions trigonométriques de `libm` (mêmes octets partout, AD-11) ; `sun_altitude` hors [0, 90°] ou `z_factor` ≤ 0 (ou non fini) → `terrain.invalid_parameter` qui nomme le paramètre, rien d'écrit ; tests existants de `terrain.slope` inchangés et verts.

**Never:** aucune dépendance nouvelle ; aucun changement de l'entraînement ni du générateur (entrées 12 et 16) ; pas de paramètre de méthode (Zevenbergen, multidirectionnel) ; pas de calcul des bords par valeur inventée ; pas de geste QGIS ni de ligne dans la table fermée (`terrain.*` jamais délégué) ; aucun code de GDAL, QGIS, GRASS ou Whitebox repris.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan incliné | `terrain.aspect {dem: mnt-plan-incline-2154.tif}` (descend vers l'est) | intérieur uniforme 90°, bord NoData, `flat_cells` 0, `nodata_cells` 136 | — |
| Colline | `terrain.aspect` sur `mnt-colline-2154.tif` | au nord du sommet ≈ 0°, à l'est ≈ 90°, au sud ≈ 180°, à l'ouest ≈ 270° ; symétrie par miroir | — |
| Plat | MNT constant 5 × 5 | intérieur NoData, `flat_cells` 9, `nodata_cells` 25 | — |
| Ombrage par défaut | `terrain.hillshade {dem: plan incliné}` | intérieur uniforme = 255 · max(0, n·s), valeur calculée à la main dans le test | — |
| Soleil en face / opposé | plan incliné, `sun_azimuth` 90° puis 270° | 90° plus clair que 270° ; MNT plat → 255 · sin(hauteur) | — |
| Grades | `sun_azimuth {value: 100, unit: gon}` | même sortie, octet pour octet, que 90° | — |
| Facteur Z | `z_factor` 2 sur le plan incliné | valeur du plan à pente doublée | — |
| Hauteur invalide | `sun_altitude` 95° ou -1° | rien d'écrit | `terrain.invalid_parameter` nommant `sun_altitude` |
| Facteur Z invalide | `z_factor` 0 ou -1 | rien d'écrit | `terrain.invalid_parameter` nommant `z_factor` |
| MNT refusé | fichier illisible ou non projeté | rien d'écrit | erreur `dem.*` du module, telle quelle |
| Rejeu | deux exécutions identiques | mêmes octets | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/terrain_slope.rs` l. 80-145 `horn_slope` -- fenêtre de Horn en ligne (`dx`, `dy` sur `cell_size_x/y`, lignes du nord au sud, donc `dy` > 0 quand le sol monte vers le sud) ; copie champ par champ du `Dem` de sortie (l. 125). Extraire `horn_gradient(dem, col, row) -> Option<(f64, f64)>` et `same_grid_as(dem, values, nodata) -> Dem` dans un module partagé, `horn_slope` les appelant sans changer un octet de sa sortie.
- `core/src/horn.rs` (nouveau, déclaré dans `core/src/lib.rs`) -- ces deux fonctions, sans dépendance au registre ; doc de provenance (Horn 1981) reprise de `terrain_slope.rs`.
- `core/src/commands/terrain_aspect.rs`, `terrain_hillshade.rs` (nouveaux) -- calqués sur `terrain_slope.rs` : `Params` `deny_unknown_fields`, `dem: RasterPath`, `out` par défaut ; `CommandSpec` `runs_in(Core)`, `cancellable(false)`, `path_params(&["dem"])`, `output_params(&["out"])`, `output_ext(".tif")`, exemples, `.tests(&[..])` qui nomme les tests réels ; `inventory::submit!`. Les modules de `commands/` sont découverts par `build.rs` (rien à déclarer).
- `core/src/angle.rs` -- `Angle::degrees()` ; doc du type dit « nord vrai » : ajouter dans le doc des champs des deux commandes « nord du quadrillage du MNT ».
- `core/src/commands/mod.rs` -- `From<DemError> for CommandFailure` à réutiliser.
- `_bmad-output/.../spec-cassini-mvp/catalogue-des-commandes.md` l. 263-264 (tableau, `planifiée` → `faite`), l. 887-940 (fiches `"status": "faite"`, au besoin déplacées dans la section des commandes faites) ; `core/tests/catalogue.rs` confronte, rien à y changer.
- `qgis-plugin/cassini_qgis/raster_output.py` -- ouverture dictée générique de toute sortie `.tif` du noyau : ne pas modifier.
- `core/examples/comparer_terrain.rs` l. 331-360 `compare_slope`, l. 573-610 rapport -- généraliser en comparaison de grilles avec une fonction d'écart (circulaire pour l'exposition : min(|a-b|, 360-|a-b|)) ; ajouter les deux sections.
- `tests/comparaison-terrain/comparer.ps1` l. 82, 106 -- ajouter `terrain.aspect`, `terrain.hillshade` (défauts) et `native:aspect`, `native:hillshade` (`Z_FACTOR=1 AZIMUTH=315 V_ANGLE=45`), normalisés comme la pente ; `LISEZMOI.md` mis à jour.
- `docs/comparaison-terrain-qgis.md` -- sections « Exposition » et « Ombrage », révision mesurée, écarts expliqués (bords, cellules plates, arrondi Float32).

## Tasks & Acceptance

**Execution:**
- [x] `core/src/horn.rs`, `core/src/lib.rs`, `core/src/commands/terrain_slope.rs` -- mise en commun de la fenêtre de Horn ; tests de la pente inchangés.
- [x] `core/src/commands/terrain_aspect.rs` -- commande et tests de la matrice (plan, colline, plat, trou, MNT refusé, octets identiques, contrat accepté).
- [x] `core/src/commands/terrain_hillshade.rs` -- commande et tests de la matrice (valeur analytique, azimuts, grades, facteur Z, refus nommés, octets identiques, contrat accepté).
- [x] `catalogue-des-commandes.md` -- deux fiches en `faite`, dans le même commit.
- [x] `core/examples/comparer_terrain.rs`, `tests/comparaison-terrain/comparer.ps1`, `tests/comparaison-terrain/LISEZMOI.md` -- protocole étendu, test unitaire de l'écart circulaire.
- [x] `docs/comparaison-terrain-qgis.md` -- mesure jouée sur le MNT de Villeneuve-le-Roi sous Windows (QGIS 3.44.15), chiffres consignés.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when `cargo test --workspace`, then tout passe, dont `catalogue` avec les deux fiches `faite` et les tests de `terrain.slope` inchangés.
- Given le registre, when on liste les commandes servies au client MCP et au modèle, then `terrain.aspect` et `terrain.hillshade` y figurent, avec un `dem` montré comme chaîne.
- Given le MNT de Villeneuve-le-Roi, when on joue le protocole de comparaison, then la note donne pour chaque commande l'écart moyen et maximal à QGIS sur les cellules définies des deux côtés, et chaque désaccord de masque est classé (bord, trou, plat, ailleurs) ; un écart inexpliqué hors de ces classes ouvre un ticket de bogue au lieu d'être consigné comme normal.

## Implementation Notes

- `terrain.hillshade` contrôle `sun_altitude` et `z_factor` avant d'ouvrir le MNT : un refus de
  paramètre l'emporte sur un MNT illisible.
- Tests d'octets figés ajoutés aux deux commandes (empreintes relevées sous Windows par
  `cassini.exe`, vérifiées sous Linux par `cargo test`).
- `ai/src/plan.rs` : le test du long prompt vérifie aussi `dem` en chaîne pour les deux
  commandes nouvelles.
- Comparaison : les lieux d'un désaccord de masque sont maintenant quatre (bord, trou, cellule
  plate, ailleurs) pour les trois grilles ; un désaccord « ailleurs » est signalé en gras.
  Mesure du 2026-11-06 : aucun désaccord ailleurs, aucun ticket de bogue.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 0, medium 0, low 5, false 0, maybe-false 0 ; 5 patchs.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| `comparer_terrain.rs` : un désaccord de masque sur cellule plate est excusé pour les trois grilles (`unexplained` ne compte que l'indice 3), alors que seule l'exposition laisse le plat sans donnée | low | patch | `place` l. 330, `unexplained` l. 408 ; la pente et l'ombrage définissent le plat : une régression y passerait sans ligne en gras |
| Note, Ombrage : l'écart maximal 4,58e-5 attribué à l'arrondi Float32, alors qu'il vaut 3 pas Float32 | low | patch | pas 2⁻¹⁶ ≈ 1,53e-5 entre 128 et 256 ; un seul arrondi de chaque côté donne au plus 1 pas |
| Note, « En bref » : « mêmes écarts de masque que la pente » pour l'exposition | low | patch | Exposition 320 / 2 476 contre 346 / 2 662 pour la pente |
| Note, tableau Ombrage : ligne « Sans donnée des deux côtés » absente | low | patch | présente dans le tableau Exposition et dans la sortie du comparateur |
| `terrain_hillshade.rs` : le test du MNT refusé ne couvre pas le fichier illisible de la matrice | low | patch | seuls le fichier absent et l'EPSG:3857 y sont ; le test de l'exposition couvre `dem.unreadable` |

## Design Notes

Repère : x vers l'est, y vers le nord ; avec la fenêtre de Horn, `p = dz/dx_est = dx`, `q = dz/dy_nord = -dy`. Exposition = direction de la descente, `atan2(-p, -q)` en (est, nord) : `libm::atan2(-dx, dy)` en degrés, ramené à [0, 360) ; `dx == 0 && dy == 0` → plat. Ombrage : normale `(-z·p, -z·q, 1)` normée, soleil `(sin A cos H, cos A cos H, sin H)` ; valeur `255 · max(0, n·s)`, sans arrondi entier (sortie Float32). Un plan qui descend vers l'est (`dx < 0`) donne `atan2(+, 0) = 90°`.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement (sous Windows : `CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH` posé)
- `cargo test --workspace` -- expected: tout vert
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `cargo run --release -p cassini-core --example comparer_terrain -- comparer <dossier>` -- expected: sections Exposition et Ombrage produites
