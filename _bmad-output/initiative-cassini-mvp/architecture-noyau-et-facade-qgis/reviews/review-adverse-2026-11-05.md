# Relecture adverse — amendements du 2026-11-05 (AD-8, AD-19, AD-20, AD-30, conventions, CAP-15)

Date : 2026-11-05. Objet : `git diff HEAD` de la colonne d'architecture, au regard de la liste de
commandes visée (`change-geometrie-et-terrain-avant-la-demo.md` §4.1), avant le découpage de
l'epic 11.

Lentille : construire, un niveau plus bas, deux tickets de l'epic 11 confiés à deux développeurs
qui ne se parlent pas, chacun fidèle à la lettre de toutes les AD, et qui pourtant ne
s'assemblent pas. Chaque paire est un trou ; chaque trou reçoit le texte de règle minimal qui le
ferme.

État vérifié dans le code : `core/src/point.rs` définit `Point { x: f64, y: f64, epsg: u32 }`
(marque `x-cassini: point`) et la liste fermée des systèmes nommés (Lambert-93, WGS84, plus une
reconnaissance des zones UTM). `catalogue-des-commandes.md` n'existe pas encore : la convention
« il fait foi » renvoie à un fichier vide de contenu, donc rien n'y fige aujourd'hui les formes
ci-dessous.

## Verdict

**À resserrer avant le découpage de l'epic 11.** Les trois amendements tiennent leur intention
(pas de reprojection de couche, pas de rééchantillonnage, une seule sortie vecteur), mais ils
laissent ouvertes les formes de données que deux tickets partagent : la représentation d'un point
MGRS ou UTM face au type `Point` d'AD-23, l'unité et la référence d'un angle, la façon de désigner
un raster qui passe de QGIS au noyau (question déjà ouverte le 2026-11-04, aggravée par
`raster.align`), le contenu et l'ouverture du GeoJSON d'AD-30, et le modèle de coût. Sept trous ;
les trois premiers sont bloquants.

---

## T1 — Un point MGRS n'a pas de forme au registre (bloquant)

**Paire.** Ticket A, `coord.convert` ; ticket B, `geo.distance_azimuth` et
`terrain.viewshed_cumulative`.

- A, fidèle à AD-19 (« conversion entre WGS84, UTM et MGRS ») et à AD-23 (« toute coordonnée est
  le type `Point` »), constate que MGRS n'a pas de code EPSG. Il prend l'entrée MGRS dans un champ
  texte `mgrs: "31UDQ4825011745"` et rend `{wgs84: Point, utm: Point, mgrs: String}`. Le contrôle
  de contrat ne refuse que les champs `x`/`y` numériques hors de `Point` : le texte passe, **et le
  garde-fou des coordonnées d'AD-23 ne le voit pas** — le modèle peut inventer une référence MGRS
  sans déclencher la question.
- B accepte un `Point` dont l'`epsg` vaut 4326 ou 326xx/327xx, et code une zone UTM comme
  `epsg` ; un autre lecteur de la même règle encoderait MGRS par un pseudo-code (`epsg: 0`) pour
  rester dans `Point`.
- Conséquences : (1) une étape `geo.*` ne peut pas référencer par `step:` la sortie MGRS de A,
  qui n'est pas un champ `Point` ; (2) côté demande, le garde-fou exige x, y et système comme
  nombres : une phrase qui ne donne qu'une référence MGRS est **toujours** refusée par lui chez B,
  **jamais** contrôlée chez A ; (3) la précision MGRS (1 à 5 chiffres) est tronquée par la
  convention NGA, arrondie par l'intuition : deux tickets rendent des carreaux voisins.

**Règle proposée (AD-19, amendement resserré ; AD-23, sous-règle « Un seul type » complétée).**
> Une coordonnée MGRS est le type partagé `MgrsRef` du noyau — une chaîne normalisée (majuscules,
> sans espace) et sa précision en chiffres (1 à 5) —, marqué `x-cassini: mgrs` dans le schéma
> engendré ; elle n'est jamais un `Point` ni un champ texte libre. Seul `coord.convert` la lit ou
> la rend. Une zone UTM s'exprime par son code EPSG (32601–32660, 32701–32760), jamais par un
> champ « zone » séparé. Le garde-fou d'AD-23 s'applique à `MgrsRef` : la chaîne normalisée figure
> telle quelle dans la demande, ou vient d'une référence `step:` vers un champ `MgrsRef`. La
> liste fermée des systèmes nommés gagne « UTM zone *nn* N/S » → 326nn/327nn. Un `Point` en
> EPSG:4326 porte la longitude en `x` et la latitude en `y`. La conversion vers MGRS **tronque**
> (NGA) ; hors de 80° S–84° N (zones UPS), elle échoue en `coord.out_of_range`. `coord.convert`
> rend toujours un `Point` par système UTM ou WGS84 demandé, pour que toute étape suivante puisse
> le référencer.

