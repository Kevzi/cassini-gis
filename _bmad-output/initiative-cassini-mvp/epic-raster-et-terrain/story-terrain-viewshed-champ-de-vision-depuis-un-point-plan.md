---
title: 'terrain.viewshed : champ de vision depuis un point'
type: 'feature'
ticket: '5'
created: '2026-11-04'
status: 'done'
baseline_revision: 'b23d5c036bf4e4a383b4fe180e118ff63d39deae'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 1
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le noyau dit si un point en voit un autre (4.4), mais pas ce qu'on voit depuis un point. C'est la dernière des trois commandes de terrain du MVP (CAP-11), en Rust et sans GPL, base du module privé.

**Approach:** Ajouter au registre `terrain.viewshed`, exécutée dans le noyau. Elle prend un MNT (`dem`), un observateur (`Point` dans le système du MNT), les hauteurs de l'observateur et de la cible, un rayon maximal et l'option courbure et réfraction. Elle écrit un GeoTIFF nommé par le noyau : 1 visible, 0 non visible, sans donnée hors rayon. Chaque cellule est jugée par **le même calcul que `terrain.line_of_sight`** vers son centre (algorithme exact « R3 », Franklin et Ray 1994), réimplémenté depuis la littérature publique, sans code de Whitebox.

## Boundaries & Constraints

**Always:**
- MNT lu et sortie écrite par `dem.rs` seul (AD-20) ; aucune reprojection (AD-19).
- Paramètres :
  - `observer_height` et `target_height` : le type `Height` de 4.4, défauts 1,6 m et 0 m.
  - `curvature` (vrai par défaut) et `refraction` (0,13 par défaut) : comme en 4.4.
  - `radius` : `Distance {value, unit}` de `analysis.buffer`, strictement positif, 5 km par défaut.
- **Borne (AD-8 ; décision du propriétaire, 2026-11-04).**
  - Un rayon donné est refusé s'il dépasse **500** fois la plus petite cellule, par l'erreur typée `terrain.viewshed_too_large` : 12,5 km à 25 m, 2,5 km à 5 m, 500 m à 1 m.
  - Le calcul n'est pas annulable. Son coût croît comme le cube du rayon en cellules : cette borne le garde à environ 5 s au pire (tout visible, MNT plat sans courbure), mesuré 5,2 s (décision du propriétaire, 2026-11-04 : garder 500).
- **Rayon par défaut (décision du propriétaire, 2026-11-04).** Sans `radius`, le rayon vaut 5 km, ramené à la borne du MNT si 5 km la dépassent ; il n'y a pas de refus.
- **Le modèle connaît la borne.** Les descriptions FR et EN de la commande la disent.
- **Sortie.**
  - GeoTIFF Float32, sans-donnée -9999, aligné sur la grille du MNT et rogné à l'emprise du cercle intersectée avec le MNT.
  - Une cellule est dans le rayon si son centre est à moins du rayon de l'observateur, bord compris.
  - Une cellule sans donnée du MNT est sans donnée en sortie. La cellule de l'observateur vaut 1.
- **Concordance.** Pour toute cellule valide, la valeur vaut 1 si et seulement si `line_of_sight` vers son centre rend `visible: true`, avec les mêmes paramètres.
- **Reproductibilité.** Calcul en f64, déterministe : même entrée, mêmes octets, et le rejeu rend `identical: true`.
- **Provenance.** Elle est écrite en tête du module et dans le message de commit, sans code repris.
  - Le principe est celui de R3 d'après la publication : une ligne de vue propre vers chaque cellule.
  - L'échantillonnage est celui de `terrain.line_of_sight` : pas constant, interpolation bilinéaire. Ce n'est pas l'interpolation aux croisements de la grille du papier, et le texte le dit.

