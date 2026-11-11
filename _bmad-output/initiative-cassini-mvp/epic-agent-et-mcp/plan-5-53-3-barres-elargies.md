---
title: 'plan_reel et question_reel portées à 40-50 phrases écrites à la main, refus compris (livrable 3 de 5.53, lot 1)'
type: 'feature'
ticket: '53'
created: '2026-11-11'
status: 'built'
baseline_revision: 'e912a11e58fd112fd3e5c42a522855fb4a326391'
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

**Problem:** `plan_reel` (25 phrases) et `question_reel` (12) sont trop courtes pour juger un
oubli : à « une phrase près » (non-oubli), un raté pèse 4 à 8 %. `question_reel` n'attend aucun
refus ; la barre par commande non plus (`deferred-work.md`, relecture du 2026-11-11).

**Approach:** porter ces deux barres à 40-50 phrases écrites à la main, chacune avec son plan, sa
question ou son refus attendu, dont au moins 3 refus par barre ; garder les sources gelées
intactes ; protéger les phrases par `recouvrement`.

Décisions du propriétaire (2026-11-11) :
- D1 : les phrases neuves vont dans un `phrases.json` par barre (`ai/tests/plan_reel/`,
  `ai/tests/question_reel/`), schéma de `par_commande/phrases.json`.
- D2 : `plan_reel` est élargie en place : le test lit `tests/epreuve-modele-local/sentences.json`,
  intact, plus son JSON ; un seul score sur le total. Même chose pour `question_reel`
  (`AMBIGUOUS` + JSON).
- D3 : au moins 3 refus attendus par barre, `question_reel` comprise ; la barre par commande
  n'est pas touchée ; l'entrée reportée « aucune phrase n'attend un refus » est close par ce
  livrable.
- D4 : seuils proportionnels aux décisions passées, arrondis vers le bas : `plan_reel` 80 % des
  plans et au plus 2/25 (prompt long) ou 1/25 (prompt court) fausses questions, rapportés au
  total ; `question_reel` 9/12 des réponses attendues.
- D5 : `ecart_reel` reste à 40 (auteur distinct du générateur).
- D6 : livrable 3 découpé en trois lots ; ce plan est le lot 1. Lots 2 (`echelle_utm_reel`,
  `terrain_reel`) et 3 (`epic11_reel`, `style_reel`, puis remesure du parent) sont dans
  `deferred-work.md`.

## Boundaries & Constraints

**Always:** vocabulaire civil (§2.9) ; phrases neuves, jamais engendrées ni recopiées d'une autre
barre ; chaque attendu passe les contrôles de `par_commande_reel` (`check_plan`, garde-fou des
coordonnées, complétion sans effet, couches, champs et rasters présents) ; inventaire de
l'épreuve tel quel ; `question_reel` ne nomme aucun fichier non ouvert ; le propriétaire relit
les phrases de terrain et de géométrie avant le commit.

**Never:** modifier `tests/epreuve-modele-local/`, le générateur, la barre par commande ou les
barres des lots 2 et 3 ; envoyer une phrase de mesure au jeu d'entraînement ; charger un modèle
ou utiliser le GPU ; toucher `~/cassini-worktrees/reentrainement-5-49` ou `CASSINI-nuit` ; pousser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Barre élargie | test sans modèle | vert, total 40-50, ≥ 3 refus attendus | — |
| Attendu invalide | plan attendu qui échoue un contrôle | rouge, nommant l'id | — |
| Refus devenu faux | commande qu'un refus dit absente ajoutée au registre | rouge, nommant l'id | — |
| Phrase engendrée ou recopiée | phrase au jeu par défaut, ou dans deux barres | `recouvrement` rouge | identifiant seul |
| Sources gelées | `git diff` sur `tests/epreuve-modele-local/` | vide | — |

</frozen-after-approval>

## Code Map

- `ai/tests/par_commande_reel.rs:52-237,345-375` -- `expected_calls`, `known`, `references`, `invalid`, `normalize_numbers`, `comparable` : à déplacer dans `ai/tests/common/` et réutiliser, pas recopier ; `par_commande_reel` les appelle ensuite, ses phrases et son essai inchangés.
- `ai/tests/par_commande/phrases.json` -- schéma (`id`, `phrase`, `expected.kind` calls|question|refusal) ; ici sans `command`.
- `ai/tests/plan_reel.rs` -- 25 phrases, attendus en Rust `expected(id)` (refus S10 S15 S18 S23 S24), seuil l. 220 (`right * 5 < total * 4`, `question_cap(style)`) ; l'en-tête documente D4.
- `ai/tests/question_reel.rs` -- `const AMBIGUOUS` (12), seuil l. 99 (`questions >= 9`) ; un refus ou un plan y compte faux aujourd'hui : le score devient « réponses attendues ».
- `training/tests/recouvrement.rs:149-182,228,251` -- `real_trials` (comptes en dur), `from_phrases`, anti-recopie de la barre par commande, somme de `the_real_trials_are_read_whole`.
- `training/src/families.rs:9677` -- `what_is_refused_out_of_registry_is_still_missing_from_the_registry` : modèle du garde-fou des refus.
- `training/entrainement/non_oubli.py` -- ne pas modifier : totaux différents → `non comparable`.

