---
title: 'Rasters de résultat : fond noir opaque dans QGIS'
type: 'bugfix'
ticket: ''
created: '2026-11-09'
status: 'built'
baseline_revision: '7b273ef951f77946e7e604bdebfdad37b8c216f7'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [adversarial, edge-cases]
review_loop_iteration: 1
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** recette QGIS du 2026-11-09 sur Annecy : les rasters que le noyau pose (champ de vision, champ de vision cumulé, pente, exposition, surface de coût, superposition pondérée) s'ouvrent dans le rendu gris par défaut de QGIS. Les cellules à 0 (non visible, aucune vue) sont noires et opaques et cachent le fond de carte. Le noyau dicte `layer.open_raster` après chaque sortie `.tif` (`journal/mod.rs` `call_logged`), mais aucun style.

**Approach:** juste après l'ouverture d'un raster de résultat, la session dicte un `layer.set_style` sur la couche ouverte, avec un style que le noyau compose selon la commande : champ de vision, 0 transparent et 1 en couleur semi-transparente ; cumulé, 0 transparent puis dégradé jusqu'au nombre maximal d'observateurs ; pente, exposition, coût et superposition, dégradé de couleurs ; nodata (-9999) transparent partout. L'ombrage garde son style `hillshade`.

## Boundaries & Constraints

**Always:** le style est décidé par le noyau, l'extension ne fait que l'appliquer (AD-4) ; le `layer.set_style` dicté est journalisé et rejoué comme les autres ordres dictés, ids substitués ; palettes sourcées (ColorBrewer, déjà dans `RAMPS`) ; une extension qui n'annonce pas le geste de style ouvre le raster comme aujourd'hui, sans erreur ; tests du noyau sans QGIS ; sonde QGIS sur Annecy, rendu PNG regardé à l'œil.

**Never:** toucher au modèle ou au générateur ; modifier les fichiers produits ; mettre une logique de style dans l'extension.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Champ de vision | `terrain.viewshed` | ouverture, puis style : 0 et nodata transparents, 1 coloré à demi | — |
| Cumulé | `terrain.viewshed_cumulative`, `max_count` = 4 | 0 transparent, dégradé de 1 à 4 | — |
| Pente | `terrain.slope`, unité degrés ou % | dégradé avec l'unité dans la légende | — |
| Ombrage | `terrain.hillshade` | style `hillshade` | — |
| Ancienne extension | geste de style non annoncé | raster ouvert, pas de style | aucune |
| Style refusé | l'extension refuse le style | même conduite qu'un `layer.set_style` refusé de la mise en forme | erreur de l'étape, journalisée |
| Rejeu | journal avec ouverture + style | rejoué identique, id de la couche rejouée | — |

**Décision du propriétaire (2026-11-09)** : nouvelle forme de `StyleSpec` pour les rasters de résultat, `result_ramp` : `stops` `{value, color, label?}` croissants, `interpolation` (`discrete` ou `linear`), `transparent` (valeurs rendues transparentes, nodata compris), `unit` (texte de légende), `opacity`. Ajoutée par un changement de cap court écrit dans ce ticket, puis la fiche « Description de style » du catalogue et la recette `core/tests/catalogue.rs`. Va aux rasters seulement (sinon `style.geometry_mismatch`).

**Décision du propriétaire (2026-11-09, après relecture)** : une extension qui annonce `layer.set_style` mais pas la forme `result_ramp` (extension plus ancienne que ce changement) reçoit le raster ouvert sans style, sans erreur, comme une extension sans le geste. La nouvelle extension annonce qu'elle sait appliquer `result_ramp` à la poignée de main ; le noyau ne dicte le style qu'à cette condition. Le contrat d'architecture gagne l'amendement qui décrit ce style dicté par la session après l'ouverture d'un raster de résultat.

</frozen-after-approval>

## Code Map

