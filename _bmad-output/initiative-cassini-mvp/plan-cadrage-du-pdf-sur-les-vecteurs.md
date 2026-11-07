---
title: 'Export PDF cadré sur les couches vectorielles, le relief en fond'
type: 'feature'
ticket: ''
created: '2026-11-07'
status: 'built'
baseline_revision: '5507527c1f65a4513d706e806ba95fad8e6f8753'
route: 'oneshot'
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

**Problem:** Sans échelle demandée, `layout.export_pdf` cadre la carte sur l'union de toutes les couches visibles, MNT compris. Un MNT déborde souvent largement de la zone d'étude : à Villeneuve, il couvre 18,5 × 16,8 km pour des vecteurs d'environ 9 × 9 km, et la zone utile reste petite au milieu de la page. Le propriétaire veut une carte « plus zoomée, parfaitement ».

**Approach:** Dans `LayoutExportGesture._read` (`qgis-plugin/cassini_qgis/layout_export.py`), le cadrage sans `scale` se fait sur l'union des couches vectorielles visibles qui ont une emprise. Les rasters restent sur la carte, en fond. Sans couche vectorielle, on garde l'emprise de toutes les couches, comme avant. Avec `scale`, rien ne change : le centre décide. Le refus « couches vides » et le contrat de la commande ne changent pas. Des tests sans QGIS couvrent les deux cas.

</frozen-after-approval>

## Implementation Notes

Route courte : quelques lignes dans `_read`, plus `is_vector` sur la doublure des tests. `LayoutBackend` hérite déjà de `QgisBackend.is_vector` (`inventory.py`).

Fait : `frame_box` est calculé dans `_read` avant l'export, seulement sans `scale` : c'est l'union des couches `is_vector`, sinon `box`. Les couches « autres » (tuiles vectorielles, maillages) sont exclues exprès, un fond de tuiles couvrant souvent le monde. Deux tests sans QGIS ont été ajoutés (vecteurs et MNT ; MNT seul). La sonde relève maintenant le cadre passé au fond : sur Villeneuve, dans QGIS 3.44, il fait 10,3 × 9,9 km, contre 18,5 km pour le MNT.

Jeux de test, hors du dépôt : Villeneuve mis en forme aux conventions IGN et au PLU CNIG, avec un ombrage ajouté (7 couches) ; sauvegarde dans `_archives/sauvegardes/`. Rendus PNG de Villeneuve (A3, UTM 1 km) et d'Annecy (A3, UTM 10 km) regardés : la zone d'étude remplit la carte.

Vérifié : 438 tests de l'extension verts ; sonde QGIS OK, 6 cas sur 6.

## Review Triage Log

- medium, patch : la sonde ne vérifiait pas le cadrage dans le vrai QGIS. Elle relève maintenant le cadre (moins de 12 km de côté sur Villeneuve).
- medium, patch : `is_vector` était appelé dans le bloc d'export, avec ou sans échelle. Le calcul se fait maintenant avant l'export, seulement sans `scale`.
- false : `is_vector` plutôt que « non raster ». C'est voulu : les couches « autres » (tuiles, maillages) couvrent souvent le monde et gâcheraient le cadrage. C'est écrit dans le code.
- low, rejeté : la doublure décide par l'extension du fichier. Elle ne sert qu'aux tests, et le vrai fond est vérifié par la sonde.
- false : risque sous-évalué. Le rejeu recalcule le cadre à partir des mêmes couches, et ce cas est déjà couvert par les sondes de rejeu de l'export.

## Verification

**Commands:**
- tests de l'extension -- expected: verts, dont les nouveaux
- `probe_layout_export.py` sous QGIS -- expected: OK

**Manual checks:**
- Rendus PNG des PDF de Villeneuve et d'Annecy : la zone d'étude remplit la carte.
