---
title: 'Surface des entités (analysis.area)'
type: 'feature'
ticket: '22'
created: '2026-11-20'
status: 'built'
baseline_revision: '5f0bc2792e1328463c3a9a712234a8771625c2e2'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Aucune commande ne rend la surface des entités : « les parcelles de plus de 2 500 m² en zone U » ne se compose pas, car la sélection par attribut n'a pas de champ de surface à comparer.

**Approach:** Ajouter `analysis.area`, déléguée à `native:fieldcalculator` avec une formule fixe de la ligne de table (AD-23, AD-24) : un champ de surface plane ajouté sur une couche nouvelle, dans l'unité demandée (`m2` ou `ha`). Le nom du champ est **fixe et porte l'unité** (`area_m2`, `area_ha`), pour que la sélection numérique qui suit compare dans la bonne unité sans signal muet.

## Boundaries & Constraints

**Always:** Paramètres `layer_id`, `unit` (`m2` ou `ha`, énumération), `out` (fourni par le noyau, `.gpkg`), `work_epsg` (facultatif, calculé par le noyau comme `analysis.buffer`, `.work_crs_param("work_epsg", "layer_id")`) ; couche de polygones exigée, vérifiée avant lancement ; formule `area($geometry)` (plane, dans le système de la couche ou du système de travail ; jamais `$area`, qui dépend de l'ellipsoïde du projet) divisée par 10 000 pour `ha` ; champ décimal nommé `area_m2` ou `area_ha`, remplacé sur place s'il existe déjà (constaté par sonde) ; résultat `layer_id`, `name`, `crs`, `feature_count`, `field`, `unit` ; chaîne de version 1 ; annulable ; une couche en degrés se mesure dans le système en mètres du noyau (changement de cap du 2026-11-12), jamais refusée ; la description de la commande nomme les deux champs pour que le modèle les écrive dans la sélection qui suit.

**Never:** Formule écrite par le modèle ou par l'utilisateur ; `$area` ; seuil par défaut (le seuil reste un paramètre écrit par l'utilisateur) ; modification d'`analysis.select_by_attribute` ; `SummaryRow` ; famille d'exemples pour le modèle (réentraînement du ticket 17).

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Mètres carrés | carré de 100 m de côté en Lambert-93, `m2` | champ `area_m2` = 10 000 ; résultat `field: area_m2`, `unit: m2` | aucune |
| Hectares | même carré, `ha` | champ `area_ha` = 1 | aucune |
| Couche en degrés | WGS84 | calcul dans `work_epsg`, résultat en mètres carrés ou hectares | aucune |
| Géométrie non polygonale | lignes ou points | refus avant lancement | `layer.bad_geometry` (ou le code existant le plus proche) |
| Unité inconnue | `km2`, majuscule, nombre | refus à la validation | `registry.invalid_params` |
| Champ déjà présent | la couche porte `area_m2` | remplacé, constaté par la sonde | aucune |
| Enchaînement | `area` puis sélection `area_m2 gt 2500` | seules les entités de plus de 2 500 m² restent | aucune |

</frozen-after-approval>

## Code Map

À construire **après** le commit de 10.21 (dépendance) : les numéros de ligne ont bougé, se repérer aux symboles.

- `core/src/commands/analysis_buffer.rs` -- `.work_crs_param("work_epsg", "layer_id")` et paramètre `work_epsg: Option<u32>` à reprendre ; `analysis_sum_line_lengths.rs` -- énumération d'unité (`unit_is_m_or_km`) à imiter ; nouveau `analysis_area.rs`, enregistré dans `core/src/commands/mod.rs`
- `core/src/commands/analysis_select_by_attribute.rs`, `core/src/attribute_guard.rs` -- ne pas modifier ; vérifier que `field: "area_m2"` après une étape `analysis.area` n'est pas refusé par le garde-fou d'inventaire, sinon le consigner comme question à l'utilisateur
- `core/src/journal/tests.rs`, `core/tests/conformite_resultats.rs`, `fixtures/resultats-extension.json` -- bras simulé avec `work_epsg`, rejeu, exemple de résultat
- catalogue des commandes (`catalogue-des-commandes.md`, section `analysis.area` et ligne de table) -- `planifiée` → `faite` ; params et résultat déjà conformes ; `core/tests/catalogue.rs`
- `qgis-plugin/cassini_qgis/analysis.py` -- `_REPROJECT_TO_WORK`, `translate_sum_prepared` (exemple de `native:fieldcalculator` : `FIELD_NAME`, `FIELD_TYPE` 0 décimal, `FORMULA`) ; ligne `analysis.sum_line_lengths` (`metric`, `work_crs: WORK_EPSG`, `geometry: "Polygon"`) ; `CANCELLABLE_ALGORITHMS` contient déjà `native:fieldcalculator` ; nouvelle `translate_area`, ligne `analysis.area`, lecture de `field` et `unit` dans le résultat
- `qgis-plugin/cassini_qgis/summary.py`, `texts.py` -- « Surface des parcelles en hectares : champ area_ha, nouvelle couche X, 4 entités »
- `qgis-plugin/tests/` -- `test_analysis.py` (`SumLineLengthsTest` pour la reprojection, `IntersectTest` pour le patron), `test_summary.py`, `test_result_conformance.py`, `table_annulable.json`, nouveau `probe_area.py` (modèle `probe_dissolve.py`)
- `ai/tests/par_commande/phrases.json`, `training/src/barres.rs` (compte de phrases, à recalculer après 10.21) -- phrase « Les parcelles de plus de 2 500 m² en zone U » : `select_by_attribute typezone eq U`, `area unit m2`, `select_by_attribute area_m2 gt 2500` avec `step:k.layer_id`
- `README.md`, `qgis-plugin/README.md`
- À ne pas toucher : `analysis_select_by_attribute.rs`, `work_crs.rs`, `families.rs`, `summary_row.rs`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_area.rs`, `mod.rs` -- commande, schéma, deux exemples, tests (déléguée, unité `m2`/`ha` seulement, `work_epsg` déclaré, sortie `.gpkg`) -- contrat du registre
- [ ] `core/src/journal/tests.rs`, `fixtures/resultats-extension.json` -- bras simulé, rejeu, exemple de résultat
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` -- chaîne reprojection puis `native:fieldcalculator`, formule fixe, polygones exigés, résumé FR/EN
- [ ] `qgis-plugin/tests/probe_area.py` -- sonde QGIS 3.44 : carré de 100 m (10 000 m², 1 ha), couche en degrés, multiparties, champ déjà présent, nuls ; constats assertés et consignés
- [ ] `qgis-plugin/tests/` unitaires, conformité, `table_annulable.json` -- refus de géométrie, unités, reprojection
- [ ] Catalogue, `phrases.json`, `barres.rs`, READMEs -- fiche `faite`, phrase « plus de 2 500 m² en zone U »

