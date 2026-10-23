---
title: 'Re-entrainement, tache A : familles manquantes et tour de redemande dans le generateur'
type: 'feature'
ticket: '15'
created: '2026-10-23'
status: 'built'
baseline_revision: '401eba1ec109142b902ce19e5f803115761ad0a7'
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

**Problem:** Le modèle ajusté de 5.10 échoue `plan_reel` (16/25) : le générateur ne lui a jamais appris à refuser ce que le registre ne sait pas faire, ni les plans à trois conditions, ni la question quand l'appartenance à une commune ou l'origine d'une isochrone manque, ni le tour de redemande, ni les formes d'inventaire que rend QGIS.

**Approach:** Tâche A de 5.15 (découpe du propriétaire, 2026-10-23 ; tâche B, ré-entraînement et mesures, dans `deferred-work.md`). Ajouter ces familles et ces formes au générateur, rendre au prompt court la précision « (en comptant à partir de 1) », écrire des exemples de redemande, et tenir `ai/` égal à l'entraînement octet pour octet. Décisions du propriétaire : 8 000 exemples d'entraînement ; fusion dans `main` seulement après la tâche B, sur la même branche.

## Boundaries & Constraints

**Always:** Une phrase mal comprise devient une famille, jamais une règle de prompt. Le jeu tenu à l'écart n'est jamais lu : H et Q ne nomment que des constats. Les phrases de `plan_reel`, `question_reel` et `duree_reel` ne sont pas recopiées : les familles visent leur sorte de demande avec d'autres tournures, et le test de recouvrement couvre aussi ces trois jeux. Vocabulaire civil (CLAUDE.md §2.9). Prompt court et tour de redemande écrits dans `training/src/prompt.rs`, copiés dans `ai/src/plan.rs`, égaux octet pour octet sur tout le jeu engendré. Générateur déterministe. Aucune dépendance ajoutée.

**Never:** Entraîner, mesurer, faire la démo (tâche B). Ajouter une commande au registre. Toucher `judge`, la grammaire, `compose_plan_with`, `retry_turns` (AD-23), `tests/epreuve-modele-local/` ou `ai/tests/tenu_a_l_ecart/`. Écrire dans un fichier de ticket. Fusionner dans `main`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Hors registre | champ de vision, ligne de vue, pente, cadrage de la carte, PDF en A3, statistiques sur un raster (MNT présent) | refus affirmatif, jamais un plan de substitution | — |
| Commune sans couche | tournure d'appartenance explicite, aucune couche de communes | question | — |
| Commune avec couche | même tournure, couche de communes présente | sélection de la commune, puis intersection | — |
| Commune en contexte | « … à Orly » | sans effet sur le plan | — |
| Isochrone sans coordonnées | origine nommée par un lieu, sans coordonnées | question qui demande le point en coordonnées | — |
| Trois conditions | zone, filtre de la cible, distance, PDF | 4 à 6 étapes, aucune perdue ni ajoutée | — |
| Une seule étape | demande d'une opération qui cite une autre couche en contexte | une étape | — |
| Redemande | sortie corrompue rejetée par `check_plan`, puis écarts | plan juste en réponse | corruption acceptée par `check_plan` : exemple écarté |
| Inventaire de QGIS | sans `source`, projet sans nom ou sans SCR, SCR non EPSG, vecteur sans géométrie, `unloaded_files` vide | plan inchangé | — |

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- `FAMILIES` l.32 ; aides `select`/`buffer`/`intersect`/`pdf`/`step`/`refusal` l.142-180 ; `draft_multi` l.992 (3 à 5 étapes) ; `isochrone` l.1258 ; `question_couche_absente` l.1540 ; `refus` l.1666 (un seul A3, figé). `finish(rng, body, q, place)` ajoute la commune de contexte.
- `training/src/inventories.rs` -- `Builder::add` l.1349 trie des indices : trier les **valeurs** comme `core/src/agent_inventory.rs` l.165 (`sort_unstable` sur `&str`). `Layer::to_json` l.1183, `finish` l.1396 : formes de QGIS, telles que `agent_inventory` les rend (clé absente, jamais `null`, sauf `geometry`). Thème raster `mnt` l.1131.
- `training/src/prompt.rs` -- `SYSTEM` l.17 ; `render_prompt` l.49 ; ajouter le rendu du prompt de redemande.
- `training/src/lib.rs` -- `Example` (dans `families.rs` l.17), `validate` l.55, `generate` l.114, `to_chat_line` l.189, test des familles l.320.
- `ai/src/plan.rs` -- `SHORT_SYSTEM` l.281 (copie) ; `retry_turns` l.322 et `judge` l.346 (format des écarts : `format!("étape {} : {}", e.step, e.message)`), lecture seule ; tests de parité l.855+.
- `training/tests/forme_inventaire.rs` -- l.140-142 exige un seul jeu de clés : passer à « chaque forme rendue par le produit est vue à l'entraînement ».
- `training/tests/recouvrement.rs` -- étendre aux phrases de `ai/tests/{plan_reel,question_reel,duree_reel}.rs`.
- `training/entrainement/pipeline.py` l.61 -- `TRAINING_COUNT` à 8 000 ; `docs/entrainement-du-modele.md` -- familles et commande.

