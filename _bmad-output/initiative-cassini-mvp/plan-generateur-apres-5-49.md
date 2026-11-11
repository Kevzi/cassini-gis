---
title: 'Générateur après 5.49 : refus des statistiques d''un raster, tampon sans distance, zone naturelle'
type: 'feature'
ticket: ''
created: '2026-11-11'
status: 'done'
baseline_revision: '3a8eefb335958402cc721ae6b7cd3fb9e845031b'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, adversarial-donnees]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** après le jeu de 5.49 (aucun candidat installé, décision du propriétaire du 2026-11-11), trois
sortes de demandes ratent : le refus des statistiques par zone d'un raster recule (S15 réponse coupée,
S23/S24 plan rendu) ; un tampon sans distance reçoit 10 m inventés au lieu d'une question (RA09) ;
« zone naturelle » rend `begins_with N` alors que l'inventaire ne montre que la valeur `N` (RA05).

**Approach:** corriger par les familles du générateur, sans pansement par phrase : famille de refus
à part et plus lourde, nouvelle famille de question du tampon sans distance, opérateur de la
catégorie de zone déduit des valeurs montrées. Puis préparer (sans la lancer) une nuit à deux graines.

Décisions tranchées en party le 2026-11-11 (ingénieur ML, géomaticien, architecte), à confirmer par
le propriétaire :
- D1 : famille `refus_stats_raster` (poids 5, ≈ 115 exemples contre ≈ 69), sortie de
  `refus_hors_registre` (9 → 6, ses autres branches gardent leur compte), tournures neuves : deux
  statistiques à la fois, « par commune », dérivé du MNT (pente, champ de vision) puis statistique.
- D2 : motif `NO_RASTER_STATS` gardé tel quel ; « raccourcir » porte sur les demandes (tournures
  courtes, une seule proposition, dominantes).
- D3 : famille `question_tampon_sans_distance` de poids 3, nouveau ; `tampon` (7) intact, c'est le
  contrepoids (mêmes verbes, distance écrite → plan). Jamais la tête « Fais un tampon autour ».
- D4 : « naturelle » → `eq N` quand aucune autre valeur commençant par `N` n'est montrée, sinon
  `begins_with N` ; N forcé 3 fois sur 4 (CNIG 2017 : pas de `Nh`). « Agricole », « urbaine » :
  `eq` ; « à urbaniser » : `begins_with AU`.

## Boundaries & Constraints

**Always:** `DEFAULT_COUNT` 8 600, graines 1 et 2 ; `MAX_PROMPT_CHARS` 19 000 et `MAX_LENGTH` 5 632
gardés ; test des ≥ 23 % de gros projets vert ; les autres poids inchangés ; comptes du jeu mesurés
avant et après ; vocabulaire civil.

**Never:** recopier une phrase de recette ou d'essai (le test `recouvrement` en est la garde) ;
baisser une borne de test ; installer ou supprimer un modèle ; lancer la nuit ; pousser ; fusionner
dans `main` ; toucher au dépôt principal.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Zone naturelle, `N` seul montré | `typezone` : A, N, U | `select … eq N` | — |
| Zone naturelle, `Nh` montré | `typezone` : N, Nh, U | `select … begins_with N` | — |
| Tampon sans distance | « Crée un tampon autour des routes » | question sur la distance | — |
| Statistique d'un raster par zone | MNT + communes, « altitude moyenne et maximale de chaque quartier » | refus `NO_RASTER_STATS` | — |

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- `FAMILIES` (l. 53, poids total 501, `gros_projet` 128 tire les autres par poids : chaque famille y garde sa part) ; `refus_hors_registre` (l. 3745, branche `_` = statistiques sur raster, `roll` 2 tiré 2 fois sur 6) ; `ZONE_CATEGORIES` (l. 1382), `zone_category` (l. 1398) et la copie dans `draft_filter_eq` (l. 847) ; `tampon_head`/`tampon` (l. 1083) ; aides `missing_layer_question`, `finish`, `example`, `pick_theme`, `vector`, `dist`, `dissolve_words` ; tests `a_plu_zone_said_in_plain_words_selects_its_code`, `the_plu_zones_of_a_category_said_in_words_are_one_selection` (l. 11117, 11150), tests de refus vers l. 8765.
- `training/src/inventories.rs` -- `Builder::add` (l. 2147) : `force` ajoute la valeur aux valeurs montrées, tirées au hasard ; `zonage.typezone` = U, AUc, AUs, A, N, Nh.
- `training/src/lib.rs` -- `generate`, test `the_default_set_holds_a_quarter_of_big_projects_and_close_layers`.
- `training/tests/recouvrement.rs` -- garde contre la recopie des phrases d'essai et de recette.
- `docs/entrainement-du-modele.md` -- section des mesures du jeu (ajouter une section « après 5.49 »).
- `~/cassini-entrainement/nuit-5-49.sh`, `fusion-5-49.sh` -- modèles des scripts de nuit (hors dépôt).

