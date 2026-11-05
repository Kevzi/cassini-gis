---
title: 'Distance, azimut, point projeté et recoupement'
type: 'feature'
ticket: '3'
created: '2026-11-05'
status: 'built'
baseline_revision: 'd9e79039044ae5024bb4159b15263992c5b858ea'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le noyau convertit un point (11.2) mais ne sait ni mesurer une distance et un azimut entre deux points, ni placer un point à un azimut et une distance, ni recouper deux visées : ces demandes courantes n'entrent dans aucun plan.

**Approach:** ajouter les géodésiques de Karney (problèmes direct et inverse sur l'ellipsoïde WGS84) au module de géodésie de 11.2, le type partagé `Angle`, et les trois commandes `geo.distance_azimuth`, `geo.destination_point`, `geo.bearing_intersection` exactement comme les figent leurs fiches du catalogue (§6) ; servies sans autre câblage par `cassini run`, le canal et MCP, et affichées en clair par le panneau.

## Boundaries & Constraints

**Always:** paramètres, résultats et erreurs exactement ceux des fiches et du type `Angle` (catalogue §3, AD-31) ; points acceptés 4326, 32601–32660, 32701–32760, tout autre → `geo.unsupported_crs` nommant `data.transform_point` (et `coord.convert` pour une référence MGRS) ; azimuts depuis le nord vrai, sens horaire, rendus dans [0, 360) deg ou [0, 400) gon ; distances rendues en `Distance` `m` ; résultat `point` dans le système de `from` / `first` ; toute trigonométrie par `libm` (rejeu identique Windows/Linux) ; arrondis rendus : WGS84 au milliardième de degré, UTM et distances au millimètre, angles au milliardième d'unité ; chaque jeu de référence cité (source, licence) dans le test ; tests sans QGIS ; comportement et messages de `coord.convert` inchangés.

**Never:** aucune dépendance nouvelle (décision de 11.2 : calculs écrits par nous d'après les publications de Karney, `libm` seul) ; aucune conversion insérée en silence ; pas de distance plane ; pas de logique de calcul dans l'extension ; pas de famille d'entraînement ni de ré-entraînement (entrées 12 et 16) ; pas de seuil métier.

**Decisions (propriétaire, 2026-11-05):** deux visées sont « parallèles » quand elles se coupent sous un angle inférieur à 1°, ou portent la même géodésique : `geo.no_intersection`, le message donne l'angle ; un recoupement en UTM dont le point tombe à plus d'une zone de `first` → `coord.out_of_range`, comme `geo.destination_point`, précisé au texte de la fiche (identifiants et paramètres inchangés) ; le plan est gardé entier (~2 600 tokens). Décision après relecture (propriétaire, 2026-11-05) : un `point` rendu en UTM garde la zone de `from` / `first`, l'hémisphère suivant la latitude (326NN ↔ 327NN), comme `coord.convert` ; la fiche le dit.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Inverse | deux points WGS84 du jeu GeographicLib | distance au mm, azimuts à 1e-9° près des références | — |
| Systèmes mêlés | `from` en 32631, `to` en 4326 | même résultat qu'en 4326 des deux côtés | — |
| Grades | `unit: gon` | azimuts dans [0, 400) | — |
| Quasi-antipodes | points du jeu GeographicLib proches de l'antipode | références retrouvées | — |
| Points confondus | `from` = `to` | — | `geo.coincident_points` |
| Direct | départ, azimut, distance (`km` compris) | point du jeu de référence, système de `from` | — |
| Direct, sortie de zone | départ 32631, arrivée à plus d'une zone | — | `coord.out_of_range` |
| Recoupement | deux stations, visées sécantes en avant | point, deux distances cohérentes avec l'inverse | — |
| Visées parallèles / divergentes | visées qui ne se coupent qu'en arrière, sous moins de 1°, ou sur la même géodésique (deux visées nord voisines) | — | `geo.no_intersection`, angle donné |
| Recoupement hors zone | stations en 32631, point à plus d'une zone | — | `coord.out_of_range` |
| Lambert-93 | un point 2154 dans chaque commande | — | `geo.unsupported_crs` nommant `data.transform_point` |
| Angle hors bornes | `value` non fini | — | erreur de paramètre |

</frozen-after-approval>

## Code Map

- `core/src/geodesy.rs` -- constantes `WGS84_A`, `WGS84_F`, `Utm`, `to_utm`, `from_utm`, `utm_zone`, `zone_distance`, `central_meridian`, `wrap_longitude`, `MIN/MAX_LATITUDE` : à réutiliser, ne pas modifier le Mercator transverse ni le MGRS.
- `core/src/commands/coord_convert.rs` -- `read_point` (l. ~119, contrôle WGS84/UTM, `geo.unsupported_crs`), `rounded` (l. ~113), `out_of_range` : les extraire dans un module partagé, paramétré par l'identifiant de commande dans le message, sans changer ce que rend `coord.convert` (ses tests le prouvent).
- `core/src/commands/analysis_buffer.rs` -- `Distance`, `DistanceUnit`, `metres()` : type à réutiliser en paramètre et en résultat (valeur > 0 à la lecture).
- `core/src/point.rs` -- `Point` (marque `x-cassini: point`) et la garde de provenance ; un résultat `point` est déjà repris comme `Point` par `step:` (`reference_is_a_point`) : ne rien changer.
- `core/src/registry.rs` -- `Command`, `CommandSpec::new::<P,O>(...).runs_in(RunsIn::Core).example(...).tests(&[...])`, `CommandFailure::new`, `inventory::submit!`.
- `core/src/commands/mod.rs`, `core/src/lib.rs` -- déclarer les modules.
- `core/tests/catalogue.rs` -- `NAMED_TYPES` contient déjà `Angle` ; une fiche `planifiee` enregistrée fait échouer la recette : passer les trois fiches et le §4 en `faite`.
- `cli/tests/parity.rs` -- passe l'`example` de toute commande `core` par run, serve et MCP : un exemple valide par commande suffit.
- `ai/src/plan.rs` -- `offered()` tire prompt long et grammaire du registre : rien à faire.
- `qgis-plugin/cassini_qgis/summary.py` (`_coord_convert` l. 316, table l. 356), `texts.py`, `tests/test_summary.py` -- modèle de phrase FR/EN.
- `training/` -- ne pas modifier.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/angle.rs` (+ `lib.rs`) -- type `Angle {value, unit: deg|gon}` (défaut `deg`, valeur finie), conversions en degrés et normalisation d'un azimut rendu.
- [x] `core/src/geodesic.rs` (+ `lib.rs`) -- problèmes direct et inverse de Karney (séries d'ordre 6 en n, Newton sur α1, départ par l'astroïde pour les quasi-antipodes) et recoupement (voir Design Notes) ; provenance citée en tête comme `geodesy.rs`.
- [x] `core/src/commands/geo_point.rs` -- lecture partagée d'un `Point` en WGS84 et rendu dans un système de la liste ; `coord_convert.rs` s'y branche.
- [x] `core/src/commands/geo_distance_azimuth.rs`, `geo_destination_point.rs`, `geo_bearing_intersection.rs` (+ `mod.rs`) -- les commandes, leurs erreurs, leurs tests : jeu d'essai GeographicLib (extrait embarqué : directs, inverses, quasi-antipodes, licence vérifiée et citée) et toute la matrice.
- [x] `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- trois fiches et §4 en `faite`, précisions sans changer identifiants ni paramètres.
- [x] `qgis-plugin/cassini_qgis/summary.py`, `texts.py`, `tests/test_summary.py` -- phrases FR/EN qui montrent distance et azimuts, le point atteint, le point de recoupement.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when on lance `cargo test --workspace`, then les références GeographicLib sont retrouvées et la recette du catalogue passe.
- Given le serveur MCP lancé, when on liste ses outils, then les trois commandes y figurent et s'appellent.
- Given QGIS et le vrai modèle, when Claude envoie par la sonde « distance et azimut entre … et … », then un plan `geo.distance_azimuth` est exécuté et le panneau montre distance et azimut ; le journal se rejoue à l'identique.

## Implementation Notes

- `geodesic.rs` : direct et inverse de Karney (2013), écrits d'après l'article ; coefficients et tolérances confrontés à GeographicLib 2.0 (MIT), dont les 20 géodésiques GeodTest et 10 cas GeodSolve sont embarqués (`geodesic::reference`). Contrôle croisé hors dépôt le 2026-11-05 sur 20 000 paires tirées au hasard (30 % quasi antipodales, 20 % courtes) contre GeographicLib Python 2.0 : distance à 7,5e-9 m, azimuts à 1,1e-9° (une géodésique de 14 m), direct à 2e-13°.
- Recoupement : départ sphérique, Newton sur (s1, s2) ; la direction de la seconde visée est ramenée au plan tangent de la première extrémité par la rotation azi2 − azi1 de l'inverse qui les joint. Écart d'arrêt 1 µm. Contrôle croisé sur 20 000 paires de visées : aucun échec de convergence ; tous les écarts venaient d'intersections au-delà de l'antipode d'une station. D'où une précision au texte de la fiche : « en avant » s'entend à moins d'un demi-tour, la visée devant rester la géodésique la plus courte de la station au point (sinon `geo.no_intersection`).
- Angle de recoupement : contrôlé sur la sphère avant Newton, puis à l'arrivée sur l'ellipsoïde ; le message le donne au millième de degré, à la française.
- Point rendu en UTM : même zone que le point d'entrée, hémisphère selon la latitude (comme `coord.convert`) ; hors de la grille (80° S – 84° N) ou à plus d'une zone → `coord.out_of_range`. Précisé au texte des fiches, identifiants et paramètres inchangés.
- `geo.coincident_points` : distance arrondie au millimètre nulle.
- `geo_point.rs` porte `read_point` (paramétré par l'identifiant de commande ; pour `coord.convert` le message est identique à l'ancien, pour `geo.*` il nomme aussi `coord.convert` pour une référence MGRS), `rounded`, `out_of_range`, `check_grid_latitude` et le rendu des points.
- Fiches déplacées de la section 6 à la section 5 (commandes faites), comme `coord.convert` en 11.2.
- Tests ajoutés hors du cœur : `cli/tests/mcp.rs` (les trois outils listés et appelés comme `cassini run`), `cli/src/main.rs` (exécution sans QGIS et rejeu identique des trois commandes).
- Tests sous WSL pour `core` ; `cargo test --workspace` et clippy de l'espace de travail par PowerShell (`LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`).

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 0, medium 1, low 7, false 1, maybe-false 0 ; rejetés en low 3.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Point rendu en 327NN pour un départ en 326NN qui passe l'équateur, contre « système de `from` » du bloc figé | medium | intent_gap → résolu | propriétaire, 2026-11-05 : même zone, hémisphère suivi ; décision écrite au bloc figé, code gardé, fiche corrigée en patch |
| Fiches : lignes de résultat « dans le système de `from` » | low | patch | contredit la décision ci-dessus ; texte et commentaires reformulés |
| Point d'entrée invalide → `coord.out_of_range`, absent des fiches | low | patch | `geo_point.rs` `read_point` ; cas ajouté au texte des trois fiches |
| Points confondus : « moins d'un millimètre » alors que le seuil est l'arrondi au mm (< 0,5 mm) | low | patch | `geo_distance_azimuth.rs` l. 82, catalogue l. 777 |
| `french_degrees` : 0,9996° écrit « 1,000°, inférieur à 1° » | low | patch | `format!("{angle:.3}")` arrondit |
| Boucle de Newton : `numit` incrémenté avant les tests, écart à GeographicLib ; `latsign`/`lonsign` sans signbit | low | patch | `geodesic.rs` l. 646-647 |
| Lignes vides PEP 8 autour du nouveau bloc Python | low | patch | `summary.py`, `test_summary.py` |
| `cargo fmt --check` échouerait (lignes > 100) | false | rejeté | `cargo fmt --check` rend 0 sur l'arbre |
| Seuil de 1° et verdict « en arrière » jugés sur la sphère avant l'ellipsoïde | low | rejeté | écart de quelques centièmes de degré au seuil seulement ; le corriger ajoute une seconde passe ellipsoïdale |
| Stations confondues, ou station sur l'autre visée : message « en arrière » | low | rejeté | cas rare, la commande refuse déjà sans rendre de point faux |
| Critère 3 (sonde QGIS, vrai modèle) sans preuve | — | noté | non joué ; le modèle n'a pas encore d'exemples (entrées 12 et 16), à faire à la présentation |

## Design Notes

Recoupement : départ sphérique (plans des grands cercles), puis Newton sur les deux longueurs (s1, s2) : à chaque pas, direct depuis chaque station, inverse entre les deux extrémités, système 2×2 dans le plan tangent (la dérivée d'une extrémité par rapport à sa longueur est l'unité selon son azimut d'arrivée). Seule l'intersection avec s1 > 0 et s2 > 0, la plus proche, compte ; sinon `geo.no_intersection`.

## Verification

**Commands:**
- `cargo test --workspace` -- expected: tout vert, dont `catalogue`, `parity` et les tests de `training`
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert

**Manual checks (if no CLI):**
- Sonde de bout en bout dans QGIS 3.44 (paquet `--features gpu`, Qwen) : plan `geo.distance_azimuth` exécuté, valeurs affichées, rejeu identique.