**Never:**
- Les algorithmes approchés (R2, XDraw) : la vitesse se compare en 4.7.
- Dépendre de GDAL, de GEOS, de GRASS ou d'une nouvelle crate.
- Apprendre la commande au modèle : le refus `NO_VIEWSHED` du générateur reste.
- Toute fonction conçue pour un usage militaire.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Colline | `mnt-colline-2154.tif`, observateur (652030, 6850250) à 2 m, rayon 1 km | cellules du versant ouest à 1, versant est derrière le sommet à 0, hors MNT rogné | — |
| Concordance | même cas, un échantillon déterministe d'au moins 200 cellules | chaque valeur égale au verdict de `line_of_sight` vers le centre | — |
| Rayon | rayon 100 m | cellules à plus de 100 m sans donnée, sortie rognée au carré du cercle | — |
| Trou | MNT troué dans le test | la cellule trouée est sans donnée, elle ne masque rien | — |
| Trop grand | rayon 20 km sur la colline (cellule 10 m) ; 5 km pile accepté | rien n'est écrit | `terrain.viewshed_too_large` |
| Défaut ramené | MNT de cellules de 1 m construit dans le test, sans `radius` | le calcul se fait avec un rayon de 500 m, sans refus | — |
| Cellule de l'observateur | rayon 1 m sur la colline | la cellule de l'observateur vaut 1 et compte comme visible, les autres sont sans donnée | — |
| Ordre des refus | observateur en EPSG:4326 et rayon de 20 km | — | `terrain.crs_mismatch`, pas la borne |
| Mauvais système, hors MNT, sans altitude | observateur en EPSG:4326, hors emprise, sur un trou | rien n'est écrit | `terrain.crs_mismatch`, `terrain.outside_dem`, `terrain.no_elevation` (mêmes contrôles qu'en 4.4) |
| Paramètres | rayon nul ou nombre nu, hauteur négative | — | `registry.invalid_params` |
| CLI et rejeu | `cassini run terrain.viewshed` sans QGIS, puis `cassini replay` | `identical: true`, GeoTIFF identique octet pour octet | — |
| Mesure | MNT de 25 m généré dans le test, rayon 10 km, puis 12,5 km (la borne) sur MNT plat | temps mesurés et consignés dans le plan, à titre d'information | — |

</frozen-after-approval>

## Code Map

- **`core/src/commands/terrain_line_of_sight.rs`** :
  - extraire de `line_of_sight` un noyau partagé, `Line` et `sight(dem, line, eye, z_tgt, target_height, curvature, refraction, stop_at_first) -> Sight` (voir le KEEP du Plan Change Log), que les deux commandes appellent ;
  - rendre `pub(crate)` : `Height`, `MetreUnit`, `bilinear`, `check_crs`, `check_extent`, `endpoint_elevation`, `EARTH_RADIUS` ;
  - ses 12 tests restent verts sans changement de comportement.
- **`core/src/commands/terrain_viewshed.rs`** (nouveau ; `build.rs` déclare les modules de `commands/`) :
  - patron `terrain_slope.rs` : `RunsIn::Core`, `.cancellable(false)`, `.path_params(&["dem"])`, `.output_params(&["out"])`, `.output_ext(".tif")`, `.example(..)`, `.tests(&[..])` avec des noms de tests qui existent, `inventory::submit!` ;
  - descriptions FR et EN ;
  - `Distance` (`analysis_buffer.rs`) pour `radius` ;
  - écriture : construire un `Dem` rogné et appeler `write_geotiff`, en copiant les champs un à un comme `horn_slope`.
