# Digest C, tour 1 : technique QGIS, hydrologie, géocodage (assistant de recherche, 2026-11-04)

Repère : dernière 3.44 = `final-3_44_15` (2026-09-25), même date que `final-4_2_3` ; master en 4.3 (releases GitHub QGIS).

- `QgsNativeAlgorithms::id()` = "native", `helpId()` = "qgis" : d'où l'affichage `qgis:` dans la doc. https://raw.githubusercontent.com/qgis/QGIS/master/src/analysis/processing/qgsnativealgorithms.cpp l. 354-362. high, capability.
- `name()` de difference, clip, union, splitwithlines, enregistrés dans le fournisseur native sur `release-3_40` et `release-3_44` : identifiants canoniques `native:difference`, `native:clip`, `native:union`, `native:splitwithlines`. Fichiers qgsalgorithm{difference,clip,union,splitwithlines}.cpp. high, version.
- Les deux préfixes marchent : `QgsProcessingRegistry::algorithmById` essaie l'id tel quel, puis pour `qgis:` le même nom dans `native` (« try mapping 'qgis' algs to 'native' algs ») ; table d'alias `addAlgorithmAlias` (QgisAlgorithmProvider.py l. 81). https://raw.githubusercontent.com/qgis/QGIS/release-3_44/src/core/processing/qgsprocessingregistry.cpp l. 210-235. high.
- Doc 3.40 affiche `qgis:clip`, `qgis:difference`, `qgis:splitwithlines` mais `native:extractbyextent` sur la même page. high.
- 3.44 : « Port SAGA Fill Sinks Wang & Liu to native tool » (commit 2025-05-07, dans release-3_44, absent de release-3_40) ; « Processing raster rank algorithm ». https://qgis.org/project/visual-changelogs/visualchangelog344/. high.
- 3.42 : « Raster zonal min/max algorithm », « extract min/max pixel from raster », « generate elevation profile images » ; pas d'hydrologie. https://qgis.org/project/visual-changelogs/visualchangelog342/. high.
- `native:fillsinkswangliu` (3.44, groupe Raster terrain analysis) : sorties MNT comblé, directions de flux (0=N…7=NW), bassins versants (`OUTPUT_WATERSHED_BASINS`), paramètre `MIN_SLOPE` ; clone 1:1 de SAGA. release-3_44/src/analysis/processing/qgsalgorithmfillsinkswangliu.cpp. high.
- Master (4.3) seulement, absents de release-4_2 : flowdirection (D8), flowconnectivity, upslopeareafrompoint, upslopeareafromlayer, ordre de Strahler (commits 2026-09-15/18). Pas en 3.44. high.
- Aucun TWI ni accumulation de flux natif en 3.44. medium.
- `whitebox_next_gen` : `MIT OR Apache-2.0` (Cargo.toml du workspace et de `crates/wbtools_oss` v0.1.3, LICENSE-MIT « Copyright 2021-2026 John Lindsay, Whitebox Geospatial Inc. »), dernier push 2026-08-31. https://github.com/jblindsay/whitebox_next_gen. high, license.
- Réserve : README dit que « la majorité » des 700+ outils sont libres dans `wbtools_oss` ; extension payante propriétaire hors workspace ; crate `wblicense_core` ; fichier `stream_network_analysis/pro_stream_tools.rs` dans le crate libre. high.
- `wbtools_oss/src/tools/hydrology/mod.rs` : fill_depressions (+ wang_and_liu, planchon_and_darboux), breach_depressions_least_cost, fill_pits, watershed, basins, subbasins, isobasins, snap_pour_points, jenson_snap_pour_points, strahler_order_basins, hillslopes, depth_in_sink, chaîne « wetness ». medium.
- `flow_algorithms/mod.rs` : d8_pointer, d8_flow_accum, dinf_*, fd8_*, rho8_*, mdinf_flow_accum, qin/quinn_flow_accumulation ; `geomorphometry/hydrologic_index_tools.rs` : relative_stream_power_index, sediment_transport_index. medium.
- BAN téléchargeable : « Format CSV historique », « CSV avec identifiants BAN », « Format Addok ». https://adresse.data.gouv.fr/outils/telechargements. high.
- Licence BAN : Licence Ouverte v2.0 (Etalab 2.0) ; jeu mis à jour le 2026-10-23. https://www.data.gouv.fr/datasets/base-adresse-nationale. high, license.
- Addok (géocodeur local, Python) : MIT, « Copyright (c) 2020 DINUM/Etalab », PyPI 1.3.2 ; addok-france MIT (1.2.0) ; addok-docker dernier push 2022-10. https://github.com/addok/addok. high, license.
- `native:batchnominatimgeocoder` (3.40, 3.44) : point d'accès fixe `https://nominatim.qgis.org/search` (qgsnominatimgeocoder.cpp l. 39) ; inutilisable hors ligne. high.

Interprétation de l'assistant (non constat) : déléguer en `native:*` ; `native:fillsinkswangliu` pour dépressions et bassins sous 3.44 ; accumulation, upslope, TWI : rien en 3.44 ni 4.2, réécriture Rust depuis wbtools_oss ou attente de QGIS 4.4 ; géocodage hors ligne : fichier BAN format Addok + Addok local (dépendance Redis non vérifiée).

Pistes : vérifier que les outils hydrologiques de wbtools_oss ne passent pas par wblicense_core et le contenu de pro_stream_tools.rs ; confirmer un identifiant wetness_index ; GDAL 3.11/3.12 et l'hydrologie ; dépendances d'Addok (Redis), Windows sans Docker ; rétroportage de flowdirection en 3.44.x.
Cherché sans trouver : changelog.qgis.org 3.42/3.44 ; sous-commandes `gdal raster` (page JS) ; recherche de code GitHub (authentification) ; TWI natif.
