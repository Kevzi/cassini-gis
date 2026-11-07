---
title: 'Écriture GeoJSON du noyau et ouverture dictée'
type: 'feature'
ticket: '8'
created: '2026-11-06'
status: done
baseline_revision: '82e7a27809ca88286acadb8428ae29cb93f13bd6'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** aucune commande du noyau ne sait rendre des entités (points, lignes) ; `terrain.peaks` (entrée 15) et `terrain.least_cost_path` (entrée 10) en ont besoin, et AD-30 exige un seul écrivain GeoJSON déterministe et une ouverture dictée `layer.open_vector`, planifiée au catalogue.

**Approach:** un module unique du noyau écrit un GeoJSON 2008 qui déclare son EPSG ; la session dicte `layer.open_vector {path: "out:<k>.<n>"}` après toute étape du noyau dont la sortie est `.geojson`, exactement comme `layer.open_raster` (journal, rejeu, couche d'origine cachée) ; l'extension reçoit le geste dans sa table fermée ; la fiche passe en `faite`.

## Boundaries & Constraints

**Always:** fiche `layer.open_vector` respectée à la lettre (`path` chaîne obligatoire, résultat `crs`, `layer_id`, `name`, erreur `layer.invalid_source`, `dictated_by_core`) ; écriture par `serde_json`, aucune dépendance nouvelle ; sortie déterministe : ordre des entités et des propriétés celui donné par l'appelant, nombres écrits pareil, aucune date ; membre `"crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::<code>"}}` ; nombre non fini refusé, rien d'écrit ; la dictée n'a lieu que si l'extension annonce `layer.open_vector` ; refusée par `command.run` (`registry.dictated`), absente du prompt, de la grammaire, du MCP et du formulaire ; le rejeu compare le GeoJSON octet pour octet (déjà vrai pour toute sortie du noyau) et laisse la couche d'origine hors de l'export rejoué ; message d'erreur sans chemin absolu ; comportement et tests de `layer.open_raster` inchangés.

**Never:** pas de reprojection (AD-19) ; pas de commande `terrain.peaks` ni aucune commande publique qui écrive un GeoJSON (entrées 15 et 10) ; pas de lecture GeoJSON dans le noyau ; pas de géométrie autre que `Point` et `LineString` ; aucun changement de l'entraînement ni du générateur ; aucun import direct de PyQt5/PyQt6.

**Decision (2026-11-06, propriétaire) :** l'essai réel se joue sur QGIS 3.44 seul, le seul installé sur le poste ; l'essai sur QGIS 4.x est reporté (`deferred-work.md`) jusqu'à l'installation de QGIS 4. Le plan est gardé entier malgré sa taille : l'écrivain et l'ouverture dictée vont ensemble (AD-30).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Deux écritures | mêmes entités, EPSG 2154 | deux fichiers identiques octet pour octet, égaux à `fixtures/points-2154.geojson` | — |
| Ordre gardé | propriétés `rank`, `elevation`, `prominence` | écrites dans cet ordre, entités dans l'ordre reçu | — |
| Nombre non fini | une coordonnée NaN ou infinie | rien d'écrit | erreur `geojson.unwritable` nommée |
| Dossier absent | chemin dans un dossier inexistant | rien d'écrit | `geojson.unwritable`, nom du fichier seul |
| Dictée | étape du noyau à sortie `.geojson`, extension qui annonce le geste | ligne suivante `layer.open_vector {path: "out:<k>.1"}`, même `origin` et `plan_step`, `layer_id` journalisé | — |
| Sans annonce | extension sans le geste, ou CLI/MCP autonome | rien dicté | — |
| Fichier illisible / sans système | GeoJSON corrompu, ou CRS que QGIS ne reconnaît pas | aucune couche ajoutée | `layer.invalid_source`, nom du fichier seul |
| QGIS réel | fixture ouverte par le vrai geste | couche valide, `crs` = `EPSG:2154`, entités et propriétés lues | — |

</frozen-after-approval>

## Code Map

- `core/src/geojson.rs` (nouveau, `pub mod geojson` dans `core/src/lib.rs`) -- l'écrivain unique : `Geometry::{Point(f64,f64), LineString(Vec<(f64,f64)>)}`, `Property::{Integer(i64), Number(f64)}`, `Feature { geometry, properties: Vec<(&'static str, Property)> }`, `pub fn write(path: &Path, epsg: u32, features: &[Feature]) -> Result<(), GeojsonError>` ; `GeojsonError` calquée sur `DemError` (`core/src/dem.rs` l. 40-100), code `geojson.unwritable`. Sérialiser avec des `impl Serialize` (struct + `serialize_map` sur le `Vec`) pour fixer l'ordre ; contrôler la finitude avant tout `fs::write`. Écrire en une fois (`fs::write`), fin de ligne finale.
- `fixtures/points-2154.geojson` (nouveau) -- deux ou trois points autour de (652250, 6850250), propriétés `rank`, `elevation`, `prominence` ; un test du module vérifie que l'écrivain le redonne octet pour octet.
- `core/src/commands/layer_open_vector.rs` (nouveau, découvert par `build.rs`) -- calqué sur `layer_open_raster.rs` : `pub const OPEN_VECTOR`, `path: String` (pas `RasterPath` : la fiche dit chaîne), sortie `{layer_id, name, crs}`, `.runs_in(Qgis)`, `.cancellable(false)`, `.path_params(&["path"])`, `.dictated_by_core()`, exemple `out:1.1`, `.tests(..)`. Le test de `layer_open_raster.rs` l. 75-80 (« aucune autre commande ne porte la marque ») doit admettre les deux.
- `core/src/journal/mod.rs` l. 149-151, 186-210 (`opens_rasters`, `with_raster_opening`), l. 270-300 (`call_logged`, `rasters_written`), l. 547-555 (couche ouverte reconnue par son fichier, `OPEN_RASTER` seul) -- ajouter `opens_vectors` + `with_vector_opening`, une dictée `OPEN_VECTOR` pour une sortie `.geojson` d'une commande `RunsIn::Core` (constante `VECTOR_EXTENSIONS` dans `geojson.rs`), et `OPEN_VECTOR` dans le `if let` du rejeu.
- `core/src/journal/tests.rs` l. 1640-1890 -- modèles `RasterQgis` et tests de dictée et de rejeu ; la dictée vectorielle s'éprouve avec une commande de test du noyau, enregistrée par `Registry::register` (l. 301) sur `Registry::standard()`, qui écrit la fixture par `geojson::write` (sortie `.geojson`).
- `core/src/serve.rs` l. 871 -- `.with_vector_opening(commands.contains(OPEN_VECTOR))` ; l. 900/936 refusent déjà toute commande dictée. `core/src/serve/plans.rs` l. 338 : commentaire.
- `ai/src/plan.rs` l. 1174 -- liste des dictées : `["layer.open_raster", "layer.open_vector"]` (ordre de `specs()`), mêmes assertions pour `open_vector`. `cli/src/mcp.rs` l. 694 : assertion sœur.
- `qgis-plugin/cassini_qgis/vector_output.py` (nouveau) -- calqué sur `raster_output.py` : `QgisBackend.add_vector(path, name)` par `QgsVectorLayer(path, name, "ogr")`, rend `(layer_id, authid)` ou `None` si la couche ou son `crs()` est invalide (`authid()` vide → invalide) ; geste `LayerOpenVectorGesture`. Inscrit dans `delegated.py` l. 28 ; `form.py` l. 25 commentaire.
- `qgis-plugin/tests/test_vector_output.py` (nouveau, modèle `test_raster_output.py`) ; `test_panel_logic.py` l. 36 et `test_tracer.py` l. 815-823 si elles énumèrent les gestes dictés. `test_delegated.py` confronte la table au catalogue : rien à y changer.
- `qgis-plugin/tests/probe_vector_output.py` (nouveau, modèle `probe_raster_output.py`) -- sous le vrai QGIS : la fixture copiée dans un projet temporaire, ouverte par le vrai geste, couche valide, `crs()` `EPSG:2154`, 3 entités, champs `rank`, `elevation`, `prominence` ; fichier corrompu → `layer.invalid_source`, rien d'ajouté.
- `catalogue-des-commandes.md` l. 95 (rien), l. 266 (`planifiée (dictée)` → `faite (dictée)` ou `faite`, selon la forme de la ligne `layer.open_raster`), fiche l. 992-1015 en `faite`, déplacée après `layer.open_raster` ; une phrase en §2 : erreur `geojson.unwritable` de l'écrivain. `core/tests/catalogue.rs` confronte, rien à y changer.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/geojson.rs`, `core/src/lib.rs`, `fixtures/points-2154.geojson` -- écrivain et tests de la matrice (octets identiques, égalité à la fixture, ordre, non fini, dossier absent, `crs` déclaré, `LineString`).
- [x] `core/src/commands/layer_open_vector.rs`, `core/src/commands/layer_open_raster.rs` -- déclaration et tests (déléguée, dictée, `path` seul, `registry.unavailable` sans QGIS).
- [x] `core/src/journal/mod.rs`, `core/src/journal/tests.rs` -- dictée vectorielle dans les trois flux, rien sans annonce, ouverture en échec nommée à son étape, rejeu identique deux fois avec couche d'origine cachée.
- [x] `core/src/serve.rs`, `core/src/serve/plans.rs` -- annonce lue à `session.hello` ; test que `command.run` refuse `layer.open_vector`.
- [x] `ai/src/plan.rs`, `cli/src/mcp.rs` -- la dictée absente du prompt, de la grammaire et du MCP.
- [x] `qgis-plugin/cassini_qgis/vector_output.py`, `delegated.py`, `form.py`, `qgis-plugin/tests/test_vector_output.py` et tests du panneau -- geste et tests avec doublure.
- [x] `qgis-plugin/tests/probe_vector_output.py` -- sonde réelle, jouée sur le poste.
- [x] `catalogue-des-commandes.md` -- fiche en `faite`, même commit.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when `cargo test --workspace`, then tout passe, dont `catalogue` avec la fiche `faite` et les tests de `layer.open_raster` inchangés.
- Given la recette de l'extension, when ses tests tournent sans QGIS, then la table fermée contient `layer.open_vector` et la confrontation au catalogue passe.
- Given QGIS 3.44 sur le poste, when la sonde ouvre `fixtures/points-2154.geojson` par le vrai geste, then la couche est valide, en `EPSG:2154`, avec ses 3 entités et ses propriétés.

