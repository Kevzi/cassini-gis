---
title: "Panneau : lancer une commande du registre, voir son resultat, l'arreter"
type: 'feature'
ticket: '4'
created: '2026-10-16'
status: 'built'
baseline_revision: '7ccad45776400052dec1f0648209336a7ace8700'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/canal-local.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/DESIGN.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le panneau du traceur n'affiche que les versions du noyau : on ne peut ni voir les commandes du registre, ni en lancer une, ni lire son resultat, ni l'arreter depuis QGIS.

**Approach:** livrer la part **[9.4]** de l'esquisse `EXPERIENCE.md` (decidee le 2026-10-16, option A) : en-tete avec logo, bandeau d'etat, saisie naturelle visible mais grisee, resultat, journal de session, mode expert replie (catalogue, formulaire par schema, Lancer/Arreter), Reglages (langue FR/EN ; modele grise). Le client expose `run_command`/`cancel_run` et le catalogue du `session.hello`.

## Boundaries & Constraints

**Always:** Qt uniquement via `qgis.PyQt` ; aucune logique metier (AD-4) : le noyau valide, le panneau ne valide rien ; une commande a la fois (AD-5) ; l'arret envoie `request.cancel` pour la requete `command.run` en vol, jamais la mort du processus (AD-12), et reste grise si la commande n'est pas `cancellable` ; jamais de code, d'expression ni de chemin absolu a l'ecran (AD-9) ; textes FR et EN du tableau de microcopy, langue memorisee dans QSettings ; logique hors widgets, testable sans QGIS ; comportements 9.1 a 9.3 et garde Python conserves.

**Never:** modifier `core/` ou `docs/canal-local.md` ; relire le journal du noyau ; saisie en langage naturel active, plan a valider, choix du modele, mode lecture seule (epic 5 / plus tard) ; reseau, PyQt5/PyQt6 directs, ressource distante ; lecture bloquante ; formulaire devine hors du sous-ensemble (champ JSON brut a la place).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Lancer | `system.about`, Lancer | resultat clair, ligne « Terminee » au journal | — |
| Arreter | `test.qgis_probe` en cours, Arreter | `request.cancel` envoye, ligne « Annulee », Lancer reapparait | — |
| Non annulable | commande `cancellable` faux | Arreter grise avec sa raison | — |
| Indisponible | `available` faux | ligne grisee, non lancable, raison | — |
| Erreur du noyau | parametres refuses | code `domaine.cause` + message FR, ligne « Erreur » | jamais d'exception vers Qt |
| JSON brut invalide | champ hors sous-ensemble | message local | rien n'est envoye |
| Noyau absent / mort / refuse | etat du client | bandeau : Choisir le noyau / Relancer ; panneau grise | refus de protocole : pas de Relancer |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/client.py` -- `CoreClient` : `_handle` (hello puis about), `_request`, `_finish(READY)` ; ajouter catalogue (resultat de hello, joint au payload READY), `run_command`, `cancel_run`, evenements de fin ; `_NoGestures`, `BUSY`, ordres 9.2 inchanges.
- `qgis-plugin/cassini_qgis/plugin.py` -- panneau minimal (label, bouton noyau, `SlowProbe`, `_on_event`) ; garde `SlowProbe` et le lancement ; deleguer l'interface a `panel.py`.
- `qgis-plugin/cassini_qgis/panel.py` -- NOUVEAU, Qt : widgets de l'esquisse.
- `qgis-plugin/cassini_qgis/form.py`, `texts.py`, `runlog.py` -- NOUVEAUX, sans Qt : schema -> champs et `params` ; textes FR/EN ; journal de session.
- `qgis-plugin/cassini_qgis/assets/` -- NOUVEAU : logos SVG copies de `ux-panneau-cassini/assets/`.
- `qgis-plugin/tests/test_tracer.py` -- `FakeProcess`, `reply()` a reutiliser ; `test_form.py`, `test_texts.py`, `test_runlog.py` NOUVEAUX ; `check_extension.py` doit rester vert.
- `core/src/serve.rs` (~l.585) -- forme du catalogue ; lecture seule.

## Tasks & Acceptance

**Execution:**
- [ ] `client.py` -- catalogue, `run_command`, `cancel_run`, evenements -- socle du panneau
- [ ] `form.py`, `texts.py`, `runlog.py` -- logique pure : champs par schema, microcopy FR/EN, journal (statut, libelle, heure) -- testable sans QGIS
- [ ] `panel.py`, `plugin.py`, `assets/` -- widgets de l'esquisse, noms accessibles, clavier, bandeau, Reglages langue -- interface [9.4]
- [ ] `tests/` -- matrice complete, catalogue, annulation, textes FR et EN de meme cles
- [ ] `README.md` -- recette manuelle : `system.about`, puis `test.qgis_probe` lance et arrete

**Acceptance Criteria:**
- Given un projet ouvert, when le noyau repond a `session.hello`, then le mode expert liste les commandes du catalogue et grise les indisponibles.
- Given `test.qgis_probe` lancee, when Arreter est clique, then `request.cancel` part vers le noyau, le journal note « Annulee » et QGIS reste reactif.
- Given `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py`, then tout passe sans QGIS.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch (fait) | `stop()` remet `_run` a None sans evenement RUN_* ; le panneau gardait `running`. Corrige par `_drop_stale_run` dans `set_banner`/`set_ready`. |
| 2 | low | defer | Bouton local d'arret des gestes 9.1 retire ; l'arret passe par request.cancel. Intention ambigue. |
| 3 | low | rejete | Refus de request.cancel ignore ; Arreter deja grise par le catalogue, correctif ajouterait des branches. |
| 4 | low | rejete | « Etape 1 » et code affiches : cosmetique, code `domaine.cause` voulu par la matrice. |
| 5 | maybe-false | defer | redact ne couvre pas un chemin dans une phrase d'erreur du noyau (AD-9) ; a verifier que le noyau n'en emet pas. |
| 6 | low | rejete | Lignes indisponibles non selectionnables ; raison presente dans le libelle (critere de la matrice atteint). |
| 7 | low | rejete | Logo choisi a la construction, pas de changement de theme a chaud. |
| 8 | low | rejete | Tests regroupes dans un fichier : ecart de forme sans effet. |

## Design Notes

`command.run` bloque le noyau jusqu'a sa fin, pas le client : `run_command` envoie et rend la main, la reponse arrive par le signal de lecture. `request.cancel` est une requete separee visant l'id du `command.run` en vol ; le noyau repond a l'ordre apres la notification a l'extension (docs/canal-local.md). Contraste : le sarcelle clair ne porte jamais de texte (DESIGN.md).

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: OK
- `python -I qgis-plugin/tests/check_extension.py` -- expected: code 0

**Manual checks (if no CLI):**
- Dans QGIS : logo et saisie grisee visibles ; lancer `system.about` ; lancer puis arreter `test.qgis_probe` ; basculer FR/EN ; noyau absent affiche « Choisir le noyau ».
