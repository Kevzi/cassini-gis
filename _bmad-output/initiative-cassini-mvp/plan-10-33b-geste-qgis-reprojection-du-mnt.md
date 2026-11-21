---
title: 'MNT en degrés, seconde moitié : le geste QGIS de reprojection'
type: 'feature'
ticket: ''
created: '2026-11-21'
status: 'built'
baseline_revision: 'ef13d4b7f36c86b451ae0ef70f1b924c2e8eff44'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, edge-cases]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/story-mnt-en-degres-calcul-dans-un-systeme-en-metres-choisi-par-le-plan.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/deferred-work.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le noyau (commit ef13d4b) dicte `raster.reproject_work` avant `terrain.slope`, `aspect`, `hillshade` et `raster.reclassify`, mais l'extension n'a aucun geste pour l'exécuter : un MNT en degrés échoue encore de bout en bout, et six tests Python sont rouges.

**Approach:** Ajouter la ligne de la table fermée `raster.reproject_work` (geste fixe `gdal:warpreproject`, sortie nommée par le noyau, cellule par défaut de gdalwarp, rééchantillonnage dicté par le noyau selon la commande suivante), et prouver le tout par une sonde sous QGIS 3.44. `raster.contour` n'est pas reprojeté.

## Boundaries & Constraints

**Always:** Aucun choix laissé au modèle ni à l'extension (AD-5, AD-24) ; le noyau nomme la sortie (AD-10) ; nodata -9999 et Float32 comme `data.convert_raster` ; le MNT d'origine n'est jamais modifié ; un `work_epsg` égal au système du raster est refusé (jamais d'identité, RA06) ; extension en GPL, noyau sans GDAL ; fiches du catalogue à jour.

**Never:** Pas de logique métier dans l'extension. Pas de couverture des sept autres commandes de MNT ni de la complétion des points WGS84 (reste en `deferred-work.md`).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| MNT flottant en degrés | Float32, EPSG:4326, `work_epsg=2154` | fichier `.tif` en 2154, bilinéaire, nodata -9999, mêmes valeurs que `gdal:warpreproject` fait à la main, à la tolérance près | — |
| Raster entier en degrés | Int16/Byte | plus proche voisin, classes non mélangées | — |
| Identité | `work_epsg` = système du raster | refus | message clair |
| Annulation | utilisateur annule | `qgis.cancelled` | pas de fichier partiel |
| Contour sur MNT en degrés | `raster.contour` | inchangé : pas de reprojection | — |

