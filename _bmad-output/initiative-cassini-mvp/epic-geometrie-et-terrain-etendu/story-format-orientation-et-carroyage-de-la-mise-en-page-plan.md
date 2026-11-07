---
title: 'Format, orientation et carroyage de la mise en page'
type: 'feature'
ticket: '11'
created: '2026-11-07'
status: done
baseline_revision: '22835462d81e566141ec6a19847c226332159d7d'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `layout.export_pdf` ne sort qu'une page A4 paysage sans carroyage ; une demande « imprime en A3 avec un carroyage UTM au kilomètre » n'a pas de commande (G11, CAP-5).

**Approach:** Le noyau déclare trois paramètres facultatifs déjà figés au catalogue — `page_size` (`a4` défaut, `a3`, `a2`, `a1`, `a0`), `orientation` (`landscape` défaut, `portrait`), `grid` (union étiquetée par `kind` : `degrees` avec `interval: Angle`, ou `projected` avec `epsg` entier et `interval: Distance`) — et l'extension les exécute par la mise en page de QGIS (taille de page, grille de la carte).

## Boundaries & Constraints

**Always:** noms, types et valeurs exactement ceux de la fiche du catalogue ; sans ces trois paramètres, le fond reçoit les mêmes arguments qu'avant et le PDF est inchangé (A4 paysage, sans grille) ; le résultat de la commande ne change pas ; un `epsg` de grille inconnu de QGIS rend `data.unknown_crs` sans fichier ; tout ce qui est refusé l'est avant l'export, sans fichier laissé ; accès à Qt par `qgis.PyQt` seulement ; aucune classe de QGIS nommée dans le noyau (AD-22).

**Never:** pas d'étiquettes MGRS (la grille de QGIS n'en a pas) ; pas de nombre d'exemplaires ni d'impression ; pas de nouvel exemple d'entraînement ni de ré-entraînement (entrée 16) ; pas de mise en page ajoutée au projet ; pas de dépendance nouvelle.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Défauts | `{"title": "Carte"}` | A4 paysage, sans grille, fond appelé comme avant | — |
| A3 paysage UTM km | `page_size: a3`, `grid: {kind: projected, epsg: 32631, interval: {value: 1, unit: km}}` | page 420 × 297 mm, grille de 1 000 m en EPSG:32631 sur la carte | — |
| Portrait degrés | `page_size: a4`, `orientation: portrait`, `grid: {kind: degrees, interval: {value: 0.01, unit: deg}}` | page 210 × 297 mm, graticule WGS84 de 0,01° | — |
| Grades | `interval: {value: 1, unit: gon}` | graticule de 0,9° | — |
| Valeur hors liste | `page_size: "a5"`, `orientation: "paysage"`, `grid.kind: "mgrs"` | — | `registry.invalid_params` au noyau ; `layout.bad_params` au geste, sans fichier |
| Intervalle nul ou négatif | `interval.value: 0` | — | `registry.invalid_params` (types `Angle`/`Distance`) ; `layout.bad_params` au geste |
| Système inconnu | `grid.epsg: 999999` | — | `data.unknown_crs`, sans fichier |
| Grille trop dense | plus de 200 lignes par axe sur le cadre de la carte | — | `layout.bad_params`, sans fichier |

</frozen-after-approval>

## Code Map

