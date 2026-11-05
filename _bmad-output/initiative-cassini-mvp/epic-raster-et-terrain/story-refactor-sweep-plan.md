---
title: 'Refactor sweep'
type: 'refactor'
ticket: '8'
created: '2026-11-05'
status: 'done'
baseline_revision: '62fa8581bdaf4d1d591caed25b34fd080fbe0290'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** L'epic 4 laisse sept constats ouverts dans `deferred-work.md` (sans `status:`), dont deux déjà réglés par des entrées plus récentes, et un doublon dans le code : les trois commandes de terrain recopient chacune la même fonction `failure(e: DemError) -> CommandFailure`. Le test des cellules rectangulaires du champ de vision, promis à 4.7, n'a pas été écrit : remplacer `min(cell_size_x, cell_size_y)` par `cell_size_x` ne casserait toujours aucun test.

**Approach:** Remplacer les trois copies par une seule conversion `From<DemError> for CommandFailure`, ajouter les tests de cellules rectangulaires (champ de vision et ligne de vue), puis écrire un `status:` et une `resolution:` sur chacun des sept constats ouverts de l'epic. Aucun changement de comportement.

</frozen-after-approval>

## Implementation Notes

Route courte : une centaine de lignes, deux fichiers de code touchés pour l'essentiel, un ajout de tests et un tri de constats.

**Code.**
- `core/src/commands/mod.rs` -- y poser `impl From<DemError> for CommandFailure` (code et message recopiés tels quels). Pas dans `dem.rs`, qui n'importe rien de `crate` et reste le module remplaçable d'AD-20 ; pas dans `registry.rs`, qui ne doit pas connaître le MNT.
- `core/src/commands/terrain_slope.rs:136`, `terrain_line_of_sight.rs:446`, `terrain_viewshed.rs:241` -- supprimer `failure`, `map_err(failure)` → `map_err(CommandFailure::from)` (5 appels). `refusal` (`terrain_line_of_sight.rs:179`) reste : autre rôle (codes propres à la commande).
- `core/src/commands/terrain_viewshed.rs` (tests) -- MNT plat à cellules rectangulaires, `cell_size_x = 20`, `cell_size_y = 10` : (a) un rayon de 5 100 m est refusé en `terrain.viewshed_too_large` (borne 500 × 10 m, prise sur la plus petite cellule) ; (b) un rayon accepté donne une fenêtre de largeur et de hauteur en cellules distinctes, `≈ 2r/20` colonnes et `≈ 2r/10` lignes, centrée sur l'observateur. Le test (a) doit échouer si l'on remplace `min` par `cell_size_x`.
- `core/src/commands/terrain_line_of_sight.rs` (tests) -- même MNT rectangulaire : `profile_step` vaut la plus petite cellule (10 m) sur une ligne courte.

**Tri de `deferred-work.md`** (entrées dont `source_plan` est sous `epic-raster-et-terrain/` et sans `status:`) — forme `status: <close|report>, 2026-11-05, entrée 4.8 (\`epic-raster-et-terrain/story-refactor-sweep-plan.md\`)` puis `resolution:` :
1. Limite de décodage `tiff` (4.1) → close : `MAX_DECODED_BYTES` = 1 Gio, `Limits` fixées dans `Dem::open`, refus annoncé (a378d094ff9e0c0c99e641477b71a34396b2630a, 4.2).
2. Sources raster relatives dans le générateur (4.3) → close : `add_dem` tire `donnees/`, `dem/`, `../donnees/` ou le nom seul (3dada665f617d58fd93ccba9f0b5e1dd75bcf84b, 4.6) ; modèle ré-entraîné en 5.41.
3. Cellules rectangulaires du champ de vision (4.5) → close : tests ajoutés ici.
4. Attendus de terrain de `plan_reel` (4.6) → close : `plan_reel.rs` mis à jour (a30bf8559597eaf4778d6440199f0911fd305b34, 5.41). `tests/epreuve-modele-local/sentences.json` et `reading_independent.json` ne sont pas réécrits : épreuve de l'étape 2, sur un catalogue factice qui définit lui-même `dem` en id de couche et `from`/`to` ; ce sont des pièces d'archive, leurs résultats y renvoient.
5. Extension et noyau de versions croisées (4.9) → report, liste de publication (étape 7).
6. Journal d'avant 4.9 qui diverge au rejeu (4.9) → report, décision du propriétaire avant la publication ; aucun journal d'utilisateur à ce jour.
7. Système écrit une fois dans le générateur (4.10) → report, prochain ré-entraînement groupé.

L'entrée du garde-fou (4.6, déjà `status: ticket` vers 4.10) n'est pas touchée.

**Fait.**
- Conversion `From<DemError> for CommandFailure` dans `core/src/commands/mod.rs` ; les trois `failure` supprimées, l'import `DemError` retiré des trois commandes.
- Tests `rectangular_cells_bound_on_the_smaller_and_crop_each_axis_apart` (champ de vision) et `rectangular_cells_sample_at_the_smaller_cell` (ligne de vue), ajoutés aussi à la liste `.tests(...)` de chaque commande. Contre-épreuve faite : avec `cell_size_x` à la place du `min`, les deux échouent ; code rendu.
- Surprise : `test_noyau_sans_qgis` (recette de l'extension) échouait déjà avant ce ticket, depuis c08517f (4.7) : le commentaire de l'exemple `comparer_terrain` dans `core/Cargo.toml` contient « QGIS », mot que la garde interdit dans le manifeste du noyau. L'exemple ne lie pas QGIS ; seul le commentaire est reformulé (« aux outils de référence, natif et GRASS »). Recette de l'extension : 379 tests, OK ; garde propre.
- Résolution de l'entrée 1 corrigée en cours de route : le refus d'un MNT trop grand est `dem.unsupported` « MNT trop grand », pas un code à part.
- Vérification Rust sous Windows (PowerShell depuis WSL, recette de 5.8) : WSL n'a pas les bibliothèques GTK de `/app`.

## Plan Change Log

## Review Triage Log

Passe 1 (`quick`) — high 0, medium 0, low 1, false 1, maybe-false 0.

- low, patch : le commentaire reformulé de `core/Cargo.toml:36` disait « à GRASS » alors que la pente se compare à `native:slope` (QGIS) ; vérifié dans `core/examples/comparer_terrain.rs:1`. Reformulé « aux outils de référence, natif et GRASS », majuscule d'origine rendue ; garde de l'extension toujours propre.
- false : « Plan Change Log vide alors que le plan a changé ». Le modèle réserve ce journal aux reprises de plan après relecture ; la correction de la résolution 1 est une note d'implémentation, consignée sous « Fait ».

## Verification

**Commands:**
- `cargo test --workspace` -- expected: tout passe (sous Windows, recette de 5.8 : `LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`)
- `cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement
- `cargo fmt --check` -- expected: rien à reformater
- `python -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` -- expected: tout passe, garde propre
- `cargo test -p cassini-training` (ou le paquet du générateur) -- expected: tout passe

**Manual checks:**
- `deferred-work.md` : aucune entrée de `epic-raster-et-terrain/` sans `status:`.
