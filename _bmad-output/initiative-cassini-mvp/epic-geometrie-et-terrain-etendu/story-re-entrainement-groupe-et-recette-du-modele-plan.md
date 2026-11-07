---
title: 'Ré-entraînement groupé et recette du modèle'
type: 'feature'
ticket: '16'
created: '2026-11-07'
status: 'in-progress'
baseline_revision: '01ed24f4c2e7c3841df6f02174d7fe707d1049d5'
route: 'full'
route_source: 'auto'
risk: 'high'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le modèle installé (5.41 n° 2) n'a vu aucune des quatorze commandes de l'epic 11 : il refuse ou rate la conversion MGRS, les mesures géodésiques, la reprojection d'un point, l'exposition, l'ombrage, le champ de vision cumulé, les points hauts, le reclassement, la superposition pondérée, le coût et le chemin de moindre coût, et l'export au format carroyé. Le générateur les enseigne depuis 11.12 (cinq lots) ; `plan_reel` attend encore un refus pour S25, que le modèle doit désormais planifier.

**Approach:** Comme en 5.41 : d'abord mettre à jour l'attendu de S25 et ajouter un essai réel qui couvre chaque nouvelle commande, puis, au feu vert du propriétaire, ré-entraîner par la commande unique (même base Qwen3.5-4B, même méthode, GPU), installer en gardant 5.41 sous son nom, mesurer sur GPU, et finir par une recette en phrases enchaînées dans une seule session QGIS, rejeu compris.

**Décisions (propriétaire, 2026-11-07) :** (1) plan gardé entier (~2 400 jetons), comme 5.41. (2) Barres de non-recul = barres de 5.41 : `plan_reel` ≥ 23/25, `question_reel` ≥ 11/12, `ecart_reel` ≥ 32/40, `echelle_utm_reel` ≥ 7/8, `terrain_reel` ≥ 7/8. (3) Claude fabrique une fois, depuis le MNT 2154, un raster de coût, deux critères notés et un masque, hors du dépôt dans `../donnees-test-cassini/` (dérivés, rien de téléchargé), et les ajoute à la copie du projet de recette : la recette couvre aussi coût, chemin de moindre coût et superposition pondérée.

## Boundaries & Constraints

**Always:** `tests/epreuve-modele-local/` (phrases et inventaire) est lu tel quel, jamais modifié : seuls les attendus de `ai/tests/plan_reel.rs` changent. Essais réels avec `--features cuda`. Le modèle 5.41 reste installé sous `cassini-composition-4b-5-41`. Une phrase mal comprise devient un constat consigné (et, sur décision du propriétaire, un exemple du générateur), jamais une règle de prompt. Vocabulaire civil (§2.9).

**Never:** lancer l'entraînement sans le feu vert du propriétaire (sa carte graphique est prise environ 5 h). Changer de branche dans le dépôt pendant l'entraînement. Toucher au générateur avant le premier entraînement. Changer la base, la méthode, `DEFAULT_COUNT` ou `DEFAULT_SEED`. Modifier le noyau, le prompt ou l'extension.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| S25 | « … depuis le point 2,4150 / 48,7350 (WGS84) … » sur MNT Lambert-93 | `data.transform_point` vers 2154, puis `terrain.viewshed` sur `step:1.point` | un refus ou un champ de vision en 4326 est un échec |
| Nouvelle commande | 2 phrases par commande dans l'essai neuf | plan conforme pour au moins 1 des 2 | 0 sur 2 = commande non apprise |
| Carroyage sans système | « … carroyage UTM au kilomètre » | question qui demande le système | un plan est un échec |
| Rejeu | journal de la recette QGIS | `identical: true` | divergence = constat, arrêt |

</frozen-after-approval>

## Code Map

