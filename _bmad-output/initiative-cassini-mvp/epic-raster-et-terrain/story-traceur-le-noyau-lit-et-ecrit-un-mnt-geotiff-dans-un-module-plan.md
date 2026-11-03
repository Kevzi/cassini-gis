---
title: 'Traceur : le noyau lit et écrit un MNT GeoTIFF dans un module unique'
type: 'feature'
ticket: '1'
created: '2026-11-03'
status: 'done'
baseline_revision: '21eaf272805b8d48b27feac54d4797cd379a39e7'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le noyau ne sait lire aucun raster ; les trois commandes de terrain en ont besoin, sans GDAL (AD-1) et dans un module unique et remplaçable (AD-20). La maturité de `wbgeotiff`/`wbraster` était à trancher par un cas d'essai.

**Approach:** Cas d'essai fait au plan (2026-11-03) : `wbgeotiff` lit le vrai MNT mais tire `jpeg-encoder` (IJG), refusé par `deny.toml` ; `wbraster` 3 refus. Décision : la crate `tiff` (image-rs, MIT) lit et écrit l'image, et le module `core/src/dem.rs` décode et écrit lui-même le géoréférencement GeoTIFF (emprise, taille de cellule, code EPSG, sans-donnée). Deux MNT synthétiques de recette sont versés dans `/fixtures`.

## Boundaries & Constraints

**Always:** Décision (propriétaire, 2026-11-03) : `tiff = { version = "=0.11.3", default-features = false, features = ["deflate", "lzw"] }`, dépendance approuvée et notée dans `core/Cargo.toml` ; `deny.toml` inchangé et `cargo deny check` vert. Décision (propriétaire, 2026-11-03) : seuls les MNT en système projeté métrique dotés d'un code EPSG sont lus ; un MNT en degrés est refusé par une erreur typée dont le message dit de le reprojeter dans QGIS (Raster > Projections > Reprojeter). Décision (propriétaire, 2026-11-03) : le MNT réel de démo (WGS84) est reprojeté une fois en Lambert-93 dans QGIS ; l'original n'est jamais modifié. `dem.rs` est le seul module du noyau qui ouvre un raster (AD-20). L'écriture est déterministe : même grille, mêmes octets.

**Never:** GDAL, GEOS, PROJ ni aucune crate qui les lie (AD-1). Une commande du registre (entrée 4.2). Reprojeter dans le noyau (AD-19). Les compressions JPEG, WebP, fax (fonctions de `tiff` désactivées). Toucher `wbgeotiff`/`wbraster` dans le dépôt.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| MNT projeté | GeoTIFF Int16/UInt16/Int32/Float32/Float64, 1 bande, EPSG:2154 | grille en f32, emprise, taille de cellule, EPSG 2154, sans-donnée | — |
| Recette plan incliné | `fixtures/mnt-plan-incline-2154.tif` | altitudes exactes du plan connu | — |
| MNT en degrés | GeoTIFF EPSG:4326, ou le vrai MNT de Villeneuve-le-Roi | refus | `dem.not_projected`, message « reprojetez-le dans QGIS » |
| Sans code EPSG | système défini par l'utilisateur, ou sans GeoKey | refus | `dem.no_epsg` |
| Unité non métrique | `ProjLinearUnitsGeoKey` présent et ≠ mètre | refus | `dem.not_projected` |
| Forme non lue | plusieurs bandes, rotation, format d'échantillon inconnu | refus | `dem.unsupported` |
| Fichier illisible | absent, pas un TIFF, sans géoréférencement | refus | `dem.unreadable` |
| Écriture deux fois | même grille | fichiers identiques octet pour octet, relus à l'identique | — |

</frozen-after-approval>

## Code Map

