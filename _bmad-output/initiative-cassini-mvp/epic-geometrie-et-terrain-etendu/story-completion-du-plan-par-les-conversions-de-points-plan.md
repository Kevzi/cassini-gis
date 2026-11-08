---
title: 'Complétion du plan par les conversions de points'
type: 'feature'
ticket: '22'
created: '2026-11-08'
status: 'built'
baseline_revision: 'b34eef3c71ceb91df17671e2e9471751ce38b9a8'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-conversion-des-points-ajoutee-au-plan/change-conversion-des-points-ajoutee-au-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Quand une phrase donne un point en WGS84 et que le MNT est en Lambert-93, le modèle local compose la ligne de vue ou le champ de vision cumulé sans `data.transform_point`. Le noyau refuse alors le plan par `terrain.crs_mismatch`. Deux ré-entraînements n'ont pas suffi (sonde d'Annecy du 2026-11-08).

**Approach:** Changement de cap approuvé le 2026-11-08. Dans `plan.propose`, une fois le plan du modèle vérifié, et donc après le garde-fou d'AD-23, le noyau ajoute une étape `data.transform_point` pour chaque point littéral d'une commande du noyau dont le système diffère de celui de son raster. Il lit lui-même ce système (AD-20), et ne fait cet ajout que si l'extension annonce `data.transform_point`. Chaque ajout est :
- un point par étape ;
- placé juste avant la commande qui l'emploie ;
- remplacé dans la commande par `step:k.point` ;
- suivi de la renumérotation des références.

Le plan complété est revérifié, puis gardé, montré, exécuté et journalisé comme un plan ordinaire. `step:k.point` devient admis comme élément d'un tableau de `Point`, et seulement là. Un appel direct n'est jamais complété.

## Boundaries & Constraints

**Always :**
- Le noyau ne reprojette jamais : la conversion reste un geste de QGIS (AD-19).
- Une conversion est toujours une étape visible du plan.
- Le système du raster est lu dans l'en-tête du GeoTIFF, sans décoder l'image : un MNT de 46 Mo ne doit pas ralentir la proposition.
- Un raster cité par `step:k` est suivi jusqu'à l'étape `k`. Si celle-ci est une commande du noyau, on prend son propre raster de premier niveau, de proche en proche (les sorties de terrain gardent la grille de leur entrée). Sinon, le système est inconnu et rien n'est ajouté.
- Un raster illisible ou introuvable n'ajoute rien : l'exécution rend son erreur habituelle.
- Le garde-fou d'AD-23 accepte un élément `step:k.point` d'un tableau de `Point` s'il désigne bien un `Point`, comme il le fait déjà au premier niveau.

**Never :**
- Pas de complétion pour `command.run`, MCP, la CLI `run` ou le rejeu.
- Pas de seconde conversion quand le point est déjà une référence (`step:k.point`, ou toute autre référence).
- Aucune référence `step:` dans un tableau qui n'est pas un tableau de `Point`.
- Ne pas toucher aux descriptions Rust des commandes ni au message de `terrain.crs_mismatch`. Elles sont lues par le modèle, et le ré-entraînement groupé (11.23) suivra.
- Aucun changement du générateur ni de l'entraînement : c'est l'affaire de 11.23.

## I/O & Edge-Case Matrix

| Scénario | Plan du modèle (MNT `mnt.tif` en 2154) | Plan complété | Erreur |
|---|---|---|---|
| Un point WGS84 | `terrain.viewshed {observer: P4326}` | 1 `data.transform_point {point: P4326, target_epsg: 2154}`, 2 `viewshed {observer: "step:1.point"}` | — |
| Deux points | `line_of_sight {observer: A4326, target: B4326}` | deux conversions, puis la ligne de vue avec `step:1.point` et `step:2.point` | — |
| Liste de points | `viewshed_cumulative {observers: [A4326, B2154, C4326]}` | conversions de A et de C, `observers: ["step:1.point", B2154, "step:2.point"]` | — |
| Déjà converti | `1 transform_point → 2 viewshed {observer: "step:1.point"}` | inchangé | — |
| Bon système | `viewshed {observer: P2154}` | inchangé | — |
| Étapes avant et après | `1 terrain.slope, 2 viewshed {observer: P4326, dem: "step:1"}… 3 X {r: "step:2"}` | conversion insérée avant l'étape 2 ; les références renumérotées (`step:1` → `step:1`, ancien `step:2` → `step:3`) | — |
| Geste non annoncé | point WGS84, `data.transform_point` absent des annonces | inchangé | `terrain.crs_mismatch` à l'exécution |
| Appel direct | `command.run terrain.viewshed` avec un point WGS84 | aucune complétion | `terrain.crs_mismatch` |
| Raster illisible | `dem: "absent.tif"` | inchangé | erreur habituelle à l'exécution |

