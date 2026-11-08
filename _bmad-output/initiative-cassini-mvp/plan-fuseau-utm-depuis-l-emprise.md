---
title: 'Fuseau UTM déduit de l''emprise du projet'
type: 'feature'
ticket: ''
created: '2026-11-08'
status: 'built'
baseline_revision: '148dac1c081043c088241bd6fd8c3a7cd3928d4f'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** pour « un carroyage UTM » sans fuseau écrit, le modèle choisit parfois un fuseau sans rapport avec le lieu (EPSG:32621 au lieu de 32632 à Annecy, sonde 11.18) : il ne voit pas où est le projet, car le noyau jette l'emprise de `project.inventory` avant de lui montrer l'inventaire, et l'emprise n'existe qu'en système du projet (Lambert-93), que le noyau n'a pas le droit de convertir (AD-19).

**Approach:** QGIS ajoute à l'emprise de l'inventaire sa version en degrés WGS84 (reprojection par QGIS, AD-19) ; le noyau en déduit le code EPSG du fuseau UTM du centre (`geodesy::utm_zone`, déjà testé) et le montre au modèle dans l'inventaire (`project.utm_epsg`) ; le générateur apprend à recopier ce code pour un carroyage UTM sans fuseau écrit, et garde la question quand l'inventaire n'en a pas. Le modèle l'apprendra au ré-entraînement groupé 11.23.

## Boundaries & Constraints

**Always:** clés de premier niveau du résultat de `project.inventory` inchangées (`extent`, `layers`, `project_crs`, `project_name`) : la version en degrés s'ajoute dans l'objet `extent` (ex. `extent.lonlat = {west, south, east, north}`) ; la reprojection de l'emprise est faite par QGIS ; le noyau ne fait que calculer un numéro de fuseau à partir d'une longitude et d'une latitude (pas une conversion de coordonnées) et l'écrit en code EPSG explicite (326xx au nord, 327xx au sud) ; sans emprise lisible, pas de `utm_epsg` et le comportement d'aujourd'hui reste (question sur le système) ; un fuseau écrit dans la demande l'emporte toujours sur celui de l'inventaire ; les inventaires d'entraînement portent `project.utm_epsg` cohérent avec leurs couches, parfois absent comme `epsg` ; tests du noyau, de l'extension et du générateur.

**Never:** pas de reprojection de couche ni de conversion Lambert-93 dans le noyau ; pas de nouveau paramètre de commande ; pas de ré-entraînement ici (11.23) ; pas de changement du panneau.

