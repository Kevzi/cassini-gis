---
title: 'terrain.line_of_sight : ligne de vue entre deux points'
type: 'feature'
ticket: '4'
created: '2026-11-04'
status: 'built'
baseline_revision: '7fa6de3bbf9681ce9c206eb48aaefd2f13b4c223'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 1
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le noyau sait calculer une pente (4.2) mais pas dire si un point en voit un autre sur le relief. C'est la deuxième des trois commandes de terrain du MVP (CAP-11), en Rust et sans GPL, base du module privé.

**Approach:** Ajouter au registre `terrain.line_of_sight`, exécutée dans le noyau. Elle prend un MNT (`dem`, chemin relatif au projet), un observateur et une cible (`Point`, dans le système du MNT), leurs hauteurs au-dessus du sol et une option de courbure terrestre avec réfraction. Elle rend le verdict visible oui/non, le premier obstacle (`Point`) et un profil d'altitudes échantillonné, borné (AD-8). Aucun fichier n'est écrit.

## Boundaries & Constraints

**Always:**
- **Lecture du MNT.** Elle passe par `dem.rs` seul (AD-20).
- **Système des points.** Les points sont dans le système du MNT : un `epsg` différent de celui du MNT est refusé avec une erreur typée, jamais reprojeté (AD-19).
- **Paramètres.**
  - `observer_height` et `target_height` : mesures à unité explicite `{value, unit}` (`m` ou `km`, comme la distance de `analysis.buffer`), au moins 0, par défaut 1,6 m et 0 m.
