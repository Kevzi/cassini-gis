---
title: 'Formulations de terrain en listes de synonymes du générateur (livrable 4 de 5.53)'
type: 'feature'
ticket: '' # livrable de 5.53, plan rattaché à story-barre-de-mesure-par-commande-phrases-ecrites-a-la-main-plan.md
created: '2026-11-11'
status: 'built'
baseline_revision: 'e912a11e58fd112fd3e5c42a522855fb4a326391'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** les familles de géométrie et de terrain écrivent leurs tournures en phrases entières
dans des `format!` ; le vocabulaire courant de la topographie et de l'orientation civiles (gisement,
relèvement, recoupement, report d'un point, contre-azimut, déclivité, bassin visuel, estompage…)
y est rare ou absent, et rien ne dit quelle tournure appelle quel paramètre (`north` surtout).

**Approach:** chaque notion devient une liste de synonymes nommée (forme de `TOPO_WORDS` : la
tournure sans le verbe), assemblée avec des listes de verbes ; chaque liste est liée à une commande
et à ses paramètres, et les tournures ambiguës sont rassemblées dans une liste interdite qu'un test
vérifie. Les formulations sont recherchées par Claude (usage civil : IGN, carte au 1:25 000,
course d'orientation, randonnée, géomètres, enseignement) ; le propriétaire ne les dicte pas.

## Boundaries & Constraints

**Always:** vocabulaire civil seul (§2.9) ; synonymes, jamais phrases recopiées d'une barre ;
chaque tournure de nord porte le `north` de sa liste ; poids des familles inchangés ; tests
existants de vocabulaire gardés (au besoin récrits pour lire les listes) ; le changement n'agit
qu'au prochain entraînement.

**Never:** GPU, chargement d'un modèle ; toucher `~/cassini-worktrees/reentrainement-5-49` ou
`CASSINI-nuit` ; modifier une barre (`ai/tests/**`, recette d'Annecy) ou le noyau ; ajouter une
commande ou un paramètre ; « cap » seul ; pousser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Gisement | demande avec un mot de `GRID_NORTH_WORDS` | plan avec `north: "grid"` | — |
| Nord magnétique | mot de `MAGNETIC_NORTH_WORDS` | `north: "magnetic"`, `date` seulement si écrite | — |
| Nord vrai | mot de `TRUE_NORTH_WORDS` | `north` absent (D1) | — |
| Mélange | une demande engendrée porte deux nords | test rouge, phrase imprimée | — |
| Tournure ambiguë | mot de `AMBIGUOUS_TERMS` dans le jeu engendré | test rouge | — |
| Barre recopiée | assemblage égal à une phrase de barre | `recouvrement` rouge | retirer la tournure, jamais toucher la barre |

## Décisions (propriétaire, 2026-11-11)

- D1 : nord géographique écrit → `north` absent (un seul plan par sens).
- D2 : « gisement Lambert » / « nord Lambert » évités (`AMBIGUOUS_TERMS`) ; gisement Lambert-93 au noyau en `deferred-work`.
- D3 : déclinaison ou convergence en un seul point hors livrable ; `deferred-work`.
- D4 : recoupement ou relèvement « à la boussole » évité ; `north` sur `geo.bearing_intersection` en `deferred-work`.
- D5 : relèvement inverse (« où suis-je ? » depuis des visées vers des repères) évité ; `deferred-work`.
- D6 : le propriétaire relit la table des tournures avant le commit.
- D7 : plan gardé entier malgré sa taille (≈ 2 600 jetons).

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- seul fichier de code touché.
  - `TOPO_WORDS` (l. 2753) : modèle de liste à suivre.
  - `distance_azimut` (l. 6443), `point_projete` (6532), `gisement` (6643, `grid_zone_words` 6576), `azimut_magnetique` (6850, `magnetic_date_words` 6811), `recoupement` (6961) : phrases entières à défaire en verbes × synonymes. Réutiliser `named_or_plain`, `grid_point`, `azimuth_words`, `angle_words`, `geo_distance`, `mgrs_result`, `de_np`, `a_np`.
  - `convert_words` (5751) et `reprojection_point` (5922, `TO_LAMBERT`) : déjà en listes, à compléter.
  - Terrain, phrases inline : `pente` 4367, `ligne_de_vue` 4398, `champ_de_vision` 4461, `exposition` 4535, `ombrage` 4587, `champ_de_vision_cumule` 4653, `points_hauts` 4733. Carroyage : `carroyage_utm_projet` 2630 (`GRID_SYSTEM_QUESTION`), `carroyage_mgrs` 7487 (`MGRS_STEPS`).
- Tests du même fichier : à garder verts.
  - `a_grid_bearing_says_grid_…` (8675) : exige gisement / quadrillage / nord utm ; interdit carte, pdf, carroyage, grille, « tous les ».
  - `every_grid_bearing_…` (8652) : jamais « lambert ».
  - `a_magnetic_azimuth_…` (8739) : exige magnétique ou boussole ; interdit gisement et quadrillage.
  - `a_distance_and_azimuth_…` (8584) : « grades » ⇔ gon, « degrés » ⇔ deg.
  - `a_slope_…` (10546) : unité ⇔ mots.
  - `an_aspect_…` (10703) : exposition et versants vus.
  - `the_orientation_of_the_slopes_is_an_aspect` (11499).
  - `a_radius_written_before_the_point_…` (9234), marqueurs `EYE_MARKERS` et `BOTH_MARKERS` (10543, 10596).
- `training/tests/recouvrement.rs` -- garde-fou ; ne pas modifier.
- `ai/tests/par_commande/phrases.json` -- barre ; lire seulement. À ne pas reproduire, même à un nombre près : « Gisement et distance du point … vers le point … », « Reporte un point à …, azimut magnétique … », « Intersection des relèvements … », « … et son orientation ? ».
- Catalogue :
  - `geo.distance_azimuth` rend `back_azimuth`, `convergence` (grid), `declination` (magnetic).
  - `geo.bearing_intersection` : nord vrai seul, sans `north`.
  - `coord.convert` : décimal seul.
  - `layout.export_pdf` `grid` : `degrees`, `projected` (epsg), `mgrs`.

## Tournures retenues

| Liste (nom indicatif) | Commande · paramètre | Tournures |
|---|---|---|
| `TRUE_NORTH_WORDS` | `geo.distance_azimuth` / `geo.destination_point`, `north` absent | l'azimut ; l'azimut géographique ; l'azimut vrai ; l'azimut par rapport au nord géographique ; la direction ; l'azimut et le contre-azimut (rendu par `back_azimuth`) |
| `DISTANCE_WORDS` | `geo.distance_azimuth` | la distance à vol d'oiseau ; la distance géodésique ; la distance en ligne droite ; combien de kilomètres séparent |
| `GRID_NORTH_WORDS` | idem, `north: "grid"` (+ `utm_zone` si écrite) | le gisement ; l'azimut du quadrillage ; l'azimut en nord du quadrillage ; l'azimut par rapport au nord du quadrillage ; l'azimut en nord UTM ; l'angle compté depuis le nord du quadrillage UTM |
| `MAGNETIC_NORTH_WORDS` | idem, `north: "magnetic"` (+ `date`) | l'azimut magnétique ; l'azimut à la boussole ; l'azimut boussole ; l'azimut lu à la boussole ; l'azimut par rapport au nord magnétique ; le cap magnétique ; en tenant compte de la déclinaison magnétique |
| `DESTINATION_VERBS` | `geo.destination_point` | reporte un point ; implante un point ; place un point ; positionne un point ; trace le point ; quel point atteint-on ; où arrive-t-on ; je pars de … et je marche |
| `INTERSECTION_WORDS` | `geo.bearing_intersection` | le recoupement ; l'intersection des deux visées ; le croisement des deux azimuts ; le point où se coupent les deux directions ; l'intersection des relèvements pris depuis |
| `convert_words` (complété) | `coord.convert` `to` | wgs84 : coordonnées géographiques, latitude et longitude, en degrés décimaux, coordonnées GPS (WGS84) ; utm : coordonnées UTM, fuseau / zone N ; mgrs : référence MGRS, coordonnées MGRS, référence USNG |
| `TO_LAMBERT` & co | `data.transform_point` `target_epsg` | Lambert-93, Lambert 93, RGF93 / Lambert-93 → 2154 ; Lambert II étendu → 27572 ; Web Mercator → 3857 |
| grilles du PDF | `layout.export_pdf` `grid` | mgrs : carroyage, grille, quadrillage MGRS, USNG ; projected 326NN : carroyage / quadrillage kilométrique UTM ; projected 2154 : carroyage Lambert-93 ; degrees (famille l. 2391) : graticule, méridiens et parallèles |
| terrain | `terrain.slope` | pente ; inclinaison ; déclivité |
| | `terrain.aspect` | exposition ; orientation des versants ; versants à l'adret et à l'ubac |
| | `terrain.hillshade` | ombrage ; estompage ; relief ombré |
| | `terrain.peaks` | sommets ; points hauts ; points culminants ; points dominants |
| | `terrain.viewshed` | champ de vision ; zones visibles ; bassin visuel ; panorama depuis |
| | `terrain.line_of_sight` | ligne de vue ; intervisibilité entre ; vue directe |
| | `terrain.viewshed_cumulative` | visibilité cumulée ; bassin visuel cumulé |

`AMBIGUOUS_TERMS`, jamais engendrés :
- « cap » sans « magnétique » (souvent magnétique, décision du 2026-11-11) ;
- « gisement Lambert » et « nord Lambert » (le gisement du noyau est UTM, D2) ;
- « coordonnées kilométriques » sans système nommé ;
- « degrés minutes secondes » en sortie (non rendu) ;
- « relèvement » ou « recoupement » « à la boussole » (D4) ;
- « où suis-je » avec des visées vers des repères (D5) ;
- « crêtes » et « points cotés » pour `terrain.peaks` (ni des sommets, ni des maximums) ;
- « triangulation ».

## Tasks & Acceptance

**Execution:**
- [x] `training/src/families.rs` -- listes de la table ci-dessus (doc en français, commande et paramètre nommés), familles géométriques récrites en verbes × synonymes, listes de terrain et de conversion complétées, `AMBIGUOUS_TERMS` -- une notion, une liste, un paramètre.
- [x] `training/src/families.rs` (tests) -- chaque tournure de chaque liste vue au moins une fois sur `geometry_examples` / `valid(make, n)` ; une demande porte les mots d'un seul nord, et son plan le `north` de cette liste ; aucune demande du jeu engendré (`generate(DEFAULT_SEED, DEFAULT_COUNT)`) ne contient un terme de `AMBIGUOUS_TERMS` ; tests existants adaptés pour lire les listes, sans relâcher leurs interdits.
- [x] `docs/entrainement-du-modele.md` -- court paragraphe « Formulations de terrain » : où sont les listes, règle « une tournure, un paramètre », liste des ambiguës.

**Acceptance Criteria:**
- Given le générateur modifié, when `cargo test -p cassini-training` tourne, then tout est vert, `recouvrement` compris.
- Given la graine par défaut, when le jeu est engendré, then chaque liste de la table y apparaît et aucune demande ne mêle deux nords.

## Implementation Notes

- Deux listes par nord : `*_NORTH_WORDS` (nom, `geo.distance_azimuth`) et `*_NORTH_HEADINGS`
  (cap avec `{az}`, `geo.destination_point`) ; « en tenant compte de la déclinaison magnétique »
  suit les points, après « l'azimut ». Verbes : `MEASURE_VERBS`, `DESTINATION_VERBS`.
- Terrain : `SLOPE_WORDS`, `ASPECT_WORDS`, `HILLSHADE_WORDS`, `PEAK_WORDS`, `VIEWSHED_WORDS`,
  `LINE_OF_SIGHT_WORDS`, `CUMULATIVE_VIEWSHED_WORDS`, assemblées par `DEM_PRODUCT_VERBS` ; la
  pente n'emploie jamais « Calcule » (SL2 et une phrase d'essai réel). « Adret et ubac » écrit
  « la répartition des versants à l'adret et à l'ubac ».
- `AMBIGUOUS_TERMS` : (fragments tous présents, fragments qui lèvent, raison), lu par
  `ambiguous_term` sur `ambiguity_text` (mots entiers) ; publics pour éviter le code mort.
- Le mélange de nords se vérifie sur les demandes dont le plan appelle une commande `geo.*` : un
  PDF « quadrillage géographique » parle de grille, pas de nord.
- Tests ajoutés : `every_wording_of_every_list_is_generated`,
  `a_request_speaks_of_one_north_and_its_plan_carries_it`, `the_ambiguity_check_reads_whole_words`,
  `the_default_set_has_every_wording_list_one_north_per_request_and_no_ambiguous_term`. Tests
  existants inchangés, tous verts.

- Après relecture : `AMBIGUOUS_TERMS` devient une liste de `Ambiguous` (`all`, `any`, `except`
  retiré occurrence par occurrence, `lifted_by`) ; « où suis-je » interdit seulement avec visée,
  repère ou relèvement, permis au bout d'un report de point ; `DISTANCE_QUESTIONS` à part ;
  tournures d'avant remises en branches ; tests de tirage bornés et par branche ; contrôle des
  tournures ambiguës sur les graines 1 à 5. 349 redemandes à la graine 1 (4,06 %) après un
  réglage de tirage dans `gisement` (1 sur 6 pour « Quel gisement faut-il prendre »), sans
  changer de poids.

## Plan Change Log

## Review Triage Log

## Design Notes

Assemblage type, sans phrase recopiée :

```rust
/// Le gisement, sans le verbe : `north: "grid"` sur les deux commandes.
const GRID_NORTH_WORDS: &[&str] = &["le gisement", "l'azimut du quadrillage", /* … */];
let body = format!("{} {} {} {}", verb, pick_str(rng, GRID_NORTH_WORDS), de_np(pa), a_np(pc));
```

Accords (le/l'/la, de/du/d') : garder `de_np` et `a_np`. Une liste de « la tournure sans le verbe »
fait varier la phrase sans multiplier les `match`.

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: vert (familles, `recouvrement`, `commande`, `forme_inventaire`).
- `cargo fmt --check && cargo clippy -p cassini-training --all-targets -- -D warnings` -- expected: aucun avertissement.

**Manual checks (if no CLI):**
- Lire 30 demandes engendrées de `gisement`, `azimut_magnetique`, `recoupement` : français naturel, bons accords.

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 0, medium 8, low 10, false 1, maybe-false 0. Corrigés 12 (patch, même implémenteur), rejetés 3.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| « où suis je » interdit partout, alors que DP1 de la barre l'attend en report de point | medium | patch | interdit seulement avec visée/repère/relèvement ; report « je pars … Où suis-je » ajouté |
| Accords faux (« Quels sont la distance et la direction », « les pentes … exprimée ») | medium | patch | accords corrigés |
| Tournures mal assemblées (double « du », estompage, ubac, « séparent A et B ») ; « le panorama » | medium | patch | corrigées ; « le panorama » retiré |
| Fragments ambigus trop étroits ; exception « cap magnétique » globale | medium | patch | structure `Ambiguous`, exception par occurrence, chaque entrée épinglée par un test |
| Tournures courantes retirées par la réécriture | medium | patch | remises (listes ou branches) |
| Test de couverture satisfait par une sous-chaîne voisine | medium | patch | tournure reconnue telle que tirée, par branche |
| Présence des listes vérifiée sur tout le jeu ; une seule graine | medium | patch | par commande ; graines 1 à 5 |
| `norths_written` : « géographique » nu et formes nues | medium | patch | marques précises ; formes nues = nord vrai |
| `DISTANCE_WORDS[len-1]` suppose l'ordre | low | patch | `DISTANCE_QUESTIONS` à part |
| Ajouts de conversion, RGF93, grilles du PDF hors du test | low | patch | couverts |
| Commentaire citant un test inexistant | low | patch | corrigé |
| Doc : prose dans une colonne de comptes, « trois tests », exemples périmés | low | patch | corrigée |
| « La barre par commande n'est pas protégée » | false | reject | relu sur le dépôt principal (autre branche) ; dans le worktree `recouvrement` lit `par_commande/phrases.json` |
| Quasi-copies d'une phrase de barre à un mot près | low | reject | `recouvrement` vert à l'identique et à un nombre près ; la règle ne va pas au-delà |
| Listes de verbes encore en ligne pour certaines familles | low | reject | l'intention vise les notions ; aucun lien tournure → paramètre en jeu |
| Liaison liste → `north` par placement, pas par construction | low | reject | le test « un seul nord par demande, porté par le plan » le garde |
| Validation du propriétaire (D6) | — | propriétaire | table approuvée le 2026-11-11 (« ok pour les deux »), doutes futurs en bmad party |

Taux de redemandes au bas de sa fourchette (4,06 %, seuil 4 %) après réglage du tirage du gisement : un prochain changement du générateur peut le faire passer sous le seuil.
