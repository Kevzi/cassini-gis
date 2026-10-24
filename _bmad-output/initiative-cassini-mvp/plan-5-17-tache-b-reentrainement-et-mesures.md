---
title: '5.17 tache B : re-entrainement, mesures sur GPU et demo QGIS'
type: 'feature'
ticket: '17'
created: '2026-10-23'
status: 'built'
baseline_revision: 'ac114284a43a5ad0cd4d9685a02245ee44814df3'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/docs/entrainement-du-modele.md', '{project-root}/_bmad-output/initiative-cassini-mvp/plan-5-15-tache-b-reentrainement-et-mesures.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le générateur de la tâche A (commit `ac11428`) apprend l'isochrone depuis un lieu de l'inventaire et des plans à plusieurs conditions plus variés, mais aucun modèle n'a été entraîné dessus. Le modèle de 5.15 invente l'origine de la phrase E (seul le garde-fou de 5.16 la rattrape) et manque `ecart_reel` de peu (32/40).

**Approach:** Ré-entraîner par la commande unique (8 000 exemples, même base, même méthode), fusionner en GGUF, re-mesurer la parité des découpeurs et les quatre essais réels, rejouer la démo dans QGIS avec la phrase exacte, consigner les chiffres. Barre : `plan_reel` ≥ 20/25 et ≤ 1 fausse question, `ecart_reel` ≥ 32/40, `question_reel` ≥ 9/12, phrase E rendue en question **par le modèle** (provenance autre que `guard`).

## Boundaries & Constraints

