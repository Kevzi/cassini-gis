---
title: 'Familles du générateur pour les nouvelles commandes — lot 2b-2, critères, masques et superposition'
type: 'feature'
lot_de_ticket: '12' # lot du ticket 12 ; le plan du ticket est celui du lot 3
created: '2026-11-07'
status: 'built'
baseline_revision: 'aea48552199f486227328f31ec6192f72ae4c49a'
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

**Problem:** Le générateur (`training/`) n'a aucune famille pour `raster.weighted_overlay`, et `raster.reclassify` n'y apparaît que sur la pente (`step:1`). L'inventaire synthétique n'a ni raster de critère, ni masque, ni raster brut ; le ré-entraînement groupé (entrée 16) ne peut pas partir de là.

**Approach:** Ajouter à l'inventaire des thèmes raster civils — critères notés, masques, rasters bruts (distance aux routes dans le groupe `voirie`, bruit) — puis deux familles, `superposition_ponderee` et `reclassement_raster`. Le validateur contrôle `criteria[i].raster` et `exclusions[i]` contre les `source` raster de l'inventaire. Lot 2b-2 du ticket (découpage du propriétaire, 2026-11-07) ; l'export au format reste au lot 3.

## Boundaries & Constraints

**Always:** chaque exemple passe `validate` avec sa demande ; vocabulaire civil (CLAUDE.md §2.9) ; tout raster cité est la `source` protégée d'une couche raster de l'inventaire (`Builder::add_raster`) ; chaque critère, masque et seuil cité est écrit dans la demande ; critères et masques dans l'ordre écrit ; poids écrits recopiés, sinon tous à 1 ; classes à bornes contiguës, valeurs 1, 2, 3 dans l'ordre quand elles ne sont pas écrites ; part des redemandes entre 4 et 7 %.

**Never:** `step:k` dans `criteria` ou `exclusions` ; le MNT, le raster de coût ou un dérivé du MNT comme critère ou masque ; un masque raster et la couche vecteur de même sujet dans un même inventaire (même groupe) ; `ONLY_A4` ni la famille d'export touchés (lot 3) ; `pente_seuil` changée ; changement du noyau, de `cassini-ai`, du prompt ou de l'extension ; entraînement ou mesure (entrée 16) ; `DEFAULT_COUNT`, `DEFAULT_SEED` changés.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Superposition | « Combine l'accessibilité (poids 2) et le calme (poids 1), en excluant les zones inondables » | `raster.weighted_overlay` `{criteria: [{raster, weight: 2}, {raster, weight: 1}], exclusions: [masque]}` | — |
| Poids non écrits | « Superpose l'accessibilité, l'ensoleillement et la proximité des services à poids égaux » | trois critères, `weight: 1` chacun, sans `exclusions` | — |
| Reclassement brut | « Classe la distance aux routes en moins de 100 m, de 100 à 500 m et plus de 500 m » | `raster.reclassify` `{raster: source, classes: [{max:100,value:1},{min:100,max:500,value:2},{min:500,value:3}]}` | seuil unique « où le bruit dépasse 65 dB » → `[{min: 65, value: 1}]` |
| Raster absent | un critère ou le raster à reclasser nommé, absent de l'inventaire | question qui demande d'ouvrir ou de nommer ce raster, jamais un plan | — |

</frozen-after-approval>

## Code Map

- `training/src/inventories.rs` -- `THEMES` (l. 1127–1164, `mnt` puis `cout`) : ajouter, en `Geom::Raster`, `fields: &[]`, quatre critères notés (accessibilité, ensoleillement, proximité des services, calme ; un groupe chacun), deux masques (zones inondables dans le groupe `inondables`, espaces protégés dans le groupe `znieff`), deux rasters bruts (`distance_routes` dans le groupe `voirie`, `bruit` dans son groupe). Aucun nom ni fichier qui contienne « pente », « MNT », « altitude », « relief », « coût ». `Builder::add_raster` (l. 1412) sert tel quel. `finish` (l. 1480) tire un distracteur raster sur quatre parmi tous les rasters : voulu.
- `training/src/families.rs` -- `FAMILIES` (l. 43, `cout_cumule`/`chemin_moindre_cout` l. 268–275 avant `redemande`) : deux familles, poids 4. Patrons : `cout_cumule` (l. 3303), `cost_words`, `cout_absent` (l. 3427, `missing_layer_question`, `theme(..).of_none`), `pente_seuil` (l. 3162, bornes et formulations de classes), `enumerate` (l. 3023), `finish`, `example`, `calls`, `question`.
- `training/src/lib.rs` -- `validate` (l. 85–130) : boucle `RASTER_PARAMS` sur les paramètres de premier niveau ; ajouter le contrôle de `criteria[i].raster` et de `exclusions[i]` (dans `dems`, jamais `step:`). Patron de test : `a_cost_raster_is_the_source_of_a_raster_of_the_inventory` (l. 422) ; test de jeu par défaut : `the_cost_families_of_the_default_set_cite_a_cost_raster_never_the_dem` (l. 579).
- Catalogue `spec-cassini-mvp/catalogue-des-commandes.md` l. 1062–1125 -- `criteria` 2 à 8, `weight` > 0, `exclusions` 0 à 8, `classes` 1 à 32 `{min?, max?, value}` ; `out` jamais écrit.
- `training/tests/recouvrement.rs` -- interdit de reprendre les phrases des essais réels.
- `docs/entrainement-du-modele.md` -- table des familles (l. ~117–125), section lot 2b-1 (l. 225–255) : ajouter « 11.12, lot 2b-2 », mettre à jour la phrase sur les listes non contrôlées (l. 215) et le reste à faire (lot 3 seul).

