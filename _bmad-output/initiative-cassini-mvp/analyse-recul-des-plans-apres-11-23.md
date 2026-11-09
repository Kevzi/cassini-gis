# Analyse du recul des plans après 11.23 (2026-11-09)

Source : `ecart_reel` rejoué avec `ajuste-apres-11-23` (générateur de 84adb16), plans rendus
affichés à titre d'essai, non commité ; journaux `~/cassini-entrainement/11-23-mesures/`.
Les constats sont écrits en familles, sans recopier de phrase du jeu tenu à l'écart.

## 1. Une part du recul est la variation d'un entraînement à l'autre

| Entraînement | Plans ratés (`ecart_reel`) |
|---|---|
| 11.23, 1er essai | 7 (dont 3 que 11.23c réussit) |
| 11.23c, installé | 5 |
| après 11.23 | 9 (dont 4 que 11.23c réussit) |

Quatre ratés reviennent à chaque fois ; les autres apparaissent et disparaissent d'un
entraînement à l'autre, sur des familles que le générateur n'a pas changées. Sur 27 plans,
l'écart d'un entraînement à l'autre est de l'ordre de ±3, plus large que la marge de la
barre (32/40). Une barre tenue ou ratée d'une phrase ne mesure donc pas à elle seule l'effet
d'une correction.

## 2. Constats durables (ratés à chaque entraînement)

- **Référence ou cible filtrée perdue dans « A filtrées près de B filtrées »** : le modèle garde
  un seul des deux filtres (trois plans). Cause : forme absente du générateur. Corrigé en
  94247a1 (nouveau rang de `draft_multi`), pas encore entraîné.
- **Catégorie de zone du PLU demandée seule** : question au lieu d'une sélection. Corrigé en
  94247a1 pour « les zones {catégorie} » ; les noms voisins (terres, secteurs, espaces) ne sont
  pas encore tirés.

## 3. Constats propres à cet entraînement, causes trouvées dans le générateur

- **Champ de nom suffixé** : le modèle écrit `nom` quand l'inventaire porte un champ de nom
  suffixé du thème (forme `nom_<thème>`). Le générateur ne tire que `nom`, `libelle`,
  `nom_usuel` (`NOM`, `training/src/inventories.rs` l. 153) : il apprend un nom par cœur au lieu
  de le recopier.
- **Fusion du tampon dite dans une proposition à part** (« … et fusionne le tout ») : perdue.
  `DISSOLVE` (`training/src/families.rs` l. 969) n'a que des compléments collés à la tête du
  tampon.
- **Valeur au pluriel d'un champ à valeurs proches** : une autre valeur de la liste est
  choisie. La valeur au pluriel existe (`va(...)`), mais rarement à côté de valeurs voisines du
  même champ.
- **Intersection avec une couche entière « toutes catégories confondues »** : question hors de
  propos (point en coordonnées). Aucune tournure « confondues » dans le générateur, et aucun
  thème de zones d'exposition au bruit en vecteur (le seul thème « bruit » est un raster).
- **Deux conditions d'appartenance sur la même cible** (zone du PLU et autre zonage) : les deux
  intersections sont dans le mauvais ordre, la cible ne reste pas la première couche de la
  chaîne. Déjà raté par 11.23c (une étape de trop) : constat durable, sous une autre forme.

## 4. Suite proposée

1. Corrections du générateur pour les constats 3 (famille ou poids chacun), avec tests.
2. Avant le prochain entraînement, décider comment lire les barres malgré la variation :
   deux entraînements à graines différentes, ou une barre qui tient compte de la variation
   observée. Décision du propriétaire.
