---
title: 'Couches raster : ouverture et inventaire d''un MNT'
type: 'feature'
ticket: '20'
created: '2026-11-19'
status: 'built'
baseline_revision: 'a4fe3e91c48a0f9946855ae0f36be82c14ded32d'
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

**Problem:** `layer.add` n'ouvre que des vecteurs : un MNT GeoTIFF ne peut pas entrer dans le projet comme couche, donc `raster.zonal_statistics` (ticket 12) n'aura pas de `layer_id` à lire, et `project.inventory` ne donne pas les dimensions d'un raster.

**Approach:** Ajouter à `layer.add` un paramètre `raster` (GeoTIFF, `RasterPath`, relatif au projet) comme troisième entrée du « exactement un de », rendre `kind` (`vector` ou `raster`) et un `layer_id` de session ; faire donner largeur et hauteur d'un raster par `project.inventory`.

## Boundaries & Constraints

**Always:** Exactement un de `path`, `table`, `raster`, refusé par le noyau à la validation (`registry.invalid_params`) et par l'extension (`layer.one_input`) ; `.tif` et `.tiff` seulement ; `QgsRasterLayer` en lecture, fichier d'origine intact (aucun `.aux.xml` ni pyramide à côté, vérifié par empreinte) ; `x_field` et `y_field` refusés avec `raster` ; `crs` donné seulement quand le raster n'en a pas, `layer.unknown_crs` sinon, `layer.crs_conflict` s'il contredit (AD-19) ; `kind` toujours rendu ; `width` et `height` optionnels dans `InventoryLayer` (pixels, raster seulement), remplis par l'extension.

**Never:** Dimensions dans la vue apprise de l'agent (`agent_inventory.rs`, coût de réentraînement) ; ouverture d'autre format raster (`.asc`, `.vrt`) ; refus d'un MNT en degrés à l'ouverture (AD-20 : `terrain.*` le refuse à la lecture) ; modification de `layer.open_raster` ; famille d'exemples pour le modèle (ticket 17).

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Nominal | `raster: "mnt.tif"` (EPSG:2154) | couche raster, `kind: raster`, `source_crs: EPSG:2154`, fichier intact | aucune |
| Inventaire | même projet | entrée `kind: raster`, `crs`, `width`, `height` | aucune |
| Sans système | raster sans SCR, sans `crs` | refus | `layer.unknown_crs` |
| SCR donné | raster sans SCR, `crs: EPSG:2154` | ouvert, SCR attribué en mémoire, fichier intact | aucune |
| SCR contraire | SCR du fichier ≠ `crs` | refus | `layer.crs_conflict` |
| Mauvaise entrée | zéro, deux ou trois de path/table/raster | refus | `registry.invalid_params`, `layer.one_input` |
| Mauvais format | `raster: "mnt.asc"` | refus | `layer.unsupported_format` |
| Coordonnées | `raster` avec `x_field` | refus | `layer.one_input` |

</frozen-after-approval>

## Code Map

