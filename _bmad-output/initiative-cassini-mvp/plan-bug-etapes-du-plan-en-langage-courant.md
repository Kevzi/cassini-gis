---
title: 'Étapes du plan en langage courant dans le panneau'
type: 'bugfix'
ticket: ''
created: '2026-11-08'
status: 'built'
baseline_revision: '750495135e44a87004a1744441710c141a60b742'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/ux-panneau-cassini/EXPERIENCE.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le panneau nomme chaque étape (plan proposé, journal, historique rechargé) par la description de la commande au registre, écrite pour le modèle (« Pente d'un MNT : calcule la pente de chaque cellule… fichier GeoTIFF désigné par `dem`, chemin de fichier MNT recopié de l'inventaire, pas un id de couche… »), et le résultat d'une commande de terrain ou de raster répète cette description faute de phrase de résumé ; constaté le 2026-11-08 sur Annecy, « Fais la carte des pentes du MNT à 25 m en pourcentage » (captures `%TEMP%\capture-panneau\340-4-plan.png`, `340-5-resultat.png`).

**Approach:** une étape se lit par une phrase courte tirée de ses paramètres (« Pente de mnt_25m en pourcentage »), la même que le résumé sait déjà faire avant résultat ; chaque commande du catalogue au statut `faite` reçoit sa phrase, avant et après résultat ; la description du registre n'est plus montrée telle quelle.

## Boundaries & Constraints

**Always:** affichage seul, aucune logique métier (AD-4, AD-5) : la phrase ne calcule rien, elle recopie paramètres et résultat ; FR et EN ; une commande sans phrase (ajoutée plus tard) se lit par un repli court — le titre de la description avant « : » quand il existe, sinon sa première phrase, jamais le texte entier ; une référence `step:k.…` se lit par le nom de l'étape qu'elle cite, comme aujourd'hui ; un test échoue quand une commande `faite` du catalogue (`catalogue-des-commandes.md`) n'a pas de phrase.

