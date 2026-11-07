---
title: 'Familles du générateur pour les nouvelles commandes — lot 2a, dérivés du MNT'
type: 'feature'
lot_de_ticket: '12' # lot du ticket 12 ; le plan du ticket est celui du lot 3
created: '2026-11-07'
status: 'built'
baseline_revision: 'acc9283880a7e55ae32408b7e46b61e4edecf604'
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

**Problem:** Le générateur (`training/`) n'a aucune famille pour `terrain.aspect`, `terrain.hillshade`, `terrain.viewshed_cumulative`, `terrain.peaks` ni `raster.reclassify`, et son refus `NO_RASTER_THRESHOLD` (« où la pente dépasse 15 % » refusé) est faux depuis `raster.reclassify` (11.9). Le ré-entraînement groupé (entrée 16) ne peut pas partir de là.

**Approach:** Cinq familles bâties sur le MNT déjà présent dans l'inventaire synthétique, comme `pente` et `champ_de_vision` : `exposition`, `ombrage`, `champ_de_vision_cumule`, `points_hauts`, et `pente_seuil`, qui remplace le refus par `terrain.slope` puis `raster.reclassify` sur `step:1`. Le validateur accepte `step:k` pour un paramètre raster. Lot 2a du ticket (découpage du propriétaire, 2026-11-07) ; le lot 2b (rasters de coût, de critères et de masque) part à `deferred-work.md`.

## Boundaries & Constraints

**Always:** chaque exemple passe `validate` **et** `guard_coordinates` avec sa demande, plan inchangé ; vocabulaire civil (CLAUDE.md §2.9) ; tout nombre du plan (seuil, azimut, hauteur, proéminence, nombre de sommets, rayon) est écrit dans la demande, sauf les valeurs de sortie des classes (rangs 1, 2, 3) ; paramètre facultatif seulement s'il est écrit ; `Angle` toujours avec `unit` (`gon` si « grades ») ; points dans le système du MNT, système écrit à chaque point ; part des redemandes entre 4 et 7 %.

