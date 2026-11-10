# Revue guidée : générateur corrigé après 11.23

Bloc en cours : 2

## [x] 1. Intention — accepté (modifié pendant la revue)
Source : section Intent du plan `_bmad-output/initiative-cassini-mvp/plan-re-entrainement-apres-11-23.md` (lien [plan](../plan-re-entrainement-apres-11-23.md)), recopiée telle quelle :

> **Problem:** le modèle installé (`ajuste-11-23c`) rate encore des demandes réelles. En priorité, à la recette QGIS du 2026-11-09 sur Annecy, une zone tampon autour des gares (couche CSV en EPSG:4326, projet en EPSG:2154) part sans `data.reproject`. Il ne connaît pas `data.convert` (10.9). Aux essais GPU, il manque des refus (vue 3D d'une partie d'une couche ; export d'un sous-ensemble filtré en tableur ; PDF qui choisit ses couches ; statistique d'altitude par zone), perd une rampe nommée d'un dégradé, écrit mal la zone UTM d'arrivée d'un point, pose une question sur un code de zone du PLU non littéral, et saute ou ajoute une étape dans les enchaînements sélection-sélection-tampon-intersection.
>
> **Approach:** corriger le générateur d'exemples par familles (jamais par phrase) d'après ces constats, puis lancer l'entraînement accéléré, les essais GPU contre les barres de 11.16 et de 11.23, et rendre les résultats au propriétaire, qui décide seul de l'installation.

Note : corrigé pendant la revue à la demande du propriétaire (« projet en UTM » → « projet en EPSG:2154 », commit 47ef2f4).

## [ ] 2. Grandes lignes — en cours
Le générateur fabrique des exemples (phrase, inventaire, plan attendu) par familles tirées selon un poids. Le changement ajoute des formes d'inventaire proches d'Annecy, des variantes de refus et de sélections, et retouche quelques poids. Points d'entrée, dans l'ordre :
- [families.rs:5616](../../../training/src/families.rs#L5616) — `reprojection_avant_analyse`, le cas d'Annecy.
- [inventories.rs:1605](../../../training/src/inventories.rs#L1605) — `project_system`, `long_inventory`, `as_csv` : les nouvelles formes d'inventaire.
- [families.rs:3234](../../../training/src/families.rs#L3234) — `refus`, les refus élargis.
- [families.rs:1198](../../../training/src/families.rs#L1198) — `zone_category`, les zones du PLU dites en mots.
- [families.rs:1454](../../../training/src/families.rs#L1454) — nouveau rang de `draft_multi` (deux sélections, tampon, intersection).

## [ ] 3. Reprojection avant analyse (Annecy) — à voir
Une analyse en mètres sur une couche en degrés commence par `data.reproject`. La famille tire maintenant une seconde couche en 4326 non citée, une source `.csv`, un inventaire long, et, quand tout est en 4326, un projet en mètres dont le système devient la cible. La famille contraire (`analyse_couche_metrique`) reçoit les mêmes formes, pour qu'aucune ne déclenche seule une reprojection.
- [families.rs:5616](../../../training/src/families.rs#L5616) — tirage de la cible et des formes.
- [families.rs:5664](../../../training/src/families.rs#L5664) — `analyse_couche_metrique`.
- [inventories.rs:1618](../../../training/src/inventories.rs#L1618) — `as_csv`.
- [families.rs:248](../../../training/src/families.rs#L248) — poids 4 → 6.

## [ ] 4. Refus élargis — à voir
Les demandes hors du catalogue (vue 3D d'une partie, tableur d'un sous-ensemble, modification ou suppression d'entités, PDF qui choisit ses couches, statistique d'altitude par zone) sont plus fréquentes et plus variées ; la réponse reste un refus.
- [families.rs:3199](../../../training/src/families.rs#L3199) — `subset_phrase`, le sous-ensemble nommé.
- [families.rs:3234](../../../training/src/families.rs#L3234) — `refus`.
- [families.rs:3369](../../../training/src/families.rs#L3369) — `refus_hors_registre`.

## [ ] 5. Zones du PLU et sélections enchaînées — à voir
Une catégorie dite en mots (agricole, urbaine, naturelle, à urbaniser) devient un `select` sur le code de zone, en condition d'une autre couche ou seule ; le libellé est alors interdit dans l'inventaire. Un nouveau rang produit le plan à quatre étapes « cibles filtrées près d'une référence filtrée ».
- [families.rs:1198](../../../training/src/families.rs#L1198) — `zone_category`.
- [families.rs:763](../../../training/src/families.rs#L763) — `draft_filter_eq`, les zones demandées seules.
- [families.rs:1454](../../../training/src/families.rs#L1454) — rang `11 | 12` de `draft_multi`.

## [ ] 6. Dégradés et fuseau UTM — à voir
La rampe nommée accompagne plus souvent la méthode du dégradé ; le passage au fuseau voisin d'un point est tiré une fois sur deux, avec six écritures du fuseau.
- [families.rs:2438](../../../training/src/families.rs#L2438) — `style_degrade`.
- [families.rs:5196](../../../training/src/families.rs#L5196) — `convert_words`.
- [families.rs:5265](../../../training/src/families.rs#L5265) — `conversion_coordonnees`.

## [ ] 7. Périphérie — à voir
- [entrainement-du-modele.md](../../../docs/entrainement-du-modele.md) — tableau des familles et résultats de l'essai.
- [recouvrement.rs](../../../training/tests/recouvrement.rs) — garde du jeu tenu à l'écart, inchangée et verte.
- [analyse-recul-des-plans-apres-11-23.md](../analyse-recul-des-plans-apres-11-23.md) — analyse des ratés.
