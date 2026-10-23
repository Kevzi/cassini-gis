---
title: '5.15 tache B : re-entrainement, mesures sur GPU et demo QGIS'
type: 'feature'
ticket: '15'
created: '2026-10-23'
status: 'built'
baseline_revision: '4484464d86387e31087ef8c5837cc349a60f4fa4'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/docs/entrainement-du-modele.md', '{project-root}/_bmad-output/initiative-cassini-mvp/plan-5-10-t4b-mesures-et-demo.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le générateur de la tâche A (commit `4484464`) apprend les familles manquantes de 5.10, mais aucun modèle n'a encore été entraîné dessus. Sur cette branche, le prompt court a changé, et le modèle livré par 5.10 ne lui correspond plus.

**Approach:** Ré-entraîner le modèle par la commande unique (8 000 exemples, même base, même méthode), puis le fusionner en GGUF. Re-mesurer ensuite la parité des découpeurs, redemande comprise, ainsi que les quatre essais réels. Rejouer la démo dans QGIS avec la phrase exacte et consigner les chiffres. Barre du ticket : `plan_reel` ≥ 20/25 avec au plus 1 fausse question, `ecart_reel` ≥ 32/40, `question_reel` ≥ 9/12, phrase E rendue en question.

## Boundaries & Constraints