## Tasks & Acceptance

**Execution:**
- [x] `ai/tests/common/` -- contrôles d'attendu de `par_commande_reel` et lecteur de `phrases.json` -- un seul contrôle pour toutes les barres.
- [x] `ai/tests/plan_reel/phrases.json` -- 15 à 25 phrases neuves (total 40-50), dont ≥ 3 refus portant `missing` (mots absents des ids du registre).
- [x] `ai/tests/question_reel/phrases.json` -- 28 à 38 phrases neuves (total 40-50), surtout des questions, ≥ 3 refus avec `missing`.
- [x] `ai/tests/plan_reel.rs`, `ai/tests/question_reel.rs` -- lire l'ancien jeu et le JSON, un seul `SCORE`, seuils D4, en-tête ; tests sans modèle : contrôles communs, total 40-50, ≥ 3 refus, mots `missing` absents du registre.
- [x] `training/tests/recouvrement.rs` -- deux sources, comptes, somme ; anti-recopie étendu à toutes les phrases écrites à la main.
- [x] `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- clore l'entrée « Barre par commande : aucune phrase n'attend un refus » (D3).
- [x] `docs/entrainement-du-modele.md` -- totaux et seuils des deux barres ; compte du jeu tenu à l'écart corrigé (28/8/4, la doc dit 26/8/6).

**Acceptance Criteria:**
- Given `cargo test -p cassini-ai --test plan_reel --test question_reel` sans modèle, then vert, 40-50 phrases chacune.
- Given la recette locale complète, then verte.

## Design Notes

**Remesure du parent installé, après le lot 3** (nuit finie, GPU libre, feu vert du
propriétaire) : worktree neuf au commit du lot 3 (jamais `CASSINI-nuit`) ; copie de
`%TEMP%\essais-5-49.ps1` pointée sur ce worktree, `par_commande_reel` ajouté à la liste ;
`$Model` = `*.modele.json` du modèle installé, `$Log` =
`~/cassini-entrainement/5-53-3-mesures/essais-gpu-parent.log` ; puis `non_oubli.py --parent` ce
relevé et la recette d'Annecy du parent. Tout candidat se mesure au même commit. D'ici là,
`plan_reel` et `question_reel` rendent `non comparable`.

## Verification

Environnement : `LIBCLANG_PATH` vers un `libclang.so`, `BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`.

**Commands:**
- `cargo fmt --all --check && cargo clippy -p cassini-core -p cassini-ai -p cassini -p cassini-training --all-targets -- -D warnings` -- sans avertissement
- `cargo test -p cassini-core -p cassini-ai -p cassini -p cassini-training --locked` -- vert
- `python3 -I -m unittest discover -s training/entrainement/tests` -- vert
- `git diff e912a11 -- tests/epreuve-modele-local/ training/src/ ai/tests/par_commande/` -- vide

**Manual checks:**
- Un attendu faussé, un refus dont la commande existe : le test nomme la phrase.

## Implementation Notes

- `ai/tests/common/mod.rs` porte désormais `read_phrases`, `expected_calls`, `expected_plan`,
  `known`, `references`, `invalid`, `refusals_now_wrong`, `registry_ids`, `normalize_numbers`,
  `comparable`, `same_calls` et `proportional`. `par_commande_reel` les appelle ; seul le contrôle
  « le plan attendu appelle sa commande » reste chez lui. Phrases, essai et sortie inchangés.
- `plan_reel` : 25 + 23 = 48 (19 plans neufs, 4 refus neufs PL20-PL23 : `mail`, `undo`,
  `geocod`, `contour` ; PL21 porte `redo`, `undo`). Les 5 refus de l'épreuve reçoivent `missing` ou `missing_params` dans `refusal_of`. Seuils : 38/48 plans, fausses questions ≤ 3 (long) ou ≤ 1 (court). Les
  25 phrases de l'épreuve gardent la comparaison exacte ; les neuves `common::same_calls`.
  Les deux tests remplacés (`every_expected_plan_is_valid_and_passes_the_coordinate_guard`,
  `every_expected_plan_is_left_unchanged_by_the_completion`) sont couverts par
  `every_expected_answer_is_valid` (contrôles communs sur les 48).
- `question_reel` : 12 + 35 = 47 (31 questions, 4 refus QR32-QR35 : `translat`, `mail`,
  `weather`/`meteo`, `watershed`/`basin`). Seuil 35/47. L'essai est renommé
  `question_reel_at_least_9_of_12_ambiguous_requests_get_the_expected_answer` (le filtre
  `question_reel` des scripts le trouve toujours). Une question du garde-fou compte toujours
  juste, comme avant.
- `recouvrement.rs` : sources groupées (`real_trials_by_source`) ; l'anti-recopie couvre les
  trois `phrases.json` de 5.53 (contre toute autre source et en double dans son propre fichier),
  et n'imprime que `fichier:id`.
- Relecture (2026-11-11), corrections : mots `missing` contrôlés (non vides, minuscules ASCII) ;
  `missing_params` pour une option absente (S18) ; titre attendu comparé exactement (PL16) ;
  valeur d'une égalité prise parmi les valeurs montrées du champ ; QR04, QR12, PL17, PL21
  réécrites pour qu'une seule réponse soit défendable ; fichier non ouvert replié comme la
  phrase, contrôlé aussi sur `plan_reel` ; tests sans modèle de la notation ; comptes de
  `HAND_WRITTEN` dans chaque entrée.
- À faire par le propriétaire avant le commit : relire les phrases de terrain et de géométrie
  (PL12-PL15, PL18, PL19, PL23 ; QR27-QR31, QR35).

## Plan Change Log

## Review Triage Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 0, medium 7, low 14, false 0, maybe-false 0. Corrigés 12 (patch, même implémenteur), reportés 2, rejetés 5.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Mot `missing` vide ou non ASCII fausse le garde | medium | patch | `is_keyword`, `malformed_refusals` |
| Titre d'export d'un attendu neuf jamais comparé (PL16) | medium | patch | titre exact si l'attendu en porte un |
| 5 refus de l'épreuve sans garde | medium | patch | `refusal_of` : zoom/cadr, `raster.zonal`, `missing_params` |
| Valeurs de sélection non comparées aux valeurs montrées | medium | patch | `Known` porte les valeurs ; `eq`/`ne` contrôlés |
| QR04, QR12, PL21 : plan ou autre réponse défendable | medium | patch | phrases remplacées |
| `answers`/`same_calls` testés seulement avec le modèle | medium | patch | tests sans modèle des titres, nombres et sortes |
| PL17 fige l'ordre de deux sélections indépendantes | medium | patch | réécrite en étape dépendante |
| Repli du nom de fichier non replié comme la phrase | low | patch | `folded` partagé, contrôle étendu à `plan_reel` |
| Assertions qui recopient la formule des seuils | low | patch | valeurs fixes seules |
| Nom de l'essai réel trompeur ; branche `calls` inutile | low | patch | renommé ; `calls` refusé par un test |
| Comptes appariés par `zip` | low | patch | compte dans chaque entrée de `HAND_WRITTEN` |
| « 80 % » / « 9/12 » au lieu de 38/48 et 35/47 | low | patch | doc et messages |
| Test anti-doublon sans cas négatif | low | defer | entrée reportée (partie inter-sources antérieure) |
| Comparaison stricte des défauts et de l'ordre | low | defer | déjà dans `deferred-work.md` (relecture du livrable 1) |
| Recompte du jeu tenu à l'écart (règle de lecture) | low | reject | compte seulement, pas le contenu ; auteur des barres, pas du générateur |
| Anti-copie sans le jeu tenu à l'écart ni `ecart_reel` | low | reject | ces sources sont gardées par `recouvrement` contre le générateur ; copies entre barres sans cas montré |
| Refactor de `par_commande_reel.rs` (frontière R4) | low | reject | voulu par la Code Map ; phrases et score inchangés |
| Synonymes manqués par le mot-clé `missing` | low | reject | limite écrite dans la doc |
| QR35 rougira à l'arrivée de `terrain.watershed` | low | reject | comportement voulu |
| Validation du propriétaire | — | propriétaire | phrases de terrain approuvées le 2026-11-11 (« ok pour les deux ») |

Intégration (2026-11-11) : réunies avec le livrable 4, PL14 et PL19 étaient produites par le nouveau générateur (« Repère les points culminants du terrain », « Convertis le point … en MGRS ») ; tournures très courantes, gardées au générateur ; les deux phrases de barre, jamais encore mesurées, sont reformulées (même demande, même attendu).
