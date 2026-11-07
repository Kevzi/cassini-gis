---
title: 'Reclassement et superposition pondérée de rasters'
type: 'feature'
ticket: '9'
created: '2026-11-06'
status: done
baseline_revision: '2d7e9e40021ab9ccc0463cabb7722ff90120a1db'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** les fiches `raster.reclassify` et `raster.weighted_overlay` du catalogue (planifiées, entrée 9) n'ont pas d'implémentation ; sans elles, aucune analyse multicritère (pente reclassée, combinée à d'autres critères) n'est possible dans le noyau, et l'entrée 10 n'a pas de raster de coût reclassé.

**Approach:** deux commandes du noyau, lues et écrites par `dem.rs` (AD-20), un contrôle de grilles identiques qui rend `raster.grid_mismatch` en nommant l'écart, le NoData propagé ; et, parce que `criteria` est une liste d'objets `{raster, weight}`, la résolution des chemins du noyau (journal, appel externe, contrat) apprend à atteindre un chemin rangé dans les éléments d'une liste.

## Boundaries & Constraints

**Always:** fiches du catalogue à la lettre (noms, bornes : 1 à 32 classes, 2 à 8 critères, 0 à 8 exclusions, `weight` > 0 et fini ; résultats `file`, `counts`, `nodata_cells` / `file`, `min`, `max`, `nodata_cells`) ; classe `[min, max[`, borne absente ouverte, première classe qui convient, hors classe → NoData, NoData reste NoData ; `min ≥ max` → `raster.invalid_class` nommant le rang (1-based) ; `counts` : une entrée par valeur de sortie distincte, dans l'ordre de première apparition dans les classes, zéro compris ; score = Σ(v × w) / Σw en f64, rendu f32 ; critère NoData → NoData ; masque : cellule valide non nulle → NoData, masque NoData n'exclut pas ; grilles identiques au sens d'AD-20 (EPSG et dimensions égaux, origine et cellule à moins d'un millionième de cellule), contrôlées sur tous les critères et masques contre le premier critère, avant tout calcul ; sorties GeoTIFF Float32 sans-donnée -9999, même grille que l'entrée ; deux exécutions → mêmes octets ; erreurs de lecture par `DemError` telles quelles ; aucune dépendance nouvelle.

**Never:** aucun rééchantillonnage ni reprojection (AD-19, AD-20) ; aucun autre module n'ouvre un raster ; pas d'extension de la grammaire `step:k.champ`, ni de `step:k` dans les éléments d'une liste ; aucun changement de l'extension QGIS, de l'entraînement ni du générateur (entrées 12 et 16) ; pas de seuil métier par défaut dans les classes.

**Decision (2026-11-06, propriétaire) :** plan gardé entier malgré sa taille (un seul livrable).

**Decision (2026-11-06, propriétaire) :** `step:k` dans les éléments de `criteria` et de `exclusions` est renvoyé à une nouvelle entrée, placée avant le ré-entraînement (12), notée dans `deferred-work.md`.

**Decision (2026-11-06, propriétaire) :** une classe de `value` -9999 (le sans-donnée écrit) est refusée par `raster.invalid_class`, qui nomme son rang.