**Always:**
- Une phrase mal comprise est consignée comme constat (forme de l'écart), jamais corrigée par une règle de prompt ni par un correctif par phrase.
- Le jeu tenu à l'écart n'est jamais lu en clair.
- Les essais réels se lancent avec `--features cuda`, sauf `duree_reel` (processeur, 4 fils).
- Tout ce qui compile `ai/` passe par PowerShell.
- Les sorties d'entraînement vivent hors du dépôt.
- Vocabulaire civil.

**Never:**
- Changer le générateur, le prompt, `judge`, `retry_turns` ou les seuils des tests.
- Écrire dans un fichier de ticket.
- Fusionner dans `main` avant la recette du propriétaire.
- Ajouter une dépendance.
- Toucher au modèle 9B ni ajouter Ministral dans `models/`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Entraînement | dossier de sortie vide hors du dépôt | GGUF fusionné Q4_K_M, deux déclarations, `rapport.json` avec `ecartes_trop_longs` = 0 | arrêt en cours : `--reprendre` ; écartés > 0 : consigné, pas de relance |
| Parité | 300 premiers exemples, plus chaque redemande des 8 000 | jetons de `ai` = jetons du prompt d'entraînement lu d'un tenant | écart : arrêt, rien n'est mesuré |
| Barre atteinte | les quatre essais réels | chiffres consignés, démo rejouée, modèle installé pour la recette | — |
| Barre manquée | un seuil sous la barre | chiffres et formes des écarts consignés, puis arrêt et compte rendu | jamais de pansement de prompt |

**Décisions du propriétaire (2026-10-23) :**
- Lancement : maintenant, carte graphique laissée libre (pas de jeu) pendant l'entraînement.
- Barre manquée : consigner et s'arrêter ; une nouvelle passe sur le générateur fera l'objet d'un autre ticket.
- Barre atteinte : Claude installe le modèle neuf dans `models/` de la machine (l'ancien 4B gardé à côté, sous un autre nom) et met à jour le noyau, pour la recette du propriétaire dans QGIS.

</frozen-after-approval>

## Code Map

- `training/entrainement/pipeline.py` -- commande unique sous WSL : `UV_PROJECT_ENVIRONMENT=~/.cache/cassini-entrainement/venv uv run --project training/entrainement python training/entrainement/pipeline.py --sortie ~/cassini-entrainement/5-15-b`. Elle produit les GGUF, `cassini-composition-4b.modele.json` et `rapport.json` (`donnees.ecartes_trop_longs`, `jetons_max`). 5.10 : 3 h 31 pour 5 000 exemples.
- `training/entrainement/parite_decoupeurs.py` -- `--gguf <sortie>/qwen3.5-4b-base-Q4_K_M.gguf --jeu <sortie>/travail/donnees/validation.jsonl`.
- `ai/src/plan.rs` l.995-1040 -- `the_short_prompt_tokens_are_those_of_the_training_prompt_read_in_one_piece` (variable `CASSINI_TEST_MODEL`, 300 exemples). Étendre ce test aux exemples `retry` des 8 000 sur le modèle de l.884-905 : `judge` → `retry_turns` → `chat_tokens`, comparé à `vocab.tokenize(prompt_text(e), false, true)`.
- `ai/tests/{plan_reel,ecart_reel,question_reel,duree_reel}.rs` -- commandes et seuils dans leur en-tête ; `CASSINI_TEST_MODEL` = la déclaration `*.modele.json`. Pour `duree_reel` : `CASSINI_TEST_THREADS=4`, `CASSINI_TEST_INVENTORY=../donnees-test-cassini/inventaire-villeneuve-traduit-4b.json`, affinité `0x55`.
- `qgis-plugin/tests/probe_demo.py` -- la démo pilotée par Claude, avec `python-qgis-ltr.bat`, `CASSINI_CORE`, `CASSINI_MODELS` et le projet `donnees-test-cassini/projet-test-villeneuve.qgz`.
- `docs/entrainement-du-modele.md` -- ajouter une section « Tâche B de 5.15 : mesures », de même forme que « Tâche 4 : mesures » (parité, tenu à l'écart, clarification, durée, démo, bilan contre la barre).
- Copies Windows : `C:\Users\kevin\cassini-modeles\ajuste-5-15\` (sorties) et un dossier de démo par liens physiques, comme `demo-4b-fusionne\`. Le binaire CUDA de mesure va sous `C:\ct\mesures-4b\`.

## Tasks & Acceptance

**Execution:**
- [x] `~/cassini-entrainement/5-15-b/` -- lancer la commande unique en arrière-plan (journal `~/cassini-entrainement/5-15-b.log`), puis lire `rapport.json` -- produire le modèle
- [x] `ai/src/plan.rs` -- étendre le test de parité au jeton aux redemandes -- constat 12 de la tâche A
- [x] PowerShell -- lancer la parité (script et test), puis `plan_reel`, `ecart_reel`, `question_reel` (cuda) et `duree_reel` (processeur) -- mesurer
- [ ] `qgis-plugin/tests/probe_demo.py` -- rejouer la démo avec la phrase exacte, sur le modèle neuf -- démo de bout en bout
- [x] `docs/entrainement-du-modele.md` -- consigner les chiffres, les formes des écarts et le bilan contre la barre -- trace

**Acceptance Criteria:**
- Given le modèle neuf, when la parité tourne, then 300/300 premiers essais et toutes les redemandes sont identiques au jeton.
- Given les quatre essais réels, when ils tournent sur GPU, then chaque `SCORE` est consigné, avec le bilan seuil par seuil.
- Given la démo, when `probe_demo.py` tourne avec la phrase exacte, then le plan, les étapes, le PDF et le journal sont consignés.
- Given la recette, when elle tourne, then elle est verte.

## Implementation Notes

- Mise en œuvre directe, sans sous-agent : l'entraînement et les mesures s'étalent sur des heures, qu'un sous-agent ne tient pas.
- Entraînement lancé le 2026-10-23 à 12:08 sous WSL ; journal dans `~/cassini-entrainement/5-15-b.log`.
- Parité au jeton, redemande comprise : `ai/src/plan.rs` étend le test sur vrai modèle aux redemandes des 8 000 exemples. Il est passé sur le processeur avec la base de 5.10 (`ajuste-5-10-t2\qwen3.5-4b-base-Q4_K_M.gguf`), car la base, sa révision et le découpeur sont les mêmes : 300 prompts identiques, 349 redemandes identiques.
- Longueur : 8 000 exemples d'entraînement et 300 de validation, aucun écarté car trop long (`MAX_LENGTH` = 2048 suffit, constat 13 de la tâche A).
- Entraînement terminé à 16:08 en 241 min. Validation : 299/300 (calls 231/232, questions 37/37, refus 31/31) ; `jetons_max` 1521. Sorties copiées dans `C:\Users\kevin\cassini-modeles\ajuste-5-15\`.
- Parité des découpeurs (`parite_decoupeurs.py`) : 300/300, 0 écart.
- GPU, déclaration fusionnée (prompt court) : `plan_reel` 25/25 et 0 question ; `question_reel` 12/12 ; `ecart_reel` 32/40 (plans 19/26, questions 8/8, refus 5/6, 316 s), 1 redemande, 0 sauvée. Écarts : H08 et H23 en question au lieu d'un plan ; H17, H18 et H20 avec une étape perdue ; H19 sur le paramètre `operator` ; H22 sur la commande de l'étape 2 ; H40 en plan au lieu d'un refus.

- CPU (`duree_reel`, affinité `0x55`, 4 fils, inventaire de Villeneuve traduit) : moyenne 24,0 s hors amorce ; A0 37,7 s ; D en question ; **E en plan de 2 étapes après redemande**, au lieu d'une question.
- Forme de E, relevée par `C:\ct\release\cassini.exe plan` (cuda) avec la déclaration neuve : `analysis.isochrone` avec une origine inventée (EPSG:4326, x 2,458, y 48,658), puis `analysis.intersect` des parcelles.
- Barre manquée sur E seulement. Décision du propriétaire appliquée : consigner et s'arrêter. Démo QGIS non rejouée, modèle non installé ; chiffres consignés dans `docs/entrainement-du-modele.md` (« Tâche B de 5.15 »).
- Recette : `cargo test --workspace`, clippy, `cargo deny` (PowerShell), `fmt` et tests Python (WSL) verts.

## Plan Change Log

## Review Triage Log

Verdicts : 0 high, 4 medium, 6 low, 4 false, 0 maybe-false.

| # | Lentille | Constat | Verdict | Route | Preuve ou action |
|---|---|---|---|---|---|
| 1 | blind, edge, vgap | La boucle de redemande ignore le point d'instantané (`points[0] == head_end`) | low | patch | Assertion ajoutée ; test relancé, 349 redemandes identiques. |
| 2 | blind, edge | `unwrap_err` sans la demande, et écarts non comparés à `retry.errors` | low | patch | `expect_err(&e.request)` et `assert_eq!(errors, retry.errors)` ajoutés. |
| 3 | blind | Le commentaire du test ne parle que des premiers essais | low | patch | Commentaire étendu aux redemandes de `DEFAULT_COUNT`. |
| 4 | blind, edge, vgap, intent | Parité au jeton de `ai` lancée sur la base de 5.10, pas sur le modèle neuf ; la doc ne le dit pas | medium | patch | Relancé sur `ajuste-5-15\qwen3.5-4b-base-Q4_K_M.gguf` : 300 + 349 identiques ; la doc nomme le fichier. |
| 5 | blind | Le recul de `ecart_reel` (33 → 32) n'est pas dit | low | patch | Phrase ajoutée à la doc. |
| 6 | blind | La durée sur processeur manque l'exigence des 30 s sans le dire | low | patch | Phrase ajoutée, renvoi à `deferred-work.md` (entrée existante de la durée de démo). |
| 7 | blind | Le ticket de suite n'est pas inscrit dans `deferred-work.md` | medium | patch | Entrée ajoutée (isochrone vers un lieu présent sans le point, plans à plusieurs conditions). |
| 8 | blind | Origine d'isochrone inventée acceptée par `judge`, sans signal | medium | defer | Décision d'architecture, hors de la portée de 5.15 ; inscrite dans `deferred-work.md`. |
| 9 | blind | Chemin Windows avec le nom d'utilisateur dans la doc publique | low | patch | Nouveau passage neutre ; deux chemins de ce type existaient déjà avant 5.15. |
| 10 | vgap | Le test sur vrai modèle rend la main sans modèle et compte comme réussi | low | rejet | Forme préexistante du test (5.10) ; il est documenté comme lancé à la main avec `CASSINI_TEST_MODEL`. |
| 11 | edge | `retries * 25 >= all.len()` passe à vide si le jeu est vide | false | rejet | `generate(DEFAULT_SEED, DEFAULT_COUNT)` rend 8 000 exemples, ce que `the_decided_families_and_vocabulary_are_present` vérifie. |
| 12 | blind | Parité vérifiée sur un jeu régénéré, pas sur les fichiers entraînés | false | rejet | Générateur déterministe (graine 1, 8 000, empreinte au rapport) ; `parite_decoupeurs.py` lit le fichier de validation produit. |
| 13 | edge, intent | Le critère « démo rejouée » n'est pas rempli | false | rejet | Décision du propriétaire dans le bloc figé : barre manquée, on consigne et on s'arrête. |
| 14 | blind | Plan en mode 100755 ; cases, notes de conception et statut non réconciliés ; ventilation de la validation, H40 non nommé | false / low | rejet | `core.filemode=false` ; correction du plan exclue ; H40 nommé dans la table des écarts ; la validation n'est pas un critère. |

## Design Notes

L'entraînement dure environ 5 à 6 h : 8 000 exemples et des séquences plus longues, contre 3 h 31 pour 5 000 en 5.10. Il tourne en arrière-plan sous WSL ; une coupure se reprend par `--reprendre`. Les mesures commencent seulement quand `rapport.json` existe.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check` (PowerShell) ; `cargo fmt --check` et `python3 -I -m unittest discover -s training/entrainement/tests` (WSL) -- expected: succès
- `CASSINI_TEST_MODEL=<base Q4_K_M> cargo test -p cassini-ai --release plan::tests::the_short_prompt_tokens` -- expected: parité totale, redemandes comprises
- `cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture plan_reel` (puis `ecart_reel`, `question_reel`) -- expected: lignes `SCORE`
