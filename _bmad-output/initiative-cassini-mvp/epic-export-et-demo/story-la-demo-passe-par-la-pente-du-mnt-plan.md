---
title: 'La démo passe par la pente du MNT'
type: 'feature'
ticket: '4'
created: '2026-11-13'
status: 'built'
baseline_revision: '7643198428ab91fec7e5acfcfc98adec779fda9b'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-export-et-demo/story-la-sonde-de-demo-juge-la-phrase-rejoue-le-journal-et-verifie-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la sonde de démo ne joue que la phrase des parcelles : rien ne prouve qu'une seconde phrase, sur le terrain, passe par `terrain.slope`, que la pente arrive dans le projet et dans le PDF, et qu'elle se rejoue.

**Approach:** le projet de démo reçoit le MNT synthétique déjà présent dans `fixtures/` (aucune nouvelle source) ; après la première phrase jugée, la sonde en propose une seconde qui demande la pente du MNT en degrés, ajoutée à la carte avec un nouvel export, la juge, puis rejoue le journal des deux plans.

## Boundaries & Constraints

**Always:** MNT = `fixtures/mnt-colline-2154.tif` copié dans le dossier du projet temporaire (jamais modifié), couche nommée `MNT`, ajoutée **sous** les couches vectorielles pour ne pas les cacher ; jugement dans `demo_verdict.py` (pur, sans qgis/Qt) ; les deux plans dans une même session ; rejeu sur `étapes du plan 1 + étapes du plan 2` ; la raison d'ECHEC nomme l'étape ou l'élément fautif ; aucune modification de l'extension ni du noyau.

**Never:** nouveau MNT ou nouvelle fixture raster ; modifier les fixtures vectorielles de l'entrée 1 ; réécrire `terrain.slope` ; style de la pente jugé (le noyau le dicte) ; données réelles.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|---------------|----------------------|------------------|
| Pente jouée | phrase 2 sur les fixtures | plan 2 avec `terrain.slope` (`unit` degrés, `dem` = le MNT), couche de pente dans le projet, nom du fichier de pente dans `sources` du second export avec plus de couches que le premier, rejeu des deux plans identique | `VERDICT : OK`, code 0 |
| Pas de `terrain.slope` | plan 2 sans cette commande | | `ECHEC — pente : terrain.slope absent du plan` |
| Mauvaise unité | `unit` = `percent` | | `ECHEC — pente : unité percent au lieu de degrees` |
| Mauvais MNT | `dem` ne désigne pas le MNT | | `ECHEC — pente : dem <valeur> n'est pas le MNT` |
| Pente absente | couche non ouverte, ou absente des `sources` du PDF | | `ECHEC — pente : couche absente du projet` / `du PDF` |
| Rejeu | deux plans journalisés | `identical`, `steps` = étapes des deux plans | `ECHEC — rejeu : …` (existant) |

</frozen-after-approval>

## Code Map

- `qgis-plugin/tests/probe_demo.py` -- `assemble_demo_project` (l.137-152) : copier le MNT dans `work`, ajouter `QgsRasterLayer(copie, "MNT", "gdal")` **avant** les vecteurs ; `reference_layers`/`missing_layers_reason` : référence au MNT ; `on_event` branche PLAN_DONE (l.419+) : après le plan 1 OK, `core.propose(PHRASE2)` (phase `plan2`), garder `steps1`/`steps2`, puis `start_replay_guarded`. Constante `PHRASE2`.
- `qgis-plugin/tests/demo_verdict.py` -- ajouter `judge_slope(steps, facts)` ; `judge_replay` reçoit `steps1 + steps2`.
- `qgis-plugin/tests/test_demo_verdict.py` -- un test par raison d'ECHEC de la matrice.
- `core/src/commands/terrain_slope.rs` -- NE PAS MODIFIER : params `dem` (chemin relatif du projet), `unit`, résultat `file`, pas de `layer_id` ; le noyau dicte ensuite `layer.open_raster` (nom = `1-terrain.slope`) et `layer.set_style`.
- Phrases d'exemple : `qgis-plugin/tests/recette/annecy.json:36` (degrés), `ai/tests/terrain_reel/phrases.json:36`.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/tests/demo_verdict.py` -- `judge_slope`, raisons nommées -- jugement pur
- [ ] `qgis-plugin/tests/test_demo_verdict.py` -- tests de la matrice -- couvre chaque ECHEC
- [ ] `qgis-plugin/tests/probe_demo.py` -- MNT dans le projet, seconde phrase, relevé des faits, rejeu des deux plans, docstring -- le verdict final

