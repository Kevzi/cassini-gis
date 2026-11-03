---
title: "Couches retirées par le budget d'inventaire signalées"
type: 'feature'
ticket: '33'
created: '2026-11-03'
status: done
baseline_revision: '10f5cd0ee5859bab8c31b1e431a6e80f2bc631a3'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le budget d'inventaire (5.18, retrait par paquets 5.19) retire de la vue du modèle les couches de résultat les plus anciennes quand la session s'allonge, sans le dire : une phrase qui cite une de ces couches (« la zone tampon de tout à l'heure ») finit en question ou en plan sans explication pour l'utilisateur.

**Approach:** Le noyau note, à chaque `plan.propose`, les couches de résultat que le budget a laissées hors de la vue du modèle, et les rend avec la réponse (plan ou question) ; le panneau affiche combien il y en a et lesquelles, et rien quand il n'y en a pas.

## Boundaries & Constraints

**Always:** seules les couches retirées **par le budget** comptent (pas les intermédiaires cachées par 5.22, qui sont voulues) ; identifiants rendus = vrais identifiants QGIS, jamais les courts de 5.19 ; clé absente quand rien n'est retiré (réponse inchangée sous le budget) ; une extension antérieure qui ignore la clé continue de marcher ; textes FR et EN avec les mêmes clés.

**Never:** dire au modèle qu'une couche est absente (famille d'entraînement, hors ticket) ; changer le prompt, l'inventaire vu par le modèle ou l'ordre du budget ; retirer quoi que ce soit de QGIS ; journaliser cette information ; la porter sur l'erreur `agent.refused` ; vérifier l'effet WAL sur la date d'un `.gpkg` (report séparé, essai à la main).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Sous le budget | inventaire qui tient sous 14 000 car. | réponse sans clé `left_out` ; panneau sans mention | — |
| Au-delà, plan | 10 résultats lourds, 5 retirés | `{"plan_id", "plan", "left_out": [{"layer_id","name"} ×5]}`, du plus ancien au plus récent | — |
| Au-delà, question | même inventaire, le modèle pose une question | `{"question","source","left_out":[…]}` | — |
| Noms répétés | 5 couches « Intersection » | panneau : « 5 couches de résultat anciennes … : Intersection ×5 » | — |
| Intermédiaires seules | résultats cachés par 5.22, sous le budget | pas de `left_out` | — |
| Clé malformée | `left_out` absent, non liste, éléments sans `name` | panneau : rien, ou l'élément ignoré | jamais d'exception |

</frozen-after-approval>

## Code Map

