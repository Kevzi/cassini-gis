---
title: 'Le panneau montre l''historique du journal à la réouverture du projet'
type: 'feature'
ticket: ''
created: '2026-11-08'
status: 'built'
baseline_revision: '129226b5dd05fbf4c62c1cf516b0736af7a34c90'
route: 'oneshot'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** À la réouverture d'un projet fait avec Cassini, le panneau affiche un journal vide, alors que `journal.jsonl` garde toutes les étapes passées (remarque du propriétaire, 2026-11-07, `deferred-work.md`). Le panneau ne connaît que la session en cours (`runlog.py`), et rien sur le canal ne lui rend le journal.

**Approach:** Le noyau, seul lecteur et seul auteur du journal (AD-11), joint à sa réponse à `session.hello` un champ `history` : les 200 dernières étapes du journal, chacune réduite à `{step, command, params, result | error | cancelled, origin?, plan_step?}`. C'est un ajout compatible, comme `read_only` (5.32), et le protocole reste en version 2. L'extension les transmet avec READY, et le panneau en remplit son journal à l'ouverture d'un projet ; les lignes de la session viennent ensuite, et un noyau relancé sur le même projet les garde. Chaque étape passée montre son état (✔, ✖, ⊘). **Décision du propriétaire (2026-11-08) :** la date et l'heure sont utiles ; le journal les garde désormais pour chaque nouvelle étape (`at`, ms depuis 1970), et le panneau les montre (« 08/11 14:02 ») ; les étapes plus anciennes n'en ont pas. Aucune commande n'est ajoutée au catalogue figé, et rien n'est journalisé à l'ouverture. Une ligne illisible du journal est sautée. `docs/canal-local.md` décrit le champ.

</frozen-after-approval>

## Implementation Notes

Route courte, une centaine de lignes en deux côtés. Risque `high` : le canal et le journal, d'où une relecture `thorough`.

Fait :
- Noyau : `Session::history` et `history_of` (`core/src/journal/mod.rs`) ; champ `history` de `session.hello` (`core/src/serve.rs`, `HISTORY_LIMIT` = 200) ; champ `at` dans chaque nouvelle ligne du journal. Le rejeu ne le compare pas : toute la suite du rejeu reste verte.
- Extension : `client.py` transmet `history` avec READY ; `plugin.py` le passe à `set_ready` ; `RunLog.from_history` et `past_time` (`runlog.py`) ; `set_ready` ne recharge le journal qu'à un changement de projet (`panel.py`) ; texte `step_error_code` pour une erreur passée, qui n'a que son code.
- `docs/canal-local.md` décrit le champ.

Vérifié : tests du noyau et suites d'intégration verts, `fmt` et `clippy` propres ; 459 tests de l'extension verts. **La démonstration réelle sous QGIS reste à faire** : rouvrir le projet de Villeneuve avec le noyau et l'extension installés. Elle sera faite après l'entraînement en cours, qui occupe la machine.

## Verification

**Commands:**
- `cargo test -p cassini-core`, `cargo fmt --check`, `cargo clippy -p cassini-core --all-targets -- -D warnings` -- expected: verts et propres
- `python -m unittest discover qgis-plugin/tests` -- expected: verts

## Review Triage Log

Lentilles : blind-hunter (10), edge-case-hunter (8), verification-gap (4 + 3), intent-alignment (descriptif).
- medium, patch : chaque READY remplaçait le journal du panneau. Un noyau relancé sur le même projet perdait les lignes que le journal ne tient pas. Le journal n'est maintenant rechargé qu'à un changement de projet ; essai `test_set_ready_fills_the_log_from_history_once_per_project`.
- medium, patch : au changement de projet, le résultat affiché (`last`, `_shown_result`) restait celui de l'ancien projet. Il est remis à zéro.
- medium, patch : un résultat résumé (`{truncated, bytes, fingerprint}`) était montré comme un vrai résultat. Il n'est plus repris, et des paramètres résumés non plus.
- low, patch : `error = (code, code)` affichait le code deux fois. Le message est vide, et le texte `step_error_code` ne montre que le code.
- medium, patch : `params` n'était pas plafonné, et la réponse à `session.hello` pouvait devenir très longue. Ils sont résumés comme un résultat (`summarize`).
- low, patch : `layer_id` était perdu ; il est gardé.
- low, patch : un `plan_step` booléen passait pour un numéro ; il est écarté.
- low, patch : `history_of` était `pub` ; il est passé en `pub(crate)`.
- medium, patch : aucun essai ne couvrait le client, le greffon, le vrai `set_ready`, ni `cancelled`, `origin` et les paramètres résumés côté noyau. Essais ajoutés.
- low, rejeté : lecture de tout le journal à chaque ouverture. `Session::open` le lit déjà en entier (`count_lines`) ; une lecture de plus, du même ordre, ne change pas la poignée de main.
- low, rejeté : pas de séparateur entre le passé et la session. La date des lignes passées les distingue maintenant.
- low, rejeté : numéros « 1. » de plusieurs plans confondus. La date sépare les plans.
- false : plan hors de l'arbre des tickets (`ticket: ''`). C'est un travail hors arbre, comme les autres correctifs de `deferred-work.md`.
- defer : démonstration réelle sous QGIS, après l'entraînement (notée ci-dessus).
