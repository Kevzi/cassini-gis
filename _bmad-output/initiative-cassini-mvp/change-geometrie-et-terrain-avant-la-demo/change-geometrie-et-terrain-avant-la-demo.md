---
type: change
title: "Géométrie et terrain étendu avant la démo, catalogue des commandes figé"
date: 2026-11-05
status: approuvé (propriétaire, 2026-11-05)
scope: major
---

# Changement de cap — géométrie et terrain étendu avant la démo

## 1. Résumé

**Déclencheur.** Décision du propriétaire, 2026-11-05 : les fonctions géospatiales
générales qui manquent encore au noyau — coordonnées, distances et azimuts, terrain
au-delà des trois commandes du MVP, options de mise en page — sont ajoutées **avant la
démo (epic 6)**, et leurs contrats (identifiant, paramètres, résultat) sont **figés en
une seule fois**, pour ne plus rouvrir le registre à chaque besoin découvert.

**Problème.** Le registre s'est construit commande par commande, au fil des besoins.
Chaque ajout ou renommage oblige à reprendre le générateur d'exemples et à ré-entraîner
le modèle de composition. Tant que la liste des commandes n'est pas fixée, ce travail se
répète. Plusieurs demandes courantes restent aujourd'hui sans réponse : convertir une
coordonnée MGRS ou UTM, mesurer une distance et un azimut, placer un point à un azimut
et une distance, imprimer en A3 avec un carroyage, trouver un chemin hors réseau, croiser
plusieurs critères, repérer les points hauts, calculer la visibilité depuis plusieurs
points.

**Type.** Nouvelle exigence du propriétaire, avec un effet de périmètre sur le MVP.

