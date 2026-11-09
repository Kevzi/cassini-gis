---
title: 'Rejouer le journal et mode expert dans le fil'
type: 'feature'
ticket: '3'
created: '2026-11-09'
status: 'built'
baseline_revision: '8c69b8aabba1b452c7752aa76d2d81b8584982cd'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/DESIGN.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-refonte-du-panneau/story-guidage-a-vous-repere-d-avancement-et-reformuler-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la spine UX place dans le repli Journal un bouton « Rejouer le journal » (seule action laissée en session en lecture seule, AD-11), mais le panneau n'en a pas : le rejeu n'existe que par le client (`client.replay`) et les sondes. La commande experte dans le fil, prévue ici, est déjà livrée par 12.1.

**Approach:** ajouter le bouton « Rejouer le journal » dans le repli Journal, branché sur `client.replay` par le greffon, avec les états de `EXPERIENCE.md` (rejeu en cours, identique, différent, impossible) affichés dans le repli ; vérifier que la commande experte dans le fil de 12.1 tient (bulle « Mode expert : … » puis carte de résultat) et combler ce qui manquerait.

## Boundaries & Constraints

**Always:** textes FR/EN de `EXPERIENCE.md` (Voice and Tone, State Patterns : « Rejeu terminé : résultat identique. », « Rejeu terminé : résultat différent à l'étape k. » avec la raison, rejeu impossible avec le message du noyau) ; le rejeu passe par le noyau (`journal.replay`), l'extension ne rejoue rien elle-même (AD-4, AD-11) ; pendant le rejeu, Envoyer, Lancer et la saisie sont grisés avec leur raison, et le bouton du rejeu aussi ; le rejeu n'est pas annulable côté client aujourd'hui : pas de bouton Arrêter pour lui ; en session en lecture seule, le bouton reste actif et tout le reste grisé ; journal vide : bouton grisé avec sa raison ; logique (texte de l'état, bouton actif ou non) dans `thread.py`/`agentview.py`, testée sans Qt ; accessibilité : bouton au clavier, résultat annoncé.

**Never:** pas de changement du noyau ni du protocole (le rejeu n'est pas rendu annulable) ; pas de progression « étape k sur n » si le noyau n'envoie pas d'événement d'étape pendant le rejeu (dire « Rejeu en cours… » seulement) ; pas de changement du modèle.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Rejeu identique | journal de 2 étapes | « Rejeu terminé : résultat identique. » dans le repli, annoncé | — |
| Rejeu différent | noyau rend `replay.diverged` à l'étape 2 | « Rejeu terminé : résultat différent à l'étape 2. » + message | — |
| Rejeu impossible | autre erreur du noyau | message du noyau, code sous Détails techniques | — |
| Rejeu en cours | après le clic | « Rejeu en cours… » ; bouton, Envoyer, Lancer, saisie grisés | — |
| Journal vide | aucune étape | bouton grisé, raison écrite | — |
| Lecture seule | seconde fenêtre QGIS | seul le bouton du rejeu actif | — |
| Commande experte | Lancer `system.about` | bulle « Mode expert : … » puis carte de résultat | erreur : carte d'erreur sans Reformuler |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/client.py` l. 213-226 (`replay_busy`, `replay`), l. 547 (`_replay_answer` → `REPLAY_DONE {steps, identical}` / `REPLAY_FAILED {code, message, step}`), l. 37-38 -- à brancher, sans les changer.
- `qgis-plugin/cassini_qgis/plugin.py` l. 96-130 (actions du panneau : `run`, `propose`, `approve`…) et `_on_event` l. 182-206 -- ajouter `replay()` et transmettre `REPLAY_DONE`/`REPLAY_FAILED` au panneau.
- `qgis-plugin/cassini_qgis/panel.py` -- repli Journal de 12.1 (`log_fold`, `log_list`), contrôles grisés (`agentview.controls`, raisons), annonces (`_announce`), ordre de tabulation.
- `qgis-plugin/cassini_qgis/thread.py`, `agentview.py` -- logique pure de 12.1/12.2 ; y mettre l'état du rejeu.
- `qgis-plugin/cassini_qgis/texts.py` -- libellés FR/EN.
- `qgis-plugin/tests/test_thread.py`, `test_panel_logic.py`, `test_tracer.py` -- tests ; `test_tracer.py` simule le panneau.
- `C:\Users\kevin\AppData\Local\Temp\capture-panneau\capture.py` -- captures ; ajouter un passage : une phrase réussie, puis clic sur Rejouer, capture du repli Journal ouvert avec « résultat identique » ; une commande experte (`system.about`) dans le fil.

## Tasks & Acceptance

**Execution:**
- [x] `thread.py`/`agentview.py` + tests -- état du rejeu et contrôles.
- [x] `plugin.py`, `panel.py`, `texts.py` -- bouton, branchement, affichage, annonces.
- [x] Vérifier la commande experte dans le fil (test et capture), combler un manque éventuel.
- [x] Captures sous QGIS 3.44 (projet Annecy copié puis archivé) et regardées.

**Acceptance Criteria:**
- Given QGIS 3.44 et le projet Annecy, when une phrase réussit puis qu'on clique Rejouer le journal, then le repli Journal affiche « Rejeu terminé : résultat identique. » (capture).
- Given le dépôt, when on lance les tests et la garde de l'extension, then tout passe.

## Implementation Notes

- Logique pure dans `thread.py` (`replay_started`, `replay_outcome`, `replay_text`, `replay_line`, `replay_details`, `replay_button`) ; `agentview.py` inchangé : le panneau passe le rejeu en vol comme une occupation (`_busy()`) à `agentview.controls` et à `thread.input_hint`, ce qui grise saisie et Envoyer avec « Patientez : une étape est en cours. » ; Lancer grisé avec « Patientez : le rejeu est en cours. », sans Arrêter.
- Issues : `replay.diverged` → « Rejeu terminé : résultat différent à l'étape k. {message} » ; autre erreur avec étape → « Rejeu arrêté à l'étape k : {message} » ; sans étape → « Rejeu impossible : {message} » (formulation ajoutée) ; code sous un repli Détails techniques dans le repli Journal. Pendant le rejeu : « Rejeu en cours… » seulement.
- Journal vide : jugé sur les lignes du Journal de session (le panneau ne relit pas le journal du noyau) ; une ligne refusée avant exécution peut donc activer le bouton, et le noyau répond alors `replay.no_journal`, affiché « Rejeu impossible : … ».
- Le verdict du rejeu ne touche ni le fil, ni le détail cliqué ; le focus revient sur le bouton du rejeu s'il y était au départ (Qt le déplace quand le bouton se grise).
- Comblé côté commande experte : la ligne du Journal de session porte le libellé court (« Vérification du noyau Cassini ») au lieu de l'identifiant.
- **Non atteint : lecture seule.** Le client (5.32) arrête le noyau quand `session.hello` répond `read_only: true` et le greffon affiche le bandeau `refused` ; sans noyau, aucun rejeu ne peut partir. Tenir la ligne « Lecture seule » de la matrice demande de changer `client.py` et `plugin.py` (garder le noyau en lecture seule, nouveau bandeau), ce que la Code Map exclut : à trancher par le propriétaire.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — medium 4, low 5 ; corrigés 7, reportés 2 (lecture seule). Choix faits seul par Claude (absence autorisée du propriétaire).

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Ligne « Lecture seule » de la matrice non tenue | medium | report | ticket à part `plan-rejeu-en-lecture-seule.md` (le noyau sert déjà le rejeu en lecture seule) |
| Test qui fige le bouton grisé sans noyau prêt | low | report | adapté dans le ticket de lecture seule |
| Le verdict ramène le focus de force | medium | patch | retour seulement si le focus n'a pas bougé, testé |
| Clic refusé par le client sans retour | medium | patch | raison affichée et annoncée |
| Reformuler actif pendant un rejeu | medium | patch | grisé, « À vous » masqué |
| Vieux verdict resté après un ajout au journal ou une relance | low | patch | verdict oublié, testé |
| Numéro d'étape répété | low | patch | texte court quand le message le nomme |
| « Rejeu impossible : » vide | low | patch | texte de repli |
| Test « rejeu en cours » sans le panneau | low | patch | test côté panneau |

## Verification

**Fait (2026-11-09)** : 615 tests verts, garde propre ; capture QGIS 3.44.15, projet Annecy copié puis archivé dans `_archives/sondes/capture-panneau-12-3/`, noyau sur processeur (`GGML_VK_VISIBLE_DEVICES=aucune`) : `system.about` en mode expert (bulle puis carte de résultat), phrase des pentes réussie, Rejouer le journal → `{"steps": 3, "identical": true}`, repli Journal « ✔ Rejeu terminé : résultat identique. » (clair et sombre). Captures `%TEMP%\capture-panneau\340-rejeu-*.png`, lancées par `lancer-12-3.bat`.

**Commands:**
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python3 -I qgis-plugin/tests/check_extension.py` -- expected: propre

**Manual checks (if no CLI):**
- Copier l'extension dans le profil QGIS, copier le projet Annecy en `annecy/projet-capture-panneau.qgz`, lancer `capture.py 340` sous le Python de QGIS (`QT_QPA_PLATFORM=windows`), regarder les PNG, archiver le projet dans `_archives/sondes/`. La carte graphique est prise par l'entraînement : lancer la composition sur processeur (modèle plus lent) ou utiliser une commande experte pour remplir le journal.