**Always:**
- Une phrase mal comprise est consignée comme constat (forme de l'écart), jamais corrigée par une règle de prompt ni un correctif par phrase.
- Le jeu tenu à l'écart n'est jamais lu en clair.
- Essais réels avec `--features cuda`, sauf `duree_reel` (processeur, 4 fils, affinité `0x55`).
- Tout ce qui compile `ai/` passe par PowerShell ; sorties d'entraînement hors du dépôt.
- La doc dit que viser des écarts nommés (H17-H22, H40…) affaiblit `ecart_reel` comme mesure aveugle.
- Vocabulaire civil.

**Never:**
- Changer le générateur, le prompt, `judge`, `retry_turns`, le garde-fou ou les seuils des tests.
- Écrire dans un fichier de ticket ; ajouter une dépendance ; toucher au 9B ; mettre Ministral dans `models/`.
- Fusionner dans `main` avant la recette du propriétaire.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Entraînement | `~/cassini-entrainement/5-17-b/` vide | GGUF Q4_K_M fusionné, déclarations, `rapport.json` avec `ecartes_trop_longs` = 0 | coupure : `--reprendre` ; écartés > 0 : consigné |
| Parité | 300 premiers exemples + redemandes des 8 000, sur la base neuve | jetons identiques | écart : arrêt, rien n'est mesuré |
| Phrase E | `cassini plan` (cuda) et `duree_reel` | question sans `"source":"guard"` | question du garde-fou = barre manquée sur E |
| Barre manquée | un seuil sous la barre | chiffres et formes des écarts consignés, compte rendu | jamais de pansement |

**Décisions du propriétaire (2026-10-23) :**
- Lancement : maintenant, carte graphique laissée libre pendant l'entraînement.
- Barre manquée (E comprise) : consigner et s'arrêter ; une nouvelle passe ferait l'objet d'un autre ticket.
- Barre atteinte : Claude installe le modèle neuf dans `models/` (celui de 5.15 gardé à côté sous un autre nom), met à jour le noyau, puis recette du propriétaire dans QGIS.
- Phrase E (2026-10-24, après mesure ; remplace une première décision prise sur une mesure fausse, phrase aux accents abîmés par PowerShell) : avec la phrase exacte, `cassini plan` (processeur et CUDA) et le panneau QGIS rendent la question du garde-fou ; seuls `duree_reel` et `llama-completion` rendent une question du modèle. Barre E **manquée** dans le produit, consignée comme telle ; le modèle de 5.17 est gardé quand même (installé, recette du propriétaire), et la suite de E fera l'objet d'un autre ticket.

</frozen-after-approval>

## Code Map

- `training/entrainement/pipeline.py` -- WSL : `UV_PROJECT_ENVIRONMENT=~/.cache/cassini-entrainement/venv uv run --project training/entrainement python training/entrainement/pipeline.py --sortie ~/cassini-entrainement/5-17-b`, en arrière-plan, journal `~/cassini-entrainement/5-17-b.log`. 5.15 : 241 min pour 8 000 exemples.
- `training/entrainement/parite_decoupeurs.py` -- `--gguf <sortie>/qwen3.5-4b-base-Q4_K_M.gguf --jeu <sortie>/travail/donnees/validation.jsonl`.
- `ai/src/plan.rs` -- test `the_short_prompt_tokens_are_those_of_the_training_prompt_read_in_one_piece` (redemandes comprises depuis 5.15), `CASSINI_TEST_MODEL` = base neuve. Inchangé.
- `ai/tests/{plan_reel,ecart_reel,question_reel,duree_reel}.rs` -- commandes dans leur en-tête ; `question_reel` compte les questions du garde-fou à part. `duree_reel` : `CASSINI_TEST_THREADS=4`, `CASSINI_TEST_INVENTORY=../donnees-test-cassini/inventaire-villeneuve-traduit-4b.json`.
- `cli/src/main.rs` l.491 -- `cassini plan` écrit `"source":"guard"` pour une question du garde-fou : c'est la preuve de provenance de E.
- `qgis-plugin/tests/probe_demo.py` -- démo pilotée par Claude (`python-qgis-ltr.bat`, `CASSINI_CORE`, `CASSINI_MODELS`, `projet-test-villeneuve.qgz`).
- `docs/entrainement-du-modele.md` -- nouvelle section « Tâche B de 5.17 », de même forme que « Tâche B de 5.15 » (l.682) : entraînement, parité, essais réels, durée, démo, bilan.
- Copies Windows : `C:\Users\kevin\cassini-modeles\ajuste-5-17\` ; binaire CUDA de mesure sous `C:\ct\`. Installation : liens durs dans `%LOCALAPPDATA%\Cassini\bin\models\` (le modèle de 5.15 gardé sous un autre nom).

## Tasks & Acceptance

**Execution:**
- [x] `~/cassini-entrainement/5-17-b/` -- lancer la commande unique, puis lire `rapport.json` -- produire le modèle
- [x] PowerShell -- parité (script et test), puis `plan_reel`, `ecart_reel`, `question_reel` (cuda), `duree_reel` (processeur), et E par `cassini plan` (cuda) -- mesurer
- [x] `qgis-plugin/tests/probe_demo.py` -- rejouer la démo avec la phrase exacte sur le modèle neuf -- démo de bout en bout
- [x] `docs/entrainement-du-modele.md` -- chiffres, formes des écarts, bilan contre la barre, note sur la mesure aveugle -- trace

**Acceptance Criteria:**
- Given le modèle neuf, when la parité tourne, then 300/300 et toutes les redemandes sont identiques au jeton.
- Given les quatre essais réels, when ils tournent, then chaque `SCORE` est consigné avec le bilan seuil par seuil, et la provenance de la réponse à E.
- Given la démo, when `probe_demo.py` tourne avec la phrase exacte, then plan, étapes, PDF et journal sont consignés.
- Given la recette, when elle tourne, then elle est verte.

## Implementation Notes

- Mise en œuvre directe, sans sous-agent (Design Notes). Entraînement lancé le 2026-10-23 à 23:51, fini le 2026-10-24 à 05:20 (330 min) ; `ecartes_trop_longs` = 0, `jetons_max` 1659, validation 298/300. Sorties copiées dans `C:\Users\kevin\cassini-modeles\ajuste-5-17\`.
- Parité : script 300/300 ; test de `ai` sur la base neuve, 300 prompts et 339 redemandes identiques.
- GPU : `plan_reel` 25/25, 0 question ; `question_reel` 12/12, 0 du garde-fou ; `ecart_reel` 33/40 (H08, H13, H17-H20, H22), 2 redemandes, 0 sauvée. Sorties dans `C:\ct\mesures-5-17\`.
- CPU (`duree_reel`, `C:\ct\cpu`, affinité `0x55`) : moyenne 19,7 s ; E en question, 1 appel, 32 jetons.
- Phrase E : une première mesure (scripts PowerShell en UTF-8 sans BOM, accents abîmés) avait donné une question du modèle sur processeur ; décision du propriétaire prise dessus. Relevée par la relecture (installation vérifiée sur le dossier installé) : avec la phrase exacte (script avec BOM), `cassini plan` sur processeur et CUDA, modèle installé ou copie d'origine, et le panneau QGIS (sonde jetable `C:\ct\mesures-5-17\probe_e.py`, noyau installé) rendent la question du garde-fou ; `duree_reel` et `llama-completion` (prompt rendu par `cassini_training::prompt::render_prompt`, programme jetable) une question du modèle. Propriétaire informé, nouvelle décision : barre E manquée, modèle de 5.17 gardé, suite dans `deferred-work.md`.
- Origines multiples en coordonnées (constat différé de la tâche A) : le modèle écrit deux origines dans un même objet, sortie rejetée, question du garde-fou ; jamais un plan faux. Reste différé.
- Installation : `models\cassini-composition-4b-Q4_K_M.gguf` lié au modèle de 5.17 (empreinte `6bde9803…`), celui de 5.15 renommé `cassini-composition-4b-5-15-Q4_K_M.gguf` avec sa déclaration ; noyau CPU recopié (`cassini.exe.avant-5-17` gardé ; le code du noyau n'a pas changé depuis 5.16).
- Démo QGIS (noyau du produit, dossier `demo-5-17`) : plan de 5 étapes, 137 / 8 / 6 792 / 2 429 objets, PDF lisible, étapes 66-70 au journal ; composition à froid 67,1 s.
- Recette : `cargo test --workspace`, clippy, `cargo deny` (PowerShell), `fmt` et tests Python (WSL) verts.

## Plan Change Log

## Review Triage Log

Passe 1 (2026-10-24, lentilles blind-hunter, edge-case-hunter, verification-gap, intent-alignment) : high 1, medium 1, low 9, false 4, rejetés 8.

| # | Lentille | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|---|
| 1 | edge, blind | Démo et mesures de E jamais faites sur le dossier installé ; `cassini plan` sans `--model` prendrait le 9B | high | patch | Vérifié : sur le dossier installé, E rend la question du garde-fou ; la mesure CPU d'avant passait une phrase aux accents abîmés. Mesures refaites (script avec BOM, sonde QGIS), propriétaire informé, nouvelle décision, doc et plan corrigés. `--model` toujours passé, désormais écrit dans la doc. |
| 2 | blind, intent | Barre E déclarée tenue après coup, contre la ligne de la matrice | medium | patch | Rendue caduque par #1 : barre E consignée **manquée**, décision du propriétaire réécrite au bloc figé. |
| 3 | edge | Sans choix retenu, le panneau prend le 9B, puis 5.15 avant 5.17 | false | rejet | `QGIS3.ini` : `cassini/model=cassini-composition-4b.modele.json`, nom gardé par le modèle de 5.17 ; dit dans la doc. |
| 4 | edge | `deferred-work.md` non mis à jour (origines multiples, E) | low | patch | Deux entrées ajoutées. |
| 5 | blind | Explication « virgule flottante » donnée comme un fait | low | patch | Réécrit : cause non mesurée ; le CUDA à 0 couche était un indice juste. |
| 6 | blind | Gain de durée attribué à E sans montrer le recul des autres phrases | low | patch | Colonne 5.15 ajoutée, recul de 1 à 8 % dit. |
| 7 | blind | Colonne « En 5.15 » incomplète, H23 et H40 absents | low | patch | Table refaite phrase par phrase. |
| 8 | blind | Commit du générateur et empreinte complète absents de la doc | low | patch | Ajoutés. |
| 9 | blind | Durée d'entraînement 241 → 330 min sans explication | low | patch | Validation 49 min et séquences plus longues dites, cause non établie. |
| 10 | blind | Redemandes de `ecart_reel` sans les phrases | low | patch | H17 et H19 nommées. |
| 11 | blind | « tâche 4 » sans ticket | low | patch | « tâche 4 de 5.10 ». |
| 12 | blind, intent | Scores de justesse sur GPU seulement, produit sur processeur | low | patch | Phrase ajoutée : GPU = référence (décision du 2026-10-17), non refaits sur processeur. |
| 13 | edge | Lien dur vers la copie `ajuste-5-17` | low | rejet | Pratique d'installation de 5.14 et 5.16 ; la copie n'est jamais réécrite. |
| 14 | blind | Plan Change Log vide alors que le bloc figé a changé | false | rejet | Le journal sert aux boucles de relecture ; les décisions du propriétaire s'écrivent au bloc figé (étape 2). |
| 15 | blind | Métadonnées de relecture incohérentes | false | rejet | Tri en cours au moment de la lecture. |
| 16 | blind | Vérification sans commandes pour E, `duree_reel`, démo ; numéro de ligne dans la Code Map | low | rejet | Correction = éditer ce plan ; commandes dans l'en-tête des tests et dans la doc. |
| 17 | edge | Intention : 5.15 « manque `ecart_reel` de peu », alors que 32/40 tenait | low | rejet | Correction = éditer le bloc figé ; la doc dit juste (32 contre 33). |
| 18 | vgap | Aucun trou de vérification (diff sans code) | false | rejet | — |

## Design Notes

Mise en œuvre directe, sans sous-agent : l'entraînement et les mesures s'étalent sur des heures. Les mesures commencent seulement quand `rapport.json` existe. Constat différé de la tâche A (plusieurs points en coordonnées) : relever la sortie du modèle sur une telle phrase pendant les essais par `cassini plan`, sans l'ajouter à un jeu.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check` (PowerShell) ; `cargo fmt --check` et `python3 -I -m unittest discover -s training/entrainement/tests` (WSL) -- expected: succès
- `CASSINI_TEST_MODEL=<base Q4_K_M neuve> cargo test -p cassini-ai --release plan::tests::the_short_prompt_tokens` -- expected: parité totale
- `cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture plan_reel` (puis `ecart_reel`, `question_reel`) -- expected: lignes `SCORE` au-dessus de la barre