**Acceptance Criteria:**
- Given un carré de 100 m de côté en Lambert-93, when `analysis.area` s'exécute en `m2` puis en `ha`, then les champs valent 10 000 et 1, et le résultat nomme `area_m2`/`area_ha` et son unité.
- Given une couche en degrés, when elle s'exécute, then la surface est calculée dans le système en mètres choisi par le noyau, sans refus.
- Given « les parcelles de plus de 2 500 m² en zone U » et l'inventaire de test, when l'agent compose, then le plan est valide et seuil et champ sont dans la même unité.
- Given un ancien journal, when on le rejoue, then le résultat est inchangé ; la commande n'est annoncée que si sa chaîne existe (10.4).

## Implementation Notes

Constats de `probe_area.py` sous QGIS 3.44.15 (VERDICT : OK, tous assertés) : carré de 100 m = 10 000 m² et 1 ha ; carré de 40 m = 1 600 m² et 0,16 ha (`FIELD_PRECISION` 0 n'arrondit pas) ; couche en WGS84 avec `work_epsg` 2154 : environ 10 000, sortie en EPSG:2154 ; multipartie = somme de ses parties ; géométrie nulle : surface nulle, entité gardée ; `area_m2` existant remplacé sur place (un seul champ) ; `area_ha` s'ajoute à côté sans toucher `area_m2` ; `area` puis `area_m2 gt 2500` ne garde que les grandes ; lignes et points refusés par `analysis.bad_params` avant tout fichier. Le garde-fou d'inventaire (`attribute_guard.rs`) saute les couches citées par `step:k` : `area_m2` après une étape n'est pas refusé, aucune question à poser. Fiche du catalogue : `work_epsg` ajouté (le test du catalogue le réclamait). Phrase ARE1 en quatre appels (sélection zone U, surface des parcelles, seuil, intersection avec la zone U) : la version en trois appels mesurait les zones et non les parcelles ; ARE2 et ARE3 ajoutées (trois phrases par commande exigées).

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : aucun constat réel retenu à corriger dans le code, 1 trace ajoutée, 4 rejetés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| low | patch | Critère 3 sans trace et Implementation Notes vide : notes écrites ci-dessus (garde-fou, sonde, catalogue) |
| low | rejeté | ARE1 en quatre appels dépasse le plan : sections non gelées du plan (Code Map) ; la phrase en trois appels ne répondait pas à la demande, le critère gelé « parcelles de plus de 2 500 m² en zone U » exige l'intersection |
| false | rejeté | Précision décimale pouvant tronquer les hectares : sonde ajoutée, 1 600 m² = 0,16 ha |
| low | rejeté | `translate_area` sans garde sur `unit` : le noyau valide l'énumération avant tout geste (AD-22), la branche serait inatteignable |
| low | rejeté | Test de la formule par retrait de `area($geometry)` plutôt qu'égalité, et pas de cas non-degré avec `work_epsg` côté extension : couverts par la sonde et le test de journal |

## Design Notes

Le champ à nom fixe règle la note du ticket : la sélection numérique n'a pas d'unité, mais le nom `area_ha` ou `area_m2` que le modèle écrit lui-même fixe celle de la comparaison. La ligne « couche en degrés refusée » de la vérification du ticket est périmée : elle précède la décision du 2026-11-12, qui l'emporte. `area($geometry)` est plane : la valeur ne dépend pas de l'ellipsoïde du projet, donc le rejeu est stable (AD-11).

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_area.py` -- expected: VERDICT OK
