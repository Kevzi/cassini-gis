---
title: 'Gisement : azimut compté depuis le nord du quadrillage'
type: 'feature'
ticket: '28'
created: '2026-11-10'
status: 'built'
baseline_revision: '624835c56a8e277651a0ab35e3f21b660c8cb50b'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-complements-geometrie-et-mise-en-page/change-complements-geometrie-et-mise-en-page.md'
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problème :** `geo.distance_azimuth` et `geo.destination_point` ne comptent les azimuts que
depuis le nord vrai ; un géomètre ou un topographe qui implante sur un plan UTM veut le
gisement, compté depuis le nord du quadrillage.

**Approche :** paramètre `north` (`true` par défaut, `grid`) et `utm_zone` sur les deux
commandes ; le gisement est l'azimut de la géodésique diminué de la convergence des méridiens
au point de mesure (D2), convergence calculée dans le noyau d'après Karney (2011, éq. 23 à 27,
séries de Krüger à l'ordre 6 déjà au code). AD-31 amendée (nord choisi), AD-19 inchangée (UTM
seul, décision du propriétaire D3).

## Boundaries & Constraints

**Always :** salle blanche (formules publiées) ; trigonométrie par `libm` ; angles au
milliardième d'unité ; `north` absent : résultat octet pour octet identique à aujourd'hui
(P2) ; un journal ancien se rejoue à l'identique ; descriptions FR et EN du registre à jour ;
famille `gisement` du générateur, jeu à 8 600, graines inchangées.

**Never :** Lambert-93 (D3) ; nouvelle dépendance ; `north: "magnetic"` ou `date` (entrée 29) ;
gisement de la corde (ΔE, ΔN) ; phrase des recettes ou du jeu tenu à l'écart dans le
générateur ; réordonner `FAMILIES`.

## I/O & Edge-Case Matrix

| Scénario | Entrée | Sortie attendue | Erreur |
|---|---|---|---|
| Défaut | `north` absent | `distance`, `azimuth`, `back_azimuth` comme aujourd'hui, sans autre champ | — |
| Nord vrai écrit | `north: "true"` | même résultat, plus `north: "true"` ; pas de `convergence` | — |
| Gisement | `north: "grid"`, `from` en WGS84 ou UTM | `azimuth` = azimut vrai − γ(from), `back_azimuth` = contre-azimut vrai − γ(to), `north: "grid"`, `convergence` = γ(from) dans `unit` | — |
| Zone imposée | `north: "grid"`, `utm_zone` voisine | quadrillage de `utm_zone` (P3) | — |
| Zone lointaine | `utm_zone` à plus d'une zone du point | — | `coord.out_of_range` |
| `to` loin | `to` à plus d'une zone de la zone du quadrillage, ou hors de 80° S – 84° N | — | `coord.out_of_range` (P1) |
| Zone sans grid | `utm_zone` sans `north: "grid"` | — | `geo.zone_unused` |
| Point projeté | `geo.destination_point`, `north: "grid"` | `azimuth` d'entrée = gisement ; azimut vrai = gisement + γ(from) ; `point`, `north`, `convergence` | — |

</frozen-after-approval>

## Code Map

