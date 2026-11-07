---
title: 'Familles du générateur pour les nouvelles commandes — lot 2b-1, raster de coût'
type: 'feature'
lot_de_ticket: '12' # lot du ticket 12 ; le plan du ticket est celui du lot 3
created: '2026-11-07'
status: 'built'
baseline_revision: 'a9d0c064cc2081601c446d3697255330fe8de21b'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le générateur (`training/`) n'a aucune famille pour `terrain.cost_distance` ni `terrain.least_cost_path` : son inventaire synthétique n'a qu'un raster, le MNT. Le ré-entraînement groupé (entrée 16) ne peut pas partir de là.

**Approach:** Ajouter à l'inventaire un thème raster civil de coût de déplacement, puis deux familles, `cout_cumule` et `chemin_moindre_cout`, dont les points sont écrits dans le système du raster de coût. Le validateur contrôle `cost` contre les `source` raster de l'inventaire. Lot 2b-1 du ticket (découpage du propriétaire, 2026-11-07) ; critères, masques, superposition et reclassement d'un raster brut partent au lot 2b-2 (`deferred-work.md`).

## Boundaries & Constraints

**Always:** chaque exemple passe `validate` **et** `guard_coordinates` avec sa demande, plan inchangé ; vocabulaire civil (CLAUDE.md §2.9) ; toute coordonnée du plan est écrite dans la demande, avec son système à chaque point ; points dans le système du raster de coût (2154, 32631 ou 32632) ; le raster de coût est la `source` d'une couche raster de l'inventaire, protégée comme celle du MNT ; part des redemandes entre 4 et 7 %.

**Never:** `step:k` dans `sources` ; pente ou autre dérivé du MNT pris comme coût ; nom du thème de coût qui contient « pente », « MNT », « altitude » ou « relief » ; thèmes de critères, de masque ou rasters bruts, `raster.weighted_overlay`, reclassement d'un raster autre que la pente (lot 2b-2) ; `ONLY_A4` ni la famille d'export touchés (lot 3) ; le refus d'itinéraire entre deux couches (`refus`, l. 2421) retiré : un réseau routier n'est pas un raster de coût ; changement du noyau, de `cassini-ai`, du prompt ou de l'extension ; entraînement ou mesure (entrée 16) ; `DEFAULT_COUNT`, `DEFAULT_SEED` changés.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Coût cumulé | « Calcule le coût de déplacement cumulé depuis le point (652 300 ; 6 862 100) en Lambert-93 sur le raster de coût » | `terrain.cost_distance` `{cost, sources: [P]}` ; 1 à 3 points écrits, dans l'ordre écrit | lieu nommé sans coordonnées → question qui demande le point en X, Y et système, jamais un plan |
| Chemin | « Trace le chemin de moindre coût du point A au point B » | `terrain.least_cost_path` `{cost, start: A, end: B}`, départ et arrivée dans l'ordre écrit | un seul point écrit → question qui demande l'autre point |
| Coût absent | demande de coût, aucun raster de coût dans l'inventaire (MNT seul ou rien) | question qui demande d'ouvrir ou de nommer le raster de coût, jamais le MNT pris comme coût | — |

</frozen-after-approval>

## Code Map

- `training/src/inventories.rs` -- `THEMES` (l. ~1127, `mnt` dernier) : ajouter le thème `cout` (`Geom::Raster`, `fields: &[]`, groupe `cout` ; noms « Coût de déplacement », « Friction », « Coût de traversée »). `Builder::add_dem` (l. 1381) : en tirer un `add_raster(rng, key, epsg)` qui pose une `source` protégée (`sourced`) et l'`epsg` ; `add_dem` garde ses fichiers UTM. `finish` (l. 1432) tire déjà un distracteur raster sur quatre : `cout` y entre (voulu : ne pas prendre un raster de coût pour le MNT). Test d'appui : `the_cited_dem_keeps_its_source_and_takes_the_chosen_system` (l. 1608).
- `training/src/families.rs` -- `FAMILIES` (l. 43) : deux familles avant `redemande`, poids 4. Réutiliser `terrain_epsg`, `terrain_point`, `enumerate`, `finish`, `example`, `calls`, la question du point manquant (l. ~4033) et le lieu nommé de `question_terrain` (l. 4054, `de_place`) ; patron `champ_de_vision_cumule` (l. 3021). `question_terrain` cas 0 (`exclude("mnt")`) : le raster de coût peut y venir en distracteur, la question reste juste ; adapter un test qui supposerait aucun raster. Tests : helpers `valid`, `commands`, `written`, `cited_dem`, `terrain_examples`, `points_in_the_dem_system` (l. 4561–5512), à généraliser au paramètre `cost`.
- `training/src/lib.rs` -- `validate` : `RASTER_PARAMS` (l. 52) gagne `cost`. Test : un `cost` inventé refusé, une `source` admise.
- `core/src/point.rs` -- `guard_coordinates` descend dans les listes (`sources`) ; lecture seule.
- Catalogue `spec-cassini-mvp/catalogue-des-commandes.md` l. 1126–1200 -- paramètres exacts (`sources` 1 à 32, `start`, `end`, `out` jamais écrit).
- `training/tests/recouvrement.rs` -- interdit de reprendre les phrases des essais réels.
- `docs/entrainement-du-modele.md` -- table des familles (l. ~117) et section lot 2a (l. 191) : ajouter une section « 11.12, lot 2b-1 », corriger la phrase de reste à faire (2b-2 et lot 3 restent).

## Tasks & Acceptance

