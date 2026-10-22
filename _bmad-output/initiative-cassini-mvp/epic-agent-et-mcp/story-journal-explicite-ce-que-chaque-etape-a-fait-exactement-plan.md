---
title: 'Journal explicite : ce que chaque etape a fait exactement'
type: 'feature'
ticket: '13'
created: '2026-10-22'
status: 'built'
baseline_revision: '846a5744723cca6187a191b73b31d91dfa03d2f3'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md', '{project-root}/docs/canal-local.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le journal et le résultat du panneau sont génériques (« La commande analysis.buffer est terminée. », « Plan exécuté : 3 étapes terminées. ») : l'utilisateur ne voit pas ce que chaque étape a fait. Demande du propriétaire (2026-10-18).

**Approach:** Une phrase claire par commande, calculée dans l'extension depuis les paramètres et le résultat déjà rendus (couche source, paramètres, couche produite et nombre d'entités, fichier écrit), avec repli sur la description du registre. Cliquer sur une ligne du journal ou sur une étape du plan affiche cette phrase dans la zone Résultat, les paramètres et le résultat bruts (masqués par `runlog.redact`) dans Détails techniques. Vaut pour les étapes d'un plan et les commandes du mode expert.

## Boundaries & Constraints

**Always:** Décisions du 2026-10-18 : la phrase vit dans l'extension (affichage seul), le noyau ne change pas ; le détail s'affiche dans la zone Résultat. Aucune logique métier (AD-4, AD-5) : la phrase ne fait que lire. Aucun chemin absolu ni code à l'écran (AD-9) : un identifiant de couche s'affiche par le nom de la couche dans QGIS (repli : nom rendu par une étape, puis l'identifiant) ; un fichier par son seul nom. `step:k` / `step:k.champ` → nom rendu par l'étape k (`name`), sinon « le résultat de l'étape k ». FR/EN mêmes clés, accord singulier/pluriel. Logique pure dans un module sans Qt testé par `unittest` ; Qt par `qgis.PyQt` seulement.

**Never:** Toucher `core/`, `cli/`, `ai/` ou le protocole. Relire le journal du noyau. Nouvelle dépendance. Sélecteur de modèle (5.14).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Étape de plan finie | `analysis.buffer {layer_id: gares, distance 500 m}` → `{name: Tampon, feature_count: 8}` | « Zone tampon de 500 m autour de Gares : nouvelle couche Tampon, 8 entités. » | — |
| Référence | `analysis.intersect {layer_id: parcelles, overlay_layer_id: "step:1.layer_id"}` | couche de l'étape 1 nommée « Tampon » | étape k sans résultat : « le résultat de l'étape k » |
| Export | `layout.export_pdf` → `{file: "…/2-layout.export_pdf.pdf", page_count: 1}` | « Carte « … » exportée en PDF : fichier 2-layout.export_pdf.pdf, 1 page. » | chemin absolu jamais montré |
| Plan en erreur | étapes 1-2 finies, 3 en erreur | clic sur 1 ou 2 : phrase depuis le résultat rendu par l'extension pour leur ordre ; clic sur 3 : message d'erreur de l'étape | étape du noyau sans ordre : phrase sans résultat |
| Commande inconnue du gabarit | id hors table | description du registre, puis Terminée | — |
| Étape en attente ou annulée | clic | libellé et statut, sans résultat | — |
| Mode expert | Lancer `analysis.buffer` | même phrase dans le résultat et au clic sur sa ligne | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/summary.py` (nouveau, sans Qt) -- `sentence(command, params, result, lang, layer_name, step_names)` : une phrase par commande du registre — `analysis.buffer` (distance `{value, unit}`, `dissolve`), `analysis.intersect` (`overlay_layer_id`), `analysis.select_by_attribute` (`field`, `operator` eq/ne/lt/lte/gt/gte/contains/begins_with, `value`), `analysis.isochrone` (`mode` walk/car, `duration {value, unit s|min}`), `analysis.zonal_statistics` (`points_layer_id`, `zones`), ces cinq rendent `{layer_id, name, feature_count, crs}` ; `layer.add {path}` → `{name, source_crs, project_crs, reprojected}` ; `layer.describe` → `{name, fields, feature_count, geometry}` ; `layout.export_pdf {title}` → `{file, page_count}` ; `project.inventory` → `{layers}` ; `model.list` → `{models}` ; `system.about` → texte `about_result`. Résultat absent ou illisible : phrase des seuls paramètres ; commande inconnue : `agentview.describe`. `layer_name(id)` rend le nom QGIS ou None ; `step_names` : {k: résultat de l'étape k}.
- `qgis-plugin/cassini_qgis/texts.py` -- une clé FR/EN par phrase, opérateurs, modes, `entities` singulier/pluriel ; le test d'égalité des clés existe.
- `qgis-plugin/cassini_qgis/runlog.py` -- `Entry` gagne `params`, `result`, `step` (None hors plan) ; `start(...)` les accepte en option.
- `qgis-plugin/cassini_qgis/agentview.py` -- `Step` gagne `result` ; `done(results)` le remplit depuis `steps[].result` ; `exec_failed`/`cancelled` prennent `outputs` {step: result} ; `step_results()` rend {k: result} pour `step_names`.
- `qgis-plugin/cassini_qgis/client.py` -- pendant un `plan.execute`, `plan.step` mémorise l'étape courante dans `self._plan` ; `_settle` réussi range le résultat de l'ordre sous cette étape ; `PLAN_EXEC_FAILED` et `PLAN_CANCELLED` portent `outputs` (`_plan_answer` l.360, `_fail_plan` l.402, `_handle_call` l.440). Rien ne change sur le fil.
- `qgis-plugin/cassini_qgis/panel.py` -- `_run` (l.807) garde les paramètres envoyés pour l'entrée RUN_STARTED (l.505), RUN_DONE range le résultat ; PLAN_STEP (l.471) crée l'entrée avec `step` et les paramètres du plan, verdicts rangent les résultats ; `log_list` et `plan_list` : `itemClicked` → `self.focus` (entrée ou étape), effacé par tout nouvel événement ; `_render_result` (l.597) affiche la phrase du focus, sinon du dernier résultat (remplace `done_generic` pour une commande experte ; `plan_done` reste, suivi de la phrase de chaque étape) ; Détails techniques = `{params, result}` masqués. `layer_name` = `QgsProject.instance().mapLayer(id).name()` protégé.
- `qgis-plugin/tests/test_plan_client.py`, `test_agent_view.py`, `test_panel_logic.py` -- motifs `FakeProcess`, import à plat sans Qt, à réutiliser.

## Tasks & Acceptance

**Execution:**
- [x] `qgis-plugin/cassini_qgis/summary.py`, `texts.py` + `qgis-plugin/tests/test_summary.py` -- phrases par commande, références, repli, FR/EN, sans chemin absolu -- lignes 1, 2, 3, 5 de la matrice
- [x] `qgis-plugin/cassini_qgis/client.py` + `qgis-plugin/tests/test_plan_client.py` -- `outputs` des étapes finies sur erreur et annulation -- ligne 4
- [x] `qgis-plugin/cassini_qgis/runlog.py`, `agentview.py` + tests -- résultat porté par l'entrée et l'étape -- lignes 4, 6
- [x] `qgis-plugin/cassini_qgis/panel.py` -- clic, focus, zone Résultat, détails -- l'interface
- [x] `qgis-plugin/README.md` -- recette manuelle du clic sur le journal et le plan -- hitl

**Acceptance Criteria:**
- Given QGIS 3.44 et la phrase de démo validée, when le propriétaire clique sur chaque ligne du journal, puis sur une étape du plan, puis sur une commande lancée en mode expert, then une phrase comme « Zone tampon de 500 m autour de Gares : nouvelle couche Tampon, 8 entités. » s'affiche, sans code ni chemin absolu, en français et en anglais.
- Given la suite sans QGIS, when `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` tournent, then tout passe.

## Implementation Notes

- `runlog.Entry` porte aussi `refs` ({k: résultat de l'étape k}, dict partagé par les lignes d'un même plan, pour nommer les références `step:k` au clic même après un nouveau plan) et `error` ((code, message) d'une ligne en erreur, pour le clic sur une ligne fautive). `runlog.step_details(params, result)` rend les Détails techniques masqués.
- `summary.sentence` prend en plus `catalogue` (mot-clé) pour le repli sur la description du registre ; des paramètres illisibles pour une commande connue se replient aussi sur cette description. Les textes utilisateurs (titre, valeur de filtre) passent par `runlog.redact`.
- La clé `done_generic` n'est plus utilisée (remplacée par la phrase) et a été retirée de `texts.py`.
- `outputs` n'est rangé que pour les ordres réussis pendant un `plan.execute` (étape annoncée par `plan.step`) ; l'étape fautive n'en reçoit jamais, même si son ordre avait réussi. Rien ne change sur le fil.
- Le clic (`itemClicked`, et `itemActivated` pour le clavier) ne refait que la zone Résultat, jamais les listes, pour ne pas détruire l'élément cliqué pendant son signal.
- `panel.py` n'est pas testé sans QGIS (aucun PyQt local) : compilé seulement ; la recette manuelle du README (ticket 13) reste à passer par le propriétaire.
- Audit de la matrice : le choix du texte au clic (lignes 4 en erreur, 6, 7) vivait dans la classe Qt, non testé. Sorti dans `summary.detail`, testé (`DetailTest`) ; 265 tests.

## Plan Change Log

## Review Triage Log

Passe 1 (quick, risque medium). 0 high, 1 medium, 6 low corrigés, 2 rejetés.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch | Le noyau résout `step:k` avant d'annoncer l'étape k (`core/src/serve/plans.rs` l.175-177) : une erreur de référence à l'étape 3 arrive avec l'étape 2 encore « en cours », qui passait en Erreur (vue et journal) et perdait sa phrase au clic (ligne 4 de la matrice). L'étape courante finie passe Terminée quand le verdict nomme une autre étape ; test. |
| 2 | low | patch | `_zonal` : `zones` ne garde que les 100 premières zones (`truncated`) ; la « zone la plus fournie » pouvait être fausse. Clause omise quand `truncated`. |
| 3 | low | patch | `number()` : `%g` arrondit à 6 chiffres (12345.75 → 12345.8). Formatage sans arrondi. |
| 4 | low | patch | `_send` / `_approve` n'effaçaient pas le détail cliqué : la zone Résultat montrait une étape disparue pendant la génération. `focus` remis à None. |
| 5 | low | patch | « En cours…. » : ponctuation doublée dans `focus_status`. Mot de statut nettoyé. |
| 6 | low | patch | `DetailTest` placé après le bloc `__main__`, jamais lancé en exécution directe. Bloc déplacé en fin de fichier. |
| 7 | low | patch | Détail au clic ou à Entrée jamais annoncé au lecteur d'écran (plancher d'accessibilité). `_announce` ajouté. |
| 8 | low | rejeté | Lignes finies en cours de plan : au clic pendant l'exécution, phrase des seuls paramètres jusqu'au verdict. Rare (un plan dure quelques secondes à quelques minutes, le clic vient après) et la correction ajoute un champ à `PLAN_STEP` et une branche ; le résultat apparaît dès la fin du plan. |
| 9 | false | rejeté | Règle §8 (relecture thorough) : elle vise les entrées de risque `high` ; celle-ci est `medium` (ticket `low`), rien ne change sur le fil, le client ne fait que garder les résultats d'ordres déjà rendus. |

## Design Notes

Une commande du noyau (pas d'ordre QGIS) n'a pas de résultat côté extension avant `plan.execute` : sur un plan en erreur, sa phrase est celle des paramètres. Le libellé de la ligne du journal reste court (description du registre) ; la phrase complète vient au clic et dans le résultat.

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: succès
- `python -I qgis-plugin/tests/check_extension.py` -- expected: code 0

**Manual checks (if no CLI):**
- QGIS 3.44, projet de démo : phrase de démo → Valider → clic sur chaque ligne et étape ; mode expert ; bascule FR/EN (recette du propriétaire, hitl).