## Tasks & Acceptance

**Execution:**
- [x] `training/src/inventories.rs` -- thèmes critères, masques, bruts ; test : aucun nom interdit, masques dans le groupe de leur vecteur, source citée protégée -- matière des familles.
- [x] `training/src/lib.rs` -- `criteria[i].raster` et `exclusions[i]` contrôlés ; tests : raster inventé ou `step:1` en liste refusé, `source` admise -- refuser l'invention.
- [x] `training/src/families.rs` -- `superposition_ponderee` (2 à 4 critères, poids écrits une fois sur deux sinon tous à 1, 0 à 2 masques ; 1 sur 8 un critère absent → question) et `reclassement_raster` (un raster brut, seuil unique ou deux à trois classes, valeurs écrites une fois sur trois ; 1 sur 8 raster absent → question) -- couvrir les deux usages.
- [x] `training/src/families.rs` (tests) -- par famille : commande et clés attendues seules, rasters cités sources des bons thèmes, ordre écrit, poids et bornes retrouvés dans la demande, classes contiguës, aucun `step:` en liste, questions sans plan -- vérifier sans modèle.
- [x] `docs/entrainement-du-modele.md` -- lignes des familles, section lot 2b-2, reste à faire -- doc à jour.

**Acceptance Criteria:**
- Given le jeu par défaut (graine 1, 8 000 exemples), when il est engendré, then les deux familles gardent chacune au moins un exemple, aucune n'est rejetée au résumé, `raster.weighted_overlay` figure dans au moins un plan et `raster.reclassify` dans au moins un plan sans `terrain.slope`.
- Given le même jeu, when on lit chaque plan, then aucun critère ni masque n'est le `source` du MNT, d'un raster de coût, ni absent de l'inventaire.
- Given `cargo test -p cassini-training`, `cargo clippy -p cassini-training -p cassini-core --all-targets -- -D warnings` et `cargo fmt --check`, when ils tournent, then tout est vert.

## Implementation Notes

- Thèmes regroupés en `CRITERIA`, `MASKS`, `RAW_RASTERS` (`inventories.rs`) ; le MNT (1 sur 4) et le raster de coût (1 sur 6) posés en distraction dans les deux familles.
- Graine 1, 8 000 exemples : `superposition_ponderee` 122, `reclassement_raster` 130, aucun rejet ; 363 redemandes (4,5 %), poids de `redemande` inchangé (44).
- Test de jeu par défaut ajouté dans `lib.rs` : `the_overlay_families_of_the_default_set_cite_criteria_and_masks_never_the_dem`.

## Plan Change Log

## Review Triage Log

Passe 1 (quick, 2026-11-07) : high 0, medium 0, low 3, false 0, maybe-false 0.

| # | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|
| 1 | `deferred-work.md` : entrée « 11.12 lot 2b-2 » restée ouverte alors que la doc dit le lot fait | low | patch | `status` et `resolution` ajoutés, comme l'entrée close de 2b-1. |
| 2 | Test du reclassement : `contains(" m")` passe sur « moins de », « Montre » ; l'unité n'est jamais vraiment contrôlée | low | patch | Chaque borne contrôlée suivie de son unité ; aucune « dB » dans une demande de distance. |
| 3 | `validate` admet la `source` du MNT ou du raster de coût en critère ou en masque | low | rejet | Même constat rejeté en 2b-1 (#5) : le test du jeu par défaut contrôle tous les critères et masques ; un contrôle par thème dans le validateur ajouterait de la complexité sans défaut observé. |

## Design Notes

Critère désigné par son sujet (« l'accessibilité », « la note de calme ») ou par le nom de sa couche ; masque par « en excluant les zones inondables », « hors espaces protégés ». Poids écrits : « avec un poids de 2 pour… », « l'accessibilité compte double ». Unités des bruts : mètres pour la distance, dB pour le bruit ; valeurs écrites : « notées 3, 2 et 1 ».

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: tous verts, nouveaux tests compris
- `cargo run -p cassini-training --release` -- expected: 8 000 exemples, aucune famille rejetée, redemandes entre 4 et 7 %
- `cargo clippy -p cassini-training -p cassini-core --all-targets -- -D warnings && cargo fmt --check` -- expected: aucun avertissement
