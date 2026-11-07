---
title: 'Ré-entraînement groupé et recette du modèle'
type: 'feature'
ticket: '16'
created: '2026-11-07'
status: 'built'
baseline_revision: '01ed24f4c2e7c3841df6f02174d7fe707d1049d5'
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
- [x] (hitl, feu vert) entraînement vers `~/cassini-entrainement/11-16`, fusion GGUF, parité des découpeurs ; copie dans `C:\Users\kevin\cassini-modeles\ajuste-11-16\`
- [x] mesures GPU avant installation : `plan_reel`, `question_reel`, `ecart_reel`, `echelle_utm_reel`, `terrain_reel`, essai neuf, sur le modèle neuf et sur 5.41 (essai neuf seulement, pour la colonne de comparaison) -- les barres
- [x] installation (5.41 renommé `-5-41` + déclaration) ; noyau `--features gpu` et extension à jour ; `model.list`
- [x] `../donnees-test-cassini/` (hors dépôt) -- raster de coût, deux critères notés, un masque, dérivés du MNT 2154 par QGIS, ajoutés à `projet-sonde-11-16.qgz` ; source notée dans `LISEZMOI.txt` -- décision 3
- [x] recette QGIS : une dizaine de phrases enchaînées dans une seule session (conversion MGRS, distance et azimut, reprojection puis champ de vision, champ de vision cumulé, exposition, points hauts, pente au-delà d'un seuil, chemin de moindre coût, export A3 carroyé, phrase de démo), puis rejeu -- le produit
- [x] `docs/entrainement-du-modele.md` -- section « Ré-entraînement de 11.16 » ; `deferred-work.md` -- report S25 clos

**Acceptance Criteria:**
- Given le modèle neuf, when les essais tournent sur GPU, then aucun ne descend sous sa barre (décision 2) et chaque score est consigné.
- Given l'essai neuf, when il tourne, then chacune des quatorze commandes a au moins une phrase composée en plan conforme.
- Given QGIS, le projet de test et le modèle installé, when les phrases s'enchaînent puis le journal est rejoué, then chaque plan est juste et s'exécute, et le rejeu rend `identical: true`.

## Implementation Notes

- Étape sans modèle (commit `ce7fe05`) : S25 attend `data.transform_point` (`target_epsg`, nom réel du catalogue) puis `terrain.viewshed` ; `ai/tests/epic11_reel.rs`, 28 phrases + F03 (carroyage sans système → question) ; recouvrement 91 phrases, vert.
- 2026-11-07, entraînement : commit `ce7fe05` (arbre propre), `~/cassini-entrainement/11-16`, 323 min (dont 49 min de validation), perte finale 0,00697, 0 exemple écarté pour longueur (jetons : médiane 738, max 1 644), validation 293/300 (plans 236/241, questions 34/36, refus 23/23), parité 300/300 (202 154 jetons), essais `llama-completion` base + adaptateur et fusionné : plan attendu. Fusionné SHA-256 `9dbed14d801eda2705e4f743b2a0057ccb101db414ef2995b3c0e17296c3ea6e`, copié dans `ajuste-11-16`.
- Ratés de validation : 2 reprojections UTM d'un fuseau à l'autre oubliées, 1 carroyage sans système planifié, 2 noms de raster, 1 refus à tort, 1 plan tronqué.
- Essais GPU (`~/cassini-entrainement/11-16-mesures/`) : `plan_reel` 23/25 (S18, S20 ; 1 fausse question), `question_reel` 12/12, `ecart_reel` 34/40 (plans 20/26, questions 8/8, refus 6/6, 180 s), `echelle_utm_reel` 8/8, `terrain_reel` 8/8, `epic11_reel` 27/28, 14/14 commandes (A01 rendu en pente + reclassement), F03 refusé (« aucune commande n'exporte de carte ») au lieu de la question. 5.41 sur `epic11_reel` : 0/28.
- S25 juste. S18 (« carte PDF A3 en portrait **des routes et des gares** ») : le modèle exporte toutes les couches visibles ; le refus reste attendu (couches nommées du PDF non portées par l'export), donc un vrai échec.
- 2026-11-07, installation : `ajuste-11-16` en lien dur sous `cassini-composition-4b-Q4_K_M.gguf` (SHA-256 vérifié), 5.41 renommé `cassini-composition-4b-5-41-Q4_K_M.gguf` + déclaration ; `model.list` voit les six. Noyau installé identique au build `--features gpu` (aucun changement du noyau depuis), extension identique au dépôt.
- Rasters de recette (décision 3) : `donnees-test-cassini/derives-11-16/` (coût, accessibilité, calme, masque < 40 m), grille du MNT 2154, ajoutés à `projet-sonde-11-16.qgz` ; source notée dans `LISEZMOI.txt`.
- Recette QGIS (sonde jetable `%TEMP%\sonde-11-16\sonde.py`) : 12/12 plans justes et exécutés, rejeu `identical: true` (29 étapes). La distance entre deux points Lambert-93 passe par deux `data.transform_point` vers 4326 : juste, `geo.distance_azimuth` n'accepte que 4326 et UTM.

## Plan Change Log

- 2026-11-07, décision du propriétaire après les mesures : le modèle `ajuste-11-16` est accepté malgré F03 (refus au lieu de la question ; ligne « Carroyage sans système » de la matrice non tenue). F03, A01, S18 et S20 vont au report `deferred-work.md` comme familles à renforcer au prochain ré-entraînement groupé ; pas de second entraînement dans ce ticket.

## Review Triage Log

Relecture `thorough` du 2026-11-07 (quatre lentilles), diff depuis `01ed24f` : high 0, medium 3, low 9, false 4, maybe-false 0 ; plusieurs lentilles ont rendu les mêmes constats, regroupés ci-dessous.

| # | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|
| 1 | `epic11_reel` rouge sur le modèle accepté (F03 compté dans l'assertion), alors que la doc annonce la barre tenue (4 lentilles) | medium | patch | Doc : l'essai reste rouge par décision, il garde l'exigence de F03 (ligne figée de la matrice) pour le prochain ré-entraînement ; l'assertion n'est pas affaiblie. |
| 2 | La recette QGIS n'exerce pas `terrain.cost_distance` (décision 3) ; `geo.destination_point`, `geo.bearing_intersection`, MGRS → WGS84 non plus (3 lentilles) | medium | patch | Seconde sonde (`projet-sonde-11-16b.qgz`) : 4/4 plans exécutés, rejeu identique ; section ajoutée à la doc. |
| 3 | Six écarts de `ecart_reel` non nommés | medium | patch | H08, H11, H13, H17, H18, H19 nommés dans la doc, comparés à 5.41. |
| 4 | Ratés de validation absents du report ; « plan tronqué » | low | patch | Rang 239 vérifié : pas tronqué, mauvaise référence `step:0` ; doc précisée (noms de raster, refus, référence d'étape). |
| 5 | Durée sur processeur non mesurée alors que le registre a grossi | low | defer | La croissance du prompt vient de 11.2 à 11.11, pas de ce ticket ; entrée au report (`duree_reel` sur CPU). |
| 6 | Doc d'installation : « déclaration … lien dur » ; SHA-256 sans dire quel fichier | low | patch | Formulation corrigée ; empreinte de la sortie, de la copie et de l'installé. |
| 7 | Aucune phrase de l'essai neuf avec un point d'un autre système ni reprojection UTM 31N → 32N | low | defer | Entrée au report, avec le prochain ré-entraînement. |
| 8 | `the_grid_question_names_no_system` : liste de mots trop courte (2 lentilles) | low | patch | `2154`, `utm 3`, `fuseau`, `wgs`, `4326`, `epsg`, `mgrs`, `°` ajoutés. |
| 9 | `s25_reprojects_…` ne vérifie ni le point en 4326 ni le MNT | low | patch | Deux assertions ajoutées. |
| 10 | Banc Python gelé (`sentences.json`, `harness.py`) garde S25 en refus | low | patch | Une phrase dans l'en-tête de `plan_reel.rs` ; le dossier gelé n'est pas touché. |
| 11 | Méthode des rasters de critères non décrite ; 3 606 m non recoupés | low | patch | Formules dans la doc ; distance et azimut recoupés à la main (≈ 3,60 km, 55,9°). |
| 12 | Code Map : `to_epsg` au lieu de `target_epsg` | low | rejet | La correction éditerait le plan ; le nom réel est dans les Implementation Notes. |
| 13 | Ordre de la voie de l'epic sans l'entrée 18 ; `verify` de 18 sans mesure du gros MNT | low | rejet | Décision du 2026-11-07 dans les Notes de l'epic (14 attend 18) ; le `verify` d'une entrée se précise à son build. |
| 14 | Z01 : `min: 15` inclusif pour « dépasse 15 % » | false | rejet | Forme enseignée par le générateur (`pente_seuil`, 11.12 lot 2a) ; la borne exacte ne change rien d'observable. |
| 15 | Un paramètre d'export en trop ferait échouer F01/F02 | false | rejet | Comparaison exacte voulue, comme `terrain_reel` ; un paramètre non demandé est une erreur. |
| 16 | Champs de relecture contradictoires (lentilles listées, journal vide) | false | rejet | La relecture était en cours ; ce journal la consigne. |
| 17 | Le lecteur des critères manquerait F03 | false | rejet | Plan Change Log et doc le disent. |

## Design Notes

Ce ticket ajoute 14 commandes au registre vu par le modèle et rééquilibre le jeu (8 000 exemples, poids inchangés) : les anciennes familles y pèsent moins. C'est le risque principal de recul ; les cinq essais existants le mesurent. Si une barre manque, arrêt et décision du propriétaire, comme en 5.41 (au plus un second entraînement après un générateur complété).

## Verification

**Commands:**
- `cargo test -p cassini-ai --test plan_reel --test terrain_reel --test epic11_reel` et `cargo test -p cassini-training` -- expected: verts sans modèle
- `CASSINI_TEST_MODEL=<déclaration> cargo test -p cassini-ai --release --features cuda -- --ignored <essai>` (PowerShell, `LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`) -- expected: les barres
- `cargo clippy -p cassini-ai -p cassini-training --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propres
