---
title: 'La sonde de démo rejoue le journal et vérifie la mise en page'
type: 'feature'
ticket: '1'
created: '2026-11-13'
status: 'built'
baseline_revision: '443de8b3df4ef595b5e3b6dabb408506a1618d64'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-export-et-demo/story-la-sonde-de-demo-juge-la-phrase-de-demo-sur-des-fixtures-syn-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la sonde de démo (`qgis-plugin/tests/probe_demo.py`) juge le plan et ses parcelles mais ne prouve ni que le journal se rejoue à l'identique après réouverture du projet, ni que la mise en page exportée porte ses cinq éléments : c'est la moitié du ticket 6.1 fusionné avec 6.2 et 6.3.

**Approach:** après un verdict de plan OK, la sonde ferme le projet, le rouvre, lance un nouveau noyau sans modèle et rejoue le journal en comparant chaque étape ; elle contrôle aussi, depuis ce que le geste d'export déclare avoir posé, titre, légende, échelle, flèche du nord et sources, et que le PDF n'est pas vide. Le jugement vit dans `demo_verdict.py`, pur et testé sans QGIS.

## Boundaries & Constraints

**Always:** aucune modification de l'extension (`cassini_qgis/`) ni du noyau ; jugement dans `demo_verdict.py` (aucun import qgis/Qt), faits relevés par la sonde ; `VERDICT : OK` ou `VERDICT : ECHEC — <raison>`, code 1 en cas d'échec, la raison nomme l'étape ou l'élément fautif ; projet fermé puis rouvert (`project.clear()`, `project.read`) avant le rejeu, nouveau processus noyau lancé sans `CASSINI_MODELS` ; les fichiers d'origine (fixtures, projet donné en argument) jamais modifiés.

**Never:** le projet réel d'Annecy altéré ; ajouter la mise en page au gestionnaire du projet ; extraire le texte du PDF ; nouvelle dépendance ; MNT (entrée 4) ; projet réel passé en argument comme preuve (entrée 6).

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|---------------|----------------------|------------------|
| Plan juste, rejeu identique | pas d'argument | plan jugé OK, projet rouvert, journal rejoué, `identical` et nombre d'étapes égal aux étapes exécutées, cinq éléments, PDF `%PDF-` non vide | `VERDICT : OK`, code 0 |
| Journal altéré | `CASSINI_DEMO_TAMPER=1`, projet de démo | le `result` d'une étape d'analyse est changé avant le rejeu | `ECHEC — rejeu : étape k <commande> diverge`, code 1 |
| Légende perdue | `CASSINI_DEMO_DROP_ITEM=legend` | l'élément est retiré de la liste relevée | `ECHEC — mise en page : légende absente`, code 1 |
| Altération sur projet donné | `CASSINI_DEMO_TAMPER=1` avec un argument | aucune altération | `ECHEC — altération refusée sur un projet donné`, code 1 |
| Rejeu en échec | `REPLAY_FAILED` | | `ECHEC — rejeu : <code> étape k` |

</frozen-after-approval>

## Code Map

