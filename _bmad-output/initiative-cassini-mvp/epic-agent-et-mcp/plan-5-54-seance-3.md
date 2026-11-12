---
title: "5.54 séance 3 — plans d'urbanisme à 4 étapes, opérateurs exacts, questions « valeur absente » et « champ absent »"
type: 'feature'
ticket: ''
created: '2026-11-12'
status: 'built'
baseline_revision: '51370b1cb5282c1c488e4d39f8bd8121e6f2eaaf'
route: 'full'
route_source: 'pinned'
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

**Problem:** La séance 2 (`ajuste-continu-5-54-2`) gagne partout mais la barre de non-oubli la refuse : `ecart_reel` 39 → 35/40 (plans d'urbanisme à 4 étapes rendus en 3, opérateur faux `<`/`<=` ou `=`/`!=`, une étape de trop, un refus au lieu d'une question) et `question_reel` 45 → 43/47 (valeur absente des valeurs connues d'un champ, ou champ absent : plan au lieu de question). Le mélange de la séance 2 les a dilués : 50 plans à 4 étapes sur 3 000 (1,7 %, contre 2,8 % au jeu graine 1), 15 questions de champ absent, aucune question de valeur absente (aucune famille ne l'apprend).

**Approach:** Séance 3 depuis B, tout l'acquis de la séance 2 gardé (familles, rappel stratifié 50 %, refus rééquilibrés). Constante neuve `THIRD_SESSION_FAMILIES` = `SECOND_SESSION_FAMILIES` + `conditions_multiples`, `constructible`, `distance_reference_filtree`, `question_champ_absent` (renforcée), `question_valeur_absente` (neuve) ; opérateurs numériques équilibrés et `ne` plus fréquent dans les cibles filtrées ; questions en contraste avec des phrases presque identiques qui rendent un plan. Séance par `nuit-5-54.sh`, installation seulement si installable et Annecy ≥ 16/20.

## Boundaries & Constraints

**Always:** vocabulaire civil ; une question quand la valeur ou le champ manque, jamais un refus ; une valeur est « absente » seulement si le champ porte une clé `values` (liste complète, `MAX_VALUES` = 10, `core/src/agent_inventory.rs:72`) qui ne la contient pas ; chaque famille touchée a son test ; verts : `recouvrement.rs`, `gros_projet` 23-27 %, redemandes ≥ 4,5 % du jeu graine 1, tout refus dans `KNOWN_REFUSALS`, liste de séance 3 : `FOCUS_COUNT` exemples sans copie de barre.

**Never:** lire `ai/tests/tenu_a_l_ecart/sentences.json`, `ai/tests/*/phrases.json`, `tests/epreuve-modele-local/sentences.json`, les constantes de phrases des `ai/tests/*_reel.rs`, `qgis-plugin/tests/recette/*.json`, ni un relevé sous `~/cassini-entrainement/*-mesures/` ; toucher aux barres, au noyau, au registre, au rappel stratifié ; lancer un modèle, le GPU ou la séance ; pousser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Valeur absente | champ `typologie` à `values` [a, b], demande « … de type c » | `question` qui nomme la couche, le champ et ses valeurs | jamais un plan ni un refus |
| Valeur présente (contraste) | même phrase, c listée | `select_by_attribute eq c` | — |
| Zone absente | `typezone` [A, N, U], « parcelles en zone AU à moins de 300 m d'une gare » | `question` (aucune valeur ne commence par AU) | — |
| Champ absent dans un plan long | parcelles sans champ de surface, « de plus de 500 m² près d'une école » | `question` (`missing_field_question`) | — |
| Champ sans `values` | champ de nom (trop varié) | plan avec la valeur écrite (`contains`/`eq`) | jamais une question |
| Opérateur | « d'au plus 500 m² », « sauf … » | `lte`, `ne` | — |

</frozen-after-approval>

## Code Map

