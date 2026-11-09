---
title: 'Conversion de format (data.convert)'
type: 'feature'
ticket: '9'
created: '2026-11-09'
status: 'built'
baseline_revision: '7aee3b86d468cbda401d8992b3c4131742c002fd'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, adversarial, edge-cases]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'utilisateur ne peut pas écrire une couche dans un autre format (« exporte les gares en shapefile ») : le générateur d'exemples le refuse aujourd'hui (« Aucune commande du catalogue n'exporte une couche vers un fichier »), alors que la fiche `data.convert` est planifiée au catalogue (epic 10, entrée 9).

**Approach:** ajouter `data.convert`, déléguée à QGIS, exactement comme sa fiche : paramètres `layer_id`, `format` (`gpkg`, `shp`, `geojson`), `out` (fourni par le noyau) ; résultat `file`, `format`, `feature_count`. Le noyau déduit l'extension du fichier du `format` (AD-10, AD-24) ; l'original reste intact. Le générateur apprend la commande (famille nouvelle, refus du shapefile retiré) pour le prochain entraînement.

## Boundaries & Constraints

**Always:** paramètres et résultat exactement ceux de la fiche, passée en `faite` avec le §4 dans le même commit ; le nom et l'extension du fichier de sortie viennent du noyau ; journal et rejeu par le chemin générique existant ; tests de l'extension par fond doublure, plus une sonde sous le vrai QGIS 3.44 ; la couche d'origine n'est jamais modifiée (§2.6).

