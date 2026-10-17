---
title: 'Statistiques par zone'
type: 'feature'
ticket: '10'
created: '2026-10-16'
status: done
baseline_revision: '40517e95d47c8ee73fd5f52613a93a10bba15d58'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/regle-de-redaction-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** CAP-3 n'a ni les statistiques par zone ni leur preuve : « combien de gares par commune » n'est pas encore une commande.

**Approach:** une entrée de plus dans la table fermée de 9.8 : `analysis.zonal_statistics` compte, pour chaque entité d'une couche de zones (polygones), les entités d'une couche de points qu'elle contient. Le résultat est une couche `.gpkg` des zones avec un champ de décompte, plus une liste bornée par le schéma des décomptes par zone (AD-8).

## Boundaries & Constraints

**Always:** paramètres neutres et typés ; aucun nom d'algorithme côté noyau (AD-22) ; sortie `out` nommée par le noyau, `.gpkg`, couche ajoutée au projet ; tâche annulable ; entrées en lecture seule ; `layer_id` (zones) et `points_layer_id` en listes déroulantes (règle 11) ; liste `zones` avec `maxItems` ; exemples, tests et description FR/EN.

**Never:** pas d'expression ni de texte libre interprété par QGIS ; pas de statistiques autres que le décompte (somme, moyenne : plus tard) ; pas de statistiques de raster ; pas de reprojection par l'extension ; pas de modification de `ProcessingGesture` au-delà de ce que l'ajout d'une ligne de table et d'un résumé facultatif exige.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Décompte | zones polygones + points | couche des zones avec `count`, `zones` = décomptes | — |
| Zone sans point | une zone ne contient rien | `count` 0 pour cette zone | — |
| Même couche des deux côtés | `layer_id` = `points_layer_id` | — | `analysis.bad_params` |
| Couche inconnue ou raster | id absent / raster | — | `layer.not_found` / `layer.not_vector` |
| Trop de zones | plus de 100 zones | 100 premières dans `zones`, `truncated` vrai ; la couche les garde toutes | — |
| Zones non polygonales | `layer_id` est une couche de points | — | `analysis.bad_params` |

**Décision du propriétaire :** une zone est nommée dans `zones` par un champ de la couche de zones, `label_field` (facultatif, choisi dans une liste des champs de cette couche, règle 11), avec repli sur le numéro d'entité quand il est absent.

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_intersect.rs` -- gabarit (`Delegated`, deux couches, `output_params`, `.gpkg`, tests) ; `core/src/commands/mod.rs` -- ajouter le module.
- `core/src/contract.rs` -- vérifie `maxItems` des tableaux de géométries ; le contrôle d'ensemble couvrira la nouvelle commande.
- `qgis-plugin/cassini_qgis/analysis.py` -- table `ALGORITHMS` : une entrée `native:countpointsinpolygon` (clé d'entrée `POLYGONS`, points via `overlay`/`overlay_key` `POINTS`) ; `ProcessingGesture._load` rend `{layer_id, name, feature_count, crs}` : ajouter une clé facultative `summary` de la table, appelée après chargement, qui complète le résultat (lecture du champ de décompte, 100 zones au plus).
- `qgis-plugin/cassini_qgis/inventory.py` (`QgisBackend`) -- méthode de lecture des couples (étiquette, décompte) ; `form.py` -- `is_field_field` (nom `field`) existe, étendre à `label_field` si (b).
- `qgis-plugin/tests/test_analysis.py` (`FakeBackend`), `probe_intersect_select.py` -- gabarits des tests et de la sonde QGIS réelle.
- `docs/regle-de-redaction-des-commandes.md`, `qgis-plugin/README.md` -- recette manuelle.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_zonal_statistics.rs`, `mod.rs` -- déclarer la commande, sortie `{layer_id, name, feature_count, crs, zones[≤100], truncated}`, tests de schéma -- contrat noyau
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `inventory.py` -- entrée de table, vérification zones polygonales, résumé borné -- traduction dans l'extension
- [ ] `qgis-plugin/cassini_qgis/form.py`, `panel.py` -- listes déroulantes de couches (et de champ) -- règle 11
- [ ] `qgis-plugin/tests/test_analysis.py`, `probe_zonal_statistics.py` -- cas de la matrice avec doublure, puis sonde QGIS réelle -- preuve
- [ ] `qgis-plugin/README.md` -- recette manuelle

**Acceptance Criteria:**
- Given une couche de communes et `gares-2154.csv` ouverts, when `analysis.zonal_statistics` est lancé depuis le panneau avec deux listes de couches, then une couche de zones avec décompte apparaît et `zones` donne les comptes attendus.
- Given plus de 100 zones, when la commande finit, then `zones` n'en porte que 100 et `truncated` est vrai.
- Given le noyau seul, when `cargo test` tourne, then tout passe sans QGIS.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Collision avec un champ `NUMPOINTS` déjà présent dans la couche de zones (ex. sortie d'un run précédent réutilisée) | maybe-false (medium si vrai) | defer | Comportement de `native:countpointsinpolygon` face au nom en double non vérifié sous QGIS ; la matrice ne le couvre pas ; le correctif ajoute une garde ou un nom unique. |
| Pas de contrôle de géométrie sur `points_layer_id` | low | rejeté | QGIS refuse alors l'entrée et le geste rend `qgis.failed` : pas de mauvais décompte silencieux ; la matrice n'exige que les zones. Correctif = branche en plus. |
| Pas de test du mode facultatif de `FieldCombo` ni de `label_field=""` | low | rejeté | Widget Qt non testable hors QGIS dans cette recette ; le cas vide est géré par `or None` et `not params.get(...)`. |
| `mod.rs` non modifié | false | rejeté | `mod.rs` inclut `commands_mods.rs` généré par `build.rs` : le module est déclaré automatiquement, `cargo test` le confirme (143 tests). |
| Trois copies de la borne 100 (Rust, schéma, Python) | low | rejeté | Le test noyau lie constante et schéma ; la copie Python est testée par le cas des 150 zones. Pas de dérive concrète. |

## Design Notes

Le décompte vit dans un champ `NUMPOINTS` écrit par QGIS ; le noyau ne connaît pas ce nom, il lit `count` dans le résultat de l'extension.

## Verification

**Commands:**
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propre
- `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` -- expected: OK
- Sonde QGIS réelle sur `../donnees-test-cassini/` -- expected: décomptes égaux à ceux faits à la main

**Manual checks (if no CLI):**
- Dans QGIS, `analysis.zonal_statistics` : deux listes de couches, résultat dans le projet.