**Test appliqué à chaque fonction** (CLAUDE.md §2.9) : une fonction n'entre dans ce
dépôt que si un bureau d'études, une collectivité, un service de secours ou un étudiant
l'utiliserait telle quelle. Toutes celles de ce changement le passent : la grille MGRS a
un équivalent civil identique (USNG) employé par les secours ; distance, azimut et
recoupement sont de la topographie ; le champ de vision cumulé sert l'éolien et les
antennes ; la surface de coût sert les corridors écologiques ; le multicritère sert le
choix de sites. **Les paramètres propres à un usage particulier** (classes de véhicules,
seuils métier, choix d'un emplacement) **ne sont pas portés par le noyau** : il fournit
les briques, les recettes se composent au-dessus.

## 2. Analyse d'impact

### Epics

| Epic | Effet |
|---|---|
| **11 (nouveau) — Géométrie et terrain étendu** | Créé, placé **avant l'epic 6**. Porte le catalogue figé et les commandes du noyau listées en §4.1. |
| 6 — Démo | Inchangé dans son contenu ; attend désormais l'epic 11. Sa démo peut montrer une commande de l'epic 11 en plus de la pente, sans obligation. |
| 10 — Catalogue étendu | Reste après la démo. Ses contrats sont recopiés dans le catalogue figé dès maintenant (renommage 10.2 compris). Deux primitives déléguées s'y ajoutent : `raster.polygonize` et `raster.align`. |
| 9 — Façade QGIS | Aucune entrée rouverte ; l'option de format et de carroyage de `layout.export_pdf` est portée par l'epic 11 (côté extension : la table fermée et la mise en page). |
| 4 — Terrain | Clos tel quel ; l'epic 11 prolonge son module raster (AD-20). |
| 5 — Agent | Le modèle de composition doit apprendre les nouvelles commandes : un ré-entraînement groupé, en fin d'epic 11, sur la chaîne de 5.10. |
| 7 — Publication | Décalé d'autant. |

### Documents en conflit

- **CLAUDE.md** §1 et §6 (« trois commandes et pas neuf », six commandes de terrain en
  V2), §7 (étapes).
- **Spec** : CAP-11 (« reste limitée à ses trois commandes »), Constraints (statistiques
  zonales), Non-goals (« MGRS au MVP », « six commandes de terrain hors MVP »),
  Assumptions (ordre de livraison).
- **Architecture** : AD-19 (le noyau ne reprojette pas) ; AD-8 (points en nombre borné) ;
  AD-20 (le module raster ne lit que le MNT) ; aucun AD ne dit comment le noyau rend un
  résultat vecteur.
- **Initiative** : `covers`, Notes, ordre des epics.

### Effet technique

- Nouveau code dans `core` : un module de géodésie et de coordonnées, des commandes de
  terrain, un écrivain GeoJSON (sans nouvelle dépendance : `serde_json` suffit).
- Dépendances candidates, **à soumettre avec leur licence avant tout ajout** : une
  bibliothèque de géodésie sous MIT ou Apache-2.0 (par exemple un portage Rust de
  GeographicLib). MGRS s'écrit d'après la spécification publique de la NGA, sans
  reprendre le code d'une extension sous GPL.
- Côté extension : deux paramètres de plus sur la mise en page (format, carroyage).
- Tests : jeux de référence publics (jeu d'essai géodésique de GeographicLib, exemples
  de la NGA), fixtures synthétiques pour le terrain.

## 3. Voie retenue

**Ajustement direct, avec révision du périmètre du MVP.** Ni retour arrière (rien de fait
n'est à défaire), ni réduction.

Le point clé : **ce qui doit être figé, c'est le contrat, pas le code.** Un document
unique, le catalogue des commandes, donne pour chaque commande — existante, de l'epic 11,
de l'epic 10, ou réservée — son identifiant, ses paramètres et son résultat. Le générateur
d'exemples s'aligne sur lui une fois. Les implémentations suivent dans l'ordre des epics,
sans changer de nom (décision du 2026-11-04 : un contrat typé stable, qui permet de
remplacer une délégation par une implémentation du noyau sans toucher à l'entraînement).

Pourquoi l'epic 10 reste après la démo : ses primitives passent par une mise à niveau
préalable (entrées 10.1 à 10.6, gate de revue E1), et ses 26 entrées repousseraient la
démo de loin. Figer ses contrats suffit à ne plus rien re-modifier.

**Effort** : élevé (environ 14 entrées pour l'epic 11). **Risque** : moyen — calculs
géodésiques à valider sur jeux de référence ; nouveau mode de sortie vecteur du noyau.
**Délai** : la démo recule de la durée de l'epic 11.

**Reporté après la démo** (identifiants réservés, paramètres écrits plus tard) :
couverture radio d'un émetteur, hydrologie (sens d'écoulement, accumulation, bassin
versant, zone inondable).

## 4. Propositions détaillées

### 4.1 Catalogue figé — nouvelles commandes

Noms provisoires jusqu'à l'entrée 1 de l'epic 11, qui les fige ; ensuite, un changement
de nom ne passe que par un nouveau changement de cap.

**Noyau (Rust, testées sans QGIS)**

| Identifiant | Paramètres principaux | Résultat |
|---|---|---|
| `coord.convert` | point(s) bornés, système source et cible parmi WGS84, UTM (zone), MGRS ; précision MGRS | coordonnées dans chaque forme demandée |
| `geo.distance_azimuth` | deux points avec EPSG explicite ; unité d'angle (degrés, grades) | distance géodésique en mètres, azimut, contre-azimut |
| `geo.destination_point` | point de départ, azimut, distance | point d'arrivée |
| `geo.bearing_intersection` | deux points et leurs azimuts | point d'intersection (recoupement), ou erreur si parallèles |
| `terrain.aspect` | MNT | raster d'exposition en degrés |
| `terrain.hillshade` | MNT, azimut et hauteur du soleil, exagération | raster d'ombrage |
| `terrain.viewshed_cumulative` | MNT, observateurs (liste bornée), hauteurs, rayon | raster du nombre d'observateurs qui voient chaque cellule |
| `terrain.peaks` | MNT, proéminence minimale, écart minimal, nombre maximal | fichier GeoJSON de points (AD à écrire), liste bornée dans le résultat |
| `terrain.cost_distance` | raster de coût, source(s) bornées | raster de coût cumulé |
| `terrain.least_cost_path` | raster de coût, départ, arrivée | fichier GeoJSON de la ligne, coût total |
| `raster.weighted_overlay` | rasters alignés et poids, masques d'exclusion facultatifs | raster de score |

**Délégué à QGIS — mise en page (epic 11)**

| Identifiant | Ajout | Note |
|---|---|---|
| `layout.export_pdf` | `page_size` (A4 à A0), `orientation`, `grid` (aucun, degrés, projeté avec intervalle, MGRS si QGIS le permet) | Le carroyage MGRS de la mise en page de QGIS est **à vérifier dans sa documentation** avant d'être promis ; sinon, carroyage UTM. Pas de nombre d'exemplaires : l'impression reste hors du produit. |

**Délégué à QGIS — ajoutés à l'epic 10, après la démo**

| Identifiant | Algorithme visé (à vérifier) |
|---|---|
| `raster.polygonize` | `gdal:polygonize` |
| `raster.align` | `native:alignrasters` |

**Déjà au plan de l'epic 10, recopiés tels quels dans le catalogue** :
`analysis.count_points_in_polygon` (renommage 10.2), `analysis.area`, `analysis.difference`,
`analysis.join_table`, `analysis.dissolve`, `data.reproject`, `data.convert`, plus proche
voisin et matrice, statistiques par catégorie, `raster.zonal_statistics`,
`raster.reclassify`, `raster.contour`, `network.shortest_path`, `network.prepare`,
`layout.export_atlas`, géocodage — identifiants exacts repris de
`epic-catalogue-etendu-qgis/tickets.toml`.

**Réservés, après la démo** : `terrain.flow_accumulation`, `terrain.watershed`,
`terrain.flood_extent`, `terrain.radio_coverage`.

### 4.2 CLAUDE.md

§6, point 5 —
OLD : « **Analyse de terrain**, trois commandes et pas neuf : champ de vision, ligne de vue
entre deux points, pente. »
NEW : « **Analyse de terrain** : champ de vision, ligne de vue, pente (epic 4), puis
exposition, ombrage, champ de vision cumulé, points hauts, surface de coût et chemin de
moindre coût, superposition pondérée (epic 11). **Géométrie** : conversion de coordonnées
WGS84, UTM et MGRS, distance et azimut, point par azimut et distance, recoupement
(epic 11). En Rust, sans aucune dépendance GPL ou LGPL. »

§6, hors périmètre — OLD : « les six autres commandes de terrain (…) » → NEW : « hydrologie
et couverture radio (après la démo, identifiants réservés) ; les paramètres propres à un
usage particulier (classes de véhicules, seuils métier), que le noyau ne porte pas ».

§7 — ajouter, entre l'étape 6 et l'étape 7 : « **Étape 6 bis** : géométrie et terrain
étendu (epic 11), catalogue des commandes figé. »

§1 — inchangé (« le fossé est l'agent hors ligne, pas la géométrie » reste vrai).

### 4.3 Spec

- **Nouvelle CAP-15 — Géométrie et terrain étendu.** *intent* : l'utilisateur convertit et
  mesure des coordonnées, et interroge le relief au-delà des trois commandes de CAP-11.
  *success* : chaque commande de §4.1 (noyau) s'exécute dans le noyau Rust, hors ligne,
  sans GPL ni LGPL, testée sur jeux de référence publics ou fixtures synthétiques ; le
  modèle de composition les enchaîne dans un plan valide ; la recette hors ligne reste
  verte.
- **CAP-5** : ajouter « au format et à l'orientation demandés, avec un carroyage en
  option ».
- **Constraints** : remplacer « CAP-11 reste limitée à ses trois commandes de terrain »
  par « CAP-11 garde ses trois commandes ; les autres calculs de terrain relèvent de
  CAP-15 ». Ajouter : « Le catalogue des commandes est figé dans
  `catalogue-des-commandes.md` ; un identifiant ou un paramètre n'y change que par un
  changement de cap. »
- **Non-goals** : retirer « MGRS au MVP » ; remplacer la ligne des six commandes de
  terrain par « hydrologie et couverture radio au MVP ; paramètres de mobilité propres à
  un véhicule ».
- **Assumptions** : ordre de livraison — l'epic 11 s'insère avant la démo.
- **Companions** : ajouter `catalogue-des-commandes.md` (créé par l'entrée 1 de
  l'epic 11).

### 4.4 Architecture (à écrire par `bmad-architecture`)

- **AD-19, amendement** : le noyau convertit des **coordonnées ponctuelles** entre WGS84,
  UTM et MGRS, par des formules publiées, et rien d'autre ; toute reprojection de couche
  et tout autre système restent à QGIS. Le terrain continue d'exiger le système du MNT.
- **AD-20, amendement** : le module raster unique lit aussi les rasters de coût et de
  critères (une bande), pas seulement le MNT ; il refuse deux grilles non alignées au
  lieu de les rééchantillonner (l'alignement est un geste QGIS, `raster.align`).
- **AD-30 (nouveau) — résultat vecteur du noyau** : une commande du noyau qui rend des
  entités écrit un fichier GeoJSON (RFC 7946, EPSG explicite) dans le dossier de sortie
  décidé par le noyau (AD-10) ; QGIS l'ouvre par `layer.add` ; le résultat du canal ne
  porte qu'une liste bornée (AD-8).
- **AD-8** : rappel, les listes d'observateurs et de sources sont bornées par le schéma.

### 4.5 Initiative

- `covers` : ajouter CAP-15.
- Notes : « Decision (2026-11-05, propriétaire) : epic 11, géométrie et terrain étendu,
  avant la démo ; catalogue des commandes figé en une fois ; hydrologie et couverture
  radio réservées après la démo. »
- L'epic 6 ajoute « Waits on epic 11 ».

### 4.6 Epic 10

- Notes : « Les contrats de cet epic sont recopiés dans le catalogue figé le 2026-11-05 ;
  un changement d'identifiant ou de paramètre passe par un changement de cap. »
- Deux entrées ajoutées dans la voie des primitives : `raster.polygonize` et
  `raster.align`, après `raster.reclassify` (25).

## 5. Passage de relais

**Portée : majeure** — spec, architecture et CLAUDE.md changent.

| Qui | Quoi |
|---|---|
| Propriétaire | Approuver ce document ; approuver chaque dépendance proposée avec sa licence. |
| `bmad-architecture` | Écrire AD-30 et les amendements d'AD-19, AD-20 (§4.4). |
| Claude (édition directe, après approbation) | CLAUDE.md, spec, initiative, notes de l'epic 10 et de l'epic 6 (§4.2, §4.3, §4.5, §4.6). |
| `bmad-ticket` | Créer l'epic 11 (enveloppe), puis son inception ; ajouter les deux entrées de l'epic 10. |
| `bmad-build` | Entrée 1 de l'epic 11 : le catalogue des commandes, figé. |

**Ordre** : ce document → architecture → éditions de CLAUDE.md et de la spec → epic 11
(enveloppe, inception, catalogue en entrée 1) → construction.

**Critères de réussite**

1. Le catalogue des commandes existe, liste toutes les commandes (faites, de l'epic 11,
   de l'epic 10, réservées) avec identifiant, paramètres et résultat, et la spec le cite.
2. Les commandes du noyau de §4.1 passent leurs tests sans QGIS, sur jeux de référence.
3. Le modèle de composition, ré-entraîné, les compose dans un plan valide, sans baisse du
   taux de plans conformes.
4. Aucune dépendance GPL ou LGPL dans le noyau ; la recette hors ligne reste verte.
5. La démo (epic 6) démarre ensuite, sans qu'aucun identifiant du catalogue ait changé.
