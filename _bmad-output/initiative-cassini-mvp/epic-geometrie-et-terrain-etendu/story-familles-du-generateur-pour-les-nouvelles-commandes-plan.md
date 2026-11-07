---
title: 'Familles du générateur pour les nouvelles commandes — lot géométrie'
type: 'feature'
ticket: '12'
created: '2026-11-07'
status: 'built'
baseline_revision: '59a09a30d14a2bc2baa5eaf9eb2c3fbd5afe1afb'
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

**Problem:** Le générateur d'exemples (`training/`) n'a aucune famille pour les commandes de géométrie de l'epic 11 (`coord.convert`, `data.transform_point`, `geo.distance_azimuth`, `geo.destination_point`, `geo.bearing_intersection`), et son refus `NO_TERRAIN_CRS` (« aucune ne reprojette un point ») est devenu faux depuis `data.transform_point`. Le ré-entraînement groupé (entrée 16) ne peut pas partir de là.

**Approach:** Six familles écrites comme les familles de terrain existantes (inventaire synthétique, gabarits français, plan attendu) : une par commande, plus `terrain_autre_systeme`, qui remplace le refus par une reprojection visible avant la commande de terrain. Un test par famille et un garde-fou qui échoue si le refus revient. Lot 1 sur 3 du ticket (découpage du propriétaire, 2026-11-07) ; les lots raster et export sont à `deferred-work.md`.

## Boundaries & Constraints

**Always:** chaque exemple passe `check_plan` **et** `guard_coordinates` avec sa demande ; vocabulaire civil (CLAUDE.md §2.9) ; tout nombre du plan (azimut, distance, précision, zone) est écrit dans la demande ; une conversion ou reprojection est une étape visible du plan ; `Angle` toujours avec `unit` ; paramètre facultatif seulement s'il est écrit ; MGRS valides, tirés par `cassini_core::geodesy::Mgrs::from_lat_lon` ; part des redemandes entre 4 et 7 %.

**Never:** familles raster, de terrain étendu ou d'export (lots 2 et 3) ; refus `ONLY_A4` et `NO_RASTER_THRESHOLD` touchés (lots 3 et 2) ; famille pour `layer.open_vector` (dictée) ; `step:k` dans un élément de liste ; changement du noyau, de `cassini-ai`, du prompt ou de l'extension ; entraînement ou mesure du modèle (entrée 16) ; `DEFAULT_COUNT`, `DEFAULT_SEED` changés ; refus de reprojection d'une **couche** retiré (`data.reproject` reste planifiée).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Conversion | « Donne la référence MGRS du point 48,85 N, 2,35 E (WGS84) » | `coord.convert` `{point: {x: 2.35, y: 48.85, epsg: 4326}, to: ["mgrs"]}` | — |
| MGRS lu | « Convertis 31U DQ 52000 12000 en latitude et longitude » | `coord.convert` `{mgrs: "31UDQ5200012000", to: ["wgs84"]}` | — |
| Lambert vers MGRS | point Lambert-93, MGRS demandé | `data.transform_point` (→ 4326) puis `coord.convert` `{point: "step:1.point", to: ["mgrs"]}` | — |
| Géodésie en Lambert-93 | distance entre deux points Lambert-93 | deux `data.transform_point` (→ 4326) puis `geo.distance_azimuth` sur `step:1.point`, `step:2.point` | — |
| Point d'un autre système | MNT en 2154, point en WGS84 ou UTM ; ou MNT 32631, point 31N | `data.transform_point` (`target_epsg` = epsg du MNT) puis terrain sur `step:1.point` | MNT sans `epsg` à l'inventaire : exemple non gardé |
| MGRS vers terrain | « … depuis 31U DQ 48251 11750 », MNT en 2154 | `coord.convert` (→ wgs84), `data.transform_point` (`step:1.wgs84`), terrain sur `step:2.point` | — |

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- `FAMILIES` (l. 43, total des poids 201) ; à réutiliser : `Origin`, `lambert_origin`, `utm_origin_in`, `origin_point` (forme WGS84), `terrain_epsg`, `terrain_point`, `dem_words`, `eye_clause`, `target_clause`, `radius_clause`, `finish`, `example`, `calls`, `thousands`. `foreign_point` (l. 2633) rend une `String` : la faire rendre une `Origin` (point + phrase) et l'élargir au point UTM 32N sur MNT 31N et inversement (deferred 5.41). Retirer le cas 5 de `refus_hors_registre` et `NO_TERRAIN_CRS`, et `dem_has_epsg` passe à la nouvelle famille. `redemande` ne corrompt que `LAYER_KEYS` : relever son poids si la part sort de 4–7 %. Test `what_is_refused_out_of_registry_is_still_missing_from_the_registry` (l. 3921) : y ajouter le volet « point reprojeté ».
- `training/src/lib.rs` -- `validate` (l. 55) : contrôle `dem` contre les `source` raster de l'inventaire (rien à changer, `dem` reste un chemin dans ce lot). `the_decided_families_and_vocabulary_are_present` impose que chaque famille garde un exemple au jeu par défaut. Modèle de test de garde : `the_utm_isochrones_of_the_default_set_pass_the_guard` (l. 402).
- `core/src/point.rs` -- `guard_coordinates`, `request_numbers` (lit `2,35`, `6 850 000`, « UTM 31N », « Lambert-93 », « WGS84 »), MGRS écrit reconnu à espaces et casse près ; lecture seule.
- `core/src/geodesy.rs` -- `Mgrs::from_lat_lon(lat, lon, precision)`, `Mgrs::canonical`, `to_utm`, `utm_zone` ; lecture seule.
- Catalogue `spec-cassini-mvp/catalogue-des-commandes.md` §5 -- paramètres exacts des cinq commandes.
- `training/tests/recouvrement.rs` -- interdit de reprendre les phrases des essais réels.
- `docs/entrainement-du-modele.md` -- section des familles du générateur.

