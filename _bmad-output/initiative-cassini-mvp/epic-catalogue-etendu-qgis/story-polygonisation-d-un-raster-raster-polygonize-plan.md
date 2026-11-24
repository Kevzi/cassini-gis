---
title: 'Polygonisation d''un raster (raster.polygonize)'
type: 'feature'
ticket: '37'
created: '2026-11-24'
status: 'built'
baseline_revision: '75f48a2cf32624e309900e282bceb7aaf6f9675e'
route: 'full'
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

**Problem:** Aucune commande ne transforme un raster en polygones : « transforme ce raster en polygones » n'a pas de plan. `raster.polygonize` est promise par le catalogue (statut `planifiee`) sans avoir été construite.

**Approach:** Ajouter `raster.polygonize`, déléguée à QGIS (`gdal:polygonize`, identifiant `gdal:` admis faute d'équivalent `native:`, décision du 2026-11-04), sur le modèle exact de `raster.contour` (raster par `RasterPath` en entrée, couche nouvelle en sortie). Elle écrit une couche de polygones, un par zone de cellules contiguës de même valeur, avec le champ `value`.

## Boundaries & Constraints

**Always:** raster désigné par `RasterPath`, `step:k` accepté (un raster reclassé est le cas d'usage, aucune unité en jeu) ; `.tif` ou `.tiff`, avec un système de coordonnées ; fichier d'origine intact et sans `.aux.xml` (AD-10) ; sortie `.gpkg` nommée par le noyau, ajoutée au projet, résultat `layer_id`, `name`, `crs`, `feature_count`. Paramètre `band` facultatif, entier ≥ 1, défaut 1 : un raster à plusieurs bandes est admis, une bande hors du nombre de bandes est refusée avant lancement (`analysis.bad_params`). Zones de 4 voisins (pas de diagonale), champ `value`. Commande non annoncée si `gdal:polygonize` manque (AD-25, automatique par `required_algorithms`). Paramètres propres à chaque série de QGIS (3.44 et 4.x) traduits dans la ligne de table.

**Never:** modifier `raster.contour`, `RasterPath`, `layer.add` ou `terrain.*` ; ré-entraîner le modèle (un seul passage après 10.38) ; ajouter une dépendance ; refuser `step:k`.

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Nominal | raster synthétique de classes (entrée 20), bande 1 | un polygone par zone, `value` exact, couche ajoutée | aucune |
| Bande choisie | raster à 3 bandes, `band` 2 | polygones de la bande 2 | aucune |
| Bande absente | `band` 4 sur 3 bandes | refus avant lancement | `analysis.bad_params` |
| Bande invalide | `band` 0, négative, texte | refus du noyau | `registry.invalid_params` |
| Chaîne | `raster: "step:1"` | accepté | aucune |
| Raster inconnu | chemin absent ou non `.tif` | refus | `layer.not_found` / `layer.invalid_source` |
| Sans SCR | raster sans système | refus net | `analysis.bad_params` |
| Algorithme absent | `gdal:polygonize` introuvable | commande non annoncée | AD-25 |

</frozen-after-approval>

## Code Map

- `core/src/commands/raster_polygonize.rs` (nouveau) -- copie de `raster_contour.rs` : `Delegated`, `.runs_in(Qgis).cancellable(true).path_params(&["raster"]).output_params(&["out"]).output_ext(".gpkg")`, `inventory::submit!` ; `band: Option<u32>` avec minimum 1 ; pas de `Distance`. `mod.rs` automatique (vérifier la ligne `mod`).
- `core/src/plan.rs` (~323) -- ne pas toucher : le refus de `step:k` est propre à `raster.contour`.
- `core/src/journal/tests.rs` -- rejeu identique d'un appel avec chemin (modèle : `raster.contour`).
- `qgis-plugin/cassini_qgis/analysis.py` -- `translate_raster_polygonize` près de `translate_raster_contour` (596) : `gdal:polygonize`, `INPUT`, `BAND`, `FIELD="value"`, `EIGHT_CONNECTEDNESS=False`, `EXTRA=None`, sortie `OUTPUT` ; ligne `ALGORITHMS["raster.polygonize"]` après `raster.contour` (1153), `raster: True`, `layer_output: True`. `_check_raster_input` (2316) exige une seule bande : lui faire accepter un `band` quand l'entrée le déclare (nouvelle clé `band_param`), sans changer `raster.contour` ni `raster.zonal_statistics`.
- `qgis-plugin/cassini_qgis/summary.py` (~398 `_contour`, 1248), `texts.py` (142, 502) -- phrase `sum_polygonize` FR/EN.
- Tests : `qgis-plugin/tests/test_analysis.py` (classe d'après `ContourTest` 3037), `test_summary.py` (1206), `test_result_conformance.py` (171, 508), `fixtures/resultats-extension.json`, `table_annulable.json` ; sonde `probe_polygonize.py` (nouvelle, d'après `probe_contour.py`) sur le raster de classes de l'entrée 20 (si absent des `fixtures`, en fabriquer un petit, synthétique).
- `training/src/families.rs` -- famille `vectorisation_raster` d'après `courbes_de_niveau` (5972, enregistrement ~327) ; `training/src/barres.rs` (compteur `par_commande`, 139) ; `ai/tests/par_commande/phrases.json` (nouvelles phrases NV*).
- `ai/tests/couverture_reelle/phrases.json` CV37 -- ne pas toucher : « image satellite » (RGB, plusieurs bandes) reste un refus juste.
- Docs : catalogue (ligne 382 et fiche 2214, `planifiee` → `faite`, paramètre `band`, champ `value`, changement de cap léger), `docs/entrainement-du-modele.md`, `docs/couverture-phrases-reelles.md` (119), `qgis-plugin/README.md` (recette), `README.md`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/raster_polygonize.rs`, `journal/tests.rs`, catalogue -- commande, schéma, tests -- registre.
- [ ] `qgis-plugin/cassini_qgis/{analysis,summary,texts}.py`, `table_annulable.json` -- geste `gdal:polygonize`, bande, résumé FR/EN -- exécution QGIS.
- [ ] `qgis-plugin/tests/*`, `fixtures/resultats-extension.json`, `probe_polygonize.py` -- tests de la matrice, AD-25, sonde sous QGIS 3.44.
- [ ] `training/src/families.rs`, `barres.rs`, `ai/tests/par_commande/phrases.json`, docs, READMEs -- famille, phrases, documentation.

**Acceptance Criteria:**
- Given le raster synthétique de classes et la bande 1, when la commande tourne dans QGIS 3.44, then les polygones recouvrent les zones attendues avec leur `value` et le raster reste intact, sans `.aux.xml`.
- Given une bande absente du raster, when la commande est lancée, then elle est refusée avant tout lancement.
- Given un QGIS sans `gdal:polygonize`, when l'extension annonce ses commandes, then `raster.polygonize` n'y est pas.
- Given « transforme ce raster en polygones », when l'agent compose, then le plan est valide et contient `raster.polygonize` ; la famille d'entraînement existe.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 constat corrigé, 3 écartés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| false | rejeté | Schéma publié sans `additionalProperties: false` : `schemars` suit `#[serde(try_from = "RawParams")]` et prend le schéma de `RawParams`, qui porte `deny_unknown_fields` ; même schéma qu'`analysis.distance_matrix` (10.36), qui passe `check_contract` ; le test « champ inconnu » refuse bien `field` |
| low | rejeté | Sonde sans cas `step:1` ni `gdal:polygonize` absent : `step:1` ne passe que par le noyau, dont `plan.rs` n'a pas changé (le refus est propre à `raster.contour`), et AD-25 est couvert par `PolygonizeTest` (`available_chains`) ; la sonde ne vise que l'exécution réelle sous QGIS |
| low | rejeté | `.asc` non hashé dans la sonde : il ne sert qu'au refus du non-GeoTIFF, refusé avant toute lecture, et la vérification finale des `.aux.xml` aurait signalé un fichier voisin |

Hors liste du plan : trois phrases `DM1` à `DM3` et une couche `Écoles` dans `ai/tests/par_commande/` (10.36 avait laissé `analysis.distance_matrix` sans ses 3 phrases, ce qui faisait échouer `par_commande_reel`), compteur de `barres.rs` 139 → 145 ; `"polygonize"` retiré de `missing` de CV37 (le mot est désormais un identifiant de commande, `refusals_now_wrong` le refusait), son refus reste juste.

## Design Notes

Plan type : `[{"command":"raster.polygonize","params":{"raster":"donnees/classes.tif"}}]`. Un raster reclassé s'enchaîne : `[... , {"command":"raster.polygonize","params":{"raster":"step:1"}}]`.

Décisions du plan, confirmées à l'approbation : `band` facultatif (la vérification du ticket veut la bande en paramètre, la fiche du catalogue ne l'avait pas : changement de cap léger, comme pour `analysis.distance_matrix`) ; 4 voisins ; champ `value` (la fiche), pas `DN`.

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` (CARGO_TARGET_DIR=C:/ct) -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_polygonize.py` -- expected: VERDICT OK
