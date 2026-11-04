---
title: 'Les sorties raster du noyau s''ouvrent dans le projet QGIS'
type: 'feature'
ticket: '3'
created: '2026-11-04'
status: 'built'
baseline_revision: '680ce0b3cc01be2dea7c54bb04e511200c947561'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `terrain.slope` (4.2) écrit un GeoTIFF dans le dossier de travail, mais l'extension n'ajoute au projet que les sorties des commandes déléguées à QGIS : la pente reste invisible. Et l'inventaire ne donne d'un raster que son nom de fichier, que le modèle ne peut pas recopier dans `dem` (chemin relatif au projet).

**Approach:** Quand une commande `runs_in: core` rend un `file` GeoTIFF, l'extension l'ouvre comme couche raster nommée du radical du fichier (AD-10), que le résultat arrive par `command.run`, `plan.execute` ou un appel d'IA externe. Pour cela le noyau rend `file` absolu à l'extension sur ces trois chemins. L'inventaire donne aux seules couches raster leur chemin relatif au fichier de projet.

## Boundaries & Constraints

**Always:** Décision (propriétaire, 2026-11-04) : le noyau rend les chemins de sortie **absolus à l'extension** dans la réponse de `plan.execute` et dans la notification `external.call` (comme `command.run` depuis 4.2) ; ce qui repart vers le relais d'IA externe reste relatif et nettoyé (AD-29), et le journal reste relatif. L'exception de AD-9 et `docs/canal-local.md` sont mis à jour en ce sens. Couche nommée du radical du fichier donné par le noyau (`3-terrain.slope`), comme les sorties déléguées. Un fichier déjà ouvert dans le projet n'est pas ajouté une seconde fois. Inventaire : `source` d'un raster = chemin relatif au dossier du fichier de projet, séparateurs `/`, `..` permis (le noyau le journalise non portable) ; `source` d'un vecteur inchangé (nom seul). Décision (propriétaire, 2026-11-04) : si aucun chemin relatif n'existe (autre lecteur sous Windows, projet non enregistré), le nom de fichier seul. Jamais de chemin absolu dans l'inventaire. Le noyau reconnaît toujours une couche de résultat raster d'après le nom de fichier de sa source.

**Never:** Logique métier dans l'extension (elle ouvre ce que le noyau nomme, rien d'autre). Ouvrir une sortie de rejeu. Ouvrir un raster d'utilisateur par `layer.add` (entrée 20 de l'epic 10). Toucher au calcul de la pente, au journal ou au rejeu. Chemin absolu vers l'IA externe. Apprendre quoi que ce soit au modèle.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Pente lancée à la main | `RUN_DONE` de `terrain.slope`, `file` absolu existant | couche raster `1-terrain.slope` ajoutée au projet | — |
| Pente dans un plan | `PLAN_DONE`, étape 2 `terrain.slope` | couche `2-terrain.slope` ajoutée ; les étapes QGIS ne sont pas rouvertes | — |
| Pente par l'IA externe | `external_call_finished` de `terrain.slope` | couche ajoutée ; la réponse au relais garde `2-terrain.slope.tif` relatif | — |
| Sortie non raster | `layout.export_pdf` (QGIS) ou `file` `.pdf` | rien n'est ajouté | — |
| Fichier absent ou illisible | `file` qui n'existe pas, ou GeoTIFF corrompu | rien n'est ajouté, le panneau ne plante pas | message dans le journal du panneau, pas d'exception |
| Déjà ouvert | même `file` reçu deux fois | une seule couche | — |
| Inventaire raster dans le projet | `<projet>/dem/mnt.tif` | `source: "dem/mnt.tif"` | — |
| Inventaire raster à côté | `<projet>/../donnees/mnt.tif` | `source: "../donnees/mnt.tif"` | — |
| Inventaire raster autre lecteur | `D:\mnt.tif`, projet sur `C:` | `source: "mnt.tif"` | — |
| Inventaire vecteur | `<projet>/data/plu.gpkg|layername=zone` | `source: "plu.gpkg"` (inchangé) | — |

</frozen-after-approval>

## Code Map

