---
title: 'Nouvelle passe sur le generateur, tache A : isochrone depuis un lieu de l''inventaire, plans a plusieurs conditions'
type: 'feature'
ticket: '17'
created: '2026-10-23'
status: 'built'
baseline_revision: '4ff027d2733dfd629472a23675ffb7010ff9937f'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/docs/entrainement-du-modele.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le modèle de 5.15 invente l'origine d'une isochrone quand le lieu de départ est une couche de points de l'inventaire (phrase E, « à 10 minutes à pied de la gare de X »), cas que seul le garde-fou de 5.16 rattrape. Sur le jeu tenu à l'écart, il perd des étapes dans les plans à plusieurs conditions (H17, H18, H20), se trompe d'opérateur ou de commande (H19, H22), pose une question au lieu d'un plan (H08, H23) et rend un plan au lieu d'un refus (H40).

**Approach:** Tâche A de 5.17 (découpe du propriétaire, 2026-10-23 ; tâche B, ré-entraînement, mesures et démo, dans `deferred-work.md`, sur la même branche, fusion dans `main` après B). Ajouter au générateur les familles qui manquent : une même tournure de trajet qui rend une question sans coordonnées et un plan avec, puis des plans à plusieurs conditions plus variés (opérateurs, cible filtrée et deux distances, inclusion dans une couche entière).

## Boundaries & Constraints

**Always:** Une phrase mal comprise devient une famille, jamais une règle de prompt. Le jeu tenu à l'écart n'est jamais lu : H et Q ne nomment que des constats. Les phrases de `plan_reel`, `question_reel` et `duree_reel`, phrase E comprise, ne sont pas recopiées (le test de recouvrement le garde). Vocabulaire civil. Générateur déterministe, aucun exemple écarté. Règles de chaînage du prompt suivies à la lettre. Une durée de trajet depuis un lieu sans coordonnées mène à une question ; une distance en mètres reste un plan. Aucune dépendance ajoutée.

**Décision (propriétaire, 2026-10-23, « le choix le plus judicieux », option a) — origines multiples :** « de chaque gare », « de toutes les gares », « des gares » → refus affirmatif (l'isochrone du catalogue part d'un seul point en coordonnées) ; « d'une gare » → question (laquelle, en coordonnées) ; « de la gare de X » → question.

**Never:** Entraîner, mesurer, faire la démo (tâche B). Toucher le prompt, `SHORT_SYSTEM`, `judge`, `retry_turns`, la grammaire, `core/src/point.rs`, `tests/epreuve-modele-local/` ou `ai/tests/tenu_a_l_ecart/`. Ajouter une commande. Écrire dans un fichier de ticket. Fusionner dans `main`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Lieu présent comme couche | cible, couche de gares et réseau, « à N minutes à pied de la gare de X » | question qui demande le point en coordonnées | — |
| Même tournure, point donné | « … à N minutes à pied du point x, y (Lambert-93) » | isochrone, puis intersection de la cible avec l'étape 1 | — |
| Origines multiples | « à 10 min à pied de chaque gare » | refus affirmatif, jamais un plan | — |
| Distance en mètres | « à moins de 500 m d'une gare » | tampon, puis intersection (inchangé) | — |
| Filtre non égal dans un plan à conditions | « … de plus de 12 m de haut … à moins de 300 m … » | `gt`/`lt`/`ne`/`contains` à l'étape du filtre | — |
| Cible filtrée et deux distances | — | 5 étapes, aucune perdue, ordre des règles de chaînage | — |

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- `FAMILIES` l.43 (poids). `draft_multi` l.1035 n'emploie que `eq` : y ajouter des branches, sous le plafond `MAX_DRAFT_STEPS` l.1206 (6). Réutiliser `numeric` l.474, les tournures de `filtre_different` l.704 et de `filtre_texte` l.666, ainsi que `near`, `area`, `filtered_reference`, `pair` et `pdf_clause`. `isochrone` l.1375 : une seule étape, sans cible ; extraire la fabrique du point et de la durée pour la réutiliser. `question_isochrone` l.2106 : la couche du lieu n'y est présente qu'une fois sur douze environ, et il n'y a aucune tournure « à N minutes de » ; questions attendues l.2160, ajouter `cassini_core::point::GUARD_QUESTION` parmi les variantes. `refus_hors_registre` l.1878, constantes de refus l.1867 : modèle pour le refus des origines multiples.
- `training/src/lib.rs` -- `validate` l.55 ; test « chaque famille présente » l.320.
- `training/tests/recouvrement.rs` -- inchangé, doit rester vert.
- `docs/entrainement-du-modele.md` -- tableau des familles l.73 et paragraphe « Depuis 5.15 » l.104.

## Tasks & Acceptance

**Execution:**
- [ ] `training/src/families.rs` -- famille `isochrone_vers_cible` (point donné : isochrone puis intersection de la cible) ; `question_isochrone` étendue (couche du lieu présente le plus souvent, cible citée, tournures « à N minutes {mode} de/du/de la … ») ; refus des origines multiples dans `refus_hors_registre` (décision a) ; tests unitaires -- phrase E et son contraire
- [ ] `training/src/families.rs` -- `draft_multi` : filtres `gt`/`lt`/`ne`/`contains`, cible filtrée et deux distances, distance et inclusion dans une couche entière ; poids de `conditions_multiples` relevé ; tests unitaires -- H17 à H22
- [ ] `docs/entrainement-du-modele.md` -- familles documentées, avec la note : viser des écarts nommés affaiblit `ecart_reel` comme mesure aveugle