- `core/src/journal/mod.rs` l. ~316-370 `call_logged`, `opener_of` -- dicte `layer.open_raster` par sortie `.tif` ; les ordres de l'étape partent avant l'ouverture. Y ajouter le style dicté après l'ouverture, avec le `layer_id` qu'elle rend.
- `core/src/registry.rs` (`dictates`, `ORDERS_FIELD`, `dictated_by_core`) -- déclarer le style dicté des commandes de terrain.
- `core/src/style/spec.rs` (`StyleSpec`, `RampStop`, validation `style.invalid`) -- ajouter `result_ramp`.
- `core/src/style/table.rs` (`RAMPS` l. ~351, `HILLSHADE_OPACITY`, constantes `_SOURCE`) -- palettes des résultats.
- `core/src/commands/terrain_*.rs`, `raster_weighted_overlay.rs` -- valeurs des sorties (viewshed 0/1, cumulé `max_count`, pente `unit`) ; nodata `-9999` (`core/src/dem.rs:44`).
- `qgis-plugin/cassini_qgis/style_gestures.py` (`normalize_style` ~l.227, cibles ~l.58-65, `apply_elevation_ramp` ~l.454) -- traduire la nouvelle forme : `QgsSingleBandPseudoColorRenderer`, valeurs transparentes (`QgsRasterTransparency`).
- `core/src/serve.rs` (`STYLE_GESTURES`, `with_raster_opening`) -- annonce des gestes.
- Tests à suivre : `map_graduated_style_dictates_one_set_style`, `journal/tests.rs` styles et ouverture de raster, `core/tests/catalogue.rs`.

## Tasks & Acceptance

**Execution:**
- [x] Changement de cap court (`_bmad-output/initiative-cassini-mvp/change-style-des-rasters-de-resultat/`) et fiche « Description de style » du catalogue.
- [x] `core/src/style/` -- forme et palettes, validées, testées.
- [x] `core/src/journal/mod.rs` + commandes de terrain -- style dicté après l'ouverture ; tests de la matrice, rejeu compris.
- [x] `qgis-plugin/cassini_qgis/style_gestures.py` + tests de l'extension.
- [x] Sonde QGIS 3.44 sur Annecy (copie du projet, archivée) : champ de vision, cumulé, pente, rendu PNG regardé.

**Acceptance Criteria:**
- Given un champ de vision posé sur Annecy, when on regarde la carte, then le fond de carte se voit sous les zones non visibles.
- Given le dépôt, when `cargo test --workspace` et les tests de l'extension, then tout passe.

## Review Triage Log

Boucle 0 (thorough : adversarial, edge-cases). Une entrée intent_gap (ancienne extension), tranchée par le propriétaire ; le code est gardé et complété plutôt que refait, la correction étant additive. Corrigés 13, rejetés 5, reportés 2.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Extension qui annonce `layer.set_style` sans connaître `result_ramp` : toute analyse de terrain échoue (AD-25) ; rejeu qui diverge après mise à jour | high | intent_gap → décision | annonce de la forme à la poignée de main |
| Second mécanisme de dictée absent du contrat d'architecture | medium | patch (avec la décision) | amendement d'AD-30 |
| Style composé jamais validé, `.ok()` muet | medium | patch | `validate()` avant la dictée, erreur nommée du noyau sinon |
| Exposition : est et ouest de même couleur, libellés Nord/Sud perdus | medium | patch | classes par direction, palette sourcée |
| Cumulé `max_count` = 1 en rouge sombre au lieu de l'orange | low | patch | `CUMULATIVE_LOW` |
| Commentaire Python faux sur `setClip` en discret ; catalogue muet au-delà du dernier palier discret | low | patch | commentaire et catalogue |
| Étiquettes `%g` (1.23457e+06, point décimal) | low | patch | format français sans notation scientifique |
| Description de `layer.set_style` du registre périmée | low | patch | texte du registre |
| Test « extension sans le geste » vrai d'office, compte de lignes faux | low | patch | test réécrit |
| Test du cumulé sans `max_count` > 1 ni `linear` | low | patch | assertions ajoutées |
| Rejeu : le geste de style exigé seulement via `map.topographic_style` | low | patch | `styling_gestures` tient compte de `result_style` |
| `result_style` sur une commande à plusieurs sorties | low | patch | refus déclaré au registre, testé |
| Commande raster du noyau sans style ni exemption | low | patch | test de garde, `raster.reclassify` exempté nommément |
| Anciens journaux rejoués au fond noir | low | patch (doc) | AD-11 : écrit au catalogue |
| Ouverture sans `layer_id` | false | rejet | le schéma de sortie de `layer.open_raster` l'exige |
| `label`/`unit` sans limite de taille | low | rejet | seul le noyau compose ces styles |
| Valeurs transparentes en double | low | rejet | sans effet sur les styles composés |
| `OverflowError` sur un entier JSON énorme | low | rejet | le noyau n'émet que des f64 |
| Raster constant ou vide | false | rejet | une classe valide, rien de noir |
| Chemin QGIS réel non testé, QGIS 4.x non sondé | medium | report | sonde 3.44 faite ; 4.x avec les autres reports |
| Exposition, coût, superposition non sondés dans QGIS | low | report | sonde après correction de l'exposition |