**Decision (2026-11-06, propriétaire, à la relecture) :** exception à « aucun changement de l'entraînement » : le mot `reclass` est ôté du garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry` (`training/src/families.rs`), rien d'autre ; le refus « seuil sur la pente » du générateur reste jusqu'au ré-entraînement (12).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Pente en 3 classes | pente de `mnt-colline-2154.tif` (calculée en test), classes `[,5[→1`, `[5,15[→2`, `[15,[→3` | comptes égaux à ceux comptés directement sur la pente ; bord NoData compté dans `nodata_cells` | — |
| Hors classes | classes ne couvrant pas toutes les valeurs | cellules non couvertes en NoData, comptées | — |
| Classe vide | classe sans cellule | entrée `{value, cells: 0}` | — |
| Classe invalide | rang 2 avec `min = max` ; rang 3 de `value` -9999 | rien d'écrit | `raster.invalid_class`, « classe 2 », « classe 3 » |
| Bornes | 0 ou 33 classes ; 1 ou 9 critères ; 9 masques ; poids 0 ou négatif | rien d'écrit | erreur de paramètres du registre |
| Deux critères | grilles synthétiques 3 × 3, poids 1 et 3 | score attendu cellule par cellule, `min`/`max` justes | — |
| NoData et masque | un trou dans un critère, un masque non nul ailleurs | NoData aux deux cellules, `nodata_cells` 2 | — |
| Tout exclu | masque partout non nul | `min`/`max` à `null`, fichier écrit | — |
| Grilles décalées | second critère à origine décalée d'une demi-cellule ; puis dimensions, cellule, EPSG différents | rien d'écrit | `raster.grid_mismatch`, message nommant le raster et l'écart (grandeur et valeurs) |
| Chemin imbriqué | appel par le journal avec `criteria[i].raster` relatif au projet | journal garde le relatif, l'exécution reçoit l'absolu, `inputs` les liste, rejeu identique | chemin qui sort du projet refusé par l'appel externe (`external.path_refused`) |

</frozen-after-approval>

## Code Map

- `core/src/commands/raster_reclassify.rs`, `core/src/commands/raster_weighted_overlay.rs` (nouveaux, découverts par `build.rs`) -- calqués sur `terrain_slope.rs` (spec, `run`, `Scratch`, `fixture`, test « court dans le noyau et le contrat l'accepte », test d'octets identiques) : `.runs_in(RunsIn::Core)`, `.cancellable(false)`, `.output_params(&["out"])`, `.output_ext(".tif")`. Listes bornées : `deserialize_with` + `#[schemars(length(...))]` comme `observers` (`terrain_viewshed_cumulative.rs` l. 39-60). Grille de sortie par `crate::horn::same_grid_as(dem, values, -9999.0)`. Fonctions pures `reclassify(&Dem, &[Class])` et `overlay(&[(&Dem, f64)], &[&Dem])` testables sans fichier ; grilles de test construites en `Dem { .. }` puis `write_geotiff`.
- `core/src/dem.rs` -- ajouter `Dem::grid_mismatch(&self, other: &Dem) -> Option<String>` (pure, décrit le premier écart en français, tolérance AD-20) avec ses tests ; ne rien changer d'autre. `Dem::open` refuse déjà un raster géographique (`dem.not_projected`), gardé tel quel.
- `core/src/registry.rs` l. 125 (`path_params`) -- documenter la forme `liste.champ` : le champ `champ` de chaque élément de la liste `liste`.
- `core/src/journal/mod.rs` l. 659-700 (`resolve`, `resolve_param`) -- pour un nom `a.b`, appliquer `resolve_param` au champ `b` de chaque objet du tableau `a` (logged et exec en parallèle) ; `check_inputs` n'a rien à changer (il lit `inputs`).
- `core/src/serve/calls.rs` l. 71-80 -- même descente pour `escapes_project` sur un nom `a.b`.
- `core/src/contract.rs` l. 160-185 (`check_path_params`) -- pour `a.b`, chercher `b` dans le schéma des éléments de `a` (suivre `items` et `$ref`), et exiger qu'il accepte une chaîne ; test de contrat ajouté (nom imbriqué valide, nom imbriqué absent refusé).
- `ai/src/plan.rs` l. 1027-1050 -- vérifier que `raster` de `criteria` et les éléments de `exclusions` sont rendus en chaîne (`inline_raster_paths` descend déjà partout), et que `criteria` (2-8), `exclusions` (≤ 8), `classes` (1-32) sont bornés pour le modèle.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` l. 270-271 (`planifiée` → `faite`), fiches l. 1066-1128 déplacées en fin de §5 avec `"status": "faite"` (comme 11.15) ; `core/tests/catalogue.rs` confronte, rien à y changer.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/dem.rs` -- `grid_mismatch` et tests (égales, origine, cellule, dimensions, EPSG, écart sous la tolérance accepté).
- [x] `core/src/registry.rs`, `core/src/contract.rs`, `core/src/journal/mod.rs`, `core/src/serve/calls.rs` -- noms de chemin `liste.champ`, avec tests (journal : relatif journalisé, absolu exécuté, rejeu ; appel externe : chemin sortant refusé).
- [x] `core/src/commands/raster_reclassify.rs` -- commande et tests de sa partie de la matrice ; `path_params(&["raster"])`.
- [x] `core/src/commands/raster_weighted_overlay.rs` -- commande et tests ; `path_params(&["criteria.raster", "exclusions"])`.
- [x] `ai/src/plan.rs` -- assertions du prompt pour les deux commandes.
- [x] `catalogue-des-commandes.md` -- fiches en `faite`, même commit.
- [x] Poste du propriétaire -- installer noyau et extension (mémoire « Mettre à jour QGIS et le noyau »), dans QGIS 3.44 : pente du MNT test, reclassement, superposition de deux critères, puis `journal.replay`.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when `cargo test --workspace`, then tout passe, dont `catalogue` avec les deux fiches `faite`.
- Given une session QGIS 3.44 avec l'extension, when une pente puis son reclassement puis une superposition de deux critères s'exécutent, then deux GeoTIFF en EPSG:2154 s'ouvrent comme couches, et `journal.replay` rend `identical: true`.

