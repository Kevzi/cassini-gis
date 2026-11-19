---
title: 'Isochrone sur un réseau en degrés : calcul en mètres dans un système choisi par le noyau'
type: 'feature'
ticket: '32'
created: '2026-11-19'
status: 'built'
baseline_revision: 'd8f814d643dcf8f53bffb713ee6f0e5932e8a9b4'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick, edge-case]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-distance-en-metres-sur-couche-en-degres/change-distance-en-metres-sur-couche-en-degres.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `analysis.isochrone` lit les longueurs du réseau et accroche l'origine dans l'unité de la couche : sur un réseau en EPSG:4326, durée, vitesse et tolérance d'accrochage sont faux (degrés pris pour des mètres).

**Approach:** Appliquer la règle de 9.16 : le noyau remplit `work_epsg` (via `work_crs_param`), l'extension place `native:reprojectlayer` en tête de chaîne quand `work_epsg` est donné, transforme l'origine vers `work_epsg`, vérifie l'accrochage en mètres ; version de chaîne de la ligne portée à 2 ; fiche du catalogue mise à jour.

## Boundaries & Constraints

**Always:** `work_epsg` rempli par le noyau seul, retiré des paramètres fournis par l'appelant, journalisé comme paramètre ordinaire et masqué au modèle, au MCP et au formulaire (`without_core_filled`). Réseau déjà métrique : `work_epsg` absent, étape de reprojection ignorée, chaîne inchangée. Rejeu d'une ligne de journal sans `work_epsg` = ancien comportement. Algorithmes `native:` seulement, aucun nom d'algorithme dans le noyau (AD-22), le geste ne choisit jamais un système (AD-5/AD-24), les valeurs par défaut de `NetworkOptions` restent résolues par le noyau (AD-28).

**Never:** changer l'identifiant, les champs du résultat (`layer_id`, `name`, `feature_count`, `crs`), le schéma `out`, les codes `analysis.origin_off_network` et `analysis.crs_not_metric` ; toucher `.claude/worktrees/*` ; utiliser GRASS ; réécrire d'anciennes lignes de journal.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Réseau en degrés | couche EPSG:4326, projet en Lambert-93 | `work_epsg=2154` rempli, isochrone de même emprise (à quelques mètres) que sur le réseau reprojeté à la main | No error expected |
| Réseau métrique | couche EPSG:2154 | pas de `work_epsg`, chaîne inchangée | No error expected |
| Origine hors réseau | origine à plus de `network.tolerance` mètres | refus avant exécution | `analysis.origin_off_network` |
| Degrés, aucun système métrique | projet en degrés ou Mercator global, étendue sans zone UTM | refus avant tout | `analysis.crs_not_metric` |
| Ancien journal | ligne sans `work_epsg` | rejouée à l'identique | No error expected |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_isochrone.rs` -- params l.59-79, spec l.102-127 ; ajouter `work_epsg: Option<u32>` et `.work_crs_param("work_epsg","layer_id")`, test `analysis_isochrone_declares_its_work_crs` (modèle : `analysis_buffer.rs` l.73/108, `analysis_sum_line_lengths.rs`).
- `core/src/work_crs.rs`, `core/src/journal/mod.rs` l.465/681-703, `core/src/serve.rs` l.1005 -- mécanisme existant, à réutiliser tel quel.
- `qgis-plugin/cassini_qgis/analysis.py` -- ligne `analysis.isochrone` l.700-722 (`"work_crs": WORK_EPSG`, `_REPROJECT_TO_WORK`, `chain_version` 2) ; `translate_isochrone` l.184 (`START_POINT` vers `work_epsg`) ; `_check_origin` l.~1440 et `backend.has_feature_near` l.1001-1022 (rayon en unités de la couche : faux en degrés) ; garde d'entrée l.1294-1298.
- `qgis-plugin/tests/test_analysis.py` l.~875-1000 (asserte `near_call == (...,100.0)` et `START_POINT`), `test_chain.py` l.153-158 (asserte `chain_version == 1` pour toutes les lignes : à adapter), `probe_sum_line_lengths.py` / `probe_buffer_degrees.py` (modèle de sonde), `probe_isochrone.py` (EPSG:2154 seulement).
- `core/tests/catalogue.rs` l.1208-1230 ; `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` l.656-682 (`### analysis.isochrone`, bloc `json fiche` l.673) : ajouter `work_epsg` « fourni par le noyau » comme pour les autres fiches.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_isochrone.rs` -- champ `work_epsg` + `work_crs_param` + test de déclaration -- le noyau choisit le système métrique
- [ ] `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- fiche mise à jour -- `catalogue.rs` compare fiche et registre
- [ ] `core/src/journal/tests.rs` -- remplissage de `work_epsg` pour `analysis.isochrone` sur couche en degrés, absent en métrique, rejeu d'une ligne sans lui -- règle de rejeu
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- reprojection en tête, origine transformée, accrochage en mètres, `chain_version` 2 -- longueurs en mètres
- [ ] `qgis-plugin/tests/test_analysis.py`, `test_chain.py` -- adapter les assertions, ajouter les cas degrés -- couverture
- [ ] `qgis-plugin/tests/probe_isochrone_degrees.py` -- sonde sous QGIS 3.44 : emprise 4326 vs réseau reprojeté à la main, à quelques mètres près -- preuve réelle