- `core/src/serve/plans.rs` l.~327 -- `let shown = relativize(&value, …)` : rendre `value` tel quel dans `steps[].result`. Tests de `serve.rs` qui attendent un `file` relatif dans `plan.execute` : les adapter (absolu, sous le dossier de travail).
- `core/src/serve/calls.rs` l.~105-135 -- garder `scrub(relativize(..))` pour la valeur rendue au relais ; la notification `external.call` `finished` porte la valeur brute (absolue). Ne pas toucher `escapes_project`, `scrub_text` des erreurs.
- `core/src/agent_inventory.rs` -- `results_of` (l.~159), `intermediate` (l.~447-464) et l.~500 comparent `source` à `WorkFile.name` : comparer le dernier segment de `source` (après `/`). Tests : un raster de résultat `p.cassini/1-terrain.slope.tif` reconnu comme résultat ; un raster d'utilisateur `dem/mnt.tif` jamais retiré.
- `qgis-plugin/cassini_qgis/inventory.py` -- `QgisBackend` : ajouter `project_dir()` (`QgsProject.instance().absolutePath()`) ; `ProjectInventoryGesture._read` : pour `kind == "raster"`, `source` par une fonction pure `raster_source(raw, project_dir)` (`os.path.relpath` sur la partie fichier, `ValueError` ou dossier vide → `source_name`). `source_name` inchangé pour les vecteurs.
- `qgis-plugin/cassini_qgis/raster_output.py` (nouveau) -- fonction pure `rasters_to_open(command, result, catalogue)` : `runs_in == "core"` au catalogue, `file` chaîne absolue d'extension `.tif`/`.tiff`, fichier existant → `[(path, radical)]` ; et `open_rasters(items, backend)` avec un `QgisBackend` (seul import de `qgis.core` : `QgsRasterLayer(path, name, "gdal")`, `isValid`, refus d'un doublon par `source()` normalisée, `addMapLayer`). Patron : `analysis.py` `load_output` et les doublures des tests.
- `qgis-plugin/cassini_qgis/panel.py` -- trois branchements, rien d'autre : `RUN_DONE` (l.~715, `payload["command"]`, `payload["result"]`), `PLAN_DONE` (l.~642, `payload["steps"]` : `{step, command, result}`), `_on_external_event` quand `kind == client.EXTERNAL_CALL_FINISHED` sans `error`. Le catalogue est `self.catalogue` (entrées avec `runs_in`).
- `docs/canal-local.md` l.~288 et l.~407, `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md` AD-9 « Exception volontaire » -- ajouter, daté 2026-11-04, que les réponses du canal de l'extension (`command.run`, `plan.execute`, notification `external.call`) portent les chemins de sortie absolus.
- `qgis-plugin/tests/probe_inventory.py` (patron de sonde) -- nouvelle sonde `probe_raster_output.py`.
- Lecture seule : `qgis-plugin/cassini_qgis/runlog.py` `result_folder` (marchera désormais après un plan), `summary.py` `file_name`, `fixtures/mnt-plan-incline-2154.tif`.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/serve/plans.rs`, `core/src/serve/calls.rs`, `core/src/serve.rs` (tests) -- chemins absolus vers l'extension, relais inchangé ; un test par chemin (plan avec `terrain.slope` : `file` absolu existant ; appel externe : notification absolue, réponse au relais relative) -- l'extension a besoin du chemin réel
- [x] `core/src/agent_inventory.rs` -- reconnaissance par nom de fichier + tests -- le budget de 5.19 reste juste avec des sources relatives
- [x] `qgis-plugin/cassini_qgis/inventory.py`, `qgis-plugin/tests/test_inventory.py` -- `raster_source` et les quatre lignes d'inventaire de la matrice (chemins Windows et POSIX)
- [x] `qgis-plugin/cassini_qgis/raster_output.py`, `qgis-plugin/tests/test_raster_output.py` -- logique et doublure : lignes 1 à 6 de la matrice
- [x] `qgis-plugin/cassini_qgis/panel.py`, `qgis-plugin/tests/test_panel_logic.py` -- les trois branchements
- [x] `qgis-plugin/tests/probe_raster_output.py`, `qgis-plugin/README.md` -- sonde sous QGIS : `cassini run terrain.slope` sur une copie de `fixtures/mnt-plan-incline-2154.tif` dans un projet temporaire, `open_rasters` ajoute une couche raster valide nommée `1-terrain.slope` ; l'inventaire du projet (MNT sous `dem/`) donne `dem/…tif`
- [x] `docs/canal-local.md`, contrat d'architecture AD-9 -- la décision du 2026-11-04

**Acceptance Criteria:**
- Given le noyau sans QGIS, when tournent `cargo test --workspace`, fmt et clippy, then tout est vert et aucun message vers le relais externe ne porte de chemin absolu.
- Given l'extension sans QGIS, when tourne `python -I -m unittest discover -s qgis-plugin/tests` et `check_extension.py`, then tout est vert.
- Given QGIS 3.44 offscreen, when tourne `probe_raster_output.py`, then elle finit par `RÉSULTAT : OK`.

## Implementation Notes

- Sorties : `calls.rs` notifie `external.call` avec le résultat brut et ne réduit (`relativize` puis `scrub`) que la réponse au relais ; le message d'erreur reste nettoyé des deux côtés. `plans.rs` rend `value` tel quel, `relativize` n'y est plus importé.
- Panneau : l'extraction des trois événements est la fonction pure `raster_output.outputs_of` (testée dans `test_panel_logic.py`) ; `panel.py` n'ajoute qu'un appel `_open_core_rasters` par branchement. Un raster absent ou illisible laisse une ligne en erreur dans le journal du panneau (`raster_output.report`, nom du fichier seul), jamais une exception.
- `raster_source` choisit `ntpath` ou `posixpath` selon la forme du chemin (tests Windows sous Linux) ; chemins de deux systèmes, relatif ou autre lecteur : nom seul.
- Vérification : sous WSL, libclang manque pour `llama-cpp-sys-2` ; `cargo test`/`clippy` du noyau tournent sous WSL avec `-p cassini-core`, l'espace de travail entier sous Windows (`CARGO_TARGET_DIR=C:\ct-4-3`). Sonde sous QGIS 3.44.15 avec `CASSINI_CORE=C:\ct-4-3\release\cassini.exe` : `RÉSULTAT : OK`.

## Plan Change Log

- 2026-11-04, audit de la matrice (ligne 5) : le premier jet écrivait l'échec d'ouverture dans le journal des messages de QGIS et écartait en silence un fichier absent. Choix du propriétaire : journal du panneau. `rasters_to_open` ne filtre plus l'existence ; `open_rasters` rend `UNREADABLE` ; `report` ajoute la ligne en erreur.

## Review Triage Log

Boucle 0 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).

| # | Lentille | Constat | Verdict | Route | Preuve |
|---|---|---|---|---|---|
| 1 | verification-gap, blind, intent | Les trois branchements de `panel.py` ne sont testés par rien | medium | patch | Pré-vérifié ; seuls `outputs_of`/`to_open` sont testés, avec des charges construites à la main. |
| 2 | verification-gap | Le vrai `QgisBackend` n'est vérifié que par la sonde manuelle | low | rejet | Patron établi du dépôt (sondes sous QGIS) ; sonde lancée dans ce build, `RÉSULTAT : OK`. |
| 3 | verification-gap | « Ouvrir le dossier » apparaît désormais pour une étape de plan | false | rejet | Voulu : la Code Map dit que `result_folder` « marchera désormais après un plan ». |
| 4 | edge-case | Un plan qui échoue après une pente réussie n'ouvre pas la pente | low | rejet | Réel : l'erreur de `plan.execute` (`plans.rs`) ne porte pas les sorties du noyau ; rare, et la corriger ajoute une surface au canal. |
| 5 | blind, edge-case | Collision de nom : un raster d'utilisateur `…/1-terrain.slope.tif` pris pour un résultat | false | rejet | Choix figé de l'intention : « reconnaît toujours une couche de résultat raster d'après le nom de fichier de sa source ». |
| 6 | edge-case | Repli sur le nom seul : le modèle recopie un nom que le noyau ne trouve pas | false | rejet | Décision du propriétaire dans l'intention figée (autre lecteur, projet non enregistré). |
| 7 | edge-case | Un `../` est refusé par le relais externe | false | rejet | `..` permis par l'intention ; le refus côté relais est AD-29, voulu. |
| 8 | blind, edge-case | Source virtuelle GDAL (`/vsizip/…`) rendue en faux chemin relatif sous Linux | low | patch | `_flavour` la classe POSIX ; correction directe d'une ligne. |
| 9 | blind | Cas de casse Windows du test indiscernable du repli | low | patch | Attendu `mnt.tif` égal au repli ; correction directe du test. |
| 10 | blind, edge-case | Raster déjà ouvert jamais rafraîchi s'il est réécrit au même chemin | false | rejet | `journal::next_step` continue la numérotation d'une session à l'autre ; le rejeu écrit sous `replay/`. |
| 11 | blind | Chemins absolus répandus dans l'extension, risque vers le modèle ou le relais | false | rejet | L'extension ne renvoie pas les résultats d'étape au noyau ; les références sont résolues dans le noyau (`resolve_references`), déjà sur valeurs brutes avant ce ticket. |
| 12 | blind, intent | La doc dit « chemins de sortie absolus », le code envoie le résultat brut | low | patch | `canal_externe.rs` vérifie `name`/`work` absolus ; correction de texte. |
| 13 | blind, intent | Le modèle n'a jamais vu de `source` raster relative | medium | defer | Hors de ce ticket (« Apprendre quoi que ce soit au modèle » interdit) ; au report pour le ré-entraînement groupé. |
| 14 | blind | Budget d'inventaire non mesuré avec des chemins plus longs | low | rejet | Peu de rasters par projet ; le mesurer ajouterait un test lourd. |
| 15 | blind | Sonde : `QgsMapLayerType` absent de QGIS 4 ; contrôle par sous-chaîne fragile | low | patch | Type : correction directe (`isinstance`) ; chemin 8.3 : rejeté, sonde passée. |
| 16 | blind | `report` écrit une commande `raster.open` hors registre dans le journal du panneau | low | rejet | Aucun lecteur de `entry.command` ne s'y trompe : statut `error`, `error` None, détail `focus_status`. |
| 17 | blind | Petites incohérences du plan (`ticket: '3'`, `2-` vs `1-`) | false | rejet | `ticket` = id d'entrée par convention ; numéro d'exemple illustratif ; correction = éditer le plan. |
| 18 | blind | Rejeu : aucune garde contre l'ouverture d'une sortie de rejeu | false | rejet | Le rejeu arrive par `REPLAY_DONE`, que `outputs_of` ignore. |
| 19 | intent | La sonde passe par la CLI, pas par `serve` | low | rejet | Les chemins `plan.execute` et `external.call` sont couverts par les tests Rust ; le panneau par le correctif 1. |
| 20 | intent | Démo réelle non faite | — | rejet | Contrôle manuel prévu à l'étape de présentation, pas un défaut du code. |
| 21 | intent | Décision sur l'extension `.tif` plutôt que sur le contenu | false | rejet | `isValid()` refuse un fichier qui n'est pas un raster lisible (sonde : GeoTIFF corrompu non ajouté). |

## Design Notes

Pourquoi un `file` absolu plutôt qu'un ordre dicté par le noyau : `command.run` le fait déjà depuis 4.2, le canal de l'extension est local (AD-9, exception) et une seule règle sert les trois chemins. L'ouverture n'est pas journalisée : c'est un affichage, comme le chargement d'une sortie déléguée ; le rejeu refait le fichier, pas la couche.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` (WSL) -- expected: vert
- `python3 -I -m unittest discover -s qgis-plugin/tests` et `python3 qgis-plugin/tests/check_extension.py` -- expected: vert
- PowerShell, QGIS offscreen : `python-qgis-ltr.bat qgis-plugin/tests/probe_raster_output.py` -- expected: `RÉSULTAT : OK`

**Manual checks:**
- Démo réelle (mémoire « démo de bout en bout ») : noyau et extension installés, dans QGIS sur le projet de Villeneuve-le-Roi avec le MNT de `donnees-test-cassini`, lancer `terrain.slope` depuis le panneau : la couche de pente apparaît ; l'inventaire donne le chemin relatif du MNT.