</frozen-after-approval>

## Code Map

- `core/src/serve/plans.rs::propose` : après `short_ids.restore` et `verify(state, &plan)`, appeler la complétion, puis `verify` une seconde fois avant de garder le plan. La disponibilité se teste par `is_available(state, "data.transform_point", Some(RunsIn::Qgis))` (`core/src/serve.rs:921`). Le dossier du projet vient de `state.session`.
- `core/src/plan.rs` :
  - `check_plan` (l. 222) refuse aujourd'hui toute référence imbriquée (`nested_reference`). Admettre `step:k.point` comme élément d'un tableau dont `items` est un `Point` (`point::is_point`), avec la même vérification que l'étape `k` est antérieure et que son champ est un `Point`.
  - `resolve_references` (l. ~370) doit résoudre ces éléments de tableau.
  - `raster_markers` reste inchangé.
- Nouveau module `core/src/plan_completion.rs` (`pub fn complete_conversions(registry, plan, raster_epsg: &dyn Fn(&str) -> Option<u32>) -> Plan`). C'est une fonction pure, testable sans canal. La lecture des fichiers passe par la fermeture fournie par `plans.rs`. Elle :
  - repère les points (`is_point`) au premier niveau et dans les tableaux de points ;
  - repère le raster par `raster_path::is_raster_path` ;
  - ne traite que les commandes du noyau (`runs_in` différent de `Qgis`) ;
  - insère les étapes et renumérote les références de premier niveau et celles des tableaux.
- `core/src/dem.rs` : extraire de `Dem::open` une lecture de l'en-tête seul, `pub fn epsg_of(path) -> Result<u32, DemError>` : mêmes contrôles jusqu'à `check_crs`, sans `read_image`.
- `core/src/journal/mod.rs::resolve_path` (l. 882) : même règle de chemin relatif au projet. Exposer un accès en lecture sur `Session` (par exemple `Session::input_path`), sans dupliquer la normalisation.
- `core/src/point.rs` : `invents_a_point` / `walk` (l. 622). Une chaîne en position de `Point` marque aujourd'hui un doute. Y accepter un élément `step:k.point` qui passe `reference_is_a`.
- Commandes concernées, sans y toucher : `terrain_viewshed`, `terrain_line_of_sight`, `terrain_viewshed_cumulative`, `terrain_cost_distance`, `terrain_least_cost_path`.
- Documents :
  - `architecture-noyau-et-facade-qgis.md` : amendements d'AD-19 (l. 431) et d'AD-23 (l. 516) ;
  - `spec-cassini-mvp/catalogue-des-commandes.md` : une phrase dans les cinq fiches ;
  - `spec-cassini-mvp/spec-cassini-mvp.md` : une phrase dans CAP-15 (l. 73) ;
  - `docs/canal-local.md` : `plan.propose` peut rendre des étapes ajoutées par le noyau.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/dem.rs` -- `epsg_of`, en-tête seul ; essai : même code que `Dem::open` sur les fixtures -- lire le système sans décoder.
- [x] `core/src/plan.rs` -- admettre `step:k.point` dans les tableaux de `Point` (vérification et résolution) ; essais : accepté dans un tableau de points, refusé ailleurs, refusé vers une étape ultérieure ou vers un champ qui n'est pas un `Point` -- amendement d'AD-23.
- [x] `core/src/point.rs` -- le garde-fou accepte ces éléments ; essai -- un plan complété ne doit pas devenir une question.
- [x] `core/src/plan_completion.rs` (+ `lib.rs`) -- `complete_conversions` ; essais : tous les cas de la matrice qui ne passent pas par le canal, dont la renumérotation -- le cœur du ticket.
- [x] `core/src/journal/mod.rs`, `core/src/serve/plans.rs` -- brancher la complétion dans `propose` ; essais de canal : un point WGS84 sur la fixture 2154 donne le plan complété, s'exécute (conversion simulée par l'exécutant de test) et se rejoue à l'identique ; sans le geste, le plan est inchangé ; `command.run` reste refusé par `terrain.crs_mismatch`.
- [x] Documents (architecture, catalogue, spec, `docs/canal-local.md`) -- amendements datés du 2026-11-08.

**Acceptance Criteria:**
- Given le noyau et l'extension installés sur le projet d'Annecy, when on envoie au vrai modèle « Le point 45,9075 N, 6,1700 E (WGS84) voit-il le point 45,8992 N, 6,1294 E (WGS84) sur le MNT à 5 m ? », then le plan montré convertit les deux points puis calcule la ligne de vue, il s'exécute, et le rejeu est identique.
- Given la même session, when on demande un champ de vision cumulé depuis deux points WGS84, then le plan est complété et exécuté.
- Given un plan complété, when on relit le journal, then les étapes `data.transform_point` y sont des lignes ordinaires, avec leur `plan_step`.

## Design Notes

L'ordre des points d'une étape suit l'ordre des paramètres dans le JSON du plan, puis l'ordre des éléments dans un tableau : le résultat est déterministe. Le plafond de 12 étapes (`ai/src/plan.rs`, `MAX_STEPS`) ne vaut que pour la sortie du modèle. Un plan complété peut le dépasser, borné par celui des commandes (32 observateurs au plus).

## Verification

**Commands:**
- `cargo test -p cassini-core` et `cargo test -p cassini-ai` -- expected: verts.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` -- expected: propres.
- `python -m unittest discover qgis-plugin/tests` -- expected: verts (aucun changement attendu).