## T2 — Angles : unité, référence et origine non fixées (bloquant)

**Paire.** Ticket A, `geo.distance_azimuth` et `geo.bearing_intersection` ; ticket B,
`terrain.aspect` et `terrain.hillshade`.

- §4.1 ne donne une unité d'angle (degrés, grades) qu'à `geo.distance_azimuth`. A lit « grades »
  comme le gon (400) ; un autre lecteur pourrait attendre une troisième unité, que la liste ne
  nomme pas. `geo.destination_point` et `geo.bearing_intersection`
  n'ont pas d'unité : A les prend en degrés, ou réutilise le paramètre facultatif de
  `distance_azimuth`.
- A calcule un azimut **géodésique vrai** (Karney, AD-19 l'autorise) ; B, qui travaille sur un MNT
  projeté, calcule une exposition par rapport au **nord du quadrillage**. En Lambert-93 à
  Villeneuve-le-Roi l'écart est faible, en UTM en bord de fuseau il dépasse 2° : un recoupement
  fait à partir d'expositions lues sur la carte se trompe sans signal.
- B suit la convention de GDAL pour les cellules plates (−9999) ou le nodata du raster ; un autre
  choisit −1. `terrain.hillshade` prend l'azimut du soleil compté depuis le nord, horaire ; rien ne
  l'écrit.
- `geo.bearing_intersection` sur deux `Point` UTM : A l'implémente par une droite plane, un autre
  par une intersection géodésique ; les deux résultats diffèrent de plusieurs mètres à 20 km.

**Règle proposée (AD nouvelle, AD-31 — Un seul type d'angle et une seule référence d'azimut).**
> Tout angle du registre est le type partagé `Angle { value: f64, unit: deg | gon }`, défini une fois dans le noyau ; aucune commande ne porte un
> angle nu ni sa propre énumération d'unités. Tout azimut est compté depuis le nord **vrai**, dans
> le sens horaire, dans [0, tour). Le nord du quadrillage n'est jamais implicite : une commande
> qui le rend le nomme (`grid_azimuth`) et rend la convergence. Toute distance est en mètres,
> géodésique sur l'ellipsoïde WGS84 quel que soit le système d'entrée (un `Point` UTM est d'abord
> ramené en géographiques par le module de coordonnées) ; `geo.bearing_intersection` est
> géodésique. Les rasters d'angle (`terrain.aspect`, `terrain.slope`) sont en degrés, origine
> nord du quadrillage du MNT, sens horaire ; une cellule plate vaut le nodata déclaré du raster
> rendu, jamais une valeur sentinelle propre à une commande.

## T3 — Un raster qui passe de QGIS au noyau n'a pas de forme (bloquant)

**Paire.** Ticket A, `raster.weighted_overlay` (noyau, AD-20 : lit des fichiers) ; ticket B,
`raster.align` (délégué, epic 10, ligne de table, AD-24 : sortie de genre `layer`, raster
désigné par identifiant de couche).

- L'amendement d'AD-20 fait de `raster.align` **le** geste qui rend deux grilles compatibles. A
  attend des chemins relatifs, B rend un `layer_id`. Le plan « aligner puis superposer » que la
  règle prescrit **ne se compose pas**. Si B choisit plutôt une sortie `file`, il rend un chemin
  `out:k.n` que la ligne d'AD-11 journalise comme repère, mais rien ne dit qui ouvre ce raster
  dans QGIS (l'amendement 4.9 d'AD-9 ne dicte `layer.open_raster` qu'après une étape
  `runs_in: core`).
- C'est la question ouverte du 2026-11-04 (« Désigner un raster et enchaîner des étapes
  rejouables entre le noyau et QGIS »), qui bloquait les entrées 12, 16 et 25 de l'epic 10 ;
  l'epic 11 en dépend désormais aussi, alors qu'elle passe avant la démo.