**Never:** nouveau thème raster d'inventaire, `raster.weighted_overlay`, `terrain.cost_distance`, `terrain.least_cost_path` (lot 2b) ; `step:k` dans un élément de liste ; `ONLY_A4` ni la famille d'export touchés (lot 3) ; `NO_RASTER_STATS` retiré (la statistique par zone reste un refus) ; changement du noyau, de `cassini-ai`, du prompt ou de l'extension ; entraînement ou mesure (entrée 16) ; `DEFAULT_COUNT`, `DEFAULT_SEED` changés ; `step:k.peaks` cité en `observers`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Exposition | « Calcule l'exposition des versants du MNT » | `terrain.aspect` `{dem}` | — |
| Ombrage réglé | « Ombrage du MNT, soleil à l'azimut 270°, hauteur 30°, exagération 2 » | `terrain.hillshade` `{dem, sun_azimuth: {270, deg}, sun_altitude: {30, deg}, z_factor: 2}` | sans réglage écrit : `{dem}` seul |
| Vision cumulée | « Combien de ces points voient chaque endroit : A, B, C » (2 à 4 points) | `terrain.viewshed_cumulative` `{dem, observers: [A, B, C]}` + hauteurs, rayon écrits | — |
| Points hauts | « Les 10 sommets les plus hauts du MNT, proéminence d'au moins 20 m » | `terrain.peaks` `{dem, max_count: 10, min_prominence: {20, m}}` | — |
| Seuil de pente | « Où la pente dépasse-t-elle 15 % » | `terrain.slope` `{dem, unit: percent}` puis `raster.reclassify` `{raster: "step:1", classes: [{min: 15, value: 1}]}` | « sous n » → `[{max: n, value: 1}]` ; « en degrés » → `unit: degrees` |
| Reclassement en classes | « Classe les pentes en moins de 5 %, 5 à 15 %, plus de 15 % » | slope puis `classes: [{max:5,value:1},{min:5,max:15,value:2},{min:15,value:3}]` | — |

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- `FAMILIES` (l. 43) : ajouter les cinq familles avant `redemande`. À réutiliser : `Builder::add_dem`, `terrain_epsg`, `terrain_point`, `dem_words`, `eye_clause`, `target_clause`, `radius_clause`/`radius_words`, `slope_unit`, `finish`, `example`, `calls` ; patrons `pente` (l. ~2765) et `champ_de_vision` (l. 2849). `refus_hors_registre` (l. 2425) : retirer le cas 4 et `NO_RASTER_THRESHOLD` (l. 2412), `below(5)` → `below(4)`, MNT ajouté seulement au cas des statistiques. Tests à revoir : `out_of_registry_requests_are_refused_never_planned` (l. ~4607, liste `[NO_RASTER_STATS, NO_RASTER_THRESHOLD]`), `what_is_refused_out_of_registry_is_still_missing_from_the_registry` (l. ~4976, commentaire 11.9), `a_threshold_on_the_slope_is_refused` (l. ~5259) remplacé. Helpers de test : `valid` (l. 4196), `has_theme` (l. 4214), patron du contrôle `guard_coordinates` (l. 4233). `redemande` ne corrompt que `LAYER_KEYS` : relever son poids si la part sort de 4–7 %.
- `training/src/lib.rs` -- `validate` (l. 55) : le contrôle `dem` contre les `source` raster devient un contrôle des paramètres raster de premier niveau `dem` et `raster`, qui admet aussi une chaîne `step:k` (`check_plan` vérifie déjà la référence et le type raster). Test d'appui : `the_decided_families_and_vocabulary_are_present`.
- `core/src/point.rs` -- `guard_coordinates` descend dans les listes (l. 646) : `observers` gardé point par point ; lecture seule.
- Catalogue `spec-cassini-mvp/catalogue-des-commandes.md` §5 (l. 910–1095) -- paramètres exacts.
- `training/tests/recouvrement.rs` -- interdit de reprendre les phrases des essais réels.
- `docs/entrainement-du-modele.md` -- table des familles (l. 104 refus, l. 112–117 lot 1) et section `### Familles de géométrie (11.12, lot 1)` (l. 156).

## Tasks & Acceptance

**Execution:**
- [x] `training/src/lib.rs` -- paramètres raster `dem` et `raster` : source raster de l'inventaire ou `step:k` ; test d'un `step:1` admis et d'un chemin inventé refusé -- rendre `pente_seuil` validable.
- [x] `training/src/families.rs` -- familles `exposition` (3), `ombrage` (3 ; `sun_azimuth`, `sun_altitude`, `z_factor` seulement écrits, degrés ou grades), `champ_de_vision_cumule` (4 ; 2 à 4 points du système du MNT, hauteurs et rayon comme `champ_de_vision`), `points_hauts` (4 ; `max_count` de 1 à 100, `min_prominence`, `min_distance` seulement écrits), `pente_seuil` (4 ; seuil simple au-dessus ou au-dessous, ou deux à trois classes) ; retrait du cas de refus -- couvrir chaque commande, retirer le refus faux.
- [x] `training/src/families.rs` (tests) -- par famille : commande et paramètres attendus, nombres retrouvés dans la demande, plan inchangé par `guard_coordinates` ; `pente_seuil` : unité de la pente égale à celle du seuil, classes ordonnées ; garde-fou : aucun refus de `refus_hors_registre` ne dit qu'aucune commande ne sélectionne les cellules d'un raster -- vérifier sans modèle.
- [x] `docs/entrainement-du-modele.md` -- lignes des cinq familles, ligne 104 corrigée (seuil sur la pente devenu un plan), section « 11.12, lot 2a » -- doc à jour.

