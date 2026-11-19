---
title: 'Comptage pondéré dans un polygone (analysis.count_points_in_polygon)'
type: 'refactor'
ticket: '2'
created: '2026-11-19'
status: 'built'
baseline_revision: 'a57e100fb0f7ef6d936fcb2573cbd2b4a0fe36d9'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 1
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `analysis.zonal_statistics` compte des points par polygone mais porte un nom que le catalogue réserve à `raster.zonal_statistics`, rend `zones` au lieu du gabarit commun, et ne sait pas sommer un champ numérique.

**Approach:** renommer la commande en `analysis.count_points_in_polygon` partout, migrer son résultat au gabarit `SummaryRow`/`SummaryTable` de 10.1 (`rows`, `truncated`), ajouter `weight_field` facultatif qui ajoute `weighted_sum` à chaque ligne.

## Boundaries & Constraints

**Always:** fiche `count_points_in_polygon` du catalogue reprise telle quelle (params `layer_id`, `points_layer_id`, `label_field`, `weight_field`, `out`; résultat `layer_id`, `name`, `crs`, `feature_count`, `rows`, `truncated`), seul son statut passe à `faite`, la fiche de l'ancien id est retirée ; `rows` borné à 100 par `length(max=100)` ; `count` porte l'unité « points », `weighted_sum` une unité vide, présent seulement avec `weight_field` ; étiquette = `label_field` sinon rang, jamais l'id d'entité ; aucun nom d'algorithme ni de champ QGIS dans le noyau (AD-22) ; textes en français et en anglais ; un test par ligne de la matrice.

