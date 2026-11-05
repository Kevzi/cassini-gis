# Relecture du spine — lentille « grille de qualité » — 2026-11-05

Objet : amendements du 2026-11-05 au contrat d'architecture
(`architecture-noyau-et-facade-qgis.md`, `git diff HEAD`) — AD-8, AD-19, AD-20,
nouvelle AD-30, convention de nommage (`coord.*`, `geo.*`, `raster.reclassify`,
`raster.weighted_overlay`), ligne CAP-15 de la table de correspondance.

Sources confrontées : changement de cap
`change-geometrie-et-terrain-avant-la-demo.md` (§4.1, §4.4), spec
`spec-cassini-mvp.md` (CAP-5, CAP-15, Constraints, Non-goals), et le reste du
spine (AD-1, AD-6, AD-8 à AD-11, AD-16, AD-21 à AD-26, Deferred).

## Verdict

**À reprendre avant l'inception de l'epic 11.** Les trois amendements demandés
par §4.4 sont écrits et leurs Rules sont pour l'essentiel vérifiables (AD-19
borne bien l'exception, AD-20 interdit bien le rééchantillonnage, AD-30 fixe une
seule forme de sortie vecteur). Mais l'amendement d'AD-19 heurte de front
AD-23 (coordonnées, type `Point`, liste fermée des systèmes), un point de
divergence laissé en Deferred est précisément celui que l'epic 11 doit
traverser, et AD-30 introduit un second mécanisme d'ouverture des sorties du
noyau à côté de celui de 4.9 (AD-9). Deux constats critiques, trois hauts.

## Constats

### C1 — Critique — AD-19 amendée contre AD-23 : une coordonnée MGRS n'a pas de type, et UTM n'est pas un système nommable

- AD-23 : « Toute coordonnée du registre est le type `Point` partagé du noyau
  (x, y, epsg) » ; la garde de provenance compare x, y et le système « comme
  nombres » ; « un système nommé compte pour son code, sur une liste fermée :
  Lambert-93 (2154) et WGS84 (4326) ».
- L'amendement d'AD-19 fait entrer MGRS (une chaîne : zone, carré de 100 km,
  chiffres) et UTM (EPSG:326xx/327xx). Conséquences non tranchées :
  1. **Entrée MGRS de `coord.convert`.** Si c'est une chaîne, elle échappe au
     type `Point` et donc à la garde de provenance : un MGRS inventé par le
     modèle passerait sans signal — exactement la faille qu'AD-23 ferme. Si on
     l'interdit, la commande ne sert pas sa phrase type (« convertis
     31U DQ 48251 11932 »).
  2. **Système nommé.** « en UTM 31 Nord » n'est pas sur la liste fermée : la
     garde rend une question sur une demande pourtant complète.
  3. **Sortie MGRS.** Une référence `step:` doit viser un champ déclaré
     `Point` ; une sortie MGRS ne l'est pas. L'enchaînement
     `coord.convert` → `terrain.*` / `geo.*` n'a pas de forme.
- Deux implémenteurs (registre de l'epic 11, générateur d'exemples de 5.10)
  trancheront chacun à sa façon.
- **Correction proposée :** amender AD-23 en même temps qu'AD-19 : (a) un type
  partagé `MgrsRef` (chaîne validée par motif, précision déduite), marqué dans
  le schéma comme coordonnée et soumis à la même garde de provenance
  (comparaison de chaîne normalisée à la demande) ; (b) étendre la liste fermée
  des systèmes nommés à « UTM zone N/S sur WGS84 » → 326NN/327NN ; (c) dire que
  `coord.convert` rend toujours un `Point` (pour l'enchaînement) et, en plus, la
  chaîne MGRS si demandée.

### C2 — Critique — Deferred laisse ouvert le point que l'epic 11 traverse : désignation des rasters et références d'étape rejouables

- Deferred, « Désigner un raster et enchaîner des étapes rejouables entre le
  noyau et QGIS » : « à trancher avant les entrées 12, 16 et 25 de l'epic 10 ».
  L'entrée 25 (`raster.reclassify`) part dans l'epic 11, **avant la démo**, et
  `raster.weighted_overlay`, `terrain.cost_distance`,
  `terrain.least_cost_path` enchaînent des rasters sortis du noyau
  (reclassify → overlay → coût cumulé → chemin) et des rasters de l'utilisateur
  ouverts dans QGIS.
- Le Deferred note lui-même le problème : « une référence `step:k` vers un
  fichier du noyau est journalisée comme un chemin et non comme un repère
  `out:k.n` d'AD-11 ». Sans décision, deux unités divergent : la famille
  `raster.*` aura des commandes désignées par identifiant de couche (AD-24,
  déléguées) et d'autres par chemin (AD-20, noyau), et le rejeu (AD-11)
  traitera les références d'étape au cas par cas.
- **Correction proposée :** trancher avant l'inception de l'epic 11, en AD
  (amendement d'AD-11/AD-20) : raster toujours par chemin relatif (comme
  `terrain.*`), type de chemin raster marqué dans le schéma, règle de journal qui
  écrit une référence `step:k` vers une sortie du noyau comme repère
  `out:k.n`. Mettre à jour l'échéance du Deferred (« avant l'epic 11 »), ou le
  retirer s'il est tranché.

