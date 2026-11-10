---
title: 'Générateur : quatre causes de ratés après 11.23 (nom suffixé, fusion, pluriel, « confondues »)'
type: 'feature'
ticket: ''
created: '2026-11-10'
status: 'in-progress'
baseline_revision: '836e12eb9739bc62acc0f3319a150eb677648504'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/analyse-recul-des-plans-apres-11-23.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/plan-re-entrainement-apres-11-23.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'analyse du 2026-11-09 (`analyse-recul-des-plans-apres-11-23.md`, section 3) trouve quatre trous du générateur d'exemples : un champ de nom suffixé (`nom_<thème>`) que le modèle remplace par `nom` ; une fusion du tampon dite dans une proposition à part, perdue ; une valeur au pluriel parmi des valeurs voisines du même champ, mal choisie ; une intersection avec une couche entière « toutes catégories confondues », tournure absente, sans thème vecteur de zones d'exposition au bruit. Trois constats durables restent ratés par `ajuste-revue-11-23` : « cibles filtrées près d'une référence filtrée » perd un filtre quand la cible est désignée par une catégorie d'une couche plus large (des écoles parmi des équipements) ou la référence par un nom propre ; deux conditions d'appartenance sur une même cible sont enchaînées à l'envers (la cible n'est plus la couche de départ).

**Approach:** corriger le générateur par familles ou poids pour ces quatre causes, avec tests ; puis entraîner, passer les essais GPU contre les barres et la recette QGIS d'Annecy, et rendre les résultats au propriétaire, qui décide seul de l'installation. Décision du propriétaire du 2026-11-10 : ces corrections se font quel que soit le score de `ajuste-revue-11-23` (installé).

## Boundaries & Constraints

**Always:** chaque correction est une famille, une tournure ou un poids dérivé d'un constat abstrait ; aucune phrase du jeu tenu à l'écart n'est recopiée ni citée, et `training/tests/recouvrement.rs` reste vert sans changer la graine ; les champs sont toujours désignés par leur rôle (`layer.field(role).name`), jamais par un nom écrit en dur ; chaque exemple passe `check_plan` ; barres inchangées (plan_reel ≥ 23/25, question_reel ≥ 11/12, ecart_reel ≥ 32/40, echelle_utm_reel ≥ 7/8, terrain_reel ≥ 7/8, epic11 ≥ 1/2 par commande et F03 en question, style_reel ≥ 9/10) ; entraînement depuis un worktree, jamais en changeant de branche dans le dépôt principal ; `ajuste-revue-11-23` reste installé tant que le propriétaire n'a pas décidé.

**Décisions du propriétaire (2026-11-10) :** les trois constats durables entrent dans ce plan (1a) ; un seul entraînement, mêmes graines, comparable à `ajuste-revue-11-23` (2a) ; le dossier de la revue guidée en pause est commité sur cette branche (3b).

