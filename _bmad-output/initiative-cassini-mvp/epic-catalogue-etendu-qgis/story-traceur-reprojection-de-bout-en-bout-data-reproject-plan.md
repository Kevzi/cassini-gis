---
title: 'Reprojection d''une couche par QGIS (data.reproject)'
type: 'feature'
ticket: '7'
created: '2026-11-08'
status: 'built'
baseline_revision: '10a929646effea78949c5d2a677c63a14e9b7837'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** une couche en degrés (les gares d'Annecy en EPSG:4326) ne passe aucune analyse métrique (`analysis.crs_not_metric`) et `data.reproject` n'est que planifiée : la phrase de démo transposée à Annecy échoue, et le générateur apprend encore au modèle qu'« aucune commande ne reprojette une couche ».

**Approach:** déclarer au registre la commande déléguée `data.reproject` exactement comme sa fiche figée du catalogue, l'exécuter par une ligne de la table fermée `ALGORITHMS` (`native:reprojectlayer`, sortie `.gpkg` nommée par le noyau), et remplacer dans le générateur le refus de reprojection de couche par une famille de plans ; le ré-entraînement reste groupé dans 11.23.

## Boundaries & Constraints

**Always:** paramètres `layer_id`, `target_epsg` (entier), `out` (fourni par le noyau) et résultat `layer_id`, `name`, `crs`, `feature_count`, exactement ceux de la fiche ; `data.unknown_crs` (message en français qui nomme l'EPSG fautif) avant tout lancement quand QGIS ne connaît pas le code cible ; la reprojection est celle de QGIS (contexte de transformation du projet), jamais du noyau (AD-19) ; la couche d'origine n'est jamais modifiée (§2.6) ; fiche et §4 du catalogue passés en `faite` dans le même commit ; tests de l'extension par fond doublure, plus une sonde sous le vrai QGIS 3.44 ; journal et rejeu par le chemin générique existant.

**Never:** les mises à niveau 10.1 à 10.6 (version de chaîne, annonce fidèle, résultat validé par le noyau, prompt filtré) : dérogation décidée par le propriétaire le 2026-11-08, notée dans `deferred-work.md` ; aucune reprojection insérée en silence par le noyau avant une analyse ; pas de ré-entraînement (11.23) ; aucune dépendance nouvelle ; `ai/`, `tests/epreuve-modele-local/` non modifiés.

**Decisions (propriétaire, 2026-11-08):** construire `data.reproject` maintenant, avant 10.1-10.6 et avant les epics 6 et 7 ; l'ordre de l'epic 10 sera remis d'aplomb par `bmad-ticket` ensuite. Le générateur reçoit deux familles : la reprojection demandée (`reprojection_couche`) et l'enchaînement « analyse métrique sur une couche en degrés → `data.reproject` puis l'analyse sur `step:1.layer_id` » (`reprojection_avant_analyse`), cible = système des autres couches vecteur en mètres de l'inventaire, EPSG:2154 à défaut. Vérification : sonde directe sous QGIS et rejeu après réouverture maintenant ; la phrase « Reprojette la couche des gares en WGS84 » est essayée dans le panneau avec le modèle installé et consignée telle quelle, jugée pour de bon après 11.23. Plan gardé entier (~2 200 tokens).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| 4326 → 2154 | couche de points 4326, `target_epsg: 2154` | nouvelle couche `.gpkg` ajoutée, `crs: EPSG:2154`, même nombre d'entités | — |
| Même système | couche 2154, `target_epsg: 2154` | nouvelle couche copiée, `crs: EPSG:2154` | — |
| Cible inconnue | `target_epsg: 99999` | rien lancé, aucun fichier | `data.unknown_crs`, nomme EPSG:99999 |
| Couche absente | `layer_id` inconnu | rien lancé | `layer.not_found` |
| Raster | `layer_id` d'un MNT | rien lancé | `layer.not_vector` |
| Paramètres invalides | `target_epsg` absent, négatif, non entier ; champ en trop | — | `registry.invalid_params` (noyau) |
| Annulation | annulé en cours | aucun fichier partiel | `qgis.cancelled` |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_intersect.rs` -- modèle exact d'une commande déléguée à sortie couche : `impl Delegated`, `.runs_in(RunsIn::Qgis).cancellable(true).output_params(&["out"]).output_ext(".gpkg")`, exemples, `.tests(&[..])`, `inventory::submit!`. Modules déclarés par `build.rs` : ajouter le fichier suffit.
- `core/src/commands/data_transform_point.rs` -- doc de `data.unknown_crs` à reprendre ; descriptions FR/EN à distinguer (point seul vs couche entière).
- `core/tests/catalogue.rs` -- échoue tant que la fiche reste `planifiee` alors que la commande est au registre.
- `qgis-plugin/cassini_qgis/analysis.py` -- table `ALGORITHMS` l. 197 et `ProcessingGesture._begin` l. 427 : ajouter une ligne `data.reproject` (`native:reprojectlayer`, `INPUT`, `TARGET_CRS` = `"EPSG:%d"`, `CONVERT_CURVED_GEOMETRIES: False`, `OUTPUT`) et une clé de table facultative (ex. `target_crs: "target_epsg"`) que `_begin` vérifie par une méthode du fond (`crs_is_known(epsg)`, seul accès à `qgis.core`) avant `translate`. Le message de `CRS_NOT_METRIC` l. 444 peut nommer la reprojection de couche. `register_all` inscrit la ligne : rien à toucher dans `plugin.py`.
- `qgis-plugin/tests/test_analysis.py` -- fond doublure existant, à étendre pour la matrice.
- `qgis-plugin/tests/probe_intersect_select.py` -- forme d'une sonde d'analyse sous le vrai QGIS, à reprendre dans `probe_reproject.py`.
- `qgis-plugin/cassini_qgis/summary.py` l. 479 (`_SENTENCES`), `texts.py` (FR l. ~144, EN l. ~311), `tests/test_summary.py` -- phrase « Couche « gares » reprojetée en EPSG:2154 : couche « … », N entités ».
- `training/src/families.rs` l. 2721-2722 -- refus « Aucune commande du catalogue ne reprojette une couche » à retirer ; garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry` l. 6539 à étendre (sur le modèle de `reprojects_no_point` l. 5517) ; famille nouvelle déclarée près de `reprojection_point` l. 222 ; couches tirées avec `epsg` (`inventories.rs` l. 1374).
- `docs/entrainement-du-modele.md` l. 104 et 186 -- tableau des familles et paragraphe « refus de reprojection d'une couche ».
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` l. 351 et 1495 -- fiche et §4.
- `cli/tests/mcp.rs` l. 67 -- une commande déléguée n'est pas servie sans QGIS : rien à changer.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/data_reproject.rs` -- params, résultat, spec déléguée, deux exemples, tests (déléguée → `registry.unavailable` sans QGIS ; matrice des paramètres invalides ; sortie `.gpkg` nommée par le noyau).
- [x] `qgis-plugin/cassini_qgis/analysis.py` -- ligne de table, traduction, contrôle `data.unknown_crs` par le fond.
- [x] `qgis-plugin/tests/test_analysis.py` -- cas de la matrice par fond doublure.
- [x] `qgis-plugin/tests/probe_reproject.py` -- sonde : couche de points 4326 → 2154, nombre d'entités conservé, coordonnée d'un point contrôlée (origine du Lambert-93), code inconnu, original inchangé.
- [x] `qgis-plugin/cassini_qgis/summary.py`, `texts.py`, `tests/test_summary.py` -- phrase FR/EN.
- [x] `training/src/families.rs` -- refus retiré, familles `reprojection_couche` et `reprojection_avant_analyse` (couche vecteur tirée en 4326, cible tirée de l'inventaire), garde-fou étendu, tests de famille (`check_plan` passe, la seconde étape cite `step:1.layer_id`).
- [x] `docs/entrainement-du-modele.md`, `qgis-plugin/README.md`, catalogue -- à jour ; `deferred-work.md` -- dérogation 10.1-10.6 consignée.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when `cargo test --workspace`, then la recette du catalogue passe avec la fiche `faite` et le garde-fou du générateur refuse tout refus « ne reprojette une couche ».
- Given QGIS 3.44 et l'extension installée, when la sonde reprojette une couche 4326 en 2154, then la couche ajoutée est en EPSG:2154 avec le même nombre d'entités, le point (3 ; 46,5) tombe à 1 cm de (700000 ; 6600000), et le fichier d'origine est inchangé.
- Given un plan `data.reproject` exécuté puis le projet fermé et rouvert, when on rejoue le journal, then la couche reprojetée est reproduite à l'identique.

## Implementation Notes

- Fichiers : `core/src/commands/data_reproject.rs` (nouveau, `target_epsg` ≥ 1 au schéma) ;
  fiche et §4 du catalogue en `faite` ; `qgis-plugin/cassini_qgis/analysis.py` (ligne
  `native:reprojectlayer`, clé `target_crs`, `QgisBackend.crs_is_known`), `summary.py`,
  `texts.py` ; tests `test_analysis.py`, `test_summary.py` ; sonde `probe_reproject.py` ;
  `training/src/families.rs` et `inventories.rs` (refus retiré, familles
  `reprojection_couche` et `reprojection_avant_analyse`, garde-fou `reprojects_no_layer`) ;
  `docs/entrainement-du-modele.md`, `qgis-plugin/README.md`, `deferred-work.md`.
- Sonde sous QGIS 3.44.15 : `RÉSULTAT : OK` — le point (3 ; 46,5) tombe à moins de 1 cm de
  (700000 ; 6600000), et le rejeu après réouverture rend `identical: true`.
- La sonde se lance avec `CASSINI_CORE=C:\ct\debug\cassini.exe` : les binaires de
  `target/debug` et `target/release` sont anciens et parlent le protocole 1.
- Reste à faire : l'essai dans le panneau de « Reprojette la couche des gares en WGS84 » avec le
  modèle installé, une fois le noyau et l'extension installés (jugé pour de bon après 11.23).

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 0, medium 1, low 6, false 1, maybe-false 0 ; rejetés : 3 low, 1 false.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Essai du panneau et passage de la sonde non consignés ; Implementation Notes vides | medium | patch | Notes ajoutées (sonde 3.44.15 OK, rejeu identique) ; l'essai du panneau se fait après installation, avant le commit |
| Message `analysis.crs_not_metric` qui propose une phrase que le modèle installé refuse, cible 2154 figée | low | patch | message rendu à sa forme d'origine, test renommé |
| `target_epsg: 0` admis par le noyau, rejeté seulement par l'extension ; `analysis.failed` absent de la fiche | low | patch | `deserialize_with` qui refuse 0 (forme de `coord_convert.rs`), cas ajouté au test, fiche complétée |
| Commentaire de l'annulation de la sonde inexact | low | patch | `start()` ne fait que programmer `_launch` (minuterie à 0) : le commentaire dit maintenant ce qui est testé |
| `_reproject` affiche le code demandé sans le comparer au `crs` rendu | low | rejeté | QGIS écrit la couche dans `TARGET_CRS` ; écart improbable, la correction ajoute une branche |
| `test_summary` cite `step:1` au lieu de `step:1.layer_id` | low | rejeté | `_STEP_REF` accepte les deux formes ; aucun effet pour l'utilisateur |
| Pas de nouveau chiffre de part de `redemande` dans la doc | low | rejeté | chiffre de documentation, à recalculer au ré-entraînement groupé 11.23 |
| `reprojection_couche` ne tire pas toujours la couche en 4326 | false | rejeté | la décision gelée ne fixe 4326 que pour `reprojection_avant_analyse` ; la doc dit « dans un autre système que la cible » |

## Design Notes

`native:reprojectlayer` (doc QGIS 3.44, Vector general §24.1.25.25) : `INPUT`, `TARGET_CRS` (crs), `CONVERT_CURVED_GEOMETRIES` (booléen, depuis 3.32), `OPERATION` (facultatif, laissé vide : les réglages de transformation du projet s'appliquent), `OUTPUT`. Référence de la sonde : origine du Lambert-93, E = 700 000 m, N = 6 600 000 m (déjà utilisée par 11.4).

## Verification

**Commands:** Rust sous Windows seulement (WSL n'a pas libclang) : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` (de même pour fmt et clippy) ; Python de l'extension par le Python de QGIS ou `python3 -I`.
- `cargo test --workspace` -- expected: tout vert, dont `catalogue` et `training`
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert

**Manual checks (if no CLI):**
- `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_reproject.py` -- expected: toutes les vérifications passent.
