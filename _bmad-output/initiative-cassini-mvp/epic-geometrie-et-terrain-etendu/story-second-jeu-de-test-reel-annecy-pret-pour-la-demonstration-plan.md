---
title: 'Second jeu de test réel (Annecy) prêt pour la démonstration'
type: 'chore'
ticket: '18'
created: '2026-11-07'
status: 'done'
baseline_revision: 'd4402238f9918c27653814dc9bddb7f1c2a4c689'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Villeneuve-le-Roi n'a ni relief fort, ni gros MNT, ni site à cheval sur deux fuseaux UTM, ni beaucoup de Shapefile et CSV réels ; la démonstration (11.14) doit pouvoir s'essayer sur un jeu qui les a. Les fichiers d'Annecy sont téléchargés (`../donnees-test-cassini/annecy/brut/`, 2026-11-07) mais bruts.

**Approach:** Hors du dépôt, dans `../donnees-test-cassini/annecy/` : décompresser, convertir les MNT en GeoTIFF par le GDAL de QGIS, tirer les gares du 74 en CSV X/Y, bâtir un projet QGIS de test à chemins relatifs, écrire `LISEZMOI.txt`, puis faire tourner une sonde enchaînée (~10 phrases, vrai modèle, vrai noyau installé) sur ce projet, rejeu compris. Le dépôt ne reçoit que ce plan ; aucun code du noyau ni de l'extension ne change.

</frozen-after-approval>

## Implementation Notes

Route courte : aucune ligne de code versionné, le travail est de la préparation de données hors du dépôt et une sonde jetable (dans `%TEMP%\sonde-11-18\`), sur le modèle de `sonde-11-16`.

Choix faits à l'investigation :
- MNT : BD ALTI 25 m sur tout le 74 (16 dalles, 5000 × 4000 cellules) ; RGE ALTI 5 m découpé sur le bassin annécien (≈ 30 × 30 km autour d'Annecy, à cheval sur le méridien 6° E, frontière 31N/32N), parce que le 5 m du département entier frôle ou dépasse le plafond de 1 Gio du noyau (`core/src/dem.rs`, `MAX_DECODED_BYTES`). GeoTIFF Float32, EPSG:2154, compression deflate (lue par la crate `tiff`).
- Gares : CSV `X,Y` en WGS84 (lon/lat tirés de `position_geographique`), filtrées sur `codeinsee` 74*.
- Projet en EPSG:2154 : communes (ADMIN EXPRESS, 74), zones du PLUi (Shapefile), parcelles d'Annecy (GeoJSON 4326), bâtiments et routes (BD TOPO, GPKG), gares (CSV), deux MNT.
- La licence des gares SNCF se vérifie sur la page de la source avant d'écrire `LISEZMOI.txt`.
- Un échec de phrase dans la sonde est un constat (exemple d'entraînement ou ticket), jamais un correctif par phrase.

Fait le 2026-11-07 (tout hors du dépôt) :
- `annecy/extrait/` : quatre archives 7z, le zip du PLUi et le cadastre décompressés (≈ 4,7 Go).
- `annecy/mnt/mnt-74-25m.tif` (5000 × 4000, 46 Mo) et `mnt-annecy-5m.tif` (6001 × 6001, 97 Mo). Le 5 m du 74 entier ferait 20 000 × 18 000 cellules, 1,34 Gio : au-delà du plafond, d'où la découpe.
- `annecy/gares-74-4326.csv` : 39 gares ; licence SNCF vérifiée à la source : ODbL.
- `annecy/projet-test-annecy.qgz` : 10 couches valides relues sous QGIS 3.44 (8 en 2154, parcelles et gares en 4326). Premier montage : les communes étaient filtrées sur le GeoPackage de France entière, et `featureCount()`, que l'inventaire transmet (`qgis-plugin/cassini_qgis/inventory.py:139`), rendait 34 746. Corrigé après la relecture : les communes sont exportées dans `annecy/communes-74.gpkg` (279 entités).
- `annecy/LISEZMOI.txt` écrit ; renvoi ajouté au `LISEZMOI.txt` général.
- Noyau reconstruit (`--features gpu`) et installé, avec l'extension, l'installé datant d'avant 11.13 ; l'ancien gardé en `cassini.exe.avant-11-18`.
- `terrain.slope` par le noyau seul : 25 m en 2,9 s, 5 m en 5,7 s.

Sonde enchaînée (`%TEMP%\sonde-11-18\sonde.py`, 11 phrases, modèle `cassini-composition-4b`) : 6/11 plans exécutés (MGRS 32T et 31T de part et d'autre de 6° E, distance et azimut, pente 25 m, points hauts et ombrage 5 m). Constats, sans correctif ici :
1. Rejeu : `replay.diverged` à l'étape 2 du premier plan, un plan arrêté en cours par une erreur (le tampon refusé n'a pas de sortie `out`). Un journal qui contient un plan échoué ne se rejoue pas.
2. Points donnés en WGS84 sur un MNT en 2154 (champ de vision, ligne de vue) : le noyau refuse (`terrain.crs_mismatch`) et le modèle ne compose pas `data.transform_point` avant.
3. Gares en EPSG:4326 : tampon refusé (`analysis.crs_not_metric`) ; `data.reproject` n'est que planifiée. La phrase de démo transposée à Annecy échoue donc.
4. Composition entre 37 et 60 s par phrase (environ 14 s sur Villeneuve) : l'inventaire de ce projet est long.
5. Carroyage UTM de 1 km en A3 refusé, avec plus de 200 lignes par axe, même après la correction des communes : l'emprise de la carte reste à vérifier.

Second passage, après la correction des communes (`sonde-rejeu.py`, les 6 phrases réussies plus l'export) : 6/7 plans exécutés et composition en 30-35 s. Le rejeu redonne à l'identique les étapes 1 à 9 (les 6 plans, ouvertures des couches de résultat comprises), puis diverge à l'étape 10, l'export échoué : c'est le défaut du constat 1.

Le dépôt reçoit aussi les cinq constats, ajoutés à `deferred-work.md`, comme le veut le workflow ; l'Intent ne nommait que le plan.

## Verification

**Manual checks:**
- Le projet s'ouvre sous le Python de QGIS 3.44 : chaque couche valide, dans son système.
- `cassini.exe run terrain.slope` lit les deux MNT et écrit une pente.
- La sonde enchaînée tourne sur le projet, et le rejeu redonne à l'identique toutes les étapes réussies.
- `LISEZMOI.txt` nomme chaque source, sa date et sa licence ; `git status` du dépôt ne montre que ce plan et `deferred-work.md`.

## Review Triage Log

- medium, patch : rejeu non démontré au premier passage (`replay.diverged` à l'étape 2). Seconde sonde lancée : étapes 1 à 9 identiques, divergence à l'étape 10 seulement (défaut reporté au constat 1).
- low, patch : l'Intent et la vérification disaient « seulement ce plan » ; `deferred-work.md` change aussi. Vérification et notes corrigées (l'Intent est figé).
- medium, patch : les communes pointaient sur le fichier de France entière ; l'inventaire annonçait 34 746 entités. Elles sont exportées dans `communes-74.gpkg`. La composition est passée de 37-60 s à 30-35 s ; le refus du carroyage, lui, demeure.
- low, patch : taille du MNT 25 m contradictoire (3 000 × 3 500 contre 5000 × 4000), corrigée.
- low, patch : raisonnement du report 5 (environ 100 km), refait avec l'emprise réelle (125 × 100 km, environ 130 lignes).
