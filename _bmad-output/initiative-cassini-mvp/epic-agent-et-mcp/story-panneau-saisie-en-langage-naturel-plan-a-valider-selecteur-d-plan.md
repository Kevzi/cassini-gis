---
title: 'Panneau : saisie en langage naturel, plan a valider'
type: 'feature'
ticket: '6'
created: '2026-10-18'
status: 'built'
baseline_revision: '300ac5ee346d06b22abcf22b71e4189a61c9fb68'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md', '{project-root}/docs/canal-local.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le canal sait proposer, exécuter et refuser un plan (5.4, 5.5), mais le panneau QGIS garde la saisie grisée : l'utilisateur ne peut ni écrire sa demande, ni voir le plan, ni le valider.

**Approach:** Le panneau active la saisie et envoie `plan.propose` ; pendant la génération il montre un indicateur ; il affiche ensuite le plan en lecture seule (étapes numérotées, statut, marqueur de l'étape en cours) ou la question, avec Valider le plan / Refuser qui envoient `plan.execute` / `plan.reject`. Découpe du propriétaire (2026-10-18) : le sélecteur de modèle est reporté (`deferred-work.md`), le modèle reste le premier du dossier et la ligne Modèle des Réglages reste grisée.

## Boundaries & Constraints

**Always:** Aucune logique métier dans le Python (AD-4, AD-5) : le panneau n'enchaîne rien, n'édite aucune étape et ne renvoie que le `plan_id`. Microcopy et états de `EXPERIENCE.md` (FR/EN, mêmes clés) ; aucun chemin absolu ni code à l'écran (AD-9) ; statut = icône + mot. Entrée envoie, Maj+Entrée saute une ligne ; saisie grisée pendant une génération, un plan en attente ou une exécution, et hors état prêt. Une seule requête en vol côté client (commande, rejeu ou plan). Arrêter pendant `plan.execute` envoie `request.cancel` sur son id ; grisé avec sa raison si la commande de l'étape n'est pas annulable (catalogue). Qt par `qgis.PyQt` seulement. Logique pure (états, libellés, activations) dans un module sans Qt, testée par `unittest`.

**Never:** Toucher le noyau Rust ou le protocole (`core/`, `cli/`, `ai/`). Sélecteur de modèle (reporté). Serveur MCP (5.7). Dialogue de réponse à une question (la réponse est une nouvelle demande). Phrase de résultat propre à chaque commande (NOTE FOR UX). Nouvelle dépendance.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan proposé | Envoyer une phrase | indicateur pendant la génération, puis « Voici ce que je propose. » et les étapes ; Valider/Refuser actifs ; saisie grisée | — |
| Valider | plan affiché | `plan.execute {plan_id}` ; à chaque `plan.step` le marqueur avance, une ligne de journal par étape passe à Terminée ; saisie rendue | — |
| Étape en erreur | erreur portant `step` | « Étape {n} en erreur ({code}) : {message} » ; étape et ligne en Erreur ; suivantes non marquées | panneau libéré |
| Arrêter | étape annulable en vol | `request.cancel` ; étape et ligne Annulée | non annulable : Arrêter grisé, raison |
| Refuser | plan affiché | `plan.reject` ; « Plan refusé. Reformulez votre demande. » ; saisie rendue | — |
| Question | réponse `{"question"}` | question affichée comme réplique, sans Valider ; saisie rendue | — |
| Échec de proposition | `agent.refused`, `ai.no_model`, `agent.plan_invalid`… | message français du noyau et son code ; saisie rendue | — |
| Noyau mort | pendant génération ou exécution | plan oublié, ligne en cours en Erreur, bandeau Relancer | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/client.py` -- état `_plan` à côté de `_run` / `_replay` (l.57-58), à ajouter aux gardes de `run_command` (l.118) et `replay` (l.138). Nouvelles méthodes `propose(request)`, `execute_plan(plan_id)`, `reject_plan(plan_id)` ; `cancel_run` (l.143) couvre aussi le `plan.execute` en vol. `_handle` (l.208) : réponses `propose` / `execute` / `reject` ; `_handle_call` (l.295) : notification `plan.step` (sans id, jamais de réponse). `_finished` / `_error` (l.411) soldent le plan en vol comme `_fail_run`. Nouveaux événements `PLAN_PROPOSED {plan_id, plan}`, `PLAN_QUESTION {question}`, `PLAN_FAILED {code, message}`, `PLAN_STEP {step, command}`, `PLAN_DONE {steps}`, `PLAN_EXEC_FAILED {code, message, step}`, `PLAN_CANCELLED {step}`, `PLAN_REJECTED`. Le `_fail_run` actuel sert de modèle.
- Protocole (lecture seule) : `docs/canal-local.md`, section « Plans de l'agent » : formes de réponse, `step` dans `error.data`, `registry.cancelled` pour l'annulation, `plan.unknown`.
- `qgis-plugin/cassini_qgis/agentview.py` (nouveau, sans Qt) -- état du plan : étapes (libellé, commande, statut `pending|running|done|error|cancelled`), étape en cours, phase `idle|generating|proposed|executing` ; libellé = description du catalogue dans la langue (`texts.pick`, repli : id) ; fonctions d'activation (saisie, Envoyer, Valider, Refuser, Arrêter et sa raison). Statuts et icônes repris de `runlog` (l.12-18) ; marqueur d'étape en cours `▶`.
- `qgis-plugin/cassini_qgis/panel.py` -- saisie (l.259-269) active, filtre clavier Entrée / Maj+Entrée ; nouvelle zone plan sous la saisie : titre, indicateur (texte + `QProgressBar` indéterminée), liste lecture seule, Valider / Refuser / Arrêter, détails techniques du plan (`runlog.redact`) ; journal : une ligne par étape (`self.log.start` sur `PLAN_STEP`, `finish` à l'étape suivante ou au verdict) ; ordre de tabulation (l.341). Le mode expert reste tel quel ; il est grisé pendant un plan.
- `qgis-plugin/cassini_qgis/plugin.py` -- actions `propose(text)`, `approve()`, `reject()` ; `stop()` (l.101) inchangé si `cancel_run` couvre le plan ; `_on_event` (l.162) route les `PLAN_*` vers le panneau.
- `qgis-plugin/cassini_qgis/texts.py` -- clés FR/EN : `plan_title`, `approve`, `reject`, `plan_rejected`, `generating` (« Je prépare un plan… La première demande charge le modèle et peut prendre plusieurs minutes. »), `question_title`, `propose_error` (« La demande n'a pas abouti ({code}) : {message} »), `plan_done` (« Plan exécuté : {n} étapes terminées. »), `input_reason` remplacé par une raison d'attente. Le test d'égalité des clés existe déjà.
- `qgis-plugin/tests/test_tracer.py` -- `FakeProcess`, `Signal` : à réutiliser dans le nouveau test client ; `test_panel_logic.py` : motif d'import sans Qt.
- `qgis-plugin/README.md` -- section recette manuelle.

## Tasks & Acceptance

**Execution:**
- [x] `qgis-plugin/cassini_qgis/client.py` + `qgis-plugin/tests/test_plan_client.py` -- envoi, réponses, `plan.step`, garde une seule requête, annulation, mort du noyau -- lignes de la matrice côté canal
- [x] `qgis-plugin/cassini_qgis/agentview.py` + `qgis-plugin/tests/test_agent_view.py` -- statuts, marqueur, libellés, activations -- logique testable sans QGIS
- [x] `qgis-plugin/cassini_qgis/texts.py` -- clés FR/EN -- microcopy
- [x] `qgis-plugin/cassini_qgis/panel.py`, `qgis-plugin/cassini_qgis/plugin.py` -- zone plan, saisie, routage -- l'interface
- [x] `qgis-plugin/README.md` -- recette manuelle : phrase de démo, Valider, Refuser, question, Arrêter -- hitl

**Acceptance Criteria:**
- Given QGIS 3.44, un projet ouvert et un modèle dans le dossier des modèles, when le propriétaire envoie la phrase de démo, then un plan s'affiche, Valider l'exécute étape par étape et le journal montre Terminée pour chaque étape ; sur un autre plan, Refuser rend la main.
- Given la suite sans QGIS, when `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` tournent, then tout passe.

## Implementation Notes

- `plugin.approve(plan_id)` / `plugin.reject(plan_id)` prennent le `plan_id` tenu par `panel.agent` : le plugin reste sans état de plan.
- Clés ajoutées en plus de la liste : `input_reason_plan` (plan en attente), `pending` (statut « En attente », icône ○), `plan_steps_name`, `plan_cancelled`, `live_plan`, `live_question`. Hors état prêt, la saisie n'affiche aucune raison propre : le bandeau dit pourquoi.
- Le résultat d'exécution (plan exécuté, étape en erreur, étape annulée) va dans la zone Résultat ; la zone plan garde la réplique (génération, question, refus, échec de proposition) et les étapes.
- Arrêter reste grisé, sans raison, entre `plan.execute` et le premier `plan.step` (aucune étape connue).
- Vérifié aussi par une sonde jetable hors dépôt sous QGIS 3.44.15 (plateforme offscreen) : panneau instancié, Entrée / Maj+Entrée, tous les événements `PLAN_*` rejoués.

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment ; CLAUDE.md §8 l'emporte sur le `quick` épinglé). 0 high, 0 medium, 12 low corrigés, 2 gaps de vérification corrigés, reste rejeté.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | low | patch | blind + edge : plan sans étape → zone plan cachée, saisie grisée, panneau bloqué. La grammaire impose une étape (5.4, n°11) mais rien ne garde côté panneau. Zone visible dès que Valider/Refuser le sont ; test. |
| 2 | low | patch | blind : la phrase est effacée à l'envoi et perdue après un refus ou un échec. Remise dans la saisie sur PLAN_FAILED si elle est vide. |
| 3 | low | patch | blind : commentaire « telle quelle » alors que la demande est `strip()`. Commentaire corrigé. |
| 4 | low | patch | blind + edge : `plan.step` et `agentview.done` acceptent un booléen comme numéro d'étape. Exclu ; tests. |
| 5 | low | patch | blind + edge : projet fermé pendant un plan → `client.stop` ne rend aucun événement, le résultat reste « En cours… ». Résultat périmé effacé dans `_drop_stale_run` (chemin expert inchangé, antérieur). |
| 6 | low | patch | blind + edge : titre « Voici ce que je propose. » laissé au-dessus d'un plan exécuté. Titre seulement en phase proposée ou en exécution. |
| 7 | low | patch | blind + intent : « 1 étapes terminées » ; `settings_model` annonce un choix « avec l'agent » alors que l'agent existe. Libellés reformulés FR/EN. |
| 8 | low | patch | blind + edge : `textChanged` reconstruit la liste des étapes et resérialise le plan à chaque frappe. Mise à jour du seul bouton Envoyer. |
| 9 | low | patch | blind : liste des PLAN_* écrite deux fois (panel, plugin), commentaire des bandeaux déplacé. Définie une fois dans `client.py`. |
| 10 | gap | patch | verification-gap : routage PLAN_* du plugin et actions propose/approve/reject sans test. Tests dans `PluginBehaviourTest`. |
| 11 | gap | patch | verification-gap + blind : choix du texte de résultat d'un plan dans la classe Qt, non testé. Sorti dans un utilitaire pur d'`agentview`, testé. |
| 12 | low | patch | blind : `PlanTextsTest` ne couvre pas quatre clés nouvelles. Ajoutées. |
| 13 | false | rejeté | blind + verification-gap : refus en échec qui oublie un plan encore en attente. Le panneau envoie le `plan_id` en attente ; seul `plan.unknown` peut revenir, et il dit justement que ce plan n'attend plus (doc du canal) ; le client interdit tout `session.busy`. |
| 14 | low | rejeté | blind : annonces vocales « Commande terminée/annulée/en erreur » reprises pour le plan, rien sur refus ou étape. Annonce présente et exacte ; plancher d'accessibilité tenu ; textes nouveaux pour un gain mince. |
| 15 | low | rejeté | blind + edge : `plan.step` d'un autre `plan_id` ou d'un numéro absent. Le noyau n'exécute qu'un plan à la fois et numérote depuis le plan rendu (5.4) ; garde sans cas démontré. |
| 16 | low | rejeté | blind : Arrêter cliqué plusieurs fois envoie plusieurs `request.cancel`. Le noyau répond à chacun sans effet de plus ; ajouter un état « arrêt en cours » est une branche pour un cas sans dommage. |
| 17 | low | rejeté | blind + edge : génération sans délai ni annulation. Décision de 5.4 (composition non annulable) ; fermer le projet arrête le noyau ; un délai côté client contredirait la première demande de plusieurs minutes. |
| 18 | low | rejeté | edge : `propose` refusé par le client ne dit rien. Seul cas : pas de client, et le bandeau l'explique déjà. |
| 19 | — | note | intent-alignment : lecture L2 implémentée (parcours complet), question en réplique, statut par étape, découpe du modèle par omission ; la surface Qt n'est vérifiée que par la recette manuelle (hitl), le reste par tests sans Qt. |

## Design Notes

Libellé d'étape : `1. Fait une zone tampon autour des entités d'une couche.` (description du registre) ; les paramètres vont dans « Détails techniques », masqués par `runlog.redact`. Le résultat final est générique (`plan_done`) : une phrase par commande reste à concevoir (NOTE FOR UX).

La composition bloque le noyau sans être annulable (5.4) : pendant la génération, Arrêter n'est pas proposé, seul l'indicateur l'est.

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: succès
- `python -I qgis-plugin/tests/check_extension.py` -- expected: code 0

**Manual checks (if no CLI):**
- QGIS 3.44, projet de démo ouvert : phrase de démo → plan → Valider → journal Terminée ; Refuser ; question ; Arrêter (recette du propriétaire, hitl).