- `core/src/agent_inventory.rs` -- `ModelInventory` (l. ~368) : ajouter `left_out: Vec<Value>` (`{"layer_id","name"}`). `model_inventory` (l. ~380) : comparer les couches de `lighter` (ids complets) à celles gardées par `within_budget_by_batches` (ids courts, via `ids.shorten`, qui garde ordre et nombre) ; les absentes, dans l'ordre stable (plus ancien d'abord), vont dans `left_out`. Ne pas changer la signature de `within_budget_by_batches` (nombreux tests).
- `core/src/serve/plans.rs` -- `propose` (l. ~123) : la fermeture `inventory` recopie `view.left_out` dans une variable locale, comme `short_ids` (dernier appel fait foi) ; l'ajouter à la réponse question et plan seulement si non vide.
- `core/src/serve.rs` -- tests : `the_composer_sees_an_inventory_reduced_to_the_budget` (l. ~2578) est le cas au-delà du budget à réutiliser ; `a_question_is_a_result_without_plan_id_and_is_never_executed` (l. ~2450) pour la question.
- `cli/src/main.rs` -- construction de `ModelInventory` dans un test (l. ~1324) : ajouter `left_out: Vec::new()`.
- `docs/canal-local.md` -- section `plan.propose` (l. ~174-227) : documenter la clé.
- `qgis-plugin/cassini_qgis/client.py` -- `_plan_answer` propose (l. ~575) : passer `left_out` (liste filtrée) dans les charges `PLAN_QUESTION` et `PLAN_PROPOSED`.
- `qgis-plugin/cassini_qgis/agentview.py` -- `AgentView.reset/proposed/question` : garder `left_out` ; nouvelle fonction pure `left_out_text(view, lang)` (None si vide).
- `qgis-plugin/cassini_qgis/texts.py` -- clé `left_out` FR/EN.
- `qgis-plugin/cassini_qgis/panel.py` -- un `QLabel` `left_out_note` sous `plan_notice` (l. ~392/424), rempli au rafraîchissement (l. ~913).
- `qgis-plugin/tests/test_plan_client.py`, `test_agent_view.py` -- tests sans Qt.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/agent_inventory.rs` -- champ `left_out` et calcul dans `model_inventory` + test unitaire (au-delà : ids complets et noms des retirés, ordre ; sous le budget : vide ; intermédiaires 5.22 non comptées) -- source unique du constat.
- [x] `core/src/serve/plans.rs` -- rendre `left_out` avec plan et question si non vide -- contrat du canal.
- [x] `core/src/serve.rs` -- étendre le test du budget (clé présente, 5 éléments, vrais ids) ; vérifier l'absence de la clé dans un test sous le budget et sur une question au-delà -- tests du noyau sans QGIS.
- [x] `cli/src/main.rs` -- adapter la construction de test.
- [x] `docs/canal-local.md` -- documenter `left_out`.
- [x] `qgis-plugin/cassini_qgis/client.py`, `agentview.py`, `texts.py`, `panel.py` -- transport, état, texte, affichage.
- [x] `qgis-plugin/tests/test_plan_client.py`, `test_agent_view.py` -- charge transmise, texte présent/absent, noms répétés, clé malformée.

**Acceptance Criteria:**
- Given une session dont l'inventaire dépasse le budget, when l'utilisateur envoie une phrase, then le panneau affiche, sous le plan ou la question, le nombre de couches de résultat que le modèle ne voit plus et leurs noms.
- Given une session sous le budget, when l'utilisateur envoie une phrase, then aucune mention n'apparaît.
- Given une nouvelle proposition, un refus ou une erreur, when elle arrive, then la mention de la proposition précédente disparaît.

## Implementation Notes

- Constat calculé par `left_out_by_budget` (`agent_inventory.rs`) : couches de résultat de la vue allégée (avant budget, ids complets) dont le court n'est plus dans la vue bornée ; signature de `within_budget_by_batches` inchangée.
- Filtre de la clé côté extension : `client.left_out_of`, réutilisé par `agentview` ; la charge `PLAN_PROPOSED`/`PLAN_QUESTION` ne porte `left_out` que s'il reste au moins un élément valide (charges d'avant inchangées sinon).
- Mention dans `left_out_note`, placé après `plan_list` (sous le plan, ou sous la question quand la liste est cachée, critère d'acceptation 1) ; effacée par `reset()` (nouvelle proposition, question, refus, erreur, nouvelle demande).
- `cargo test -p cassini` et la clippy de l'espace de travail passent par PowerShell (`LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`) : sous WSL, `cassini-ai` ne trouve pas libclang.

- Recette QGIS (2026-11-03, Claude, noyau `--features gpu` installé, modèle ajusté 4b) : 20 phrases enchaînées sur une copie du projet de Villeneuve. Phrases 1-15 sous le budget : aucune clé `left_out`, aucune mention. Phrase 16 et suivantes : 8 couches hors de vue (étapes 4, 6, 9, 10, 11, 13, 14, 18), conformes à la mesure hors QGIS des inventaires relevés une fois les intermédiaires de 5.22 ôtées ; mention FR/EN affichée sur plans et questions. Phrase 20 (« la zone tampon de 200 m … de tout à l'heure ») : question du modèle, et la mention cite `10-analysis.buffer`. Une recette de 12 phrases n'avait pas atteint le budget (vue allégée sous 14 000 car.).

## Plan Change Log

## Review Triage Log

**Passe 1 (quick, 2026-11-03)** — high 0, medium 0, low 2, false 1, maybe-false 0.

| Verdict | Route | Constat | Preuve / action |
|---|---|---|---|
| false | rejeté | Risque `low` et relecture quick alors que le contrat du canal change (CLAUDE.md §8) | La règle §8 vise les entrées de risque `high` ; le ticket est `low` dans l'arbre, l'ajout est une clé facultative, ignorée par une extension antérieure ; la correction éditerait le plan. |
| low | patch | Couche de résultat au nom vide : le noyau rend `"name": ""`, l'extension l'écarte, le compte est trop bas | Repli sur l'identifiant quand le nom est vide ou blanc (`left_out_by_budget`), test unitaire. |
| low | patch | « ancienne(s) » / « older » faux quand le budget retire le résultat le plus récent (`kept.pop_back()`) | Mot retiré des textes FR/EN ; doc du canal précisée ; tests du texte mis à jour. |

## Design Notes

Texte FR : « Le modèle ne voit plus {n} couche(s) de résultat de cette session (d'ordinaire les plus anciennes, parfois la plus récente quand elle ne tient pas seule) : {noms}. » Noms distincts dans l'ordre d'arrivée, suffixés « ×k » quand répétés. EN équivalent. Accord singulier/pluriel par deux clés (`left_out_one`, `left_out_many`) si `texts.tr` n'a pas de pluriel.

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: vert, nouveaux tests compris
- `cargo test -p cassini` -- expected: vert
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `cd qgis-plugin && python3 -m unittest discover -s tests` -- expected: vert