**Acceptance Criteria:**
- Given la graine 1 et 8 000 exemples, when on génère deux fois, then les jeux sont identiques à l'octet, chaque famille y figure, aucun exemple n'est écarté et tous passent `check_plan`.
- Given le jeu engendré, when le test de recouvrement tourne, then aucune phrase tenue à l'écart, ni de `plan_reel`, `question_reel` ou `duree_reel`, n'y figure.
- Given le jeu engendré, when on le parcourt, then aucune question attendue n'a pour demande une distance en mètres, et chaque demande de trajet sans coordonnées attend une question.
- Given la recette complète, when elle tourne, then elle est verte.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (2026-10-23, lentilles blind-hunter, edge-case-hunter, verification-gap, intent-alignment) : high 0, medium 2, low 7, false 3, rejetés 10, différé 1.

| # | Lentille | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|---|
| 1 | blind, intent | Plan toujours « le point … », question toujours un lieu nommé : le modèle peut apprendre « lieu nommé → question » au lieu de « coordonnées → plan » | medium | patch | `isochrone_vers_cible` tire `origin_point(...).place` seul ; ajouter un lieu nommé avec ses coordonnées |
| 2 | verification-gap | Mode et durée de `travel`/`isochrone_call` non testés : inverser walk/car laisse tout vert | medium | patch | aucun test ne lit `mode` ni `unit` ; test ajouté |
| 3 | blind | « d'une gare » rend `GUARD_QUESTION` une fois sur trois, qui ne demande pas laquelle | low | patch | décision a : « question (laquelle, en coordonnées) » |
| 4 | blind, edge | Seuil ≥ 2/3 à ~1,5 σ du taux 25/36, compte aussi la cible | low | patch | seuil abaissé avec marge |
| 5 | blind | `gte`, `lte`, `begins_with` non vérifiés | low | patch | liste du test étendue |
| 6 | blind | `contains("des ")` du test de refus ne vérifie rien | low | patch | origine plurale vérifiée après le mode ou « depuis » |
| 7 | blind, edge | `has_meter_distance` ignore « km » et prend « périmètre » | low | patch | `dist()` rend des km ; mots entiers |
| 8 | blind | Titre PDF déduit réduit à « Les parcelles » dans `isochrone_vers_cible` | low | patch | `Draft.np` sans la condition de trajet |
| 9 | blind | `the_isochrone_still_starts_from_a_single_point` passe sur un `$ref` | low | patch | `origin` est `$ref` vers `$defs/Point` (5.16) |
| 10 | blind | Plusieurs points en coordonnées sans exemple de refus | maybe-false (medium) | defer | hors de la décision a ; à juger sur les mesures de la tâche B |
| 11 | blind | Part de `redemande` diluée sous les 5 % promis | false | rejet | graine 1, 8 000 : 388 redemandes, 4,9 % |
| 12 | blind | Refus existants dilués de 1/6 à 1/7 | low | rejet | poids de famille inchangé, aucune part promise par branche |
| 13 | blind | Constats H cités différemment entre doc et test | low | rejet | commentaire ; aucune harm nommée |
| 14 | blind | Règle « phrase E » de `travel_clause` sur une seule tournure | low | rejet | le test de recouvrement est la garde ; documenté |
| 15 | verification-gap | Ordre des intersections « couche entière » non vérifié | low | rejet | intersection commutative, inversion improbable, ajout de test |
| 16 | edge | « en auto » attrape « en automne » | low | rejet | test qui échoue bruyamment, aucune phrase concernée |
| 17 | edge | « sauf » enchaîné non vérifié | low | rejet | garde par construction (`chained`), régression improbable |
| 18 | edge | Branches 0 et 4 : câblage non vérifié | low | rejet | préexistant, inchangé |
| 19 | edge | Critère « chaque trajet sans coordonnées → question » contredit par le refus | false | rejet | décision a du propriétaire, dans le bloc figé |
| 20 | edge | Critère « aucune question sur une distance en mètres » faux pour `question_couche_absente` | low | rejet | correction = éditer ce plan ; le test vise les questions de coordonnées, sens de la matrice |
| 21 | intent | Symptômes au niveau du modèle non prouvés par ce diff | false | rejet | tâche B (mesures), découpe du propriétaire |
| 22 | intent | H08, H23 et la commande de H19/H22 sans contrepartie claire | low | rejet | le jeu tenu à l'écart n'est jamais lu ; contraste trajet et constat 1 sont la contrepartie possible, mesurée en B |

## Design Notes

C'est le contraste qui fait l'apprentissage. La même tournure de trajet rend une question sans coordonnées et un plan avec, et une distance en mètres reste un plan. La couche des gares est présente dans la plupart des exemples de question, parce que c'est là que le modèle de 5.15 invente.

## Verification

**Commands:**
- `cargo test -p cassini-training`, `cargo clippy -p cassini-training --all-targets -- -D warnings`, `cargo fmt --check` (WSL) -- expected: succès
- PowerShell : `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check` -- expected: succès
- `cargo run -p cassini-training --release -- --format chat --seed 1 --count 8000 --out <scratch>/a.jsonl` deux fois, `cmp` -- expected: identiques, résumé sans exemple écarté
