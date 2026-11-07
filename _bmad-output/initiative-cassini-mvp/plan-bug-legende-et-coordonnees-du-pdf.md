---
title: 'Export PDF : coordonnées du carroyage visibles et légende qui ne déborde plus'
type: 'bugfix'
ticket: ''
created: '2026-11-07'
status: 'built'
baseline_revision: '91ad2e818b9d0b1d61b137e1a48e8c3a46af6eed'
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

**Problem:** Sur le projet d'Annecy (12 couches, dont un ombrage en mode de fusion « produit »), l'export PDF (`layout.export_pdf`) pose un carroyage sans aucune coordonnée, et la légende, longue, recouvre la flèche du nord. La cause du premier défaut : dès qu'une couche a un mode de fusion, QGIS rastérise toute la carte et coupe ce qui dépasse du cadre, donc les étiquettes de la grille (vérifié le 2026-11-07 : étiquettes présentes avec le MNT seul, absentes avec l'ombrage).

**Approach:** Dans `qgis-plugin/cassini_qgis/layout_export.py`, le carroyage est porté par une seconde carte, sans couche ni fond, posée exactement sur la première (même système, même emprise) : QGIS la garde en vectoriel. Ses étiquettes restent à gauche et en bas, mais un seul axe par bord (les Y à gauche, les X en bas), pour ne plus superposer deux nombres dans les coins d'une grille inclinée. La flèche du nord passe dans le coin haut droit de la carte. La légende garde toute la colonne de droite, et passe en deux puis trois colonnes si sa hauteur l'exige. Ni le contrat de la commande ni son résultat ne changent.

</frozen-after-approval>

## Implementation Notes

Route courte : une fonction de mise en page modifiée, une quarantaine de lignes. Vérification par la sonde `probe_layout_export.py` sous QGIS et par un rendu PNG du PDF d'Annecy (voir la mémoire verifier-le-pdf-a-l-oeil).

Preuve de la cause (2026-11-07, `%TEMP%\sonde-11-18\grille.py`) : même mise en page, même grille. Avec `communes` et `mnt_25m`, les étiquettes sont présentes ; avec `communes` et `relief_ombre`, en mode « produit », il n'y en a plus aucune.

Fait (`qgis-plugin/cassini_qgis/layout_export.py`) :
- seconde carte `grid_map`, sans fond, avec une couche mémoire vide (une liste vide voudrait dire « toutes les couches »), même système et même emprise ;
- Y à gauche et X en bas ;
- flèche à (193, 24, 15 × 15) sur un fond blanc ;
- `fit_legend` : 1, 2 puis 3 colonnes, et textes à 100, 85 puis 70 %. Si rien ne tient, une seule colonne au plus petit. La réduction des textes va au-delà de l'Intent, qui ne parlait que des colonnes ; je la garde parce que trois colonnes ne tiennent pas en 72 mm.

Sonde `probe_layout_export.py` : elle vérifie en plus la carte du quadrillage (sans fond, couche vide, Y à gauche, X en bas).

Vérifié : 436 tests de l'extension verts ; sonde QGIS 3.44 OK, 6 cas sur 6 ; rendus PNG du PDF d'Annecy (A3, UTM 32N à 10 km : coordonnées lisibles, légende dégagée) et de l'A3 de Villeneuve.

## Verification

**Commands:**
- tests de l'extension (`python -m unittest discover qgis-plugin/tests`) -- expected: verts
- `probe_layout_export.py` sous le Python de QGIS -- expected: tous les cas OK

**Manual checks:**
- Le PDF A3 d'Annecy en UTM 32N à 10 km, rendu en PNG, montre les coordonnées à gauche et en bas, et une légende qui ne touche pas la flèche du nord.

Ajout à la demande du propriétaire (2026-11-07), après la relecture : les textes de la carte (titre, légende, coordonnées, échelle, sources) sont en Helvetica (`MAP_FONT`) quand elle est installée, sinon dans la police par défaut. Aucune police n'est livrée avec Cassini (Helvetica est commerciale, achetée par le propriétaire et installée pour son compte Windows). Rendu vérifié ; sonde OK.

## Review Triage Log

- medium, patch : `fit_legend` retombait sur 3 colonnes, la mise en page la plus large, et pouvait sortir de la page à droite. Elle retombe maintenant sur 1 colonne au plus petit.
- low, patch : le commentaire du module promettait un PDF inchangé sans les nouveaux paramètres ; il est réécrit.
- low, patch : la couche vide prenait le système du projet (`crs=` vide pour un système sans code) ; elle est maintenant en « Point » sans système.
- medium, patch : la sonde ne vérifiait pas la carte du quadrillage ; elle contrôle maintenant son fond, sa couche vide et l'axe de chaque bord.
- low, rejeté : la réduction des textes dépasse l'Intent. Je la garde et je la note : sans elle, une longue légende déborde quand même.
- low, rejeté : `setExtent` sans lien entre les deux cartes. Rien ne déplace `carte` après la pose, et un lien ajouterait une mécanique pour un cas inaccessible.
- false : cause non prouvée. L'essai `grille.py` est consigné ci-dessus, et le rendu d'Annecy montre la seconde carte en vectoriel avec ses étiquettes.