- `ai/tests/plan_reel.rs` l. 14-15 (en-tête S25), l. 67-113 (`expected`, S25 dans la liste des refus l. 110), tests sans modèle l. 216-250 : S25 attend le plan `data.transform_point` (`point` WGS84, `to_epsg` 2154 ; noms exacts au catalogue) puis `terrain.viewshed` (`observer: step:1.point`, 10 m, 1 km). Report ouvert de `deferred-work.md` l. 753 : le clore.
- `ai/tests/terrain_reel.rs` -- patron du nouvel essai : `SENTENCES`, `expected(id)`, `comparable`, `compose_plan_with`, `common::shown_inventory`, test sans modèle qui passe chaque attendu à `check_plan` et à `guard_coordinates`.
- `tests/epreuve-modele-local/inventory.json` -- six vecteurs et un MNT (`donnees/mnt_5m.tif`). Il n'a ni raster de coût ni critère : l'essai neuf part de lui et y ajoute en mémoire les rasters qu'il lui faut (coût, deux critères notés, un masque), dans la forme des inventaires de QGIS que montrent `training/src/inventories.rs` (`Builder::add_raster`).
- `training/src/families.rs` -- les formes apprises (lots 1 à 3 de 11.12) : à recopier dans les attendus, sans les deviner. Familles : `coord_convert`…, `export_pdf_format`, `superposition_ponderee`, `cout_cumule`, `chemin_moindre_cout`, etc. (sections 11.12 de `docs/entrainement-du-modele.md`, l. 166-316).
- `training/tests/recouvrement.rs` -- ajouter l'essai neuf (`include_str!` + `pairs`), comme `TERRAIN_REEL`.
- `training/entrainement/pipeline.py` -- commande unique (`--sortie`), `MAX_LENGTH = 2048` : regarder au rapport le nombre d'exemples écartés pour longueur (le registre a grossi).
- `docs/entrainement-du-modele.md` § « Ré-entraînement de 5.41 » -- patron du compte rendu (tableau des entraînements, tableau des essais, SHA-256, décisions).
- Installation et recette : mémoire `mettre-a-jour-qgis-et-noyau` (liens durs dans `%LOCALAPPDATA%\Cassini\bin\models\`, `cassini.exe run model.list '{}'`), sonde jetable sur le patron de `%TEMP%\sonde-5-41\sonde.py` et `sonde-rejeu.py`, projet `../donnees-test-cassini/projet-test-villeneuve.qgz` (MNT 2154 depuis 11.6) copié en `projet-sonde-11-16.qgz`.

## Tasks & Acceptance

**Execution:**
- [x] `ai/tests/plan_reel.rs` -- attendu S25, en-tête, tests sans modèle -- l'attendu ne périme plus
- [x] `ai/tests/epic11_reel.rs` (nouveau) -- 2 phrases par commande (28), inventaire étendu en mémoire ; test sans modèle : chaque attendu passe `check_plan` et le garde-fou ; essai ignoré : score par commande -- mesure ce que 11.12 a enseigné
- [x] `training/tests/recouvrement.rs` -- inclure l'essai neuf -- aucune de ses phrases dans le jeu engendré
- [ ] (hitl, feu vert) entraînement vers `~/cassini-entrainement/11-16`, fusion GGUF, parité des découpeurs ; copie dans `C:\Users\kevin\cassini-modeles\ajuste-11-16\`
- [ ] mesures GPU avant installation : `plan_reel`, `question_reel`, `ecart_reel`, `echelle_utm_reel`, `terrain_reel`, essai neuf, sur le modèle neuf et sur 5.41 (essai neuf seulement, pour la colonne de comparaison) -- les barres
- [ ] installation (5.41 renommé `-5-41` + déclaration) ; noyau `--features gpu` et extension à jour ; `model.list`
- [ ] `../donnees-test-cassini/` (hors dépôt) -- raster de coût, deux critères notés, un masque, dérivés du MNT 2154 par QGIS, ajoutés à `projet-sonde-11-16.qgz` ; source notée dans `LISEZMOI.txt` -- décision 3
- [ ] recette QGIS : une dizaine de phrases enchaînées dans une seule session (conversion MGRS, distance et azimut, reprojection puis champ de vision, champ de vision cumulé, exposition, points hauts, pente au-delà d'un seuil, chemin de moindre coût, export A3 carroyé, phrase de démo), puis rejeu -- le produit
- [ ] `docs/entrainement-du-modele.md` -- section « Ré-entraînement de 11.16 » ; `deferred-work.md` -- report S25 clos

**Acceptance Criteria:**
- Given le modèle neuf, when les essais tournent sur GPU, then aucun ne descend sous sa barre (décision 2) et chaque score est consigné.
- Given l'essai neuf, when il tourne, then chacune des quatorze commandes a au moins une phrase composée en plan conforme.
- Given QGIS, le projet de test et le modèle installé, when les phrases s'enchaînent puis le journal est rejoué, then chaque plan est juste et s'exécute, et le rejeu rend `identical: true`.

## Design Notes

Ce ticket ajoute 14 commandes au registre vu par le modèle et rééquilibre le jeu (8 000 exemples, poids inchangés) : les anciennes familles y pèsent moins. C'est le risque principal de recul ; les cinq essais existants le mesurent. Si une barre manque, arrêt et décision du propriétaire, comme en 5.41 (au plus un second entraînement après un générateur complété).

## Verification

**Commands:**
- `cargo test -p cassini-ai --test plan_reel --test terrain_reel --test epic11_reel` et `cargo test -p cassini-training` -- expected: verts sans modèle
- `CASSINI_TEST_MODEL=<déclaration> cargo test -p cassini-ai --release --features cuda -- --ignored <essai>` (PowerShell, `LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`) -- expected: les barres
- `cargo clippy -p cassini-ai -p cassini-training --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propres