### C3 — Haute — AD-30 contre AD-9 (amendement 4.9) : deux façons d'ouvrir une sortie du noyau

- AD-9, amendement du même jour (ticket 4.9) : après une étape `runs_in: core`
  dont la sortie est un GeoTIFF, **le noyau dicte** `layer.open_raster`
  (`dictated_by_core`, hors du modèle, journalisée, rejouée).
- AD-30 : le GeoJSON est « ouvert dans QGIS par une étape `layer.add` du
  plan » — donc composée par le modèle, avec une référence `step:` vers un
  fichier du noyau (le cas non résolu de C2).
- Divergence : `terrain.peaks` et `terrain.least_cost_path` dépendent de la
  mémoire du modèle pour s'afficher, alors que `terrain.hillshade` s'affiche
  seul ; le générateur d'exemples de 5.10 doit apprendre la différence ; un
  appel MCP ou CLI hors plan n'a pas d'étape `layer.add`.
- **Correction proposée :** aligner AD-30 sur 4.9 : après une étape du noyau
  dont la sortie est un GeoJSON, le noyau dicte une ouverture vecteur
  (`layer.add` en mode dicté, ou `layer.open_vector` `dictated_by_core`), avec
  les mêmes `origin`, `plan_step`, journalisation et rejeu ; rien dicté sans
  QGIS branché.

### C4 — Haute — `geo.*` : les systèmes acceptés ne sont pas fixés

- `geo.distance_azimuth`, `geo.destination_point`, `geo.bearing_intersection`
  prennent des points « avec EPSG explicite ». Une distance géodésique se calcule
  sur l'ellipsoïde depuis des coordonnées géographiques ; or AD-19 amendée
  interdit au noyau toute conversion hors WGS84/UTM/MGRS (Lambert-93 compris).
- Rien ne dit ce que fait `geo.*` d'un point en EPSG:2154 : refus
  (`coord.unsupported_crs` ?), calcul plan dans la projection, ou conversion
  implicite (interdite). Les quatre commandes seront écrites différemment, et
  la question remonte au modèle.
- Même trou pour `geo.bearing_intersection` : azimut vrai (géodésique) ou azimut
  de quadrillage (UTM) ; et pour l'unité d'angle « grades » (le résultat
  porte-t-il l'unité, comme l'exige la convention « Mesures » ?).
- **Correction proposée :** une phrase dans l'amendement d'AD-19 : `geo.*`
  n'accepte que 4326 et 326xx/327xx, calcule sur l'ellipsoïde WGS84 (Karney),
  rend des azimuts vrais, refuse tout autre système par une erreur nommée
  (AD-21) ; tout angle et toute distance rendus portent leur unité.

### C5 — Haute — Le catalogue « fait foi », sans lien vérifiable avec le registre (AD-6)

- Convention de nommage : « Le catalogue des commandes
  (`../spec-cassini-mvp/catalogue-des-commandes.md`) fige identifiants,
  paramètres et résultats ; il fait foi sur cette liste. » AD-6 : « Le registre
  est la seule source des commandes », schémas engendrés par `schemars`.
- Deux sources de vérité, aucune règle qui les confronte. Le fichier n'existe
  pas encore (créé par l'entrée 1 de l'epic 11) ; le spine n'en dit rien.
- **Correction proposée :** préciser dans AD-6 (ou la convention) : le
  catalogue est le contrat amont, le registre l'implémente ; un test de la
  recette compare identifiants, `runs_in` et noms de paramètres du registre au
  catalogue (commandes réservées exclues), comme AD-24 confronte la table au
  registre. Mentionner que le fichier naît à l'entrée 1 de l'epic 11.

### C6 — Moyenne — AD-20 amendée : `raster.align` arrive après la démo, et la comparaison des grilles n'est pas fixée

- L'alignement est renvoyé à `raster.align`, commande de l'epic 10 (après la
  démo). Entre l'epic 11 et l'epic 10, `raster.weighted_overlay` refuse tout
  jeu de rasters non alignés sans voie de sortie dans Cassini : la tentation de
  rééchantillonner « juste pour la démo » est exactement la divergence que
  l'amendement interdit.
- « Grilles identiques (système, taille de cellule, origine, dimensions) » :
  égalité stricte ou tolérance sur des flottants ? Deux commandes écriront deux
  comparaisons. NoData non traité : propagation, barrière (coût), ou zéro
  (superposition) — trois commandes, trois choix possibles. Coût négatif ou nul
  dans `terrain.cost_distance` : erreur nommée ?