**Acceptance Criteria:**
- Given un réseau de test en EPSG:4326, when l'isochrone est exécutée, then son emprise égale celle du même réseau reprojeté à la main à quelques mètres près.
- Given un réseau métrique, when l'isochrone est exécutée, then le résultat et les paramètres journalisés sont inchangés.
- Given l'annonce `session.hello`, then `analysis.isochrone` annonce la version de chaîne 2 et les autres lignes restent à 1.
- Given `cargo test`, `cargo clippy`, les tests Python de l'extension et `core/tests/catalogue.rs`, then tout est vert.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (lentilles quick + edge-case) : 0 high, 0 medium, 3 low, 1 false, 0 maybe-false.

| Verdict | Route | Constat et preuve |
|---|---|---|
| low | patch (fait) | AC 3 : aucun test de l'annonce de version 2 pour `analysis.isochrone` ; test ajouté dans `test_chain.py` (`AnnouncementTest`), vert. |
| low | rejeté | `geometry.transform` / `QgsCsException` non vérifiés dans `has_feature_near` et `transform_point` : la branche métrique d'origine a le même comportement, cas hors zone très improbable, la correction ajoute des gardes. |
| low | rejeté | Avertissement de rejeu « chaîne en version 2, journal en version 1 » sur d'anciennes lignes métriques : c'est la montée de version voulue par le ticket (« version de chaîne augmentée »), un avertissement, pas un échec ; le bilan reste identique. |
| low | rejeté | Message de `transform_point` qui parle de l'origine pour un SCR cible invalide : masqué par `_check_work_crs`, qui passe avant. |
| false | rejeté | « `project.inventory` lu pour chaque isochrone » : voulu, c'est le mécanisme `work_crs_param` de 9.16 ; `IsochroneQgis` répond déjà à l'inventaire. |
| false | rejeté | Double traduction de l'étape isochrone : `_step_parameters` est le point d'entrée unique, la seconde passe sur la couche reprojetée est voulue. |
| false | rejeté | Écart d'accrochage entre pré-contrôle et `serviceareafrompoint` : sonde réelle à 0,000 m d'écart, refus à 200 m/100 m et accord à 250 m. |

## Design Notes

Le ticket citait `_ORIGIN_SNAP_METERS` (100 m) : cette constante n'existe plus, l'accrochage vient de `network.tolerance` (`_network_tolerance_meters`). Le contrôle d'accrochage doit se faire en mètres : transformer origine et géométries vers `work_epsg` (ou tester sur la couche reprojetée), jamais un rayon en mètres sur une couche en degrés. Aucun précédent de montée de version de chaîne : ajouter l'attente par ligne dans `test_chain.py` plutôt qu'un `== 1` global.

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: vert, dont `catalogue.rs`
- `cargo clippy --all-targets` -- expected: aucun avertissement
- `python-qgis-ltr.bat qgis-plugin/tests/probe_isochrone_degrees.py` (QGIS 3.44.15) -- expected: VERDICT OK