- `core/src/commands/layer_add.rs` -- `raster: Option<RasterPath>` dans RawParams/LayerAddParams, TryFrom compte exactement un des trois, `kind` dans le résultat, `path_params` + `raster`, descriptions FR/EN, exemple `{"raster":"dem/mnt.tif"}`, tests
- `core/src/commands/project_inventory.rs:49-78` -- `InventoryLayer` : `width`, `height` optionnels
- `core/src/agent_inventory.rs:665-672` -- ne pas toucher (vue apprise)
- `qgis-plugin/cassini_qgis/layer_add.py` (FORMATS 22-28, `QgisBackend.load` 45, `_open` 128-170, `_resolve_crs` 194-217) -- méthode `QgsRasterLayer(path, name, "gdal")` (voir `raster_output.py:34-36`), extension `.tif/.tiff`, `kind` dans le résultat, refus de `x_field`/`y_field`
- `qgis-plugin/cassini_qgis/inventory.py` (`kind_of` 121, `raster_source` 263, `_read` ~390-430) -- `width`/`height` d'un raster
- `qgis-plugin/cassini_qgis/form.py:91-94` (`is_path_field`), `panel.py`, `summary.py:391-399`, `texts.py` -- `raster` en sélecteur de fichier (filtre `*.tif *.tiff` si le sélecteur sait filtrer, comme pour `table`), phrase de résumé
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` (l.342, 450-482, 160-165) -- `raster` et `kind` sortis de `planned_*` ; `core/tests/catalogue.rs` compare ; fiche d'inventaire : `width`/`height`
- `core/src/journal/tests.rs` (~4967 modèle) -- le chemin relatif de `raster` est une entrée portable, le rejeu est identique ; bras simulés `layer.add` (~775, 1530, 2996) avec `kind`
- `fixtures/resultats-extension.json:87-95` -- cas raster, et `"kind":"vector"` aux trois cas existants ; `qgis-plugin/tests/test_result_conformance.py` (~280-300, 453)
- `qgis-plugin/tests/test_layer_add.py`, `test_inventory.py` (~230-270), `test_summary.py`, `test_panel_logic.py`, nouveau `probe_layer_add_raster.py`
- `fixtures/mnt-colline-2154.tif`, `mnt-plan-incline-2154.tif` (régénérés par `Dem::write_geotiff`), `mnt-colline-sans-systeme.asc` -- à réutiliser
- `ai/tests/par_commande/phrases.json` (LA1-LA3, ajouter LA4) + `inventory.json`, `training/src/barres.rs:54` (compte 114 + phrases ajoutées ; échoue sinon), `README.md:111`, `qgis-plugin/README.md:122+`
- À ne pas toucher : `raster_output.py`, `terrain.*`, `work_crs`, `families.rs`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/layer_add.rs` -- param `raster`, un seul des trois, `kind`, tests, `.tests(&[...])` à jour -- contrat du registre
- [ ] `core/src/commands/project_inventory.rs`, catalogue -- `width`/`height` optionnels, fiches `layer.add` et inventaire alignées
- [ ] `qgis-plugin/cassini_qgis/layer_add.py`, `inventory.py` -- ouverture raster, SCR, `kind`, dimensions
- [ ] `qgis-plugin/cassini_qgis/form.py`, `panel.py`, `summary.py`, `texts.py` -- sélecteur de fichier pour `raster`, résumé FR/EN
- [ ] `qgis-plugin/tests/probe_layer_add_raster.py` -- QGIS 3.44 : ouverture du MNT synthétique, SCR, largeur/hauteur de l'inventaire, empreinte du fichier avant/après, absence de `.aux.xml`, raster sans SCR avec et sans `crs`, constats consignés dans Implementation Notes
- [ ] Tests (plugin, conformité, journal), `fixtures/resultats-extension.json`, `phrases.json` (phrase « ouvre le MNT … » qui ne déclenche pas `layer.open_raster`) et `inventory.json`, `barres.rs`, READMEs

**Acceptance Criteria:**
- Given un GeoTIFF synthétique, when `layer.add` l'ouvre, then `project.inventory` le montre avec son système, sa largeur et sa hauteur, et le fichier est intact.
- Given une phrase demandant l'ouverture d'un MNT et un inventaire adapté, when l'agent compose, then le plan est valide.
- Given un ancien journal de `layer.add` avec `path`, when on le rejoue, then le résultat est inchangé.

## Implementation Notes

Constats de la sonde `probe_layer_add_raster.py` sous QGIS 3.44.15 (VERDICT : OK) :