**Execution:**
- [x] `training/src/inventories.rs` -- thème `cout`, `add_raster` ; test que la source citée survit à `qgis_shapes` et prend l'`epsg` choisi -- matière des familles.
- [x] `training/src/lib.rs` -- `cost` contrôlé ; test -- refuser l'invention.
- [x] `training/src/families.rs` -- `cout_cumule` (1 à 3 sources ; 1 sur 6 lieu nommé → question), `chemin_moindre_cout` (1 sur 6 un seul point → question), et le cas « coût absent » (question) dans l'une d'elles -- couvrir les deux commandes.
- [x] `training/src/families.rs` (tests) -- par famille : commande et clés attendues seules, `cost` source d'un raster `cout` de l'inventaire, coordonnées retrouvées dans la demande, points dans le système du raster (2154, 32631, 32632 vus), ordre départ/arrivée, plan inchangé par `guard_coordinates` ; questions sans plan, jamais le MNT en `cost` -- vérifier sans modèle.
- [x] `docs/entrainement-du-modele.md` -- lignes des deux familles, section lot 2b-1, phrase de reste à faire -- doc à jour.

**Acceptance Criteria:**
- Given le jeu par défaut (graine 1, 8 000 exemples), when il est engendré, then les deux familles gardent chacune au moins un exemple, aucune n'est rejetée au résumé, et `terrain.cost_distance` et `terrain.least_cost_path` figurent chacun dans au moins un plan attendu.
- Given le même jeu, when on lit chaque plan, then aucun `cost` n'est le `source` d'un MNT ni absent de l'inventaire, et aucun `step:` ne figure dans `sources`.
- Given `cargo test -p cassini-training`, `cargo clippy -p cassini-training -p cassini-core --all-targets -- -D warnings` et `cargo fmt --check`, when ils tournent, then tout est vert.

## Implementation Notes

- `Builder::add_dem` et `Builder::add_raster` passent par `add_sourced` ; le tirage de `add_dem` est inchangé (mêmes appels dans le même ordre), le raster de coût prend `rasters/` à la place de `dem/`.
- Le cas « coût absent » vit dans `chemin_moindre_cout` (1 sur 6, `cout_absent`), avec une demande de coût cumulé ou de chemin ; le MNT y est une fois sur deux.
- `cout_cumule` : 1 sur 6 lieu nommé (seul ou parmi des points écrits) → question du point. `chemin_moindre_cout` : 1 sur 6 un seul point écrit, l'autre nommé (départ ou arrivée) → question du point.
- `question_terrain` : son test compte l'absence du MNT (`has_theme(e, "mnt")`), plus l'absence de tout raster. `a_travel_without_coordinates_never_becomes_a_plan` (lib.rs) admet les deux familles de coût parmi celles qui demandent un point.
- Test de jeu par défaut ajouté dans `lib.rs` : `the_cost_families_of_the_default_set_cite_a_cost_raster_never_the_dem`.
- Graine 1, 8 000 exemples : `cout_cumule` 140, `chemin_moindre_cout` 136, aucun rejet ; 383 redemandes (4,8 %).

## Plan Change Log

- 2026-11-07 : poids de `redemande` relevé de 36 à 44. Les deux familles, à une étape et sans couche, ne se corrompent pas : à poids inchangé, 313 redemandes (3,9 %), sous le plancher de 4 % des Boundaries ; à 40, 322 (4,03 %), trop juste.

## Review Triage Log

Passe 1 (quick, 2026-11-07) : high 0, medium 1, low 6, false 0, maybe-false 0.

| # | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|
| 1 | `refus` cas 11 : « Aucune commande du catalogue ne calcule d'itinéraire » faux depuis `terrain.least_cost_path` | medium | patch | Le modèle apprendrait une affirmation absolue fausse ; raison réécrite (réseau routier entre deux couches, le chemin de moindre coût demande un raster de coût et deux points). |
| 2 | Doc : MNT en distraction « une fois sur trois », réellement ≈ 1/2 (`finish` tire aussi `mnt`) | low | patch | Taux corrigé dans la doc. |
| 3 | Note du plan sur `cout_absent` (« une fois sur deux ») ≈ 5/8 | low | rejet | Correction du plan lui-même ; le cas « aucun raster » reste tiré (≈ 3/8), la matrice est couverte. |
| 4 | Commentaire de `dem_words` : « le MNT est le seul raster » devenu faux | low | patch | Commentaire réécrit. |
| 5 | `validate` admet le MNT ou `step:k` en `cost` | low | rejet | Le test `the_cost_families_of_the_default_set_cite_a_cost_raster_never_the_dem` contrôle tout `cost` du jeu par défaut, toutes familles ; un contrôle par thème dans le validateur ajouterait de la complexité sans défaut observé. |
| 6 | `deferred-work.md` : entrée « 11.12 lot 2b » restée ouverte | low | patch | `status` et `resolution` ajoutés, comme l'entrée close de 2a. |
| 7 | Contrôle d'ordre par rang du premier nombre égal à X, fragile si un X égale un autre nombre | low | rejet | Graine fixe, tests déterministes et verts ; une collision ne ferait qu'un faux échec visible, jamais un exemple faux. |

## Design Notes

Coût par mètre : la demande parle de « coût de déplacement », « effort », « friction », jamais d'unité. Plusieurs sources : « depuis les points A, B et C » (`enumerate`). Le MNT peut être présent en distracteur : « sur le raster de coût » ou le nom de sa couche le désigne, jamais « du terrain ».

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: tous verts, nouveaux tests compris
- `cargo run -p cassini-training --release` -- expected: 8 000 exemples, aucune famille rejetée au résumé, redemandes entre 4 et 7 %
- `cargo clippy -p cassini-training -p cassini-core --all-targets -- -D warnings && cargo fmt --check` -- expected: aucun avertissement
