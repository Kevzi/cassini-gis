---
title: 'Modele ajuste, tache 4a : inventaire du produit au format appris'
type: 'feature'
ticket: '10'
created: '2026-10-22'
status: 'built'
baseline_revision: 'ab6171b0edd1c2c3d3675679948b29682228d40e'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['quick', 'blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/docs/entrainement-du-modele.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le modèle ajusté a appris sur des inventaires qui portent, par couche, la sorte, la géométrie, le SCR, le nombre d'objets et les champs (type, et pour les champs texte peu variés, leurs valeurs). Dans QGIS, `project.inventory` ne rend que l'id, le nom, la source et le SCR : le modèle ne voit aucun champ (`typezone` du PLU), et la démo échoue pour une raison étrangère à sa qualité.

**Approach:** Tâche 4a du ticket 5.10 (découpe du propriétaire le 2026-10-22 ; 4b, mesures et démo, dans `deferred-work.md`). L'extension rapporte en plus les faits bruts de QGIS (sorte, géométrie WKB, nombre d'objets, champs avec leur type QGIS, valeurs distinctes des champs texte peu variés, nom du projet). Le noyau, seul à décider, les traduit dans la forme exacte de l'entraînement avant de composer le plan ; un test compare cette forme à celle du générateur.

## Boundaries & Constraints

**Always:** Ajouts seulement à la sortie de `project.inventory` : les clés actuelles restent, avec le même sens (rejeu, résumé du panneau, MCP). La traduction (type QGIS → `string`/`integer`/`number`, `EPSG:2154` → `2154`, forme `{project, layers, unloaded_files}`) vit dans le noyau, en Rust, testée sans QGIS ; l'extension ne fait que lire (CLAUDE.md §2.7). Valeurs : champs texte seulement, au plus 10 valeurs distinctes non nulles, triées ; au-delà, aucune clé `values`. Couche raster : `kind: "raster"`, `geometry: null`, `fields: []`. `unloaded_files` vaut `[]` : l'extension ne parcourt pas le disque. Accès à Qt par `qgis.PyQt` seulement. Aucune dépendance ajoutée.

