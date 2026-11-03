---
title: 'Couches intermédiaires retrouvées après une relance du noyau ou un rejeu'
type: 'feature'
ticket: '34'
created: '2026-11-03'
status: 'built'
baseline_revision: '4c7e65b08d7ebca085b9e4276672d01339979701'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le noyau cache au modèle les couches intermédiaires d'un plan exécuté (5.22), mais il ne les connaît qu'en mémoire (`State.intermediates`, alimenté par `plan.execute`). Après une relance du noyau (QGIS rouvert) ou un `journal.replay`, elles redeviennent visibles toute la session : inventaire plus lourd, phrase plus lente, retrait par paquets de 5.19 ramené.

**Approach:** Le journal du projet devient la seule source des intermédiaires : le noyau les en déduit (sorties d'une étape consommées par une étape réussie du même plan), et les reconnaît dans l'inventaire par leur fichier source, que QGIS garde d'une session à l'autre et que le rejeu reproduit sous le même nom dans `replay/`.

**Décision (2026-11-03, propriétaire) :** chaque ligne écrite par `plan.execute` porte une clé facultative `plan_step` (place de l'étape dans le plan, un nouveau plan recommence à 1), consignée sous AD-11 comme `origin`. Le rejeu ne la lit pas. Les journaux anciens, sans la clé, ne cachent rien de leurs plans passés (vus une fois, comme aujourd'hui à la relance). Plan gardé entier (~2 200 jetons).

## Boundaries & Constraints

**Always:** seule la vue du modèle change ; une couche de l'utilisateur (source hors du dossier de travail et de ses dossiers de rejeu) n'est jamais cachée ; même règle qu'en 5.22 — seule une étape réussie qui produit une couche consomme, et seulement par une référence de premier niveau `step:k` ou `step:k.layer_id` ; une sortie reprise par une phrase suivante reste vue ; un plan arrêté ne cache que ce qu'une étape exécutée a consommé ; les journaux déjà écrits restent rejouables à l'identique ; tests du noyau sans QGIS.

**Never:** fichier d'état à côté du journal ; changer le rejeu, ses repères `out:k.n` ou sa comparaison ; règle de prompt ; toucher à `results_without_values`, à l'ordre stable, aux identifiants courts ou au budget.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Relance | plan démo exécuté, nouvelle session sur le même projet, couches rechargées avec de nouveaux ids | mêmes intermédiaires cachés qu'avant la relance | — |
| Rejeu | `journal.replay` dans la même session ou après relance | les copies sous `replay/` (ou `replay-<n>/`) des intermédiaires sont cachées, la couche finale reste vue | — |
| Phrase suivante | plan 2 consomme la sortie finale du plan 1 | la sortie du plan 1 reste vue, avant comme après relance | — |
| Plan arrêté | échec à l'étape 3 | seules les sorties consommées par les étapes 2 réussies sont cachées | — |
| Couche de l'utilisateur | `layer.add` puis `step:1` | jamais cachée | — |
| Journal absent ou ligne illisible | — | rien de caché pour ce qui n'est pas lisible | aucune erreur rendue |

</frozen-after-approval>

## Code Map

- `core/src/journal/mod.rs:225` `call_from` -- écrit la ligne (`step`, `id`, `params` canonisés où un `step:k` est déjà résolu en `layer_id`, `layer_id` rendu si succès, `origin` facultatif). Y passer la place dans le plan. `:378` `output_file` donne le nom de sortie `{k}-{id}{ext}` (ou `-{name}` si plusieurs) d'après `spec.output_params`/`output_ext` ; `content_lines` lit le journal. Nouvelle fonction publique : noms de fichiers de sortie intermédiaires d'après le journal et le registre.
- `core/src/serve/plans.rs:283` `execute` -- remplace l'alimentation de `state.intermediates` (`:319-325`) ; `consumed_layers` (`:337`) et ses tests deviennent la règle appliquée à la lecture du journal. `:152-175` l'inventaire : lire les intermédiaires au journal (`state.session` → `journal_path()`), petit fichier relu à chaque proposition.
- `core/src/serve.rs:249-253` `State.intermediates` -- à retirer (une seule source). `:869` `Session::open` ; `:966` `journal_replay` (n'écrit pas au journal, ne pas modifier) ; `:2715` test bout en bout de 5.22, modèle des nouveaux tests (relance = nouvel `exchange` sur le même projet).
- `core/src/agent_inventory.rs:449` `without_intermediates` -- filtrer sur `source` (nom de fichier, comme `result_date` `:498`) au lieu de `layer_id` ; `model_inventory` `:382` reçoit l'ensemble de noms ; tests `:1320`, `:1481`.
- `cli/src/main.rs` (appel de `model_inventory`, cherché par `grep -n model_inventory`) -- mis à jour.
- Contrat d'architecture AD-11 -- une ligne sur la clé ajoutée.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/journal/mod.rs` -- clé de plan sur la ligne ; fonction qui déduit du journal les fichiers de sortie intermédiaires ; tests unitaires (plan, plan arrêté, phrase suivante, `layer.add`, `step:k.feature_count`, ligne illisible, journal ancien)
- [x] `core/src/agent_inventory.rs` -- filtre par fichier source ; tests mis à jour
- [x] `core/src/serve/plans.rs`, `core/src/serve.rs` -- `execute` passe la place dans le plan ; inventaire lu d'après le journal ; `State.intermediates` retiré ; tests bout en bout : relance, rejeu, phrase suivante
- [x] `cli/src/main.rs` -- appel mis à jour
- [x] contrat d'architecture -- AD-11 complété

**Acceptance Criteria:**
- Given un plan démo exécuté par le compositeur factice, when une nouvelle session s'ouvre sur le même projet et l'inventaire rend les mêmes fichiers sous de nouveaux ids, then la vue de la proposition suivante est celle d'avant la relance.
- Given un journal écrit avant ce ticket, when il est rejoué, then le bilan est `identical: true`.
- Given `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check`, then tous verts.

## Implementation Notes

- `Session::call_in_plan(id, params, plan_step)` écrit la clé ; `call` et `call_from` passent par une même fonction privée. `journal::intermediate_files(registry, journal)` applique la règle à la lecture : une ligne `plan_step` ≤ 1 ou sans clé ferme le plan en cours ; une ligne réussie qui rend un `layer_id` consomme toute valeur de premier niveau de ses `params` égale au `layer_id` d'une ligne antérieure du même plan (ou une sortie entière qui le porte, cas d'un `step:k` nu) ; les noms sont ceux d'`output_file`, numérotés par la position de la ligne comme au rejeu.
- `consumed_layers` et son test retirés de `plans.rs` : la règle vit dans le journal, testée par `intermediate_files_follow_the_rule_of_an_executed_plan`.
- `cli/src/main.rs` : signature de `model_inventory` inchangée (`&HashSet<String>`, désormais des noms de fichier) ; l'appel du test, avec un ensemble vide, n'a pas eu à changer.
- Tests bout en bout : nouvelle commande de test `test.chain` (couche + fichier `.gpkg`), car `test.make` n'écrit aucun fichier ; le test 5.22 est devenu `the_intermediates_of_an_executed_plan_stay_hidden_after_a_restart`, plus `the_next_sentence_keeps_its_input_and_replayed_intermediates_stay_hidden` (phrase suivante, rejeu, relance).
- Recette QGIS par Claude (2026-11-03, sonde jetable `%LOCALAPPDATA%\Temp\sonde-5-34\`, projet `projet-sonde-5-34.qgz`, modèle `cassini-composition-4b`, paquet `--features gpu` installé, ancien noyau gardé en `cassini.exe.avant-5-34`). Plan de démo exécuté (5 étapes, journal `plan_step` 1 à 5), projet enregistré, puis « Garde les zones U du PLU. » composée sans exécution, en jetons lus par le modèle :
  - même session : 1 967 ;
  - après relance, nouveau noyau : 1 967 (vue identique) ; ancien noyau : 2 680 (les trois intermédiaires revus) ;
  - après `journal.replay` (`identical: true`, 14 couches), nouveau noyau : 2 355 (seule la copie de la couche finale s'ajoute) ; ancien : 3 784.
  QGIS garde les ids de couche dans le projet enregistré : la relance prouve surtout la perte de la mémoire du noyau ; le rejeu prouve la reconnaissance par le fichier (copies sous `replay/`, ids neufs).

## Plan Change Log

## Review Triage Log

Passe 1 (2026-11-03, lentilles blind-hunter, edge-case-hunter, verification-gap, intent-alignment) : high 0, medium 0, low 8, false 8, maybe-false 0. Aucun correctif ni report ; verification-gap : aucun trou.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | Numéro d'étape pris de la position de la ligne, pas de sa clé `step` (blind) | false | rejet | `Session::open` pose `next_step = count_lines + 1` (`journal/mod.rs:171`), avec le même `content_lines` : position et `step` coïncident par construction, comme au rejeu. |
| 2 | Noms de sortie recalculés par `output_file` au lieu d'être lus aux `params` (blind) | low | rejet | Le rejeu recalcule de la même façon (`resolve_reference`) ; une commande retirée du registre ne serait plus rejouable non plus. Pas de cas courant. |
| 3 | Règle de consommation plus large que 5.22 (toute chaîne égale à un `layer_id` antérieur) (blind, intent) | low | rejet | Une chaîne résolue égale à un id de couche ne vient que de `step:k`/`step:k.layer_id` : avant exécution, le plan ne peut pas connaître ces ids ; `step:k.name` donne un nom, pas un id. |
| 4 | Cas de l'ancien test `consumed_layers` non repris (listes, `step:0`, `feature_count`) (blind, edge) | false | rejet | La règle lit des valeurs résolues : `feature_count` devient un nombre (cas `"n": 3`, `journal/tests.rs:1343`) ; références hors bornes rejetées par `resolve_references` avant toute ligne ; listes non lues, comme avant. |
| 5 | Un résultat sans fichier de sortie ne peut plus être caché (blind) | false | rejet | En 5.22 aussi, `without_intermediates` exigeait `result_date` (source parmi les fichiers de travail) : une couche sans fichier n'a jamais été cachée. |
| 6 | Une ligne sans `plan_step` coupe le plan en cours (blind, intent) | false | rejet | `plan.execute` tient l'état du serveur jusqu'à la fin du plan (constaté par verification-gap) : aucune autre ligne ne peut s'intercaler. |
| 7 | Journal relu à chaque proposition, sans cache (blind, edge, intent) | low | rejet | Quelques centaines de lignes JSON : de l'ordre de la milliseconde contre 15 à 30 s de composition. Un cache ajouterait un état à garder juste. |
| 8 | Fichiers restés d'un journal effacé ou remis à zéro, de même nom (blind, edge, intent) | low | rejet | Il faut effacer le journal à la main en gardant le dossier de travail ; défaut déjà présent dans `result_date` (5.18). |
| 9 | Ajout sous AD-11 incomplet (rejeu, relecture à chaque proposition) (blind) | low | rejet | Le contrat fixe la règle (clé facultative, écrite par `plan.execute`, ignorée du rejeu) ; le reste relève du code. Ajout décidé par le propriétaire au plan. |
| 10 | Plan absent du diff (blind) | false | rejet | Exclu volontairement : c'est le fichier de claims, lu à part. |
| 11 | Tests manquants : `plan_step` incohérents, étape 1 échouée (blind) | low | rejet | Une étape 1 échouée porte `plan_step: 1` et remet à zéro ; `plan_step` n'est écrit que par `execute`, toujours 1, 2, 3… |
| 12 | Couche intermédiaire rouverte à la main depuis le dossier de travail : cachée (edge) | low | rejet | Réel mais rare (ouvrir un fichier du dossier `.cassini`) ; le fixer demande de suivre les ouvertures. |
| 13 | Couche de l'utilisateur hors dossier de travail au même nom de fichier qu'un intermédiaire : cachée (edge) | low | rejet | Il faudrait un fichier de l'utilisateur nommé `2-analysis.buffer.gpkg` ; comparaison par nom déjà en place depuis 5.18. |
| 14 | `cli/src/main.rs` annoncé « mis à jour » mais inchangé (edge) | false | rejet | Signature inchangée, appel avec ensemble vide valide ; les Implementation Notes le disent. |
| 15 | Recette réelle non testée : forme de `source` après réouverture de QGIS (intent) | false | rejet | Pas un défaut du diff : la recette QGIS du plan (Verification) le confirmera. L'extension réduit `source` au nom de fichier (`source_name`). |
| 16 | Copies du rejeu des couches finales visibles à côté des originaux (intent) | false | rejet | Hors de l'intention (seuls les intermédiaires sont cachés) ; comportement inchangé depuis 5.36. |

## Design Notes

Lien étape ↔ couche rechargée : par le fichier, jamais par l'id (QGIS en donne de neufs, AD-11). L'inventaire rend `source` en nom de fichier ; `work_files` reconnaît déjà le dossier de travail, `replay/` et `replay-<n>/`, où le rejeu écrit sous le même nom `{k}-{id}{ext}`. Consommation dans le journal : une valeur de premier niveau des `params` d'une ligne réussie qui rend un `layer_id`, égale au `layer_id` rendu par une ligne antérieure du même plan.

## Verification

**Commands:**
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `cargo deny check` (PowerShell, `CARGO_TARGET_DIR=C:\ct`) -- expected: verts.
- tests Python de l'extension (WSL) -- expected: verts, inchangés.

**Manual checks (if no CLI):**
- Recette QGIS par Claude (noyau installé avant) : plan démo, fermer et rouvrir QGIS, phrase suivante — les intermédiaires ne reviennent pas dans l'inventaire montré ; puis `journal.replay` et une phrase — idem.