**Acceptance Criteria:**
- Given les fixtures, when la sonde tourne sous QGIS avec le vrai modèle, then elle rend `VERDICT : OK` : plan 2 avec `terrain.slope` en degrés, pente dans le projet et dans le PDF, rejeu identique.
- Given un plan 2 sans `terrain.slope`, en `percent` ou sur un autre fichier, when la sonde le juge, then elle rend ECHEC en nommant la cause.
- Given le MNT ajouté au projet, when la première phrase est jouée, then le jugement de ses parcelles et de sa mise en page reste OK (le raster n'a rien caché).

## Implementation Notes

Essais réels du 2026-11-13 (QGIS 3.44.15, noyau gpu local, modèle installé) : 3 exécutions sur 3 `VERDICT : OK`, même plan à chaque fois, rejeu `{"steps": 9, "identical": true}`. Écarts au plan : (1) `PHRASE2` = « Calcule les pentes en degrés à partir du MNT, puis sors-moi la carte en PDF. » ; avec « ajoute-la à la carte » le modèle 4B composait `raster.reclassify` ; (2) rejeu sur 9 étapes (journal) et non 7 ; (3) un projet passé en argument doit avoir une couche raster nommée « MNT ».
PDF du second export ouvert (Ghostscript de QGIS) : titre « Pentes en degrés à partir du MNT », légende avec la rampe de pente (3,53° à 23,29°) et le MNT, sources avec `6-terrain.slope.tif`, texte lisible. MAIS la pente n'est pas visible sur la carte : les couches vectorielles des étapes 1 à 4 restent visibles et la recouvrent (voir `deferred-work.md`).

## Plan Change Log

- 2026-11-13, après l'essai réel et la relecture complète : trois lignes du bloc figé ne tiennent plus et ne sont pas réécrites. (1) « le second export a plus de couches que le premier » : remplacé par « aucune sortie du plan 1 dans les `sources` du second export » (le noyau cache ces sorties depuis `plan-pente-cachee-dans-le-pdf`, il y en a donc moins) ; (2) « la pente est visible sous les parcelles » : vrai seulement parce que les polygones de la démo sont maintenant en contour seul (un résultat raster est rangé sous les vecteurs de l'utilisateur depuis 7.9) ; ce choix des fixtures de la sonde n'est pas dans le « Never » mais le complète ; (3) `sources1` n'est plus un fait de `judge_slope`, remplacé par `earlier_names`, les noms des sorties du plan 1.

## Review Triage Log

Passe 1 (quick) : low 3, false 3, rejet 2.

- Rejeu : les étapes attendues viennent du journal (9 lignes) et non de « plan 1 + plan 2 » (7) -- false : l'essai réel a montré que le noyau journalise aussi `layer.open_raster` et `layer.set_style` ; la sonde vérifie d'abord que les commandes des deux plans figurent dans l'ordre. Le journal d'un projet de démo est neuf (dossier temporaire).
- `judge_slope` compare le MNT par nom de fichier seul -- low, rejeté : le noyau reçoit un chemin relatif du projet ; une comparaison plus stricte casserait `./mnt…`.
- Branches de `judge_slope` sans test (résultat sans fichier, appel sans params) -- low, patch : deux tests ajoutés.
- `slope_in_project` teste par sous-chaîne -- low, rejeté : le nom `N-terrain.slope` est unique dans le projet de démo.
- Ligne de 110 caractères -- false : `check_extension.py` est propre.
- PDF du plan 1 pris pour celui du plan 2 -- false : les fichiers s'appellent `5-layout.export_pdf.pdf` et `9-layout.export_pdf.pdf` (essai réel).
- Ordre des vérifications de `judge_slope` -- low, rejeté : l'unité avant le MNT, un seul motif nommé par échec.

## Design Notes

`terrain.slope` prend un chemin de fichier, pas un `layer_id` : le modèle doit recopier le chemin relatif donné par l'inventaire (`mnt-colline-2154.tif`). La pente n'a pas de `layer_id` en sortie : on la retrouve par le nom `1-terrain.slope` (étape `layer.open_raster`) et par `sources` du second export. L'ajout du raster doit se faire avant les vecteurs, sinon l'ordre des couches le met au-dessus et le premier PDF perd ses parcelles.

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: OK
- `python -I qgis-plugin/tests/check_extension.py` -- expected: propre

**Manual checks (if no CLI):**
- Sur le poste : `probe_demo.py` sous le Python de QGIS (noyau gpu, modèle installé) : `VERDICT : OK` ; ouvrir le PDF du second export (Ghostscript de QGIS) : la pente est visible sous les parcelles, texte lisible.