**Acceptance Criteria:**
- Given le jeu par défaut (graine 1, 8 000 exemples), when il est engendré, then chacune des cinq familles garde au moins un exemple, aucune n'est rejetée au résumé, et chacune des cinq commandes figure dans au moins un plan attendu.
- Given le même jeu, when on cherche un refus qui dit qu'aucune commande ne sélectionne les cellules d'un raster par leur valeur, then il n'y en a aucun, et le garde-fou échoue si on le réintroduit.
- Given `cargo test -p cassini-training`, `cargo clippy -p cassini-training -p cassini-core --all-targets -- -D warnings` et `cargo fmt --check`, when ils tournent, then tout est vert.

## Implementation Notes

- Jeu par défaut (graine 1, 8 000) : exposition 97, ombrage 108, champ_de_vision_cumule 155, points_hauts 144, pente_seuil 141, aucun écarté ; redemandes 348 (4,35 %), poids de `redemande` laissé à 36.
- `max_count` tiré de 2 à 100, jamais 1 : « le sommet le plus haut » n'écrirait pas le nombre.
- Le rayon d'un champ de vision cumulé reprend `radius_clause`, qui écrit parfois en lettres (« un kilomètre ») comme `champ_de_vision` ; le test des nombres écrits ne porte donc que sur les hauteurs, les points, les seuils et les réglages d'ombrage et de sommets.
- Garde-fou `selects_no_cell` (`families.rs`, `cfg(test)`) : vérifié sur le jeu par défaut (`lib.rs`), dans `refus`/`refus_hors_registre` sur 2 000 tirages, et sur le texte de l'ancien refus.
- Message du validateur : « MNT absent » devient « raster absent » (test existant mis à jour).
- Entrée de `deferred-work.md` sur `NO_RASTER_THRESHOLD` (11.9) close.

## Plan Change Log

## Review Triage Log

Passe 1 (quick, 2026-11-07) : high 0, medium 0, low 3, false 1, maybe-false 0.

| # | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|
| 1 | `docs/entrainement-du-modele.md` l. 187 : « lot 2 … reste à faire » contredit la section 2a | low | patch | Phrase réécrite : 2a fait, 2b et lot 3 restent. |
| 2 | `points_hauts` : `max_count` tiré de 2 à 100, la tâche dit « de 1 à 100 » | low | rejet | 1 à 100 est la borne du catalogue ; tirer dedans en évitant 1, que « le sommet le plus haut » n'écrirait pas, respecte la règle « tout nombre écrit ». Aucun dommage. |
| 3 | Rayon de `champ_de_vision_cumule` parfois en lettres (« un kilomètre »), non contrôlé par le test | low | rejet | Tournure reprise de `champ_de_vision` (déjà apprise) ; le rayon est bien écrit, en lettres ; le garde-fou ne porte que sur les points. |
| 4 | Lignes longues de `pente_seuil` peut-être mal formatées | false | rejet | `cargo fmt --check` passé (rustfmt laisse les chaînes trop longues). |

## Design Notes

Sens d'un seuil : « dépasse, supérieure à, plus de n » → `{min: n, value: 1}` ; « sous, inférieure à, moins de n » → `{max: n, value: 1}` ; une cellule hors classe devient NoData, ce qui est la sélection voulue. Classes multiples : valeurs de sortie 1, 2, 3 dans l'ordre, bornes contiguës. L'unité de `terrain.slope` suit celle du seuil (« % » → `percent`, « degrés » → `degrees`). Un azimut d'ombrage se rapporte au nord du quadrillage : la demande dit « soleil à l'ouest (270°) », jamais un azimut calculé.

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: tous verts, nouveaux tests compris
- `cargo run -p cassini-training --release` -- expected: 8 000 exemples, aucune famille rejetée au résumé, redemandes entre 4 et 7 %
- `cargo clippy -p cassini-training -p cassini-core --all-targets -- -D warnings && cargo fmt --check` -- expected: aucun avertissement
