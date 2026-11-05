# Relecture « vérification » — changements du 2026-11-05

Portée : AD-19 amendée, AD-20 amendée, AD-30 nouvelle, convention de nommage, ligne CAP-15
(`git diff HEAD` du contrat d'architecture). Question : chaque affirmation factuelle a-t-elle
été confrontée à une source plutôt qu'affirmée de mémoire ?

## Verdict

Les faits portés par AD-19 et AD-30 tiennent ; deux points dépendent de QGIS et doivent être
précisés avant de construire : `native:alignrasters` n'est pas l'algorithme à appeler hors
modeleur, et la mise en page de QGIS n'a pas de format d'annotation MGRS natif.

## Points vérifiés

1. **GeoJSON 2008 et membre `crs` (AD-30) — confirmé, une réserve.** La documentation GDAL
   (pilote GeoJSON, RFC 65) indique que `RFC7946=NO` par défaut, donc l'écriture suit la
   version 2008, qui garde le membre `crs` quand la couche n'est pas en WGS84. La lecture du
   `crs` de type `name` par OGR est documentée ; QGIS ouvre le GeoJSON par OGR, donc hérite
   du système déclaré. Réserve : la chaîne exacte `urn:ogc:def:crs:EPSG::<code>` n'a pas été
   retrouvée mot pour mot dans les pages consultées ; à confirmer par un test réel (écrire un
   fichier EPSG:2154, l'ouvrir dans QGIS 3.44 et 4.x) — c'est le critère d'acceptation à
   inscrire au ticket. Noter aussi que RFC 7946 a retiré `crs` : le choix 2008 est assumé et
   doit le rester explicitement.
   Sources : https://gdal.org/drivers/vector/geojson.html ,
   https://gdal.org/development/rfc/rfc65_rfc7946_geojson.html
2. **Codes UTM WGS84 32601–32660 / 32701–32760 (AD-19) — confirmé.** Registre EPSG
   (« WGS 84 / UTM zone 1N » = 32601 … « 60S » = 32760). https://epsg.io/32601
3. **MGRS sur WGS84, NGA ; formules de Karney publiées (AD-19) — confirmé.** MGRS est défini
   par la norme publique NGA.SIG.0012 (« Universal Grids and Grid Reference Systems ») ;
   Karney, « Transverse Mercator with an accuracy of a few nanometers », J. Geodesy 85 (2011),
   série de Krüger d'ordre 6, en libre accès (arXiv:1002.1417). Attention aux bandes
   particulières (Norvège 32V, Svalbard 31X–37X) et aux zones polaires UPS, à couvrir par les
   jeux de référence. https://earth-info.nga.mil , https://arxiv.org/abs/1002.1417
4. **`native:alignrasters` et `gdal:polygonize` — confirmé avec réserve importante.** Les deux
   existent. Mais la documentation QGIS 3.40 (Outils raster) signale `native:alignrasters`
   comme réservé au contexte du modeleur graphique, et l'outil interactif « Align rasters »
   comme distinct ; la page « testing » dit l'inverse. Pour un appel par l'extension,
   préférer `native:alignsingleraster` (un raster sur une référence) ou vérifier
   `QgsApplication.processingRegistry().algorithmById(...)` dans QGIS 3.44 et 4.x avant de
   figer l'identifiant dans le catalogue. `gdal:polygonize` est documenté (Conversion raster,
   GDAL). https://docs.qgis.org/3.40/en/docs/user_manual/processing_algs/qgis/rastertools.html
5. **Carroyage MGRS par la mise en page de QGIS (CAP-15) — non, pas nativement.** La grille
   de carte (`QgsLayoutItemMapGrid`) accepte un système quelconque (donc une grille UTM de la
   zone) et des formats d'annotation décimal / DMS / personnalisé par expression, mais aucun
   format MGRS. Un carroyage UTM annoté en MGRS (lettres de carré de 100 km) demanderait une
   expression personnalisée ou une extension tierce ; le greffon MGRS de la NSA ne fait que
   saisie et zoom. CAP-15 (« carroyage … par la mise en page de QGIS ») est donc imprécis :
   écrire « carroyage UTM par la mise en page, étiquettes MGRS par expression à démontrer »
   ou retirer MGRS du carroyage. https://qgis.org/api/classQgsLayoutItemMapGrid.html ,
   https://github.com/NationalSecurityAgency/qgis-mgrs-plugin

## Actions proposées

- A1 (AD-30) : critère d'acceptation par aller-retour réel GDAL/QGIS du membre `crs`.
- A2 (changement de cap) : remplacer ou vérifier `native:alignrasters` sur 3.44 et 4.x.
- A3 (CAP-15) : reformuler le carroyage ; MGRS n'est pas un format natif de la grille.