- Le MNT synthétique `mnt-colline-2154.tif` s'ouvre : `kind: raster`, `source_crs: EPSG:2154` ; l'inventaire rend 50 x 50 pixels, égaux à ceux de GDAL.
- **Écart au plan, constaté puis corrigé** : un simple `QgsRasterLayer(path, name, "gdal")` fait écrire à GDAL un `<fichier>.tif.aux.xml` (statistiques min/max calculées par QGIS à l'ouverture), même avec `loadDefaultStyle=False`. Le fichier d'origine reste identique à l'empreinte, mais un fichier apparaît à côté. `QgisBackend.load_raster` ouvre donc la couche avec `GDAL_PAM_ENABLED=NO` le temps de l'ouverture seulement, puis rend l'option telle quelle : plus aucun `.aux.xml`, les autres rasters de la session ne sont pas touchés.
- Raster sans SCR : `layer.unknown_crs` sans `crs` ; avec `crs: EPSG:2154`, ouvert, SCR attribué en mémoire, fichier intact ; `crs` contraire sur un fichier qui a un SCR : `layer.crs_conflict` ; `.asc` : `layer.unsupported_format`.
- Formulaire : `Option<RasterPath>` s'engendre en `anyOf` ; `form._follow` le reconnaît (marque `x-cassini: raster`) pour en faire un champ texte avec bouton de choix de fichier (le sélecteur ne filtre pas, comme pour `table`).
- `ai/tests/common/mod.rs` : `layer.add{raster}` est vérifié contre `unloaded_files`, pas contre les rasters déjà ouverts. Phrase LA4 ajoutée (115 phrases à la main) ; `inventory.json` porte déjà `donnees/mnt_zone_nord.tif` non chargé, rien à y changer.
- Rendu : la sonde rend la couche une fois (`QgsMapRendererSequentialJob`, rendu par défaut) puis relit le dossier : aucun `.aux.xml` ni pyramide, empreinte inchangée.
- `{"raster":"step:1"}` : le noyau le valide (un `RasterPath` admet `step:k`, traduit en `out:<k>.1` seulement dans un plan), mais `layer.add` n'a pas de sortie raster à lui donner ; sur le canal, l'extension reçoit un nom sans extension `.tif` et répond `layer.unsupported_format`. Constaté, non utilisé ici.
- Cas « sans SCR, avec crs » de la sonde : la branche exécutée est celle où GDAL a bien retiré le SCR de la copie (`CreateCopy` puis `SetProjection("")`, « sans-scr.tif porte un SCR : False »), donc `layer.unknown_crs` puis ouverture avec `crs` ont été réellement constatés ; la branche de repli (« GDAL a gardé le SCR ») n'a pas servi.
- Rejeu : `kind`, `width`, `height` ajoutés à `LATE_FIELDS` (`journal/summary.rs`). Limite : `LATE_FIELDS` ne retire que des champs de premier niveau ; `width`/`height` d'un inventaire sont dans `layers[]`, un ancien inventaire contenant un raster ne leur doit donc rien de mieux qu'avant (un inventaire est de toute façon gardé résumé).

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 1 medium et 1 maybe-false corrigés par patch, 2 low repris dans les notes.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch | `LATE_FIELDS` (`journal/summary.rs:22`) ne liste que `source_details` : une ligne d'avant `kind`, `width`, `height` s'afficherait comme différente au rejeu, contre le 3e critère ; les ajouter et tester une ligne ancienne écrite à la main |
| maybe-false | patch | `.aux.xml` possible après l'ouverture (statistiques calculées à l'affichage) ; la sonde ne regarde qu'avant tout rendu ; la sonde rend la couche puis revérifie, et le rapport tranche |
| low | patch | `step:k` accepté par le noyau mais refusé par l'extension : le plan disait « à constater » ; consigné dans Implementation Notes |
| low | patch | Branche de la sonde « sans SCR, avec crs » non consignée ; consignée dans Implementation Notes |

## Design Notes

Désignation : `layer.add{raster}` prend un `RasterPath` et rend un `layer_id` ; `raster.zonal_statistics` (ticket 12) désignera ensuite la couche par ce `layer_id` (AD-9, ticket et fiche conciliés). `RasterPath` accepte aussi `step:k` : à constater, non à utiliser ici.

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_layer_add_raster.py` -- expected: VERDICT OK