- `core/src/dem.rs` (nouveau, déclaré dans `core/src/lib.rs` l.6-17) -- `Dem` (largeur, hauteur, origine haut-gauche, taille de cellule x/y, `epsg: u32`, `nodata: Option<f64>`, valeurs `Vec<f32>` ligne par ligne) ; `Dem::open(&Path) -> Result<Dem, DemError>` ; `Dem::write_geotiff(&self, &Path)` ; accès `value(col,row)`, `extent()`. `DemError` à codes `dem.*` sur le patron de `CommandFailure::new(code, message_fr)` (`core/src/registry.rs` l.~164) et des erreurs de `core/src/error.rs`. Balises : `ModelPixelScaleTag` 33550, `ModelTiepointTag` 33922, `GeoKeyDirectoryTag` 34735 (clés `GTModelTypeGeoKey` 1024 = 1 projeté, `ProjectedCSTypeGeoKey` 3072, `ProjLinearUnitsGeoKey` 3076 = 9001 mètre, `GeographicTypeGeoKey` 2048), `GDAL_NODATA` 42113 (texte ASCII). En-tête de module : l'unique porte d'entrée raster (AD-20), aucune GDAL (AD-1).
- `core/Cargo.toml` l.10-17 -- ajouter `tiff` avec le commentaire d'approbation daté, sur le patron de `windows-sys`.
- `fixtures/` + `fixtures/README.md` (tableau) -- `mnt-plan-incline-2154.tif` (plan de pente connue, par exemple 10 % vers l'est, cellules de 10 m, quelques dizaines de cellules de côté) et `mnt-colline-2154.tif` (colline lisse, sommet central), Float32, EPSG:2154, écrits par `Dem::write_geotiff` ; un test les régénère et compare aux fichiers versionnés (preuve de déterminisme et de reproductibilité). Ils servent ensuite aux entrées 4.2 à 4.5 et 10.20.
- `deny.toml` -- ne pas modifier ; `cargo deny check` sous Windows (`cargo-deny.exe`).
- Essai du plan, hors dépôt : `wbgeotiff` relit le vrai MNT (901 × 541, Int16, EPSG 4326, sans-donnée -32768, altitudes 12 à 146 m, conformes aux statistiques de GDAL) ; `tiff` 0.11.3 compile sous Windows et Linux et passe `deny.toml`.

## Tasks & Acceptance

**Execution:**
- [x] `core/Cargo.toml`, `core/src/lib.rs`, `core/src/dem.rs` -- lecture, contrôles, écriture, erreurs typées ; tests de chaque ligne de la matrice (les GeoTIFF de refus sont fabriqués dans le test, dans un dossier temporaire) -- le module
- [x] `fixtures/mnt-plan-incline-2154.tif`, `fixtures/mnt-colline-2154.tif`, `fixtures/README.md` -- recette versionnée et test de régénération -- la base des commandes suivantes
- [x] vérification manuelle (hors dépôt) : reprojeter le vrai MNT en Lambert-93 par le Python de QGIS (`gdal:warpreproject`) dans un nouveau fichier `donnees-test-cassini/dem/mnt-villeneuve-le-roi-2154.tif`, puis le relire par le module ; consigner dimensions, EPSG et altitudes min/max dans les Implementation Notes -- un vrai fichier, pas seulement des synthétiques

**Acceptance Criteria:**
- Given le noyau sans QGIS, when tournent les tests de `cassini-core` sous Linux et Windows, then chaque ligne de la matrice est tenue et les fixtures régénérées sont identiques aux fichiers versionnés.
- Given le dépôt, when tourne `cargo deny check` avec `deny.toml` inchangé, then aucune licence n'est refusée et aucune crate GDAL, GEOS ou PROJ n'est dans l'arbre.
- Given le MNT réel reprojeté en Lambert-93, when le module le lit, then il rend EPSG 2154 et des altitudes dans la plage de l'original (environ 12 à 146 m).

## Implementation Notes

- `core/src/dem.rs` : `Dem::open`, `Dem::value` (rend `None` hors grille, en sans-donnée ou NaN), `Dem::extent`, `Dem::write_geotiff`. `DemError { code, message }` à codes `dem.unreadable`, `dem.unsupported`, `dem.not_projected`, `dem.no_epsg`, plus `dem.unwritable` pour l'écriture (grille incohérente, EPSG non inscriptible, échec disque), absent de la matrice mais nécessaire.
- Géoréférencement accepté : point de rattachement unique + échelle, ou matrice `ModelTransformationTag` sans rotation. `GTRasterTypeGeoKey` = PixelIsPoint décale l'origine d'une demi-cellule, comme GDAL (vérifié sur le MNT réel, qui sort de `gdalwarp` en `AREA_OR_POINT=Point`). Plusieurs points de rattachement, rotation, échelle négative : `dem.unsupported`. Types lus : entiers 8/16/32 bits, F16, F32, F64 ; entiers 64 bits et `SampleFormat` hors 1..3 refusés.
- Relecture : sans `ProjLinearUnitsGeoKey` (souvent omise par GDAL), seuls les codes connus en mètres sont acceptés (2154, 3942-3950, 27561-27564, 27571-27574, 32601-32660, 32701-32760, 25828-25838) ; sinon `dem.not_projected` « le mètre ne peut pas être vérifié ». La sans-donnée se compare en f32. `value()` ne panique pas sur une grille incohérente. 7 tests de branches ajoutés (21 au total), dossiers temporaires effacés à la fin de chaque test, `Cargo.lock` limité à `tiff` et ses dépendances.
- Les fixtures n'emploient que +, ×, ÷ (colline en `50 + 100 / (1 + d²/150²)`, pas d'`exp`) pour que les octets soient identiques sous Linux et Windows ; vérifié : le test de régénération passe sur les deux. `CASSINI_REGENERER_FIXTURES=1` les réécrit. `gdalinfo` (QGIS 3.44.15) relit les deux fixtures : EPSG 2154, origine, taille de cellule et sans-donnée -9999 conformes.
- Tâche 3 : `qgis_process run gdal:warpreproject` (EPSG:2154, bilinéaire) → `donnees-test-cassini/dem/mnt-villeneuve-le-roi-2154.tif` ; original intact. Relu par le module : 783 × 712, EPSG 2154, origine (648425,05 ; 6855790,25), cellule 23,656 m, sans-donnée -32768, altitudes 12 à 143 m, 549 712 cellules valides sur 557 496 (98,6 %, comme GDAL). Le maximum passe de 146 à 143 m par le lissage du rééchantillonnage bilinéaire. L'original WGS84 est refusé en `dem.not_projected` avec le message « reprojetez-le dans QGIS (Raster > Projections > Reprojeter) ».
- Vérifications : `cargo test -p cassini-core` (WSL, 304 tests) et `cargo test --workspace` (Windows) verts ; `cargo fmt --check` ; `cargo clippy -p cassini-core --all-targets -D warnings` (WSL) et `cargo clippy --workspace --all-targets -D warnings` (Windows) sans avertissement ; `cargo deny check` (Windows) : advisories, bans, licenses, sources ok, `deny.toml` inchangé. Nouvelles crates tirées : `tiff`, `half`, `quick-error`, `weezl`, `zerocopy`, `crunchy` ; `flate2` en `miniz_oxide` (pur Rust). Aucune crate GDAL, GEOS ou PROJ.

