---
title: 'Rejeu en session en lecture seule'
type: 'feature'
ticket: ''
created: '2026-11-09'
status: 'built'
baseline_revision: '9bd7eaeed7d00ea80eb9f755854107be9c1566d6'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, adversarial, edge-cases]
review_loop_iteration: 1
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-refonte-du-panneau/story-rejouer-le-journal-et-mode-expert-dans-le-fil-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** quand le même projet est ouvert dans une seconde fenêtre QGIS, le noyau répond `read_only: true` à `session.hello` ; l'extension (`client.py`, ticket 5.32) le traite comme un échec, arrête le noyau et montre le bandeau « refusé ». La spine UX veut au contraire que cette session reste ouverte, dise qu'elle est en lecture seule, et ne laisse que « Rejouer le journal » (AD-11). Le noyau, lui, sert déjà `journal.replay` dans une session en lecture seule (`serve.rs` `journal_replay`, qui ne passe pas par l'écriture du journal).

**Approach:** l'extension garde le noyau vivant en lecture seule : le client devient prêt avec un indicateur de lecture seule, le greffon affiche le bandeau de lecture seule de la spine, et le panneau grise tout sauf « Rejouer le journal », avec la raison de chaque contrôle grisé. Décision de Claude (absence autorisée du propriétaire, 2026-11-09), notée pour relecture.

## Boundaries & Constraints

**Always:** texte du bandeau et des raisons de `EXPERIENCE.md` (« Ce projet est ouvert dans une autre fenêtre QGIS. Ici, vous pouvez seulement rejouer. », état « Session en lecture seule ») ; en lecture seule, aucune commande, aucun plan, aucune IA externe ne part (Envoyer, Lancer, saisie, modèle, interrupteur de l'IA externe grisés) ; « Rejouer le journal » actif ; l'historique du journal est affiché comme d'habitude ; une session normale ne change pas ; logique dans `thread.py`/`agentview.py`, testée sans Qt ; le noyau, le protocole et le catalogue ne changent pas.

**Never:** écrire au journal depuis la session en lecture seule ; changer le noyau ; contourner le verrou du journal.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Seconde fenêtre | `session.hello` → `read_only: true` | bandeau de lecture seule, noyau vivant, seul Rejouer actif | — |
| Rejeu en lecture seule | clic sur Rejouer | verdict du rejeu dans le repli Journal | erreur : message du noyau |
| Tentative d'envoi | Envoyer, Lancer | grisés avec la raison de lecture seule | — |
| Session normale | `read_only: false` | comportement inchangé | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/client.py` l. ~495-515 -- `read_only` à la poignée de main → `_fail(message)` aujourd'hui ; garder la session prête avec un indicateur `read_only` (événement READY avec ce champ).
- `qgis-plugin/cassini_qgis/plugin.py` `_on_event` (READY, FAILED) -- bandeau de lecture seule au lieu de « refusé ».
- `qgis-plugin/cassini_qgis/panel.py` -- bandeau (`set_banner`), contrôles grisés (`agentview.controls`), bouton Rejouer de 12.3 (`thread.replay_button`).
- `qgis-plugin/cassini_qgis/thread.py`, `agentview.py`, `texts.py` -- logique et textes.
- `qgis-plugin/tests/test_plan_client.py`, `test_tracer.py`, `test_thread.py`, `test_panel_logic.py` -- dont le test de 12.3 `test_not_ready_greys_without_reason` à adapter.

## Tasks & Acceptance

**Execution:**
- [x] client, greffon, panneau, textes ; tests de la matrice.
- [x] Sonde sous QGIS 3.44 : deux noyaux sur le même projet (le second en lecture seule), rejeu depuis le second.

**Acceptance Criteria:**
- Given un projet déjà ouvert par une première session, when une seconde session s'ouvre, then elle reste connectée, dit qu'elle est en lecture seule, et seul Rejouer y est actif.
- Given le dépôt, when on lance les tests et la garde de l'extension, then tout passe.

## Implementation Notes

- Client : `read_only: true` à `session.hello` ne fait plus `_fail` ; le client devient prêt, garde `_read_only` (`read_only()`), et READY porte toujours `read_only`. En lecture seule, seul `journal.replay` part : `command.run`, `plan.*`, `agent.models` et `session.external` sont refusés côté client (`_can_write`), en plus du panneau grisé. Le message du noyau n'est plus montré : le bandeau prend le texte de la spine.
- Greffon : READY avec `read_only` → `panel.set_ready(..., read_only=True)` et pas de liste des modèles (le modèle est grisé). FAILED ne sert plus qu'au refus de protocole.
- Panneau : bandeau `read_only` avec « Relancer » (reprendre l'écriture une fois la première fenêtre fermée) ; `read_only` est dérivé (`ready` et bandeau `read_only`) : tout ce qui teste `ready and banner is None` reste grisé sans changement. Raison « Session en lecture seule : seul « Rejouer le journal » est possible ici. » (formulation ajoutée, la spine ne donne que le bandeau) sous la saisie, sur Envoyer, Lancer, le modèle et l'interrupteur de l'IA externe ; le repli Mode expert s'ouvre encore pour montrer la raison de Lancer ; pas de « À vous » ni de Reformuler.
- Logique pure : `agentview.controls(read_only=)` rend aussi `expert_enabled` (repli ouvrable en lecture seule), `run_reason`, `model_reason`, `external_reason` ; le panneau ne fait qu'appliquer. `thread.replay_button` exige un noyau prêt (le panneau passe `self.ready`, lecture seule comprise) ; `thread.shown_guidance(read_only=)`. Le test 12.3 `test_not_ready_greys_without_reason` garde son cas (noyau absent) ; le cas lecture seule est un test à part.
- Docs : `docs/canal-local.md` et `qgis-plugin/README.md` (recette manuelle 6) décrivent le nouveau comportement. Sonde ajoutée : `qgis-plugin/tests/probe_read_only.py`.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — medium 2, low 5 ; corrigés 6, constat de règle 1 (risque relevé à `high`, relecture `thorough` : CLAUDE.md §8, rejeu et canal).

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Risque `medium` et relecture `quick` pour un changement de canal et de rejeu | medium | patch | risque `high`, relecture `thorough` (boucle 1) |
| Décisions de lecture seule prises dans `panel.py` | medium | patch | `agentview.controls(read_only=…)` rend drapeaux et raisons, testés sans Qt |
| Contrat du repli Mode expert contraire au panneau | low | patch | contrat aligné, test corrigé |
| `replay_button` actif sans noyau prêt | low | patch | noyau prêt exigé |
| Aucune sortie de la lecture seule | low | patch | action « Relancer » sur le bandeau, docs |
| Docstrings de `client.py` et `set_banner` inexactes | low | patch | corrigées |

Boucle 1 (thorough : angle adverse et cas limites, faite par Claude en direct, le lancement des relecteurs étant bloqué par un greffon) — aucun constat bloquant. Le noyau refuse toute écriture au journal en lecture seule (`journal.read_only`, `core/src/journal/mod.rs`), y compris venant d'un relais externe ; les gardes du client ne sont qu'ergonomiques. `request.cancel` et `answer_approval` ne sont pas gardés mais sans effet possible (rien n'est en vol en lecture seule). Relancer rouvre en lecture seule tant que la première fenêtre tient le verrou, puis en écriture.

## Verification

**Fait (2026-11-09)** : 634 tests verts, garde propre ; sonde `probe_read_only.py` sous QGIS 3.44.15 (`QT_QPA_PLATFORM=windows`, `GGML_VK_VISIBLE_DEVICES=aucune`, noyau installé `%LOCALAPPDATA%\Cassini\bin\cassini.exe`, sans modèle) : 18/18 — première session `layer.add`, seconde ouverte pendant que la première vit, prête en lecture seule, noyau vivant, commandes refusées, vrai panneau avec bandeau de la spine, saisie, Envoyer, Lancer, modèle et IA externe grisés avec la raison, Rejouer actif, rejeu `{"steps": 1, "identical": true}`, « résultat identique » dans le repli, journal inchangé.

**Commands:**
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python3 -I qgis-plugin/tests/check_extension.py` -- expected: propre
