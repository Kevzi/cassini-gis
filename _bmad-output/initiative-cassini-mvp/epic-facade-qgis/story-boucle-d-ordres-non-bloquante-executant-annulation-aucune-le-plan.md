---
title: "Boucle d'ordres non bloquante : executant, annulation, aucune lecture bloquante"
type: 'feature'
ticket: '2'
created: '2026-10-15'
status: 'built'
baseline_revision: '194dab620f7a7f3e1d28dab47b175a55704f2d72'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/docs/canal-local.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'extension sait lancer le noyau (9.1) mais ignore tout message `method` venant de lui : elle ne peut ni executer un `order.execute`, ni relayer une annulation, et son `stop()` bloque l'interface.

**Approach:** faire de `CoreClient` l'executant distant du noyau : un registre de gestes, une boucle d'ordres pilotee par les signaux de QProcess, `request.cancel` relaye vers le geste en cours, stderr draine en continu. Un geste de test lent prouve que QGIS ne gele jamais.

## Boundaries & Constraints

**Always:** Qt uniquement via `qgis.PyQt` ; aucune logique metier dans Python (AD-4) ; un seul ordre a la fois (AD-5) ; toute annulation se termine par une reponse a l'ordre, jamais par la mort du processus (AD-12) ; `session.hello` annonce les ids des gestes enregistres ; protocole version 1 inchange ; comportements 9.1 conserves (ordre du localisateur, FAILED/DIED, arret a la fermeture) ; testable sans QGIS.

**Decisions:** (1) un geste inconnu rend `error` avec `data.code` = `qgis.gesture_unknown` ; (2) `stop()` garde son `waitForFinished(3000)`, car il ne sert qu'a la fermeture de QGIS (comportement 9.1 conserve).

**Never:** modifier `core/src/serve.rs`, `registry.rs`, `docs/canal-local.md` ; socket, urllib, requests, PyQt5/PyQt6 directs ; lecture bloquante (`waitFor*`) dans la boucle d'ordres ; tuer le noyau pour annuler ; garde Python et job de recette (9.3).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Ordre nominal | `order.execute` `test.qgis_probe` | geste lance sans bloquer, puis `{"id","result":<valeur>}` | — |
| Annulation | `request.cancel` pendant un geste lent | le geste s'arrete, l'ordre recoit une reponse, le noyau rend `registry.cancelled` | — |
| Geste inconnu | `order.execute` d'un id non enregistre | reponse `error` avec `data.code` | `qgis.gesture_unknown` |
| Geste en echec | le geste leve une exception | reponse `error` code -32000, `data.code`/`data.message` | exception jamais propagee a Qt |
| stderr bavard | gros volume sur stderr | lu a chaque `readyReadStandardError`, rien ne bloque | octets ignores |
| Annulation sans ordre | `request.cancel` d'un id inconnu | ignoree | pas de reponse (notification) |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/client.py` -- `CoreClient` : `_handle` jette les messages `method` (point d'extension) ; `_send` a des ids entiers fixes ; `stop()` appelle `waitForFinished(3000)`.
- `qgis-plugin/cassini_qgis/gestures.py` -- NOUVEAU : registre id -> geste, contrat de jeton d'annulation.
- `qgis-plugin/cassini_qgis/plugin.py` -- lance le client ; importe qgis, non teste unitairement.
- `qgis-plugin/tests/test_tracer.py` -- `FakeProcess`, `reply()`, `ClientTest` ; a etendre (stderr alimentable).
- `core/src/commands/test_commands.rs` (~l.100) -- `test.qgis_probe`, cible du geste lent ; lecture seule.
- `docs/canal-local.md` -- formes des messages ; lecture seule.
- `qgis-plugin/README.md` -- recette manuelle a completer.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/cassini_qgis/gestures.py` -- registre et contrat d'un geste asynchrone (demarrage, fin par rappel, annulation) -- aucune lecture bloquante possible
- [ ] `qgis-plugin/cassini_qgis/client.py` -- traiter `order.execute` et `request.cancel`, hello annonce les gestes, ids generiques -- extension de la boucle
- [ ] `qgis-plugin/cassini_qgis/plugin.py` -- enregistrer le geste `test.qgis_probe`, bouton d'arret -- cablage
- [ ] `qgis-plugin/tests/` -- tests : ordre nominal, annulation, geste inconnu, exception, stderr bavard, ordre pendant ordre -- couvre la matrice
- [ ] `qgis-plugin/README.md` -- recette manuelle « QGIS ne gele pas pendant un geste lent »

**Acceptance Criteria:**
- Given un geste de test lent, when le noyau envoie `request.cancel`, then l'extension repond a l'ordre et `command.run` rend `registry.cancelled`, sans appel bloquant.
- Given un stderr tres volumineux, when il arrive, then chaque lecture le vide et la lecture de stdout continue.
- Given `python -I -m unittest discover -s qgis-plugin/tests`, then tous les tests passent sans QGIS.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 1 medium-bas patche, 1 false, 7 rejetes (low/hors perimetre).

| Verdict | Route | Constat et evidence |
|---|---|---|
| low | patch (fait) | `_handle_call` repondait `cancelled: true` pour tout ordre en cours, meme si l'id differe ; corrige (`matches`) + test ajoute. |
| low | rejete | `ORDER_ENDED` non emis a la mort du noyau : DIED est livre apres READY (`_done` reste faux) et le plugin masque alors le bouton ; cas contraire sans effet. |
| low | rejete | Geste qui ne rappelle jamais `done` : violation de contrat (plan : doit rappeler), SlowProbe la respecte ; garde-fou ajouterait de la complexite. |
| low | rejete | `id` non hashable dans `_pending.pop` : code anterieur, le noyau n'envoie que des chaines. |
| low | rejete | `plugin.py` non teste : importe qgis, non testable sans QGIS (Code Map) ; couvert par la recette manuelle. |
| false | rejete | `SlowProbe` ignore `params` : conforme a `QgisProbeOutput`. |
| false | rejete | `stop()` bloquant dans `_fail` : comportement 9.1, decision 2 du plan. |
| false | rejete | Test stderr ne relit pas stdout : `self.order()` apres l'inondation prouve la lecture. |
| low | rejete | Pas de test d'integration avec le vrai noyau : hors ticket (9.3). |


## Design Notes

Un geste recoit un `done(result|error)` et un `cancel()` ; il est pilote par la boucle d'evenements Qt (QTimer ou QgsTask), jamais par une attente. Le client garde un seul `current_order` ; `request.cancel` appelle `cancel()` et le geste doit tout de meme rappeler `done` pour que l'ordre recoive une reponse.

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: OK
- `cargo test` -- expected: noyau inchange, vert

**Manual checks (if no CLI):**
- Dans QGIS, lancer le geste lent puis l'annuler : l'interface reste reactive.
