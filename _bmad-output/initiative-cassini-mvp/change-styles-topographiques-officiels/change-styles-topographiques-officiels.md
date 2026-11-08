---
type: change
title: "Mise en forme topographique des couches selon les normes IGN et CNIG"
date: 2026-11-08
status: approuvé (propriétaire, 2026-11-08 ; rangement et fiches du catalogue approuvés le même jour, entrée 11.19)
scope: moderate
---

# Changement de cap — mise en forme topographique des couches

## 1. Résumé

**Déclencheur.** Demande du propriétaire, le 2026-11-07 et le 2026-11-08. Sur le jeu
d'Annecy (ticket 11.18), les couches ouvertes telles quelles, avec le MNT par-dessus, donnaient
une carte et un PDF où l'on ne voyait rien. Il a fallu les mettre en forme à la main par un
script jetable (`%TEMP%\sonde-11-18\style.py`) pour obtenir une carte lisible (`deferred-work.md`,
idée de commande de style topographique). Le propriétaire veut que l'IA sache le faire sur
une phrase : « mets la carte aux couleurs topographiques ». Il place ce travail en deuxième,
juste après l'historique du journal.

**Problème.** Cassini sait ouvrir, analyser et exporter, mais pas rendre une carte lisible.
Une couche ouverte reçoit la couleur au hasard que choisit QGIS. Le PDF de démonstration
dépend donc d'une mise en forme manuelle, que l'utilisateur visé, qui ne sait pas où cliquer,
ne fera pas. Aucune extension QGIS existante ne comble ce manque hors ligne et sur des données
quelconques : BD TOPO Extractor ne traite que la BD TOPO et passe par le réseau ; AIAMAS appelle
une IA en ligne ; Adjust Style ne suit aucune norme.

**Type.** Nouvelle exigence du propriétaire. Commande nouvelle, donc changement du catalogue
figé (catalogue des commandes, §1).

**Décisions du propriétaire (2026-11-08) :**
1. **Référence :** la sémiologie publique de l'IGN (carte topographique au 1:25 000, BD TOPO)
   pour l'eau, la végétation, les routes, le bâti, les limites et le relief ; le standard CNIG
   pour les zones d'un PLU (U, AU, A, N).
2. **Portée :** une couleur par thème, **et** un classement par attribut quand la donnée le
   permet : routes par importance, zones du PLU par type, bâti par usage, MNT en teintes
   d'altitude.
3. **Reconnaissance :** d'après le nom de la couche et de son fichier, puis ses champs connus
   (par exemple `nature` et `importance` de la BD TOPO, `typezone` du PLU). Une couche qui ne
   correspond à rien garde son style, et Cassini le dit.
4. **Moment :** sur demande seulement. La commande est journalisée et rejouable, et ne touche
   jamais aux fichiers d'origine (§2.6).

