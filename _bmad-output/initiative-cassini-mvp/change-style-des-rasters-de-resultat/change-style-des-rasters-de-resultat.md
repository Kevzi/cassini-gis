---
type: change
title: "Les rasters de résultat du noyau reçoivent un style dicté après leur ouverture"
date: 2026-11-09
status: approuvé (propriétaire, 2026-11-09)
scope: minor
---

# Changement de cap — style des rasters de résultat

## 1. Résumé

**Déclencheur.** Recette QGIS du 2026-11-09 sur Annecy. Les rasters que le noyau pose (champ de
vision, champ de vision cumulé, pente, exposition, surface de coût, superposition pondérée)
s'ouvrent dans le rendu gris par défaut de QGIS. Les cellules à 0 (non visible, aucune vue)
sont noires et opaques, et cachent le fond de carte. Le noyau dicte `layer.open_raster` après
chaque sortie `.tif`, mais aucun style.

**Type.** Bogue de rendu, réglé par un ordre dicté de plus. Aucune commande nouvelle, aucun
paramètre changé, rien à apprendre au modèle.

**Décision du propriétaire (2026-11-09).** Une nouvelle forme de `StyleSpec`, `result_ramp`,
pour les rasters de résultat :

- `stops` : paliers `{value, color, label?}`, valeurs strictement croissantes ;
- `interpolation` : `discrete` (une couleur par classe) ou `linear` (couleurs interpolées) ;
- `transparent` : valeurs dessinées transparentes, sans-donnée (-9999) compris ;
- `unit` : unité, en texte de légende ;
- `opacity` : de 0 à 1.

Elle va aux rasters seulement (`style.geometry_mismatch` sur un vecteur).

## 2. Ce qui change

**Noyau.** Juste après l'ouverture dictée d'un raster de résultat, la session dicte un
`layer.set_style` sur la couche que l'ouverture rend, de même `origin` et de même
`plan_step`. Le style est composé par le noyau d'après la commande et son résultat :

| Commande | Style |
|---|---|
| `terrain.viewshed` | 0 et -9999 transparents, 1 coloré à demi (opacité 0,5). |
| `terrain.viewshed_cumulative` | 0 et -9999 transparents, dégradé de 1 à `max_count`, de l'orange (5e classe de YlOrRd : la 1re, presque blanche, se perdait sur le fond) au rouge sombre. |
| `terrain.slope` | dégradé du minimum au maximum du résultat, unité `°` ou `%`. |
| `terrain.aspect` | une classe par direction (N, NE, E, SE, S, SO, O, NO, le nord coupé autour de 0 et 360°), couleurs du schéma qualitatif Set1 de ColorBrewer. |
| `terrain.cost_distance` | dégradé de 0 au coût cumulé maximal. |
| `raster.weighted_overlay` | dégradé du score minimal au score maximal. |
| `terrain.hillshade` | le style `hillshade` de la mise en forme topographique. |

Les couleurs sont celles des dégradés ColorBrewer déjà dans la table (`RAMPS`), plus la 5e
classe de YlOrRd et le schéma Set1, tous ColorBrewer (licence Apache-2.0, attribution au
`NOTICE`). `raster.reclassify` n'est pas stylé : ses valeurs de
sortie sont celles de l'utilisateur, sans sens commun.

**Extension.** Le geste `layer.set_style` traduit `result_ramp` en pseudo-couleur de QGIS,
valeurs transparentes comprises. Aucune décision de couleur dans l'extension (AD-4, AD-5).

**Journal et rejeu.** La ligne `layer.set_style` est journalisée et rejouée comme les autres
lignes dictées : `layer_id` substitué par celui de la couche rejouée, style journalisé tel quel.

## 3. Conduites

- Une extension qui n'annonce pas `layer.set_style`, ou qui l'annonce sans annoncer la forme
  `result_ramp` dans `style_kinds` de `session.hello` (extension plus ancienne que ce
  changement) : le raster est ouvert sans style, comme avant, sans erreur (décision du
  propriétaire après relecture, 2026-11-09). Ce n'est pas un geste requis des commandes de
  terrain.
- Un style que le noyau composerait invalide est une erreur nommée du noyau (`style.invalid`),
  avant l'ouverture, jamais un style sauté en silence.
- Un style refusé par l'extension : même conduite qu'un `layer.set_style` refusé de la mise en
  forme, l'erreur de l'appel, journalisée à sa ligne.
- Un rejeu dont l'exécutant n'annonce pas `layer.set_style`, ou pas la forme `result_ramp`,
  s'arrête à la ligne de style sur `style.unavailable`, comme pour la mise en forme.
- Un journal écrit avant ce changement n'a pas de ligne de style : il se rejoue sans style, à
  l'identique (AD-11).

## 4. Documents touchés

- Catalogue des commandes : fiche « Description de style » (forme `result_ramp`, bloc
  ` ```json type `), conventions communes, fiches `layer.open_raster` et `layer.set_style`.
- Recette `core/tests/catalogue.rs` : inchangée, elle confronte le nouveau bloc au schéma.
- Cas partagés `fixtures/style-cases.json` : cas de `result_ramp`, lus par le noyau et
  l'extension.
- `docs/canal-local.md` : champ `style_kinds` de `session.hello`.
- Contrat d'architecture : amendement du 2026-11-09 d'AD-30.

Architecture : ce style dicté par la session après l'ouverture d'un raster, hors du mécanisme
`dictates`, n'était décrit par aucune AD. Il l'est par l'amendement du 2026-11-09 d'AD-30
(décision du propriétaire), qui en fixe la condition (`layer.set_style` et `result_ramp`
annoncés). AD-4, AD-22 et AD-25 sont tenus.