**Never:** Toucher le prompt court, `SHORT_SYSTEM`, `training/src/prompt.rs`, le générateur ou ses familles (ré-entraînement). Toucher `ai/tests/tenu_a_l_ecart/` ou `tests/epreuve-modele-local/`. Changer `cassini plan --inventory`, qui prend déjà un inventaire au format appris. Lire ou modifier un fichier source de l'utilisateur au-delà de la lecture des valeurs. Faire les mesures ou la démo de la tâche 4b.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Couche vecteur | PLU, `typezone` texte (U, N, Nh), `surface` réel | `{kind:"vector", geometry:"MultiPolygon", epsg:2154, feature_count, fields:[{name:"typezone",type:"string",values:["N","Nh","U"]},{name:"surface",type:"number"}], …}` | — |
| Champ texte très varié | `id_parcelle`, 5 000 valeurs | champ sans `values` | — |
| Types QGIS | `Integer`, `Integer64`, `Real`, `Double`, `String`, `Date` | `integer`, `integer`, `number`, `number`, `string`, `string` | type inconnu → `string` |
| SCR non EPSG | `IGNF:LAMB93` | clé `epsg` absente | — |
| Raster | MNT | `kind:"raster"`, `geometry:null`, `fields:[]` | — |
| Ancienne extension | sortie sans les nouvelles clés | traduction sans `fields` ni `geometry`, pas d'erreur | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/project_inventory.rs` -- `InventoryLayer` (l.28), `ProjectInventoryOutput` (l.41) : ajouter des champs facultatifs (`kind`, `geometry`, `feature_count`, `fields[{name, type, values?}]`, et `project_name` en tête). Spec et exemple inchangés.
- `core/src/serve/plans.rs` l.108-120 -- fermeture `inventory` : appliquer la traduction au résultat de `project.inventory` avant `composer.compose`.
- Nouveau `core/src/agent_inventory.rs` (déclaré dans `core/src/lib.rs`) -- `pub fn agent_inventory(&Value) -> Value` : forme de l'entraînement.
- Forme visée, relevée sur le jeu engendré (graine 1) : clés de couche `epsg, feature_count, fields, geometry, kind, layer_id, name, source` ; `project: {name, epsg}` ; `unloaded_files` présent (vide dans 1 604 exemples sur 5 000) ; types `string|integer|number` ; valeurs seulement sur des champs `string`, 2 à 9. `serde_json::Value` trie les clés : même ordre que l'entraînement.
- `training/src/inventories.rs` -- forme produite par le générateur ; lecture seule.
- `qgis-plugin/cassini_qgis/inventory.py` -- `QgisBackend` (l.26) : ajouter `wkb_name` (`QgsWkbTypes.displayString(layer.wkbType())`), `text_values(layer, name, limit)` (`layer.uniqueValues(idx, limit + 1)`), `project_name` (`QgsProject.instance().baseName()`) ; `ProjectInventoryGesture._read` (l.173) les rapporte. `fields_of` et `feature_count` existent (l.82, l.85), à réutiliser.
- `qgis-plugin/tests/test_inventory.py` -- doublure de fond (l.14-60) à étendre.
- `qgis-plugin/cassini_qgis/summary.py` l.256 -- résumé de l'inventaire : ne pas changer, vérifier qu'il tolère les clés ajoutées.
- `core/src/serve.rs` l.1871-1936 -- tests qui jouent `project.inventory` dans le canal : modèle des nouveaux tests de bout en bout du noyau.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/project_inventory.rs` -- champs facultatifs ajoutés, documentés -- le contrat public grandit sans rien casser
- [x] `core/src/agent_inventory.rs`, `core/src/lib.rs` -- traduction et tests unitaires de toute la matrice -- la logique dans le noyau
- [x] `core/src/serve/plans.rs`, `core/src/serve.rs` -- traduction branchée ; test qu'un `plan.propose` transmet au composeur l'inventaire traduit -- le modèle voit la forme apprise
- [x] `qgis-plugin/cassini_qgis/inventory.py`, `qgis-plugin/tests/test_inventory.py` -- faits bruts rapportés ; tests avec la doublure (valeurs plafonnées, raster, champ sans valeurs) -- l'extension lit, ne décide pas
- [x] `training/tests/forme_inventaire.rs` -- une sortie de `project.inventory` représentative, traduite par `agent_inventory`, a exactement les clés et les sortes de valeurs du jeu engendré (graine 1) -- garde-fou contre la dérive
- [x] `docs/entrainement-du-modele.md` -- section « L'inventaire vu par le modèle dans QGIS » -- consigné

**Acceptance Criteria:**
- Given le projet réel `projet-test-villeneuve.qgz` ouvert dans QGIS, when une sonde jetable demande `project.inventory` par le vrai noyau et applique la traduction, then la couche du PLU porte `typezone` avec ses valeurs, et la taille de l'inventaire traduit (en jetons du GGUF de base) est notée dans ce plan.
- Given la recette complète, when elle tourne, then elle est verte (Rust, Python de l'extension, contrôle de l'extension).

## Implementation Notes

- Acceptation sur le projet réel (2026-10-23) : sonde jetable hors dépôt sous le Python de QGIS 3.44.15 (plateforme offscreen), geste `ProjectInventoryGesture` avec le vrai `QgisBackend` sur `projet-test-villeneuve.qgz`, puis traduction par `cassini_core::agent_inventory::agent_inventory` (petit binaire jetable qui dépend du crate `core` du dépôt). Six couches (`mnt` raster, `gares`, `routes`, `batiments`, `parcelles`, `plu_zones`) ; `plu_zones` porte `{"name":"typezone","type":"string","values":["N","Nh","U"]}`, `id` et `libelle` (11 valeurs ou plus) n'ont pas de `values`.
- **Taille de l'inventaire traduit : 2 439 jetons** du GGUF de base (`qwen3.5-4b-base-Q4_K_M.gguf`, `llama-tokenize` du commit épinglé, sans BOS), 5 289 caractères ; sortie brute compacte : 3 196 jetons. C'est plus que la plus longue séquence entière de l'entraînement (1 326 jetons) : `plu_zones` seule fait 2 890 caractères (24 champs, empreintes `gpu_doc_id`, libellés longs). Tient dans `N_CTX` (8 192), mais hors de la longueur apprise : risque noté pour la tâche 4b dans `deferred-work.md`.
- Raster : pas de `feature_count` (QGIS n'en a pas pour un raster) ; seul écart de forme avec le générateur, qui en tire un. Le garde-fou `training/tests/forme_inventaire.rs` exige les clés exactes du jeu pour une couche vecteur, et ces clés moins `feature_count` pour un raster.
- Lecture des valeurs dans l'extension : `text_values` saute les champs numériques et les dates (`isNumeric`, `isDateOrTime`), lit `uniqueValues(idx, limit + 2)` (une place pour la valeur nulle, une pour le texte vide) avec `limit = 11`, puis `_distinct_texts` garde les textes non vides, dédoublonnés, triés, une de plus que le plafond du noyau. Le noyau décide seul : valeurs gardées pour un type texte (`String`, `text`, `varchar`…), non vides, dédoublonnées, triées, 1 à 10. Une lecture qui lève garde le champ sans valeurs, sans faire échouer l'inventaire.
- Géométrie : le nom WKB est ramené aux six géométries de l'entraînement (suffixes `Z`, `M`, `ZM`, `25D` ôtés ; courbes vers leur équivalent linéaire) ; `NoGeometry` ou inconnu donne `null`. Une sorte `other` (maillage, nuage de points) n'a pas de clé `kind`.

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : quick, blind-hunter, edge-case-hunter, verification-gap, intent-alignment ; CLAUDE.md §8, risque high). 0 high, 6 medium, 8 low, 2 false ; aucun intent_gap ni bad_plan.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch | `text_values` : un seul emplacement de plus pour NULL et `""` ; avec `""` et 11 valeurs ou plus, le noyau ôte `""` et garde 10 valeurs comme une liste complète (4 lentilles). Filtre et coupe dans une fonction pure, lecture de `limit + 2`, test. |
| 2 | medium | patch | Lecteurs réels de `QgisBackend` (`kind_of`, `wkb_name`, `text_values`, `project_name`) sans test au dépôt ; la doublure applique la coupe elle-même et rend des doublons. Sonde `probe_inventory.py` versionnée, doublure à valeurs distinctes. |
| 3 | low | patch | `field_type`/`is_text` : `tinyint`, `mediumint`, `int16`, `int32`, `serial`, `bigserial`, `longlong` deviennent `string` ; `bpchar`, `citext` perdent leurs valeurs. Noms ajoutés. |
| 4 | low | patch | Branche `kind: "other"` sans test (verification-gap). Test ajouté. |
| 5 | low | patch | Deux lignes vides dans la classe avant `_facts`, ligne trop longue dans la docstring. Corrigé. |
| 6 | low | patch | Champ booléen : l'extension envoie `values: []`, contraire à la doc du contrat. La fonction pure rend `None`. |
| 7 | medium | defer | Ordre des valeurs : le générateur garde l'ordre de définition du thème, le produit trie (contrainte gelée « triées ») ; QGIS n'a pas d'ordre canonique. À aligner au prochain entraînement (générateur trié). |
| 8 | medium | defer | Formes jamais vues : couche sans fichier (sorties temporaires des étapes précédentes, sans `source`), projet non enregistré, SCR non EPSG, table sans géométrie, `unloaded_files` toujours vide (68 % des exemples en ont). À mesurer en 4b, à apprendre au prochain entraînement. |
| 9 | medium | defer | Durée de l'inventaire : un `uniqueValues` par champ texte sur le fil de QGIS, à chaque `plan.propose`, non interruptible ; non mesurée (maybe-false sur la gravité). Chronométrer en 4b sur le vrai projet. |
| 10 | medium | defer | Critère 1 « par le vrai noyau » : la sonde a appelé le geste puis `agent_inventory`, sans passer par `cassini serve` ni `plan.propose`. La démo de 4b passe par toute la chaîne avec le modèle. |
| 11 | low | defer | Aucune borne de longueur des valeurs ni du nombre de champs : rejoint l'entrée de taille (2 439 jetons) déjà notée pour 4b. |
| 12 | low | rejeté | `feature_count` vaut 0 quand QGIS ne sait pas compter : fournisseurs de fichiers du MVP (OGR) comptent toujours ; garde non démontrée. |
| 13 | low | rejeté | Seule la lecture des valeurs est protégée : `kind_of`, `wkb_name`, `fields_of` n'ont pas d'échec démontré sur une couche valide. |
| 14 | low | rejeté | Une seule valeur gardée (l'entraînement montre 2 à 9) : une valeur unique informe le modèle ; écart sans dommage nommé. |
| 15 | low | rejeté | Sortie malformée de `project.inventory` traduite en inventaire vide : l'extension est la nôtre, aucun chemin démontré. |
| 16 | false | rejeté | Empreinte du registre changée : `registry_version` n'est qu'écrite au journal (`journal/mod.rs` l.174, l.269), jamais comparée au rejeu ; changer avec le contrat est son rôle. |
| 17 | false | rejeté | `lenses_ran` sans triage : la relecture était en cours, ce tableau la consigne. |

## Design Notes

Pourquoi traduire dans le noyau plutôt que faire écrire la forme par l'extension : la forme est celle d'un apprentissage, elle bouge avec un ré-entraînement, et un test Rust la confronte au générateur ; l'extension, elle, garde un contrat stable de faits QGIS. Le plafond de 10 valeurs suit le jeu tenu à l'écart (10 au plus) et l'entraînement (9 au plus) : une liste plus longue n'a jamais été vue.

## Verification

Environnement : `core` et `training` se testent sous WSL ; ce qui compile `ai` (et `cargo test --workspace`) passe par PowerShell (`LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`). QGIS 3.44.15 : `C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat`.

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check`, `cargo deny check` (PowerShell) -- expected: succès
- `python3 -I -m unittest discover -s qgis-plugin/tests` et le contrôle de l'extension de la recette -- expected: succès
- Sonde jetable sous le Python de QGIS (hors dépôt, plateforme offscreen) sur `projet-test-villeneuve.qgz` -- expected: inventaire traduit avec les champs du PLU
