# Comparaison des commandes de terrain à QGIS

Protocole rejouable de la note `docs/comparaison-terrain-qgis.md` : il lance la pente,
l'exposition, l'ombrage, le champ de vision et la ligne de vue du noyau, puis leurs
équivalents QGIS et GRASS, avec les mêmes paramètres, et mesure les écarts et les temps.

Rien de ce que produit le protocole n'entre au dépôt : le MNT et toutes les sorties restent
dans un dossier de travail choisi hors du dépôt. Seuls les chiffres vont dans la note.

## Ce qu'il faut

- Le MNT de Villeneuve-le-Roi en Lambert-93, hors dépôt :
  `../donnees-test-cassini/dem/mnt-villeneuve-le-roi-2154.tif`.
- Le noyau Windows construit depuis la révision à mesurer : `C:\ct\release\cassini.exe`.
- QGIS 3.44.15 installé dans `C:\Program Files\QGIS 3.44.15` (fournit `qgis_process`,
  GRASS 8.5 et `gdal_translate`).

## Les trois étapes

Depuis WSL, à la racine du dépôt, avec un dossier de travail vide hors du dépôt (ici
`C:\Users\kevin\comparaison-terrain`) :

```bash
mkdir -p /mnt/c/Users/kevin/comparaison-terrain

# 1. Les points : 3 observateurs, 30 cibles de 1 à 10 km, au centre de cellules valides.
cargo run --release -p cassini-core --example comparer_terrain -- \
  points ../donnees-test-cassini/dem/mnt-villeneuve-le-roi-2154.tif \
  > /mnt/c/Users/kevin/comparaison-terrain/points.json

# 2. Les deux outils, chronométrés (environ 40 s).
powershell.exe -NoProfile -ExecutionPolicy Bypass -File tests/comparaison-terrain/comparer.ps1 \
  -Travail 'C:\Users\kevin\comparaison-terrain'

# 3. Les écarts, en Markdown.
cargo run --release -p cassini-core --example comparer_terrain -- \
  comparer /mnt/c/Users/kevin/comparaison-terrain
```

Le script accepte aussi `-Mnt`, `-Cassini` et `-Qgis` pour d'autres chemins.

## Ce que fait chaque étape

- `points` choisit O1 au centre du MNT, O2 au point le plus haut et O3 au point le plus bas
  à moins de 4 km du centre, puis, pour chacun, une cible à 1, 2, … 10 km. Le choix est
  déterministe : le même MNT donne les mêmes points.
- `comparer.ps1` copie le MNT en `mnt.tif` dans le dossier de travail, lance le noyau
  (`system.about` à vide, `terrain.slope`, `terrain.aspect`, `terrain.hillshade`, trois
  `terrain.viewshed`, trente `terrain.line_of_sight`) puis QGIS (`qgis_process help` à vide,
  `native:slope`, `native:aspect`, `native:hillshade`, trois `grass:r.viewshed -c -r -b`), et écrit les durées murales dans `temps.json`. Les sorties de
  QGIS sont normalisées par `gdal_translate -co COMPRESS=NONE -ot Float32 -a_srs EPSG:2154` en
  GeoTIFF lisibles par la porte raster du noyau ; le message « Unable to export color table »
  de `gdal_translate` sur les champs de vision est sans effet (la table de couleurs de GRASS
  est seulement abandonnée).
- `comparer` vérifie que chaque sortie est sur la grille du MNT (sinon il s'arrête avec un
  message clair), compare cellule par cellule (l'exposition par l'écart sur le cercle :
  359° et 1° sont à 2°), classe chaque désaccord de masque selon le lieu de la cellule (bord
  de grille, voisine d'un trou, cellule plate, ailleurs) et signale en gras tout désaccord
  « ailleurs », qui appelle un ticket de bogue ; puis il calcule les lignes de vue du noyau
  par le registre et lit `r.viewshed` à la cellule de chaque cible.

Paramètres communs : observateur 1,6 m, cible 0 m, courbure et réfraction actives,
coefficient 0,13 (passé à GRASS par `refraction_coeff`, qui vaut 1/7 par défaut), rayon
10 km, pente en degrés avec un facteur Z de 1 ; exposition avec un facteur Z de 1 ; ombrage
avec les défauts du noyau, passés à QGIS (`Z_FACTOR=1 AZIMUTH=315 V_ANGLE=45`).

Les tests de l'instrument tournent avec la recette :
`cargo test -p cassini-core --example comparer_terrain`.