**Never:** aucun changement de mise en page (taille des cadres, ordre des zones, styles) : c'est le lot suivant, par `bmad-ux` ; aucun changement des descriptions du registre côté noyau (le modèle les lit) ; pas de noyau, de générateur ni de modèle touchés.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan de pente proposé | `terrain.slope {dem: …/mnt-74-25m.tif, unit: percent}` | ligne « 1. Pente de mnt-74-25m en pourcentage » (nom de fichier sans chemin) | — |
| Étape terminée | même étape, résultat rendu | phrase du résultat, sans la description du registre | — |
| Étape enchaînée | `analysis.intersect {overlay_layer_id: "step:1.layer_id"}` | la référence lue « la zone tampon de l'étape 1 » (forme actuelle) | — |
| Historique rechargé | journal d'une session passée | mêmes libellés courts que pendant la session | paramètres résumés (> 200 octets) : repli court |
| Commande sans phrase | id inconnu de `_SENTENCES` | titre court tiré de la description, ou l'id | jamais d'exception jusqu'à l'utilisateur |
| Paramètres illisibles | paramètre attendu absent ou mal typé | repli court | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/agentview.py` l. 195-218 -- `describe`, `step_label`, `step_line` : libellé d'une étape = description entière ; à remplacer par la phrase courte. `summary.py` importe déjà `agentview` (attention à l'import circulaire : le libellé peut être fourni par le panneau, ou `describe` rendre le titre court).
- `qgis-plugin/cassini_qgis/summary.py` -- `_SENTENCES` l. ~481 (phrases par commande, chaque fonction rend la forme « avant résultat » quand `r` est None), `sentence` l. 508, `fallback` l. 502 (description entière + « Terminée » : à faire court), `detail` l. 526. Manquent aujourd'hui, entre autres : `terrain.slope`, `terrain.aspect`, `terrain.hillshade`, `terrain.viewshed`, `terrain.viewshed_cumulative`, `terrain.line_of_sight`, `terrain.peaks`, `terrain.cost_distance`, `terrain.least_cost_path`, `raster.reclassify`, `raster.weighted_overlay` ; vérifier les commandes déléguées (`layer.*`, `map.*`) contre la liste `faite` du catalogue.
- `qgis-plugin/cassini_qgis/texts.py` -- phrases FR (`_FR`) et EN (`_EN`), clés `sum_*`.
- `qgis-plugin/cassini_qgis/panel.py` l. 582 (`RunLog.from_history`), l. 643-645 (`PLAN_STEP`), l. 858 (`_render_focus`), l. 961 (`step_line`) -- appels du libellé.
- `qgis-plugin/cassini_qgis/runlog.py` l. 64-95 -- `from_history(history, describe)` : le libellé ne reçoit que la commande ; il doit pouvoir lire les paramètres de l'entrée.
- `qgis-plugin/tests/test_summary.py`, `test_agent_view.py`, `test_panel_logic.py` -- tests à étendre ; formes existantes à garder.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- liste des commandes `faite` (fiches ```` ```json fiche ```` avec `"status": "faite"`), source du test de complétude.
- Captures avant correction : `C:\Users\kevin\AppData\Local\Temp\capture-panneau\` ; script de capture réutilisable `capture.py` (même dossier, lancé par le Python de QGIS avec `QT_QPA_PLATFORM=windows`).

## Tasks & Acceptance

**Execution:**
- [x] `qgis-plugin/cassini_qgis/summary.py`, `texts.py` -- phrases FR/EN avant et après résultat pour chaque commande `faite` qui n'en a pas ; libellé d'étape = phrase avant résultat ; repli court.
- [x] `qgis-plugin/cassini_qgis/agentview.py`, `panel.py`, `runlog.py` -- le plan, le journal et l'historique rechargé prennent ce libellé.
- [x] `qgis-plugin/tests/test_summary.py`, `test_agent_view.py`, `test_panel_logic.py` -- cas de la matrice ; test de complétude contre le catalogue.
- [x] Captures refaites après correction (même script, même phrase) et regardées.

**Acceptance Criteria:**
- Given le catalogue des commandes, when on lance les tests de l'extension, then chaque commande `faite` a une phrase FR et EN, avant et après résultat.
- Given QGIS 3.44 et la phrase « Fais la carte des pentes du MNT à 25 m en pourcentage » sur Annecy, when le plan est proposé puis exécuté, then aucune ligne du plan, du résultat ni du journal ne contient la description du registre (« GeoTIFF désigné par `dem` », « recopié de l'inventaire »).

## Implementation Notes

- `summary.label` rend la phrase avant résultat (sans point) ; `summary.short_title` le repli court (avant « : », sinon première phrase, coupée au mot au-delà de 80 caractères, sinon l'id). `fallback` rend désormais le titre court + « Terminée ».
- Import circulaire évité : `summary` n'importe plus `agentview` ; c'est `agentview` (et `external`) qui importent `summary`.
- `agentview.describe` garde la description entière, pour la seule question d'accord de l'IA externe ; la ligne de journal de l'IA externe (`external.label`) prend le titre court.
- `RunLog.from_history(history, describe)` appelle `describe(command, params)`, `params` à None quand le journal les a résumés.
- Un raster se lit par son nom de fichier sans dossier ni extension (`mnt-74-25m`) ; une référence `step:k.file` par l'étape citée. En français, « de le résultat » devient « du résultat » (contraction appliquée à toute phrase).
- Les valeurs de résultat raster (min, max, coût, altitude) sont affichées au dixième.
- Test de complétude : `CatalogueCompletenessTest` lit les fiches `faite` de `catalogue-des-commandes.md` et exige un exemple dans `SAMPLES` et une phrase FR/EN avant et après résultat.
- Captures du 2026-11-08 (QGIS 3.44.15, 340 px) : plan « 1. Pente de mnt-74-25m en pourcentage », résultat « … : fichier 1-terrain.slope.tif, de 0 à 5265,7 % », journal idem ; projet de capture archivé dans `donnees-test-cassini/_archives/sondes/projet-capture-panneau-etapes-langage-courant.*`.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 0, medium 4, low 2, false 0, maybe-false 0 ; tous corrigés.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Libellé d'étape au passé pour `layout.export_pdf` et `data.reproject` : un plan proposé se lit comme fait | medium | patch | clés `sum_export_pdf_ask`, `sum_reproject_ask` ; test `test_no_label_uses_an_after_result_key` |
| `short_title` coupe dans `EPSG:4326` (`coord.convert`) | medium | patch | coupe seulement sur « : » suivi d'une espace |
| La coupe à 80 caractères garde les parenthèses écrites pour le modèle | medium | patch | coupe avant « ( » ou « ; » ; test sur les vraies descriptions du registre |
| Contraction « de le » → « du » appliquée à tout le texte, noms de l'utilisateur compris | medium | patch | contraction limitée à la référence d'étape (marques) ; test « de le test » intact |
| Double ponctuation « …. Terminée. » dans le repli | low | patch | fin du titre nettoyée avant le point |
| Exemple `system.about` hors des clés du catalogue | low | patch | `core_version`, `protocol_version` |

## Verification

**Commands:**
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python3 -I qgis-plugin/tests/check_extension.py` -- expected: propre

**Manual checks (if no CLI):**
- `powershell.exe -NoProfile -Command '$env:QT_QPA_PLATFORM="windows"; & "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" "$env:TEMP\capture-panneau\capture.py" 340'` après avoir copié l'extension dans le profil QGIS (`rsync -a --delete --exclude __pycache__ qgis-plugin/cassini_qgis/ /mnt/c/Users/kevin/AppData/Roaming/QGIS/QGIS3/profiles/default/python/plugins/cassini_qgis/`) et une copie `annecy/projet-capture-panneau.qgz` du projet Annecy (archivée ensuite dans `_archives/sondes/`) -- expected: captures `340-4-plan.png` et `340-5-resultat.png` en phrases courtes.