**Decisions (Claude, 2026-11-08, nuit autorisée par le propriétaire, à relire au réveil):** la forme `extent.lonlat` dans le résultat existant plutôt qu'une nouvelle clé de premier niveau (le catalogue fige les clés de premier niveau) ; le centre de l'emprise WGS84 détermine le fuseau, même si l'emprise chevauche deux fuseaux.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Annecy | emprise du projet autour de 6,1° E, 45,9° N | `project.utm_epsg = 32632` dans l'inventaire vu par le modèle | — |
| Villeneuve-le-Roi | autour de 2,4° E, 48,7° N | `32631` | — |
| Hémisphère sud | centre à −20° de latitude | `327xx` | — |
| Emprise absente | projet sans couche ou `extent` null | pas de `utm_epsg` | — |
| `lonlat` illisible | valeurs non finies ou hors bornes | pas de `utm_epsg` | aucune erreur |
| Générateur, sans fuseau écrit, inventaire avec `utm_epsg` | « carroyage UTM au kilomètre » | grille `projected` avec `epsg = utm_epsg` | — |
| Générateur, sans fuseau écrit, sans `utm_epsg` | même phrase | question sur le système (comme aujourd'hui) | — |
| Générateur, fuseau écrit | « en UTM 31N » avec `utm_epsg = 32632` | `32631` (le texte l'emporte) | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/inventory.py` l. 88 (`extent_of`, reprojection QGIS), l. 268 (`_extent`), l. 338-366 (`_read` : union des emprises en système du projet) -- ajouter la version WGS84 de l'union (reprojeter les boîtes vers `EPSG:4326` par QGIS, comme `extent_of(layer, to_authid)`), dans `extent.lonlat`.
- `qgis-plugin/tests/test_inventory.py` -- fond doublure existant.
- `core/src/agent_inventory.rs` l. 511-526 (`agent_inventory`) -- lire `extent.lonlat`, centre, `crate::geodesy::utm_zone(lat, lon)` (`core/src/geodesy.rs` l. 176), écrire `project.utm_epsg` ; tests à côté des fixtures existantes (l. ~678, 794).
- `training/src/inventories.rs` l. 1776-1827 -- objet `project` des inventaires (`epsg`, retraits aléatoires de `qgis_shapes`) : ajouter `utm_epsg` cohérent (31 ou 32 selon les couches et le lieu tiré ; 30 possible pour l'ouest), retiré parfois.
- `training/src/families.rs` l. 1778-1846 (`grid_words`), l. 1904 (`GRID_SYSTEM_QUESTION`) -- sans fuseau écrit : grille sur `utm_epsg` quand l'inventaire l'a, question sinon ; tests de famille.
- `docs/entrainement-du-modele.md` -- famille du carroyage et forme de l'inventaire.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- fiche `project.inventory` : décrire `extent.lonlat`.
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- entrée « fuseau UTM sans rapport » (sonde 11.18) : ajouter sa résolution.

## Tasks & Acceptance

**Execution:**
- [x] `qgis-plugin/cassini_qgis/inventory.py` + test -- `extent.lonlat` par QGIS.
- [x] `core/src/agent_inventory.rs` + tests -- `project.utm_epsg` (matrice).
- [x] `training/src/inventories.rs`, `training/src/families.rs` + tests -- inventaires et famille du carroyage.
- [x] docs, catalogue, deferred-work.
- [x] Sonde sous QGIS 3.44 : `project.inventory` sur le projet Annecy rend `extent.lonlat` autour de 6° E, 45,9° N, et l'inventaire vu par le modèle porte `utm_epsg: 32632`.

**Acceptance Criteria:**
- Given le projet Annecy ouvert dans QGIS 3.44, when le noyau prépare l'inventaire pour le modèle, then il porte `project.utm_epsg = 32632`.
- Given le dépôt, when on lance `cargo test --workspace` et les tests de l'extension, then tout passe, dont les tests de la famille du carroyage.

## Implementation Notes

- Extension : `extent.lonlat` (avec `crs: "EPSG:4326"`, AD-19) est l'union des étendues de chaque couche reprojetées vers `EPSG:4326` par `extent_of(layer, "EPSG:4326")` ; une couche sans SCR valide, une reprojection qui lève, une valeur non finie (un NaN casserait le JSON du canal) ou une étendue hors de [-180, 180] × [-90, 90] (des mètres non reprojetés) laisse la couche hors de l'union, sans erreur. Clé absente quand aucune couche n'y entre.
- Noyau : `utm_epsg` exige `lonlat.crs = EPSG:4326`, les quatre bornes finies, une largeur d'au plus 12° de longitude (une couche du monde ou une emprise qui passe l'antiméridien n'a pas de fuseau), longitudes dans [-180, 180], latitudes dans [-90, 90], ouest ≤ est, sud ≤ nord, et un centre dans le domaine de l'UTM (80° S à 84° N) ; schéma de sortie de `project.inventory` complété (`InventoryExtent.lonlat`, facultatif).
- Générateur : un carroyage « UTM » sans fuseau (3 tirages sur 6 des carroyages sans système) recopie `project.utm_epsg`, fixé dans l'inventaire (présent 3 fois sur 4 ; 32631, 32631, 32630 ou 32632 tiré sur des couches en Lambert-93) ; « quadrillage tous les 500 m », « grille de 1 km », « carroyage kilométrique » restent des questions (Lambert-93 et UTM tous deux possibles). Hors de cette famille, `utm_epsg` vient des couches (UTM du vecteur ou du MNT, sinon 31N) sans tirage, et disparaît une fois sur quatre des formes de QGIS ; ce tirage n'a pas lieu quand le fuseau est fixé par la famille (`keep_utm` testé d'abord).
- `training/tests/recouvrement.rs` : un premier essai qui tirait le fuseau au hasard dans chaque inventaire décalait la suite aléatoire et engendrait par hasard la phrase C de `duree_reel` ; un second, qui montait la part des carroyages sans système à 1 sur 4, engendrait Z01 d'`epic11_reel`. Version gardée : le fuseau des inventaires ordinaires vient des couches sans tirage ; seul le retrait d'`utm_epsg` dans les formes de QGIS tire (une fois par inventaire mis en forme, jamais quand le fuseau est fixé), part des carroyages sans système de 1 sur 6. `recouvrement.rs` vert après la correction de relecture. Signal mince (5 plans, 3 questions sur 8 000), à revoir au ré-entraînement 11.23.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 1, medium 3, low 4 ; corrigés 5, reportés en 11.23 : 2, vérification reportée : 1. Choix faits seul par Claude (nuit autorisée), à relire.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Couche du monde ou antiméridien : fuseau faux (32731) | high | patch | pas de `utm_epsg` au-delà de 12° de longitude, testé |
| `lonlat` sans code EPSG (AD-19) | medium | patch | `crs: EPSG:4326`, ignoré sinon |
| Une couche mal projetée supprime le fuseau du projet | medium | patch | boîtes hors degrés et couches sans système écartées une à une |
| Critère vérifié hors du vrai chemin `serve` | medium | report | sonde QGIS d'Annecy de 11.23 par le vrai noyau |
| Tirage aléatoire même avec `keep_utm` | low | patch | `!keep_utm` testé d'abord |
| Doc qui contredit le code | low | patch | doc alignée |
| Fuseau écrit ≠ inventaire jamais engendré dans l'autre sens | low | report | famille dédiée de 11.23 |
| Fuseaux tirés incohérents avec les noms de projet | low | report | famille dédiée de 11.23 |

## Verification

**Commands:** Rust sous Windows seulement (WSL n'a pas libclang) : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` (de même pour fmt et clippy).
- `cargo test --workspace` -- expected: vert
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: vert

**Résultats (2026-11-08)** : `cargo test --workspace` vert ; `cargo fmt --check` et `cargo clippy --all-targets -- -D warnings` sans avertissement ; tests de l'extension 594 OK, garde de l'extension propre. Sonde QGIS 3.44.15 sur `annecy/projet-sonde-utm.qgz` (archivée dans `_archives/sondes/`) : `extent.lonlat = {west 5,564, south 45,522, east 7,234, north 46,470}`, traduit par `model_inventory` du noyau en `project = {epsg 2154, name, utm_epsg 32632}` (traduction faite par un petit binaire jetable lié à `cassini-core`, hors dépôt, plutôt que par `cassini.exe`).

**Manual checks (if no CLI):**
- Sonde jetable sous `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat"` sur une copie du projet Annecy (`annecy/projet-sonde-utm.qgz`, archivée ensuite dans `_archives/sondes/`), avec le noyau construit (`CASSINI_CORE=C:\ct\debug\cassini.exe`) : relever `extent.lonlat` et `utm_epsg`.
