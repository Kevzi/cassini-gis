---
title: 'terrain.slope : pente d''un MNT, en degrés ou en pourcentage'
type: 'feature'
ticket: '2'
created: '2026-11-03'
status: 'done'
baseline_revision: '5b83b97739e6ad6f0bacbd92194fde532dd09942'
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

**Problem:** Le noyau lit un MNT (4.1) mais ne calcule encore rien dessus ; la pente est la première des trois commandes de terrain natives (CAP-11), sans GDAL ni GRASS.

**Approach:** Une commande du registre `terrain.slope`, exécutée dans le noyau (`runs_in: core`) : elle ouvre le MNT par `dem.rs`, calcule la pente par la méthode de Horn et écrit un GeoTIFF Float32 que le noyau nomme dans le dossier de travail (AD-10). Le journal et le rejeu existants comparent déjà les octets d'une sortie du noyau : rien n'y change.

## Boundaries & Constraints

**Always:** Décision (propriétaire, 2026-11-03) : le MNT se désigne par un chemin relatif au projet (`dem`, paramètre de chemin, AD-9/AD-20), jamais par un id de couche. Paramètres : `dem`, `unit` (`degrees` | `percent`, requis), `out` (fourni par le noyau). Résultat typé : `file` (le GeoTIFF écrit ; Décision (propriétaire, 2026-11-03) : le journal le garde relatif au dossier de travail, la réponse à l'extension et à la CLI le rend absolu, comme pour `layout.export_pdf`, dont le bouton « Ouvrir le dossier » a besoin), `unit`, `min`, `max` (sur les cellules valides, absents si aucune), `nodata_cells`. Horn avec les tailles de cellule x et y du MNT ; la sortie garde emprise, cellule et EPSG du MNT, sans-donnée -9999. Une cellule de bord, ou dont un des 8 voisins est sans donnée, est sans donnée (comportement de GDAL par défaut et de GRASS). Erreurs `dem.*` du module rendues telles quelles (code et message français). Exécutable par `cassini run terrain.slope --project …` sans QGIS.

**Never:** Ouvrir un raster ailleurs que dans `dem.rs` (AD-20). Reprojeter (AD-19). Toucher au journal, au rejeu ou à l'extension QGIS. Apprendre la commande au modèle (entrée dédiée, ré-entraînement groupé à part). Charger la sortie dans QGIS (entrée 4.3).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan incliné, degrés | `fixtures/mnt-plan-incline-2154.tif`, `degrees` | cellules intérieures = atan(0,1) = 5,7106° (à 1e-4), min = max ; `nodata_cells` = 136 (bord de 40 × 30) | — |
| Plan incliné, pourcentage | idem, `percent` | cellules intérieures = 10 % (à 1e-4) | — |
| Colline | `fixtures/mnt-colline-2154.tif` | minimum au centre, maximale sur un anneau, symétrique ; Décision (propriétaire, 2026-11-03) : le sommet de cette fixture tombe au coin de quatre cellules (pente minimale 3,5°), la pente nulle au sommet (à 1e-3) se vérifie sur la même colline en 51 × 51 construite dans le test ; fixture inchangée | — |
| Trou dans le MNT | une cellule sans donnée au milieu | elle et ses 8 voisines sans donnée | — |
| MNT minuscule | 2 × 2 cellules | toutes sans donnée, `min`/`max` absents | — |
| MNT refusé | WGS84, sans EPSG, illisible, absent | rien n'est écrit | code `dem.*` du module, message inchangé |
| Unité inconnue | `unit: "radians"` | rien n'est exécuté | `registry.invalid_params` |
| Deux exécutions | même MNT, même unité | fichiers identiques octet pour octet | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/terrain_slope.rs` (nouveau ; `build.rs` déclare seul les modules de `commands/`) -- patron : `core/src/commands/test_commands.rs` (`impl Command`, `.runs_in(RunsIn::Core)`, `.path_params(&["dem"])`, `.output_params(&["out"])`, `inventory::submit!`) et `layout_export_pdf.rs` pour `.output_ext(".tif")`, `.example(..)`, `.tests(&[..])` (le contrat 2.3, `core/src/contract.rs`, exige des noms de tests existants et des paramètres présents au schéma). Descriptions FR/EN lues par le modèle : dire que `dem` est un chemin de fichier MNT recopié de l'inventaire.
- `core/src/dem.rs` -- réutiliser `Dem::open`, `Dem::value(col,row)` (rend `None` hors grille, sans-donnée, NaN), champs publics `width/height/origin_*/cell_size_*/epsg/nodata/values`, `Dem::write_geotiff`. `DemError { code, message }` → `CommandFailure::new(code, message)` (`core/src/registry.rs` l.174). Ne pas modifier, sauf la limite de décodage (tâche 2).
- `core/src/journal/mod.rs` -- lecture seule : `output_file` nomme `<étape>-terrain.slope.tif` ; le rejeu (l.~885) compare déjà les octets des sorties `runs_in: core`. `relativize` rend `file` relatif au dossier de travail dans le journal et la réponse (`serve/calls.rs` l.117).
- `cli/src/main.rs` -- `cassini run <id> [json] --project <fichier>` ouvre une `Session` ; `cassini replay <projet>` rend `ReplayReport { steps, identical }`. Tests de patron : `temp_project()` l.~735.
- `ai/src/plan.rs` `catalog_entry` -- la commande entre seule au catalogue du modèle (`out` caché) ; le test `the_catalog_comes_from_the_real_registry_and_hides_output_paths` doit rester vert.
- `fixtures/README.md` -- ajouter `terrain.slope` (4.2) dans « Utilisé par » des deux MNT si la colonne ne le dit pas déjà.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/terrain_slope.rs` -- paramètres, résultat, calcul de Horn (fonction pure sur `Dem`, rend une grille `Dem` de sortie et les statistiques), écriture, conversion des erreurs ; tests de chaque ligne de la matrice (MNT de trou et minuscule fabriqués par `write_geotiff` dans un dossier temporaire effacé en fin de test) -- la commande
- [x] `core/src/dem.rs` -- constat différé 8 de 4.1 : relever la limite de décodage de `tiff` (256 Mo par défaut) à un plafond explicite d'environ 1 Gio, au-delà `dem.unsupported` « MNT trop grand » ; un test du message -- un MNT de 1 m sur une commune doit passer
- [x] `cli/src/main.rs` (tests) -- `cassini run terrain.slope '{"dem":"…plan-incline…","unit":"percent"}' --project <temporaire>` écrit `p.cassini/1-terrain.slope.tif` relu par `Dem::open` à 10 % ; puis `cassini replay` rend `identical: true` -- la preuve sans QGIS
- [x] `fixtures/README.md` -- colonne « Utilisé par » -- traçabilité

**Acceptance Criteria:**
- Given le noyau sans QGIS, when tournent `cargo test --workspace` sous Linux et Windows, then chaque ligne de la matrice est tenue et le contrat du registre (2.3) accepte `terrain.slope`.
- Given un projet temporaire, when on lance `cassini run terrain.slope` puis `cassini replay`, then le GeoTIFF existe dans le dossier de travail et le rejeu rend `identical: true` avec les mêmes octets.
- Given le dépôt, when tourne `cargo deny check`, then il reste vert sans nouvelle dépendance.

## Implementation Notes

- Vérification manuelle (2026-11-03, `C:\ct\debug\cassini.exe`, MNT `donnees-test-cassini/dem/mnt-villeneuve-le-roi-2154.tif`, copié dans un projet temporaire) : degrés min 0, max 49,2598, `nodata_cells` 10 770, 0,85 s ; pourcentage min 0, max 116,096, 10 770, 0,86 s (binaire de débogage). `cassini replay` : `identical: true`.
- Comparaison à `gdaldem slope -alg Horn` (GDAL de QGIS 3.44.15, ce qu'exécute `gdal:slope`) : masque sans-donnée identique cellule pour cellule (10 770), même géotransformation ; écart maximal 3,8e-6° en degrés et 7,6e-6 % en pourcentage, écart moyen ~1e-7 (arrondis f32).
- Limite de décodage : `Limits::decoding_buffer_size` porté à 1 Gio (`MAX_DECODED_BYTES`), et contrôle explicite avant décodage (cellules × max(octets par échantillon, 4)) qui rend `dem.unsupported` « MNT trop grand … plafond de 1 Gio … découpez-le dans QGIS ».

## Plan Change Log

- Colline : le sommet de `mnt-colline-2154.tif` (grille 50 × 50) tombe au coin commun de quatre cellules, si bien qu'aucune fenêtre 3 × 3 n'y est centrée : la pente minimale vaut 3,53°, pas 0. Le test vérifie sur la fixture le minimum au centre, la symétrie (deux axes et diagonale) et le maximum sur un anneau ; la « pente nulle au sommet à 1e-3 » est vérifiée sur la même colline construite en 51 × 51 dans le test (sommet au centre d'une cellule). La fixture versionnée n'est pas changée.
- Parité (`cli/tests/parity.rs`) : `terrain.slope` est la première commande `core` qui lit un fichier et en écrit un. Le test copie les deux MNT de recette dans chaque projet temporaire (les exemples de la commande les nomment) et ramène les chemins sous le dossier du projet à `<projet>/…` avant comparaison : `run`, `serve` (`command.run`) et `mcp --project` rendent `file` en absolu, chacun dans son projet. Les réponses ne sont pas relativisées : l'extension lit `file` absolu (`runlog.result_folder`) ; seul le journal (et le chemin de l'IA externe, `serve/calls.rs`) le relativise.
- Garde-fou `training/src/families.rs` (`what_is_refused_out_of_registry_is_still_missing_from_the_registry`) : il exclut désormais `terrain.slope`. Le refus `NO_SLOPE` du générateur devient faux ; il est laissé à l'entrée « Le modèle apprend les trois commandes de terrain » (Never : apprendre la commande au modèle ici).

## Review Triage Log

Passe 1 (2026-11-03, lentille quick) : high 0, medium 1, low 6, false 0.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | `intermediate_buffer_size` reste à 128 Mio : un MNT de 400 Mo en une seule bande passe `check_size` puis est refusé (`dem.rs`, `Dem::open`) | medium | patch | vérifié dans tiff 0.11.3 `decoder/image.rs` l.894 et `mod.rs` l.501 ; relever aussi cette limite à `MAX_DECODED_BYTES` |
| 2 | Le plafond annoncé « 1 Gio en mémoire » sous-estime le pic réel (tampon décodé et copie en f32) | low | patch | `to_f32` garde les deux ; reformuler : le plafond porte sur la grille |
| 3 | `..dem.clone()` recopie toute la grille d'entrée dans `horn_slope` | low | patch | trois grilles en mémoire à la fois ; copier les champs scalaires un par un |
| 4 | Le test de la CLI ne prouve pas que `file` est absolu, et sa dernière assertion compare un fichier que le rejeu ne touche pas | low | patch | `ends_with` accepte aussi la forme relative ; vérifier `is_absolute()` et comparer avec `replay/1-terrain.slope.tif` |
| 5 | L'exception pour `terrain.slope` du garde-fou du générateur est sans condition | low | patch | la lier à la constante `NO_SLOPE`, pour que la compilation casse si on retire l'une sans l'autre |
| 6 | `strip_prefix(dir)` sans `/` final dans `in_project` | low | patch | `…-run-1` correspondrait à `…-run-10/…` ; comparer avec `"{dir}/"` |
| 7 | Deux messages « MNT trop grand » différents pour un même refus | low | patch | `map_read_error` (`LimitsExceeded`) reprend le texte de `check_size` |

## Design Notes

Horn (1981), noyau 3 × 3 `a b c / d e f / g h i` :
`dz/dx = ((c + 2f + i) − (a + 2d + g)) / (8 · cell_size_x)`, `dz/dy = ((g + 2h + i) − (a + 2b + c)) / (8 · cell_size_y)` ;
`p = sqrt(dx² + dy²)` ; pourcentage `100 · p`, degrés `atan(p)` en degrés. Calcul en f64, écrit en f32. Sur le plan incliné (1 m tous les 10 m), `p = 0,1`. `atan` dépend de la bibliothèque mathématique du système : le rejeu compare sur la même machine, les tests comparent avec une tolérance, jamais à un fichier versionné.

## Verification

**Commands:**
- `cargo test -p cassini-core -p cassini` (WSL) et PowerShell `cargo test --workspace` (`CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH`) -- expected: vert
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement
- PowerShell : `cargo deny check` -- expected: vert

**Manual checks (if no CLI):**
- Hors dépôt : `cassini run terrain.slope` sur `donnees-test-cassini/dem/mnt-villeneuve-le-roi-2154.tif` ; consigner min, max, `nodata_cells` et la durée dans les Implementation Notes, et comparer quelques cellules à `gdal:slope` de QGIS (écart attendu nul aux arrondis près).