- **Unités (décision du propriétaire, 2026-11-04).** Aucune mesure nue ne traverse le canal (contrat d'architecture, ligne « Mesures »). Le résultat porte `unit: "m"`, qui s'applique à toutes ses distances et altitudes, comme `terrain.slope` porte `unit` à côté de `min` et `max`.
  - `curvature` : booléen, vrai par défaut (courbure terrestre).
  - `refraction` : coefficient de 0 à moins de 1, par défaut 0,13. Il n'agit que si `curvature` est vrai.
- **Échantillonnage.**
  - Pas d'échantillonnage : la plus petite taille de cellule.
  - Altitude d'un échantillon : interpolation bilinéaire. Elle est absente si l'une des quatre cellules voisines est sans donnée.
  - Un échantillon sans donnée ne masque rien et il est compté.
- **Profil.** Il est borné à 256 échantillons dans le schéma (`maxItems`), premier et dernier compris. La visibilité, elle, est calculée sur tous les échantillons.
- **Reproductibilité.** Calcul en f64, déterministe : même entrée, même résultat à l'octet, et le rejeu rend `identical: true`.

**Never:**
- Écrire un fichier.
- Reprojeter.
- Dépendre de GDAL, de GEOS ou de GRASS.
- Le champ de vision (entrée suivante).
- Apprendre la commande au modèle. Le refus `NO_LINE_OF_SIGHT` du générateur reste ; son retrait appartient à l'entrée « Le modèle apprend les trois commandes de terrain ».
- Toute fonction conçue pour un usage militaire.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Masqué par la colline | `mnt-colline-2154.tif` : observateur et cible de part et d'autre du sommet (x = 652030 et 652470, y = 6850250), à 60 m au-dessus du sol | `visible: false`, premier obstacle côté observateur, à moins de 50 m du sommet | — |
| Même versant | observateur (652030, 6850250) et cible (652130, 6850250), à 2 m | `visible: true`, pas d'obstacle | — |
| Courbure | MNT plat de 20 km, construit dans le test ; observateur à 2 m, cible au sol | sans courbure : visible ; avec courbure : masqué | — |
| Mauvais système | un point en EPSG:4326 sur un MNT en 2154 | rien n'est calculé | erreur typée `terrain.crs_mismatch` |
| Hors du MNT | un point hors de l'emprise | — | erreur typée `terrain.outside_dem` |
| Point sans altitude | l'observateur ou la cible sur une cellule sans donnée | — | erreur typée `terrain.no_elevation` |
| Unités | hauteur d'observateur `{"value": 0.06, "unit": "km"}` ; hauteur sans `unit` ou nombre nu | même verdict qu'avec `{"value": 60, "unit": "m"}` ; résultat avec `unit: "m"` | nombre nu ou unité absente : `registry.invalid_params` |
| Points confondus | observateur et cible au même endroit | `visible: true`, profil d'un seul échantillon | — |
| Longue ligne | plus de 256 échantillons | profil réduit à 256 échantillons, verdict calculé sur tous | — |
| CLI et rejeu | `cassini run terrain.line_of_sight` sans QGIS, puis `cassini replay` | même résultat ; rejeu `identical: true` | — |

</frozen-after-approval>

## Code Map

- **`core/src/commands/terrain_line_of_sight.rs`** (nouveau ; `build.rs` déclare seul les modules de `commands/`).
  - Patron : `terrain_slope.rs` (`impl Command`, `.runs_in(RunsIn::Core)`, `.cancellable(false)`, `.path_params(&["dem"])`, `.example(..)`, `.tests(&[..])` avec des noms de tests qui existent, `inventory::submit!`).
  - Pas d'`output_params` : aucun fichier n'est écrit.
  - Descriptions en français et en anglais, comme pour la pente : `dem` est un chemin de fichier MNT recopié de l'inventaire.
  - Points : `crate::point::Point` dans les paramètres et en sortie, comme `analysis_isochrone.rs` (`origin`) et `layout_export_pdf.rs` (`center`).
  - Hauteurs : un type `Height { value, unit }` propre à la commande, qui reprend `DistanceUnit` de `analysis_buffer.rs` (à rendre `Clone + Copy` si besoin, et à exporter s'il ne l'est pas) ; `Distance` n'est pas réutilisée car elle refuse 0. Conversion en mètres (`km` × 1000) dès l'entrée de `line_of_sight`.
- **`core/src/dem.rs`** — en lecture seule :
  - à utiliser : `Dem::open`, `Dem::value(col,row)` (qui rend `None` hors de la grille ou sans donnée), `extent()`, les champs `origin_*`, `cell_size_*`, `epsg`, `width`, `height` ;
  - pour les erreurs : `DemError` → `CommandFailure::new(code, message)`.
- **`training/src/families.rs`** (garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry`, vers la ligne 3337) — exclure `terrain.line_of_sight` comme `terrain.slope`, avec le même commentaire.
- **`cli/tests/parity.rs`** — les exemples de la commande doivent passer sous `run`, `serve` et `mcp`. Le test copie déjà les MNT de recette ; il faut ajouter le cas sans fichier de sortie s'il le faut.
- **`cli/src/main.rs`** — en lecture seule : `cassini run … --project`, `cassini replay`, patron `temp_project()`. Ajouter un test `run` puis `replay` → `identical: true`, comme 4.2.
- **`ai/src/plan.rs`** — le test `the_catalog_comes_from_the_real_registry_and_hides_output_paths` doit rester vert.
- **`fixtures/README.md`** — ajouter `terrain.line_of_sight` dans la colonne « Utilisé par » de la colline.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/terrain_line_of_sight.rs` — paramètres, résultat, calcul et une fonction pure testable, plus les tests de la matrice. C'est la commande elle-même.
- [x] `training/src/families.rs` — exclusion du garde-fou, pour que les tests du générateur restent verts sans rien apprendre au modèle.
- [x] `cli/tests/parity.rs`, `cli/src/main.rs` (tests) — parité des points d'entrée, `run` puis `replay`, pour la ligne « CLI et rejeu ».
- [x] `fixtures/README.md` — la colonne « Utilisé par ».

**Acceptance Criteria:**
- Given le noyau sans QGIS, when tournent `cargo test --workspace`, `cargo fmt --check` et `cargo clippy --workspace --all-targets -- -D warnings`, then tout est vert et le contrôle de contrat accepte la commande.
- Given le résultat de la commande, when on lit son schéma, then `profile` porte `maxItems: 256` et `obstruction` est un `Point` marqué `x-cassini: point`.

## Implementation Notes

- **Interpolation bilinéaire.** Entre les centres des quatre cellules voisines. Sur un axe où l'échantillon tombe pile sur un centre (poids nul de la voisine), seule la cellule de ce centre est lue : sinon un échantillon posé sur une cellule valide serait déclaré sans altitude à cause d'une voisine qui ne pèse rien. À moins d'une demi-cellule du bord de l'emprise, le point prend la valeur du bord.
- **Positions des échantillons.** `distance · i / n` (multiplication avant division), pour qu'un échantillon tombe exactement sur un centre quand il le doit : résultat stable et décompte des sans-donnée exact.
- **Profil réduit.** Au-delà de 256 échantillons, le profil est ré-échantillonné à `D · i / 255` (256 positions également espacées, premier et dernier compris, mêmes règles d'interpolation), plutôt que de prendre un sous-ensemble des échantillons du calcul, dont les rangs ne tombent pas à intervalle constant : `profile_step` vaut alors exactement `D / 255`. Le verdict, l'obstacle et `nodata_samples` portent sur tous les échantillons du calcul.
- **Altitude du profil.** C'est l'altitude du sol interpolée, sans la chute de courbure.
- **Ordre des refus.** Le système des **deux** points est vérifié d'abord (`terrain.crs_mismatch`), puis l'emprise des **deux** points (`terrain.outside_dem`, bord compris dedans), puis l'altitude des deux (`terrain.no_elevation`) : chaque contrôle passe sur les deux points avant le suivant.
- **Altitude d'une extrémité.** Interpolation bilinéaire ; si elle échoue à cause d'une voisine sans donnée, l'altitude est celle de la cellule qui contient le point. Le refus `terrain.no_elevation` ne vient que si cette cellule-là est sans donnée (matrice : « sur une cellule sans donnée »).
- **Paramètres hors bornes.** Hauteur négative et réfraction hors de [0, 1) sont refusées à la désérialisation (`registry.invalid_params`), comme la durée de l'isochrone ; le schéma porte `minimum` et `exclusiveMaximum`.

## Plan Change Log

- **2026-11-04, boucle 1 (intent_gap, constat 3 de la relecture quick).** Mesures nues sur le canal, contraires au contrat d'architecture. Décision du propriétaire : unités explicites. Amendé : bloc figé (paramètres, décision « Unités », ligne « Unités » de la matrice), Code Map (type `Height`), Design Notes (résultat avec `unit`), Implementation Notes (ordre des refus et altitude d'une extrémité, constats 1 et 2). État mauvais évité : hauteurs et distances en nombres nus. **KEEP** : la première version, revenue au point de départ, est gardée en lecture dans `/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/c0201ce7-5fe2-4c1a-9aa9-d5acd496f05f/scratchpad/terrain_line_of_sight.v1.rs` ; en garder l'interpolation (centre exact, bord), les positions `distance · i / n`, le profil ré-échantillonné à `D · i / 255`, les erreurs typées, les 12 tests (à adapter aux unités) et leurs noms dans `.tests(..)`, la description FR/EN ; garder aussi le test `run` puis `replay` de `cli/src/main.rs` (résultat résumé par empreinte, `truncated: true`), l'exclusion de `training/src/families.rs` et la ligne de `fixtures/README.md`.

## Review Triage Log

| # | Lentille | Constat | Verdict | Route | Preuve |
|---|----------|---------|---------|-------|--------|
| 1 | quick | Ordre des refus : l'emprise de la cible n'est vue qu'après l'altitude de l'observateur | low | patch | `line_of_sight` appelle `endpoint(o)` (emprise puis altitude) avant `endpoint(t)` : observateur sur un trou + cible hors MNT → `no_elevation` au lieu de `outside_dem`, contraire aux Implementation Notes ; correction = vérifier l'emprise des deux points avant les altitudes. |
| 2 | quick | Une extrémité posée sur une cellule valide voisine d'un trou est refusée `no_elevation` | medium | patch | `endpoint` passe par `bilinear`, qui lit les quatre voisines : (28, 35) sur `holed()` lit la colonne 3 (trou). La matrice ne refuse que l'extrémité « sur une cellule sans donnée » ; un MNT réel reprojeté a des bords sans donnée. Correction : si l'interpolation échoue, prendre la cellule qui contient le point, refuser seulement si elle est sans donnée. |
| 3 | quick | Mesures nues sur le canal (hauteurs, distances, profil en f64 sans unité) | medium | intent_gap | Contrat d'architecture, table des conventions, ligne « Mesures » : « aucune valeur nue ne traverse le canal (AD-19) » ; `analysis.buffer` prend `Distance {value, unit}`, `terrain.slope` rend `unit` à côté de `min`/`max`. L'intention dit seulement « en mètres » : le format est à trancher par le propriétaire (CLAUDE.md §8, AD à ne pas re-décider en passant). |
| 4 | quick | Relecture `quick` sur une entrée « analyses » (CLAUDE.md §8 : thorough) | false | rejet | Risque `medium` fixé au plan et approuvé par le propriétaire ; la correction serait d'éditer le plan de ce build. Signalé au propriétaire. |
| 5 | quick (boucle 2) | Les défauts des hauteurs n'apparaissent pas dans le schéma des paramètres | low | patch | `Height` et `DistanceUnit` ne dérivent pas `Serialize` : schemars n'écrit pas `default` pour un type qu'il ne sait pas sérialiser, et MCP comme le catalogue du modèle reprennent `params_schema` tel quel. Correction : dériver `Serialize` et tester `default` dans le schéma. |
| 6 | quick (boucle 2) | `f64::hypot` dépend de la libm de la plateforme | medium | patch | `hypot` n'est pas garanti arrondi correctement (MSVC CRT, glibc) : une ligne diagonale peut différer d'un ulp entre Windows et Linux, et le rejeu compare par empreinte → `identical: false` d'une machine à l'autre. `(dx²+dy²).sqrt()` est exact en IEEE. |
| 7 | quick (boucle 2) | Extrémité de repli (cellule brute) discontinue avec l'échantillon voisin interpolé | low | rejet | Comportement exigé par les Implementation Notes (« l'altitude est celle de la cellule qui contient le point ») ; ne se voit que sur une pente au bord d'un trou, et la correction ajouterait des branches. |
| 8 | quick (boucle 2) | La démo réelle n'est pas tracée dans le plan | false | rejet | L'implémenteur l'a faite sur le MNT de Villeneuve-le-Roi (vallée → plateau masqué à 2,85 km, vallée → pied du coteau visible, rejeu `identical: true`) ; la correction serait d'éditer le plan. |

## Design Notes

**Distances.** Pour un échantillon à la distance horizontale `d` de l'observateur, sur une ligne de longueur `D` :

```
chute(d)  = curvature ? (1 − refraction) · d² / (2 · 6 371 000) : 0
sol(d)    = z(d) − chute(d)
œil       = z(0) + observer_height
visée     = z(D) − chute(D) + target_height
ligne(d)  = œil + (visée − œil) · d / D
```

**Obstacle.** Le premier échantillon intermédiaire (0 < d < D) où `sol(d) > ligne(d)` est le premier obstacle. Il est rendu comme `Point` dans l'EPSG du MNT, avec `obstruction_distance`. S'il n'y en a aucun, la cible est visible.

**Résultat.** `{visible, unit: "m", distance, obstruction?, obstruction_distance?, profile: [{distance, elevation|null}], profile_step, nodata_samples}` ; `unit` vaut toujours `"m"` et qualifie toutes les distances et altitudes du résultat.

**Paramètres.** `observer_height` et `target_height` s'écrivent `{"value": 1.6, "unit": "m"}` ; défauts `{1.6, m}` et `{0, m}` ; valeur finie ≥ 0.

**Profil réduit.** Quand le profil dépasse 256 échantillons, on en garde 256 également espacés, premier et dernier compris. `profile_step` donne alors l'écart réel entre deux échantillons rendus.

**Pourquoi la courbure est active par défaut.** Au-delà de quelques kilomètres elle change le verdict. Le coefficient 0,13 est l'usage courant en topographie et dans les outils de visibilité de QGIS.

**Comparaison à GRASS.** Elle se fera dans l'entrée de comparaison ; GRASS `r.los` n'est pas lu ici.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` — attendu : vert. L'espace de travail entier passe sous Windows (WSL n'a pas libclang) ; `-p cassini-core -p training` passe sous WSL.

**Manual checks:**
- Démo réelle : `cassini.exe run terrain.line_of_sight` sur `projet-sonde-4-3.qgz` avec le MNT Lambert-93 de Villeneuve-le-Roi, entre deux points de part et d'autre du coteau de la Seine. Le verdict et l'obstacle doivent être plausibles au vu de la couche de pente.