**Décisions du propriétaire après relecture (2026-11-21) :** (1) le noyau ajoute un champ de rééchantillonnage à `raster.reproject_work` et le fixe selon la commande suivante : bilinéaire pour `terrain.*`, plus proche voisin pour `raster.reclassify` ; l'extension ne devine plus par le type de bande. (2) `raster.contour` n'est pas branché : un MNT en degrés y donne déjà des courbes justes ; son `work_epsg`, sa garde et la mention dans sa fiche sont retirés.

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/analysis.py` l.408-440, 1256-1263 -- `translate_convert_raster` et sa ligne : modèle du geste ; constantes `_RASTER_NODATA`, `_RASTER_FLOAT32`, `_RASTER_NEAREST` ; ajouter `_RASTER_BILINEAR = 1`
- `qgis-plugin/cassini_qgis/analysis.py` l.934-960, 1028-1036 -- `_REPROJECT_TO_WORK` (vectoriel) et ligne `raster.contour` ; `AnalysisBackend` l.1710-1729 (`raster_source`, `raster_crs`) : y ajouter `raster_is_integer`
- `core/src/commands/raster_contour.rs`, `core/src/registry.rs` l.355 -- `work_crs_on_raster()` s'applique dès que `work_crs_param` est déclaré ; ajouter `work_epsg` et la garde `check_dem_system`, comme `terrain_aspect.rs:128,149`
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- fiche `raster.contour`
- `qgis-plugin/tests/test_delegated.py` (5 tests), `test_result_conformance.py` l.500-560 -- `producers()` sans clé `("raster.reproject_work","MNT reprojeté")` ; `_convert_raster` l.301-312 : modèle ; `fixtures/resultats-extension.json:114` : entrée déjà présente
- `qgis-plugin/tests/probe_convert_raster.py`, `probe_buffer_degrees.py` -- squelette de la sonde ; commande : `set QT_QPA_PLATFORM=windows` puis `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_dem_degrees.py`, noyau via `CASSINI_CORE=C:\ct-5-57g\release\cassini.exe`
- À ne pas toucher : `core/src/journal/mod.rs`, dictée et rejeu (première moitié)

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- `translate_reproject_work`, `raster_is_integer`, ligne `raster.reproject_work` dans la table, étape de reprojection raster dans la ligne `raster.contour` -- le geste
- [ ] `core/src/commands/raster_contour.rs` et la fiche -- `work_epsg` fourni par le noyau, garde -- pour que le noyau dicte la reprojection
- [ ] `qgis-plugin/tests/test_analysis.py`, `test_result_conformance.py` -- cas de `ReprojectWorkTest`, producteur du résultat -- rendre les six tests verts
- [ ] `qgis-plugin/tests/probe_dem_degrees.py` -- sonde : MNT 4326 fabriqué depuis `fixtures/mnt-colline-2154.tif`, égalité avec la reprojection faite à la main, nodata, original inchangé (sha256), identité refusée, annulation, dictée par le noyau avant `terrain.slope` -- preuve sous QGIS 3.44
- [ ] `deferred-work.md` -- ne pas modifier ; consigner dans les notes d'implémentation ce qui reste

**Acceptance Criteria:**
- Given un MNT Float32 en EPSG:4326, when `terrain.slope` est planifiée, then le noyau dicte `raster.reproject_work`, l'extension l'exécute et la pente égale celle du MNT reprojeté à la main, à la tolérance près.
- Given un raster entier, when il est reprojeté, then ses classes ne sont pas mélangées.
- Given `python -m unittest discover -s qgis-plugin/tests`, then tout est vert ; `cargo test -p cassini-core` vert ; la sonde donne `VERDICT OK`.

## Implementation Notes

Geste `raster.reproject_work` dans la table fermée ; le noyau dicte `resampling` (bilinear pour terrain.*, nearest pour raster.reclassify). `raster.contour` laissé tel quel (décision après relecture). Sonde `probe_dem_degrees.py` OK sous QGIS 3.44. Durcissements consignés en `deferred-work.md`.

## Plan Change Log

## Review Triage Log

Passe 1 (lentilles quick + cas limites), en attente de décision du propriétaire.

| Verdict | Route | Constat et preuve |
|---|---|---|
| high | intent_gap | Règle figée « raster entier = plus proche voisin » : un MNT Int16 (SRTM, BD ALTI) en degrés, cas visé par le ticket, est rééchantillonné au plus proche voisin, d'où pentes et ombrage en escaliers. L'extension ne sait pas quelle commande suit. |
| medium | intent_gap | `raster.contour` : un MNT en degrés donnait déjà des courbes justes (intervalle vertical) ; la reprojection forcée altère les altitudes (bilinéaire) ou les terrasse (Int16). Le plan prévoyait une étape dans la ligne, l'implémenteur s'appuie sur la dictée du noyau. |
| medium | patch (en attente) | `raster_is_integer` lit `dataType(1).name` : en l'absence du nom ou pour CInt16/Int8, retombe sur « non entier » donc bilinéaire, classes fondues sans erreur ; comparer aux membres de l'énumération et retomber sur le plus proche voisin pour un type inconnu. |
| low | patch (en attente) | Fiche `raster.reproject_work` : liste de quatre commandes sans `raster.contour`, annulation non mentionnée, refus `data.unknown_crs`/`analysis.crs_not_metric` non documentés ; fiche contour sans le refus `analysis.bad_params` du système. |
| medium | defer | Nodata de la source non posé (SRTM -32768), gardes sur la taille de grille (pôles, antiméridien, BIGTIFF), annulation en cours de calcul non prouvée (la sonde annule avant le lancement), PAM/`.aux.xml` et bandes multiples non contrôlés dans le geste. |
| low | rejeté | Seuil `len(checks) >= 20` de la sonde : fragile mais sans effet réel. |

## Verification

**Commands:**
- `python -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `cargo test -p cassini-core` -- expected: vert, `core/tests/catalogue.rs` compris
- `python-qgis-ltr.bat qgis-plugin/tests/probe_dem_degrees.py` -- expected: `VERDICT OK`

**Manual checks (if no CLI):**
- Ouvrir la sortie de la sonde reprojetée dans QGIS ou comparer les en-têtes : système, taille de cellule, nodata.
