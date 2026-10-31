---
title: "Le panneau montre chaque appel de l'IA externe et peut l'arrêter"
type: 'feature'
ticket: '26'
created: '2026-10-31'
status: 'built'
baseline_revision: '45e4d2c0c41adcdc7858ae884060b32f0ee8e4a4'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context:
  - '{project-root}/docs/canal-local.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le noyau sait servir une IA externe (5.24, 5.28, 5.29), mais l'extension ignore `external.call`, répond « méthode inconnue » à `external.approve` et n'a aucun moyen d'ouvrir le canal : l'utilisateur ne peut ni autoriser, ni voir, ni arrêter, ni valider ce que fait l'IA externe (AD-29, flux 4).

**Approach:** Réglages : interrupteur « Autoriser une IA externe » et case « Me demander avant chaque appel », envoyés par `session.external`. Panneau : bandeau avec **Couper**, appels en direct au journal marqués « IA externe » avec **Arrêter**, question **Autoriser / Refuser** sans fenêtre modale, saisie grisée « L'IA externe travaille » pendant un appel.

## Boundaries & Constraints

**Always:**
- Interrupteur éteint à chaque démarrage du noyau, jamais rallumé seul ; l'état affiché est celui que le noyau a répondu.
- Case éteinte par défaut, **retenue dans QSettings** (décision du propriétaire, 2026-10-31) ; la changer interrupteur allumé renvoie `session.external {enabled:true, ask}`.
- `external.channel_taken` : interrupteur éteint et grisé, « Une autre session QGIS accepte déjà l'IA externe. » ; il redevient cliquable à la prochaine ouverture des Réglages.
- Ligne du journal par `call_id` : commande, paramètres et résultat au clic (chemins masqués par `runlog.redact`), durée, état ; **Arrêter** envoie `request.cancel {id: call_id}`, grisé avec sa raison si la commande n'est pas annulable.
- Question d'accord : une seule ; réponse `{"approved": true|false}` à son `id` ; `request.cancel` du noyau sur cet `id` la ferme sans réponse ; jamais de `exec()` ni d'attente bloquante.
- Appel externe en vol ou question ouverte : Envoyer, Lancer et Valider le plan grisés, raison « L'IA externe travaille ».
- Noyau mort, relancé ou arrêté : interrupteur éteint, question fermée, lignes externes en cours closes annulées.
- Logique sans Qt dans un module testé ; Qt seulement par `qgis.PyQt` (AD-18) ; aucune logique métier (AD-4).