## Tasks & Acceptance

**Execution:**
- [x] `training/src/families.rs` -- familles `conversion_coordonnees` (WGS84, UTM, MGRS entre eux ; `mgrs_precision`, `utm_zone` si écrits ; Lambert-93 → UTM ou MGRS en deux étapes), `reprojection_point` (dès qu'un système hors WGS84/UTM/MGRS est en jeu), `distance_azimut` (`unit: gon` si « grades » ; points Lambert-93 reprojetés d'abord), `point_projete` (option : résultat demandé en MGRS → `coord.convert` sur `step:1.point`), `recoupement`, `terrain_autre_systeme` (champ de vision ou ligne de vue ; WGS84, UTM d'une autre zone, Lambert-93 sur MNT UTM, MGRS) ; poids 3 à 5 ; retrait du refus -- couvrir chaque commande, retirer le refus faux.
- [x] `training/src/families.rs` (tests) -- par famille : commande et paramètres attendus, nombres retrouvés dans la demande, plan inchangé par `guard_coordinates` ; garde-fou du refus retiré -- vérifier sans modèle.
- [x] `docs/entrainement-du-modele.md` -- section 11.12 lot 1 : familles ajoutées, refus retiré, règle « WGS84/UTM/MGRS → `coord.convert`, autre système → `data.transform_point` » -- doc à jour.

**Acceptance Criteria:**
- Given le jeu par défaut (graine 1, 8 000 exemples), when il est engendré, then chacune des six familles garde au moins un exemple et chacune des cinq commandes figure dans au moins un plan attendu.
- Given le même jeu, when on cherche un refus qui dit qu'aucune commande ne reprojette un point, then il n'y en a aucun, et le garde-fou des refus échoue si on le réintroduit.
- Given tout exemple d'appels des six familles, when il passe `check_plan` puis `guard_coordinates` avec sa demande, then le plan revient inchangé.
- Given `cargo test -p cassini-training`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --check`, when ils tournent, then tout est vert.

## Implementation Notes

- Poids : `conversion_coordonnees` 5, `reprojection_point` 4, `distance_azimut` 4, `point_projete` 4, `recoupement` 3, `terrain_autre_systeme` 4 ; `redemande` relevé de 24 à 36 (419 redemandes sur 8 000, 5,2 %), les plans de géométrie n'ayant pas de couche à corrompre.
- Vers le système d'un MNT, toujours `data.transform_point` (`target_epsg` = epsg du MNT), y compris d'UTM 32N vers 31N, comme le veut la matrice : `coord.convert` exigerait un `utm_zone` non écrit. Une référence MGRS y passe par `coord.convert` → wgs84 puis `step:k.wgs84`.
- `target_epsg` n'est jamais égal à un système écrit pour un point du même plan : le garde-fou retire les nombres hors `Point` avant les points, et un tel plan rendrait sa question.
- `refus_hors_registre` tire désormais `below(5)` (cas 5 retiré) ; le test `a_terrain_point_in_another_system_than_the_dem_is_refused` est remplacé par `a_terrain_point_in_another_system_is_reprojected_into_the_dem_system_first`.
- `cargo clippy --workspace` ne se construit pas dans cet environnement (bibliothèques système de GTK pour `cassini-app`, construction de `llama-cpp-sys-2`) ; clippy passé sur `cassini-training` et `cassini-core`.
- Reporté à `deferred-work.md` : attendu de S25 dans `ai/tests/plan_reel.rs` à mettre à jour avant la mesure de l'entrée 16.
- Contrôle de l'étape 3 : les azimuts de `recoupement` étaient tirés au hasard (20–70°, 290–340°) et certains plans rendaient `geo.no_intersection` à l'exécution. Ils visent désormais un vrai point au nord des deux stations (`geodesic::inverse`, arrondi à l'unité, angle de recoupement entre 30 et 150°). Nouveau test `the_geometry_steps_of_the_core_run_without_error` : la première étape du noyau de `point_projete`, `recoupement`, `distance_azimut` et `conversion_coordonnees` est exécutée sur chaque exemple.

## Plan Change Log

## Review Triage Log

Passe 1 (quick, 2026-11-07) : high 0, medium 0, low 5, false 0, maybe-false 0 ; 1 relevé de vérification reporté.

| # | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|
| 1 | Points étrangers de `terrain_autre_systeme` tirés loin du MNT (Lambert-93 près de Paris sur un MNT 32N ; observateur et cible à 400 km) | low | rejet | Réel, mais l'inventaire synthétique ne porte aucune emprise de MNT : le plan composé est le même quel que soit le lieu, et l'ancien refus tirait déjà ces points. Contrairement au recoupement, aucun nombre du plan n'en dépend ; le corriger demanderait des tirages Lambert-93 par région. |
| 2 | Critère 4 : `cargo clippy --workspace` non vérifié ici | low | defer | Réel : le poste WSL ne construit ni `cassini-app` (GTK) ni `llama-cpp-sys-2`, cause préexistante ; clippy vert sur `cassini-training` et `cassini-core`, la recette GitHub passe l'espace entier. Noté à `deferred-work.md`. |
| 3 | `lambert_coord`, `utm_coord`, `wgs84_coord` dupliquent les plages de `lambert_origin`, `utm_origin_in`, `origin_point` | low | rejet | Réel, mais `Origin` ne porte ni coordonnées nues ni système écrit (nécessaires au système écrit une fois) ; réécrire les anciens tirages changerait les tournures des familles déjà apprises. Pas de dommage à l'usage. |
| 4 | `docs/entrainement-du-modele.md` l. 104 : « MNT présent pour les deux derniers » devenu faux | low | patch | Ligne réécrite : MNT présent pour l'altitude par zone et le seuil, point d'un autre système renvoyé à `terrain_autre_systeme`. |
| 5 | Commentaire de `terrain_epsg` périmé | low | patch | Commentaire réécrit. |
| 6 | `reprojects_no_point` ne reconnaît que l'ancienne tournure | low | patch | Garde élargi : toute raison qui parle de reprojection et de point ; le refus de reprojeter une couche n'en parle pas (66 tests verts). |

## Design Notes

Choix de commande : les deux systèmes dans {WGS84, UTM, MGRS} → `coord.convert` ; un autre système (Lambert-93) → `data.transform_point`, puis `coord.convert` si MGRS est demandé (catalogue : « un point en Lambert-93 passe d'abord par `data.transform_point` »). Écritures : `{"value": 45, "unit": "deg"}`, grades « 50 gr », « 50 grades » → `gon` ; MGRS du plan en forme canonique sans espace ; WGS84 « 48,8566 N, 2,3522 E (WGS84) » ou « longitude …, latitude … » ; points d'une même demande dans un système, écrit une fois ou par point.

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: tous verts, nouveaux tests compris
- `cargo run -p cassini-training --release` (génération par défaut) -- expected: 8 000 exemples, aucune famille rejetée au résumé
- `cargo clippy --workspace --all-targets -- -D warnings && cargo fmt --check` -- expected: aucun avertissement