- `core/src/commands/layout_export_pdf.rs` -- `LayoutExportPdfParams` (`deny_unknown_fields`) : ajouter les trois champs `Option`, types `PageSize`, `PageOrientation` (enums sans commentaire par variante, `rename_all = "lowercase"`, cf. `analysis_isochrone.rs:11`) et `MapGrid` (`#[serde(tag = "kind", rename_all = "snake_case")]`, cf. `core/src/plan.rs:28`). Réutiliser `crate::angle::Angle` et `crate::commands::analysis_buffer::Distance`. `epsg: u32` comme `Point`. Mettre à jour les descriptions FR/EN (A4 paysage par défaut, format, orientation, carroyage), `.example(...)` (un A3 carroyé), `.tests(&[...])`.
- `core/tests/catalogue.rs` -- recette qui confronte la fiche au registre ; gère déjà unions, `Angle`, `Distance`. Ne pas la modifier.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md:583-633` -- fiche `layout.export_pdf` : passer `page_size`, `orientation`, `grid` de `planned_params` à `params` ; `planned_by` ne garde que l'epic 10, entrée 15 (`skipped_layers`) ; phrase d'intro « Aujourd'hui une page A4 paysage… » mise au présent.
- `qgis-plugin/cassini_qgis/layout_export.py` -- `LayoutBackend.export_pdf` monte une A4 paysage à positions fixes en mm (297 × 210) ; `LayoutExportGesture._read` valide les paramètres puis appelle le fond. Les codes d'erreur sont des constantes du module ; `data.unknown_crs` est `point_transform.UNKNOWN_CRS` (`knows(epsg)` y montre comment tester un code).
- `qgis-plugin/tests/test_layout_export.py` -- `FakeBackend` ; `test_without_scale_the_backend_gets_the_same_arguments_as_before` exige `kwargs == {}` sans les nouveaux paramètres : à garder vrai.
- `qgis-plugin/tests/probe_layout_export.py` -- sonde sous le vrai QGIS 3.44 (projet hors dépôt) : y ajouter le cas A3 paysage UTM km.
- `qgis-plugin/README.md:200` -- doc d'usage de l'export.
- `ai/` -- le prompt long liste les schémas : les tests de `cassini-ai` doivent rester verts ; aucun changement de famille d'entraînement ici.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/layout_export_pdf.rs` -- ajouter les types et champs, descriptions, exemple, et des tests : les trois paramètres sont facultatifs et absents de `required` ; valeurs permises et refusées (matrice) ; union refuse un `kind` inconnu ou un champ manquant ; `check_contract()` vide -- G11 côté noyau.
- [x] `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- déplacer les paramètres planifiés vers `params` -- la recette du catalogue doit rester verte.
- [x] `qgis-plugin/cassini_qgis/layout_export.py` -- valider `page_size`, `orientation`, `grid` dans le geste (`layout.bad_params`), passer au fond `page=(size, orientation)` et `grid=...` seulement s'ils sont donnés ; le fond pose la taille de page, recalcule les positions des éléments en proportion de la page, ajoute une grille à la carte (système, intervalle en unités du système, étiquettes en décimal) et refuse une grille trop dense ou un système inconnu avant l'export ; la docstring du module est mise à jour -- G11 côté extension.
- [x] `qgis-plugin/tests/test_layout_export.py` -- tests à doublure de chaque ligne de la matrice côté geste, plus « défauts : kwargs inchangés ».
- [x] `qgis-plugin/tests/probe_layout_export.py` -- cas A3 paysage avec `grid` projeté EPSG:32631 à 1 km : la boîte de page du PDF vaut 1190,55 × 841,89 pt (± 1 pt), la carte de mise en page porte une grille active de ce système et de cet intervalle ; un export sans les nouveaux paramètres garde 841,89 × 595,28 pt.
- [x] `qgis-plugin/README.md` -- décrire format, orientation et carroyage.

**Acceptance Criteria:**
- Given une session QGIS 3.44 avec le projet test, when la sonde exporte en A3 paysage avec un carroyage UTM au kilomètre, then le PDF a la taille de page A3 paysage et la grille est présente sur la carte.
- Given un journal écrit avant ce ticket, when il est rejoué, then le PDF sort en A4 paysage sans grille, comme avant.
- Given la recette du dépôt, when elle tourne, then la recette du catalogue, les tests du noyau, de l'IA et de l'extension passent, `cargo fmt` et `cargo clippy` sans avertissement.

## Implementation Notes

- Noyau : `PageSize`, `PageOrientation` (`#[schemars(inline)]`, rendus en chaîne + `enum`), `MapGrid` (`tag = "kind"`, `deny_unknown_fields`). `Angle` admet 0 et les négatifs : la variante `degrees` passe par `positive_angle` pour que `interval.value: 0` donne `registry.invalid_params`, comme le veut la matrice.
- Extension : le geste valide et normalise (`_page_param`, `_grid_param`), vérifie `knows(epsg)` avant tout (`data.unknown_crs`) et ne passe `page`/`grid` au fond que s'ils sont donnés. Le fond refuse par `GridError` (→ `layout.bad_params`, fichier ôté) une grille de plus de 200 lignes par axe, un système géographique pour `projected` ou un cadre non reprojetable ; `check_grid_density` est partagé avec la doublure des tests.
- Étiquettes de grille à gauche et en bas seulement : à droite elles mordaient sur la légende (vu au rendu A3).
- Deux tests hors du Code Map ajustés : `ai/src/plan.rs` (l'entrée du long prompt de l'export garde maintenant `MapGrid`, `Angle`, `Distance` dans `$defs`, le `Point` caché en sort toujours) et le garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry` de `training/src/families.rs` (vérification des noms `page`/`size`/`orientation` ôtée, comme en 11.9). Le refus `ONLY_A4` du générateur reste, noté dans `deferred-work.md` pour l'entrée 16.

- Après relecture (orchestrateur, 2026-11-07) : correctifs 1, 2, 4 appliqués ; 435+ tests de l'extension verts, `cargo test -p cassini-core` vert (dont la recette du catalogue), `cargo fmt --check` et `cargo clippy -p cassini-core` propres ; sonde `probe_layout_export.py` sous QGIS 3.44.15 : `RÉSULTAT : OK` (A3 paysage 1191 × 842 pt, grille EPSG:32631 de 1 000 m active ; export par défaut inchangé).

## Plan Change Log

## Review Triage Log

**Passe 1 (quick)** — high 0, medium 1, low 5, false 1, maybe-false 1.

| # | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|
| 1 | `_grid_param` refuse `epsg: 0` en `layout.bad_params` alors que le noyau l'admet (u32) et que la règle veut `data.unknown_crs` | low | patch | Condition `epsg <= 0` ôtée, `knows()` tranche ; test ajouté. |
| 2 | `check_grid_density` compte des intervalles, pas des lignes : 201 lignes passent | low | patch | Lignes = ⌊côté / intervalle⌋ + 1, test corrigé. |
| 3 | `LayoutBackend.knows` recopie `point_transform.QgisBackend.knows` | low | rejet | Trois lignes ; le partager demanderait un mélange de classes de fond, pour un risque de dérive faible. |
| 4 | Première phrase de la docstring : « mise en page A4 paysage » sans réserve | low | patch | « A4 paysage par défaut ». |
| 5 | Étiquettes du bas et de gauche peut-être sur la barre d'échelle ou hors marge en A4 | maybe-false | rejet | Déduit des cotes, pas d'un rendu ; l'agent a regardé les rendus A3 et portrait sans chevauchement vu. Au plus cosmétique (low) ; à trancher en regardant un PDF A4 carroyé. |
| 6 | La famille de refus `ONLY_A4` apprend encore à refuser A3/portrait ; le garde-fou n'y veille plus | medium | defer | Réel, mais l'intention exclut l'entraînement (Never) ; noté à `deferred-work.md` pour l'entrée 16. |
| 7 | Le schéma de `degrees.interval` (un `Angle`) n'annonce pas « > 0 » au modèle | low | rejet | Le refus tient à la validation (`registry.invalid_params`) ; l'annoncer demanderait un type d'angle positif à part. |
| 8 | Le passage de la sonde n'est consigné nulle part ; la substitution de `exportToPdf` sur une classe SIP est peut-être sans effet | false | rejet | Sonde rejouée après correctifs sous QGIS 3.44.15 : `RÉSULTAT : OK`, A3 (1191, 842) pt et grille `('EPSG:32631', 1000.0, 1000.0, True)` relue : la substitution agit. |

## Design Notes

- Disposition : les positions actuelles sont pensées pour 297 × 210 mm ; pour une autre page, chaque abscisse et largeur est multipliée par `largeur_page / 297`, chaque ordonnée et hauteur par `hauteur_page / 210`, les tailles de police par le plus petit des deux facteurs. A4 paysage donne les facteurs 1 : rendu identique à l'octet près des positions.
- Taille de page : les noms QGIS « A4 » … « A0 » et `QgsLayoutItemPage.Orientation` ; dimensions vérifiables par `layout.pageCollection().page(0).pageSize()`.
- Grille : `QgsLayoutItemMapGrid` ajoutée à `carte.grids()`, système `EPSG:4326` pour `degrees` (intervalle en degrés, grades × 0,9), `EPSG:<epsg>` pour `projected` (intervalle en mètres, `km` × 1 000). Densité : nombre de lignes par axe = côté du cadre de la carte (ramené au système de la grille) / intervalle, plafonné à 200 (AD-8).
- Le résultat ne change pas : `page_size`, `orientation` et `grid` sont des paramètres, donc déjà au journal, et le rejeu les repasse tels quels.

## Verification

**Commands:**
- `cargo test -p cassini-core layout_export_pdf` -- expected: tous verts
- `cargo test -p cassini-core --test catalogue` -- expected: vert
- `cargo test --workspace` -- expected: vert (dont `cassini-ai`)
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement
- `python3 -m unittest discover -s qgis-plugin/tests` -- expected: vert

**Manual checks (if no CLI):**
- Sonde `probe_layout_export.py` sous `python-qgis-ltr.bat` (QGIS 3.44.15, Windows) : tous les cas OK, puis ouvrir le PDF A3 et voir la grille.