## Tasks & Acceptance

**Execution:**
- [x] `training/src/families.rs` -- D4 : une fonction partagée pose la couche de zonage et rend l'opérateur d'après les valeurs montrées ; `ZONE_CATEGORIES` sans opérateur figé ; tests adaptés (les deux opérateurs vus pour « naturelle », opérateur conforme aux valeurs montrées).
- [x] `training/src/families.rs` -- D1 : `refus_stats_raster` (branche sortie, tournures neuves), `refus_hors_registre` 9 → 6 ; test : refus seul, MNT présent, tournures à deux statistiques et terrain-puis-statistique vues.
- [x] `training/src/families.rs` -- D3 : `question_tampon_sans_distance` ; test : question, aucun chiffre dans la demande, couche cible présente.
- [x] `docs/entrainement-du-modele.md` -- décisions, comptes avant/après.
- [x] `~/cassini-entrainement/nuit-apres-5-49.sh`, `fusion-apres-5-49.sh` -- copies adaptées, non lancées.

**Acceptance Criteria:**
- Given le jeu par défaut (graines 1 et 2), when il est engendré, then 8 600 exemples, 23–27 % de gros projets, aucun prompt au-delà de 19 000 caractères, et les comptes par famille touchée sont relevés avant/après.
- Given la recette locale (fmt, clippy des 4 crates, tests Rust, tests Python du pipeline et de l'extension, garde de l'extension), then verte, `recouvrement` compris.
- Given les scripts de nuit, then ils écrivent dans `ajuste-apres-5-49-a/-b/-fusion` et n'installent rien.

## Design Notes

Part d'une famille ≈ poids / 373 du jeu (gros projets compris) : 5 → ≈ 115 exemples, 3 → ≈ 69.
Poids total 501 → 505 : dilution de 0,8 % des autres familles, `gros_projet` 25,3 %.

## Verification

**Commands:**
- `cargo fmt --all --check && cargo clippy -p cassini-core -p cassini-ai -p cassini -p cassini-training --all-targets -- -D warnings` -- sans avertissement
- `cargo test -p cassini-core -p cassini-ai -p cassini -p cassini-training --locked` -- vert
- `python3 -I -m unittest discover -s training/entrainement/tests` -- vert
- `cargo run -p cassini-training --release -- --seed 1 --count 8600 --out <scratch>/apres.jsonl` -- comptes relevés

## Implementation Notes

- D4 : `zone_operator` et `in_zone_category` (une valeur est de la catégorie si elle est le code
  ou le code suivi d'un indice en minuscule : `AUc` n'est ni agricole ni urbain) ;
  `zone_category_select` partagé par `zone_category` (`dans_une_zone`…) et `draft_filter_eq`.
  Précision de réalisation : `N` forcé trois fois sur quatre retire aussi `Nh` des valeurs
  montrées (CNIG 2017) ; sans ce retrait, `Nh` sortait au hasard du tirage des valeurs et
  « naturelle » ne donnait `eq N` que 3 fois au jeu (17 `begins_with`). Avec : 13 `eq`, 7
  `begins_with`.
