---
title: 'Points hauts d''un MNT'
type: 'feature'
ticket: '15'
created: '2026-11-06'
status: 'built'
baseline_revision: '5f85209c64cb0cc3f87bf3d070bb9a03177dd449'
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

**Problem:** le noyau ne sait pas trouver les sommets d'un MNT ; la fiche `terrain.peaks` du catalogue (planifiée, entrée 15) attend son implémentation, première commande publique à écrire un GeoJSON par l'écrivain de l'entrée 8.

**Approach:** une commande du noyau calcule les sommets par inondation descendante (cellules triées de la plus haute à la plus basse, union de composantes en 8-connexité) : chaque composante naît sur un sommet, et la proéminence du sommet le plus bas tombe au col où deux composantes se rejoignent ; filtre par proéminence, puis par écart, puis par nombre ; écrit les points par `geojson::write` dans l'EPSG du MNT ; la sortie `.geojson` déclenche l'ouverture dictée `layer.open_vector` déjà en place ; la fiche passe en `faite`.

## Boundaries & Constraints

**Always:** fiche du catalogue respectée à la lettre (paramètres `dem`, `min_prominence` défaut 10 m, `min_distance` défaut 100 m, `max_count` 1 à 100 défaut 20, `out` ; résultat `file`, `feature_count`, `peaks` `{point, elevation, prominence}`, `truncated`, `unit` = `m`) ; propriétés GeoJSON `rank`, `elevation`, `prominence` dans cet ordre ; point au centre de la cellule, EPSG du MNT ; ordre total déterministe : altitude décroissante, puis ordre de lecture (ligne, puis colonne), pour le tri, les égalités et le classement ; **égalité** : une cellule n'est sommet que si aucune voisine n'est plus haute et qu'aucune voisine de même altitude ne la précède dans l'ordre de lecture (un plateau donne un sommet, sa première cellule) ; **bord** : une cellule du bord du MNT, ou voisine d'un trou, n'est jamais un sommet mais compte comme terrain dans l'inondation ; proéminence du point culminant retenu, ou d'une composante jamais rejointe : dénivelé jusqu'à la cellule valide la plus basse du MNT ; deux sommets à moins de `min_distance` (distance planimétrique) : seul le premier dans l'ordre est gardé ; `truncated` vrai si un sommet admis par les deux filtres est écarté par `max_count` ; un résultat vide est un succès qui écrit un GeoJSON sans entité ; aucune dépendance nouvelle ; pas de parallélisme ; erreurs du MNT par `DemError` telles quelles, de l'écriture par `geojson.unwritable` ; coût au pire (MNT de 1 Gio) dit dans la doc du module.

**Never:** pas de reprojection (AD-19) ; pas de lecture de GeoJSON ; pas d'extension de la grammaire des références (`step:k.peaks.0.point` reste hors de portée : une seule clé de premier niveau) ; pas de citabilité du `layer_id` de l'ouverture dictée (entrée renvoyée à `deferred-work.md`) ; aucun changement de l'entraînement ni du générateur (entrées 12 et 16) ; aucun changement de l'extension QGIS ni de `layer.open_vector`.

**Decision (2026-11-06, propriétaire) :** plan gardé entier malgré sa taille (un seul livrable) ; règles du plateau et du bord approuvées.

