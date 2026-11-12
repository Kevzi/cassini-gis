---
title: "5.54 séance 2 — refus moins réflexes, contraste ligne de vue/recoupement, rappel stratifié"
type: 'feature'
ticket: ''
created: '2026-11-12'
status: 'built'
baseline_revision: '4fc3fe33cd31f6f7e9e9c3f8962c7bc1f0f66663'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La séance continue 1 (depuis B, `FIRST_SESSION_FAMILIES`, rappel 40 % préfixe du jeu graine 1) gagne sur les barres élargies mais la barre de non-oubli la refuse (ecart_reel 39→35/40, recoupement 2/3→0/3) : refus devenus réflexe (dégradé selon un champ numérique, modèles d'IA du poste, chemin de moindre coût entre deux points, décompte par zone avec le nom, « 5 classes »), « aimant » du champ de vision (~600 exemples contre ~8 de ligne de vue), `transform_point` vers 4326 posé sur des points déjà en WGS84/UTM, oubli des petites familles du rappel.

**Approach:** Décisions du propriétaire : (1) séance 2 depuis B ; (2) `refus_fonction_absente` poids 6 → 2 ou 3, presque-refus une fois sur deux, voisins neufs (chemin de moindre coût entre deux points écrits, dégradé selon un champ numérique à plusieurs verbes et nombre de classes seul, décompte par zone avec `label_field`, questions du poste `model.list`/`system.about`), test : tout refus appris est une constante `NO_*` connue ; (3) contraste au jeu ciblé : `ligne_de_vue` (poids ≈ 3, « voit-on B depuis A », hauteur de la cible, points WGS84 vers le système du MNT) et `recoupement` (poids ≈ 2, stations WGS84 ou UTM recopiées sans conversion), branche recoupement de `point_lambert_vers_geo` variée, `champ_de_vision_cumule` 6 → 3, `ordre_nord_est` champ de vision 1/3 → 1/6 plus une branche ligne de vue ; (4) rappel 50 % stratifié (au moins N ≈ 15 exemples par famille, le reste au poids), gardé par `recouvrement.rs`, plancher de la chaîne inchangé (≥ 0,4), `--part-rappel 0.5` ; (5) `options_ecrites` : dégradés qui n'écrivent que le nombre de classes ; (6) séance par `nuit-5-54.sh` (sortie `ajuste-continu-5-54-2`), juge `non_oubli.py` contre `reference-non-oubli/`, installation seulement si installable et Annecy ≥ 16/20.

## Boundaries & Constraints

**Always:** vocabulaire civil ; option au plan ssi écrite ; un refus nomme une fonction absente du registre ; chaque famille touchée a son test ; verts : `recouvrement.rs`, tests des familles, `gros_projet` 23-27 % et redemandes ≥ 4,5 % du jeu graine 1 ; le rappel stratifié n'est fait que d'exemples du jeu graine 1 complet (`generate(DEFAULT_SEED, DEFAULT_COUNT)`), que la garde contrôle déjà, et le binaire le contrôle encore (`barres::copies`, code 3).

**Never:** lire `ai/tests/tenu_a_l_ecart/sentences.json`, `ai/tests/*/phrases.json`, `tests/epreuve-modele-local/sentences.json`, les constantes de phrases des `ai/tests/*_reel.rs`, `qgis-plugin/tests/recette/*.json` (seule la garde compare ; en cas de collision, regarder la phrase *engendrée* et retirer sa tournure) ; toucher aux barres, au noyau, au registre ; lancer un modèle, le GPU ou la séance ; pousser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Rappel stratifié | `--seed 1 --count 1560 --rappel-stratifie 15` | 1 560 exemples du jeu graine 1, ≥ 15 par famille, `gros_projet` ≥ 20 % | — |
| Rappel coupé | 1 500 premiers des 1 560 | tous les exemples du plancher y sont | — |
| Autre graine | `--rappel-stratifie 15 --seed 2` | rien d'écrit | code 2 |
| Avec `--familles` | les deux options | rien d'écrit | code 2 |
| « Voit-on B depuis A » | deux points | `terrain.line_of_sight`, jamais `terrain.viewshed` | — |
| Recoupement WGS84/UTM | stations 4326 ou 326NN | `geo.bearing_intersection` direct, aucun `transform_point` | — |
| Dégradé « en 5 classes » | champ numérique | `map.graduated_style{layer_id, field, class_count}` | jamais un refus |

</frozen-after-approval>

## Code Map

- `training/src/lib.rs` -- `DEFAULT_SEED`/`DEFAULT_COUNT` (28-31) ; `FOCUS_*`, `FIRST_SESSION_FAMILIES` (286-313), `generate_from` (tirage pondéré, `seen` dédoublonne) ; tests : `the_recall_set_is_a_prefix_of_the_full_set` (1449, à remplacer), `the_pipeline_reads_the_family_names_and_constants_as_written_here` (1457, ajouter la constante neuve), `the_first_session_families_give…` et `the_session_command_of_the_doc…` (1520-1547, à dupliquer pour la séance 2), gros projets (l. ~755), redemandes.
- `training/src/main.rs` -- options ; `--familles` refuse un jeu qui copie une barre (code 3) : même contrôle pour `--rappel-stratifie`.
- `training/src/families.rs` -- `Example.family` (24) ; table `FAMILIES` (53, 72 familles, poids 573 ; `ligne_de_vue` 4, `recoupement` 3, `champ_de_vision_cumule` 6, `refus_fonction_absente` 6, `gros_projet` 148, `redemande` 76) ; `description` (3458, branches `system.about`/`model.list` 3497-3513 : tournures à ne pas recopier) ; `decompte_par_zone` (3349, `label_field` 3393) ; `ligne_de_vue` (4541) ; `champ_de_vision_cumule` (4808, `foreign_point`) ; `chemin_moindre_cout` (5190 : `cost_inventory`, `cost_words`, `terrain_point`, `foreign_terrain_point`, `de_np`/`a_np`) ; `recoupement` (7396) ; constantes `NO_*` (3676-3842, 8466-8485 ; `NO_CURVATURE` est une liste de tournures, pas un refus) et `ABSENT_FUNCTIONS` ; `refus_fonction_absente` (8516, `chance(1, 3)`) ; `presque_refus` (8850, 4 branches) ; `options_degrade` (`mask` dans `[3, 5, 6, 7]`) ; `geo_input`, `point_lambert_vers_geo` (9202, branche 2 = recoupement Lambert-93) ; `ordre_nord_est` (9444, `below(3)`, branche 1 = champ de vision) ; tests du presque-refus (~14602, ~14648 « un presque refus pour deux refus »).
- `training/entrainement/pipeline.py` -- `RECALL_SHARE`/`MIN_RECALL_SHARE` (127-128, inchangés), `RECALL_MARGIN` (133), `GENERATOR_CONSTANT`/`generator_constants` (539), `session_counts`, `session_mix` (coupe le rappel à `recall_count` : à garder), `lineage`, `check_resume_parent`, `generation_plan` (860 : le rappel = graine 1 sans option), docstrings « préfixe » (29-32, 860-864) ; tests `training/entrainement/tests/test_ajustement_continu.py` (124, 319-342).
- `training/tests/recouvrement.rs` -- garde complète et par famille : ne pas modifier.
- Hors dépôt : `~/cassini-entrainement/nuit-5-54.sh` (`WT` l.21, `NOM` l.23, `MES` l.27, `--part-rappel 0.4` l.98).
- Comptes relevés à `4fc3fe3` (graine 1, 8 600) : plus petite famille `recoupement` 35, `ligne_de_vue` 59, `gros_projet` 2 219 (25,8 %), `redemande` 330 ; dans les 1 500 premiers : `recoupement` 4, `reclassement_raster` 4, `ligne_de_vue` 13. Simulation du plancher sur 1 500 : N = 15 → `gros_projet` 21,9 %, `redemande` 3,2 % ; N = 20 → `gros_projet` 5 % (écarté).

## Tasks & Acceptance

**Execution:**
- [ ] `training/src/families.rs` -- poids : `refus_fonction_absente` 6 → 3, `champ_de_vision_cumule` 6 → 3 ; `gros_projet`/`redemande` réajustés si besoin pour 24-26 % et ≥ 4,5 % -- décisions 2-3.
- [ ] `training/src/families.rs` -- `refus_fonction_absente` : presque-refus `chance(1, 2)` ; `presque_refus` gagne quatre branches (chemin de moindre coût entre deux points écrits → `terrain.least_cost_path` ; dégradé à plusieurs verbes, nombre de classes seul une fois sur deux → `class_count` ; décompte par zone avec le nom → `analysis.zonal_statistics` + `label_field` ; question du poste → `model.list`/`system.about`, tournures neuves) -- décision 2.
- [ ] `training/src/families.rs` -- `ligne_de_vue` : tournures « voit-on B depuis A », « B est-il visible de A » ; hauteur de la cible plus souvent ; une fois sur trois des points WGS84 (convention 11.22 de `foreign_point`/`foreign_terrain_point`) ; `recoupement` : tournures neuves, stations 4326/326NN recopiées ; `point_lambert_vers_geo` branche 2 : une station Lambert-93 (transformée) avec une station UTM recopiée une fois sur deux ; `ordre_nord_est` : `below(6)`, champ de vision 1/6, ligne de vue 1/6 ; `options_degrade` : `mask` 2 (nombre de classes seul) ajouté au tirage, doc « au moins une option » -- décisions 3 et 5.
- [ ] `training/src/families.rs` (tests) -- tout `refusal` du jeu graine 1 et du jeu ciblé de la séance 2 a pour raison une constante d'une liste `KNOWN_REFUSALS` (tous les `NO_*` de refus) ; presque-refus ≈ un sur deux, chaque voisin neuf présent ; « voit » → jamais `terrain.viewshed` ; `recoupement` sans `data.transform_point` ; `point_lambert_vers_geo` : `transform_point` seulement sur un point 2154 ; `ordre_nord_est` : les quatre commandes présentes ; nombre de classes seul → plan sans `method`/`ramp`.
- [ ] `training/src/lib.rs` -- `pub const RECALL_FLOOR: usize = 15;` ; `pub fn stratified_recall(count, floor) -> Result<Vec<Example>, String>` (Design Notes) ; `pub const SECOND_SESSION_FAMILIES` = `FIRST_SESSION_FAMILIES` + `ligne_de_vue`, `recoupement` ; tests : le rappel stratifié (matrice), le test du préfixe remplacé, la liste de séance 2 (`FOCUS_COUNT` exemples, aucune copie), la commande de la doc pour la séance 2, `RECALL_FLOOR` lu par la chaîne.
- [ ] `training/src/main.rs` -- `--rappel-stratifie N` : graine ≠ `DEFAULT_SEED`, `--familles` ou `--count` > `DEFAULT_COUNT` → code 2 ; copie de barre → code 3 ; en-tête mis à jour.
- [ ] `training/entrainement/pipeline.py` (+ tests) -- `RECALL_FLOOR` lu par `generator_constants` ; `generation_plan` passe `--rappel-stratifie <RECALL_FLOOR>` au rappel ; `lineage` note `rappel_plancher` ; `check_resume_parent` le compare (absent = ancien préfixe) ; docstrings.
- [ ] `docs/entrainement-du-modele.md` -- section « Séance 2 (5.54) » : causes, familles, rappel stratifié, comptes avant/après, commande de la séance (ligne exacte `nuit-5-54.sh <SECOND_SESSION_FAMILIES>`) et règle d'installation.
- [ ] Hors dépôt : `nuit-5-54.sh` -- `NOM=${NOM:-ajuste-continu-5-54-1}`, `MES=${MES:-$E/5-54-mesures}`, `PART_RAPPEL=${PART_RAPPEL:-0.4}` passé à `--part-rappel` ; en-tête à jour.

**Acceptance Criteria:**
- Given le code, when `cargo test -p cassini-training --release` et `uv run … pytest`, then tout est vert, `recouvrement.rs` compris.
- Given une séance de 3 000 à 0,5, when `generation_plan`, then rappel 1 500 (+ 60 de marge) stratifié, cible 1 500 ≤ `FOCUS_COUNT`.

## Design Notes

Party (ingénieur ML, géomaticien, architecte) :
- **Quotas du rappel** : `full = generate(DEFAULT_SEED, DEFAULT_COUNT)` ; `n_f` = exemples de `f` dans `full` ; `q_f = min(n_f, max(min(N, n_f), ⌊λ·n_f⌋))`, λ maximal tel que Σ q_f ≤ `count` ; reliquat comblé dans l'ordre de `full` parmi les non pris. Sortie : les `min(N, n_f)` premiers de chaque famille (ordre de `full`), puis le surplus (ordre de `full`) ; la marge coupée par `session_mix` ne mord que le surplus. L'entraînement mélange (`pipeline.py:1077`) : l'ordre est sans effet. Poids = comptes réels du jeu complet (rejets de `gros_projet` compris), pas `Family.weight`. Raison : sous-ensemble du jeu gardé → couverture de `recouvrement.rs` sans test neuf ; N = 15 garde ~22 % de gros projets.
- **Stratifié toujours en séance** (pas d'option de la chaîne) : un seul chemin ; la lignée l'écrit.
- **Poids globaux de `ligne_de_vue` (4) et `recoupement` (3) inchangés** : les baisser vers 3 et 2 les retirerait du jeu complet, contre la cause d'oubli ; dans la liste de séance 2 (poids 30) ils pèsent 13 % et 10 %, le rapport 3:2 voulu. Refus à 3 plutôt que 2 : avec un presque-refus sur deux, 2 rendrait les voisins neufs trop rares.
- **Points WGS84 de la ligne de vue** : recopiés, le noyau convertit (11.22), comme `chemin_moindre_cout` et le cumulé ; MGRS seul garde ses étapes.
- **`point_lambert_vers_geo`** : partenaire UTM, jamais WGS84 (garde-fou, lot 2).
- **Liste de séance 2** = les six de la séance 1 + `ligne_de_vue` + `recoupement` : la séance repart de B, les gains doivent se réapprendre.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy -p cassini-training --all-targets -- -D warnings && cargo test -p cassini-training --release` -- expected: vert.
- `uv run --project training/entrainement python -m pytest training/entrainement/tests` -- expected: vert.
- `cargo run -p cassini-training --release -- --format chat --seed 1 --count 1560 --rappel-stratifie 15 --out <scratch>` -- expected: code 0 ; `--seed 2` : code 2.

**Commande de la séance** (après fusion en avance rapide sur `main`, sans push, `REV` = HEAD du worktree, propre ; GPU libre, `ps` vérifié) :
`git -C "/mnt/c/Users/kevin/Downloads/Projet code/CASSINI-nuit" checkout -q --detach REV && WT=~/cassini-worktrees/5-54-seance-2 NOM=ajuste-continu-5-54-2 MES=~/cassini-entrainement/5-54-2-mesures PART_RAPPEL=0.5 bash ~/cassini-entrainement/nuit-5-54.sh export_pdf_echelle_format,refus_fonction_absente,options_ecrites,champ_de_vision_cumule,point_lambert_vers_geo,ordre_nord_est,ligne_de_vue,recoupement`

**Installation** : seulement si `nuit-ajuste-continu-5-54-2.etat` (état propre à la séance 2, voir Plan Change Log) dit « VERDICT non-oubli : installable » et `[annecy] SCORE` ≥ 16/20 ; B reste installé à côté.

## Implementation Notes

- **Rappel stratifié** : `stratified_recall` / `stratified_recall_with_floor` (`training/src/lib.rs`), quotas des Design Notes par dichotomie sur λ ; `--rappel-stratifie N` écrit à côté du rappel `<out>.familles.json` (famille de chaque ligne, plancher `min(N, n_f)` de chaque famille). `pipeline.py` revérifie après `drop_too_long` et la coupe de la marge que le rappel a son compte et garde chaque plancher (`check_recall_floor`, famille nommée), refuse avant le GPU un rappel plus court que `RECALL_FLOOR` × nombre de familles, et écrit `rappel_plancher` et `rappel_version` (`RECALL_VERSION`, version de l'algorithme) dans le maillon ; la reprise les compare.
- **Refus** : les refus écrits en clair dans `refus` deviennent des constantes `NO_*` (les deux raisons de suppression d'entités fusionnées : 3 refus du jeu changent de texte) ; `KNOWN_REFUSALS` (32) et `OTHER_REFUSALS` : chaque refus a des mots-clés absents des identifiants du registre.
- **Ligne de vue** : extrémités étrangères par `foreign_point` (observateur, cible ou les deux, une fois sur trois), comme les autres familles de terrain ; garde des tournures « voit-on le/la/l' », « aperçoit-on », « est-il (elle) visible », « se voit-il (elle) » (jamais un champ de vision ni un refus) et sa réciproque sur les familles du champ de vision.
- **`point_lambert_vers_geo`** : dans la branche mixte, la station en Lambert-93 (transformée) passe toujours en premier, pour que le recoupement rende son point en WGS84.
- **Script de nuit** : `NOM`, `MES`, `PART_RAPPEL` et `ETAT` réglables ; l'état vaut `nuit-<NOM>.etat` hors de la séance 1.
- **Comptes** (jeu graine 1, après) : gros projets 2 124 (24,7 %), exemples de redemande 428 (4,98 %, famille `redemande` seule 337), refus 464 ; rappel de 1 500 entraînés : gros projets 318 (21,2 %), chaque famille ≥ 15.

## Plan Change Log

- 2026-11-12 — **`carroyage_utm_projet` poids 6 → 9** (hors plan) : avec les poids de la séance, `the_default_set_never_reprojects_a_metric_layer_unasked_and_asks_for_a_missing_utm_zone` tombait (35 questions pour 71 recopies à 6, 36 pour 76 à 8) ; même correction qu'au lot 2.
- 2026-11-12 — **test `north_before_east_is_read_by_its_labels`** : accepte « la latitude et la longitude » à côté de « latitude et longitude », tournure existante de `convert_words` que le tirage déplacé fait sortir.
- 2026-11-12 — **poids globaux de `ligne_de_vue` (4) et `recoupement` (3) gardés**, au lieu de ≈ 3 et ≈ 2 (décision 3) : les baisser les retirerait du jeu complet, contre la cause d'oubli (Design Notes) ; le rapport 3:2 voulu est tenu dans la liste de la séance 2 (13 % et 10 % de ses poids).
- 2026-11-12 — **état du script de nuit** : `nuit-ajuste-continu-5-54-2.etat` au lieu de `nuit-5-54.etat`, pour que la séance 2 ne vide pas le verdict de la séance 1 ; règle d'installation mise à jour.
- 2026-11-12 — **fichier `<rappel>.familles.json`** et **`RECALL_VERSION`** ajoutés après revue : la chaîne revérifie le plancher après l'écart des exemples trop longs, et la reprise compare la version de l'algorithme.

## Review Triage Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 0, medium 8, low 9, false 0, maybe-false 0. Corrigés 15 (patch, même implémenteur), reportés 2.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Plancher par famille et compte du rappel perdus après le filtrage des exemples trop longs | medium | patch | fichier des familles écrit par le générateur, `check_recall_floor` après filtrage et coupe |
| Plancher × familles plus grand que le rappel non refusé avant le GPU ; plancher 0 ; version de l'algorithme non comparée en reprise | medium | patch | refus avant le GPU, code 2, `RECALL_VERSION` |
| Chemin `--rappel-stratifie` du binaire jamais exécuté | medium | patch | test de `commande.rs` (octets, codes 2) |
| Ligne de vue : seul l'observateur étranger, WGS84 seulement | medium | patch | cas étrangers de 11.22 des deux côtés, test |
| Garde des tournures de ligne de vue étroite, sans réciproque | medium | patch | `line_of_sight_wording`, refus interdit, réciproque sur les familles de champ de vision |
| Questions sur le poste : six phrases fixes, deux mal choisies | medium | patch | tournures tirées, Cassini nommé |
| Nouvelles constantes de refus non vérifiées absentes du registre ; deux raisons pour la suppression | medium | patch | `OTHER_REFUSALS`, union vérifiée contre le registre ; raison unique |
| `nuit-5-54.sh` : état partagé, verdict de la séance 1 effacé | medium | patch | `nuit-<NOM>.etat` |
| Branches 4 et 6 du presque-refus (virgule, verbe, élision, inventaire) | low | patch | corrigées, `de_plural` partagé |
| Recoupement branche 8 sans azimut ; système du résultat selon l'ordre | low | patch | « à l'azimut » ; station Lambert-93 en premier, résultat en 4326, test |
| Comptes de la doc à deux dénominateurs, comptes figés | low | patch | rapportés aux 1 500 entraînés, recomptés |
| Écarts au plan non consignés | low | patch | Implementation Notes et Change Log |
| Poids ligne de vue / recoupement gardés à 4 et 3 au lieu de ≈ 3 et ≈ 2 | low | defer | les baisser les retirerait du jeu complet ; 13 % et 10 % du jeu ciblé |
| Rapport ligne de vue / champ de vision non mesuré sur le mélange réel de séance | low | defer | comptes donnés sur le jeu complet et le rappel ; mesure du mélange à ajouter au rapport de séance |

Poids : `carroyage_utm_projet` 6 → 9.
