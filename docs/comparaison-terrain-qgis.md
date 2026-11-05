# Comparaison à QGIS des commandes de terrain

Les trois commandes de terrain du noyau — `terrain.slope`, `terrain.line_of_sight` et
`terrain.viewshed` — comparées à l'outil de référence des utilisateurs, QGIS, et à GRASS pour
la visibilité, sur un vrai modèle numérique d'élévation. Ticket 7 de l'epic « raster et
terrain », mesuré le 2026-11-05 sur la révision `d45d5a5`.

**En bref.** La pente du noyau est celle de QGIS à 4 millionièmes de degré près ; seuls les
bords et les abords des trous diffèrent, parce que QGIS les calcule et que le noyau, comme
`gdaldem`, les laisse sans donnée. Les 30 lignes de vue ont le verdict de GRASS. Les champs de
vision concordent sur 98,7 à 100 % des cellules, et presque tous les désaccords sont sur une
lisière entre zone vue et zone cachée, ce qu'explique la façon différente dont les deux outils
lisent le relief entre deux centres de cellule. Aucun écart ne relève d'une erreur du noyau :
aucun ticket de bogue n'est ouvert. Le noyau répond 2,9 à 124 fois plus vite que QGIS selon l'appel, surtout
parce que QGIS paie 2,4 à 2,5 s de démarrage à chaque appel.

## Protocole

- **MNT** : Villeneuve-le-Roi, Lambert-93 (EPSG:2154), 783 × 712 cellules de 23,656 m
  (18,5 × 16,8 km), entiers 16 bits, sans-donnée −32768, 1,4 % de cellules sans donnée. Il
  reste hors du dépôt (`../donnees-test-cassini/`), comme toutes les sorties.
- **Points**, choisis par l'outil de comparaison, de façon déterministe, au centre d'une
  cellule valide :

  | Observateur | Lieu | Altitude du sol |
  |---|---|---|
  | O1 | centre du MNT (657 686,6 ; 6 847 356,7) | 36 m |
  | O2 | point le plus haut à moins de 4 km du centre (654 351,0 ; 6 848 137,4) | 115 m |
  | O3 | point le plus bas à moins de 4 km du centre (658 561,8 ; 6 847 309,4) | 12 m |

  Pour chacun, dix cibles, à 1, 2, … 10 km (la dernière à 9,97-9,98 km, pour rester dans le
  rayon), soit 30 couples.
- **Paramètres, les mêmes des deux côtés** : observateur 1,6 m, cible 0 m, courbure et
  réfraction actives, coefficient de réfraction 0,13 (GRASS prend 1/7 par défaut : il lui est
  passé par `refraction_coeff=0.13`), rayon 10 km ; pente en degrés, facteur Z 1.
- **Outils** :

  | Commande du noyau | Équivalent QGIS |
  |---|---|
  | `terrain.slope`, `unit: degrees` | `native:slope`, `Z_FACTOR=1` |
  | `terrain.viewshed`, rayon 10 km | `grass:r.viewshed -c -r -b`, `max_distance=10000` |
  | `terrain.line_of_sight` | `r.viewshed` ci-dessus, lu à la cellule de la cible |

- **Machine** : AMD Ryzen 7 5800X, Windows 11 ; QGIS 3.44.15 avec GRASS 8.5.0 ; noyau
  `cassini.exe` construit en release depuis la révision mesurée.
- **Mesure** : les sorties de QGIS passent par `gdal_translate` (GeoTIFF Float32 non
  compressé) puis sont lues par la porte raster du noyau ; l'outil vérifie qu'elles sont sur
  la grille du MNT et compare cellule par cellule. Le protocole se rejoue :
  `tests/comparaison-terrain/LISEZMOI.md`.

## Pente

| Mesure | Valeur |
|---|---|
| Cellules du MNT | 557 496 |
| Définies des deux côtés | 546 726 |
| Écart moyen | 0,0000001° |
| Écart maximal | 0,0000038° |
| Cellules à plus de 0,001° d'écart | 0 |
| Définies par QGIS seul : bord de grille | 346 |
| Définies par QGIS seul : voisines d'un trou | 2 662 |
| Définies par QGIS seul : ailleurs | 0 |
| Définies par le noyau seul | 0 |

**Intérieur : même résultat.** Les deux outils appliquent la même formule de Horn ; l'écart
maximal, 3,8 millionièmes de degré, est l'arrondi de la sortie en Float32. C'est le chiffre
déjà trouvé contre `gdaldem slope -alg Horn` au ticket 4.2 (écart maximal 3,8e-6°, masque
identique).

**Masque : écart attendu.** `native:slope` calcule aussi les cellules de bord de grille et
celles qui touchent une cellule sans donnée, en remplaçant la voisine manquante par une autre
valeur. Le noyau les laisse sans donnée, comme `gdaldem` et GRASS par défaut : une pente
calculée avec une altitude inventée n'est pas une mesure. Les 3 008 cellules concernées sont
toutes au bord ou au contact d'un trou ; aucune ailleurs.

