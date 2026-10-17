---
title: 'Zone tampon par le cadre de traitement de QGIS'
type: 'feature'
ticket: '8'
created: '2026-10-16'
status: done
baseline_revision: '57e3351fda428cd1bd374580d98aa607f546db87'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/regle-de-redaction-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** CAP-3 n'a aucune analyse : aucune commande ne produit encore une couche, et chaque analyse future (intersection, sélection, statistiques, isochrones) devrait réinventer l'appel au cadre de traitement, le nommage de la sortie et le résultat.

**Approach:** décision du propriétaire (option C). Un **exécuteur générique** de l'extension lance un algorithme du cadre de traitement de QGIS depuis une table de traduction ; le noyau garde une liste choisie de commandes à noms neutres (AD-22), `analysis.buffer` d'abord. Ajouter une analyse = une déclaration Rust courte + une ligne de la table, sans nouvelle logique. Le tampon pose ce gabarit : couche d'entrée choisie dans une liste, sortie nommée par le noyau (AD-10), couche de sortie ajoutée au projet, nombre d'entités rendu.

## Boundaries & Constraints

**Always:** paramètres neutres et typés : `layer_id`, `distance {value, unit}` avec `unit` ∈ `m`, `km`, `dissolve` booléen facultatif ; jamais `native:buffer` côté noyau (AD-22). Le noyau nomme la sortie (paramètre `out` rempli par lui, comme `test.write_output`) ; l'extension écrit exactement ce fichier et ne l'invente pas. Un fichier de sortie de type couche porte l'extension `.gpkg` : ajouter au registre une extension de sortie déclarée par la commande (défaut `out`, comportement actuel inchangé) utilisée par le nommage et le nettoyage d'annulation. Le travail avance en tâche QGIS annulable (`QgsProcessingAlgRunnerTask`), jamais en bloquant l'interface. Lecture seule de l'entrée. Hors ligne. Le formulaire du panneau masque les paramètres remplis par le noyau (`out`).

**Never:** pas de reprojection par l'extension ; pas de chemin ni de nom de sortie saisis par l'utilisateur ; pas d'exécution d'un algorithme choisi par le modèle (la table est fermée, AD-23) ; pas de réécriture de `layer.add`, du journal ni du canal ; pas d'autres analyses dans cette entrée.

</frozen-after-approval>

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Tampon de points | 8 points en EPSG:2154, 500 m | couche `.gpkg` dans le dossier de travail, ajoutée au projet ; `feature_count` 8 ; `crs` EPSG:2154 | — |
| Distance en km | `{value: 0.5, unit: "km"}` | identique à 500 m | — |
| Fusion | `dissolve: true` | `feature_count` 1 | — |
| Couche inconnue | `layer_id` absent du projet | — | `layer.not_found` |
| SCR en degrés | couche en EPSG:4326 | — | `analysis.crs_not_metric` (dit de reprojeter la couche) |
| Distance invalide | `value` ≤ 0 | — | refusée par le noyau avant l'ordre |
| Annulation | arrêt en cours | tâche annulée, aucun fichier partiel | `qgis.cancelled`, sortie retirée par le noyau |
| Raster | `layer_id` d'un raster | — | `layer.not_vector` |

## Code Map

