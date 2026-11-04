---
title: 'Le modèle apprend les trois commandes de terrain'
type: 'feature'
ticket: '6'
created: '2026-11-04'
status: 'done'
baseline_revision: '19dd9b4b8ea5c3de90af9e74873e41273d669e7c'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `terrain.slope`, `terrain.line_of_sight` et `terrain.viewshed` sont au registre (4.2, 4.4, 4.5), mais le générateur civil apprend encore au modèle à les refuser (`NO_SLOPE`, `NO_LINE_OF_SIGHT`, `NO_VIEWSHED` dans `refus_hors_registre`). Un modèle ré-entraîné ainsi dirait « aucune commande ne calcule la pente » alors que le catalogue la montre.

**Approach:** Retirer ces trois refus et leurs exceptions de test, et ajouter des familles qui enseignent les trois commandes : MNT désigné par le `source` de la couche raster de l'inventaire, points en Lambert-93 ou UTM 31N/32N écrits dans la demande, hauteurs et rayon seulement s'ils sont écrits, question quand le MNT ou un point manque. Aucun ré-entraînement ici (passe groupée 5.41).

## Boundaries & Constraints

**Always:** chaque exemple gardé passe `validate` (dont `check_plan`) et sort inchangé de `guard_coordinates` ; le `dem` d'un plan est le `source` d'une couche `kind: raster` de l'inventaire ; le système du MNT dans l'inventaire est celui des points ; vocabulaire civil seulement (CLAUDE.md §2.9) ; aucune phrase des essais réels ni du jeu tenu à l'écart recopiée (`tests/recouvrement.rs`).

**Never:** ré-entraîner ; toucher au noyau, à `ai/` ou au prompt ; un paramètre non écrit dans la demande (hauteur, rayon, réfraction, courbure) dans le plan attendu ; un point inventé ; un `out` dans le plan.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Pente | « carte des pentes du MNT en pourcentage », MNT `donnees/mnt_5m.tif` | `terrain.slope {dem: "donnees/mnt_5m.tif", unit: "percent"}` | — |
| Ligne de vue | deux points Lambert-93, « œil à 2 m », cible sans hauteur | `terrain.line_of_sight {dem, observer, target, observer_height: {2, m}}` | — |
| Champ de vision UTM | point « UTM 31N », « sur 2 km », MNT en 32631 | `terrain.viewshed {dem, observer: {.., epsg: 32631}, radius: {2, km}}` | — |
| Pas de MNT | demande de terrain, aucun raster dans l'inventaire | question qui demande d'ouvrir ou de nommer le MNT | — |
| Point manquant | « depuis le clocher de Thiais » sans coordonnées, ou une ligne de vue avec un seul point écrit | question qui demande le point en X, Y et système | — |
| MNT sans `source` | le raster cité est une couche temporaire | jamais engendré pour un plan : la source du MNT cité est protégée | — |
| Pente sans unité | « fais la carte des pentes du terrain » | question qui demande degrés ou pourcentage | — |
| Seuil de pente | « où la pente dépasse 15 % » | refus affirmatif : aucune commande ne sélectionne des cellules d'un raster par leur valeur | — |

**Décisions du propriétaire (2026-11-04) :**
- Pente sans unité écrite : une question (degrés ou pourcentage), jamais une unité devinée.
- Seuil sur la pente : un refus affirmatif, nouveau motif dans `refus_hors_registre`.
- Ligne de vue : le générateur écrit le système à chaque point, le garde-fou exigeant une occurrence du système par `Point` ; une entrée de `deferred-work.md` demande que le garde-fou accepte un système écrit une fois pour deux points. Le noyau n'est pas touché ici.
- Point et MNT de systèmes différents : non engendré ; le modèle recopie le point, le noyau rend son erreur typée (AD-19).

</frozen-after-approval>

## Code Map