## Ligne de vue

**30 couples, 30 verdicts identiques** (5 visibles, 25 masqués). Pour chaque couple, le
verdict de `terrain.line_of_sight` est aussi celui du champ de vision du noyau à la cellule de
la cible, comme le veut la construction de `terrain.viewshed`. Six cibles (O1-C03, O2-C01,
C02, C04, C05 et C08) sont sur une lisière du champ de vision de GRASS, lu dans le disque du
noyau ; aucune n'est en désaccord.

| Observateur | Cibles visibles (noyau et GRASS) |
|---|---|
| O1 | aucune sur 10 |
| O2 | C04, C05, C06, C08, C09 (5 sur 10, de 4 à 9 km) |
| O3 | aucune sur 10 |

O1 et O3 sont au fond de la vallée de la Seine : 1,6 m au-dessus du sol, ils ne voient
presque rien au-delà du fleuve, et les deux outils le disent de la même façon.

## Champ de vision

| Observateur | Cellules comparées | Désaccords | Noyau visible, GRASS masquée | Noyau masquée, GRASS visible | Désaccords sur une lisière | Cellules comparées sur une lisière |
|---|---|---|---|---|---|---|
| O1 | 502 638 | 1 332 (0,27 %) | 1 108 | 224 | 99,3 % | 6,5 % |
| O2 | 430 366 | 5 491 (1,28 %) | 4 490 | 1 001 | 100 % | 37,5 % |
| O3 | 495 414 | 3 (0,00 %) | 0 | 3 | 100 % | 0,01 % |

Part des désaccords par bande de distance :

| Observateur | 0–2,5 km | 2,5–5 km | 5–7,5 km | 7,5–10 km |
|---|---|---|---|---|
| O1 | 1,12 % | 0,52 % | 0,15 % | 0,07 % |
| O2 | 1,83 % | 1,31 % | 1,34 % | 1,02 % |
| O3 | 0,01 % | 0,00 % | 0,00 % | 0,00 % |

Une cellule est « sur une lisière » quand l'une de ses huit voisines a l'autre verdict, dans
l'un des deux outils ; GRASS n'y est lu que dans le disque défini par le noyau, son 0 hors du
rayon et sur les trous n'étant pas une cellule masquée. Sont comparées les cellules que les deux outils définissent, c'est-à-dire
celles du disque de 10 km hors des trous du MNT. Les cellules que GRASS seul définit (47 074,
119 346 et 54 298) ne sont pas des désaccords : avec `-b`, GRASS écrit 0 aussi hors du rayon et
sur les trous, là où le noyau écrit « sans donnée ».

**Explication : deux lectures du relief entre les centres.** Les deux outils jugent chaque
cellule en son centre et lisent le relief entre l'observateur et ce centre, mais pas de la
même façon :

- le noyau échantillonne la ligne à pas constant, une taille de cellule, et interpole chaque
  échantillon en bilinéaire entre les quatre centres voisins ;
- `r.viewshed` (balayage radial de Haverkort, Toma et Zhuang) confronte la pente de visée de la
  cellule à celles des cellules « actives » que la ligne de visée traverse, aux croisements de
  la ligne avec les cellules.

Un sommet de relief qui tombe entre deux échantillons du noyau peut lui échapper, alors que
GRASS l'atteint par la cellule qu'il traverse : c'est pourquoi les désaccords vont surtout dans
le sens « le noyau voit, GRASS ne voit pas » (4 sur 5). L'inverse arrive aussi, les deux modèles
du sol n'étant pas les mêmes. Dans les deux cas, la différence n'importe que pour une cellule
dont la visée frôle le relief, c'est-à-dire en lisière de zone vue : 99,3 à 100 % des
désaccords y sont, quand les lisières ne font que 0,01 à 37,5 % des cellules comparées. Les
désaccords sont plus fréquents près de l'observateur, où un même écart d'altitude entre les deux
modèles change davantage l'angle de visée. O2, au sommet du plateau, voit loin et a beaucoup de
lisières : c'est lui qui a le plus de désaccords, 1,28 %.