**Never:** changer le modèle installé ou lancer un entraînement ; ajouter un format hors de la fiche (Excel, CSV restent refusés) ; toucher au rendu ou au style.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| GeoJSON → GeoPackage | couche GeoJSON, `format: gpkg` | fichier `.gpkg` au nom dicté par le noyau, `feature_count` égal | — |
| GeoJSON → Shapefile | `format: shp` | `.shp` et ses fichiers compagnons écrits, original intact | — |
| Format inconnu | `format: xlsx` | refus du noyau avant QGIS | erreur de paramètres |
| Couche absente | `layer_id` inconnu | erreur nommée du geste | `layer.not_found` (correction du propriétaire, 2026-11-09 : le code de toutes les analyses) |
| Rejeu | journal avec conversion | rejoué identique (fichier existe, non vide, même résultat typé) | — |
| Plan | « convertis les gares en GeoPackage » | plan valide à une étape `data.convert` | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/data_reproject.rs` -- modèle exact d'une commande déléguée (`impl Delegated`, `.runs_in(RunsIn::Qgis).cancellable(true).output_params(&["out"])`, exemples, `.tests`, `inventory::submit!`) ; nouveau fichier `data_convert.rs` à côté (modules déclarés par `build.rs`).
- `core/src/registry.rs` l. ~57-61, 169-176 (`output_params`, `output_ext` statique) -- ajouter une extension tirée d'un paramètre énuméré (ex. `output_ext_from("format", &[("gpkg", ".gpkg"), ("shp", ".shp"), ("geojson", ".geojson")])`) et une méthode qui rend l'extension pour des paramètres donnés ; l'extension statique reste le défaut des autres commandes.
- `core/src/journal/mod.rs` -- `output_file(step, id, outputs, name, ext)` l. ~759, appelée l. ~587, 833, 872, 906, 1383 avec `spec.output_ext` : passer par la nouvelle méthode avec les paramètres de l'appel. Un Shapefile écrit des compagnons (`.shx`, `.dbf`, `.prj`, `.cpg`) : vérifier le nettoyage des sorties intermédiaires de plan (`intermediate_files`, ticket 5.34) et l'emplacement des sorties du rejeu, pour ne laisser aucun compagnon orphelin.
- `core/src/commands/layout_export_pdf.rs` -- forme d'un résultat `file` (chemin rendu au panneau et au journal, AD-9).
- `qgis-plugin/cassini_qgis/analysis.py` -- table `ALGORITHMS` (ligne `data.reproject` l. ~272) : ligne `data.convert` sur l'algorithme de sauvegarde de QGIS (`native:savefeatures`), sortie au chemin dicté ; le fichier n'est pas ajouté au projet (le résultat de la fiche n'a pas de `layer_id`) ; résultat `file`, `format`, `feature_count`.
- `qgis-plugin/cassini_qgis/summary.py` (`_SENTENCES`), `texts.py` FR/EN, `tests/test_summary.py` -- phrase « Couche « gares » convertie en GeoPackage : fichier …, N entités ».
- `qgis-plugin/tests/test_analysis.py` -- fond doublure, matrice ; `qgis-plugin/tests/probe_*.py` -- forme d'une sonde sous le vrai QGIS.
- `training/src/families.rs` l. ~2787 -- refus « Exporte la couche … en shapefile » à retirer (garder le refus Excel l. ~2788) ; famille nouvelle `conversion_format` sur le modèle de `reprojection_couche` ; garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry`.
- `docs/entrainement-du-modele.md` -- tableau des familles.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` l. ~372 (§4) et ~1605 (fiche) ; `core/tests/catalogue.rs` les confronte au registre.
- `ai/tests/` (`common/mod.rs`, plans attendus) -- un plan `data.convert` valide passe la validation du planificateur.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/registry.rs` + `core/src/journal/mod.rs` -- extension tirée du `format`, tests (nom du fichier par format, compagnons du Shapefile au nettoyage).
- [x] `core/src/commands/data_convert.rs` + catalogue -- commande, fiche et §4 en `faite`.
- [x] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` + tests -- geste, résumé, matrice.
- [x] `training/src/families.rs`, `docs/entrainement-du-modele.md` -- famille, refus retiré, garde-fou.
- [x] Sonde QGIS 3.44 (copie du projet Annecy, archivée) : gares en GeoPackage puis en Shapefile, fichiers ouverts à part dans QGIS, original intact, rejeu identique.

**Acceptance Criteria:**
- Given QGIS 3.44 et le projet Annecy, when on convertit les gares en Shapefile, then le fichier `.shp` dicté par le noyau s'ouvre dans QGIS avec le même nombre d'entités, et le fichier d'origine n'a pas changé.
- Given le dépôt, when `cargo test --workspace` et les tests de l'extension, then tout passe.

## Implementation Notes

- 2026-11-09 (construction) : `native:savefeatures` échoue sous QGIS 3.44 sur les noms que dicte le noyau (`2-data.convert.gpkg` : le pilote est cherché d'après `convert.gpkg`, « OGR driver for '' not found ») ; la ligne de table emploie `native:extractbyexpression` sur `TRUE`, copie de toutes les entités par le puits de sortie, qui choisit le pilote d'après l'extension. Couche absente : le geste générique rend `layer.not_found` (comme `data.reproject` et les analyses), pas `qgis.layer_missing`, propre aux gestes de style. Le repère `out:<k>.<n>` ne désigne pas une sortie de `data.convert` (son extension dépend de ses paramètres, que le repère n'a pas). La nouvelle famille décale le tirage : un gabarit de `pente_seuil` sortait une phrase des essais réels (Z01), réécrit « du MNT ».
- 2026-11-09 : le ticket est rangé après l'entrée 22 (`analysis.area`), qui n'est pas construite. Cet ordre est un choix de séquence, sans dépendance technique ; le propriétaire a demandé `data.convert` maintenant, pour qu'il entre dans le prochain entraînement.

## Review Triage Log

Boucle 0 : relecture quick, puis thorough (adversarial, edge-cases) parce que le changement touche le rejeu et le nommage des sorties (CLAUDE.md §8). Corrigés 12, rejetés 6, reportés 1.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Matrice : `qgis.layer_missing` au lieu de `layer.not_found` | low | décision du propriétaire | matrice corrigée (le code de toutes les analyses) |
| Relecture trop légère pour un changement du rejeu | medium | appliqué | risque relevé à `high`, relecture thorough |
| Compagnons du Shapefile différents entre noyau et extension (`.qix`) | low | patch | listes alignées, catalogue compris |
| Test des compagnons cassé par l'ajout de `.qix` ; `.qix` jamais écrit par les doublures | medium | patch | attente et doublures complétées |
| Limite du repère `out:` muette au registre | low | patch | note sur `output_ext_from` |
| Table sans géométrie ou géométrie mixte vers Shapefile : erreur tardive trompeuse | medium | patch | refus nommé avant le lancement |
| Format réellement écrit jamais vérifié | medium | patch | type de stockage vérifié à la relecture du fichier |
| Encodage du Shapefile laissé au système (cp1252) | medium | patch | UTF-8 imposé pour cette commande, sonde avec accents |
| Pertes du Shapefile et système du GeoJSON non dits | medium | patch | descriptions, fiche, docstring |
| `plan_reel` attend encore un refus pour S09 | medium | patch | plan `data.convert` attendu, comme S05 |
| Commentaire du générateur qui cite une phrase tenue à l'écart | medium | patch | citation retirée |
| `fid` non entier vers GeoPackage | low | rejet | cas rare, erreur de QGIS rendue telle quelle |
| `feature_count` lu dans le fichier écrit | low | rejet | c'est ce que l'utilisateur obtient |
| Rejeu qui n'exige pas les compagnons | low | rejet | le format vérifié à l'écriture couvre le cas |
| `output_ext` et `output_ext_from` ensemble, ouvreur | low | rejet | aucune commande du noyau ne le fait ; noté au registre |
| Casse des extensions | low | rejet | le noyau n'écrit que des minuscules |
| `-wal`/`-shm` d'un GeoPackage annulé | low | rejet | antérieur au changement |
| `step:k.file` passé à `layer.add` contourne `replay/` | low | report | antérieur, non atteint par le générateur |

## Verification

**Commands:**
- Rust sous Windows seulement : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` (de même pour `cargo fmt --all -- --check` et `cargo clippy --workspace --all-targets -- -D warnings`) -- expected: vert
- `python3 -I -m unittest discover -s qgis-plugin/tests` et `python3 -I qgis-plugin/tests/check_extension.py` -- expected: vert

**Manual checks (if no CLI):**
- Sonde : noyau `cargo build --release -p cassini --features gpu` (même environnement, plus `VULKAN_SDK`), installé dans `%LOCALAPPDATA%\Cassini\bin\` en gardant l'ancien à côté ; extension copiée dans le profil QGIS ; script PyQGIS sous le Python de QGIS 3.44 (`QT_QPA_PLATFORM=offscreen`) sur une copie du projet Annecy de `../donnees-test-cassini/`, archivée dans `_archives/sondes/` (modèle : `_archives/sondes/projet-sonde-fond-noir-2-rendus/sonde.py`).