## Implementation Notes

- `Dem::grid_mismatch` contrôle, dans l'ordre, système, dimensions, taille de cellule, origine ; tolérance d'un millionième de la cellule de la grille de référence. Nombres des messages arrondis au millionième, virgule décimale.
- `raster.reclassify` refuse la table (`raster.invalid_class`) avant d'ouvrir le raster. `counts` regroupe les valeurs égales une fois rendues en f32 (la valeur écrite), sous la valeur de la première classe.
- `raster.weighted_overlay` : `exclusions` est un `Vec<RasterPath>` à `#[serde(default)]` (jamais `null`) ; `min`/`max` sont sérialisés `null` quand aucune cellule n'est valide (pas de `skip_serializing_if`).
- Exemples des deux commandes écrits sur les MNT de `/fixtures` : la recette de parité (`cli/tests/parity.rs`) exécute chaque exemple d'une commande du noyau.
- Écart à la frontière « aucun changement de l'entraînement » : le garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry` (`training/src/families.rs`) échouait à l'arrivée de `raster.reclassify` (mot `reclass`) ; seul ce mot en est ôté, le générateur et ses exemples sont intacts, et le refus « seuil sur la pente » devenu faux est noté dans `deferred-work.md` pour l'entrée 12. Confirmé par le propriétaire à la relecture (Decision ci-dessus).
- Vérifié sous Windows (`C:\ct`) : `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace` verts. `canal_externe::a_relay_that_stops_reading_does_not_hold_the_switch_off` a échoué une fois sur délai (1,5 s), vert au passage suivant.
- Installé (2026-11-06) : noyau `--features gpu` dans `%LOCALAPPDATA%\Cassini\bin` (ancien en `cassini.exe.avant-11-9`), extension dans le profil. Sonde QGIS 3.44.15 (offscreen, noyau installé, copie `projet-sonde-11-9.qgz`, sonde `donnees-test-cassini/sonde_reclass_overlay_11_9.py`) : pente en degrés du MNT de Villeneuve-le-Roi (783 × 712), reclassement en 3 classes (488 594 / 57 594 / 538 cellules, 10 770 NoData, total = grille), superposition pente × 1 + reclassement × 2 (min 0,667, max 18,42) ; trois couches raster en EPSG:2154 ; journal à chemins relatifs dans `criteria` ; rejeu `identical: true` (6 étapes), mêmes couches.

## Plan Change Log

## Review Triage Log

Relecture `thorough`, itération 0 (blind-hunter, edge-case-hunter, verification-gap, intent-alignment).