**Courbure : écart négligeable.** Le noyau abaisse le sol de (1 − k) d² / 2R sur une sphère de
6 371 km ; GRASS prend le rayon de l'ellipsoïde du système (GRS80 pour le Lambert-93), entre
6 356,8 et 6 378,1 km. Avec k = 0,13, la chute à 10 km vaut 6,828 m pour le noyau, entre
6,820 et 6,843 m selon le rayon que prendrait GRASS : au plus 1,5 cm d'écart à 10 km, à
comparer aux 23,7 m de la cellule et au mètre de l'altitude du MNT. Le manuel de GRASS ne
publie pas sa formule de réfraction ; la concordance mesurée, qui ne se dégrade pas avec la
distance (elle s'améliore pour O1), montre qu'il applique le même coefficient de la même
façon à cette précision.

## Temps

Durée murale de chaque appel, en secondes, sur la même machine, par le script de comparaison,
lors de deux passages complets du protocole (les écarts mesurés y sont identiques) ; à titre
d'information, sans seuil.

| Appel | Noyau (`cassini.exe run`), passage 1 / 2 | QGIS (`qgis_process`), passage 1 / 2 |
|---|---|---|
| À vide (démarrage) | 0,04 / 0,01 | 2,37 / 2,52 |
| Pente | 0,18 / 0,16 | 2,47 / 2,46 |
| Champ de vision O1 | 0,84 / 0,76 | 9,09 / 4,75 |
| Champ de vision O2 | 1,60 / 1,63 | 5,30 / 4,79 |
| Champ de vision O3 | 0,04 / 0,05 | 4,97 / 4,49 |
| 30 lignes de vue (30 appels) | 0,53 / 0,49 | 19,36 / 14,03 (les 3 `r.viewshed` ci-dessus) |

Lecture :

- QGIS paie environ 2,4 à 2,5 s de démarrage à chaque appel de `qgis_process`, le noyau
  quelques centièmes. Le premier `r.viewshed` du passage 1 (9,1 s) a payé en plus une mise en
  route de GRASS à froid ; au passage 2, chaque `r.viewshed` coûte 4,5 à 4,8 s.
  Dans QGIS ouvert, ce démarrage disparaît ; il reste, pour `r.viewshed`, l'import du MNT dans
  une session GRASS temporaire et l'export du résultat.
- Le champ de vision du noyau dépend de ce que voit l'observateur : chaque ligne s'arrête au
  premier obstacle. O3, au fond de la vallée, est masqué presque partout dès les premières
  cellules (0,04 s) ; O2, qui voit loin, coûte le plus (1,6 s). Le balayage de GRASS a un
  coût presque indépendant de ce qui est vu.
- Les lignes de vue n'ont pas d'équivalent direct dans QGIS : les 30 verdicts de GRASS sont
  lus dans les trois champs de vision complets, dont le coût cumulé tient donc lieu de temps
  QGIS pour cette ligne.
- Rapports, appel par appel : de 2,9 (champ de vision O2, passage 2) à 124 (champ de vision
  O3, passage 1) ; 14 à 15 pour la pente, 29 à 37 pour les lignes de vue.

## Provenance du code

Aucune ligne de code de GRASS, de Whitebox, de GDAL ni de QGIS n'a été reprise dans le
noyau. Les trois commandes ont été écrites d'après les publications et les formules
classiques ci-dessous, sans aucun fichier porté ; les en-têtes des modules le disent
(`core/src/commands/terrain_slope.rs`, `terrain_line_of_sight.rs`, `terrain_viewshed.rs`).
Les GeoTIFF sont lus et écrits par `core/src/dem.rs` avec la crate `tiff` (MIT), sans GDAL.

| Commande | Ce qui est repris | Source |
|---|---|---|
| `terrain.slope` | Différences finies pondérées de Horn sur la fenêtre 3 × 3 | B. K. P. Horn, « Hill shading and the reflectance map », *Proceedings of the IEEE*, 69(1), 1981, p. 14-47 |
| `terrain.line_of_sight` | Test direct de la ligne de visée, échantillonnée à pas constant, altitude interpolée en bilinéaire | Méthode générale, sans source propre |
| | Chute due à la courbure, flèche d²/2R d'une sphère (approximation pour d ≪ R) | Géodésie classique |
| | Rayon R = 6 371 km : rayon moyen R1 de l'ellipsoïde GRS80, arrondi | H. Moritz, « Geodetic Reference System 1980 », *Journal of Geodesy*, 74, 2000, p. 128-133 |
| | Réfraction : chute réduite du facteur (1 − k), k = 0,13 par défaut | Coefficient de réfraction terrestre de C. F. Gauss, triangulation du Hanovre (années 1820), valeur classique de la géodésie |
| `terrain.viewshed` | Une ligne de vue propre par cellule, de l'observateur vers son centre (algorithme exact « R3 ») | W. R. Franklin et C. Ray, « Higher isn't necessarily better: visibility algorithms and experiments », dans T. C. Waugh et R. G. Healey (dir.), *Advances in GIS Research: Sixth International Symposium on Spatial Data Handling*, Édimbourg, 1994, p. 751-770 |
| | Variante : l'échantillonnage de chaque ligne est celui de `terrain.line_of_sight`, pas les croisements avec la grille du papier | Choix du noyau (ticket 4.5), pour que le champ de vision et la ligne de vue rendent toujours le même verdict |

Historique : `a378d09` (pente, 4.2), `c246372` (ligne de vue, 4.4), `4e4a12f` (champ de vision,
4.5) et `61728bb` (message de refus du champ de vision), complétés par les références
ci-dessus au ticket 7.

`r.viewshed` est cité ici comme outil de comparaison seulement. Sa référence :
H. Haverkort, L. Toma et Y. Zhuang, « Computing visibility on terrains in external memory »,
*ACM Journal on Experimental Algorithmics*, 13, 2009.
