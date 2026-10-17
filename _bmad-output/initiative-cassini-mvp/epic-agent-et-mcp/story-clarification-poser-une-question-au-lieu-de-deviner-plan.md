---
title: 'Clarification : poser une question au lieu de deviner'
type: 'feature'
ticket: '5'
created: '2026-10-17'
status: 'built'
baseline_revision: '1e100c98d9af001052701825169a41bfc74d674a'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Quand un mot de la demande ne se relie à aucune couche ni aucun champ de l'inventaire (« les pharmacies », « les parcelles chères »), le modèle n'a que deux sorties : un plan deviné sur une couche voisine, ou un refus. L'épreuve n'a mesuré que le refus.

**Approach:** Troisième variante de plan, `{"kind":"question","question":<texte>}`, ajoutée au format du noyau, à la grammaire, au prompt (consigne + un exemple) et au canal. Mesurée par un essai réel sur un jeu de phrases ambiguës fabriquées, à côté des 25 phrases claires de l'épreuve.

## Boundaries & Constraints

**Always:** Une question n'est jamais gardée en attente ni exécutée : `plan.execute` après une question répond `plan.unknown`. La question est rendue comme un résultat, pas comme une erreur : `compose_plan` rend `Ok(Plan::Question)`, `cassini plan` l'affiche en JSON (code 0), `plan.propose` répond `{"question": <texte>}` sans `plan_id`. Une question vide est un écart (redemande, comme un refus sans raison). Refus = aucune commande ne fait la chose ; question = la commande existe mais un mot ne désigne rien de l'inventaire. AD-23 tenu : la sortie reste contrainte par grammaire, rien n'est réparé. Aucune nouvelle dépendance, hors ligne. Décision du propriétaire (2026-10-17) sur le seuil : au moins 9 des 12 phrases ambiguës rendent une question (un refus y compte comme échec), au plus 1 des 25 phrases claires de l'épreuve devient une question, et les 80 % de plans conformes de 5.3 tiennent. Renégocié par le propriétaire le 2026-10-17 après trois essais de prompt (7/12 ; 11/12 et 2 questions sur 25 ; 4/12) : pour le modèle actuel, le plafond passe à **au plus 2** phrases claires devenues questions, le prompt de l'essai 2 est retenu ; le seuil strict (≥ 9/12, ≤ 1 fausse question) est reporté au modèle ajusté du ticket 5.10.

**Never:** Panneau QGIS (5.6) : il affichera la question, ce ticket ne fait que la livrer par le canal. Dialogue de réponse à la question (la réponse est une nouvelle demande). Comparaison automatique des mots de la demande à l'inventaire côté Rust : c'est le modèle qui juge. Toucher `tests/epreuve-modele-local/`. Changer la signature de `compose_plan` ou du trait `Composer`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Question | modèle rend `{"kind":"question","question":"Quelle couche ..."}` | `Ok(Plan::Question)`, sans redemande | — |
| Question vide | `question: "  "` | traitée comme invalide : redemande | puis `agent.plan_invalid` |
| Canal, question | `plan.propose` | `result: {"question": ...}`, rien en attente | `plan.execute` → `plan.unknown` |
| CLI, question | `cassini plan` | JSON de la question sur `out`, code 0 | — |
| `check_plan` | `Plan::Question` | `Ok(())` | — |

</frozen-after-approval>

## Code Map