- D1 : `refus_hors_registre` tire ses quatre branches restantes une fois sur quatre (0, 1, 3, 5) ;
  au poids 6, chacune garde 1,5 unité de poids, comme avant (1/6 × 9).
- D5 (ajout du coordinateur, 2026-11-11, essai QGIS avec `ajuste-5-49-a`, à confirmer par le
  propriétaire) : `gisement` 3 → 5 ; `grid_point` (WGS84 deux fois sur trois, un point WGS84 sur
  trois nommé devant ses coordonnées : `named_landmark`, lieux génériques, ni gare ni Annecy) ;
  tournures « azimut du quadrillage », « nord du quadrillage », « nord UTM » des deux côtés ;
  test `every_grid_bearing_says_grid_with_named_wgs84_places_and_utm_north`. Le test existant
  accepte désormais « nord utm » comme marque du quadrillage (à côté de « gisement » et
  « quadrillage »). **« Azimut grille » non posé** : le test du gisement interdit « grille »
  (tournure du carroyage du PDF, garde de 5.49) ; l'ajouter aurait affaibli cette garde.
- Poids total 501 → 508 (et non 505 : D1 +2, D3 +3, D5 +2). Gros projets 24,4 % (2 096).
- Comptes avant/après : `docs/entrainement-du-modele.md`, « Générateur après 5.49 ».

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : lentilles `quick` et `adversarial-donnees`, ce dernier ayant engendré les graines 1 à 10).
Verdicts : high 0, medium 6, low 6, false 1, maybe-false 0. Corrigés 8 (patch), reportés 3, rejetés 2.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Poids de `gisement` changé hors du bloc gelé (D5) | false | reject | D5 vient de l'appelant (constat du coordinateur, 2026-11-11), tranchée en party, notée à confirmer ; bloc gelé non retouché |
| `named_landmark` : communes (Annecy, Niort…) sur des coordonnées fausses | medium | patch | noms génériques seuls |
| Lieu nommé + WGS84 seulement avec `north: "grid"` (79 / 0 / 0) | medium | patch | même tirage de points dans `distance_azimut` et `azimut_magnetique` |
| Trois têtes de la question absentes de `tampon` (0 plan) | medium | patch | têtes ajoutées à `tampon_head` avec distance |
| Quasi-copies de RA09 (« Crée un tampon autour des routes ») | medium | patch | thèmes du réseau routier exclus de la famille de question |
| Quasi-copies de S15 et S23 dans `refus_stats_raster` | medium | patch | paire moyenne/maximale, « Fais la pente, puis … » et l'unité retirées |
| S24 à un mot dans la branche 8 (déplacée telle quelle) | low | defer | antérieur au changement (5.41) ; `deferred-work.md` |
| « statistiques » seulement dans des refus | low | reject | forme « statistiques du MNT » antérieure ; aucun essai ne la contredit, correctif dans une autre famille |
| Accord « zone tampon … fusionné » | low | defer | `dissolve_words` partagé, antérieur ; `deferred-work.md` |
| « prendre du refuge du col » | low | patch | « depuis {pa} » |
| Doc : « fourchette du hasard » non étayée, graine 2 absente | low | patch | recompte d'un seul script, écart sur graines 3–5, graine 2 |
| Nuit : sortie de recette réutilise `recette-5-49\` (vieux scores) | medium | patch | dossier neuf, `rm -f`, garde du commit de `CASSINI-nuit` |
| `grid_zone_words` : zone forcée à 6–7° du point | low | defer | antérieur (10.28) ; `deferred-work.md` |
