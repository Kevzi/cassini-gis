---
title: 'Exemples corrigés et second essai du ré-entraînement terrain'
type: 'feature'
ticket: ''
created: '2026-11-08'
status: 'built'
baseline_revision: '2c0012673d9f85c786f1f9ceb8fb6c84e2365ee0'
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

**Problem:** Le premier ré-entraînement terrain (terrain-11-18, `plan-points-d-un-autre-systeme-pour-le-terrain.md`) manque la barre `plan_reel` (22/25 < 23 : S24 nouveau, S18, S20) et perd C02 et K02 d'`epic11_reel`, qui citent le fichier non ouvert `mnt_zone_nord.tif` au lieu du MNT ouvert. Sur Annecy, le modèle cite l'identifiant de la couche du MNT au lieu de son fichier (`dem.unreadable`). Le propriétaire approuve le second et dernier essai permis par la règle de 11.16 (2026-11-08).

**Approach:** Dans le générateur (`training/src/inventories.rs`, `training/src/families.rs`) :
1. Les paires de MNT prennent des noms variés, dont des noms à l'allure de fichier (« mnt_5m_vallee »), un identifiant fait du nom comme dans QGIS, et un fichier d'un autre vocabulaire (`rgealti_5m.tif`, `mnt-bassin-5m.tif`…) : le plan cite toujours la `source`.
2. Les inventaires de terrain portent parfois un fichier MNT non ouvert dans `unloaded_files` ; la phrase ne désigne aucun MNT, et le plan cite le MNT ouvert (l'attendu de C02 et K02).
3. Le refus « calcul de terrain puis statistique par zone » reçoit la forme de S24 : point écrit, hauteur, rayon, puis « sa moyenne par commune ».

Puis tests du générateur, entraînement par la commande unique vers `~/cassini-entrainement/terrain-11-18b`, validation et essais réels sur GPU. Installation seulement sur décision du propriétaire, si les barres de 11.16 sont tenues.

</frozen-after-approval>

## Implementation Notes

Route courte : trois retouches du générateur, moins de 100 lignes. Le fichier non ouvert vient de `Builder::add_dem`, donc de toutes les familles de terrain. Ses noms ne reprennent pas `mnt_zone_nord`, le fichier de l'essai.

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: vert
- `cargo fmt --check`, `cargo clippy -p cassini-training --all-targets -- -D warnings` -- expected: propres

## Review Triage Log

- low, patch : la doc de `Builder::finish` disait `unloaded_files` toujours vide hors `add_file` ; elle nomme maintenant le MNT non ouvert d'`add_dem`.
- false : seuil de 90 % d'inventaires sans fichier (`forme_inventaire.rs`) menacé. L'essai passe avec le changement (environ 3 % de plus).
- medium, patch : `rgealti_…` et `…_l93` pouvaient nommer un MNT en UTM, contre la règle d'`add_dem`. Le vocabulaire dépend maintenant du système (`terrain_…`, `elevation_…` en UTM).
- low, patch : l'essai du MNT non ouvert ne vérifiait pas que la phrase ne le nomme pas ; il le vérifie. L'`assert_ne!` identifiant/fichier, toujours vrai, est retiré.
- medium, patch : la forme S24 figeait « (Lambert-93) » et les séparateurs de milliers, trop près de la phrase d'essai. Elle n'est plus qu'une forme sur trois, avec système et chiffres variés.
