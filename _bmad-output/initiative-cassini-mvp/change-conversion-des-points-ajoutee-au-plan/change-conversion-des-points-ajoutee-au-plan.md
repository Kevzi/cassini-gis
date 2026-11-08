---
type: change
title: "La conversion d'un point vers le système du raster est ajoutée au plan par le noyau"
date: 2026-11-08
status: approuvé (propriétaire, 2026-11-08)
scope: moderate
---

# Changement de cap — conversion des points ajoutée au plan par le noyau

## 1. Résumé

**Déclencheur.** La sonde d'Annecy du 2026-11-08, faite avec le modèle installé ce jour-là (terrain-11-18b) :
- la ligne de vue entre deux points WGS84, sur un MNT en Lambert-93, est composée sans `data.transform_point` ;
- le champ de vision cumulé depuis des points WGS84 l'est aussi ;
- le noyau refuse les deux (`terrain.crs_mismatch`).

Deux ré-entraînements (11.12, 11.16, puis terrain-11-18 et 11-18b) n'ont pas suffi à faire composer cette conversion au modèle de façon fiable. Par ailleurs, le garde-fou AD-23 rend une question quand un plan mêle un point converti (`step:k.point`) et un point écrit tel quel (`deferred-work.md`). Le propriétaire place ce changement en premier, avant le ré-entraînement groupé (2026-11-08).

**Problème.** Le modèle doit aujourd'hui savoir trois choses :
- que le MNT est dans un autre système ;
- qu'il faut convertir chaque point ;
- comment citer le point converti.

C'est une règle mécanique, que le noyau sait appliquer seul : il lit le système du raster lui-même (AD-20). La laisser au modèle produit des refus que l'utilisateur ne comprend pas.

**Type.** Échec d'une approche (l'apprentissage), remplacée par une règle du noyau. Ce n'est pas une nouvelle commande.

**Décisions du propriétaire (2026-11-08) :**
1. **Dans le plan.** Avant de montrer le plan, le noyau y ajoute lui-même une étape `data.transform_point` (exécutée par QGIS) pour chaque point d'une commande du noyau qui n'est pas dans le système de son raster. L'utilisateur la voit et la valide avec le reste. Elle est journalisée et rejouée comme une étape ordinaire. La règle d'AD-19, « jamais en silence », est tenue.
2. **Appel direct inchangé.** Un appel direct (`command.run`, client MCP, CLI `run`) n'a pas de plan. Il reste refusé par `terrain.crs_mismatch`, avec son message, et l'IA externe compose elle-même la conversion.
3. **Entraînement.** Le modèle local apprend à écrire le point tel que la phrase le donne. Le noyau complète le plan. Si le modèle compose déjà la conversion, le noyau n'en ajoute pas une seconde.

## 2. Analyse d'impact

**Architecture.**
- **AD-19.** Son amendement du 2026-11-05 dit qu'une conversion « est une étape visible du plan ». Il reste vrai : l'étape est ajoutée par le noyau au lieu du modèle. Un amendement le précise, et dit qui l'ajoute et quand.
- **AD-23.** Le garde-fou s'applique au plan du modèle, avant la complétion. Les points écrits viennent donc de la demande, et passent comme aujourd'hui. Les étapes ajoutées sont du noyau, et leurs références `step:k.point` sont de sa main. Le cas mixte du report disparaît pour les plans du modèle local, qui n'écrira plus de conversion. Il reste possible pour un plan déjà converti en partie, et la complétion le règle aussi : elle convertit le point écrit qui manque.
- **AD-5.** C'est le noyau qui tient la boucle et compose. C'est cohérent : la complétion est une décision du noyau.
- **Références d'étape.** Une référence `step:k` ne vaut aujourd'hui qu'au premier niveau d'un paramètre. Les listes de points (`observers` de `terrain.viewshed_cumulative`, `sources` de `terrain.cost_distance`) en demandent une **à l'intérieur d'un tableau de `Point`**. C'est un amendement des règles de référence (AD-11, AD-23) : `step:k.point` est admis comme élément d'un tableau de `Point`, et seulement là.

**Catalogue.** Aucune commande ni aucun paramètre ne change. Les fiches de terrain disent que la complétion s'applique aux plans, et que `terrain.crs_mismatch` reste l'erreur d'un appel direct.

**Spec.** CAP-15 : « un point donné dans un autre système que le MNT est converti par une étape visible que le noyau ajoute au plan ».