- **Correction proposée :** une seule fonction de comparaison de grilles dans
  le module raster, tolérance déclarée (par ex. 1e-9 × taille de cellule) ;
  règle NoData unique (propagée en sortie, infranchissable pour le coût) ; coût
  ≤ 0 → erreur nommée. Pour `raster.align`, soit l'avancer dans l'epic 11, soit
  écrire que la démo ne montre la superposition que sur des rasters déjà
  alignés (Deferred explicite).

### C7 — Moyenne — AD-30 : déterminisme et rejeu non dits ; ordre des axes

- AD-26 exige un CSV « déterministe » ; AD-30 ne le dit pas, alors que la liste
  bornée du résultat (points hauts) est comparée au rejeu (AD-11) : ordre des
  ex aequo, arrondi des flottants, ordre des membres.
- Le membre `crs` en URN `urn:ogc:def:crs:EPSG::4326` implique, au sens OGC,
  l'ordre latitude/longitude ; GDAL lit l'ordre x/y traditionnel, d'autres
  lecteurs non.
- **Correction proposée :** « écriture déterministe (tri défini, ex aequo
  départagés par rang de cellule) ; coordonnées en ordre x, y (est, nord ou
  longitude, latitude) quel que soit le système ». Préciser aussi « dossier de
  travail » (terme d'AD-10) au lieu de « dossier de sortie ».

### C8 — Moyenne — Table de correspondance et Binds incomplets

- Ligne CAP-15 : manquent AD-23 (garde de coordonnées, centrale pour `coord.*`
  et `geo.*`), AD-6 (`runs_in: core`, schémas engendrés), AD-11 (journal et
  rejeu des sorties du noyau), AD-1 et AD-16 (critère « sans GPL » de CAP-15),
  AD-9 (ouverture dictée des rasters, amendement 4.9).
- Ligne CAP-5 : l'ajout de la spec (« format, orientation, carroyage en
  option ») n'est gouverné par aucune AD nouvelle. Points de divergence non
  fixés : l'énumération `grid` doit être neutre (AD-22 — pas de nom de classe
  QGIS) ; une valeur d'énumération indisponible sur une série de QGIS (« MGRS si
  QGIS le permet ») n'est pas couverte par AD-25, qui annonce des commandes et
  non des valeurs. Proposer : ajouter AD-22 et AD-25 à la ligne CAP-5, et
  décider qu'une valeur d'énumération est fixe dans le catalogue — si le
  carroyage MGRS n'est pas vérifié dans la documentation de QGIS, il n'entre pas
  dans l'énumération.
- Binds à compléter : AD-8, AD-10, AD-22, AD-23 et AD-1 ne citent pas CAP-15
  alors que leurs règles la gouvernent.

### C9 — Basse — Deferred et sources obsolètes

- Deferred « Les six autres commandes de terrain […] hors périmètre du MVP par
  la spec » : faux depuis le 2026-11-05 (exposition, ombrage, intervisibilité
  multi-points, points dominants passent à CAP-15). À réécrire : « hydrologie et
  couverture radio, identifiants réservés ».
- MGRS au-delà de 84° N et 80° S relève d'UPS, non cité : écrire « refusé par
  une erreur nommée » pour fermer la question.
- AD-8 parle de GeoJSON « (RFC 7946) » dans le canal alors que les points y
  portent un EPSG quelconque (AD-19) et qu'AD-30 assume l'écart : harmoniser la
  mention.
- L'écart d'AD-30 à §4.4 du changement de cap (RFC 7946 → GeoJSON 2008) est
  justifié dans le spine ; le signaler au changement de cap pour que la source
  ne contredise pas le contrat.
- Spec, hors spine : CAP-14 cite encore « reclassement d'un raster » parmi les
  primitives exécutées par QGIS, alors que CAP-15 le place dans le noyau.

## Ce qui tient

- AD-19 amendée : exception bornée et vérifiable (liste d'EPSG explicite,
  aucune conversion de couche, conversion toujours visible comme étape du
  plan). Conforme à AD-1/AD-16 (pas de PROJ, formules publiées).
- AD-20 amendée : un seul module raster, refus du rééchantillonnage, erreur
  nommée conforme à AD-21.
- AD-30 : une seule forme de sortie vecteur, chemin décidé par le noyau
  (AD-10), résultat borné (AD-8), aucune dépendance nouvelle (AD-16). Le choix
  GeoJSON 2008 avec membre `crs` est cohérent avec « le noyau ne reprojette
  pas ».
- AD-8 : la borne sur les observateurs et sources est vérifiable par le
  contrôle de schéma existant.
- Aucun amendement ne touche AD-22 (paramètres neutres) ni AD-24 à la lettre.