- `training/src/families.rs` :
  - `refus_hors_registre` (l.~2366) : retirer les bras 0, 1, 2 (vue, ligne de vue, pente) et les constantes `NO_VIEWSHED`, `NO_LINE_OF_SIGHT`, `NO_SLOPE` ; garder le MNT pour `NO_RASTER_STATS` ; renuméroter le tirage.
  - Tests l.~2958 (`out_of_registry_requests_are_refused_never_planned`) et l.~3334 (`what_is_refused_out_of_registry_is_still_missing_from_the_registry`) : retirer les trois exceptions ; le test garde « aucun refus ne nomme une commande du registre ».
  - Réutiliser : `utm_origin`, `origin_point` (forme Lambert seulement, pas WGS84), `lambert_point`, `thousands`, `finish`, `calls`, `question`, `example`, `place_with_layer`, le patron de `question_isochrone` (l.~2631) pour la question de point, `question_couche_absente` (l.~2157) pour la question de MNT.
  - Ajouter à `FAMILIES` : `pente`, `ligne_de_vue`, `champ_de_vision`, `question_terrain` (poids de l'ordre de 4).
- `training/src/inventories.rs` : `Builder` gagne de quoi ajouter le MNT cité avec un `epsg` choisi (2154, 32631, 32632) et protégé dans `qgis_shapes` (l.~1443 : pas de retrait de `source` pour lui ; l'`epsg` peut manquer comme ailleurs). Thème `mnt` (l.~1127), seul thème raster.
- `training/src/lib.rs` `validate` (l.~55) : pour un appel portant `dem`, refuser s'il n'est pas le `source` d'une couche raster de l'inventaire.
- `docs/entrainement-du-modele.md` table des familles (l.~73) : quatre lignes, ligne `refus_hors_registre` mise à jour.
- Lecture seule : `core/src/commands/terrain_{slope,line_of_sight,viewshed}.rs` (paramètres, défauts 1,6 m / 0 m / 5 km), `core/src/point.rs` (`guard_coordinates`), `training/tests/recouvrement.rs`.

## Tasks & Acceptance

**Execution:**
- [x] `training/src/inventories.rs` -- MNT cité à système choisi et source protégée -- le plan cite un chemin qui doit exister
- [x] `training/src/lib.rs` -- contrôle du `dem` dans `validate` + test -- un chemin de MNT inventé ne passe pas
- [x] `training/src/families.rs` -- retrait des trois refus, quatre familles, tests de la matrice (chaque famille valide, garde-fou inchangé, `dem` = source raster, paramètres non écrits absents, formes UTM présentes, questions) -- l'enseignement des trois commandes
- [x] `docs/entrainement-du-modele.md` -- table des familles -- la doc suit le générateur
- [x] `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- une entrée (format du fichier, `source_plan` = ce plan) : le garde-fou accepte un système écrit une fois pour deux points -- décision 3

**Acceptance Criteria:**
- Given `cargo test -p cassini-training`, when il tourne, then tout est vert, `recouvrement` compris.
- Given le jeu par défaut engendré (`cargo run -p cassini-training`), when on lit le résumé par famille, then les quatre familles ont des exemples gardés et aucun refus ne cite le terrain.

## Implementation Notes

- `Builder::add_dem(rng, epsg)` ajoute le MNT cité ; `qgis_shapes` tire toujours le retrait de `source` (la suite des autres familles ne bouge pas) mais l'ignore pour lui.
- `lambert_point` (refus retirés) devient `lambert_origin`, sans la forme « X / Y (Lambert-93) » ; `utm_origin` délègue à `utm_origin_in(rng, zone)`, à suite de tirage identique.
- `refus_hors_registre` : 5 bras (cadrage, A4, statistiques raster, origines multiples, seuil de pente `NO_RASTER_THRESHOLD`).
- `redemande` relevé de 20 à 24 : les plans de terrain ne se corrompent pas, le taux tombait à 3,6 % (borne du test : 4 %).
- « Calcule la pente du terrain en pourcentage » est la phrase S22 de l'épreuve : les gabarits disent « Calcule les pentes ».
- Test de `lib.rs` sur la question de coordonnées : `question_terrain` y est admise à côté des trajets.
- Après l'implémentation : « les coordonnées de le clocher » corrigé en « du clocher » (`point_question`), avec un test.
- Correctifs de relecture : `add_dem` écrit la source sous les formes de 4.3 (`donnees/`, `dem/`, `../donnees/`, nom seul), ce qui répond à l'entrée de `deferred-work.md` de 4.3 sur les sources raster relatives ; un MNT en UTM ne s'appelle jamais `rge_alti_1m`.
- Critère du jeu par défaut vérifié (graine 1) : `pente` 175, `ligne_de_vue` 169, `champ_de_vision` 155, `question_terrain` 156 exemples gardés, aucun écarté ; 656 refus, aucun ne cite pente, ligne de vue, champ de vision ni visibilité.

## Plan Change Log

## Review Triage Log

**Passe 1 (quick), 2026-11-04** — 1 medium, 3 low, 1 rejeté low, 1 false.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | `ai/tests/plan_reel.rs` attend encore un refus pour S20-S24 ; `sentences.json` écrit `dem` en id de couche et `from`/`to` | medium | defer | Attendu périmé depuis 4.2/4.4/4.5, pas causé par 4.6 ; bloque la mesure de 5.41. Entrée ajoutée à `deferred-work.md`. |
| 2 | L'entrée 4.3 de `deferred-work.md` (sources raster relatives) reste ouverte ; toutes les sources de MNT sont `donnees/x.tif` | low | patch | `add_dem` tire `donnees/`, `dem/`, `../donnees/` ou le nom seul ; test des quatre formes. |
| 3 | La ligne `ligne_de_vue` de la doc montre le système écrit une seule fois, contraire à la décision 3 | low | patch | Exemple corrigé (système écrit à chaque point). |
| 4 | Un MNT `rge_alti_1m` (Lambert-93 seulement) peut porter un code UTM | low | patch | Corrigé avec le constat 2 : noms neutres en UTM, test. |
| 5 | Rayon de 800 m à 3 km sur un MNT nommé « 1 m » : le noyau refusera | low | rejeté | Le plan recopie le rayon écrit, ce qu'il doit faire ; le refus typé du noyau est le comportement voulu (borne de 500 cellules, 4.5). Le modèle ne connaît pas la taille de cellule. |
| 6 | Le critère « aucun refus ne cite le terrain » n'est vérifié que sur `refus_hors_registre` | false | — | Les motifs de `refus` sont des chaînes fixes, aucune ne parle de terrain (le seul « vue » est « vue en 3D ») ; `redemande` ne produit pas de refus ; le jeu par défaut vérifié : 656 refus, zéro mention. |

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: vert
- `cargo fmt --check && cargo clippy -p cassini-training --all-targets -- -D warnings` -- expected: aucun avertissement