- Second propriétaire : l'entrée 10.25 passe en `dropped`, mais rien n'empêche une ligne
  `raster.reclassify` de survivre dans la table fermée (AD-24) ; si l'extension l'annonce à
  `session.hello` alors que le registre dit `runs_in: core`, AD-6 (« jamais les deux ») est
  violée sans test qui le voie. Le sens des bornes de classe (`[min, max)` ou `(min, max]`) et le
  sort des cellules hors classe (nodata ou valeur d'origine) sont aussi laissés à chaque ticket —
  `native:reclassifybytable` en offre plusieurs.

**Règle proposée (AD-20, second amendement ; AD-24 « Genres » resserrée ; ferme la question
ouverte).**
> Un raster, dans tout paramètre du registre, est le type partagé `RasterPath` (chemin relatif au
> projet, marqué `x-cassini: raster`), jamais un identifiant de couche ; une couche raster du
> projet se désigne par le chemin de sa source que rend `project.inventory`. Toute commande qui
> produit un raster, noyau ou déléguée, l'écrit en GeoTIFF au chemin `out:k.n` décidé par le
> noyau (AD-10) et le rend comme sortie de genre `file` ; le noyau dicte ensuite
> `layer.open_raster`, quel que soit le `runs_in` de l'étape (généralisation de l'amendement 4.9
> d'AD-9). La recette vérifie que les identifiants de la table fermée sont exactement les
> commandes `runs_in: qgis` du registre. Le reclassement prend des classes `[min, max)`, la
> dernière fermée ; une cellule hors de toute classe devient nodata ; ces deux règles sont dans
> le schéma Rust.

## T4 — Le GeoJSON d'AD-30 : contenu, liste bornée et ouverture non fixés (majeur)

**Paire.** Ticket A, `terrain.peaks` ; ticket B, `terrain.least_cost_path`.

- **Noms des propriétés.** A écrit `elev`, `prominence`, `rank` ; B écrit `z`, `cost`. Aucune
  règle ne fixe les noms, les unités ni l'ordre ; le panneau, l'agent (AD-27) et une étape
  `analysis.*` qui lirait le champ cassent selon le ticket.
- **Liste bornée du résultat.** AD-8 dit que les géométries du canal sont du GeoJSON RFC 7946
  (donc WGS84) ; AD-30 dit que le fichier est en GeoJSON 2008 dans le système du raster. A rend sa
  liste bornée en `Feature` RFC 7946 — qu'il ne peut pas produire sans reprojeter, ce qu'AD-19
  interdit — ; B la rend en `Point` (AD-23). Les deux obéissent à une AD.
- **Sens et forme de la ligne.** B part du départ ou de l'arrivée selon le sens de son
  rétro-parcours ; sommets au centre des cellules ou aux coins ; avec ou sans Z.
- **Ouverture.** AD-30 veut une étape `layer.add` **du plan** ; l'amendement 4.9 d'AD-9 fait au
  contraire **dicter par le noyau** l'ouverture d'un GeoTIFF du noyau, hors du modèle. A laisse le
  modèle composer `layer.add` avec une référence `step:k` vers le chemin — exactement la référence
  qu'AD-11 ne sait pas journaliser comme repère (question ouverte du 2026-11-04) — ; B suit 4.9.
  Deux voies d'ouverture pour la même sorte de sortie du noyau.
- **Axes.** Avec `urn:ogc:def:crs:EPSG::4326`, l'ordre des axes est latitude puis longitude par
  l'autorité EPSG ; le pilote GeoJSON de GDAL lit en ordre SIG traditionnel. Sans objet tant que le
  raster source est projeté, mais un MNT en 4326 suffit à faire basculer les points. À vérifier
  dans la documentation du pilote avant d'écrire le module.

**Règle proposée (AD-30 resserrée ; AD-8 précisée).**
> Le noyau écrit ses GeoJSON par un seul module (`vector_out`), qui prend un type Rust par commande
> pour les propriétés : noms en snake_case anglais, unité dans le nom (`elevation_m`,
> `prominence_m`, `cost_total`, `length_m`), `rank` entier à partir de 1, coordonnées 2D,
> sommets au centre des cellules, ligne orientée du départ vers l'arrivée, entités triées de façon
> déterministe (rang, puis x, puis y), nombres arrondis à une précision fixée par le module. Le
> catalogue reprend ces types. La liste bornée du résultat du canal est une liste de `Point`
> (AD-23) avec les mêmes propriétés, jamais du GeoJSON : la mention RFC 7946 d'AD-8 ne vaut que
> pour une géométrie fournie **par l'utilisateur ou QGIS** en WGS84. L'ouverture d'un GeoJSON du
> noyau est **dictée par le noyau** (`layer.add` portant `dictated_by_core` ou
> `layer.open_vector`), comme `layer.open_raster` en 4.9, jamais composée par le modèle. Un MNT
> en système géographique est refusé par les commandes qui rendent des entités
> (`raster.geographic_crs`) tant que l'ordre des axes n'est pas vérifié.

## T5 — Rejeu d'une commande du noyau qui écrit un GeoJSON (majeur)

**Paire.** Ticket A, `terrain.least_cost_path` (écrit `out:k.1.geojson`) ; ticket B, le rejeu
(AD-11) de ce journal **dans la même session**.

- AD-11 ne dit pas comment se compare une sortie **du noyau** : seul le cas des sorties de QGIS
  (« octets non comparés ») est écrit. A, voulant un rejeu fidèle, compare l'empreinte du fichier ;
  B compare le résultat typé seulement. Un ordre d'entités non déterministe (T4) rend le premier
  faux sans faute de calcul.
- Le rejeu réécrit au même chemin `out:k.n` ; la couche ouverte par l'exécution d'origine tient le
  fichier. Sous Windows, l'écriture échoue (AD-10 le dit pour la suppression) ; A écrit dans un
  temporaire et renomme, ce qui échoue aussi ; B demande à l'extension de libérer la couche, mais
  aucun geste n'est nommé pour une couche vecteur.

**Règle proposée (AD-11, complément).**
> Une sortie fichier écrite par le noyau est déterministe : à entrées égales, mêmes octets. Le
> rejeu compare son empreinte (SHA-256 consignée dans la ligne du journal) et signale l'écart
> comme une différence nommée, sans refuser. Avant de réécrire un `out:k.n` qu'une couche de la
> session peut tenir ouvert, le noyau dicte sa libération et son retrait (AD-10), pour un raster
> comme pour un vecteur ; à défaut, le rejeu écrit dans un sous-dossier de rejeu daté du dossier
> de travail, et lie les repères à ces nouveaux chemins.

## T6 — Modèle de coût et unités (majeur)

**Paire.** Ticket A, `terrain.cost_distance` ; ticket B, `terrain.least_cost_path`.

- A lit la valeur de cellule comme un coût **par mètre** parcouru (multiplié par la taille de
  cellule, ×√2 en diagonale, moyenne des deux cellules) ; B comme un coût **par cellule**. A est
  en 8 voisins, B en 16. Le coût total rendu par B ne vaut pas la valeur du raster cumulé de A à
  l'arrivée.
- Nodata et coût ≤ 0 : obstacle chez A, erreur chez B.
- Un raster de coût en système géographique : taille de cellule en degrés, coût faux sans signal.
- `raster.weighted_overlay` : poids normalisés ou non, masque d'exclusion à 0 ou à nodata.

**Règle proposée (AD-20, troisième point de l'amendement).**
> Un seul module de coût dans le noyau, dont `cost_distance` et `least_cost_path` sont deux
> lectures : valeur de cellule = coût par mètre, finie et ≥ 0 ; nodata = infranchissable ; 8
> voisins ; coût d'un pas = moyenne des deux cellules × longueur du pas (taille de cellule, ×√2 en
> diagonale) ; `least_cost_path` remonte le raster cumulé et son coût total égale la valeur
> cumulée à l'arrivée (test). Un raster de coût ou de critères en système géographique est refusé
> (`raster.geographic_crs`). Dans `weighted_overlay`, les poids sont rendus à une somme de 1 par
> le noyau, et une cellule masquée devient nodata.

## T7 — Un point MGRS ne peut pas rejoindre un MNT en Lambert-93 (majeur)

**Paire.** Ticket A, `coord.convert` (sorties WGS84, UTM, MGRS seulement, AD-19) ; ticket B,
`terrain.viewshed_cumulative` (exige le système du MNT, AD-19), sur le MNT IGN de la démo, en
EPSG:2154.

- A ne peut pas rendre du 2154 ; B refuse tout autre système ; `data.reproject` ne traite que des
  couches. Le plan « observateur donné en MGRS, champ de vision sur le MNT IGN » est impossible,
  alors que c'est le cas d'usage qui motive l'amendement. Chacun des deux tickets est conforme.

**Règle proposée (AD-19, amendement complété).**
> La conversion d'un `Point` vers ou depuis un système autre que WGS84 et UTM est une commande
> déléguée, `coord.transform_point` (`runs_in: qgis`, bornée à une liste de points par son
> schéma, AD-8), étape visible du plan, exécutée par le moteur de QGIS ; le noyau n'en porte
> aucune formule. Ou bien, si le propriétaire refuse cette commande : la limite est écrite dans
> le catalogue, et `terrain.*` refuse en nommant la sortie (`terrain.crs_mismatch`, message :
> reprojeter le MNT en UTM par `data.reproject` ou `raster.align`).

---

## Ordre de fermeture conseillé

1. T3 d'abord : il ferme la question ouverte du 2026-11-04 et débloque à la fois l'epic 10
   (entrées 12, 16) et l'epic 11.
2. T1, T2 et T7 ensemble : un seul amendement d'AD-19 et une AD-31 pour les types partagés
   (`MgrsRef`, `Angle`), à écrire avant l'entrée 1 de l'epic 11 qui fige le catalogue.
3. T4, T5, T6 : resserrements d'AD-30, AD-11 et AD-20, à reporter dans le catalogue.

Chaque règle ci-dessus est un **type Rust partagé ou un module unique** du noyau : c'est la forme
qui empêche deux tickets de diverger, puisque les schémas sont engendrés (AD-6).
