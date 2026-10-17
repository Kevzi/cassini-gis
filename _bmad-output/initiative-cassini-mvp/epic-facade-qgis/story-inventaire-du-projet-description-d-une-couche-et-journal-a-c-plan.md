---
title: 'Inventaire du projet, description d une couche et journal a cote du projet'
type: 'feature'
ticket: '7'
created: '2026-10-16'
status: done
baseline_revision: 'cf04edd04d47e0076b59da2c8e25899b573288b0'
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

**Problem:** CAP-10 n'est pas servi : rien ne dresse l'inventaire du projet ni ne décrit une couche, et la portabilité « projet + journal déplacés ensemble » n'est prouvée par aucun test.

**Approach:** deux commandes déléguées à QGIS, `project.inventory` (couches, ordre, étendue, SCR) et `layer.describe` (champs, étendue, SCR), déclarées dans le noyau, exécutées par deux gestes de l'extension sur le modèle de `layer.add`. Un test du noyau copie le projet et son dossier `.cassini/` dans un autre dossier et rejoue le journal.

## Boundaries & Constraints

**Always:** lecture seule (aucune couche, aucun fichier modifié) ; SCR en code explicite (AD-19) ; une couche se désigne par son id QGIS, valable pour la session seulement (AD-9) ; aucun chemin absolu dans les résultats ni les messages : une source de couche est rendue par son nom de fichier ; étendue = `{xmin, ymin, xmax, ymax, crs}` ; descriptions fr et en, au moins un test par commande ; aucune logique métier dans le Python.

**Never:** pas de reprojection par le noyau ; pas de nouveau paramètre de chemin ; pas de modification du format du journal ni de `Session::open` (le journal est déjà dans `<projet>.cassini/`) ; pas d'inventaire des styles ni des données.

</frozen-after-approval>

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Inventaire | projet avec 2 couches | `layers` dans l'ordre de la légende (haut en bas), `project_crs`, `extent` de l'ensemble | — |
| Projet vide | aucune couche | `layers: []`, `extent: null` | pas d'erreur |
| Description | `layer_id` d'une couche vecteur | `fields[{name,type}]`, `extent`, `crs`, `feature_count`, `geometry` | — |
| Couche inconnue | `layer_id` absent du projet | — | `layer.not_found` |
| Couche sans étendue | couche vide | `extent: null` | pas d'erreur |

## Code Map

- `core/src/commands/layer_add.rs` -- modèle de déclaration déléguée (`Delegated`, `runs_in(Qgis)`, `.tests`, `inventory::submit!`) ; les fichiers de `commands/` sont déclarés par `build.rs`, rien d'autre à toucher.
- `qgis-plugin/cassini_qgis/layer_add.py` -- modèle de geste (fond `QgisBackend` injectable, planificateur, `GestureError`) ; ne pas le modifier.
- `qgis-plugin/cassini_qgis/plugin.py:150-152` -- enregistrement des gestes dans `GestureRegistry`.
- `qgis-plugin/tests/test_layer_add.py` -- modèle de test avec doublure de fond.
- `core/src/journal.rs` -- `Session::open`/`locate` (dossier `.cassini/`), `Replay::load`/`run`, `temp_project()` (~l.728) et `replay_of()` (~l.950) pour le test de portabilité.
- `qgis-plugin/README.md` -- recette manuelle (§ layer.add, l.45).

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/project_inventory.rs` -- déclarer `project.inventory` (paramètres vides) et son résultat -- CAP-10
- [ ] `core/src/commands/layer_describe.rs` -- déclarer `layer.describe` (`layer_id`) et son résultat -- CAP-10
- [ ] `qgis-plugin/cassini_qgis/inventory.py` -- gestes `ProjectInventoryGesture` et `LayerDescribeGesture`, fond `QgisBackend` isolé -- seul endroit qui touche `qgis.core`
- [ ] `qgis-plugin/cassini_qgis/plugin.py` -- enregistrer les deux gestes
- [ ] `qgis-plugin/tests/test_inventory.py` -- tests des deux gestes avec doublure (cas de la matrice)
- [ ] `core/src/journal.rs` -- test : projet + `.cassini/` copiés ailleurs, `Replay` identique, journal sans chemin absolu
- [ ] `qgis-plugin/README.md` -- recette manuelle des deux commandes

**Acceptance Criteria:**
- Given un projet de test à deux couches, when `project.inventory` est lancé depuis le panneau, then le résultat liste les couches dans l'ordre, le SCR du projet et l'étendue.
- Given le projet copié avec son dossier `.cassini/` dans un autre dossier, when le journal est rejoué, then le rejeu est identique.
- Given le noyau seul, when `cargo test` et la recette de l'extension tournent, then tout passe sans QGIS.

## Design Notes

`layer.describe` prend un `layer_id` de session : l'id n'est jamais une clé de rejeu (AD-9). Une étape qui en porte un sera donc rejouée en pratique après un `layer.add` de la même séquence ; ce raccord (référence entre étapes) relève de l'epic agent et n'est pas résolu ici.

## Verification

**Commands:**
- `cargo test` -- expected: tout passe, dont le test de portabilité
- `cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `python qgis-plugin/tests/test_inventory.py` et `python qgis-plugin/tests/check_extension.py` -- expected: OK

**Manual checks (if no CLI):**
- Dans QGIS 3.44 avec `../donnees-test-cassini/`, lancer `project.inventory` puis `layer.describe` sur une couche : résultat cohérent avec la table d'attributs.

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch (fait) | `isEmpty()` rendait `extent: null` pour une couche d'un seul point ; seul `isNull()` est testé. |
| 2 | low | rejete | SCR sans `authid` (SCR personnalisé) : rare, la garde demanderait une nouvelle forme de résultat. |
| 3 | medium | patch (fait) | `source_name` laissait passer une chaîne PostGIS (identifiants) ou une URI mémoire ; seules les sources fichier avec extension sont rendues, le reste est `null`. Test ajouté. |
| 4 | low | rejete | Nom de fichier contenant `|` ou `?` tronqué : improbable. |
| 5 | low | patch (fait) | `featureCount()` = -1 ne tient pas dans un `u64` : ramené à 0. |
| 6 | low | patch (fait) | `geometryDisplayString` rend `Null` : ramené à `NoGeometry`, valeur documentée. |
| 7 | low | rejete | Couches cassées sautées de l'inventaire : hors matrice, comportement acceptable. |
| 8 | low | rejete | Codes d'erreur non listés dans la spec : `layer.add` ne les liste pas non plus. |
| 9 | false | rejete | `layer.describe` hors du test de portabilité : voulu, le `layer_id` n'est pas une clé de rejeu (Design Notes). |
| 10 | low | rejete | Dossier `p.cassini` en dur dans le test : `temp_project()` fixe le nom `p.qgz`. |