- `core/src/geodesy.rs` -- `series()`, `conformal_tan`, `tm_forward` (Karney 2011) : ajouter `tm_convergence(lat, dlon)` (γ = γ′ + γ″, éq. 23 à 27) et `grid_convergence(lat, lon, zone)` à côté ; même provenance, citée en tête du module.
- `core/src/angle.rs` -- `Angle::azimuth` ramène dans un tour ; ajouter un angle signé arrondi (`Angle::signed`) pour la convergence ; ajouter `North` (enum `true`/`grid`, sérialisée en minuscules) si partagée par les deux commandes.
- `core/src/commands/geo_point.rs` -- `read_point`, `check_grid_latitude`, `out_of_range`, `zone_distance` : y placer la lecture commune du quadrillage (zone de `from` / zone normale / `utm_zone`, refus P1, P3, `geo.zone_unused`).
- `core/src/commands/geo_distance_azimuth.rs`, `geo_destination_point.rs` -- params, sorties (`skip_serializing_if`), descriptions FR/EN, exemples, liste `.tests`, tests.
- `core/src/commands/coord_convert.rs` -- règle de la zone imposée voisine, à reproduire (message).
- `core/src/journal/tests.rs` -- `hand_journal`, `replay_of` : test de rejeu (journal ancien sans `north`, journal neuf `north: grid`).
- `_bmad-output/.../spec-cassini-mvp/catalogue-des-commandes.md` §7 -- fiches des deux commandes : `north` (`["true","grid"]`), `utm_zone` en `params`, `north`, `convergence` en `result` ; reste planifié pour l'entrée 29 ; tableaux et texte (correction de corde chiffrée, P1 à P3).
- `training/src/families.rs` -- famille `gisement` après `point_projete` dans `FAMILIES` (ajout groupé), fabrique après `point_projete` ; réutiliser `wgs84_or_utm`, `utm_coord`, `azimuth_words`, `geo_distance`, `finish`, `example`.
- `training/src/lib.rs` -- test de la famille de géométrie (`GEOMETRY`) : ajouter `gisement`.
- `docs/entrainement-du-modele.md` -- ligne de la table des familles.
- `qgis-plugin/cassini_qgis/summary.py`, `texts.py` -- résumé : dire « gisement » quand `north` vaut `grid` (affichage seul, AD-5).
- Ne pas toucher : `geodesic.rs`, `geo_bearing_intersection.rs`, le jeu tenu à l'écart.

## Tasks & Acceptance

**Execution :**
- [x] `core/src/geodesy.rs` -- convergence d'après Karney 2011 + tests contre TMcoords.dat (Karney, Zenodo 10.5281/zenodo.32470, CC0), aux deux hémisphères et des deux côtés du méridien central (symétries).
- [x] `core/src/angle.rs`, `geo_point.rs` -- angle signé, `North`, choix du quadrillage.
- [x] `geo_distance_azimuth.rs`, `geo_destination_point.rs` -- paramètres, sorties, descriptions, tests de la matrice (deg et gon).
- [x] `core/src/journal/tests.rs` -- rejeu d'un journal ancien et d'un journal `grid`.
- [x] catalogue -- fiches passées au livré de 28, `core/tests/catalogue.rs` vert.
- [x] `training/` -- famille `gisement`, `check_plan` et garde-fou des coordonnées verts, `recouvrement` vert.
- [x] extension -- résumé du gisement et son test.

**Acceptance Criteria :**
- Given un point de TMcoords.dat, when la convergence est calculée, then elle égale la valeur publiée à 1e-12° près.
- Given un segment de 1 km à 300 km du méridien central, when son gisement est calculé, then il diffère du gisement de la corde de moins de 1″ (correction de corde chiffrée à la fiche).

## Design Notes

Convergence (Karney 2011) : τ′ = latitude conforme ; ξ′ = atan2(τ′, cos λ), η′ = asinh(sin λ /
√(τ′² + cos² λ)) ; γ′ = atan(tan ξ′ · tanh η′) ; p = 1 + Σ 2jα_j cos(2jξ′) cosh(2jη′),
q = Σ 2jα_j sin(2jξ′) sinh(2jη′) ; γ″ = atan2(q, p) ; γ = γ′ + γ″, positive à l'est du méridien
central dans l'hémisphère nord (gisement du nord du quadrillage, compté depuis le nord vrai).

## Verification

Travail dans le worktree `/home/kevin/cassini-worktrees/nord-choisi` (branche
`story/10-28-29-nord-choisi`) ; ne jamais changer de branche, ne rien pousser ni commiter
(le commit vient après la revue). Avant `cargo`, sous WSL :
`export LIBCLANG_PATH=/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/38f6e1c7-3a62-4e6d-a1e2-bc44d994e1ac/scratchpad/clang/x/libclang-18.1.1.data/platlib/clang/native BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`.
Valeurs de référence de la convergence : `TMcoords.dat` (Karney, CC0, méridien central 0,
k0 = 0,9996, colonnes lat lon x y γ k) dans
`/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/2e5a5c8c-be23-49c2-8f90-89ac407890e9/scratchpad/tm/TMcoords.dat` ;
recopier dans le test une quinzaine de lignes à λ ≤ 9,5°, latitudes de 3° à 83° (n° de ligne
cités), pas le fichier. La famille `gisement` écrit des tournures de géomètre (« gisement »,
« en nord du quadrillage », « par rapport au quadrillage UTM », grades fréquents), pour les deux
commandes, `utm_zone` seulement si la zone est écrite ; aucune phrase recopiée de
`ai/tests/`, `tests/epreuve-modele-local/` ni `qgis-plugin/tests/recette/`.