- `core/src/plan.rs` -- enum `Plan` (l.24) : ajouter `Question { question: String }` ; `check_plan` (l.128) rend déjà `Ok` hors `Calls` ; test de sérialisation à côté de `a_refusal_is_accepted_and_the_plan_json_round_trips` (l.582).
- `core/src/serve/plans.rs` -- `propose` (l.95) : après le bloc `Plan::Refusal` (l.120), rendre `{"question"}` sans `Pending` ; doc du trait `Composer` (l.21). Tests dans `core/src/serve.rs` près de `a_refusal_or_an_invalid_plan_is_refused_and_not_kept` (l.1791), composer fixe `composer(plan)` (l.1105).
- `ai/src/plan.rs` -- `plan_grammar` (l.92, `root ::= calls | refusal | question`), `SYSTEM` (consigne question vs refus), `examples()` (un exemple question sur un mot absent de l'inventaire), `judge` (l.281, question vide = écart), `compose_plan` (l.353, `Ok(Plan::Question)` remonte tel quel). Tests unitaires du module (générateur scripté, `done(..)`) ; tests de grammaire l.631.
- `cli/src/main.rs` -- `plan` (l.341) sérialise déjà tout `Ok(plan)` : seulement la doc et un test si un générateur injectable y est accessible (sinon rien).
- `ai/tests/plan_reel.rs` -- inchangé dans sa logique : une question sur une phrase claire compte déjà en échec (`_ => false`). Ajouter le décompte des questions rendues (`SCORE`).
- `ai/tests/question_reel.rs` (nouveau) -- phrases ambiguës sur l'inventaire de l'épreuve (`include_str!` comme `plan_reel.rs`), gardé par `CASSINI_TEST_MODEL` et `#[ignore]`.
- `docs/canal-local.md` -- section `plan.propose` (l.102) : réponse question.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/plan.rs` -- variante `Question` + test aller-retour JSON et `check_plan` Ok -- format du noyau
- [x] `core/src/serve/plans.rs`, `core/src/serve.rs` (tests) -- `propose` rend `{"question"}`, rien en attente ; test : question puis `plan.execute` → `plan.unknown`, aucune notification -- canal
- [x] `ai/src/plan.rs` -- grammaire, consigne, exemple, `judge`, tests scriptés (question rendue sans redemande, question vide puis plan valide, question vide deux fois → `agent.plan_invalid`) -- composition
- [x] `ai/tests/question_reel.rs` -- 12 phrases ambiguës fabriquées sur l'inventaire de l'épreuve (couche absente : pharmacies, arrêts de bus, rivières chargées nulle part ; champ absent : parcelles chères, bâtiments anciens, routes larges, quartier nord...), jamais un fichier listé dans `files` de l'inventaire ; seuil : au moins 9 questions sur 12, un refus compte comme échec -- verify du ticket
- [x] `ai/tests/plan_reel.rs` -- compter les questions sur les 25 et asserter au plus 2 (renégocié le 2026-10-17) -- « les phrases claires ne deviennent pas des questions »
- [x] `docs/canal-local.md` -- réponse question de `plan.propose`

**Acceptance Criteria:**
- Given un générateur scripté rendant une question, when on compose, then `Plan::Question` est rendu au premier appel et le générateur n'est appelé qu'une fois.
- Given un client du canal et un compositeur qui rend une question, when il appelle `plan.propose` puis `plan.execute {plan_id:"plan-1"}`, then la première réponse porte `question` sans `plan_id` et la seconde est `plan.unknown`.
- Given le modèle réel et les phrases ambiguës, when on compose, then au moins 9 des 12 rendent une question.
- Given le modèle réel et les 25 phrases de l'épreuve, when on compose, then au moins 80 % des plans restent conformes et au plus 2 deviennent des questions (renégocié le 2026-10-17).

## Implementation Notes

Essais réels (Qwen3.5-9B Q4_K_M, GPU `--features cuda`), 2026-10-17 :
- Essai 1 (consigne question après la consigne refus) : `SCORE 7/12 questions posées` (Q03, Q04, Q05, Q08, Q11 en refus) ; `plan_reel` passé (≥ 80 %, ≤ 1 question), ligne SCORE non capturée (sans `--nocapture`).
- Essai 2 (consigne question avant le refus, « une couche ou un champ absent n'est jamais une raison de refus ») : `SCORE 11/12 questions posées` (Q08 en refus) ; `SCORE 22/25 plans conformes, 2 questions` (S20, S21 deviennent des questions) : seuil « au plus 1 question » manqué.
- Arrêt après deux essais (CLAUDE.md §8) : le code livré porte la consigne de l'essai 2. Décision du propriétaire attendue.
- Essai 3, dernier, décidé par le propriétaire (ordre explicite : 1. une commande fait-elle l'opération ? sinon refus ; 2. chaque mot de couche ou de champ se retrouve-t-il dans l'inventaire ? sinon question ; 3. un point en coordonnées n'est pas une couche ; règles génériques seules) : `SCORE 24/25 plans conformes, 0 questions` (S10 en plan `analysis.zonal_statistics` au lieu d'un refus) ; `SCORE 4/12 questions posées` (Q02, Q03, Q04, Q05, Q07, Q08, Q10, Q11 en refus, plusieurs avec un texte de question dans `reason`). Seuil 9/12 manqué. Le code livré porte la consigne de l'essai 3. Arrêt, quel que soit le résultat.
- **Essai 2 retenu** (décision du propriétaire, 2026-10-17) : consigne de l'essai 2 rétablie dans `SYSTEM`, plafond des questions sur les phrases claires porté à 2 dans `plan_reel.rs`. Contre-essai GPU : `SCORE 22/25 plans conformes, 2 questions` (S10 en plan, S20 et S21 en question) et `SCORE 11/12 questions posées` (Q08 en refus) ; les deux essais passent.

- Audit de la matrice : la ligne « CLI, question » n'avait pas de test. `cli/src/main.rs` : la fin de `plan` passe par `print_composed` (plan ou question en JSON sur `out`, code 0 ; erreur typée sur `err`), testée par `a_question_is_printed_as_json_on_out_with_code_0`. Suite core 177, cli 23, ai 24 ; fmt et clippy propres.

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment). 0 high, 0 medium, 6 low, 9 rejetés (false ou low hors usage courant), 1 defer.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | false | rejete | edge : chemin absolu dans une question (AD-9). `scrub` ne masque que le modele et son dossier, que le prompt ne contient jamais ; une question ne peut donc pas les citer. |
| 2 | low | rejete | edge : question vide d'un autre `Composer` sur le canal. Seul `ModelComposer` existe, et il passe par `judge` qui rejette la question vide ; garde non demontree. |
| 3 | low | rejete | edge : le plan cite rivieres et quartier nord, absents du jeu. Correctif = editer le plan. |
| 4 | false | rejete | blind : formes differentes CLI (`kind`) et canal (`{"question"}`). Forme du canal fixee par le bloc fige (Always). |
| 5 | low | patch | blind : aucun test « plan en attente puis question ». Test ajoute, doc du canal completee. |
| 6 | low | patch | blind : plafond de 2 sans rappel du 5.10. Commentaire dans `plan_reel.rs`. |
| 7 | low | rejete | blind : pas d'exemple de champ absent dans le prompt. Toucher le prompt invalide les mesures retenues ; a reprendre au 5.10. |
| 8 | low | rejete | blind : la grammaire accepte une question vide. Rattrape par `judge` ; changer la grammaire impose une nouvelle mesure. |
| 9 | low | rejete | blind : SCORE sans ventilation refus/plan/invalide. Lignes par phrase deja imprimees. |
| 10 | medium | defer | blind : question ecrite dans un refus (`reason` en forme de question), non detectee. Comportement du modele, a traiter dans le jeu d'entrainement du 5.10. |
| 11 | false | rejete | blind : l'echange question n'est pas journalise. Le journal ne porte que les executions ; une proposition de plan ne l'est pas non plus (5.4). |
| 12 | low | rejete | blind : saut de ligne brut et accent dans `eprintln!`, motif copie de `plan_reel.rs` existant ; cosmetique. |
| 13 | low | rejete | blind : « refuser ;. » en fin de `SYSTEM`, anterieur ; toucher le prompt invalide les mesures. |
| 14 | low | patch | blind : `print_composed` teste seulement sur la question. Test plan et refus ajoute. |
| 15 | low | patch | verification-gap : regle `question` verifiee par un prefixe. Ligne de regle complete assertee (et `refusal`). |
| 16 | — | note | intent-alignment : descriptif ; panneau (R3) exclu par le Never, forme du canal fixee par le bloc fige, preuve du comportement dans les essais reels notes plus haut. |

## Design Notes

Décisions prises sans redemander : la question est un résultat et non une erreur, parce que ce n'est pas un échec de l'agent et que 5.6 doit l'afficher comme une réplique ; sur le canal, sa forme est `{"question"}` sans `plan_id` pour qu'aucune question ne puisse être exécutée. Les phrases ambiguës vivent dans le test Rust, pas dans `tests/epreuve-modele-local/` (gelé). Une phrase qui nomme un fichier de l'inventaire non ouvert (écoles) est exclue du jeu : `layer.add` y répond légitimement.

Exemple attendu dans le prompt :
`Inventaire : {"layers":[{"layer_id":"gares_7f3a","name":"Gares",...}]}` / `Demande : Fais une zone tampon de 100 m autour des pharmacies.` / `Plan : {"kind":"question","question":"Aucune couche de pharmacies dans le projet : laquelle faut-il utiliser ?"}`

## Verification

**Commands:**
- `cargo test --workspace` -- expected: tout passe
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo deny check` -- expected: succès
- `CASSINI_TEST_MODEL=<gguf> cargo test -p cassini-ai --release -- --ignored question_reel plan_reel` -- expected: 9/12 questions au moins, au plus 2 fausses questions, 80 % des plans conformes