**Rejeu.** Rien de neuf : les étapes ajoutées sont des lignes de plan ordinaires, rejouées comme telles.

**Générateur et entraînement.**
- La famille `terrain_autre_systeme` et les variantes « point d'un autre système » de 11.18 changent d'attendu : le plan cible n'a plus d'étape `data.transform_point`, et le point y est recopié de la phrase.
- Les questions sur les listes de points dans un autre système disparaissent, puisque le noyau convertit chaque point.
- Les essais réels (`terrain_reel`, `epic11_reel`, S25 de `plan_reel`) mesurent le **plan complété**, et non plus le plan brut. Changer leur attendu est une décision, notée ici.

**Effort.** Moyen. Une complétion de plan dans le noyau, les références dans les tableaux, des essais. Le générateur suit au ré-entraînement groupé.

**Risque.** Élevé : la composition du plan, les références d'étape et le garde-fou en dépendent. D'où une relecture `thorough`.

## 3. Approche retenue

**Ajustement direct.** Deux entrées dans l'epic 11 :
- la complétion dans le noyau ;
- le générateur, joint à l'entrée du ré-entraînement groupé.

Rien n'est retiré, et aucune commande n'est ajoutée.

## 4. Changements proposés

### 4.1 Architecture

- **Amendement d'AD-19 (2026-11-08).** La conversion d'un point vers le système du raster d'une commande du noyau est une étape `data.transform_point`, que **le noyau ajoute au plan** avant de le montrer :
  - un point par étape ;
  - juste avant la commande qui l'emploie ;
  - la commande reçoit `step:k.point`.

  Le noyau ne l'ajoute que si le système du point diffère de celui du raster, lu par le noyau (AD-20), et si l'extension annonce `data.transform_point`. Un appel direct n'est jamais complété : il reste refusé par `terrain.crs_mismatch`.
- **Amendement des références (AD-23).** `step:k.point` est admis comme élément d'un tableau de `Point`, et seulement là. Il désigne le `point` rendu par l'étape `k`.

### 4.2 Catalogue et spec

- Les fiches `terrain.viewshed`, `terrain.line_of_sight`, `terrain.viewshed_cumulative`, `terrain.cost_distance` et `terrain.least_cost_path` gagnent une phrase sur la complétion. Leurs paramètres ne changent pas.
- CAP-15 gagne la phrase du §2.

### 4.3 Entrées nouvelles, epic 11

- **11.22 — Complétion du plan par les conversions de points.**
  - Dans le noyau, après la validation et le garde-fou, et avant `plan.proposed` : les étapes `data.transform_point` ajoutées, les références renumérotées, et `step:k.point` admis dans les tableaux de `Point`.
  - Essais :
    - un point WGS84 sur un MNT en Lambert-93 ;
    - deux points pour une ligne de vue ;
    - une liste de points pour un champ de vision cumulé ;
    - un plan déjà converti, qui ne reçoit pas de seconde conversion ;
    - un point déjà dans le bon système ;
    - le geste non annoncé, où rien n'est ajouté ;
    - l'appel direct, toujours refusé ;
    - le rejeu identique.
  - Sonde QGIS sur Annecy, avec les trois phrases de terrain de la sonde du 2026-11-08.
- **11.23 — Ré-entraînement groupé.**
  - Les exemples des points tels qu'écrits (décision 3) ;
  - les exemples de `map.topographic_style` et `map.graduated_style` ;
  - l'essai `style_reel` ;
  - le refus périmé du style ;
  - les attendus des essais réels sur le plan complété.

  Il vient après 11.22 et après les autres réglages que le propriétaire veut faire avant : `data.reproject`, puis le fuseau UTM.

## 5. Mise en œuvre

**Portée :** modérée.

**À faire après approbation :**
1. Créer les entrées 11.22 et 11.23 par `bmad-ticket`.
2. Écrire les amendements avec 11.22.
3. Construire 11.22 par `bmad-build`, avec un risque `high`.

**Critères de réussite :**
- Sur Annecy, avec le modèle installé :
  - « Le point 45,9075 N, 6,1700 E (WGS84) voit-il le point 45,8992 N, 6,1294 E (WGS84) sur le MNT à 5 m ? » donne un plan qui convertit les deux points puis calcule la ligne de vue, et qui s'exécute.
  - Le champ de vision cumulé depuis deux points WGS84 donne de même un plan exécuté.
- Un appel direct avec un point WGS84 sur un MNT en Lambert-93 est toujours refusé par `terrain.crs_mismatch`.
- Le rejeu est identique.
