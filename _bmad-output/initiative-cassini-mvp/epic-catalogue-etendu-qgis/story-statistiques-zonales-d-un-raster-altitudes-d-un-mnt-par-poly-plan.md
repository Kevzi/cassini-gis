---
title: 'Statistiques zonales d''un raster, altitudes d''un MNT par polygone'
type: 'feature'
ticket: '12'
created: '2026-11-20'
status: 'built'
baseline_revision: '2240fa97a2ccfbb61345e2f6023c15bff3a65521'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Aucune commande ne rend la moyenne, le minimum et le maximum d'un raster par polygone : le générateur d'exemples refuse ces phrases (`NO_RASTER_STATS`) et le geste générique de l'extension ne sait traiter un raster qu'en entrée et sortie raster.

**Approach:** Ajouter `raster.zonal_statistics`, déléguée à QGIS (`native:zonalstatisticsfb`, qui écrit une nouvelle couche), avec un raster `RasterPath` et une couche de polygones par `layer_id` ; le noyau valide un résumé au gabarit ; le générateur remplace son refus par une famille d'exemples.

## Boundaries & Constraints

**Always:** Le raster se désigne par `RasterPath` (chemin relatif au projet pris de l'inventaire, ou `step:k`), comme le fixent AD-24 (amendement du 2026-11-05) et la fiche du catalogue : le texte du ticket et du ticket 20 (« identifiant de couche ») est périmé, et on le corrige. Les polygones se désignent par `layer_id` ; `label_field` facultatif. Une seule bande, résultat `mean`/`min`/`max` en `{value, unit}`, 100 lignes au plus avec `truncated` (gabarit `SummaryRow`). Fichiers d'origine intacts, sans `.aux.xml` (AD-10), raster libéré avant l'algorithme. Reprojection ou refus si le SCR du raster et celui des polygones diffèrent. Test et sonde sous QGIS 3.44.

**Never:** Recalculer un `terrain.*` ; algorithme qui écrit dans la couche d'entrée (`qgis:zonalstatistics`) ; modifier `RasterPath`, `layer.add`, `raster_output.py`, `agent_inventory.rs` ou `terrain.*` ; ré-entraîner le modèle (groupé au ticket 17) ; toucher `NO_RASTER_THRESHOLD` (ticket 25).

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Nominal | MNT synthétique `mnt-colline-2154.tif`, polygones en 2154 | une ligne par polygone, `mean`/`min`/`max` exacts avec unité, couche résultat ajoutée | aucune |
| Chaîne | `raster: "step:1"` après `terrain.slope` | rejeu identique, marqueur `out:<j>.1` journalisé | aucune |
| Hors raster | polygone entièrement hors de l'emprise | valeurs absentes pour ce polygone, ligne rendue | aucune |
| Mauvais type | `layer_id` d'une couche non polygonale | refus | code de geste existant pour une géométrie inadaptée |
| Raster inconnu | chemin absent ou non `.tif` | refus | `layer.unsupported_format` ou équivalent existant |
| SCR différents | raster 2154, polygones en WGS84 | statistiques exactes ou refus net, jamais un résultat faux | à constater par la sonde |

</frozen-after-approval>

## Code Map

- `core/src/commands/raster_zonal_statistics.rs` (nouveau, ~170 l.) -- modèle : `analysis_dissolve.rs` et `analysis_count_points_in_polygon.rs` (`Params`, `Output`, `Delegated`, `.path_params(&["raster"])`, exemples, `.tests`) ; `summary_row.rs` pour le gabarit ; `commands/mod.rs` pour l'inscription
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` (l.380, 2082-2105) -- fiche déjà écrite, `planifiee` → livrée ; `core/tests/catalogue.rs` compare
- `fixtures/resultats-extension.json`, `core/tests/conformite_resultats.rs`, `core/src/journal/tests.rs` -- cas de résultat, rejeu avec `step:k`
- `qgis-plugin/cassini_qgis/analysis.py` -- `ProcessingGesture._begin` (l.1408), `_begin_raster` (1537-1569), doc des clés (601-607), `ALGORITHMS` : nouvelle ligne avec entrée mixte (`input: "layer_id"` + `raster_input: "raster"`), `translate_zonal_statistics`, `summarize_zonal` d'après `summarize_count_points` (347)
- `qgis-plugin/cassini_qgis/summary.py` (~1032), `texts.py`, `form.py`, `panel.py` -- entrée de répartition, textes FR/EN ; sélecteur de fichier raster déjà là (`x-cassini: raster`)
- `qgis-plugin/tests/` : `test_analysis.py`, `test_summary.py`, `test_result_conformance.py`, `test_panel_logic.py`, `table_annulable.json` ; nouvelle `probe_zonal_statistics.py` (~230 l., d'après `probe_dissolve.py`)
- `training/src/families.rs` -- retirer `NO_RASTER_STATS` (l.4562), famille `refus_stats_raster` (4757-4880, 249-251, 240), entrées 9345 et 9384, tests 12753-12835, 12969, 14666 ; ajouter une famille d'après `decompte_par_zone` (~3500)
- `ai/tests/par_commande/phrases.json`, `inventory.json`, `ai/tests/plan_reel.rs` (156, 185-190, 484-485 : S15, S23, S24 passent de refus à plan), `training/src/barres.rs:54` (compte des phrases)
- Docs : `docs/entrainement-du-modele.md` (116, 261, 1133, 1170, 2343), `deferred-work.md` (652, 124-127), READMEs, tickets du ticket 12 et 20 à corriger
- À ne pas toucher : voir Never ; le dossier `.claude/worktrees/agent-ad593...` est une vieille copie.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/raster_zonal_statistics.rs`, `mod.rs`, catalogue -- commande, schéma, résumé, tests -- contrat du registre
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` -- geste raster + polygones, `native:zonalstatisticsfb`, résumé avec unité (lue du raster ou déclarée), SCR -- exécution par QGIS
- [ ] `qgis-plugin/tests/probe_zonal_statistics.py` -- QGIS 3.44 : altitudes exactes par polygone, unité, source intacte sans `.aux.xml`, polygone hors emprise, SCR différents ; constats dans Implementation Notes
- [ ] `fixtures/resultats-extension.json`, tests plugin, journal -- conformité et rejeu `step:k`
- [ ] `training/src/families.rs`, `phrases.json`, `plan_reel.rs`, `barres.rs` -- retrait du refus, famille d'exemples, « pente moyenne par commune » en plan via `step:k` après `terrain.slope`, sinon refus
- [ ] Docs, READMEs, texte des tickets 12 et 20 -- alignés sur `RasterPath`

**Acceptance Criteria:**
- Given le MNT synthétique de l'entrée 20 et des polygones connus, when la commande s'exécute dans QGIS, then les altitudes moyenne, minimale et maximale par polygone sont exactes, avec leur unité, et le fichier raster reste intact.
- Given un résumé de l'extension, when le noyau le valide, then il respecte le gabarit et reste sous 100 lignes avec `truncated`.
- Given une phrase demandant l'altitude moyenne par commune, when l'agent compose, then le plan est valide.
- Given un journal avec `raster: step:k`, when on le rejoue, then le résultat est identique.

## Implementation Notes

Constats de la sonde `qgis-plugin/tests/probe_zonal_statistics.py` (QGIS 3.44.15, `VERDICT : OK`) :

- `native:zonalstatisticsfb`, `STATISTICS = [2, 5, 6]` (moyenne, minimum, maximum), `COLUMN_PREFIX = "_"` : la sortie ajoute `_mean`, `_min`, `_max` aux champs d'entrée (plus `fid` du GeoPackage) ; la couche d'entrée n'est pas touchée.
- Plan incliné (valeur = 99,5 - colonne) : moyenne, minimum et maximum exacts (95 / 90,5 / 99,5 et 85 / 80,5 / 89,5). Colline : égalité avec la lecture cellule par cellule de QGIS (400 cellules).
- Polygone entièrement hors de l'emprise : champs nuls, donc une ligne sans valeur (`values: {}`), les autres lignes intactes.
- SCR différents (polygones WGS84, raster Lambert-93) : QGIS reprojette lui-même les polygones, les statistiques sont exactes et la couche produite reste en WGS84. Le geste ne refuse donc pas ce cas ; il refuse un raster sans système.
- `.aux.xml` : QGIS en pose un à côté du raster à la fermeture d'une `QgsRasterLayer` (le raster d'origine reste identique octet pour octet, mais un fichier annexe apparaît). `GDAL_PAM_ENABLED=NO`, pris avant la première ouverture du raster et rendu à la fin du geste (compteur de détenteurs), l'évite, y compris pour la couche que l'algorithme ouvre seul. Le réglage se lit à l'ouverture du jeu de données : il n'a pas besoin de tenir jusqu'à sa fermeture.
- L'unité n'est pas portée par les GeoTIFF de test (`GetUnitType()` vide) : elle est `unit` si le plan la donne (« ° » ou « % » pour une pente), sinon l'unité GDAL du fichier (`metre`, `degrees`, normalisées), à défaut `m`. Paramètre `unit` ajouté à la fiche du catalogue.

Décisions : une seule bande (refus de plusieurs) ; extension `.tif` ou `.tiff` seulement (`layer.invalid_source` sinon) ; `layer.not_found` pour un chemin absent ; `analysis.bad_params` pour une couche non polygonale (garde `geometry` existante). Générateur : `refus_stats_raster` et `NO_RASTER_STATS` retirés, famille `statistiques_raster_par_zone` (altitude, pente par `step:1`), S15 et S23 attendent un plan dans `plan_reel` ; S24 (champ de vision puis moyenne par commune) reste un refus (`NO_VISIBILITY_STATS`) faute d'unité pour une part visible, ce qui précise la ligne « S15, S23, S24 en plan » du Code Map au profit de la tâche « sinon refus ».

## Plan Change Log

- 2026-11-20, relecture quick, constat intent_gap : l'unité du résultat tombait sur `m` sans `unit` ni métadonnée, donc une pente enchaînée par `step:k` sortait en mètres. Le plan ne disait pas comment l'unité se détermine. Dérogation décidée par le propriétaire : au lieu d'annuler le code et de replanifier, le correctif s'applique comme correctif ciblé (le noyau refuse `step:k` sans `unit` ; un raster par chemin garde `m`). Évite un résultat faux silencieux. KEEP : toute l'implémentation, la sonde, le geste mixte, la famille d'exemples et le retrait de `NO_RASTER_STATS`.

## Review Triage Log

Passe 1 (quick) : 2 constats réels (1 medium, 1 medium en attente de décision), 3 écartés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch | Le catalogue (l.2104-2106), le commentaire de `layer_id` et la description envoyée au modèle disent « même système de coordonnées, sinon `analysis.bad_params` », alors que le geste admet des SCR différents (QGIS reprojette, sonde : statistiques exactes) ; aligner les trois textes sur le comportement |
| medium | intent_gap | `summarize_raster_zonal` rend l'unité `m` quand `unit` manque et que le fichier n'en porte pas ; une pente enchaînée par `step:k` sans `unit` s'affiche en mètres. Le plan ne dit pas comment l'unité se détermine : refuser sans `unit` quand `raster` est `step:k` (noyau, `check_plan`), ou autre lecture. Décision du propriétaire attendue |
| false | rejeté | `tickets.toml` : 26 repointé sur 12 et 25 retiré de 18 sont les modifications de l'abandon de 10.25 décidé par le propriétaire le 2026-11-20 via `bmad-ticket`, pas de ce ticket |
| false | rejeté | `couverture_reelle/phrases.json` CV03, CV21, CV22 reformulées : ces modifications ne viennent pas de 10.12 (le plan n'y touche que CV26) ; sans doute 10.8 (garde anti-fuite), à vérifier à la relecture de 10.8 |
| low | rejeté | S24 reste un refus : écart au Code Map mais conforme à la ligne Tasks « sinon refus » ; la valeur par défaut `m` serait fausse pour un raster de visibilité sans unité ; documenté dans Implementation Notes |

## Design Notes

Désignation du raster : l'amendement AD-24 du 2026-11-05 tranche la question laissée ouverte le 2026-11-04. Un `layer_id` de session ne se rejoue pas (relecture adversariale T4) ; un `RasterPath` se rejoue, y compris par `step:k` traduit en `out:<j>.1`. `layer.add{raster}` (ticket 20) ne sert donc qu'à afficher le raster, pas à désigner l'entrée.

Exemple de plan : `[{"command":"raster.zonal_statistics","params":{"raster":"dem/mnt.tif","layer_id":"communes","label_field":"nom"}}]`

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_zonal_statistics.py` -- expected: VERDICT OK
