---
title: 'Exemples : points d''un autre système pour toutes les commandes de terrain, deux MNT'
type: 'feature'
ticket: ''
created: '2026-11-08'
status: 'built'
baseline_revision: '129226b5dd05fbf4c62c1cf516b0736af7a34c90'
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

**Problem:** Sur Annecy (2026-11-08), « Qu'est-ce qu'on voit depuis le point 45,9075 N, 6,1700 E (WGS84), à 2 m de haut, sur 5 km, sur le MNT à 5 m ? » et la ligne de vue équivalente sont composées sans `data.transform_point`, et le noyau les refuse (`terrain.crs_mismatch`). La famille `terrain_autre_systeme` (11.12) n'a qu'un MNT dans l'inventaire et ne couvre que le champ de vision et la ligne de vue. Le report de 11.16 demande aussi un point d'un autre système pour les nouvelles commandes de terrain.

**Approach:** Dans `training/src/families.rs` et `training/src/inventories.rs` :
- `terrain_autre_systeme` reçoit parfois un second MNT, que la phrase désigne, et les tournures « Qu'est-ce qu'on voit depuis » et « Que voit-on depuis ».
- `chemin_moindre_cout` : départ et arrivée sont parfois écrits dans un autre système, puis convertis par `data.transform_point` et cités par `step:k.point`.
- `champ_de_vision_cumule` et `cout_cumule` : des points dans un autre système donnent une question qui demande les points dans le système du raster. Une référence `step:k` ne vaut qu'au premier niveau d'un paramètre, et aucune étape ne reprojette une liste.

Ensuite, entraînement de nuit par la commande unique, puis validation. L'installation n'a lieu que si les barres de 11.16 sont tenues, sur décision du propriétaire.

</frozen-after-approval>

## Implementation Notes

Route courte, générateur seul. `Builder::add_dem_pair` pose deux MNT de résolutions distinctes (1, 5 ou 25 m), dont le nom, l'identifiant et le fichier portent la résolution. La phrase désigne le MNT visé : « sur le MNT à 5 m », « de 25 m », « avec le MNT à 1 m », ou la couche entre guillemets. `list_system_question` écrit la question des listes. `raster_has_epsg` vérifie le système du raster cité (MNT ou raster de coût). La question est au singulier pour un seul point. Tests mis à jour : la dernière étape porte la commande, les étapes d'avant ne sont que des conversions, et un essai couvre les questions du champ de vision cumulé.

Constat (2026-11-08) : un plan qui mêle un point reprojeté (`step:k.point`) et un point écrit tel quel rend une question au garde-fou AD-23 (`guard_coordinates`). Le générateur n'engendre donc que des exemples où les deux points sont reprojetés. Le cas mixte est reporté.

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: vert
- `cargo fmt --check`, `cargo clippy -p cassini-training --all-targets -- -D warnings` -- expected: propres

## Review Triage Log

- low, patch : un commentaire de documentation était mal placé ; il est remis sur `dem_has_epsg`.
- low, patch : `cost_has_epsg` ne vérifiait pas le raster cité. Il est remplacé par `raster_has_epsg(inventaire, source)`.
- medium, patch : avec deux MNT, le nom et le fichier ne portaient pas une résolution cohérente, et aucune phrase ne disait « le MNT à 5 m » (le cas d'Annecy). Corrigé par `add_dem_pair` et les tournures par résolution.
- medium, patch : le second MNT et les nouvelles tournures n'étaient pas testés. Essai `with_two_dems_the_request_names_the_one_the_plan_cites` ajouté.
- low, patch : la question était au pluriel pour une seule source. Une variante au singulier est ajoutée.
- medium, patch : les questions du coût cumulé n'étaient pas testées (le test « x, y, système » passait par hasard). Essai ajouté : EPSG cité, « X et Y », point d'interrogation.
- medium, patch : `terrain_examples` écartait les questions de toutes les familles. Une question n'est maintenant admise que pour le champ de vision cumulé.
- low, patch : le chemin de moindre coût admettait une seule conversion. Il en exige maintenant deux, le départ d'abord.
