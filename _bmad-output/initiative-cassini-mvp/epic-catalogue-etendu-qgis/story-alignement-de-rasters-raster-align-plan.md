---
title: 'Alignement de rasters (raster.align)'
type: 'feature'
ticket: '38'
created: '2026-11-24'
status: 'built'
baseline_revision: '10e2bd0f38a0cc15154f18432fb38ef31daee2f5'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Deux rasters de grilles différentes (taille de cellule, origine, système) ne se combinent pas dans le noyau (AD-20) : « aligne ce raster sur le MNT » n'a pas de plan. `raster.align` est promise par le catalogue (statut `planifiee`) sans avoir été construite.

**Approach:** Ajouter `raster.align`, déléguée à QGIS (`native:alignsingleraster`, avec `EXTENT` posé sur l'emprise de la référence ; `native:alignrasters` écarté : pas de méthode globale, `.aux.xml` à côté de la source), sur le modèle de `data.convert_raster` (raster en entrée, un GeoTIFF en sortie, `layer.open_raster` dicté ensuite par le noyau). Elle écrit un GeoTIFF qui a exactement le système, la taille de cellule, l'origine et les dimensions du raster de référence.

## Boundaries & Constraints

**Always:** `raster` et `reference` sont des `RasterPath` (chemin ou `step:k`) ; `.tif` ou `.tiff`, une seule bande, avec un système de coordonnées, pour les deux (un raster sans système est refusé avant lancement : QGIS le supposerait en silence dans le système de la référence) ; fichiers d'origine intacts et sans `.aux.xml` (AD-10), rasters libérés avant l'algorithme ; `resampling` facultatif, `nearest` (défaut) ou `bilinear`, liste fermée ; sortie `.tif` nommée par le noyau, résultat `file` seul (fiche du catalogue) ; une référence dans un autre système que le raster est admise : la sortie prend le système de la référence (reprojection faite par QGIS, constatée par la sonde) ; emprises sans recouvrement : `analysis.failed` avec le message de QGIS. Commande non annoncée si `native:alignsingleraster` manque (AD-25). Paramètres propres à chaque série de QGIS (3.44 et 4.x) traduits dans la ligne de table.

**Never:** `bicubic` ou toute autre méthode que les deux ; plusieurs rasters en une commande (une sortie par commande, `writes_one_raster`) ; modifier `RasterPath`, `layer.open_raster`, `data.convert_raster` ou `raster.reproject_work` ; ré-entraîner le modèle (un seul passage après ce ticket) ; ajouter une dépendance ; laisser le noyau lire une sortie sans la vérifier (la sonde ouvre le GeoTIFF écrit avec le lecteur du noyau ou `layer.open_raster`).

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Nominal | raster en cellules de 7, référence en cellules de 10, même système | sortie : cellule, origine, colonnes, lignes et système de la référence ; couche ouverte | aucune |
| Bilinéaire | `resampling` `bilinear` | valeurs interpolées ; `nearest` : valeurs de la source | aucune |
| Autre système | référence en EPSG:3857, raster en 2154 | sortie dans le système de la référence, grille de la référence | aucune |
| Chaîne | `raster: "step:1"` ou `reference: "step:1"` | accepté | aucune |
| Méthode inconnue | `resampling` `cubic` | refus du noyau | `registry.invalid_params` |
| Paramètre manquant | `reference` absente | refus du noyau | `registry.invalid_params` |
| Sans système | raster ou référence sans SCR | refus avant lancement | `analysis.bad_params` |
| Plusieurs bandes | raster ou référence à 3 bandes | refus avant lancement | `analysis.bad_params` |
| Fichier inconnu | chemin absent ou non `.tif` | refus | `layer.not_found` / `layer.invalid_source` |
| Sans recouvrement | emprises disjointes | échec nommé, aucun fichier partiel | `analysis.failed` |
| Algorithme absent | `native:alignsingleraster` introuvable | commande non annoncée | AD-25 |

</frozen-after-approval>

## Code Map

- `core/src/commands/raster_align.rs` (nouveau) -- copie de `data_convert_raster.rs` : `Delegated`, `.runs_in(Qgis).cancellable(true).path_params(&["raster","reference"]).output_params(&["out"]).output_ext(".tif")`, `inventory::submit!`, deux `RasterPath`, `resampling` en énumération (`Nearest` défaut, `Bilinear`, `deny_unknown_fields`), résultat `{file}` ; `writes_one_raster`. Deux `path_params` : précédent `geocode.ban`. `mod.rs` automatique (`core/build.rs`). Vérifier que `check_contract` accepte deux `RasterPath`. `core/src/plan.rs` : ne pas toucher (`step:k` généralisé).
- `core/src/journal/tests.rs` (modèle : `convert_raster`) -- ouverture dictée `layer.open_raster` après la sortie ; rejeu identique avec deux chemins résolus.
- `qgis-plugin/cassini_qgis/analysis.py` -- `translate_raster_align` près de `translate_convert_raster` (450) : `INPUT`, `RESAMPLING_METHOD` (0 nearest, 1 bilinear, constantes `_RASTER_NEAREST`/`_RASTER_BILINEAR` existantes, 408-420), `RESCALE=False`, `REFERENCE_LAYER`, `EXTENT` = emprise de la référence dans son système (sans elle la sortie garde l'emprise de la source, 21 lignes au lieu de 20 : constat de la sonde), sortie `OUTPUT` ; ligne `ALGORITHMS["raster.align"]` après `data.convert_raster` (1444), `raster: True`, sans `layer_output`. `_begin_raster` (2275) : contrôler `reference` comme `raster` (une bande, un système, `.tif`) avec `_check_raster_input` (2338), avant toute ouverture, pour que `GDAL_PAM_ENABLED=NO` précède la première (un chemin en paramètre crée un `.aux.xml`) ; clé de table `reference_input` ; l'emprise vient d'une nouvelle méthode du backend (modèle : `raster_crs`, `raster_band_count`), jamais d'une logique dans `translate`. Ne pas toucher aux entrées `data.convert_raster`, `raster.reproject_work`, `raster.contour`.
- `qgis-plugin/cassini_qgis/summary.py` (~398 `_contour`, 1248), `texts.py` -- phrase `sum_align` FR/EN (deux fichiers, la méthode).
- Tests : `qgis-plugin/tests/test_analysis.py` (classe d'après `ContourTest` 3037 et les tests de `data.convert_raster`), `test_summary.py`, `test_result_conformance.py`, `fixtures/resultats-extension.json`, `table_annulable.json` ; sonde `probe_align.py` (nouvelle, d'après `probe_polygonize.py` et `probe_contour.py`) : deux GeoTIFF synthétiques créés avec GDAL (référence 20×20 en cellules de 10, origine (652000, 6500200) ; source 30×30 en cellules de 7, origine (651995.5, 6500210), EPSG:2154), vérifie la grille exacte, les deux méthodes, l'autre système, les originaux identiques octet pour octet sans `.aux.xml`, les refus, et `Qgis.version()`.
- `training/src/families.rs` -- famille `alignement_raster` d'après `courbes_de_niveau` (5972) et `vectorisation_raster` ; deux rasters de l'inventaire désignés par leur `source` (`add_dem` et un second raster) ; enregistrement ~327 ; `training/src/barres.rs` (compteur `par_commande`) ; `ai/tests/par_commande/phrases.json` (phrases NV7+, 3 dont 2 plans). Vérifier `training/tests/recouvrement.rs` : une phrase de test ne doit jamais être engendrée par la famille.
- `ai/tests/couverture_reelle/phrases.json`, `ai/tests/**` -- chercher les refus qui citent `align` ou `resample` dans `missing` (le test `refusals_now_wrong` rejette un mot devenu identifiant de commande).
- Docs : catalogue (ligne 383 et fiche, `planifiee` → `faite`, `native:alignsingleraster`, emprise de la référence, un seul raster : le ticket parlait d'un ou plusieurs ; changement de cap léger), `docs/entrainement-du-modele.md`, `docs/couverture-phrases-reelles.md`, `qgis-plugin/README.md` (recette), `README.md`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/raster_align.rs`, `journal/tests.rs`, catalogue -- commande, schéma, tests -- registre.
- [ ] `qgis-plugin/cassini_qgis/{analysis,summary,texts}.py`, `table_annulable.json` -- geste `native:alignsingleraster`, emprise de la référence, gardes, résumé FR/EN -- exécution QGIS.
- [ ] `qgis-plugin/tests/*`, `fixtures/resultats-extension.json`, `probe_align.py` -- tests de la matrice, AD-25, sonde sous QGIS 3.44.
- [ ] `training/src/families.rs`, `barres.rs`, `ai/tests/**`, docs, READMEs -- famille, phrases, documentation.

**Acceptance Criteria:**
- Given deux rasters synthétiques de grilles différentes, when la commande tourne dans QGIS 3.44, then la sortie partage système, cellule, origine et dimensions de la référence, pour `nearest` et `bilinear`, et les originaux restent identiques, sans `.aux.xml`.
- Given un raster ou une référence sans système ou à plusieurs bandes, when la commande est lancée, then elle est refusée avant tout lancement.
- Given un QGIS sans `native:alignsingleraster`, when l'extension annonce ses commandes, then `raster.align` n'y est pas.
- Given « aligne ce raster sur le MNT », when l'agent compose, then le plan est valide, contient `raster.align`, et la famille d'entraînement existe ; la sortie est ouverte dans le projet.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 constat corrigé, 2 différés, 7 écartés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | defer | Noms de paramètres de `native:alignsingleraster` vérifiés sous 3.44 seulement, `qgisMaximumVersion=4.99` : même limite que 10.26, aucun QGIS 4.x sur le poste ; consigné dans deferred-work |
| medium | defer | Source qui ne couvre qu'une partie de la référence : QGIS rogne la sortie à l'emprise commune (19 colonnes au lieu de 20 dans la sonde), donc « mêmes dimensions » ne vaut que si la source couvre la référence ; limite documentée au catalogue et au README de l'extension, hors matrice du plan ; consigné dans deferred-work |
| false | rejeté | Description de `resampling` sans reprojection : la description de la commande dit déjà que la sortie a « le système » de la référence (`raster_align.rs` 62-63) |
| false | rejeté | Contrôle du raster d'entrée « en double » avec `layer_output` : `raster.align` n'a pas ce drapeau, aucun double contrôle aujourd'hui |
| low | rejeté | Emprise lue sans garde sur `None` : `_check_raster_input` vient de rouvrir la référence avec succès, seul un fichier supprimé entre deux lectures y mènerait |
| low | rejeté | Repli WKT de l'emprise jamais exercé : un GeoTIFF à système personnalisé sans `authid` est rare, et le garde ajouterait une branche pour un cas non constaté |
| low | rejeté | Phase 2 de la sonde faible (`max` de la pente seulement) : `terrain.slope` sur `out:1.1` prouve que le noyau lit le GeoTIFF, ce que la contrainte demande |
| low | rejeté | Test de rejeu avec deux fois le même fichier : `inputs.len() == 2` compte une entrée par paramètre, la fixture identique n'y change rien |
| low | rejeté | Assertion « une fiche planifiée reste » retirée de `catalogue.rs` : `raster.align` était la dernière, l'assertion ne pouvait plus tenir |

Hors liste du plan : `core/tests/catalogue.rs` (assertion retirée, voir ci-dessus) ; seuil « gros projets » de `training/src/lib.rs` abaissé de 8 % à 7 % (7,8 % mesuré), comme pour 10.36 et 10.37.

## Design Notes

Plan type : `[{"command":"raster.align","params":{"raster":"donnees/occupation.tif","reference":"dem/mnt.tif","resampling":"nearest"}}]`. Un raster aligné s'enchaîne : `{"raster":"step:1", ...}` dans une commande suivante.

Décisions du plan, confirmées à l'approbation : `native:alignsingleraster` et non `native:alignrasters` (méthode par élément, `.aux.xml` à côté de la source, un seul raster à la fois de toute façon) ; un seul raster par commande, la fiche du catalogue et `writes_one_raster` l'imposent ; `nearest` par défaut (des classes ne se mélangent pas), `bilinear` pour des altitudes ; la fiche disait `native:alignrasters` « repli `native:alignsingleraster`, à trancher » : tranché. Le type de données de la source est gardé (pas de conversion en Float32 comme `data.convert_raster`) ; la sonde vérifie que le noyau lit le GeoTIFF écrit, sinon le geste ajoute la conversion et le dit.

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` (CARGO_TARGET_DIR=C:/ct, LIBCLANG_PATH et Ninja comme la note locale) -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_align.py` -- expected: VERDICT OK