| Constat | Verdict | Preuve | Suite |
|---|---|---|---|
| Garde-fou d'entraînement modifié malgré la frontière « Never » (blind, verification-gap, intent) | medium | `training/src/families.rs` : mot `reclass` ôté ; le générateur enseigne encore le refus du seuil de pente | intent_gap tranché par le propriétaire (Decision ajoutée), code gardé ; refus du générateur déjà dans `deferred-work.md` pour 12 |
| Commentaire de doc de `escapes_project` déplacé sur `param_escapes_project` (blind, verification-gap, intent) | low | `core/src/serve/calls.rs` : deux paragraphes `///` au-dessus de `param_escapes_project`, aucun sur `escapes_project` | patch |
| Pluriel « (-2 cellule) » figé par un test (blind) | low | `core/src/dem.rs` l. 282 et test l. 1263 | patch |
| Exemples de `raster.weighted_overlay` dégénérés, le second exclut toute la grille (blind, edge) | low | `mnt-plan-incline-2154.tif` vaut `99,5 - colonne` > 0 partout : masque partout non nul ; les exemples sont lus par l'IA | patch : exemple sans `exclusions` (les deux MNT n'ont pas la même grille) |
| `step:k` dans `criteria[i].raster` passe la vérification du plan et échoue tard en fichier introuvable (blind, edge) | medium | `core/src/plan.rs` `raster_markers` ne lit que le premier niveau | defer : exclu par l'intention (Decision `step:k`), ajouté à `deferred-work.md` |
| Score pondéré tombant exactement sur -9999 compté valide mais relu NoData (blind, edge) | low | `overlay` n'a pas de garde ; moyenne pondérée égale à -9999 en f32 très improbable | rejeté (rare, la correction ajoute une branche) |
| Poids finis énormes → `inf`/NaN compté valide (blind, edge) | low | `total` peut déborder avec des poids ~1e308 | rejeté (rare, garde à ajouter) |
| Valeur de classe hors du domaine f32 écrite `inf` (blind, edge) | low | `check_classes` ne teste que -9999 après conversion | rejeté (rare, garde à ajouter) |
| `external.path_refused` ne nomme pas le rang de l'élément (blind) | low | message cite `criteria.raster` | rejeté (le chemin refusé reste lisible par l'IA ; indexer ajoute de la complexité) |
| Unité « m » écrite en dur dans `grid_mismatch` (blind) | low | `Dem::open` n'accepte que des systèmes projetés, en mètres pour 2154 et UTM | rejeté (rare) |
| Tous les rasters chargés avant le contrôle des grilles, sans plafond total (blind) | low | chaque raster plafonné à 1 Gio par `dem.rs` ; la Boundaries demande le contrôle avant tout calcul, respecté | rejeté (rare) |
| Test qui fige `4 * 49` cellules de bord (blind) | low | fixture versionnée et figée | rejeté |
| Refus imbriqué testé seulement au niveau de l'aide (blind, verification-gap, intent) | low | `call` n'utilise que `param_escapes_project` ; un retrait le rendrait inutilisé, attrapé par clippy | rejeté |
| Assertions du prompt incomplètes (`exclusiveMinimum`, `deny_unknown_fields`) (blind) | low | le registre valide de toute façon les paramètres | rejeté |
| `lenses_ran` sans journal de tri (blind) | false | tri en cours d'écriture | rejeté |

## Design Notes

Le chemin imbriqué est le seul point délicat : la fiche fige `criteria` en liste d'objets, et aujourd'hui seul un paramètre de premier niveau (chaîne ou tableau de chaînes) est rendu relatif au journal et absolu à l'exécution. Sans la forme `liste.champ`, un appel par le panneau passerait un chemin relatif que le noyau ouvrirait depuis son dossier courant, et le journal garderait ce qu'il reçoit (AD-9). D'où le risque `high` : le rejeu et le canal sont touchés.

Message d'écart, par exemple : « Le critère 2 (« pente.tif ») n'a pas la grille du critère 1 : origine x décalée de 5 m (0,5 cellule). Alignez-le d'abord dans QGIS. »

## Verification

**Commands:**
- `cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement (sous Windows si le WSL manque de libclang)
- `cargo test --workspace` -- expected: tout vert

**Manual checks (if no CLI):**
- Dans QGIS 3.44, les trois rasters ouverts en EPSG:2154 ; après `journal.replay`, comparaison identique.