- `qgis-plugin/tests/probe_demo.py` -- `on_event`, branche PLAN_DONE (l.320-337) : au lieu de `finish(judge_done(...))` direct, enchaîner `core.stop()`, `project.clear()`, `project.read(PROJECT)`, nouveau `QProcess` + `CoreClient` (même registre), `.replay()` sur READY ; gérer `client.REPLAY_DONE` / `REPLAY_FAILED`. Modèle à copier : `probe_end_to_end.py:208-282` (`on_second`, `tamper`). `Timed` enveloppe déjà les gestes : lire `inner.items_placed` du geste d'export.
- `qgis-plugin/tests/demo_verdict.py` -- ajouter `judge_replay(kind, payload, executed)` et `judge_layout(items, facts)` ; même convention (raison ou `None`).
- `qgis-plugin/tests/test_demo_verdict.py` -- un test par raison d'ECHEC, style `JudgeDoneTest`.
- `cassini_qgis/layout_export.py` -- NE PAS MODIFIER : `LayoutExportGesture.items_placed` (l.1187), `EXPECTED_ITEMS` (l.375) ; la mise en page est temporaire (jamais au gestionnaire).
- Journal : `<projet>.cassini/journal.jsonl` ; le rejeu écrit sous `<projet>.cassini/replay/` ; `replay.diverged` porte `step` (rang à partir de 1).

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/tests/demo_verdict.py` -- `judge_replay`, `judge_layout` -- jugement pur, raisons nommant étape ou élément
- [ ] `qgis-plugin/tests/test_demo_verdict.py` -- tests des deux fonctions, une raison d'ECHEC par test -- couvre la matrice
- [ ] `qgis-plugin/tests/probe_demo.py` -- rejeu après réouverture, relevé des éléments, `CASSINI_DEMO_TAMPER` et `CASSINI_DEMO_DROP_ITEM`, docstring à jour -- le verdict final

**Acceptance Criteria:**
- Given le projet de démo, when la sonde tourne sous QGIS avec le vrai modèle, then elle rend `VERDICT : OK` après un rejeu identique étape par étape.
- Given `CASSINI_DEMO_TAMPER=1`, when la sonde rejoue, then elle rend ECHEC en nommant l'étape qui diverge.
- Given `CASSINI_DEMO_DROP_ITEM=legend`, when la sonde juge la mise en page, then elle rend ECHEC nommant la légende.

## Implementation Notes

Essais réels du 2026-11-13 (QGIS 3.44.15, noyau gpu local, modèle installé) : nominal `VERDICT : OK` (rejeu 5/5 identique) ; `CASSINI_DEMO_TAMPER=1` ECHEC nommant l'étape 1 ; `CASSINI_DEMO_DROP_ITEM=legend` ECHEC « légende absente ». Les `items_placed` relevés : map, title, legend, scalebar, northarrow, sources.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : medium 1, low 1, false 2, rejet 1.

- `CASSINI_DEMO_DROP_ITEM` jamais validé (une faute de frappe ou `map` rend OK) -- medium, patch : `drop_item_refused_reason`, la sonde refuse l'élément inconnu au démarrage (vérifié : `legende` donne ECHEC), test ajouté.
- Branches de `demo_verdict` sans test (nombre de pages, événement inattendu, `step` absent ou hors plage, charge non dict) -- low, patch : tests ajoutés.
- `PLAN_FAILED` / `MODELS_FAILED` non traités pendant le rejeu -- low, rejeté : le délai de 30 minutes de la sonde couvre le cas, la correction ajouterait des branches pour un cas improbable.
- L'étape altérée n'est pas confrontée à l'étape nommée par le rejeu -- false : essai réel avec `CASSINI_DEMO_TAMPER=1`, journal altéré à l'étape 1 `analysis.select_by_attribute`, verdict « rejeu : étape 1 analysis.select_by_attribute diverge ».
- Étapes du journal et étapes exécutées peut-être non égales -- false : essai réel nominal, `replay_done {"steps": 5, "identical": true}` pour 5 étapes exécutées.

## Design Notes

Constat du propriétaire après coup (2026-11-13) : le PDF de la sonde avait tout son texte en cases noires. Cause : la sonde forçait `QT_QPA_PLATFORM=offscreen`, qui ne charge aucune police (0 famille, contre 154 sous la plateforme `windows`) ; le PDF n'a alors ni police ni texte, seulement des contours vides. Le produit n'était pas en cause. `%PDF-` et la taille ne suffisaient pas à le voir. Corrigé : plateforme `windows` par défaut sous Windows, et ECHEC « aucune police dans QGIS » si aucune police n'est chargée. Le PDF rendu a été regardé : titre, carte, flèche, légende, échelle et sources lisibles.

La relecture « vraie » de la mise en page est impossible sans toucher à l'extension : la mise en page n'est jamais ajoutée au gestionnaire du projet. On juge donc ce que le geste d'export déclare avoir posé (`items_placed`), plus les champs du résultat (`sources`, `page_count`) et le PDF. `CASSINI_DEMO_DROP_ITEM` simule la perte d'un élément sur cette liste : il prouve le chemin de jugement, pas l'extension.

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: OK
- `python -I qgis-plugin/tests/check_extension.py` -- expected: propre

**Manual checks (if no CLI):**
- Sur le poste du propriétaire : `probe_demo.py` sous le Python de QGIS avec le noyau gpu et le modèle installé : `VERDICT : OK` ; puis avec `CASSINI_DEMO_TAMPER=1` et avec `CASSINI_DEMO_DROP_ITEM=legend` : ECHEC nommant l'étape, puis la légende.