5. **Rangement (ajout du 2026-11-08, après l'approbation) :** en plus des couleurs et
   des classes, les couches sont rangées en groupes par thème (Hydrographie, Transport,
   Bâti, Urbanisme, Relief…) et dans l'ordre d'affichage : relief en dessous, puis
   surfaces, lignes, points. Sur Annecy, le MNT posé par-dessus cachait tout. Cet ajout
   fait une troisième commande, `layer.arrange`, dictée comme `layer.set_style` (§4.1).

6. **Dégradé gradué (ajout du 2026-11-08) :** `StyleSpec` gagne une cinquième forme, `graduated`. C'est un dégradé de couleurs selon un champ numérique d'un vecteur, en `class_count` classes, par intervalles égaux, quantiles ou seuils naturels (`jenks`). QGIS calcule les seuils sur les valeurs réelles ; le noyau ne choisit que le champ, la méthode, le nombre de classes et les deux couleurs extrêmes.

7. **Commande de dégradé (ajout du 2026-11-08, demande du propriétaire) :** `map.graduated_style`, offerte au modèle, colore une couche selon un champ numérique (« colore les communes selon la population »).
   - Paramètres : `layer_id` et `field` ; `method` (seuils naturels par défaut), `class_count` (5 par défaut, de 2 à 12) et `ramp`, un dégradé nommé (`yellow_red` par défaut, `blues`, `greens`, `reds`, `purples`, `greys`, `red_blue`).
   - Le noyau compose le `graduated` et dicte un seul `layer.set_style`, comme `map.topographic_style`.
   - Les dégradés sont sourcés, par exemple ColorBrewer, sous licence Apache-2.0. Le catalogue fait foi.

8. **Visibilité selon l'échelle (ajout du 2026-11-08) :** bâti et parcelles n'apparaissent qu'à partir du 1:50 000 ; champ facultatif `max_scale` des formes vecteur de `StyleSpec`.
   - Ajusté le 2026-11-08 : une classe peut porter son propre `max_scale` (petites routes à partir du 1:100 000) ; seuil du détail (bâti, parcelles) porté au 1:100 000 ; légende d'altitude en barre continue, bornée par les altitudes réelles du MNT.
   - Ajusté le 2026-11-08 : zones du PLU translucides (`opacity` facultative de `categories`, 0,6 soit 40 % de transparence, CNIG) ; plaines crème comme le fond du Plan IGN (0 à 300 m) et ombrage allégé (opacité choisie : 0,4).

**Test de CLAUDE.md §2.9 :** une carte aux couleurs IGN sert un bureau d'études, une
collectivité ou un étudiant tels quels. C'est une fonction civile. La symbologie militaire
(APP-6) reste hors périmètre (spec, Non-goals), et rien ici ne s'en approche.

## 2. Analyse d'impact

**Epics.**
- Epic 11 (géométrie et terrain étendu, en cours) : la démonstration (11.14) gagne une carte
  lisible sans script manuel. Nouvelles entrées 11.19 à 11.21 (§4.4), avant 11.14.
- Epic 10 (catalogue étendu, après le MVP) : rien ne change. `data.reproject` y reste planifiée.
- Aucun epic n'est rendu caduc.

**Spec.**
- CAP-5 (export PDF) : la carte exportée peut être mise en forme par une commande du registre.
- La question ouverte « Connaissance des schémas français sans les données » est tranchée pour
  la symbologie : oui, le noyau reconnaît les schémas BD TOPO, ADMIN EXPRESS et le standard
  CNIG du PLU d'après les noms et les champs, sans livrer aucune donnée.

**Architecture.**
- AD-5 et AD-22 sont tenues. **La règle de reconnaissance et la table des couleurs vivent dans
  le noyau** : c'est une décision métier. L'extension n'applique qu'une description de style
  neutre (couleurs, épaisseurs, classes par valeur de champ, rampe d'altitudes), qu'elle
  traduit en moteur de rendu de QGIS. Le noyau ne nomme aucune classe de QGIS.
- **Amendement à écrire** (dans l'esprit d'AD-30, sur les ouvertures dictées) : une commande
  du noyau, `map.topographic_style`, lit l'inventaire du projet, puis dicte une étape
  `layer.set_style` par couche reconnue, chacune journalisée. Le noyau dicte déjà des ordres
  après une étape (`layer.open_raster`, `layer.open_vector`) ; ici il en dicte plusieurs pour
  une seule demande.
- AD-24 : `layer.set_style` est une ligne de la table fermée, d'un genre nouveau. Elle ne
  produit ni couche ni fichier : le genre de sortie `none` est à ajouter, une seule fois.

**UX.** Le panneau montre le résultat : les couches mises en forme, avec le thème reconnu, et
celles laissées telles quelles. Aucun écran nouveau.

**Modèle.** Une commande nouvelle doit être apprise. Elle est simple (« mets aux couleurs
topographiques » donne `map.topographic_style`, sans paramètre ou avec une liste de couches).
Ses exemples entrent au **prochain ré-entraînement groupé**, et non dans terrain-11-18b, qui
tourne sur la branche des exemples de terrain.

**Juridique.**
- Les couleurs sont des valeurs relevées dans la documentation publique de l'IGN et du CNIG,
  sourcées une à une dans la table du noyau, et réécrites par nous.
- Aucun fichier de style de l'IGN n'est recopié tant que sa licence n'est pas vérifiée (Licence
  Ouverte Etalab 2.0 probable, mais à confirmer à la source).
- Rien d'Esri (§2.1).

## 3. Approche retenue

**Ajustement direct**, effort moyen, risque moyen. Trois commandes au catalogue, deux amendements
d'architecture, trois entrées. Pas de retour en arrière, pas de réduction du MVP.

Pourquoi le noyau décide et ne laisse pas le modèle choisir le style de chaque couche :
- La reconnaissance par nom et par champs est **déterministe et testable sans QGIS**.
- Le modèle n'apprend qu'une phrase simple, au lieu d'un style par thème.
- Le même résultat vaut pour la CLI, le client MCP et le panneau (§4 de CLAUDE.md).

## 4. Changements proposés

### 4.1 Catalogue des commandes (`spec-cassini-mvp/catalogue-des-commandes.md`)

Trois fiches planifiées, ajoutées à la liste du §4 et en fiches au §6. **Le catalogue fait foi** : il précise l'esquisse ci-dessous (entrée 11.19, après relecture), par exemple `layer_id` au lieu de `layer`, le groupe « Autres couches », la lecture de l'inventaire hors du journal et le rejeu.

`map.topographic_style` — planifiée, `runs_in: core`, offerte au modèle.
- `layers` : liste de `LayerId`, facultative. Absente : toutes les couches du projet.
- Résultat : `styled` (liste `{layer_id, theme}`), `unchanged` (liste `{layer_id, reason}`).

`layer.set_style` — planifiée, `runs_in: qgis`, `dictated_by_core: true`.
- `layer` : `LayerId`.
- `style` : `StyleSpec`, un type nommé nouveau (§3 du catalogue).
- Résultat : `layer_id`.

`layer.arrange` — planifiée, `runs_in: qgis`, `dictated_by_core: true` (décision 5).
- `groups` : liste de `{name, layers}`, de haut en bas.
- Résultat : `group_count`, `layer_count`. `map.topographic_style` rend aussi `groups`.

`StyleSpec` (neutre, AD-22), l'une de quatre formes :
- `single` : `fill`, `stroke` (couleurs `#rrggbb`), `width_mm`, `opacity`.
- `categories` : `field`, puis `classes` = liste de `{values, fill, stroke, width_mm, label}`,
  et un style `other` pour les valeurs non listées.
- `elevation_ramp` : `stops` = liste de `{value, color}` (teintes d'altitude d'un MNT).
- `hillshade` : `opacity` (ombrage posé en fondu sur les teintes d'altitude).

Les identifiants, paramètres et champs ci-dessus sont figés à l'approbation de ce changement.

### 4.2 Spec (`spec-cassini-mvp/spec-cassini-mvp.md`)

- CAP-5, success : ajouter « les couches peuvent être mises en forme selon la sémiologie
  topographique de l'IGN et le standard CNIG par une commande du registre, sans modifier les
  fichiers d'origine ».
- Open Questions, « Connaissance des schémas français » : *Tranché le 2026-11-08 pour la
  symbologie : reconnaissance par nom et champs dans le noyau, aucune donnée livrée.*

### 4.3 Architecture (`architecture-noyau-et-facade-qgis.md`)

- Amendement d'AD-30 (ouvertures dictées) : une commande du noyau peut dicter **plusieurs**
  ordres `dictated_by_core` (`layer.set_style`, puis `layer.arrange`) pour une seule demande,
  après avoir lu l'inventaire hors du journal. Chacun est une étape
  du journal, et le rejeu les redonne dans le même ordre.
- Amendement d'AD-24 : le genre de sortie `none`, pour une commande qui change l'état du
  projet sans produire ni couche ni fichier.

### 4.4 Entrées nouvelles, epic 11

- **11.19 — Contrat de la mise en forme topographique.** Fiches du catalogue, type `StyleSpec`,
  amendements d'architecture. La recette `core/tests/catalogue.rs` reste verte. Documents
  seulement.
- **11.20 — Table de sémiologie et reconnaissance dans le noyau.**
  - La table des thèmes (eau de surface, cours d'eau, végétation, routes par importance, voie
    ferrée, bâti, limites administratives, zones du PLU, MNT, ombrage), chaque couleur sourcée.
  - Les règles de reconnaissance par nom, fichier et champs, `map.topographic_style` et ses
    tests. Elles sont essayées sur les inventaires de Villeneuve et d'Annecy reproduits en
    fixtures synthétiques.
- **11.21 — Gestes `layer.set_style` et `layer.arrange` dans l'extension, et démonstration.**
  - Les lignes de la table fermée, la traduction de `StyleSpec` en rendu QGIS et le rangement
    de l'arbre des couches.
  - La sonde QGIS : Villeneuve et Annecy mis en forme par une phrase, PDF rendu en image et
    regardé (mémoire « vérifier le PDF à l'œil »), rejeu identique.
- Les exemples du générateur pour `map.topographic_style` vont dans l'entrée du prochain
  ré-entraînement groupé : des tournures variées (« aux couleurs de l'IGN », « rends la carte
  lisible », « style topo », « habille les couches »…), avec des fautes de frappe et un registre
  familier, et des demandes limitées à certaines couches (`layers`).
- **Essai réel `style_reel`** (demande du propriétaire, 2026-11-08), dans la même entrée.
  - Une dizaine de phrases qui ne sont pas dans les exemples, tournées autrement, que le
    propriétaire peut compléter, sur le modèle d'`ecart_reel`.
  - Barre : au moins 9 sur 10 rendent `map.topographic_style`, avec les bonnes couches quand la
    phrase en nomme.
  - Le même essai porte quelques phrases de dégradé (« colore les communes selon la
    population », « mets les parcelles en dégradé de bleus par surface »), qui doivent rendre
    `map.graduated_style` avec la bonne couche, le bon champ et, quand la phrase le dit, le
    bon dégradé.
  - Une phrase hors sujet, comme « mets les routes en rose fluo », reçoit une question ou un
    refus, jamais la mise aux normes.
- 11.14, la démonstration, prend 11.21 dans `after`.

## 5. Mise en œuvre

**Portée :** modérée. Réorganisation du carnet (trois entrées), puis développement.

**À faire après approbation :**
1. Écrire les fiches, la spec et les amendements (11.19), dans `../cassini-wt-suite`.
2. Créer les entrées 11.19 à 11.21 par `bmad-ticket` et mettre 11.21 dans l'`after` de 11.14.
3. Construire 11.20, puis 11.21, par `bmad-build` : risque `high` pour 11.21, qui touche le
   canal, d'où une relecture `thorough`.

**Critères de réussite :**
- La phrase « mets la carte aux couleurs topographiques » donne, sur Villeneuve et sur
  Annecy, l'eau en bleu, les bois en vert, les routes classées par importance et le PLU aux
  couleurs CNIG. Le PDF rendu en image le montre.
- Une couche non reconnue garde son style, figure dans `unchanged` et va au groupe « Autres
  couches ».
- Les couches sont rangées en groupes par thème, le relief en dessous : aucun MNT ne recouvre
  la carte.
- Le rejeu du journal redonne les mêmes styles.
- Aucun fichier d'origine n'est modifié.
- Le modèle ré-entraîné tient la barre de `style_reel` (au moins 9 phrases nouvelles sur 10),
  sans faire baisser les barres des essais existants.
