---
type: epic
title: "Geometrie et terrain etendu, catalogue des commandes fige"
parent: initiative-cassini-mvp
covers: [CAP-15, CAP-5, CAP-6]
after: []
assignee: ""
risk: high
---

# Géométrie et terrain étendu, catalogue des commandes figé

## Description

Le noyau apprend les calculs de géométrie et de terrain qui manquent encore aux
demandes courantes : convertir une coordonnée entre WGS84, UTM et MGRS, mesurer une
distance et un azimut, placer un point à un azimut et une distance, recouper deux
visées, calculer exposition, ombrage, champ de vision cumulé, points hauts, coût de
déplacement et chemin de moindre coût, reclasser et superposer des rasters ; la mise
en page sort au format, à l'orientation et avec le carroyage demandés. Avant tout
cela, le catalogue des commandes est écrit et figé : chaque commande, faite,
planifiée ou réservée, y a son identifiant, ses paramètres et son résultat, et une
recette le confronte au registre. Le modèle de composition apprend ensuite les
nouvelles commandes en un seul ré-entraînement.

## Outcome

Un utilisateur obtient en français les conversions, mesures et analyses de relief
courantes, hors ligne et sans QGIS côté calcul ; le signal est CAP-15, chaque
commande testée sans QGIS et composée par le modèle dans un plan valide, avec un
catalogue qui ne bouge plus.

## Requirements

Source : la spec, `CAP-15` (géométrie et terrain étendu), `CAP-5` (format,
orientation et carroyage ajoutés le 2026-11-05) et `CAP-6` (registre) ; la liste
des commandes et leurs paramètres provisoires sont au changement de cap, §4.1 ; les
règles sont au contrat d'architecture, AD-6, AD-19, AD-20, AD-23, AD-24, AD-30,
AD-31.

- G1 (CAP-6) : le catalogue des commandes est écrit dans
  `spec-cassini-mvp/catalogue-des-commandes.md`, approuvé par le propriétaire, et
  une recette le confronte au registre (AD-6).
- G2 (CAP-15) : `coord.convert` convertit des coordonnées ponctuelles entre WGS84,
  UTM et MGRS, avec le type `MgrsRef` sous la garde de provenance (AD-19, AD-23).
- G3 (CAP-15) : `geo.distance_azimuth`, `geo.destination_point`,
  `geo.bearing_intersection`, géodésiques, avec le type `Angle` (AD-31).
- G4 (CAP-15) : `data.transform_point`, reprojection d'un point par QGIS (AD-19,
  AD-31).
- G5 (CAP-15) : tout raster se désigne par `RasterPath` ; la table fermée de
  l'extension contient exactement les commandes déléguées (AD-24).
- G6 (CAP-15) : `terrain.aspect` et `terrain.hillshade`.
- G7 (CAP-15) : `terrain.viewshed_cumulative`, observateurs bornés (AD-8).
- G8 (CAP-15) : écriture GeoJSON unique et ouverture dictée `layer.open_vector`
  (AD-30), première utilisée par `terrain.peaks`.
- G9 (CAP-15) : `raster.reclassify` et `raster.weighted_overlay`, grilles
  identiques exigées (AD-20).
- G10 (CAP-15) : `terrain.cost_distance` et `terrain.least_cost_path`, module de
  coût unique (AD-20).
- G11 (CAP-5) : `layout.export_pdf` reçoit `page_size`, `orientation` et `grid`.
- G12 (CAP-15) : le modèle de composition compose les nouvelles commandes dans un
  plan valide, après un ré-entraînement groupé, sans baisse du taux de plans
  conformes.

## Done when

1. Le catalogue des commandes existe, est approuvé, et la recette échoue dès qu'il
   diverge du registre.
2. Chaque commande du noyau de G2, G3 et G6 à G10 passe ses tests sans QGIS, sur
   jeux de référence publics (géodésie, MGRS) ou fixtures synthétiques (terrain).
3. Dans QGIS, sur le poste du propriétaire, une demande en français enchaîne une
   conversion MGRS, une analyse de terrain et un PDF A3 carroyé, par le vrai
   modèle, et se rejoue à l'identique depuis le journal.
4. Le modèle ré-entraîné compose les nouvelles commandes, sans baisse du taux de
   plans conformes sur le jeu de référence.
5. Aucune dépendance GPL ou LGPL dans le noyau ; la recette hors ligne reste verte.

## Boundaries

Frontière par capacité : CAP-15 dans le noyau, plus deux gestes QGIS qui la servent
(`data.transform_point`, options de mise en page). Pas de primitive du cadre de
traitement de QGIS (epic 10), pas d'hydrologie ni de couverture radio (réservées),
pas de paramètre propre à un usage particulier (classes de véhicules, seuils
métier) : le noyau fournit les briques. Aucune parité avec QGIS visée.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, Notes (changement de cap du 2026-11-05)
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-15, CAP-5, CAP-6, Constraints, Non-goals
- change — `../change-geometrie-et-terrain-avant-la-demo/change-geometrie-et-terrain-avant-la-demo.md`, §4.1 (liste des commandes)
- constraint — `../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-6, AD-8, AD-9 (amendement 4.9), AD-10, AD-11, AD-19, AD-20, AD-21, AD-23, AD-24, AD-30, AD-31
- constraint — `CLAUDE.md` §2.2 (aucune GPL dans le noyau), §2.9 (test civil), §8 (licence demandée avant toute dépendance)

## Notes

- Decision (2026-11-05, propriétaire) : epic créé par le changement de cap du jour, placé avant la démo (epic 6) ; le catalogue des commandes est figé en une fois.
- Decision (2026-11-05, propriétaire) : `raster.reclassify` est calculé par le noyau et construit ici, sous l'identifiant prévu à l'epic 10, dont l'entrée 25 est retirée.
- Decision (2026-11-05, propriétaire) : les entités rendues par le noyau sont un GeoJSON 2008 qui déclare son système (AD-30).
- Decision (2026-11-05) : la mise en page de QGIS n'a pas d'étiquettes MGRS ; le carroyage est en degrés ou projeté (UTM au kilomètre, dont les lignes sont celles de MGRS).
- Decision (2026-11-05) : tracer bullet = entrée 2 (`coord.convert` de bout en bout, noyau, canal, panneau, MCP), qui est aussi la moins sûre ; l'entrée 1 (catalogue) la précède parce qu'elle fige les noms et paramètres que toutes les autres implémentent.
- Decision (2026-11-05) : une seule voie, chaque entrée après la précédente (1, 2, 3, 4, 5, 17, 6, 7, 8, 15, 9, 10, 11, 12, 16, puis 13 et 14), parce que presque toutes touchent le registre, les types partagés de `contract.rs` et la table fermée de l'extension ; un seul porteur construit l'epic. Les entrées 5, 8 et 12 ont été coupées en deux (17, 15, 16) pour tenir chacune en une session.
- Open question : l'essai réel d'un GeoJSON 2008 en EPSG:2154 ouvert dans QGIS 3.44 et 4.x (critère d'acceptation de l'entrée qui écrit le premier GeoJSON).
- Unknown : bibliothèque de géodésie (portage Rust de GeographicLib sous MIT, ou formules écrites à la main) — licence soumise au propriétaire avant tout ajout.
- Waits on epic 4 because: le module raster unique et les trois commandes de terrain, que cet epic prolonge.
- Waits on epic 5 because: la chaîne d'entraînement reproductible (5.10) et le générateur d'exemples, pour le ré-entraînement groupé.
- Waits on epic 9 because: la table fermée de l'extension et la mise en page, pour `data.transform_point` et les options d'export.