- `training/src/lib.rs` -- `FOCUS_SEED`/`FOCUS_COUNT` (286-289), `SECOND_SESSION_FAMILIES` (320-329) ; tests `the_second_session_families_give…` (1727) et `the_session_command_of_the_doc_names_the_second_session_families` (1753) à dupliquer ; `the_default_set_holds_a_quarter_of_big_projects…` (877, 23-27 %).
- `training/src/families.rs` -- table `FAMILIES` (53 ; 72 familles, poids 570 ; `conditions_multiples` 13 l.124, `constructible` 7 l.130, `distance_reference_filtree` 6 l.103, `question_champ_absent` 5 l.198, `redemande` 76 l.474, `gros_projet` 148 l.491) ; `select` (499) ; `missing_layer_question`/`missing_field_question` (3537/3552) ; `numeric` (797, `below(7)` : gt 3/7, lt 2/7, gte 1/7, lte 1/7) ; `draft_filter_eq` (897), `qualified` (776) ; `zone_category_select` (1485, `eq` ou `begins_with` selon les valeurs montrées) ; `filtered_target` (1711, `below(7)`, `ne` 1/7 branche 4) ; `draft_multi` (1784, 15 branches de 3 à 6 étapes), `MAX_DRAFT_STEPS` (2100) ; `conditions_multiples` (2102, PDF 1/4) ; `draft_constructible`/`constructible` (2174-2245) ; `question_couche_absente` (3565), `question_champ_absent` (3636, branche « constructible » 1/5) ; `KNOWN_REFUSALS` (8578) ; `gros_projet` (9886, enveloppe toute famille via `draw`, 10146) ; tests `several_conditions_vary_operators_and_lose_no_step` (11985), `a_zone_operator_follows_the_values_shown` (14116), `every_learned_refusal_is_a_known_constant…` (~15476).
- `training/src/inventories.rs` -- `Builder::add` (2147 : valeurs d'un champ texte = sous-ensemble de 2 à toutes, triées ; `force`), `Builder::set_values` (2119), `Layer::field` (1581).
- `core/src/agent_inventory/sorted_view.rs` -- `capped` (358) : un gros projet peut ôter un champ à valeurs non nommé par la demande (lecture seule).
- `training/entrainement/pipeline.py` -- lit les noms de famille dans `families.rs` : aucun changement attendu.
- Comptes relevés à `51370b1` : jeu graine 1 (8 600) : plans à 4 étapes 244 (dont `conditions_multiples` 77, `constructible` 71, `gros_projet` 41), à 5 étapes 187 ; questions 887 dont `question_champ_absent` 91, valeur absente 0 ; opérateurs des sélections : `eq` 1 681, `ne` 145, `gt` 117, `lt` 72, `gte` 51, `lte` 44. Jeu ciblé de la séance 2 (graine 3, 1 800) : 6 plans à 4 étapes, 0 question, 104 refus. Rappel stratifié (1 500) : 44 plans à 4 étapes, 26 à 5, `question_champ_absent` 15, `conditions_multiples` 38, `constructible` 22.

## Tasks & Acceptance

**Execution:**
- [x] `training/src/families.rs` -- famille neuve `question_valeur_absente` (poids 5, après `question_champ_absent`) : couche à champ texte, valeur demandée tirée des valeurs du thème hors des valeurs listées (`set_values` en retire une si toutes le sont ; jamais une valeur dont la forme normalisée en contient une listée ou y est contenue) ; branches : filtre seul, cible filtrée + distance à une référence (+ PDF 1/4), zonage « en zone {code} » sans valeur qui commence par le code ; deux fois sur trois la question `missing_value_question(layer, field, value, listed)` (tournures tirées, valeurs listées nommées), une fois sur trois la même phrase avec la valeur listée → plan -- cause 2.
- [x] `training/src/families.rs` -- `question_champ_absent` poids 5 → 7 ; branche neuve (1/4) : le champ manque dans une demande à plusieurs conditions (cible filtrée + distance, ou constructible + distance) → question ; contraste (1/4) : même phrase, champ présent → plan du brouillon -- cause 2.
- [x] `training/src/families.rs` -- `numeric` : `below(8)`, deux tirages par opérateur ; `filtered_target` : `below(8)`, `ne` deux tirages -- opérateurs exacts.
- [x] `training/src/families.rs` -- `gros_projet` : écarte (rejet compté) un `question_valeur_absente` dont la vue triée ne montre plus le champ et toutes ses valeurs -- garde-fou de la question.
- [x] `training/src/families.rs` (tests) -- `question_valeur_absente` : question ⇔ valeur hors de `values`, plan ⇔ valeur listée, chaque branche vue, aucun refus ; `question_champ_absent` : question ⇔ champ absent, branche longue et contraste vus ; opérateurs : les six (`eq`, `ne`, `gt`, `lt`, `gte`, `lte`) dans `conditions_multiples`, chacun ≥ 10 % des sélections numériques ; plans de `conditions_multiples`/`constructible` : nombre d'étapes = conditions de la phrase (+ PDF) ; `every_learned_refusal…` étendu au jeu ciblé de la séance 3.
- [x] `training/src/lib.rs` -- `pub const THIRD_SESSION_FAMILIES` = `SECOND_SESSION_FAMILIES` + `conditions_multiples`, `constructible`, `distance_reference_filtree`, `question_champ_absent`, `question_valeur_absente` ; tests : `the_third_session_families_give_the_whole_focused_set_without_a_bar_sentence` (préfixe = séance 2, `FOCUS_COUNT` exemples, chacune présente, aucune copie, plans à 4 étapes ≥ 8 % et questions ≥ 10 % du jeu ciblé) et `the_session_command_of_the_doc_names_the_third_session_families`.
- [x] `docs/entrainement-du-modele.md` -- section « Séance 3 (5.54) » : causes, table familles/tests, poids, comptes avant/après (jeu graine 1, jeu ciblé, rappel), commande de la séance (ligne exacte `nuit-5-54.sh <THIRD_SESSION_FAMILIES>`), règle d'installation.

**Acceptance Criteria:**
- Given le code, when `cargo test -p cassini-training --release` et `pytest`, then tout est vert, `recouvrement.rs` compris, sans le modifier.
- Given le jeu graine 1 régénéré, when on compte, then `gros_projet` 23-27 %, redemandes ≥ 4,5 %, plus petite famille du rappel ≥ 15.

## Design Notes

Party (ingénieur ML, géomaticien urbaniste, architecte) :
- **Familles existantes au jeu ciblé plutôt qu'une famille « plans longs » neuve** (ML) : `conditions_multiples` et `constructible` portent déjà les formes à 3, 4, 5 et 6 étapes ; le manque est de dilution (6 plans à 4 étapes sur 1 800 au ciblé de la séance 2). `distance_reference_filtree` (3 étapes) donne le contraste « pas d'étape de trop ». Poids de la liste : 68, dont 44 % aux huit de la séance 2 (~660 exemples sur 1 500, bien au-dessus d'un rappel) : leurs gains tiennent.
- **Valeur absente = clé `values` sans la valeur** (architecte) : le noyau ne montre `values` que si le champ a au plus 10 valeurs distinctes, liste complète ; sans clé, la valeur écrite reste un plan (champs de nom, déjà appris). Le zonage suit `zone_category_select` : « AU » est présent dès qu'une valeur commence par AU.
- **Question qui cite les valeurs connues** (urbaniste) : « Le champ typologie de la couche « X » n'a pas de valeur « c » (valeurs : a, b) : laquelle faut-il prendre ? » ; l'utilisateur répond d'un mot.
- **Deux tiers de questions, un tiers de contraste** : le rappel et `filtre_egalite` apprennent déjà le plan ; la cause est le plan à tort.
- **Opérateurs équilibrés partout** (`numeric` global) : l'écart 3:1 entre `gt` et `lte` est un a priori appris ; « au moins », « au plus » sont courants en PLU. Le jeu graine 1 change : comptes et fourchettes revérifiés.
- **Poids globaux** : `question_valeur_absente` 5 (comme le champ absent avant), `question_champ_absent` 7 ; total 577, `gros_projet` reste vers 25 %.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy -p cassini-training --all-targets -- -D warnings && cargo test -p cassini-training --release` -- expected: vert.
- `uv run --project training/entrainement python -m pytest training/entrainement/tests` -- expected: vert.
- `cargo run -p cassini-training --release -- --seed 3 --count 1800 --familles <THIRD_SESSION_FAMILIES> --out <scratch>` -- expected: code 0, comptes de la doc.

**Commande de la séance** (après fusion en avance rapide sur `main`, sans push, `REV` = HEAD du worktree, propre ; GPU libre, `ps` vérifié) :
`git -C "/mnt/c/Users/kevin/Downloads/Projet code/CASSINI-nuit" checkout -q --detach REV && WT=~/cassini-worktrees/5-54-seance-3 NOM=ajuste-continu-5-54-3 MES=~/cassini-entrainement/5-54-3-mesures PART_RAPPEL=0.5 setsid nohup bash ~/cassini-entrainement/nuit-5-54.sh export_pdf_echelle_format,refus_fonction_absente,options_ecrites,champ_de_vision_cumule,point_lambert_vers_geo,ordre_nord_est,ligne_de_vue,recoupement,conditions_multiples,constructible,distance_reference_filtree,question_champ_absent,question_valeur_absente`

**Installation** : seulement si `nuit-ajuste-continu-5-54-3.etat` dit « VERDICT non-oubli : installable » et `[annecy] SCORE` ≥ 16/20 ; B reste installé à côté.

## Implementation Notes

- Garde-fou du gros projet : plutôt qu'un test propre à `gros_projet`, `Example` porte `listed` (couche, champ, valeurs citées par la question), réécrit par `big` avec les identifiants courts ; `validate` écarte l'exemple quand l'inventaire montré n'a plus ce champ avec toutes ses valeurs : le rejet est compté sous `gros_projet`. Au jeu graine 1, 17 questions de valeur absente emballées par un gros projet sont gardées.
- Test des étapes : un tampon par distance écrite (nombres de la demande moins ceux des valeurs et champs sélectionnés, des noms de couche cités et d'un titre de l'utilisateur, plus les kilomètres en lettres), PDF ⇔ « PDF », chaque étape sert à une suivante.
- `pytest` n'est pas une dépendance du projet `training/entrainement` : lancé par `uv run --project training/entrainement --with pytest python -m pytest training/entrainement/tests` (118 verts).
- Après revue : `question_champ_absent` porté à 10 (et non 7) pour garder la question courte d'avant à un poids effectif de 5 ; total 580, liste de séance 71. `question_valeur_absente` a un contraste d'un sixième (champ de nom sans `values` → plan) ; une question n'est jamais tirée si le rendu liste la valeur ailleurs. Les sélections des plans longs sont comptées contre `WRITTEN_CONDITIONS`, compteur posé par le générateur là où il écrit chaque condition. La tournure « Génère la carte en PDF » de `export_pdf_echelle_format` devient « Génère-moi la carte en PDF » : le tirage déplacé lui faisait reprendre une phrase d'essai réel au jeu ciblé de la séance 3.
- Comptes avant relevés sur un worktree temporaire à `51370b1` (identiques à ceux du Code Map), après sur le code de ce plan ; recopiés dans la doc.

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Verdicts : high 0, medium 7, low 5, false 0, maybe-false 0 ; rejetés (low, coût > gain ou édition du plan) 4. Corrigés 10 (patch, même implémenteur).

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Raison de rejet `listed_shown` absente de la liste de `no_family_produces_rejected_examples` | medium | patch | raison admise, commentaire à jour |
| Zonage : `starts_with(code)` retire AUc/AUs pour « zone A » | medium | patch | `in_zone_category`, cas « A demandée, AUc présent » testé |
| Ligne de la matrice « champ sans `values` → plan » sans générateur ni test | medium | patch | `unlisted_name_plan` (1/6) et test ; écart au plan traité en correctif, le reste du code étant bon (pas de reprise complète) |
| Question posée alors qu'un autre champ ou une autre couche montrée liste la valeur | medium | patch | `listed_elsewhere`, mots entiers ; test |
| Valeur citée ≠ valeur écrite dans la demande | medium | patch | valeur citée telle qu'écrite, test |
| Test des opérateurs trop lâche (plancher 10 %) | medium | patch | 20-30 % chacun ; `ne` > 33 % de eq+ne sur `filtered_target` |
| Sélection perdue jamais détectée par le test des plans longs | medium | patch | compteur `WRITTEN_CONDITIONS` du générateur comparé aux `select_by_attribute` |
| `question_champ_absent` : questions courtes ramenées à 3,5 | low | patch | poids 10, corps ancien 1/2 |
| Garde `listed` via `big` jamais exercée | low | patch | test `a_missing_value_question_in_a_big_project_keeps_its_field_or_is_rejected` |
| Doc : commande non recopiable, « 4 étapes » ambigu, décalages des comptes non commentés | low | patch | ligne exacte, libellé, paragraphe |
| Commande pytest du plan sans `--with pytest` ; `ticket` vide | low | reject | édition du plan |
| Clarté « 1 560 tirés / 1 500 » | low | reject | déjà expliqué en séance 2 (marge coupée par `session_mix`) |
| `listed.values` non triées | low | reject | `listed_shown` trie les deux côtés |
| Lien jeu → barre non vérifiable hors entraînement (intent-alignment) | low | reject | par construction : la séance et son juge le vérifient |

Ajout hors plan : `export_pdf_echelle_format` « Génère la carte en PDF » → « Génère-moi la carte en PDF » (copie d'une barre vue par la garde, sur la phrase engendrée).