**Decision (2026-11-06, propriétaire, relecture) :** « point culminant » se lit au sens du catalogue, point culminant du MNT : si la cellule la plus haute est au bord ou près d'un trou, le plus haut sommet retenu garde la proéminence de son col.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Colline | `mnt-colline-2154.tif`, défauts | 1 sommet, cellule (24, 24) du plateau central 2 × 2 : point (652245, 6850255), `rank` 1, proéminence = altitude − cellule la plus basse | — |
| Plan incliné | `mnt-plan-incline-2154.tif` | 0 sommet, GeoJSON sans entité, `truncated` faux | — |
| Deux collines | MNT synthétique en test, deux bosses de hauteurs différentes | deux sommets, le plus bas avec la proéminence de son col | — |
| Proéminence | même MNT, `min_prominence` au-dessus de celle du second | un seul sommet | — |
| Écart | même MNT, `min_distance` au-delà de leur écart | le plus haut seul | — |
| Nombre | trois sommets admis, `max_count` 2 | 2 écrits, `truncated` vrai | — |
| Trou | sommet voisin d'une cellule sans donnée | pas de sommet là | — |
| Bornes | `max_count` 0 ou 101, hauteur négative, distance nulle | rien d'écrit | erreur de paramètres du registre |
| MNT refusé | géographique, absent | rien d'écrit | code de `dem.rs` tel quel |
| Rejeu | deux exécutions | GeoJSON identiques octet pour octet | — |
| QGIS réel | commande lancée dans QGIS 3.44, puis `journal.replay` | couche de points ouverte en EPSG:2154, rouverte au rejeu | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/terrain_peaks.rs` (nouveau, découvert par `build.rs`) -- calqué sur `terrain_slope.rs` (spec l. 120-144, `run` l. 146-160, `Scratch` et `fixture` des tests) : `.runs_in(RunsIn::Core)`, `.cancellable(false)`, `.path_params(&["dem"])`, `.output_params(&["out"])`, `.output_ext(".geojson")` (déclenche la dictée, `VECTOR_EXTENSIONS` de `geojson.rs` l. 22), exemples sur les deux fixtures, `.tests(..)`. Types à reprendre : `Height` (`terrain_line_of_sight.rs` l. 65, `metres()`), `Distance` (`analysis_buffer.rs` l. 24, strictement positive), `Point` (`point.rs` l. 43), `RasterPath`. `max_count` : `u32` avec `#[schemars(range(min = 1, max = 100))]` et contrôle à la désérialisation, comme `observers` de `terrain_viewshed_cumulative.rs` l. 42-51. Défauts par fonctions `default_*`. Fonction pure `peaks(dem, ...) -> Vec<Peak>` testable sans fichier.
- `core/src/commands/mod.rs` -- ajouter `impl From<GeojsonError> for CommandFailure`, à côté de celui de `DemError` (l. 14-18), code et message tels quels.
- `core/src/dem.rs` -- `Dem` (l. 111 : `width`, `height`, `origin_x`, `origin_y` coin haut-gauche, `cell_size_*`, `epsg`, `value(col,row) -> Option<f32>`) ; MNT de test construits en mémoire puis `write_geotiff` dans un `Scratch`. Fixture `colline()` l. 644 : 50 × 50, cellule 10 m, origine (652000, 6850500), sommet au milieu de quatre cellules égales. Ne rien changer à `dem.rs`.
- `core/src/geojson.rs` -- `write(path, epsg, &[Feature])`, `Geometry::Point`, `Property::{Integer, Number}` ; ne pas modifier.
- `ai/src/plan.rs` l. 1025-1050 -- ajouter `"terrain.peaks"` à la liste des commandes raster vérifiées pour le prompt (`RasterPath` effacé), et une assertion sur `max_count` borné (1 à 100), comme pour `observers`.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` l. 269 (`planifiée` → `faite`, forme de la ligne `terrain.viewshed_cumulative`), fiche l. 1019-1052 : `"status": "faite"`, phrase sur l'égalité (plateau) et le bord. `core/tests/catalogue.rs` confronte, rien à y changer.
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- annoter l'entrée « `layer_id` de `layer.open_vector` citable » : toujours ouverte après 11.15, l'entrée 15 ne l'a pas exigée.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/mod.rs` -- conversion `GeojsonError` → `CommandFailure`.
- [x] `core/src/commands/terrain_peaks.rs` -- commande, algorithme, tests de toute la matrice hors QGIS (dont le test « la commande court dans le noyau et le contrat l'accepte » des autres terrains, et deux exécutions identiques).
- [x] `ai/src/plan.rs` -- commande vue par le prompt sans `RasterPath`, `max_count` borné.
- [x] `catalogue-des-commandes.md`, `deferred-work.md` -- fiche en `faite`, même commit.
- [ ] Poste du propriétaire -- installer noyau et extension (mémoire « Mettre à jour QGIS et le noyau »), lancer `terrain.peaks` sur le MNT du projet test dans QGIS 3.44, puis `journal.replay`.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when `cargo test --workspace`, then tout passe, dont `catalogue` avec la fiche `faite`.
- Given une session QGIS 3.44 avec l'extension, when `terrain.peaks` s'exécute sur un MNT du projet, then une couche de points en EPSG:2154 s'ajoute, avec `rank`, `elevation`, `prominence`, et `journal.replay` la rouvre à l'identique.

## Implementation Notes

- Algorithme dans `peaks(dem, min_prominence, min_distance, max_count)` : tri total (`total_cmp` inversé, puis index), union-find à division de chemin ; le filtre d'écart est glouton contre les sommets déjà gardés et s'arrête au `max_count + 1`-ième gardé (borne la boucle à 101 comparaisons par sommet admis).
- Règle d'égalité appliquée à la lettre : un plateau non convexe (en V, par exemple) peut donner une seconde cellule « sans voisine traitée », de proéminence 0 ; elle n'est gardée que si `min_prominence` vaut 0 et qu'elle est hors de `min_distance`.
- Mesure du coût (release, WSL, i5) : 4 M cellules 0,57 s, 16 M cellules 3,1 s ; extrapolé à 268 M (plafond 1 Gio), une à deux minutes, dit dans la doc du module et la fiche.
- Fiche `terrain.peaks` déplacée en fin de §5 (commandes faites), l'en-tête du §6 descend après elle, comme à l'entrée 7.
- Sonde QGIS 3.44.15 (offscreen, noyau `C:\ct\release\cassini.exe --features gpu`, extension du dépôt, copie `projet-sonde-11-15.qgz`) : 20 sommets sur `dem/mnt-villeneuve-le-roi-2154.tif`, `truncated` vrai, couche `1-terrain.peaks` en EPSG:2154 aux champs `rank`, `elevation`, `prominence` ; rejeu `identical: true` (2 étapes), mêmes entités. Extension installée dans le profil ; noyau **non installé** dans `%LOCALAPPDATA%\Cassini\bin` (arrêt du serveur MCP qui verrouille l'exe refusé par le garde-fou de la session).

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 0, medium 0, low 6, false 1, maybe-false 0, plus un écart d'intention tranché par le propriétaire ; 6 patchs envoyés à l'implémenteur, 1 rejet, 1 reste à faire sur le poste.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| `cargo fmt` non appliqué à `terrain_peaks.rs` | false | rejet | `cargo fmt --all --check` sort 0 sans diff |
| Mémoire au pire sous-estimée (8 o/cellule ; `found` et `summits` oubliés) | low | patch | `found: Vec<Peak>` (24 o) et `summits` vivent avec `order`/`parent` ; doc et fiche portées à ~15 o/cellule, ~3,7 Gio |
| Doc « compression de chemin » alors que `find` divise le chemin | low | patch | `find` pose `parent[i] = grand-parent` ; doc corrigée |
| Fiche : `peaks` « utilisables comme références d'étape » | low | patch | `parse_reference` n'admet qu'un champ de premier niveau ; fiche : liste citable entière par `step:k.peaks` |
| Règle du plateau énoncée sans l'exception du plateau non convexe | low | patch | Second candidat de proéminence 0 gardé si `min_prominence` = 0 ; une clause dans doc et fiche |
| Ligne « MNT refusé » : aucun MNT géographique éprouvé | low | patch | Cas EPSG:4326 ajouté au test, `dem.not_projected`, rien d'écrit |
| Proéminence du plus haut sommet retenu quand le culminant est au bord : deux lectures, test indiscernable | low | intent_gap → décision | Propriétaire : proéminence au col (lecture du catalogue, code inchangé) ; test refait avec col 13 m ≠ cellule basse −50 m, 17 m attendus |
| Critère QGIS non joué avec le noyau installé | medium | reste à faire | Sonde jouée avec `C:\ct\release\cassini.exe` ; l'installation attend l'arrêt du `cassini.exe` qui verrouille le fichier |

Vérification après patchs (WSL) : `cargo fmt --all --check` OK ; `cargo clippy -p cassini-core --all-targets -D warnings` OK ; `cargo test -p cassini-core` 493 + recettes vertes, sauf `canal_externe::closing_the_input_warns_the_relays_and_removes_the_channel`, délai dépassé sous charge, vert 3 fois sur 3 seul (fichier non touché par ce changement).

## Design Notes

Inondation (union-find, racine = sommet de la composante) : parcourir les cellules valides dans l'ordre total ; une cellule sans voisine déjà traitée ouvre une composante (sommet candidat) ; sinon elle rejoint ses voisines traitées ; si elle en relie plusieurs, chaque composante dont le sommet n'est pas le premier dans l'ordre reçoit `prominence = z(sommet) − z(cellule)` et se fond dans la première. Les sommets marqués « bord » gardent leur rôle de terrain mais sont écartés à la fin. Mémoire : deux `Vec<u32>` (ordre, parent) en plus du MNT, soit 8 octets par cellule ; tri `sort_unstable_by` sur clé totale (`f32::total_cmp` inversé, puis index).

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement (sous Windows si le WSL manque de libclang)
- `cargo test --workspace` -- expected: tout vert

**Manual checks (if no CLI):**
- Dans QGIS 3.44, couche « points hauts » ajoutée, entités lisibles ; après `journal.replay`, même couche rouverte et comparaison identique.