**Manual checks:**
- Sonde QGIS sur Annecy (critères d'acceptation), après installation du noyau, avec le modèle installé (terrain-11-18b).

## Review Triage Log

Passe 1, lentilles blind-hunter (14), edge-case-hunter (5), verification-gap (1 + 1), intent-alignment (descriptif). Bilan : 0 high, 0 medium, 6 low, 11 false ; 5 patch, 1 defer, le reste rejeté.
- low, patch : l'ordre des points est alphabétique (serde_json sans `preserve_order`), et les commentaires disent « ordre du plan ». Commentaires corrigés, essai `terrain.least_cost_path` (blind-hunter, verification-gap).
- low, patch (gap pré-vérifié) : aucun essai ne renumérote une référence à champ, ou un élément de tableau, dont le numéro change. Essai ajouté.
- low, patch : `grid_of` est appelé deux fois par point, avec un `expect`. Un seul appel.
- low, patch : aucun essai ne prouve que `epsg_of` ne décode pas l'image. Essai sur une image tronquée.
- low, patch : `docs/canal-local.md` disait « point » au lieu de « point littéral ».
- low, defer : les mesures et la composition de la CLI notent le plan brut, sans complétion. Cela concerne les attendus des essais réels de 11.23.
- false : `resolve_references`/`renumber` sur tout tableau. `check_plan` (dans `verify`, avant la proposition et avant l'exécution) refuse toute référence dans un tableau qui n'est pas de `Point` : aucun chemin n'y mène.
- false : un échec du second `verify` refuse un plan valide. Le plan complété n'a changé que là où le plan brut finissait sur `terrain.crs_mismatch`, et un défaut de complétion reste visible au lieu d'être caché.
- false : pas de marque « ajouté par le noyau ». L'intention veut une étape ordinaire, montrée et journalisée comme les autres.
- false : nombre d'étapes non borné. Chaque commande borne ses points (32 au plus), et une conversion n'est ajoutée que pour un point d'un autre système ; aucun cas réel n'approche la borne théorique.
- false : la complétion vaut pour toute commande du noyau, pas pour cinq seulement. C'est la règle d'AD-19 telle qu'amendée (« une commande du noyau »).
- false : la règle « même grille » n'est pas vérifiée. Le noyau ne reprojette jamais (AD-19) : aucune de ses sorties raster ne change de système.
- false : rien n'est ajouté sans session. Sans projet ouvert, aucun plan ne s'exécute (`execute` exige la session).
- false : un `epsg` flottant (4326.0). `Point.epsg` est un `u32` : `check_plan` refuse le plan avant la complétion.
- false : un point cité par une référence vers un autre système n'est pas converti. L'intention ne vise que les points littéraux.
- false : raster cité `step:k.champ`. `check_plan` refuse un raster cité avec un champ (amendement d'AD-24).
- false : `ticket: '22'`. C'est l'`id` de l'entrée, comme le veut le gabarit. La sonde QGIS reste à faire et figure dans Verification.
