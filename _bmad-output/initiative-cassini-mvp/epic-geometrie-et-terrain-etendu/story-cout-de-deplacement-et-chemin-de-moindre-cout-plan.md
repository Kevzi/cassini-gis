---
title: "Coût de déplacement et chemin de moindre coût"
type: 'feature'
ticket: '10'
created: '2026-11-06'
status: 'built'
baseline_revision: 'c25d12718aab3b190165b0be6b414cf0be560878'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le noyau ne sait pas encore dire combien coûte un déplacement sur un terrain, ni par où passer au moindre coût ; G10 de l'epic 11 l'exige, sans QGIS ni GPL.

**Approach:** Un module de coût unique du noyau (AD-20) calcule le coût cumulé aux 8 voisins ; `terrain.cost_distance` en écrit le raster, `terrain.least_cost_path` remonte du point d'arrivée au point de départ et écrit la ligne en GeoJSON (AD-30). Les identifiants, paramètres, résultats et erreurs sont ceux des deux fiches du catalogue, §6, sans écart.

## Boundaries & Constraints

**Always:** règles de coût d'AD-20 (coût par mètre ; NoData, nul ou négatif infranchissable ; pas = moyenne des deux cellules × taille de cellule, × √2 en diagonale) ; `total_cost` de la ligne = valeur du raster cumulé à la cellule d'arrivée ; sorties déterministes (mêmes entrées, mêmes octets) ; `sources` bornées de 1 à 32 par le schéma et la désérialisation (AD-8) ; aucun fichier écrit quand une erreur survient ; Decision (2026-11-06, propriétaire) : un pas en diagonale n'est permis que si les deux cellules orthogonales qu'il frôle sont franchissables, pour qu'un mur tracé en diagonale bloque ; raster de coût lu seulement par `Dem::open` (AD-20).

**Never:** aucune dépendance nouvelle ; pas de coût anisotrope, ni de pente calculée dans la commande, ni de paramètre propre à un usage (classes de véhicules, vitesses) ; pas de rééchantillonnage ; aucun code GRASS ou GPL lu ni porté ; pas de parallélisme.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Coût uniforme, même ligne | coût 1 partout, départ et arrivée sur une même rangée | ligne droite ; `total_cost` = `length` = distance entre les centres | — |
| Obstacle | mur de NoData ou de 0 avec une ouverture | la ligne passe par l'ouverture ; ses cellules sont toutes franchissables | — |
| Cumul multi-sources | 2 sources | chaque cellule vaut le minimum des cumuls de chaque source seule ; source = 0 | — |
| Inatteignable | zone franchissable enfermée par un mur | cellules à -9999, comptées dans `unreachable_cells` | `least_cost_path` vers elle : `terrain.no_path` |
| Source ou extrémité infranchissable | point sur NoData, 0 ou négatif | rien n'est écrit | `terrain.impassable_source`, nommée par son rang (« source n° 2 », « départ », « arrivée ») |
| Point hors emprise / autre système | — | rien n'est écrit | `terrain.outside_dem` / `terrain.crs_mismatch` (fonctions existantes) |
| Liste de sources | 0 ou 33 | refus de paramètres | message de la désérialisation |

</frozen-after-approval>

## Code Map