**Never:**
- Rien dans le noyau Rust ni `cassini mcp` ; pas de nouvelle méthode ni version de protocole.
- Pas de file d'appels ni de seconde question ; pas d'accord donné par défaut.
- Aucun chemin absolu affiché (AD-9).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Allumer | interrupteur coché | `session.external {enabled:true, ask}` ; réponse → bandeau + Couper | `channel_taken` : grisé et raison ; autre erreur : éteint, message |
| Couper | Couper ou interrupteur décoché | `{enabled:false}` ; bandeau caché | — |
| Appel | `external.call` started puis finished | ligne « IA externe » en cours puis Terminée / Erreur, durée | `finished` sans `started` : ligne créée close |
| Arrêter | clic pendant l'appel | `request.cancel {id: call_id}` | non annulable : bouton grisé, raison |
| Accord | `external.approve` (id `core-n`) | question ; Autoriser → `{approved:true}`, Refuser → `{approved:false}` | 2e demande : la 1re reçoit `false` |
| Expiration | `request.cancel {id:"core-n"}` du noyau | question fermée, aucune réponse | clic tardif : rien n'est envoyé |
| Occupé | appel en vol ou question ouverte | saisie, Lancer, Valider grisés « L'IA externe travaille » | `session.busy` reçu : affiché comme aujourd'hui |
| Mort | noyau arrêté pendant un appel ou une question | interrupteur éteint, question fermée, ligne annulée | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/client.py` -- `_handle_call` (l.499) : `external.call` → événements `EXTERNAL_CALL_STARTED/FINISHED` ; `external.approve` → `APPROVAL_ASKED {request_id, call_id, command, params}` gardé dans `self._approval` ; `request.cancel` (l.523) : si l'`id` est celui de la question → `APPROVAL_WITHDRAWN`. Nouvelles méthodes `set_external(enabled, ask)` (requête hors `_can_send`, étape `"external"` dans `_handle`, l.316 → `EXTERNAL_STATE` / `EXTERNAL_FAILED {code, message}`), `answer_approval(request_id, approved)`, `cancel_external(call_id)` (étape `"cancel"`). `_finished`/`_error`/`stop` oublient la question. Réutiliser `_respond`, `_request`, `_error_fields`, `_error_message`, `_notify`.
- `qgis-plugin/cassini_qgis/external.py` (nouveau, sans Qt) -- état : `enabled`, `ask`, `taken` (message ou None), `calls` {call_id: Entry}, `approval` ; transitions par événement ; `controls(state, ready, catalogue)` → visibilité du bandeau, Couper, Arrêter (+ raison via `agentview.is_cancellable`), interrupteur actif, raison ; `busy(state)` ; texte de la question (description du catalogue + `runlog.redact(params)` en JSON).
- `qgis-plugin/cassini_qgis/runlog.py` -- `Entry` gagne `origin` (None/`"external"`), `call_id`, `duration_ms` ; `line_text` ajoute la durée ; `RunLog.finish` ne doit pas fermer une ligne externe (filtrer `origin is None`), `finish_call(call_id, status)` pour les externes ; `controls` reçoit `external_busy`.
- `qgis-plugin/cassini_qgis/agentview.py:282` -- `controls(..., external_busy=False)` : saisie et Valider grisés, raison `input_reason_external`.
- `qgis-plugin/cassini_qgis/panel.py` -- Réglages (l.273) : `QCheckBox` interrupteur, case, raison ; bandeau externe distinct de `banner_zone` (l.298, réservé aux états du noyau) avec Couper et Arrêter ; zone question avec Autoriser / Refuser ; `on_event` route les nouveaux événements ; `set_ready`/`_drop_stale_run` remettent l'état externe à zéro ; ordre de tabulation (l.441). Lignes externes : libellé `t("external_tag") · description`, clic → `summary.detail` existant.
- `qgis-plugin/cassini_qgis/plugin.py` -- actions `set_external`, `stop_external`, `answer_approval` ; `_on_event` (l.177) transmet les nouveaux événements au panneau.
- `qgis-plugin/cassini_qgis/texts.py` -- clés fr/en : interrupteur, case, bandeau, Couper, `external_tag`, `input_reason_external`, question, Autoriser, Refuser, canal pris, durée.
- `qgis-plugin/tests/test_external.py` (nouveau) -- `FakeProcess`, `reply` de `test_tracer` ; client et `external.py`.
- `docs/canal-local.md:365` -- retirer « L'extension, qui ne connaît pas encore cette notification, l'ignore » ; dire ce que fait l'extension.

## Tasks & Acceptance

**Execution:**
- [x] `qgis-plugin/cassini_qgis/client.py` -- `session.external`, `external.call`, `external.approve`, retrait, mort -- transport sans logique.
- [x] `qgis-plugin/cassini_qgis/runlog.py`, `agentview.py` -- ligne externe, occupation externe -- grisage commun.
- [x] `qgis-plugin/cassini_qgis/external.py` -- état et contrôles -- logique testable sans QGIS.
- [x] `qgis-plugin/cassini_qgis/texts.py`, `panel.py`, `plugin.py` -- widgets et câblage.
- [x] `qgis-plugin/tests/test_external.py` -- une ligne de la matrice par test, noyau simulé.
- [x] `docs/canal-local.md`, `qgis-plugin/README.md` si l'extension y est décrite -- l'extension réelle.

**Acceptance Criteria:**
- Given une question ouverte, when le noyau la retire puis l'utilisateur clique Autoriser, then rien n'est écrit sur le canal et la saisie redevient active.
- Given un noyau relancé après un appel externe, when le panneau redevient prêt, then l'interrupteur est éteint et aucune ligne externe n'est « en cours ».
- Given la recette, when `python -m pytest qgis-plugin/tests` et `check_extension.py` tournent, then ils passent.

## Implementation Notes

- Refuser le plan est grisé aussi pendant un appel externe (le plan ne nommait que Valider) : le noyau répondrait `session.busy` à `plan.reject`, et le panneau oublierait un plan que le noyau garde.
- La 2e demande d'accord est tranchée dans le client (réponse `{approved:false}` à la 1re, puis `APPROVAL_WITHDRAWN` et `APPROVAL_ASKED`) ; le panneau ne voit jamais deux questions.
- Le libellé des lignes externes est recalculé à chaque rendu, pour suivre la langue.
- Vérification : `pytest qgis-plugin/tests` (319 verts, via `uv run --with pytest`, pytest n'étant pas installé dans le Python système), `check_extension.py` propre, pyflakes propre, et essai du panneau réel hors écran (PyQt6 sous un faux `qgis.PyQt`, dans le dossier temporaire de session) : interrupteur, case retenue, ligne et durée, question, retrait, mort, canal pris, Couper.

## Plan Change Log

## Review Triage Log

Relecture `thorough` (blind-hunter, edge-case-hunter, verification-gap, intent-alignment), 2026-10-31. Comptes : high 0, medium 4, low 17, false 4 ; rejetés low 13.

| # | Lentille | Constat | Verdict | Preuve | Suite |
|---|---|---|---|---|---|
| 1 | BH, ECH | Un `session.external` refusé (Couper, renvoi de `ask`) éteint l'interrupteur alors que le noyau garde le canal | medium | `refused` pose `enabled = False` quel que soit l'envoi | patch |
| 2 | BH, ECH | Case changée entre l'envoi d'allumage et la réponse : aucun renvoi, le noyau garde l'ancien `ask` | medium | `_ask_clicked` ne renvoie que si `enabled`, encore faux à ce moment | patch (renvoi si la réponse diffère) |
| 3 | BH | Paramètres de l'IA externe rendus en texte riche dans la question | medium | `QLabel` en `AutoText` ; `json.dumps` garde `<b>` | patch (`PlainText`) |
| 4 | VG, BH, IA | Les tests rejouent une copie du routage, pas `_on_external_event` | medium | classe `Panel` du test ; supprimer le routage du panneau ne casse rien | patch (`external.apply`) |
| 5 | VG | Lecture de la case retenue non testée | low | analyse de QSettings seulement dans `panel.py` | patch (`external.ask_setting`) |
| 6 | BH | Annonce de fin d'appel sans son issue | low | `_announce(self.log_list, entry.label)` | patch (`focus_status`) |
| 7 | BH | Chaque appel externe ferme le détail cliqué | low | `self.focus = None` sur `started` et `finished` | patch (suppression) |
| 8 | BH | Recette du README sans « Refuser » parmi les grisés | low | `agentview.controls` le grise | patch |
| 9 | BH | Ligne de docstring trop longue dans `agentview.controls` | low | correction directe | patch |
| 10 | VG | Mort ou relance : appel de `lost` par le panneau non testé | low | `_drop_stale_run` dans du code Qt que la recette ne teste pas ; recette manuelle étape 6 | rejeté |
| 11 | BH | Question sans limite de longueur | low | panneau défilant, boutons atteignables ; troncature = nouvelle logique | rejeté |
| 12 | BH | Couper sans annonce dans `lost` | low | bandeau caché, ajout d'un cas d'annonce | rejeté |
| 13 | BH | Couper pendant un appel ou une question, renvoi en vol non testés | low | comportement du noyau, testé en 5.24/5.29 ; renvoi couvert par le n° 2 | rejeté |
| 14 | BH | Appel refusé sans trace au journal | false | voulu : « un appel refusé n'apparaît ni au fil du panneau ni au journal » (5.29) | rejeté |
| 15 | BH | Durée sans minutes (« 300,0 s ») | low | lisible ; ajout d'un format | rejeté |
| 16 | BH | `render` réécrit `entry.label` | low | sert la traduction, sans effet de bord visible | rejeté |
| 17 | BH | `_external_approve` sans contrôle de type de `params` | false | `_handle_call` ramène `params` à `{}` s'il n'est pas un dict (client.py l.502) | rejeté |
| 18 | ECH | Deux `started` pour un même `call_id` | low | le noyau ne le fait jamais (compteur `external-<n>`) | rejeté |
| 19 | ECH | `finished` malformé : occupation bloquée | low | le noyau seul émet ; garde supplémentaire | rejeté |
| 20 | ECH | Saisie brièvement active entre Autoriser et `started` | low | le noyau répond `session.busy`, affiché comme aujourd'hui | rejeté |
| 21 | ECH | Détail d'une ligne externe en erreur : chemins non masqués | false | le noyau réduit les chemins de `external.call` (5.28, AD-9) | rejeté |
| 22 | ECH | Chemin absolu en clé de paramètre | low | improbable ; masquer les clés change `redact` partagé | rejeté |
| 23 | ECH | Seul l'appel le plus récent est arrêtable | false | le noyau sert un appel à la fois (occupation unique) | rejeté |
| 24 | IA | Arrêter dans le bandeau et non par ligne ; grisage plus large que la saisie | low | un seul appel à la fois : même effet ; grisage de Refuser justifié (Implementation Notes) | rejeté |
| 25 | IA | Surface QGIS réelle couverte seulement par la recette manuelle | low | convention du dépôt (panneau non testé sous Qt) ; recette README | rejeté |

## Design Notes

```json
{"jsonrpc":"2.0","id":"core-4","method":"external.approve","params":{"call_id":"external-3","command":"layer.describe","params":{"layer_id":"l1"}}}
{"jsonrpc":"2.0","id":"core-4","result":{"approved":false}}
{"jsonrpc":"2.0","method":"request.cancel","params":{"id":"core-4"}}
```

Le client ne décide rien : il rend les événements et répond quand le panneau le demande. Tant que la fenêtre Réglages (modale) est ouverte, la question n'est pas cliquable ; elle expire au délai du noyau, ce qui reste sûr (refus).

## Verification

**Commands:**
- `python -m pytest qgis-plugin/tests` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` -- expected: ok (aucun import PyQt5/6 direct)