## Implementation Notes

- Le pilote GeoJSON d'OGR prend **WGS84 en silence** quand le membre `crs` manque ou nomme un code
  qu'il ne connaît pas (constaté sous QGIS 3.44.15 : `EPSG::999999` lu `EPSG:4326`). Le geste lit
  donc le code déclaré par le fichier (`declared_crs`, forme de 2008 seule) et le passe au fond,
  qui refuse toute couche dont `crs().authid()` diffère : rien n'est ajouté, `layer.invalid_source`.
  C'est la seule façon de tenir la ligne « CRS que QGIS ne reconnaît pas » de la matrice.
- La commande de test `test.write_points` (sortie `.geojson`) vit dans `core/src/journal/tests.rs`
  et s'inscrit par `Registry::register` sur `Registry::standard()` ; elle écrit la fixture par
  `geojson::fixture_points()` (`#[cfg(test)]`), si bien que la dictée, le rejeu et la sonde
  portent sur les mêmes octets.
- `fixtures/points-2154.geojson` se régénère comme les MNT :
  `CASSINI_REGENERER_FIXTURES=1 cargo test -p cassini-core geojson::tests::two_writes`.
- Vérifié : clippy (`--workspace --all-targets -D warnings`) et `cargo test` des quatre crates sous
  Windows (le WSL n'a pas libclang pour `llama-cpp-sys-2`) ; `cargo fmt --check` ; recette de
  l'extension (425 tests) et sa garde ; sonde `probe_vector_output.py` sous QGIS 3.44.15 : OK.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment) — high 0, medium 3, low 13, false 6, maybe-false 0 ; 8 patchs (6 entrées envoyées à l'implémenteur), 2 renvois, le reste rejeté.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Un GeoJSON vide (aucune entité) est refusé `layer.invalid_source` (cas limite, aucun test) | medium | patch | Essai sous QGIS 3.44.15 : couche valide, `wkbType` 100 (NoGeometry), `crs()` invalide, alors qu'OGR lit 2154 ; un résultat vide légitime de `terrain.peaks` serait une erreur. Correctif : `setCrs` du code déclaré pour une couche vide, et cas ajouté à la sonde |