- **`core/src/dem.rs`** : lecture seule ; utiliser `Dem::open`, `value`, `extent`, `write_geotiff`.
- **`training/src/families.rs`** (vers la ligne 3345) : exclure `terrain.viewshed` du garde-fou, comme `terrain.line_of_sight`.
- **`cli/src/main.rs`** (vers la ligne 804) : test `run` puis `replay`, patron `terrain_slope_runs_without_qgis_and_replays_identically`.
- **`cli/tests/parity.rs`** : les exemples doivent passer sous `run`, `serve` et `mcp` ; il couvre déjà la sortie fichier de la pente.
- **`ai/src/plan.rs`** : le test `the_catalog_comes_from_the_real_registry_and_hides_output_paths` doit rester vert.
- **`fixtures/README.md`** : ajouter `terrain.viewshed` (4.5) dans la colonne « Utilisé par » de la colline.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/terrain_line_of_sight.rs` — extraire le noyau partagé, sans changer le comportement — c'est ce qui garantit la concordance.
- [x] `core/src/commands/terrain_viewshed.rs` — la commande, une fonction pure `viewshed(&Dem, &params) -> (Dem, stats)` et les tests de la matrice ; la mesure à 10 km dans un test `#[ignore]`, lancé en `--release`.
- [x] `training/src/families.rs` — l'exclusion du garde-fou.
- [x] `cli/src/main.rs` — le test `run` puis `replay`.
- [x] `fixtures/README.md` — la colonne « Utilisé par ».

**Acceptance Criteria:**
- Given le noyau sans QGIS, when tournent `cargo test`, `cargo fmt --check` et `cargo clippy --all-targets -- -D warnings`, then tout est vert et le contrôle de contrat accepte la commande.
- Given le résultat de la commande, when on le lit, then il porte `file`, `visible_cells`, `hidden_cells` et `nodata_cells`, sans aucune mesure nue.

## Implementation Notes