**Never:** installer sans décision du propriétaire ; supprimer un modèle (déplacer seulement) ; toucher au noyau, au catalogue, au prompt ou à l'extension ; baisser une barre.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Nom suffixé | couche de zones dont le champ de nom s'appelle `nom_<thème>` ; statistique par zone « avec le nom » | `label_field` = ce nom exact | — |
| Nom ordinaire | même demande, champ `nom` | `label_field: "nom"` (inchangé) | — |
| Fusion à part | tampon puis « … et fusionne/regroupe le résultat » dans une proposition séparée | `dissolve: true` | — |
| Pluriel parmi voisines | tampon autour des {noms} {adjectif au pluriel}, champ à plusieurs valeurs dans l'inventaire | `select` sur la valeur de cet adjectif, pas une voisine | — |
| Couche entière « confondues » | cible dans une couche de zones, « toutes zones/catégories confondues » | une intersection avec la couche entière, sans `select` | — |
| Catégorie précise | même couche, une zone nommée | `select` puis intersection (inchangé) | — |
| Cible par catégorie, référence filtrée | « {catégorie} (valeur d'un champ d'une couche plus large) à moins de {d} d'une {référence filtrée} » | `select(cible)`, `select(réf)`, `buffer(step:2)`, `intersect(step:1, step:3)` | — |
| Référence par nom propre | « … à moins de {d} de {nom propre d'un objet} » | `select` sur le champ de nom de la référence, puis tampon | — |
| Deux appartenances | cible dans une zone filtrée et dans une autre couche de zones | `select(zone)`, `intersect(cible, step:1)`, `intersect(step:2, autre couche)` | — |

</frozen-after-approval>

## Code Map

- `training/src/inventories.rs` `NOM` (l. 153, noms `nom`, `libelle`, `nom_usuel`) et `Builder::add` (l. 1650, choix du nom de champ par `rng.pick(def.names)`) -- cause 1 : y tirer parfois `nom_<slug du thème>` pour le rôle `nom`. Les familles lisent déjà le nom par rôle (`zones.field("nom")?.name`, `families.rs` l. 2912) : aucune famille à modifier pour la recopie.
- `training/src/families.rs` `DISSOLVE` (l. 969, compléments collés à la tête) et `tampon_head` (l. 976), `tampon` (l. 989) -- cause 2 : ajouter une forme en proposition séparée après la tête.
- `training/src/families.rs` `qualified` (l. 644, adjectif au pluriel 2 fois sur 3), `filtered_reference` (l. 1076), `draft_distance_ref` -- cause 3 : valeurs à adjectif (`va`, ex. classes de routes, `inventories.rs` l. 520) ; s'assurer que l'inventaire porte d'autres valeurs du champ à côté et tirer plus souvent ce cas.
- `training/src/families.rs` `AREAS` (l. 1132), familles d'appartenance (`appartenance_commune` l. 6192 et voisines) ; `training/src/inventories.rs` thèmes de zones (ex. `inondables` l. 740) -- cause 4 : tournure « toutes … confondues » → intersection avec la couche entière ; nouveau thème vecteur de zones d'exposition au bruit (polygones, champ de zone A à D), distinct du thème raster `bruit` (l. 1321).
- Constats durables : `training/src/families.rs` `filtered_target` (l. 1290, catégorie par champ texte ou nom) et `filtered_reference` (l. 1076, champ texte seulement, jamais le nom) ; rang `11 | 12` de `draft_multi` (l. 1454) ; aucun thème « équipements » à champ de type dont les valeurs sont des catégories nommées (écoles, mairies…) : les écoles sont un thème à part (`ecoles`, `inventories.rs` l. 887). Deux appartenances : rangs de `draft_multi` autour de `area()` (l. 1214) ; aucun rang ne garde la cible en `layer_id` de la première intersection puis croise une seconde couche de zones entière.
- `docs/entrainement-du-modele.md` -- tableau des familles et section de cet essai.
- Chaîne et essais : même commande que la nuit du 2026-11-09 (`~/cassini-entrainement/nuit-2026-11-09.sh`, `%TEMP%\essais-revue-11-23.ps1`, recette `%TEMP%\recette-revue-11-23\sonde.py`), à recopier sous un nouveau nom.

## Tasks & Acceptance

**Execution:**
- [x] `training/src/inventories.rs`, `training/src/families.rs` + tests -- les causes et constats de la matrice, un test par ligne -- constats de l'analyse.
- [x] `docs/entrainement-du-modele.md` -- tableau des familles et section de l'essai.
- [x] `cargo test --workspace`, fmt, clippy sous Windows ; commit sur `story/quatre-causes-apres-11-23`.
- [x] Worktree de ce commit, entraînement, copie dans `C:\Users\kevin\cassini-modeles\ajuste-quatre-causes\`, essais GPU, recette QGIS d'Annecy (copie du projet, modèle chargé à part).
- [x] Rendu au propriétaire : scores par barre comparés à `ajuste-revue-11-23`, ratés par constat ; pas d'installation.

**Acceptance Criteria:**
- Given le jeu engendré par défaut, when on compte les exemples par cause, then chaque cause de la matrice y figure au moins une fois et `recouvrement.rs` est vert.
- Given les essais GPU, when ils finissent, then chaque barre est lue et rapportée, tenue ou non.

## Implementation Notes

- 2026-11-10, générateur : `inventories.rs` — nom de champ `nom_<premier mot du thème>` une fois sur quatre pour le rôle `nom` (`name_slug`) ; `Builder::set_values` ; thèmes `zones_bruit` (polygones, zones A à D, groupe du zonage : « en zone A » ne désigne qu'une couche) et `equipements_publics` (groupe des écoles, champ `categorie`) ; `CATEGORIES`, `NAMED_PLACES`. `families.rs` — `DISSOLVE_APART` et `dissolve_words` (moitié des fusions en proposition à part) ; `adjective_plural` dans `tampon_filtre` (1/3, poids 5 → 6) ; `toutes_zones_confondues` (poids 4) et `zones_bruit` dans `PERIMETERS` et `AREAS` ; `category_target` (rang de `filtered_target`, qui rend aussi le nom pour l'accord, et 1/3 du rang 11-12) ; `named_reference` (1/4 de `filtered_reference`) ; rangs 13-14 de `draft_multi` (deux appartenances, cible en tête de chaîne) ; `decompte_par_zone` nomme une fois sur deux. Recouvrement : « Exporte les … en GeoPackage » de `conversion_format` rendait S09 de `plan_reel` → « Exporte-moi » devant « les … », graine inchangée. Tests : huit nouveaux (un par ligne de la matrice, plus le décompte par cause sur le jeu par défaut). `cargo test --workspace`, fmt, clippy verts sous Windows.
- 2026-11-10, entraînement : commit `ea591bc`, worktree `~/cassini-worktrees/revue-11-23` remis sur ce commit (détaché, même `.venv`), chaîne de la nuit du 2026-11-09 (`~/cassini-entrainement/quatre-causes.sh`), 07 h 25 → 09 h 05 (100 min). Modèle dans `C:\Users\kevin\cassini-modeles\ajuste-quatre-causes\`, **non installé**. Validation GGUF **283/300** (295 pour `ajuste-revue-11-23`).
- Essais GPU (`~/cassini-entrainement/11-23-mesures/essais-gpu-quatre-causes.log`), face à `ajuste-revue-11-23` : `plan_reel` 23/25 (24 ; S15, S20), `question_reel` 12/12 (12), `ecart_reel` 37/40 (36 ; plans 24/27, ratés H08 `operator`, H17 et H18 une étape en moins ; H11 et H20 réussis), `echelle_utm_reel` 8/8, `terrain_reel` 8/8, `epic11_reel` 27/28 (28 ; R02, point projeté 1/2, barre tenue), F03 en question, points d'un autre système 4/4 hors barre (3/4), `style_reel` 9/10 (10 ; ST01). Toutes les barres sont tenues, à la limite pour `plan_reel` et `style_reel`.
- Recette QGIS d'Annecy (copie `projet-recette-quatre-causes.qgz`, modèle par `CASSINI_MODELS`, journal `~/cassini-entrainement/11-23-mesures/recette-annecy-quatre-causes.txt`) : **recul**. Les trois tampons autour des gares (CSV en 4326) partent sans `data.reproject` (`analysis.crs_not_metric`), alors que `ajuste-revue-11-23` les réussissait ; la fusion dite à part donne bien `dissolve: true` ; « avec le nom de chaque commune » rend `label_field: "commune"`, champ absent de la couche (`analysis.unknown_field`) ; seule la conversion en shapefile s'exécute (1/5) ; rejeu identique. Piste non vérifiée : les nouvelles familles diluent `reprojection_avant_analyse` (166 exemples contre 182 au jeu de 84adb16). Décision au propriétaire ; `ajuste-revue-11-23` reste installé.

## Plan Change Log

## Review Triage Log

## Verification

**Commands:**
- `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` -- expected: vert
- Essais GPU `--features cuda` sur le nouveau modèle -- expected: barres lues