| `with_vector_opening` dans `hello` n'est éprouvé par aucun test qui observe une dictée (3 lentilles) | medium | patch | Les tests du journal posent le drapeau à la main ; `scripted_registry()` n'a aucune commande à sortie `.geojson` ; supprimer la ligne laisse tout vert |
| Une sortie vecteur du noyau ne donne pas son `layer_id` à `step:k.layer_id` dans un plan | medium | defer | `plans.rs` l. 336-342 : `outputs[k]` ne tient que le résultat du noyau ; même chose qu'un raster aujourd'hui (parité tenue), mais l'enchaînement points hauts → analyse QGIS en aura besoin (entrée 15) |
| Refus du système non concordant de `QgisBackend.add_vector` vérifié seulement par la sonde manuelle | low | defer | `FakeBackend` refait le refus ; seule `probe_vector_output.py` exerce le vrai fond ; convention des sondes raster, une recette automatique exigerait QGIS |
| Commentaire « seule l'ouverture raster annoncée » contre `.with_raster_opening(false)` (3 lentilles) | low | patch | `journal/tests.rs`, `a_geojson_without_the_announced_gesture_dictates_nothing` |
| Aucun test avec les deux ouvertures annoncées (raster puis GeoJSON) | low | patch | Branche de `opener_of` non exercée ; ajout d'un cas |
| Message d'`geojson.unwritable` avec l'`io::ErrorKind` en anglais | low | patch | `format!(.., e.kind())` ; AD-21 veut un message en français |
| `json.load` peut lever `RecursionError`, non attrapée | low | patch | Sortirait en `qgis.gesture_failed` au lieu de l'erreur nommée ; ajout à l'`except` |
| Regex `^…$` accepte un saut de ligne final | low | patch | `re.match` et `$` ; `fullmatch` |
| Sonde : `QgsWkbTypes.PointGeometry` retiré de QGIS 4 | low | patch | Alias déprécié depuis 3.30 ; `Qgis.GeometryType.Point` |
| Dossiers `scratch()` des tests jamais effacés ; commentaire de `call_logged` mal replié | low | patch | Corrections directes |
| Écriture partielle si `fs::write` échoue en cours | low | rejet | Rare (disque plein) ; même façon que `Dem::write_geotiff` ; un fichier temporaire renommé ajoute de la complexité |
| `LineString` de moins de 2 positions, propriétés en double acceptées | low | rejet | Aucun appelant aujourd'hui ; les gardes sont à poser avec le premier appelant (entrées 15 et 10) |
| Ordre des axes d'un EPSG géographique | false | rejet | Tout appelant écrit dans le système du MNT, qui doit être projeté (`dem.not_projected`) ; aucun chemin n'écrit en 4326 |
| Le test du fichier illisible ne vérifie que la doublure | false | rejet | Le test éprouve la propagation de l'erreur déléguée, nommée et journalisée à son étape ; le message vient de QGIS par construction |
| Le second contrôle du geste viendrait après `addMapLayer` | false | rejet | Le fond refuse avant d'ajouter ; le cas n'est pas atteignable |
| « Tests de `layer.open_raster` inchangés » alors que l'un est élargi | false | rejet | Le test n'admet que la commande sœur (prévu par la Code Map) ; comportement raster inchangé, tests raster verts ; corriger reviendrait à modifier le plan |
| Code Map et section Verification en retard, Change Log vide | low | rejet | Le correctif modifie le plan ; l'écart est dit dans les Implementation Notes |
| Le « Problem » reste vrai : aucune commande de production n'écrit de GeoJSON | false | rejet | Voulu : bornes « Never » (entrées 15 et 10) |
| Code EPSG non relié au raster source ; unicité par construction seulement | false | rejet | Aucun appelant ; la règle se vérifie à l'entrée 15 |
| Doublon `vector_output.py` / `raster_output.py` ; JSON lu en entier sur le fil de l'interface | low | rejet | Fichiers bornés (100 points au plus) ; refactor sweep à l'entrée 13 |
| Le refus d'EPSG 0 n'est pas dit au catalogue | low | rejet | Cosmétique |

## Design Notes

Forme écrite (une ligne, compacte, comme `serde_json::to_string`) :
`{"type":"FeatureCollection","crs":{"type":"name","properties":{"name":"urn:ogc:def:crs:EPSG::2154"}},"features":[{"type":"Feature","geometry":{"type":"Point","coordinates":[652250.0,6850250.0]},"properties":{"rank":1,"elevation":187.5,"prominence":42.0}}]}` + `\n`.
GDAL lit ce membre `crs` hérité de 2008 et le pose sur la couche ; l'axe est est/nord pour 2154, sans ambiguïté. Le fichier de test reste la référence que la sonde ouvre dans QGIS : le même octet que l'écrivain produira pour `terrain.peaks`.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `cargo test --workspace` -- expected: tout vert
- `cd qgis-plugin/tests && python3 -I -m unittest discover -s .` (forme exacte de la recette à reprendre) -- expected: tout vert

**Manual checks (if no CLI):**
- Sonde `probe_vector_output.py` par `python-qgis-ltr.bat`, QGIS fermé ou non : sortie « OK », couche en `EPSG:2154`.
