---
title: 'MNT en degrés : calcul dans un système en mètres choisi par le noyau'
type: 'feature'
ticket: '33'
created: '2026-11-21'
status: 'built'
baseline_revision: '2cca560aeac5a30f5233652d9bfbb05d38c9320a'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, edge-cases]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-distance-en-metres-sur-couche-en-degres/change-distance-en-metres-sur-couche-en-degres.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/story-isochrone-sur-un-reseau-en-degres-calcul-dans-un-systeme-en-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `dem.rs` refuse tout MNT en degrés (`dem.not_projected`) : pente, champ de vision et les commandes voisines échouent sur un MNT en EPSG:4326, cas courant. 9.16 et 10.32 ont réglé cela pour les couches vectorielles et les réseaux, pas pour les MNT.

**Approach (première moitié du ticket, noyau seul ; le geste QGIS est reporté, voir décisions) :** Le noyau choisit `work_epsg` pour un MNT (MNT en mètres, sinon projet en mètres, sinon fuseau UTM de l'étendue, sinon refus `dem.not_projected` qui dit pourquoi), le journalise et le relit au rejeu. Le noyau sait aussi nommer la sortie de la reprojection (AD-10) et dicter l'étape, mais l'exécution par l'extension (`gdal:warpreproject`) est la seconde moitié. `dem.rs` reste sans GDAL (AD-1, AD-20) et aucune étape n'est ajoutée au plan (AD-23).

## Boundaries & Constraints

**Always:** Reprojeter seulement un MNT en degrés, jamais par identité (amendement RA06). MNT métrique : résultat inchangé. Rejeu identique dans la même session et dans une session neuve, y compris après un changement du système du projet. Ligne ancienne sans `work_epsg` complétée par le système du MNT. Fiches du catalogue à jour, `core/tests/catalogue.rs` vert. Aucun choix laissé au modèle (AD-5, AD-24).

**Never:** Pas de GDAL ni GEOS dans le noyau. Pas d'étape visible ajoutée au plan. Pas de réseau.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| MNT en degrés | `terrain.slope` sur EPSG:4326 | `work_epsg` choisi et journalisé ; l'égalité avec le MNT reprojeté à la main relève de la seconde moitié | — |
| MNT en mètres | EPSG:2154 | résultat inchangé, pas de reprojection | — |
| Rejeu | ligne journalisée avec `work_epsg` | identique, sans nouvelle reprojection | ligne ancienne : complétée par le système du MNT |
| Aucun système métrique | MNT en degrés, projet en degrés, étendue inconnue | refus | `dem.not_projected` avec la raison |

**Décisions du propriétaire (2026-11-21) :**
1. La reprojection est ordonnée par une commande déléguée cachée (`raster.reproject_work`), dictée avant l'étape, sortie nommée par le noyau.
2. Rééchantillonnage bilinéaire, taille de cellule par défaut de gdalwarp, fixés dans la ligne de table (seconde moitié).
3. Liste fermée de `is_known_metric` conservée ; un projet dans un autre système métrique retombe sur le fuseau UTM.
4. Découpage en deux : ce plan couvre le noyau (lecture du système du MNT, règle `work_epsg`, journal et rejeu, refus `dem.not_projected`, dictée avant, fiches). Le geste QGIS, la sonde et l'égalité avec le MNT reprojeté à la main vont dans `deferred-work.md`, à reprendre en ticket.
5. `raster.contour` reçoit son étape de reprojection avec la seconde moitié.

</frozen-after-approval>

## Code Map

- `core/src/dem.rs` l.516, 146, 574 -- `check_crs`, `epsg_of`, `is_known_metric` ; ajouter la lecture du système du MNT (métrique ou géographique)
- `core/src/work_crs.rs` l.39 -- `work_epsg` pour les couches ; modèle de la règle
- `core/src/journal/mod.rs` l.465, 681, 698 -- `fill_work_crs`, rejeu
- `core/src/registry.rs` l.151, 344 -- `work_crs_param` ne désigne qu'une couche ; un MNT se désigne par chemin (`raster_path.rs`)
- `core/src/commands/terrain_*.rs`, `raster_reclassify.rs`, `raster_weighted_overlay.rs` -- ouvrent par `Dem::open`
- `core/src/serve/plans.rs` l.274 -- complétion des points WGS84 (11.22), doit viser `work_epsg`
- `qgis-plugin/cassini_qgis/analysis.py` l.408-440, 934, 1028 -- `translate_convert_raster` (`gdal:warpreproject`), `_REPROJECT_TO_WORK`, ligne `raster.contour`
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- ~11 fiches
- `qgis-plugin/tests/probe_buffer_degrees.py`, `probe_isochrone_degrees.py` -- modèles de sonde ; `fixtures/` : aucun MNT en 4326

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/dem.rs`, `core/src/work_crs.rs` -- lecture du système du MNT et règle de choix de `work_epsg` -- cœur de la règle
- [ ] `core/src/journal/mod.rs`, `core/src/registry.rs` -- remplissage, journalisation, rejeu pour un paramètre raster -- selon la décision 1
- [ ] commandes terrain et raster, fiches du catalogue -- champ `work_epsg` fourni par le noyau -- contrat
- [ ] `core/src/commands/raster_reproject_work.rs`, `core/src/serve.rs` -- commande cachée et dictée avant l'étape -- ordre au noyau
- [ ] tests noyau (`journal/tests.rs`, `dem.rs`) -- règle, refus, rejeu, MNT métrique inchangé -- preuves de la matrice

**Acceptance Criteria:**
- Given un MNT en EPSG:4326 et un projet, when une commande de terrain est planifiée, then le noyau remplit `work_epsg` et dicte la reprojection avant l'étape.
- Given un MNT métrique, then le résultat est inchangé.
- Given aucun système en mètres trouvable, then `dem.not_projected` dit pourquoi.
- Given `cargo test --workspace`, then vert ; `core/tests/catalogue.rs` vert.

## Implementation Notes

Première moitié (noyau) : `crs_of`, `work_epsg_for_dem`, `check_dem_system`, commande cachée `raster.reproject_work` dictée avant l'étape, quatre commandes branchées (`terrain.slope`, `aspect`, `hillshade`, `raster.reclassify`). Suite : `deferred-work.md`.

## Plan Change Log

## Review Triage Log

Passe 1 (lentilles quick + cas limites) : 3 high/medium corrigés, reste consigné.

| Verdict | Route | Constat et preuve |
|---|---|---|
| high | patch | `step:k` après une étape reprojetée désignait la reprojection, pas le terrain (reclassement sur des altitudes sans erreur) ; `command_step()` ajouté, test de régression. |
| medium | patch | Décision 3 non tenue : un projet en 3035/27700/2056 était choisi puis refusé par `Dem::open` ; filtré par `is_known_metric`, test 3035/27700/2056 vers 32632. |
| medium | patch | Reprojection de l'étape 1 non collectée par `current_plan` ni listée par `intermediate_files` ; corrigé, test attendu [1,2,3,4]. |
| medium | defer | Sept commandes de MNT non couvertes et complétion des points WGS84 (`plans.rs`) : consignées dans `deferred-work.md`. |
| medium | defer | Rééchantillonnage bilinéaire sur un raster catégoriel passé à `raster.reclassify` : consigné. |
| low | defer | Fuseau UTM (exception 32V, antiméridien, pôle) : code antérieur ; consigné. Rejeu de la reprojection par QGIS : consigné. |
| medium | non résolu | Six tests Python rouges (`test_delegated.py` x5, `test_result_conformance.py` x1) tant que le geste QGIS n'existe pas : `catalogue.rs` exige une fiche « faite » pour toute commande enregistrée. |

## Design Notes

Taille estimée du ticket entier : 500 à 900 lignes ; cette moitié en vise environ la moitié, plus grosse que 10.32 : le MNT se désigne par chemin, les commandes terrain ne sont pas déléguées (pas de ligne de table), et le noyau doit dicter une étape avant la lecture, ce que `dictated_by_core` ne fait pas aujourd'hui (il dicte après).

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: vert