**Never:** toucher à `raster.zonal_statistics` (entrée 12) ; réécrire les phrases S15, S23, S24 ni `tests/epreuve-modele-local/catalog.json` et les `resultats-*` archivés ; ré-entraîner le modèle ; garder l'ancien id en alias ; toucher à `.claude/worktrees` ni à `_bmad-output/party-mode/`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Comptage simple | polygones, points, sans `weight_field` | une ligne par zone, `count` avec unité « points » | — |
| Comptage pondéré | `weight_field` numérique sur les points | chaque ligne ajoute `weighted_sum`, `count` reste le nombre de points | — |
| Champ non numérique | `weight_field` désigne un texte | refus avant écriture | erreur de l'extension, rien produit |
| Plus de 100 zones | 150 polygones | 100 lignes, `truncated: true` | — |
| Aucune zone | couche de polygones vide | `rows` vide, `truncated: false` | — |
| Résultat mal formé | `zones` à l'ancienne | rejeté | `qgis.bad_result`, rien journalisé |
| Ancien id | `analysis.zonal_statistics` | inconnu du registre | refus du plan |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_zonal_statistics.rs` -- à renommer (`git mv`) en `analysis_count_points_in_polygon.rs` ; structs, `MAX_ZONES`, exemples, 4 tests ; pas de `mod` à éditer (`core/build.rs`).
- `core/src/commands/summary_row.rs` -- `SummaryRow`, `SummaryValue`, `SummaryTable` à réutiliser ; première commande qui les déclare.
- `core/tests/catalogue.rs`, `core/tests/conformite_resultats.rs`, `fixtures/resultats-extension.json`, `core/src/journal/tests.rs` (l. 2316, 4320, 4336) -- fiche, variantes « avec zones/tronqué/sans zone », ids.
- `catalogue-des-commandes.md` -- tableau l. 348 et 368, fiche l. 657-681 à retirer, fiche l. 1722-1746 à passer `faite`.
- `qgis-plugin/cassini_qgis/analysis.py` -- `ALGORITHMS["analysis.zonal_statistics"]` (l. 445), `translate_`/`summarize_zonal_statistics`, `counts_of` (l. 770) ; `native:countpointsinpolygon` avec `WEIGHT`.
- `qgis-plugin/cassini_qgis/summary.py` (`_zonal` l. 261, table l. 953), `texts.py` -- lire `rows` ; clés `sum_zonal*`.
- `qgis-plugin/tests/` -- `test_analysis.py` (`ZonalStatisticsTest`), `test_summary.py`, `test_result_conformance.py`, `test_recette_verdict.py`, `table_annulable.json`, `recette/annecy.json`, `probe_zonal_statistics.py`.
- `training/src/families.rs` (l. 3512 famille, 9364 `NO_RASTER_STATS`), `ai/tests/par_commande/phrases.json`, `ai/tests/plan_reel/phrases.json`, `ai/tests/tenu_a_l_ecart/sentences.json`, `ai/tests/plan_reel.rs` -- phrases et ids ; le refus `raster.zonal` de S15/S23/S24 ne change pas.
- `README.md` (l. 119), `qgis-plugin/README.md`, `docs/entrainement-du-modele.md` (l. 3379) -- docs.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_count_points_in_polygon.rs` -- renommage, `weight_field: Option<String>`, sortie `rows: Vec<SummaryRow>` (max 100) + `truncated` -- contrat de la fiche
- [ ] `catalogue-des-commandes.md` -- fiche et tableau : nouvel id `faite`, ancienne fiche retirée -- seul changement permis
- [ ] `core/tests/conformite_resultats.rs`, `fixtures/resultats-extension.json`, `core/src/journal/tests.rs` -- ids et exemples au gabarit, variante pondérée
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` -- ligne renommée ; pondération : le décompte et la somme sont deux passes de `native:countpointsinpolygon` (champs distincts), `weight_field` refusé s'il n'est pas numérique ; résumé produit `rows`
- [ ] `qgis-plugin/tests/*` -- une épreuve par ligne de la matrice ; sonde renommée `probe_count_points_in_polygon.py`, lancée sous QGIS
- [ ] `training/src/families.rs`, `ai/tests/*`, `README.md`, `qgis-plugin/README.md`, `docs/entrainement-du-modele.md` -- ids, famille d'exemples avec et sans pondération
- [ ] vérification globale -- `grep` de `analysis.zonal_statistics` : seuls restent l'épreuve gelée et les `resultats-*` archivés

**Acceptance Criteria:**
- Given un comptage simple et un comptage pondéré, when l'extension rend son résumé, then le noyau le valide au gabarit sans `qgis.bad_result`.
- Given le registre, when on cherche `analysis.zonal_statistics`, then la commande est inconnue ; S15, S23 et S24 restent en refus `raster.zonal` jusqu'à l'entrée 12.
- Given `cargo test --workspace` et les tests Python, when on les lance, then ils réussissent, `catalogue.rs` et `test_delegated.py` compris.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| Constat | Verdict | Preuve |
|---|---|---|
| README : `` de `15in` perdu (retour-arrière) | low → patch | octet 0x08 trouvé dans `qgis-plugin/README.md` ; remplacé par `in` |
| README promet des champs numériques ; `FieldCombo` liste tous les champs | low → patch | `panel.py` `FieldCombo.reload` sans filtre de type ; le refus du champ texte reste porté par `_check_numeric_field` ; README corrigé |
| `_count_points` : `row.get` sans garde `isinstance(row, dict)` | low → patch | l'ancien code gardait ; garde rétablie |
| `weight_field` vide : `is not None` contre valeur logique | low, rejeté | `form.build_params` omet une saisie vide (form.py l. 156) ; le panneau n'envoie jamais `""` ; correctif = changer la sémantique partagée de `when` |
| `.tests(&[...])` sans trois tests ajoutés ; « rien journalisé » non testé ici | low → patch | trois noms ajoutés ; « rien journalisé » est prouvé par les tests de 10.1 (`qgis.bad_result`) |
| `FOCUS_EXCLUDED` vidé hors plan | false | forcé par le tirage pondéré ; `cargo test --workspace` vert, `recouvrement` compris |
| Phrases pondérées absentes de `ai/tests/*` | low, rejeté | le plan demande la famille d'exemples, faite dans `training/src/families.rs` ; correctif = ré-entraînement groupé (fin d'epic 10) |

## Design Notes

`native:countpointsinpolygon` avec `WEIGHT` écrit la somme pondérée à la place du décompte dans son champ : pour rendre `count` et `weighted_sum` ensemble, la chaîne passe deux fois, chaque passe avec son propre `FIELD`. Si la chaîne actuelle n'accepte qu'un `overlay_key` par étape, vérifier avant de coder que deux étapes peuvent le déclarer.

## Verification

**Commands:**
- `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` -- expected: vert
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert

**Manual checks (if no CLI):**
- Sonde `probe_count_points_in_polygon.py` sous QGIS : comptage et somme pondérée sur fixture synthétique.