- **Noyau partagé.** `terrain_line_of_sight.rs` expose `Line` (échantillonnage du segment, `at`) et `sight(dem, line, eye, z_tgt, target_height, curvature, refraction, stop_at_first) -> Sight {obstruction, nodata_samples}`. `line_of_sight` l'appelle avec `stop_at_first = false` ; ses 12 tests passent sans changement. Deviennent `pub(crate)` : `bilinear`, `check_crs`, `check_extent`, `endpoint_elevation`, les défauts et le désérialiseur `refraction` (`Height`, `MetreUnit`, `EARTH_RADIUS` étaient déjà publics).
- **`Distance`** (`analysis_buffer.rs`) gagne `Clone, Copy, PartialEq, Serialize` (schemars a besoin de `Serialize` pour écrire le défaut de 5 km dans le schéma) et `metres()`. Aucun changement de comportement pour `analysis.buffer`.
- **Ordre des contrôles** : système, emprise, altitude de l'observateur (comme 4.4), puis la borne `terrain.viewshed_too_large`. Rayon de 500 cellules pile accepté (5 km sur la colline), 5 000,1 m refusé.
- **Rayon absent** : `radius` est un `Option<Distance>` côté Rust ; le schéma reste celui de `Distance`, sans défaut écrit (5 km recopiés seraient refusés sur un MNT fin), et `null` est refusé (`registry.invalid_params`). Absent, il vaut `min(5 km, borne)` ; donné, il est refusé au-delà de la borne.
- **Cellule de l'observateur** : celle qui le contient (bord droit et bas compris), forcée à 1 même si le rayon est plus petit qu'une demi-cellule.
- **Concordance** : chaque cellule appelle `endpoint_elevation` sur son centre puis `sight`, exactement comme `line_of_sight` vers ce centre ; le test compare 834 cellules (avec courbure) puis 357 (sans courbure, cible à 5 m).
- **Mesure** (`cargo test -p cassini-core --release -- --ignored viewshed --nocapture`, WSL, 2026-11-04) : MNT synthétique 1001 × 1001 de 25 m, rayon 10 km, grille 801 × 801 : **0,31 s** sur un relief ondulé (beaucoup d'arrêts au premier obstacle), **1,35 s** sur un MNT plat (seule la courbure masque, les lignes vont loin). À la borne, 12,5 km sur MNT plat, grille 1001 × 1001 : **1,85 s** ; sans courbure, toutes les cellules visibles (vrai pire cas) : **5,20 s** ; le propriétaire garde 500 cellules et le bloc figé dit désormais « environ 5 s ».
- **Concordance, compléments (boucle 1)** : fenêtre rognée 31 × 31 au centre de la colline (rayon 150 m, comparée par `at`) et MNT troué 7 × 7, toutes les cellules valides comparées.

## Plan Change Log

- **2026-11-04, boucle 1 (intent_gap, constats 1 et 2 de la relecture thorough).**
  - **Décisions du propriétaire**, écrites dans le bloc figé : borne ramenée de 1 000 à 500 cellules ; rayon par défaut ramené à la borne au lieu d'être refusé.
  - **Bloc figé amendé aussi** : la provenance décrit la variante exactement (constat 4) ; le modèle connaît la borne (constat 9) ; trois lignes de matrice ajoutées (défaut ramené, cellule de l'observateur, ordre des refus) ; la mesure porte aussi sur la borne.
  - **État mauvais évité** : une borne annoncée à « quelques secondes » qui en coûte environ 21, et un appel sans rayon refusé sur un MNT fin.
  - **KEEP.** La première version, retirée, est gardée en lecture dans `/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/25884aa7-b3f8-4438-b9d2-84b68a5f4648/scratchpad/keep-4-5/` :
    - `terrain_viewshed.v1.rs` : le nouveau fichier ;
    - `code-v1.patch` : le diff des fichiers existants (`terrain_line_of_sight.rs`, `analysis_buffer.rs`, `families.rs`, `cli/src/main.rs`, `fixtures/README.md`).
  - **À garder de cette version :**
    - le noyau `Line` / `sight` et la refonte de `line_of_sight` (12 tests verts sans changement) ;
    - `Distance` avec `Serialize` et `metres()` ;
    - la fenêtre rognée, `cell_centre`, `index` ;
    - les 9 tests et le test de mesure `#[ignore]` ;
    - le test CLI `run` puis `replay` ;
    - l'exclusion de `families.rs` et la ligne de `fixtures/README.md`.
  - **À ajouter** (constats `patch` 5 à 8) :
    - un test à rayon 1 m pour la cellule de l'observateur ;
    - un cas « EPSG:4326 et 20 km » dans `a_bad_observer_writes_nothing` ;
    - la concordance vérifiée aussi sur une fenêtre rognée (par exemple rayon 150 m au centre de la colline, par l'aide `at`) et sur un MNT troué ;
    - la somme `visible + hidden + nodata == w × h` dans `the_radius_crops_the_output`.

## Review Triage Log

Passe 1 (thorough) : high 0, medium 3, low 9, false 2, maybe-false 0.

| # | Lentille | Constat | Verdict | Route | Preuve |
|---|----------|---------|---------|-------|--------|
| 1 | blind, edge, intent | La borne de 1 000 cellules ne tient pas « sous quelques secondes » | medium | intent_gap | Le coût croît comme r³ ; mesuré 1,35 s à 400 cellules (plat), donc ≈ 21 s à 1 000 cellules, non annulable. L'affirmation est dans le bloc figé : le choix de la borne revient au propriétaire. |
| 2 | blind, intent | Le rayon par défaut de 5 km est refusé sur un MNT de moins de 5 m (LiDAR 1 m : 1 km au plus) | medium | intent_gap | `radius > MAX_RADIUS_CELLS * cell` s'applique au défaut ; le défaut de 5 km est fixé dans le bloc figé. |
| 3 | blind | MNT en degrés : message trompeur « cellules de 0,0002 m » | false | rejet | `Dem::open` refuse déjà un MNT géographique (`dem.rs:428`, « pas dans un système projeté en mètres ») avant `viewshed`. |
| 4 | intent | La provenance annonce « R3 exact d'après la publication », mais l'échantillonnage est celui de 4.4 (pas constant, bilinéaire), pas les intersections avec la grille du papier | medium | patch | En-tête de `terrain_viewshed.rs` ; la provenance est un livrable de l'epic (Done when 2). Correction : décrire la variante exactement. |
| 5 | verif, blind, intent | Cellule de l'observateur forcée à 1 : aucun test ne l'exerce (rayon < distance au centre) | low | patch | Supprimer la branche `own` ne casse aucun test (démontré par la lentille). Ajouter un test à rayon 1 m. |
| 6 | verif | Ordre « observateur puis borne » non fixé par un test | low | patch | Aucun test ne combine un mauvais observateur et un rayon trop grand. Ajouter un cas. |
| 7 | blind | La concordance n'est testée ni sur une fenêtre rognée ni sur un MNT troué | low | patch | `every_cell_agrees_with_the_line_of_sight` ne couvre que la grille entière de la colline sans trou ; l'aide `at` n'est appelée qu'avec un décalage nul. |
| 8 | blind | `the_radius_crops_the_output` ne vérifie pas la somme des comptes | low | patch | Il n'affirme que `nodata_cells > 0`. Ajouter `visible + hidden + nodata == w × h`. |
| 9 | blind | Les descriptions FR et EN ne disent pas la borne au modèle | low | patch | `spec()` ne la mentionne pas ; seul le commentaire FR de `radius` la donne. |
| 10 | edge | Colonne ou ligne entièrement sans donnée en trop quand `(o ± r − x0)/c` tombe juste | low | rejet | C'est réel et fréquent avec des coordonnées rondes (le test fixe une largeur de 14), mais sans effet : une bande sans donnée de plus. La formule est celle des Design Notes. |
| 11 | edge | Cellules non carrées : la cellule de l'observateur, forcée à 1, peut contredire la ligne de vue vers son centre | low | rejet | Seulement si le centre de la cellule est à plusieurs pas, sur des cellules non carrées, rares en entrée. |
| 12 | verif, blind | Cellules rectangulaires non testées (borne et fenêtre) | low | defer | Disposition de la lentille ; à couvrir en 4.7. |
| 13 | blind | Message d'erreur en flottants bruts (point décimal) | low | rejet | Rare et cosmétique ; la correction ajoute un formatage. |
| 14 | blind | `nodata_cells` ne sépare pas les trous et le hors rayon | low | rejet | Le résultat est fixé par les Design Notes ; aucun appelant n'en a besoin aujourd'hui. |
| 15 | blind | Plan incohérent (journal de tri vide, Code Map différente) | false | rejet | La relecture est en cours ; la correction serait d'éditer le plan. |

Passe 2 (thorough, boucle 1) : high 0, medium 2, low 9, false 4, maybe-false 0.

| # | Lentille | Constat | Verdict | Route | Preuve |
|---|----------|---------|---------|-------|--------|
| 16 | intent, verif, edge | Le schéma annonce `radius` par défaut à 5 km, mais 5 km envoyés tels quels sont refusés sur un MNT de moins de 10 m | medium | patch | `#[schemars(with = "Distance", default = "default_radius")]` ; `catalog_entry` garde `default` dans le prompt du modèle. Contredit « sans radius, ramené à la borne ». Correction : pas de défaut dans le schéma, le défaut ramené dit en texte. |
| 17 | blind, edge | La mesure « pire cas » (plat, courbure active) n'est pas le pire : au-delà de l'horizon (≈ 5 km), les lignes s'arrêtent tôt | medium | patch | Le cas où tout est visible (`curvature: false`) parcourt chaque échantillon de chaque ligne. Correction : mesurer ce cas à la borne. |
| 18 | blind | L'en-tête du module dit encore « quelques secondes au pire » | low | patch | C'est la formulation rejetée en boucle 1 ; le bloc figé dit « environ 3 s ». |
| 19 | blind | `cargo fmt --check` échouerait | false | rejet | L'implémenteur a passé `cargo fmt --all --check` ; il sera relancé après les correctifs. |
| 20 | blind | Le plan versionné pointe vers un dossier temporaire (KEEP) | low | rejet | La correction serait d'éditer le plan de ce build. |
| 21 | blind | Le résultat ne rend pas le rayon effectivement utilisé quand le défaut est ramené | low | rejet | Le résultat est fixé par les Design Notes ; la sortie rognée montre l'emprise dans QGIS. À reprendre quand le modèle apprendra la commande. |
| 22 | blind | La cellule forcée de l'observateur pourrait être un trou si les règles de cellule diffèrent | false | rejet | `own` et `endpoint_elevation` prennent la même cellule (floor, bornée, bords droit et bas compris) : un observateur sur un trou est refusé `no_elevation` avant (vu par la lentille intent). |
| 23 | blind | La Code Map contredit les Implementation Notes ; parity et `ai/src/plan.rs` non attestés | low | rejet | La correction serait d'éditer le plan ; la lentille verif confirme que parity exécute les deux exemples sous run, serve et mcp, et que le test du catalogue est générique. |
| 24 | blind | La démo réelle n'est pas faite | low | rejet | C'est une vérification manuelle, faite à la présentation (étape 5), pas un défaut du code. |
| 25 | blind | La seconde boucle de concordance ne compte pas ses cellules | low | rejet | Cosmétique ; la concordance exhaustive sur fenêtre rognée et MNT troué couvre le chemin. |
| 26 | blind | Concordance non testée avec une autre réfraction ni un observateur quelconque dans une cellule | low | rejet | Le code est partagé avec la ligne de vue ; un défaut improbable. |
| 27 | blind | `Sight.nodata_samples` change de sens avec `stop_at_first` | low | rejet | Le compte est ignoré dans `viewshed` ; aucun appelant n'est trompé aujourd'hui. |
| 28 | blind | `NO_VIEWSHED` peut-être absent ou vide | false | rejet | Défini à `training/src/families.rs:2352`, non vide. |
| 29 | edge | `viewshed()` publique avec un rayon négatif ou NaN : débordement u32 | low | rejet | Hors d'atteinte par le registre (serde refuse) ; la panique est bruyante. |
| 30 | intent, edge | Cellule de l'observateur sur cellules non carrées | low | rejet | carried — constat 11, même lieu, même affirmation. |

## Design Notes

**Pourquoi R3.** Le ticket exige la concordance cellule par cellule avec la ligne de vue. Partager son code la garantit par construction ; un algorithme approché (R2, balayage) s'en écarterait sur les bords des crêtes. Coût : environ π·r² cellules × r échantillons, avec arrêt au premier obstacle. À 10 km sur 25 m, cela fait 400 cellules de rayon, soit environ 500 000 lignes, de l'ordre de la seconde en `--release`.

**Rognage.** La fenêtre va des colonnes `floor((ox − r − x0)/cx)` à `floor((ox + r − x0)/cx)`, de même pour les lignes, bornées à la grille. L'origine de la sortie est le coin de la première cellule : la grille reste alignée sur celle du MNT dans QGIS.

**Résultat.** `{file, visible_cells, hidden_cells, nodata_cells}` : des comptes de cellules, pas de mesure.

## Verification

**Commands:**
- `cargo test -p cassini-core -p training`, `cargo fmt --check`, `cargo clippy -p cassini-core -p training --all-targets -- -D warnings` sous WSL ; l'espace de travail entier sous Windows (WSL n'a pas libclang) — attendu : vert.
- `cargo test -p cassini-core --release -- --ignored viewshed` — le temps à 10 km, consigné dans les Implementation Notes.

**Manual checks:**
- Démo réelle : `cassini.exe run terrain.viewshed` sur le MNT Lambert-93 de Villeneuve-le-Roi, depuis le coteau, rayon 3 km. On ouvre la sortie dans QGIS : le champ de vision doit être plausible au vu du relief.