## Tasks & Acceptance

**Execution:**
- [x] `training/src/inventories.rs` -- valeurs triées ; formes de QGIS (une part des exemples) ; `unloaded_files` vide sauf quand la famille ouvre un fichier -- dettes de 5.10
- [x] `training/src/families.rs` -- `refus_hors_registre`, `appartenance_commune`, `question_isochrone`, `etape_unique`, `draft_multi` étendu (trois conditions, PDF, jusqu'à 6 étapes) ; tests unitaires -- ce qui manque au modèle
- [x] `training/src/prompt.rs`, `training/src/lib.rs` -- précision « (en comptant à partir de 1) » ; exemples de redemande -- apprendre le tour
- [x] `ai/src/plan.rs` -- `SHORT_SYSTEM` mis à jour ; test de parité du prompt de redemande contre `retry_turns`
- [x] `training/tests/forme_inventaire.rs`, `training/tests/recouvrement.rs` -- garde-fous étendus
- [x] `training/entrainement/pipeline.py`, `docs/entrainement-du-modele.md` -- 8 000 exemples, familles documentées

**Acceptance Criteria:**
- Given la graine 1 et 8 000 exemples, when on génère le jeu, then chaque famille nouvelle y figure, aucun exemple n'est écarté, tous passent `check_plan` et deux générations sont identiques à l'octet.
- Given le jeu engendré, when le test de recouvrement tourne, then aucune phrase du jeu tenu à l'écart, de `plan_reel`, de `question_reel` ni de `duree_reel` n'y figure (texte normalisé, chiffres et communes masqués).
- Given les 8 000 exemples, when les tests de parité de `ai` tournent, then les tours de `ai` (premier essai et redemande) valent le prompt d'entraînement octet pour octet.
- Given la recette complète, when elle tourne, then elle est verte.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| # | Lentille | Constat | Verdict | Route | Preuve |
|---|---|---|---|---|---|
| 1 | blind | `question_isochrone` : « de le centre de X » (attendu et demande) | medium | patch | `origin = "le centre {de_place}"` placé après « de » dans les gabarits 0, 2 de la question et 4 de la demande. |
| 2 | blind | « sur/dans la commune de X » (contexte, `finish`) contre « sur le territoire de X » (étape) : signaux contraires | medium | patch | `finish` l.323 ajoute ces deux prépositions sans étape ; `belonging` crée une étape pour un quasi-synonyme. |
| 3 | edge | `conditions_multiples` avec PDF : commune de contexte ajoutée après la clause PDF | medium | patch | `finish(rng, &head, q, true)` après `head.push_str(&clause)` ; `export_pdf` passe `false`. |
| 4 | edge | `refus_hors_registre` : « Calcule le champ de vision depuis le point X / Y (Lambert-93) » est le début de S24 ; « Centre la carte sur les communes » à un mot de S10 | medium | patch | `sentences.json` S24 et S10 relus ; la comparaison masquée n'attrape pas un préfixe. |
| 5 | blind, intent | Refus de terrain, cadrage et A3 non liés au registre : rien n'échoue quand l'étape 6 ajoute ces commandes | medium | patch | Aucun test ne lit `Registry::standard().specs()` ; CLAUDE.md §6 prévoit `terrain.*`. |
| 6 | blind | `corrupt` ne produit jamais `step:0`, l'erreur que vise « en comptant à partir de 1 » | medium | patch | `k = i + 1 + below(2)` : soi-même ou suivante seulement ; les Design Notes disent « `step:k` hors plage ». |
| 7 | blind, edge | La doc promet le PDF « une fois sur quatre » sur zone + deux distances, impossible à 6 étapes | low | patch | `with_pdf` exige `calls.len() < MAX_DRAFT_STEPS` ; correction de texte. |
| 8 | blind | Plancher du test de parité de redemande à 2 % au lieu de 4 % | low | patch | `checked * 50 > len` contre le plancher de 4 % du test de `lib.rs`. |
| 9 | verification-gap | La ligne chat d'une redemande (`to_chat_line`) n'est pas testée | medium | patch | Constat déposé : remettre `render_prompt` dans `to_chat_line` ne fait échouer aucun test. |
| 10 | verification-gap | Le plafond de 6 étapes d'`export_pdf` n'est pas testé | low | patch | Constat déposé : retirer la garde laisse passer 7 étapes (`validate` borne à 12). |
| 11 | verification-gap, intent | « Aucun exemple écarté » vérifié sur 3 000 exemples, pas 8 000 | low | patch | `no_family_produces_rejected_examples` appelle `generate(DEFAULT_SEED, 3000)`. |
| 12 | verification-gap, edge | Parité au jeton du prompt de redemande avec le vrai GGUF non vérifiée | maybe-false (medium) | defer | Le test sur vrai modèle ne construit que `first_turns` ; il faut le GPU et le modèle (tâche B). |
| 13 | blind, edge | `MAX_LENGTH = 2048` non remesuré avec les redemandes | maybe-false (medium) | defer | `data.drop_too_long` compte `ecartes_trop_longs` (pas de troncature muette) ; à lire dans le rapport de la tâche B. |
| 14 | blind, edge | Raster avec `feature_count` dans deux inventaires sur trois | false | rejet | Les deux formes sont apprises ; l'inventaire de l'épreuve (`tests/epreuve-modele-local/inventory.json`) porte la clé, le produit non. |
| 15 | blind | Redemandes toujours à un seul écart | low | rejet | `judge` n'en rend plusieurs que pour les paramètres de sortie ; il faudrait une corruption de plus. |
| 16 | blind, edge | Corruption « identifiant inconnu » toujours laissée | low | rejet | Documenté ; seulement un rendement moindre, compensé par le poids 20. |
| 17 | blind | `a_corruption_that_check_plan_accepts_is_left_out` ne passe pas par `redemande` | low | rejet | `validate_retry` rejette déjà une corruption acceptée ; le constat 11 l'attrape sur le jeu entier. |
| 18 | blind | `recouvrement.rs` lit les phrases en analysant du Rust | low | rejet | `the_real_trials_are_read_whole` vérifie les comptes et trois textes exacts. |
| 19 | blind | Peu de variété dans la branche zone + deux distances | low | rejet | Pas de défaut ; il faudrait des gabarits de plus. |
| 20 | blind | Incohérences du fichier de plan (cases, notes) | — | rejet | La correction modifie ce plan. |
| 21 | blind | Plan en mode 100755 | false | rejet | `core.filemode=false` ; les fichiers suivis du dossier sont en 100644. |
| 22 | blind | Coût des tests à 8 000 exemples | low | rejet | Pas de défaut. |
| 23 | intent | Isochrone sans aucune origine non couverte | false | rejet | La matrice figée nomme « origine nommée par un lieu, sans coordonnées ». |
| 24 | intent | Sortie rejetée toujours canonique, jamais brute | low | rejet | Une sortie brute de modèle ne s'engendre pas sans modèle ; le gabarit est égal octet pour octet. |

## Design Notes

Redemande : partir d'un exemple `calls` valide, corrompre son plan (identifiant de couche inconnu, `step:k` hors plage, étape retirée qui casse une référence), garder la corruption seulement si `check_plan` la rejette, puis écrire le prompt complet (système, demande, sortie rejetée en tour assistant, tour des écarts au texte exact de `retry_turns`) avec le plan juste pour réponse. Une part d'environ 5 % du jeu, environ 400 exemples. La perte ne porte que sur la réponse finale ; `data.py` lit `prompt` et `completion` sans changement.

Appartenance : seule une tournure explicite (« qui appartiennent à la commune de X », « sur le territoire de X ») crée une étape ; une commune citée seule reste un contexte, comme dans la phrase de démo.

## Verification

Environnement : `training` sous WSL ; `ai` sous PowerShell (`LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`, voir 5.10 t4b).

**Commands:**
- `cargo test -p cassini-training`, `cargo clippy -p cassini-training --all-targets -- -D warnings`, `cargo fmt --check` (WSL) -- expected: succès
- PowerShell : `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check` -- expected: succès
- `python3 -I -m unittest discover -s training/entrainement/tests` -- expected: succès
- `cargo run -p cassini-training --release -- --format chat --seed 1 --count 8000 --out <scratch>/a.jsonl` deux fois, `cmp` -- expected: identiques ; résumé sans exemple écarté