## Verification

**Fait (2026-11-09)** : `cargo test --workspace` vert (645 tests de la bibliothèque du noyau, recette du catalogue comprise), `cargo fmt --all -- --check` propre, `cargo clippy --workspace --all-targets -- -D warnings` sans avertissement ; `python3 -I -m unittest discover -s qgis-plugin/tests` vert (636 tests), `check_extension.py` propre. Noyau `--features gpu` reconstruit et installé dans `%LOCALAPPDATA%\Cassini\bin\cassini.exe` (l'ancien gardé sous `cassini.exe.avant-fond-noir`), extension copiée dans le profil QGIS. Sonde QGIS 3.44.15 (`QT_QPA_PLATFORM=offscreen`, sans modèle) sur une copie d'Annecy, archivée dans `donnees-test-cassini/_archives/sondes/projet-sonde-fond-noir.*` (rendus PNG et script dans `projet-sonde-fond-noir-rendus/`) : champ de vision, cumulé et pente, chacun suivi de `layer.open_raster` puis `layer.set_style` (rendu `singlebandpseudocolor`, 0 et -9999 transparents) ; rejeu dans la même session `{"steps": 9, "identical": true}`. PNG regardés : fond de carte visible sous les zones non vues, champ de vision rouge à demi, pente jaune à rouge sombre. Premier essai du cumulé : la classe la plus claire de YlOrRd (`#ffffcc`) faisait un voile pâle ; remplacée par la 5e classe (`#fd8d3c`, `CUMULATIVE_LOW`, sourcée). Script jetable : `%TEMP%\sonde-fond-noir\sonde.py`.

**Après relecture (2026-11-09)** : les 13 constats corrigés (Review Triage Log). Tests ciblés verts : `cargo test -p cassini-core --lib` (650), recette `--test catalogue`, fmt et clippy du noyau propres ; extension 639 tests, `check_extension.py` propre. Noyau `--features gpu` réinstallé, extension recopiée. Seconde sonde QGIS 3.44.15 sur une copie neuve d'Annecy, archivée `_archives/sondes/projet-sonde-fond-noir-2.*` (rendus dans `projet-sonde-fond-noir-2-rendus/`) : champ de vision, cumulé, pente et exposition, chacun ouvert puis stylé (`style_kinds` annoncé par l'extension) ; exposition en 9 classes par direction (est jaune, ouest rose) ; libellé de pente « 88,91° » ; rejeu `{"steps": 12, "identical": true}`.

**Commands:**
- `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` -- Rust sous Windows seulement : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` (de même pour fmt et clippy) -- expected: vert
- `python3 -I -m unittest discover -s qgis-plugin/tests` et `python3 -I qgis-plugin/tests/check_extension.py` -- expected: vert

**Manual checks (if no CLI):**
- Sonde : construire le noyau `cargo build --release -p cassini --features gpu` (même environnement, plus `VULKAN_SDK`), l'installer dans `%LOCALAPPDATA%\Cassini\bin\` en gardant l'ancien à côté, copier l'extension dans le profil QGIS, lancer un script PyQGIS sous le Python de QGIS 3.44 (`QT_QPA_PLATFORM=offscreen` ou `windows`) sur une copie du projet Annecy de `../donnees-test-cassini/`, archivée ensuite dans `_archives/sondes/` ; modèles voisins : la vérification du plan `plan-bug-rejeu-mise-en-forme-et-couches-creees.md`.
- PNG de la carte avec le champ de vision et la pente : fond visible, couleurs lisibles.