## Plan Change Log

## Review Triage Log

Passe 1 (2026-11-03, lentilles thorough) : high 0, medium 3, low 13, false 1.

| # | Constat (lentille) | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | Système projeté en pieds accepté quand `ProjLinearUnitsGeoKey` est absent (EPSG:2263…) ; GDAL n'écrit souvent que `ProjectedCSTypeGeoKey` (blind, edge) | medium | patch | `check_crs` l.429 ne contrôle l'unité que si la clé est là ; la décision « seuls les MNT métriques » impose le refus : sans clé d'unité, n'accepter qu'une liste fermée de codes métriques connus (2154, 3942-3950, 27561-27564, 27571-27574, 326xx, 327xx, 25828-25838), sinon `dem.not_projected` « unité non vérifiable » |
| 2 | Sans-donnée comparé en f64 à une cellule f32 : une valeur non exacte en f32 n'est pas masquée (blind, edge, gap) | medium | patch | `value()` l.204 `f64::from(v) == nd` ; comparer `v == nd as f32` ; test avec un sans-donnée non entier et avec NaN |
| 3 | Branches non testées : NaN masqué, `GTModelType` absent (projeté accepté, géographique refusé), plusieurs points de rattachement, sans-donnée illisible, point de rattachement non nul (gap) | medium | patch | tests dans `dem::tests` avec `raw_tiff` |
| 4 | `value()` panique sur un `Dem` construit à la main incohérent (blind, edge) | low | patch | `values.get(..)` au lieu de l'indexation |
| 5 | `Cargo.lock` : `cssparser-macros` passe de `syn` 2 à `syn` 3, sans lien avec le ticket (intent, edge, blind, gap) | low | patch | dérive de re-résolution ; rendre la ligne d'origine, `cargo build --locked` vert |
| 6 | Dossiers temporaires des tests jamais effacés (blind) | low | patch | effacer en fin de test |
| 7 | Perte de précision f64/u32/i32 → f32 non documentée (blind, edge) | low | patch | une phrase dans la doc de `Dem` |
| 8 | Limite mémoire implicite de `tiff` (256 Mo) pour un très grand MNT (blind) | low | defer | un MNT 1 m sur une commune peut dépasser ; à fixer avec la première commande (4.2) |
| 9 | Écriture sans fichier temporaire ni garde sur la source (blind, edge) | low | reject | les sorties sont nommées par le noyau dans le dossier de travail (AD-10) ; jamais un fichier d'entrée |
| 10 | L'écrivain accepte un EPSG géographique ou des tailles de cellule non finies (edge) | low | reject | l'écrivain ne reçoit que des grilles issues d'un MNT lu et contrôlé |
| 11 | Octets compressés dépendants du moteur de `flate2` (blind) | low | reject | `miniz_oxide` seul dans l'arbre ; mêmes octets vérifiés sous Linux et Windows |
| 12 | Régénération des fixtures en parallèle des autres tests (blind) | low | reject | commande documentée filtrée sur le seul test |
| 13 | `DemError` non relié au modèle d'erreurs du registre (blind) | low | reject | la conversion appartient à la première commande (4.2) |
| 14 | `Decoder::new` sans `map_read_error`, clés GeoKey hors ligne ignorées, version non lue (blind, edge) | low | reject | formes rares ; le refus reste sûr |
| 15 | Pas de test d'entrée LZW, tuilée, à plusieurs IFD (blind) | low | reject | lues par `tiff` ; le MNT réel reprojeté par QGIS a été relu |
| 16 | Message de « point de rattachement absent » peu précis (edge) | low | reject | diagnostic seulement |
| 17 | Le plan manque au diff (blind) | false | reject | non suivi par git à dessein ; il est lu à part par les lentilles qui en ont besoin |

## Design Notes

Valeurs gardées en f32 : un MNT de 4 000 × 4 000 cellules tient en 64 Mo, et la précision de l'altitude (centimètre) reste bien en deçà. Les tuiles et les bandes (strips) sont toutes deux lues par `tiff`. L'écriture se fait en Float32 compressé `deflate` (pur Rust, déterministe) ; aucune date ni aucun logiciel n'est écrit dans les balises, pour que les octets ne dépendent que de la grille.

## Verification

**Commands:**
- `cargo test -p cassini-core` (WSL) et PowerShell `cargo test --workspace` (`CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH`) -- expected: vert
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement
- PowerShell : `cargo deny check` -- expected: vert, `deny.toml` inchangé

**Manual checks (if no CLI):**
- Relecture du MNT réel reprojeté (tâche 3), chiffres consignés.