- `core/src/commands/layer_add.rs`, `core/src/commands/layer_describe.rs` -- modèles de commande déléguée (`Delegated`, `.tests`, `inventory::submit!`).
- `core/src/commands/test_commands.rs` (`WriteOutputParams`) -- modèle du paramètre `out` rempli par le noyau (`#[serde(default)]`, `.output_params`).
- `core/src/registry.rs` (`CommandSpec`, ~l.40-115) -- ajouter `output_ext` (`#[serde(skip)]`, défaut `out`) et son setter.
- `core/src/journal.rs` (`output_file`, ~l.318, appelé en l.243, 355, 587) -- utiliser l'extension déclarée ; ne pas changer le format du journal.
- `qgis-plugin/cassini_qgis/inventory.py` -- modèle de geste (fond `QgisBackend` injectable, `_ReadGesture`, `GestureError`) ; ne pas le modifier. `plugin.py:150` -- enregistrement des gestes.
- `qgis-plugin/cassini_qgis/form.py`, `panel.py` (`_build_form`, `LayerCombo`) -- masquer `out`.
- `docs/regle-de-redaction-des-commandes.md` -- règle 6 à nuancer (le paramètre `out`), règle 11.
- Fait vérifié sur QGIS 3.44.15 : avec un chemin se terminant par `.out`, le cadre de traitement écrit `<chemin>.out.gpkg` (pas `<chemin>`) ; le nommage `.gpkg` évite ce décalage et le fichier orphelin à l'annulation.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/registry.rs`, `core/src/journal.rs` -- `output_ext` déclarable par commande, utilisé par `output_file` -- sortie `.gpkg` sans casser les sorties existantes
- [ ] `core/src/commands/analysis_buffer.rs` -- déclarer `analysis.buffer` (exemples, tests, `output_ext(".gpkg")`) -- gabarit du noyau
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- exécuteur générique (`ProcessingGesture`, table `ALGORITHMS` : commande → algorithme + traduction des paramètres, sortie), fond `QgisBackend` isolé -- seul endroit touchant `qgis.core`/`processing`
- [ ] `qgis-plugin/cassini_qgis/plugin.py` -- enregistrer les commandes de la table
- [ ] `qgis-plugin/cassini_qgis/form.py`, `panel.py` -- masquer les champs remplis par le noyau
- [ ] `qgis-plugin/tests/test_analysis.py`, tests Rust -- cas de la matrice avec doublure ; test que ajouter une ligne de table suffit
- [ ] `docs/regle-de-redaction-des-commandes.md`, `qgis-plugin/README.md` -- règle `out`, recette manuelle

**Acceptance Criteria:**
- Given 8 points de test en EPSG:2154, when `analysis.buffer` est lancé à 500 m depuis le panneau (couche choisie dans la liste), then une couche de 8 entités apparaît dans le projet et le fichier est dans `<projet>.cassini/`, sans réseau.
- Given la commande lancée puis annulée, when le noyau répond `registry.cancelled`, then aucun fichier de sortie ne reste dans le dossier de travail.
- Given le noyau seul, when `cargo test` tourne, then tout passe sans QGIS.

## Design Notes

Le gabarit : une entrée de table `{algorithm: "native:buffer", params(translate), output}` + `ProcessingGesture(entry)` qui lance `QgsProcessingAlgRunnerTask`, charge la sortie et rend `{layer_id, name, feature_count, crs}`. La couche porte comme nom le radical du fichier fourni par le noyau (`1-analysis.buffer`) : AD-10 interdit à l'extension d'en inventer un ; un nom plus lisible est une amélioration ultérieure. L'entrée `layer_id` est une valeur de session : rejouer un tampon d'une couche ouverte par `layer.add` demande de lier les repères de sortie aux couches (AD-11) ; cela relève de l'entrée 9.14 et n'est pas résolu ici.

## Verification

**Commands:**
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propre
- `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` -- expected: OK
- Sonde QGIS réelle (`python-qgis-ltr.bat`) sur `../donnees-test-cassini/gares-2154.csv` -- expected: 8 entités à 500 m, fichier `.gpkg` créé

**Manual checks (if no CLI):**
- Dans QGIS, `analysis.buffer` : liste déroulante des couches, aucun champ `out`, couche affichée autour des gares.

## Review Triage Log

| # | Finding | Verdict | Route | Evidence |
|---|---------|---------|-------|----------|
| 1 | Annulation : fichier verrouillé par le fil de la tâche sous Windows | maybe-false (medium) | defer | À régler par une annulation réelle dans QGIS. |
| 2 | `cancel()` après réponse supprime la sortie livrée | medium | patch | Garde `_done is None` ajoutée + test. |
| 3 | Échec après `addMapLayer` supprime le fichier | medium | patch | `_loaded` + `_fail` + test. |
| 4 | Rejeu : octets du `.gpkg` et `layer_id` de session | medium | defer | Relève de 9.14 (déjà noté au plan). |
| 5 | Pas de test du noyau sur `output_file` en `.gpkg` | low | patch | Test `output_file_uses_the_declared_extension`. |
| 6 | `QgsUnitTypes.DistanceUnit` absent en QGIS 4 | medium | patch | `Qgis.DistanceUnit.Meters`. |
| 7 | `panel.py` non modifié | false | — | Le panneau passe par `build_fields`, qui masque `out`. |
| 8 | Schéma `minimum: 0` | low | rejeté | Le noyau refuse 0 ; correction non directe. |