- `core/src/cost.rs` (nouveau, déclaré dans `core/src/lib.rs`) -- le module de coût unique : fonction pure `accumulate(cost: &Dem, sources: &[(u32, u32)]) -> Accumulated` (Dijkstra 8 voisins, tas binaire, clé totale `(coût, index)` pour l'ordre figé), avec le prédécesseur de chaque cellule ; `path(&Accumulated, end) -> Vec<(u32, u32)>`. Tests unitaires sur des `Dem` construits en mémoire.
- `core/src/commands/terrain_cost_distance.rs`, `core/src/commands/terrain_least_cost_path.rs` (nouveaux, découverts par `build.rs`) -- calqués sur `terrain_viewshed_cumulative.rs` (spec bilingue, `run`, `Scratch`, `fixture`, liste bornée `observers` l. 39-60, test « runs in the core and the contract accepts it », test d'octets identiques) : `.runs_in(RunsIn::Core)`, `.cancellable(false)`, `.path_params(&["cost"])`, `.output_params(&["out"])`, `.output_ext(".tif")` / `.output_ext(".geojson")`. Contrôle des points : `check_crs` et `check_extent` de `terrain_line_of_sight.rs` (l. 209-235), la cellule d'un point par `dem.extent()` et la taille de cellule. Sortie raster : `crate::horn::same_grid_as(cost, values, -9999.0)`. Ligne : `crate::geojson::write(path, epsg, &[Feature])` avec `Geometry::LineString`, propriétés `total_cost` et `length` en `Property::Number` ; `length` du résultat en `Distance` (`analysis_buffer.rs`). `From<GeojsonError>` existe déjà (`commands/mod.rs`).
- `core/src/dem.rs` -- `Dem` (`width`, `height`, `origin_x/y` coin haut-gauche, `cell_size_x/y`, `epsg`, `value(col,row)`, `extent()`) ; ne rien changer.
- `core/src/journal/tests.rs` -- test de plan rejoué, sur le modèle de `a_raster_then_an_export_replays_identically_in_a_new_session` (l. 1877) et des aides `with_dem`, `temp_project`, `Replay::load(..).run(..)`, sans QGIS : pente → reclassement → chemin de moindre coût, puis rejeu identique.
- `ai/src/plan.rs` l. 1026-1090 -- assertions du prompt : `cost` des deux commandes rendu en chaîne (`RasterPath` effacé), `sources` bornées 1-32, `out` caché.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` l. 272-273 (`planifiée` → `faite`), fiches de §6 déplacées en fin de §5 avec `"status": "faite"`, comme 11.9 ; `core/tests/catalogue.rs` confronte, rien à y changer.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/cost.rs` -- module et tests de la matrice (uniforme, obstacle, multi-sources = minimum des sources seules, inatteignable, diagonale = √2).
- [x] `core/src/commands/terrain_cost_distance.rs` -- commande, exemples, tests (bornes de `sources`, erreurs nommées par rang sans fichier écrit, comptes, octets identiques, contrat).
- [x] `core/src/commands/terrain_least_cost_path.rs` -- commande, exemples, tests (droite sur coût uniforme, contournement, `total_cost` = cumul à l'arrivée, `terrain.no_path`, GeoJSON 2008 à `crs` du raster, ligne du départ à l'arrivée, octets identiques, contrat).
- [x] `core/src/journal/tests.rs` -- plan pente → `raster.reclassify` → `terrain.least_cost_path`, rejoué à l'identique.
- [x] `ai/src/plan.rs` -- assertions du prompt pour les deux commandes.
- [x] `catalogue-des-commandes.md` -- fiches en `faite`, même commit.
- [ ] Poste du propriétaire -- installer noyau et extension (mémoire « Mettre à jour QGIS et le noyau »), dans QGIS 3.44 sur le MNT test en 2154 : pente, reclassement, coût cumulé, chemin de moindre coût, puis `journal.replay`.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when `cargo test --workspace`, then tout passe, dont `catalogue` avec les deux fiches `faite`.
- Given une session QGIS 3.44 avec l'extension, when pente, reclassement, coût cumulé et chemin de moindre coût s'exécutent, then le raster cumulé et la ligne s'ouvrent en EPSG:2154, la ligne relie départ et arrivée, et `journal.replay` rend `identical: true`.

## Implementation Notes

- `core/src/cost.rs` : Dijkstra 8 voisins, tas à clé totale `(coût, index)`, prédécesseur changé seulement sur un coût strictement plus petit. Pas en ligne = `cell_size_x`, en colonne = `cell_size_y`, en biais = `hypot(cell_size_x, cell_size_y)` (= taille × √2 sur cellule carrée). Une cellule de coût non fini est traitée comme infranchissable, comme NoData. Un pas en biais entre deux voisines infranchissables reste permis (AD-20 ne l'interdit pas) : un mur d'une cellule posé en diagonale laisse passer.
- `cell_of` et `locate` (système, emprise, franchissable) vivent dans `terrain_cost_distance.rs`, partagés par `terrain_least_cost_path.rs` ; les bords droit et bas de l'emprise appartiennent à la dernière cellule, comme `endpoint_elevation`.
- Départ et arrivée dans la même cellule : ligne de deux sommets confondus, `total_cost` et `length` nuls (une ligne GeoJSON demande deux positions).
- `max` est `Option<f64>` pour coller à la fiche (« nombre ou null ») ; il vaut toujours au moins 0, une source étant atteinte.
- Vérification : `cargo fmt --check` propre ; `cargo clippy -p cassini-core -p cassini-ai -p cassini -p cassini-training --all-targets -- -D warnings` propre et `cargo test` des mêmes paquets tout vert (catalogue et parité compris), lancés par `cargo` Windows (LIBCLANG_PATH, `CARGO_TARGET_DIR=C:\ct`) : sous WSL, `llama-cpp-sys-2` et la coquille Tauri (`app/src-tauri`) ne se compilent pas faute de libclang et de GTK. `app/src-tauri` non relu par clippy, non touché.
- Sonde QGIS 3.44 (dernière tâche) : non jouée.

## Plan Change Log

## Review Triage Log

Relecture rapide (quick), passage 1 : 0 high, 2 medium, 3 low, 0 false, 0 maybe-false.
- medium : sonde QGIS 3.44 non jouée (critère 2). Ce n'est pas un défaut du code : la tâche reste à faire. Claude joue la sonde avant la fusion.
- medium, patch après décision : passage en coin entre deux cellules infranchissables, qui laissait fuir un mur tracé en diagonale. Le propriétaire a choisi le 2026-11-06 de l'interdire (option A), et la décision est inscrite dans les Boundaries.
- low, patch : la mémoire du tas était sous-estimée (Dijkstra à suppression paresseuse, jusqu'à 8 entrées par cellule ; le tableau de sortie coexiste avec le cumul). La doc est corrigée.
- low, rejeté : le test de rejeu tourne sur un coût presque uniforme. Le critère « une pente reclassée sert de coût dans un plan qui se rejoue » est rempli, et le contournement est testé dans les modules.
- low, rejeté : « Le point source n° 2… » est grammaticalement raide, mais suit le style existant (« Le point observateur n° 2 » de `terrain.viewshed_cumulative`) ; le corriger toucherait aux messages communs.

## Design Notes

Ligne : sommets aux centres des cellules, de la cellule de départ à celle d'arrivée, les sommets intermédiaires alignés retirés ; `length` est la longueur de cette ligne. Coût cumulé en `f64`, écrit en Float32. Mémoire : 12 octets par cellule (cumul `f64`, prédécesseur `u32`) plus le tas, dit dans la doc de la commande comme pour `terrain.peaks` ; aucune borne nouvelle, la lecture est déjà bornée à 1 Gio. Une source répétée ne change rien.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement
- `cargo test --workspace` -- expected: tout vert

**Manual checks:**
- Sonde QGIS 3.44 de la dernière tâche, résultats consignés dans Implementation Notes.
