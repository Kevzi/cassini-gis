---
title: 'Conversion d''un raster en GeoTIFF (data.convert_raster)'
type: 'feature'
ticket: '30'
created: '2026-11-10'
status: 'done'
baseline_revision: '624835c56a8e277651a0ab35e3f21b660c8cb50b'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-complements-geometrie-et-mise-en-page/change-complements-geometrie-et-mise-en-page.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** les commandes de terrain ne lisent que le GeoTIFF (AD-20) ; un MNT livré en ASCII Grid (`.asc`, BD ALTI de l'IGN), ERDAS (`.img`), JPEG 2000 (`.jp2`) ou VRT n'est pas utilisable sans passer par QGIS à la main.

**Approach:** ajouter `data.convert_raster`, commande déléguée à QGIS (une ligne de la table fermée, AD-24), qui écrit le raster en GeoTIFF Float32, sans-donnée -9999, première bande, à l'emplacement nommé par le noyau (AD-10) ; `assign_epsg` seulement pour une source sans système, jamais de reprojection (AD-19) ; le noyau dicte ensuite `layer.open_raster` ; famille du générateur `conversion_raster`, dont l'enchaînement conversion puis pente sur `step:1`.

## Boundaries & Constraints

**Always:** original intact (§2.6) ; message d'erreur sans dossier (AD-9) ; fiche du catalogue écrite exactement comme le changement de cap D10, passée à `faite` ; un raster désigné par `RasterPath`, jamais par un id de couche.

**Never:** reprojection ; choix entre deux algorithmes dans la chaîne (AD-24) ; nouvelle dépendance ; phrase des recettes ou des jeux tenus à l'écart dans le générateur.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| ASCII Grid sans `.prj` | `raster` .asc, `assign_epsg` 2154 | GeoTIFF Float32, une bande, -9999, même grille, EPSG:2154 ; le noyau ouvre la couche | — |
| Sans système, rien déclaré | `raster` .asc | GeoTIFF sans système, `epsg: null`, le résumé le signale | — |
| Autre système déclaré | source EPSG:2154, `assign_epsg` 32632 | rien lancé, aucun fichier | `analysis.bad_params` |
| Même système déclaré | source EPSG:2154, `assign_epsg` 2154 | accepté | — |
| Code inconnu | `assign_epsg` 99999 | rien lancé | `data.unknown_crs` |
| Fichier absent / illisible | chemin absent, texte | rien lancé | `layer.not_found` / `layer.invalid_source` |
| Échec, annulation | algorithme en échec, arrêt | aucun `.tif` ni `.aux.xml` | `analysis.failed` / `qgis.cancelled` |

</frozen-after-approval>

## Code Map

- `core/src/commands/data_convert_raster.rs` -- commande déléguée, schéma, tests du registre et de `check_plan`.
- `core/src/journal/mod.rs` -- `opener_of` : une commande déléguée qui écrit un `.tif` est suivie de `layer.open_raster` (amendement du 2026-11-05 d'AD-24) ; le vecteur reste réservé au noyau.
- `core/src/journal/tests.rs` -- doublure QGIS `data.convert_raster`, journal et rejeu.
- `qgis-plugin/cassini_qgis/analysis.py` -- genre d'entrée `raster` ajouté une fois au geste générique, ligne `gdal:warpreproject`.
- `qgis-plugin/cassini_qgis/summary.py`, `texts.py` -- phrase du résumé.
- `training/src/families.rs`, `inventories.rs`, `lib.rs` -- famille `conversion_raster`, MNT d'un autre format, validation d'un raster non ouvert cité par la conversion.
- `fixtures/mnt-colline-sans-systeme.asc` -- ASCII Grid synthétique sans `.prj`, trois cellules -99999.
- `qgis-plugin/tests/probe_convert_raster.py` -- sonde réelle QGIS 3.44.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/data_convert_raster.rs` -- commande et tests
- [x] `core/src/journal/mod.rs` -- ouverture dictée après une sortie raster déléguée
- [x] `qgis-plugin/cassini_qgis/analysis.py` -- ligne de table, gardes, relecture du GeoTIFF
- [x] `_bmad-output/.../catalogue-des-commandes.md` -- fiche `faite`, algorithme retenu
- [x] `training/src/families.rs` -- famille `conversion_raster`

**Acceptance Criteria:**
- Given un ASCII Grid de `/fixtures`, when il est converti par QGIS, then le noyau lit le GeoTIFF (pente calculée) avec le même nombre de cellules et -9999.
- Given un journal conversion puis pente, when le projet est rouvert et rejoué, then `identical: true`.
- Given `core/tests/catalogue.rs`, when il tourne, then il est vert avec la fiche faite.

## Implementation Notes

- `gdal:warpreproject` seul, et non `gdal:translate` : gdal_translate change l'étiquette du sans-donnée sans réécrire les cellules (le -99999 de la BD ALTI deviendrait une altitude) ; une chaîne fixe ne choisit pas entre deux algorithmes (AD-24). Sans `TARGET_CRS` ni résolution ni emprise, la grille est celle de la source (vérifié : géotransformation identique sur 1000 × 1000 cellules). `DATA_TYPE` 6 = Float32 sous QGIS 3.44 (énumération relue sur le vrai QGIS), `-srcband 1 -dstband 1`, DEFLATE.
- Erreurs alignées : `layer.not_found` (absent), `layer.invalid_source` (illisible, comme `layer.open_raster`).
- Sonde réelle : 35/35, BD ALTI 25 m (`BDALTIV2_25M_FXX_0925_6525`) convertie à l'identique, pente de 0 à 79,3°, rejeu identique.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 11 constats — medium 3, low 4, false 0, rejetés 2, différés 2.

| # | Verdict | Sort | Constat et preuve |
|---|---|---|---|
| 1 | medium | patch | Générateur : la conversion d'un MNT ouvert n'écrivait pas `assign_epsg` ; un `.asc` sans `.prj` donnait un GeoTIFF sans système, refusé ensuite par la pente. Le plan déclare désormais le système que l'inventaire donne à la couche (le geste accepte le même système redéclaré). |
| 2 | low | patch | Un GeoTIFF dont le système n'a pas de code EPSG était dit « sans système », avec un conseil voué au refus : le résumé dit « sans code EPSG » et réserve le conseil à une source sans système. |
| 3 | medium | patch | `same_crs` comparait l'objet SCR : un `.prj` ESRI du Lambert-93 non rattaché à EPSG:2154 aurait été refusé. Comparaison par code, puis par égalité, puis par définition PROJ. |
| 4 | maybe-false (medium) | defer | Index `DATA_TYPE` = 6 vérifié sous QGIS 3.44 seulement ; à vérifier sous 4.x (sonde 4.x déjà reportée par 11.8). |
| 5 | medium | patch | La sonde ne faisait pas passer la fixture `/fixtures` par le noyau : la phase 2 convertit la fixture et vérifie `nodata_cells ≥ 3` lu par la pente (36/36 sous QGIS 3.44.15). |
| 6 | low | defer | Annulation d'un gdalwarp en cours non jouée sous le vrai QGIS (comme pour `data.reproject`) : couverte par la doublure seulement. |
| 7 | low | patch | Message d'un `assign_epsg` invalide emprunté à la reprojection : message propre au système déclaré. |
| 8 | low | patch | La source ouverte par `raster_source` restait référencée pendant le travail : relâchée avant le lancement. |
| 9 | low | patch | `("asc", …)` en double sans commentaire : commentaire ajouté (poids voulu, format de l'IGN). |
| 10 | rejeté | — | Échappement de `tickets.toml` (ligne de l'entrée 29) : nécessaire, le TOML était illisible par `tickets.py` ; correction minimale, signalée au commit. |
| 11 | rejeté | — | Erreur `layer.invalid_source` ajoutée à la fiche : la fiche du catalogue laissait l'erreur « alignée au ticket » ; alignée sur `layer.open_raster`. |

## Verification

**Commands:**
- `cargo test -p cassini-core` -- vert, `catalogue.rs` compris
- `cargo test -p cassini-training --release` -- vert, garde de recouvrement compris
- `python3 -m unittest discover -p "test_*.py"` (dans `qgis-plugin/tests`) -- vert
- `python -I qgis-plugin/tests/check_extension.py` -- propre
