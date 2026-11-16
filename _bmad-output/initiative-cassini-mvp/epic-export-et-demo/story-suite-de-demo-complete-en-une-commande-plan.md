---
title: 'Suite de demo complete en une commande'
type: 'feature'
ticket: '8'
created: '2026-11-17'
status: 'built'
baseline_revision: 'e9a0e79c7fcd284093977acf3cbec7ac98e408d4'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/epic-export-et-demo/epic-export-et-demo.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La sonde de demo (`probe_demo.py`) juge la phrase de demo, le rejeu, le PDF et l'absence de reseau, mais elle s'arrete au premier echec, ne rend qu'une ligne `VERDICT`, n'a aucun verdict D4 nomme, ne traite qu'une entree par lancement, et son mode d'emploi n'existe que dans sa docstring. Le proprietaire ne peut pas la lancer « en une commande » ni lire un rapport D1 a D4.

**Approach:** Garder la sonde comme seul moteur (pas de second point d'entree) : elle accumule un etat par critere D1 a D4 et imprime un rapport lisible ; un script de lancement la joue sur les fixtures, puis sur le projet reel, et rend un code de sortie global ; une page de documentation dit comment la lancer.

## Boundaries & Constraints

**Always:** `probe_demo.py` reste l'unique sonde de la phrase de demo, son comportement actuel (ligne `VERDICT`, code 0/1, variables `CASSINI_DEMO_*`) est conserve ; le verdict par critere est une fonction pure testee hors QGIS (`demo_verdict.py`) ; les fixtures et le projet reel ne sont jamais modifies ; aucune donnee reelle versionnee ; le rapport reprend D1, D2, D3 et D4 avec OK, ECHEC ou « non juge » et la raison.

**Never:** pas de nouvelle commande du registre ni de logique metier dans l'extension ; pas de second point d'entree qui rejouerait la phrase d'une autre facon ; pas de lancement dans la CI (ni QGIS ni modele) ; pas de changement du noyau, du modele ni du garde-fou.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Tout passe | fixtures puis projet reel | rapport D1 a D4 OK pour chaque entree, `RESULTAT GLOBAL : OK`, code 0 | — |
| Un critere echoue | ex. mise en page sans legende | D3 ECHEC avec sa raison, les autres critères restent juges quand ils ont pu l'etre, code 1 | `VERDICT : ECHEC` conserve |
| Echec avant le rejeu | plan refuse ou erreur d'etape | D1 ECHEC, D2 D3 D4 « non juge », code 1 | pas de faux OK |
| Pas de projet reel | option `-SansProjetReel` ou projet absent | fixtures seules jugees, mention « projet reel : non joue », code 0 si OK | projet absent donne code 2, jamais un OK silencieux |
| Sonde non partie | noyau, modele ou QGIS introuvable | message clair, code 2 | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/tests/probe_demo.py` -- sonde (659 l.) : `finish` (l.~373) court-circuite au premier echec ; D1 `check_phrase`/`judge_done`/`poll_network`, D2 `start_replay` (~533), D3 `layout_verdict` (~452), D4 seulement `judge_slope` (PHRASE2). Accumuler un dict par critere sans changer la ligne `VERDICT`.
- `qgis-plugin/tests/demo_verdict.py` -- verdict pur : y ajouter `judge_terrain(steps)` (D4, au moins une commande `terrain.*` executee par le noyau) et `report_lines(criteria)` ; `verdict_line` (l.43) reste.
- `qgis-plugin/tests/test_demo_verdict.py` -- tests purs : rapport tout OK, un ECHEC, critere « non juge », D4 sans commande de terrain.
- `../outils-cassini/scripts/demo-11-14.ps1` -- modele du lanceur (`CASSINI_CORE`, `CASSINI_MODELS`, Python de QGIS `python-qgis-ltr.bat`, `QT_QPA_PLATFORM=windows`, codes 0/1/2). Hors depot : ne pas le toucher, s'en inspirer.
- `qgis-plugin/tests/lancer_demo.ps1` -- a creer : le lanceur versionne.
- `docs/lancer-la-demo.md` -- a creer, liee depuis `README.md` (l.~566, ligne de `probe_demo.py`).
- `fixtures/demo/`, `fixtures/mnt-colline-2154.tif` (fixtures par defaut) ; `../donnees-test-cassini/annecy/projet-test-annecy.qgz` (projet reel, hors depot).

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/tests/demo_verdict.py` -- `judge_terrain` et `report_lines` (une ligne par critere D1 a D4 : OK, ECHEC ou non juge, avec la raison) -- verdict pur testable
- [ ] `qgis-plugin/tests/test_demo_verdict.py` -- tests de la matrice (tout OK, un echec, non juge, D4 sans terrain)
- [ ] `qgis-plugin/tests/probe_demo.py` -- noter chaque critere au fil du plan, imprimer le bloc `RAPPORT` avant `VERDICT`, sans changer le code de sortie ni la ligne `VERDICT`
- [ ] `qgis-plugin/tests/lancer_demo.ps1` -- joue la sonde sur les fixtures, puis sur le projet reel (`-Projet`, `-SansProjetReel`, `-Modeles`, `-Sortie`), ecrit le rapport complet dans un fichier, `RESULTAT GLOBAL`, codes 0/1/2
- [ ] `docs/lancer-la-demo.md` et `README.md` -- page de lancement (fixtures, projet reel, ses propres donnees, lecture du rapport, prerequis : noyau et modele installes) et lien depuis le README

**Acceptance Criteria:**
- Given le noyau et le modele installes, when `lancer_demo.ps1` est lance sur le poste du proprietaire, then il rend `RESULTAT GLOBAL : OK` avec les verdicts D1 a D4 pour les fixtures et pour le projet reel d'Annecy.
- Given une sonde qui echoue (`CASSINI_DEMO_DROP_ITEM=legend`), when le lanceur tourne, then le rapport nomme D3 en echec et le code de sortie est 1.
- Given `probe_demo.py` lance seul comme avant, then sa sortie garde `VERDICT : OK` ou `ECHEC` et son code.

## Implementation Notes

Fait le 2026-11-17 : `demo_verdict.py` (`judge_terrain`, `terrain_commands`, `criterion`, `report_lines`), tests (888 verts), `probe_demo.py` (etat par critere, bloc `RAPPORT`, `CASSINI_DEMO_LABEL`), `lancer_demo.ps1`, `docs/lancer-la-demo.md`, lien du README. Ecart hors plan, necessaire au critere d'Annecy : la sonde accepte un champ de zonage en majuscules (`TYPEZONE`) et un raster dont le nom commence par `mnt` (le projet reel a `mnt_25m` et `mnt_5m_annecy`, pas `MNT`) ; la pente est jugee sur le MNT choisi par le plan. Statut du critere : « non juge » avec accent.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 3 medium, 1 low, 1 maybe-false.

| Verdict | Route | Evidence |
|---|---|---|
| medium | patch (fait) | `finish` remplacait la raison de D2/D4 par la premiere defaillance notee (`state["first"]`) : chaque critere garde maintenant sa raison, `first` ne sert qu'a la ligne VERDICT. |
| medium | patch (fait) | Une sonde sortie sans ligne VERDICT (fixture ou projet illisible) rendait code 1 : le lanceur la traite en « sonde non partie », code 2. |
| medium | patch (fait) | Chemins relatifs resolus avant puis apres `Set-Location` : `-Projet`, `-Modeles`, `-Sortie` sont rendus absolus au depart (chemins deja absolus gardes : verifie en jouant le lanceur, un premier essai les cassait). |
| low | patch (fait) | Detail OK de D4 pris sur les etapes des deux plans alors que le verdict l'est sur le plan 2 : detail sur les etapes jugees. |
| maybe-false | rejete | `CASSINI_DEMO_DROP_ITEM=legend` : lanceur joue sur fixtures et projet reel, D3 en ECHEC « legende absente » les deux fois, code 1, D1/D2/D4 OK. Le constat ne se produit pas. |


## Design Notes

Rapport attendu (une entree) :
```
RAPPORT fixtures
  D1 phrase de demo, hors ligne   OK
  D2 rejeu identique              OK (9 etapes)
  D3 PDF : titre, legende, echelle, nord, sources   OK
  D4 commande de terrain du noyau OK (terrain.slope)
```

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: tout vert
- `qgis-plugin/tests/lancer_demo.ps1` sur le poste -- expected: `RESULTAT GLOBAL : OK`, code 0

**Manual checks (if no CLI):**
- Ouvrir le PDF du projet reel et le regarder (pas seulement le verdict).