**Commands :**
- `cargo fmt --check && cargo clippy -p cassini-core -p cassini-ai -p cassini -p cassini-training --all-targets -- -D warnings` -- sans avertissement.
- `cargo test -p cassini-core && cargo test -p cassini-training` -- verts (catalogue, recouvrement compris).
- `python -m pytest qgis-plugin/tests/test_summary.py` -- vert.

## Implementation Notes

- Convergence : `geodesy::tm_convergence` / `grid_convergence` ; quinze lignes de TMcoords.dat
  (n° 19 à 784) vertes à 1e-12°, symétries comprises ; ajout demandé en cours de route : la
  table 3d du rapport technique du WMM2025 (80° S, 120° O, zones 10 et 11, ±2,9545046801°),
  verte à 1e-10°.
- `utm_zone` : le lecteur de `coord.convert` déplacé dans `geo_point::utm_zone_param` (même
  message), partagé par les trois commandes.
- Correction de corde mesurée par le test : au plus ≈ 0,76″ pour 1 km à 300 km du méridien
  central (seize directions), chiffrée à la fiche.
- Journal : un résultat de plus de 200 octets est journalisé par son empreinte ; le test du
  journal `grid` vérifie la convergence sur le résultat rendu, et le rejeu compare l'empreinte.
- Générateur : famille `gisement`, poids 3, après `point_projete` ; jeu à 8 600, graines
  inchangées.

## Plan Change Log

## Review Triage Log

Passe 1 (lentille `quick`, 2026-11-10) — high 0, medium 1, low 4, false 0, maybe-false 0.

| Constat | Verdict | Route | Preuve / action |
|---|---|---|---|
| `check_in_grid` : dans la bande X (31X/33X/35X/37X) et en 32V, deux points voisins de part et d'autre d'une limite ont des zones normales à deux numéros d'écart ; `north: grid` refuse à tort l'arrivée | medium | patch | vérifié : `utm_zone(75, 20.9)` = 33, `utm_zone(75, 21.1)` = 35, `zone_distance` = 2. Correctif : accepter aussi à une zone de la zone numérique simple ; test ajouté. |
| `grid_zone` : `utm_zone` 32 ou 34 acceptée en 33X, 33 en 32V | low | rejeté | règle P3 (celle de `coord.convert`), voulue par la party ; la convergence calculée est juste pour la zone demandée. |
| test de la famille `gisement` : `zone {z}` satisfait par le système du point (« UTM zone 31 Nord ») | low | patch | test resserré sur les tournures de la zone du quadrillage. |
| `training/src/lib.rs` : commentaire « six familles » pour sept | low | patch | commentaire corrigé. |
| `docs/entrainement-du-modele.md` : poids total « 324 à 342 » périmé (345 avec `gisement`) | low | patch | nombres mis à jour, 8 600 inchangé. |

## Décisions de party (2026-11-10)

Tranchées en party le 2026-11-10 (géodésien, architecte, géomètre-topographe), à confirmer par
le propriétaire :
- **P1** — `north: grid` et `to` à plus d'une zone UTM de la zone du quadrillage, ou hors de
  80° S – 84° N : refus `coord.out_of_range` (un contre-gisement dans une zone lointaine serait
  un nombre faux d'apparence juste), message qui renvoie au nord vrai.
- **P2** — `north` absent : résultat inchangé, octet pour octet (le rejeu compare le résultat
  entier, AD-11) ; `north` écrit est recopié au résultat ; `convergence` présente pour `grid`
  seulement (champ absent, comme les sorties facultatives de `coord.convert`).
- **P3** — `utm_zone` l'emporte sur la zone de `from`, si elle est la zone normale du point ou
  une voisine (règle de `coord.convert`).
