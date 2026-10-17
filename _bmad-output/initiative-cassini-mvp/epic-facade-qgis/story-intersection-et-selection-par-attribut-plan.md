---
title: 'Intersection et sélection par attribut'
type: 'feature'
ticket: '9'
created: '2026-10-16'
status: done
baseline_revision: 'e5dcb0c54786785ce2077511e6ba7610020e72b1'
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

**Problem:** CAP-3 n'a que la zone tampon. Il manque l'intersection de deux couches et le filtre par attribut, dont la phrase de démo a besoin (« zones constructibles », « parcelles dans la zone »).

**Approach:** deux entrées de plus dans la table fermée de l'exécuteur de 9.8 : `analysis.intersect` et `analysis.select_by_attribute`. Le filtre se compose d'un champ, d'un opérateur et d'une valeur, jamais d'une expression (AD-22). Le champ se choisit dans une liste des champs de la couche choisie (règle 11).

## Boundaries & Constraints

**Always:** paramètres neutres et typés ; aucun nom d'algorithme ni expression côté noyau ; sortie `out` nommée par le noyau, `.gpkg`, couche ajoutée au projet, `{layer_id, name, feature_count, crs}` rendu ; tâche annulable ; entrées en lecture seule ; `layer_id` et `overlay_layer_id` en listes déroulantes ; chaque commande avec exemples, tests et description FR/EN.

**Never:** pas d'expression QGIS ni de texte libre interprété par QGIS ; pas de chemin saisi ; pas de reprojection par l'extension ; pas de modification de `ProcessingGesture` au-delà de ce que l'ajout d'une ligne de table exige ; pas d'autres analyses (statistiques, isochrones).

**Décisions du propriétaire :** `select_by_attribute` produit une **nouvelle couche** `.gpkg` des entités retenues (pas une sélection sur l'entrée) ; opérateurs : `eq, ne, lt, lte, gt, gte, contains, begins_with`, tous avec valeur (pas de `is_null` pour l'instant) ; `value` est toujours une **chaîne**, que QGIS convertit selon le type du champ.

</frozen-after-approval>

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Intersection | deux couches vecteur du projet | couche `.gpkg` des parties communes, ajoutée au projet | — |
| Même couche des deux côtés | `layer_id` = `overlay_layer_id` | — | `analysis.bad_params` |
| Overlay inconnu ou raster | id absent / raster | — | `layer.not_found` / `layer.not_vector` |
| Filtre égal | `field: "zone"`, `eq`, `"U"` | couche des entités où zone = U | — |
| Filtre numérique | `field: "pop"`, `gt`, `"1000"` | entités où pop > 1000 | — |
| Aucune entité retenue | filtre sans résultat | couche vide, `feature_count` 0 | — |
| Champ inconnu | `field` absent de la couche | — | `analysis.unknown_field` |
| Opérateur inconnu | hors liste | — | refusé par le noyau (schéma) |

## Code Map

- `core/src/commands/analysis_buffer.rs` -- gabarit à copier (`Delegated`, `output_params`, `output_ext(".gpkg")`, tests) ; `core/src/commands/mod.rs` -- ajouter les deux modules.
- `qgis-plugin/cassini_qgis/analysis.py` -- table `ALGORITHMS` (+ fonctions `translate_*`) ; `ProcessingGesture._begin` ne valide que `input` : prévoir `overlay` et le champ via des clés facultatives de l'entrée de table, sans changer le déroulé.
- `qgis-plugin/cassini_qgis/form.py` (`is_layer_field`), `panel.py` (`_build_form`, `LayerCombo`) -- ajouter `is_field_field` (nom `field`) et `FieldCombo` qui relit les champs de la couche choisie ; `inventory.py` (`QgisBackend`) pour lister les champs.
- `docs/regle-de-redaction-des-commandes.md` règle 11, `qgis-plugin/README.md` -- nommage `field`, recette manuelle.
- `core/src/contract.rs` -- contrôle le contrat des nouvelles commandes.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_intersect.rs`, `analysis_select_by_attribute.rs`, `mod.rs` -- déclarer les deux commandes, tests de schéma -- contrat noyau
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- deux entrées de table (`native:intersection`, `native:extractbyattribute`), vérification du champ -- traduction dans l'extension
- [ ] `qgis-plugin/cassini_qgis/form.py`, `panel.py`, `inventory.py` -- `FieldCombo` des champs de la couche choisie -- règle 11
- [ ] `qgis-plugin/tests/test_analysis.py`, `test_panel_logic.py`, tests Rust -- cas de la matrice avec doublure
- [ ] `docs/regle-de-redaction-des-commandes.md`, `qgis-plugin/README.md` -- règle et recette

**Acceptance Criteria:**
- Given `gares-2154.csv` ouvert et sa zone tampon, when `analysis.intersect` est lancé depuis le panneau avec deux listes de couches, then une couche de parties communes apparaît dans le projet.
- Given une couche à champs, when `analysis.select_by_attribute` est choisi, then le champ se choisit dans une liste, l'opérateur dans une liste, et la couche des entités retenues apparaît.
- Given le noyau seul, when `cargo test` tourne, then tout passe sans QGIS.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| # | Finding | Verdict | Route | Evidence |
|---|---------|---------|-------|----------|
| 1 | `value` sans indice `values` (règle 4) | low | rejeté | `value` est une donnée de l'utilisateur, pas une catégorie fermée ; les exemples montrent des valeurs. |
| 2 | `FieldCombo` sans test Qt | medium | defer | Le panneau Qt n'a pas de harnais de test ici ; la recette manuelle du README le couvre. |
| 3 | `FieldCombo` exige un `layer_id` exact | low | rejeté | Les deux commandes l'ont ; élargir ajoute de la logique sans cas réel. |
| 4 | Liste de champs vide sans entrée vide | low | rejeté | Donne `analysis.unknown_field`, message clair. |
| 5 | Comparaison numérique sur texte : `analysis.failed` générique | low | rejeté | Hors matrice ; corriger ajoute une garde. |
| 6 | Sonde : débogage et `len(names) and 1` | low | patch | Battement, `print` et expression retirés. |

## Design Notes

`select_by_attribute` ne laisse jamais le texte de l'utilisateur dans une expression : la traduction vers l'algorithme d'extraction par attribut passe champ, code d'opérateur et valeur comme paramètres séparés.

## Verification

**Commands:**
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propre
- `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` -- expected: OK
- Sonde QGIS réelle sur `../donnees-test-cassini/` -- expected: intersection et filtre rendent le nombre d'entités attendu

**Manual checks (if no CLI):**
- Dans QGIS, `analysis.select_by_attribute` : listes déroulantes de couche, de champ et d'opérateur.
