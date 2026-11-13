---
title: "5.54 lot 2 — familles du jeu de la première séance continue"
type: 'feature'
ticket: ''
created: '2026-11-11'
status: 'built'
baseline_revision: '0043f36b41798e0febe2d31ca38f2d91630c493f'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La première séance continue (5.54, lot 1) attend son contenu : cinq familles sont refusées au jeu ciblé (`FOCUS_EXCLUDED`) car, tirées seules à la graine 3, elles engendrent une phrase de barre à un nombre ou une commune près, et les leçons de la remesure (« Seuils au niveau du modèle installé », `docs/entrainement-du-modele.md`) ne sont enseignées par aucune famille.

**Approach:** Réparer les cinq familles et vider `FOCUS_EXCLUDED` ; ajouter ou renforcer les familles des leçons (format gardé avec l'échelle, refus des fonctions absentes, options écrites, champ de vision cumulé, point Lambert-93 vers `geo.*`/`coord.convert`, N/E inversés) ; nommer la liste des familles de la première séance ; préparer hors dépôt le script de recette dont `nuit-5-54.sh` a besoin.

## Boundaries & Constraints

**Always:** vocabulaire civil (dépôt public) ; une option n'est au plan que si la phrase l'écrit, et toute option écrite y est ; une conversion est une étape visible (`data.transform_point` vers 4326, puis `step:k.point`) ; tout refus nomme une fonction absente du registre, jamais une commande voisine en substitut ; chaque famille neuve a son test ; tous les garde-fous actuels restent verts.

**Never:** lire les phrases de barre ou du jeu tenu à l'écart pour s'en inspirer (seule la garde les compare ; en cas de collision, on regarde la phrase *engendrée* et on retire sa tournure) ; toucher aux barres (`ai/tests/`, `qgis-plugin/tests/recette/`), au noyau ou au registre ; lancer un modèle ou le GPU ; lancer la séance ; pousser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Famille réparée seule | `generate_families(FOCUS_SEED, FOCUS_COUNT, ["pente"])` | exemples écrits, `barres::copies` vide | — |
| Liste de séance | `FIRST_SESSION_FAMILIES` à graine 3 | exactement `FOCUS_COUNT` exemples, aucune copie | test rouge sinon |
| PDF échelle + format | « …au 1:25 000 sur un A3 » | un `layout.export_pdf` avec `scale` et `page_size` | jamais un refus |
| Fonction absente | courbes de niveau, plus court chemin… | `refusal`, aucun appel | — |
| Point Lambert-93 vers `geo.*` | point EPSG:2154 | `data.transform_point` (→4326) puis `step:k.point` | — |
| N/E inversés | « N 6 862 000, E 652 000 (Lambert-93) » | `x` = 652000, `y` = 6862000 | axes non étiquetés jamais inversés |

</frozen-after-approval>

## Code Map

- `training/src/lib.rs:286-301` -- `FOCUS_SEED`, `FOCUS_COUNT`, `FOCUS_EXCLUDED` (forme lue par `pipeline.py:141` : garder `pub const FOCUS_EXCLUDED: &[&str] = &[…];`, vide = `&[];`). Ajouter `FIRST_SESSION_FAMILIES` à côté. Tests à garder : `the_default_set_holds_a_quarter_of_big_projects…` (l.734, 23-27 %), redemandes ≥ 4 % (l.1227), `a_focused_set…`, `the_recall_set_is_a_prefix…`, `the_pipeline_reads…`, `no_family_produces_rejected_examples`, `no_plan_of_the_generated_set_passes_six_steps`.
- `training/src/families.rs:53-460` -- table `FAMILIES` (ordre fixe, poids total 472 ; `redemande` 58, `gros_projet` 128). Réutiliser : `pdf_at` (484), `PAGE_SIZES` (2275), `page_words` (2450), `scale_text`, `export_pdf_scale` ; `refus_hors_registre` (3813) et ses constantes `NO_*` (3788-3803), test `what_is_refused_out_of_registry_is_still_missing_from_the_registry` (10517) ; `style_degrade` (2868, `GRADUATED_METHODS/RAMPS`) ; `champ_de_vision`, `champ_de_vision_cumule` (4757) ; `Coord`, `wgs84_coord`, `projected_coord` (5654), `utm_coord`, `lambert_coord` (5703) ; `conversion_coordonnees` (5944, seul modèle actuel Lambert-93 → `transform_point` → `coord.convert`, test 8840) ; `distance_azimut` (6954), `point_projete` (7037), `recoupement` (7313).
- `training/tests/recouvrement.rs` -- garde complète et par famille (`no_family_drawn_alone…`, liste exacte dans les deux sens) : ne pas modifier.
- `training/src/barres.rs:215` `copies` -- n'imprime que l'identifiant.
- Noyau (lecture seule) : `geo.*` et `coord.convert` n'acceptent que 4326/326NN/327NN (`geo.unsupported_crs`) ; `core/src/plan_completion.rs:46` `complete_conversions` n'insère de conversion que devant une commande à raster, jamais devant `geo.*`/`coord.convert` : le plan appris doit porter l'étape. Paramètres : `geo.distance_azimuth{from,to}`, `geo.destination_point{from,azimuth,distance}`, `geo.bearing_intersection{first,first_azimuth,second,second_azimuth}`, résultat `point` de `data.transform_point{point,target_epsg}`. Défauts (catalogue `spec-cassini-mvp/catalogue-des-commandes.md`) : viewshed `observer_height` 1,6, `target_height` 0, `radius` 5 km, `curvature` true, `refraction` 0,13 ; `graduated_style` `jenks`, 5 classes, `yellow_red` ; PDF `a4`, `landscape`.
- Fonctions sans commande (catalogue, statuts `prévue`/`réservée`) : `raster.contour`, `geocode.ban`, `network.shortest_path`, export image, temps de trajet, `terrain.flood_extent`, `terrain.flow_accumulation`, `terrain.watershed`, déblai/remblai, lissage, rugosité, style par catégories, étiquettes, opacité, symboles proportionnels.
- Hors dépôt : `~/cassini-entrainement/nuit-5-54.sh` (`RECETTE_PS1` l.33, garde `preparatifs` l.76, `WT` l.21 par défaut sur le worktree du lot 1) ; modèle d'appel : `ESSAIS_PS1` dans `outils-cassini\scripts\essais-cuda.ps1` ; sonde `qgis-plugin/tests/probe_recette.py <projet> <phrases>` (code 2 si elle ne part pas).

## Tasks & Acceptance

**Execution:**
- [x] `training/src/families.rs` -- réparer `filtre_numerique`, `pente`, `reprojection_couche`, `retrait_couche`, `carroyage_mgrs` : trouver, par un test local jetable non commité, les phrases *engendrées* de la famille (graine 3, `FOCUS_COUNT`) dont la clé `masked` collisionne, et retirer ou reformuler leur tournure -- la garde par famille doit passer.
- [x] `training/src/families.rs` -- familles neuves : `export_pdf_echelle_format` (échelle + format et/ou orientation, titre facultatif, seul ou au bout d'une recherche), `refus_fonction_absente` (une branche et une constante `NO_*` par fonction de la liste, tournures variées), `options_ecrites` (dégradé à deux options écrites ou plus ; champ de vision « sans tenir compte de la courbure » → `curvature: false`, hauteurs, rayon, réfraction écrits), `point_lambert_vers_geo` (les trois `geo.*` et `coord.convert` sur un ou deux points Lambert-93), `ordre_nord_est` (WGS84 « lon E, lat N », projeté « N = …, E = … », toujours étiqueté) ; renforcer `champ_de_vision_cumule` (tournures de décompte, 2 à 6 points, poids 4 → 6) -- leçons de la remesure.
- [x] `training/src/families.rs` -- poids : compenser l'ajout pour garder `gros_projet` dans 24-26 % et les redemandes ≥ 4,5 % du jeu graine 1 (relever `gros_projet` et `redemande`) -- marge au-dessus des seuils des tests.
- [x] `training/src/families.rs` (tests) -- un test par famille neuve : refus jamais un plan et mot-clé absent des identifiants du registre (même forme que le test l.10517) ; option au plan ssi écrite ; points de `geo.*`/`coord.convert` en 4326/326NN ou `step:k.point` d'un `transform_point` 2154 → 4326 ; `x`/`y` justes quel que soit l'ordre écrit ; PDF portant `scale` et `page_size`/`orientation`, jamais un refus ; décompte → `terrain.viewshed_cumulative`.
- [x] `training/src/lib.rs` -- `FOCUS_EXCLUDED` vidé (commentaire mis à jour) ; `pub const FIRST_SESSION_FAMILIES: &[&str]` = les six familles neuves ou renforcées ; test : liste acceptée par `generate_families`, `FOCUS_COUNT` exemples rendus, `barres::copies` vide.
- [x] `docs/entrainement-du-modele.md` -- section « Familles de la première séance continue (5.54, lot 2) » : familles, liste, comptes avant/après du jeu graine 1, commande de la séance.
- [x] `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- clore les entrées « lot 2 », `FOCUS_EXCLUDED` et `RECETTE_PS1` (`status: close`, date, ce plan).
- [x] Hors dépôt : `outils-cassini\scripts\recette-5-54.ps1` (UTF-8 avec BOM) -- `param($Models,$Project,$Phrases,$Out)` ; `CASSINI_CORE=%LOCALAPPDATA%\Cassini\bin\cassini.exe`, `CASSINI_MODELS=$Models`, `QT_QPA_PLATFORM=windows` ; depuis `CASSINI-nuit`, `python-qgis-ltr.bat` (QGIS 3.44.15) sur `probe_recette.py`, sortie et `CODE <code>` dans `$Out` ; puis `nuit-5-54.sh` : `RECETTE_PS1` par défaut sur ce chemin.

**Acceptance Criteria:**
- Given le jeu graine 1, when on le régénère, then 8 600 exemples, aucun écarté hors `gros_projet`, et la doc donne les comptes avant/après.
- Given `FOCUS_EXCLUDED` vide, when `cargo test -p cassini-training`, then tout est vert, `recouvrement.rs` compris.
- Given le script de recette posé, when la garde `preparatifs` de `nuit-5-54.sh` tourne, then elle ne dit plus « script de recette absent ».

## Implementation Notes

- Réparations sans tirage de plus (réécriture déterministe de la tournure engendrée) : le reste du jeu ne bouge pas pour elles. Au jeu complet, le tirage déplacé par les familles neuves a fait sortir CC1 (tournure « Convertis le point … en coordonnées UTM » de `conversion_coordonnees`, emballée par `gros_projet`) : reprise en « Convertis-moi », même méthode.
- `point_lambert_vers_geo` : jamais un point en Lambert-93 à côté d'un point en WGS84. Le garde-fou (`core/src/point.rs`, lecture seule) retire du pool de la demande les nombres entiers du plan, dont `target_epsg: 4326`, qui prend alors le code de « WGS84 » écrit pour le second point : le plan deviendrait la question des coordonnées. Le partenaire mixte est un point UTM.
- Hors du code map, rendus nécessaires par `FOCUS_EXCLUDED` vide : `training/tests/commande.rs` et `training/entrainement/tests/test_ajustement_continu.py` indexaient `FOCUS_EXCLUDED[0]` / attendaient `retrait_couche` (contrôle rendu conditionnel, refus essayé sur une liste posée par `mock.patch`) ; message de `pipeline.py` sans « à reprendre au lot 2 ». Le test existant du cumulé (`…_two_to_four_…`) devient `…_two_to_six_…`.
- Poids : `gros_projet` 128 → 148, `redemande` 58 → 76, `carroyage_utm_projet` 4 → 6 (graine 1 : 25,80 %, 5,23 %).
- Le recoupement en Lambert-93 vise juste grâce à `lambert93_to_lat_lon` (inverse de la projection, constantes IGN), écrit dans le générateur.

## Plan Change Log

## Review Triage Log

## Design Notes

Décisions tranchées (party : ingénieur ML, géomaticien, architecte) :
- **Familles neuves plutôt que retouches des existantes** : une famille nommée se cible seule à la séance et se garde seule ; les existantes restent telles quelles, sauf les cinq réparées et le cumulé.
- **Liste de séance = les six familles neuves ou renforcées**, sans les cinq réparées : le jeu ciblé vise les fautes mesurées ; le rappel (40 %) tient le reste. Constante en Rust, source unique, recopiée dans la commande ; pas de lecture par le script.
- **`FOCUS_EXCLUDED` gardé, vide** : la garde par famille reste le contrôle des familles futures.
- **N/E inversés seulement étiquetés** : sans étiquette, l'ordre inversé est ambigu, donc jamais enseigné.
- **Étiquettes refusées sur une couche seulement** : « le nombre par commune avec son nom » reste un plan (`label_field` du décompte). Catégories : refus d'une couleur par valeur de champ texte, la mise aux couleurs topographiques restant un plan.
- **Pas de découpage** : la séance a besoin de tout ; la construction se fait en deux commits (réparations + script, puis familles neuves).
- Avant (graine 1, `0043f36`) : 8 600 exemples, 349 redemandes (4,06 %), `gros_projet` 2 106 (24,5 %), `transform_point` → `geo.*` 29 (seul `distance_azimuth`), `curvature` 0, PDF échelle + format 0, cumulé 100.

## Verification

**Commands:**
- `cargo test -p cassini-training --release` -- expected: tout vert.
- `cargo fmt --check && cargo clippy -p cassini-training --all-targets -- -D warnings` -- expected: aucun avertissement.
- `cargo run -p cassini-training --release -- --familles <FIRST_SESSION_FAMILIES> --seed 3 --count 1800 --out <scratch>` -- expected: code 0, 1 800 exemples.
- `powershell.exe -NoProfile -ExecutionPolicy Bypass -File '…\outils-cassini\scripts\recette-5-54.ps1' -Models x -Project x -Phrases x -Out <TEMP>\essai.txt` -- expected: « SONDE NON PARTIE » et `CODE 2` (aucun modèle chargé).

**Commande de la séance** (après fusion en avance rapide sur `main`, `REV` = HEAD du worktree, propre ; accord du propriétaire, GPU libre) :
`git -C ".../CASSINI-nuit" checkout -q --detach REV && WT=~/cassini-worktrees/5-54-lot-2 bash ~/cassini-entrainement/nuit-5-54.sh export_pdf_echelle_format,refus_fonction_absente,options_ecrites,champ_de_vision_cumule,point_lambert_vers_geo,ordre_nord_est`

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 0, medium 9, low 12, false 0, maybe-false 0. Corrigés 19 (patch, même implémenteur), reportés 2.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Refus sans contre-exemples voisins : risque d'apprendre à refuser des plans valides | medium | patch | presque-refus une fois sur trois (reclassement d'altitude, isochrone, pente/exposition, styles) ; mots des commandes existantes interdits dans les refus |
| Refus du fichier de clients (CSV supporté), phrase fixe | medium | patch | remplacée par une adresse postale écrite |
| Français : « de le collège », « terrain du terrain », « Quelles sont la référence », options collées | medium | patch | `de_np`/`a_np`, formes « à partir de », accords, virgules |
| `ordre_nord_est` : observateur hors du MNT ; WGS84 jamais latitude d'abord | medium | patch | `terrain_point` ; latitude d'abord une fois sur quatre |
| Échelle et cible non vérifiées par les tests | medium | patch | échelle dans `SCALES` et écrite ; `target_epsg`/`to` vérifiés |
| Tournure de CC1 recréée (`point_lambert_vers_geo`, `ordre_nord_est`) | medium | patch | « Convertis-moi » |
| Liste de séance recopiée à la main dans la doc | medium | patch | test qui compare la doc à `FIRST_SESSION_FAMILIES` |
| Script de recette : code nul pris pour succès, paramètres facultatifs | medium | patch | `Test-Path`, `-ErrorAction Stop`, `Mandatory`, code 2 |
| `nuit-5-54.sh` : recette sans SCORE rend la barre de non-oubli illisible | medium | patch | relevé sans SCORE non donné au juge, noté dans l'état |
| `geo_point_is_readable` : panique, référence trop permissive | low | patch | `checked_sub`/`get`, cible WGS84 exigée |
| Branche de refus de `check_focus` plus exécutée | low | patch | `check_focus_against` testé sur une liste fournie |
| Courbure dite seule jamais enseignée | low | patch | ajoutée |
| Deux points égaux en temps de trajet | low | patch | écarté |
| Raison du poids de `redemande` fausse | low | patch | corrigée |
| Chemins Windows personnels dans le plan | low | patch | forme relative |
| Tournures maladroites, mot-clé mort | low | patch | corrigés |
| Garde-fou : point Lambert-93 + point WGS84 dans une même phrase → question | low | defer | limite du garde-fou du noyau (lecture seule ici) |
| Réparations par une tournure ; proximité non mesurée | low | defer | la garde reste exacte à un nombre ou une commune près ; une mesure de distance serait un ticket à part |

Poids réglés : `gros_projet` 148, `redemande` 76, `carroyage_utm_projet` 4 → 6 (deux tests du jeu complet). Comptes du jeu graine 1 dans la doc.
