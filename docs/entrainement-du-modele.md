# Entraînement du modèle de composition

Le ticket 5.10 ajuste un petit modèle pour qu'il transforme une phrase française en plan
de commandes du registre. Ce document décrit ses quatre tâches : le jeu d'exemples engendré,
le jeu tenu à l'écart qui le mesure et le score de référence (tâche 1), l'entraînement
reproductible et la fusion en GGUF (tâche 2), le branchement du modèle ajusté dans `ai/`
(tâche 3), puis l'inventaire vu par le modèle dans QGIS et les mesures (tâche 4, en fin de
document).

## Régénérer le jeu d'exemples

Une seule commande, depuis la racine du dépôt :

```
cargo run -p cassini-training --release -- --seed 1 --count 8000 --out exemples.jsonl
```

- `--seed` (par défaut 1) : la graine. Même graine, même fichier, à l'octet près, sous
  Windows comme sous Linux.
- `--count` (par défaut 8000 depuis 5.15, 5000 avant) : le nombre d'exemples.
- `--out` : le fichier écrit. Le jeu n'est pas versionné : il se régénère.

Le résumé par famille (exemples gardés, écartés, doublons) s'imprime sur la sortie
d'erreur. La commande échoue si elle n'atteint pas le nombre demandé.

Chaque ligne du fichier est un enregistrement JSON :

```
{"inventory": {...}, "request": "...", "expected": {...}}
```

- `inventory` : l'inventaire d'un projet synthétique (couches, champs et valeurs,
  fichiers non ouverts), de la même forme que celui de l'épreuve du modèle local.
- `request` : la demande en français.
- `expected` : le plan attendu, sous la forme exacte que rend l'agent —
  `{"kind":"calls","calls":[...]}`, `{"kind":"question","question":"..."}` ou
  `{"kind":"refusal","reason":"..."}`.
- `retry` (exemples de redemande seulement, depuis 5.15) : `{"rejected": "...", "errors": [...]}`,
  la sortie rejetée et ses écarts (voir « La redemande » plus bas).

Ce format structuré n'écrit aucun texte de prompt. Le format `--format chat` de la
tâche 2 (plus bas) écrit, lui, le texte exact vu par le modèle.

## Comment un exemple est fait

Le générateur vit dans le crate `training/` (`cassini-training`, jamais publié). Il
dépend du noyau (`cassini-core`) et pas de la couche IA. Le hasard vient d'un petit
générateur pseudo-aléatoire écrit à la main (`training/src/rng.rs`) : aucune dépendance
n'a été ajoutée.

1. Une famille tire un inventaire : les couches dont la phrase a besoin, puis des couches
   de distraction et des fichiers non ouverts. Deux couches d'un même groupe (par exemple
   « Routes » et « Réseau routier ») ne se retrouvent jamais ensemble : « une route »
   deviendrait ambigu.
2. Elle écrit la demande à partir de gabarits à emplacements : tournure d'ouverture,
   distance, valeur de champ, accord des participes, ponctuation, politesse, et parfois
   une commune citée comme simple contexte (« à Orly », seule forme depuis 5.15).
3. Elle construit le plan attendu en suivant les règles de chaînage du prompt : le filtre
   sur un champ avant le tampon ou l'intersection, le tampon sur la couche de référence,
   puis des intersections successives dont la première porte la couche cherchée.
4. L'exemple n'est écrit que s'il passe `Registry::check_plan` du registre réel, et
   quelques gardes de plus : les couches citées existent dans l'inventaire, le fichier
   ouvert est parmi les fichiers non chargés, une question finit par « ? », un refus n'est
   jamais formulé en question. Un exemple écarté est compté dans le résumé, jamais écrit.
   Depuis 4.6, le `dem` d'une commande de terrain doit être la `source` d'une couche raster de
   l'inventaire ; le MNT qu'un plan cite garde toujours sa `source` (jamais la forme « couche
   temporaire » de QGIS), son `epsg` pouvant manquer comme ailleurs.

Les thèmes de couches sont tous civils : urbanisme (parcelles, zonage du PLU, bâtiments,
espaces verts), environnement (rivières, zones humides, zones inondables, ZNIEFF,
captages, forêts, arbres), réseaux (gares, arrêts de bus, routes, pistes cyclables,
lignes électriques, canalisations, parkings), équipements et SIG général (écoles,
hôpitaux, pharmacies, commerces, communes, quartiers). Les valeurs imitent les
standards publics et ne sont tirées d'aucun fichier réel.

## Les familles

| Famille | Ce qu'elle enseigne |
|---|---|
| `filtre_egalite` | « les gares de la ligne D », « les routes nationales » → sélection par attribut |
| `garde_seulement` | « garde seulement », « ne garde que », « conserve uniquement » → une sélection, jamais une suppression |
| `filtre_numerique` | « bâtiments de plus de 12 m de haut », « communes de moins de 12 500 habitants » → `gt`, `lt`, `gte`, `lte` |
| `filtre_texte` | « dont le nom contient », « commence par » → `contains`, `begins_with` |
| `filtre_different` | « sauf celles de la section AB », « qui ne sont pas » → `ne` |
| `tampon` | zone tampon en m ou en km, fusionnée sur demande seulement |
| `tampon_filtre` | tampon autour d'une couche filtrée |
| `distance` | « à moins de 500 m d'une gare » → tampon sur la référence, puis intersection |
| `distance_reference_filtree` | distance à une référence filtrée (« des gares de la ligne D ») |
| `dans_une_zone` | « en zone N », « en zone inondable d'aléa fort » → sélection de la zone, puis intersection |
| `intersection` | « situés dans une zone humide », « croise la couche A avec la couche B » |
| `conditions_multiples` | zone et distance, cible filtrée et distance, deux distances, inclusion dans une couche entière : intersections successives |
| `constructible` | vocabulaire d'urbanisme : « parcelles constructibles », avec ou sans distance et PDF |
| `export_pdf` | « sors-moi la carte en PDF », titre donné par l'utilisateur ou déduit de la demande |
| `export_pdf_scale` | « exporte la carte en PDF au 1:25 000 » → `scale: 25000`, l'entier écrit ; échelles courantes de 1:1 000 à 1:250 000, écrites « au 1:25 000 », « au 1/25000 », « au 25 000e », « au 1:25k » (multiple de 1 000 seulement) ou « à l'échelle 1/25 000 » ; la carte seule ou au bout d'une recherche (5.38) ; depuis 5.40, aussi sans le mot « PDF » (« imprime la carte au 1/50000 », « sors-moi la carte au 25 000e »), un export et jamais un refus de format |
| `export_pdf_format` | « exporte la carte en PDF au format A3 » → `page_size: "a3"`, « en portrait » ou « à la française » → `orientation: "portrait"` (« paysage », « à l'italienne » → `landscape`), « avec un carroyage Lambert-93 tous les 1 km » → `grid: {kind: projected, epsg: 2154, interval: {value: 1, unit: km}}`, « avec un graticule tous les 0,01° » → `grid: {kind: degrees, interval: {value: 0.01, unit: deg}}` (grades → `gon`) ; seuls les paramètres écrits, jamais `scale` ; la carte seule ou au bout d'une recherche ; un carroyage « UTM » sans fuseau (« carroyage UTM au kilomètre ») → `epsg` = `project.utm_epsg` de l'inventaire, question quand il n'y en a pas ; un carroyage sans système (« quadrillage tous les 500 m ») → question qui demande le système (11.12, lot 3 ; fuseau de l'inventaire, 2026-11-08 ; renforcé par `carroyage_utm_projet` en 11.23) |
| `carroyage_utm_projet` | le carroyage UTM d'un export, le fuseau du projet en main (11.23) : « avec un carroyage UTM tous les 2 km », « avec une grille UTM de 1 km », « avec un quadrillage UTM au kilomètre » → `grid` projeté sur `project.utm_epsg`, recopié ; sans `utm_epsg`, la question du système ; un fuseau écrit l'emporte même quand l'inventaire en porte un autre, dans les deux sens (« UTM 31N » avec 32632, « UTM 32N » avec 32631) ; aucun système → la question, jamais un refus (constat F03 de 11.16) ; depuis le second essai de 11.23, poids 4, trois recopies pour deux questions sans `utm_epsg` (constat F03 du premier essai : fuseau inventé) ; jamais la tête « Fais un PDF de la carte » (celle de F03) |
| `isochrone` | « zone atteinte en 10 minutes à pied depuis le point ... (Lambert-93) » |
| `isochrone_utm` | « zone atteinte en 10 minutes à pied depuis le point 412 300 / 5 412 000 (UTM 31N) » → même isochrone, code 32631 ou 32632 ; zone 31N ou 32N, écrite « UTM 31N », « UTM 31 N », « UTM zone 31 Nord » ou « UTM31N » (5.37) |
| `isochrone_vers_cible` | « les écoles à 10 min à pied du point ... (Lambert-93) » → isochrone, puis intersection de la cible avec l'étape 1 ; la couche du lieu (gares, écoles...) est souvent dans l'inventaire, seules les coordonnées donnent l'origine ; une fois sur trois, le lieu est nommé avec ses coordonnées (« de la gare de X (652 000 / 6 862 000, Lambert-93) ») et reste un plan |
| `decompte_par_zone` | « nombre d'écoles par commune », nom des zones sur demande seulement |
| `ouverture` | ouvrir un fichier non chargé, CSV à coordonnées compris |
| `description` | décrire une couche, inventaire du projet, version, modèles installés |
| `question_couche_absente` | une couche dont rien ne parle dans l'inventaire → question |
| `question_champ_absent` | un critère qu'aucun champ ne porte → question, jamais un champ voisin deviné |
| `refus` | courriel, suppression, export de couche, couleur choisie à la main (« mets les gares en rouge » : depuis 11.23, le refus dit que le catalogue ne pose pas de couleur choisie mais sait les couleurs topographiques et le dégradé, jamais qu'aucune commande ne change le style), itinéraire sur le réseau routier entre deux objets d'une couche (depuis 11.12, lot 2b-1, le refus dit que le chemin de moindre coût demande un raster de coût et deux points en coordonnées), téléchargement, 3D → refus affirmatif |
| `refus_hors_registre` | cadrage ou zoom de la carte, PDF dans un format hors de la série du catalogue (A5, lettre US), ou envoyé à une imprimante ou avec un nombre d'exemplaires (« envoie la carte à l'imprimante », « en 3 exemplaires ») ; « imprime la carte » seul reste un export, PDF qui nomme les couches à montrer (« fais une carte PDF A3 des écoles et des gares », depuis 11.23 : l'export met en page la carte affichée sans choisir ses couches, même avec un format ou une orientation connus), altitude moyenne par zone (statistiques sur un raster ; depuis 5.41 aussi après une pente ou un champ de vision : « la pente puis sa moyenne par commune » est un refus entier), MNT présent pour l'altitude par zone, trajet depuis plusieurs lieux (« de chaque gare », « de toutes les gares », « des gares ») → refus affirmatif, jamais un plan de substitution. Le point de terrain écrit dans un autre système que le MNT, refusé en 5.41, est un plan depuis 11.12 (`terrain_autre_systeme`). Le champ de vision, la ligne de vue et la pente n'y sont plus depuis 4.6 : le registre les a. Le seuil sur la pente (« où la pente dépasse 15 % »), refusé de 4.6 à 11.12, est un plan depuis 11.12, lot 2a (`pente_seuil`). Le PDF en A3, A0 ou portrait, refusé jusqu'au lot 3 de 11.12 (« seulement une page A4 paysage »), est un plan depuis (`export_pdf_format`) |
| `pente` | « fais la carte des pentes du MNT en pourcentage » → `terrain.slope`, `dem` recopié de la `source` du raster de l'inventaire, `unit` telle qu'écrite (`degrees` : « en degrés », « en ° » ; `percent` : « en pourcentage », « en % », « en pour cent ») (4.6) |
| `ligne_de_vue` | « y a-t-il une ligne de vue entre le point (... ; ...) en Lambert-93 et le point (... ; ...) en Lambert-93, œil à 2 m » → `terrain.line_of_sight`, deux points écrits en Lambert-93 ou en UTM 31N/32N, le système écrit à chaque point (le garde-fou en demande une occurrence par point), le MNT dans ce même système ; `observer_height` et `target_height` seulement s'ils sont écrits, jamais courbure ni réfraction (4.6) ; depuis le second essai de 11.23, une fois sur quatre, une même hauteur pour les deux bouts (« , 2 m de hauteur de chaque côté », « observateur et cible à 5 m du sol », « de part et d'autre ») → les deux hauteurs (constat S21) |
| `champ_de_vision` | « calcule le champ de vision depuis le point ... (UTM 31N) sur 2 km » → `terrain.viewshed`, même règle de points et de MNT ; hauteurs et `radius` seulement s'ils sont écrits (4.6) ; depuis 5.41, le rayon aussi avant le point (« dans un rayon de 800 m autour du point … ») ; depuis 11.23, une fois sur six, un lieu nommé suivi de ses coordonnées (« depuis le château d'eau de Thiais (652 000 ; 6 862 000, Lambert-93) ») reste un plan, jamais une question (constat S20 de 11.16) |
| `question_terrain` | demande de terrain sans MNT dans l'inventaire (un raster de coût peut y être depuis 11.12, lot 2b-1) → question qui demande d'ouvrir ou de nommer le MNT ; vue depuis un lieu nommé sans coordonnées (« depuis le clocher de Thiais ») ou ligne de vue avec un seul point écrit → question qui demande le point en X, Y et système ; pente sans unité → question « degrés ou pourcentage » (4.6) |
| `appartenance_commune` | « qui appartiennent à la commune de X », « sur le territoire de X » → sélection de la commune par son nom, puis intersection ; sans couche de communes, ou sans nom de commune, une question. Une commune citée seule (« à Orly ») reste un contexte sans effet |
| `question_isochrone` | isochrone ou cible à une durée de trajet d'un lieu nommé (« depuis la mairie de X », « les parcelles à 10 min à pied de la gare de X »), sans coordonnées → question qui demande le point en X et Y avec son système ; « d'une gare » → question qui demande laquelle |
| `etape_unique` | une seule opération demandée, une autre couche citée pour le contexte (« je m'occuperai des écoles plus tard ») → une étape |
| `conversion_coordonnees` | « donne la référence MGRS du point 48,8566 N, 2,3522 E (WGS84) », « convertis 31U DQ 52000 12000 en latitude et longitude » → `coord.convert` entre WGS84, UTM et MGRS, une ou deux formes demandées ; `mgrs_precision` (« à 3 chiffres ») et `utm_zone` (« dans le fuseau 32 ») seulement s'ils sont écrits ; un point en Lambert-93 vers l'UTM ou le MGRS → `data.transform_point` vers 4326, puis `coord.convert` sur `step:1.point` (11.12) ; depuis 11.23, un point UTM vers le fuseau voisin (« … (UTM 31N) en UTM dans le fuseau 32 ») → `coord.convert` vers `utm`, `utm_zone` écrit (report de 11.16) |
| `reprojection_point` | « reprojette le point … (Lambert-93) en WGS84 », « donne les coordonnées du point … (UTM 31N) en Lambert-93 », « … en Web Mercator (EPSG:3857) » → une étape `data.transform_point`, le système d'arrivée écrit (11.12) |
| `reprojection_couche` | « reprojette la couche « Gares » en Lambert-93 », « convertis les écoles en WGS84 », « passe la couche des parcelles en UTM 32N » → une étape `data.reproject`, le système d'arrivée écrit ; la couche est tirée dans un autre système que la cible. Remplace le refus « aucune commande ne reprojette une couche » (10.7) |
| `reprojection_avant_analyse` | tampon ou recherche à distance (« les écoles à moins de 500 m des gares ») sur une couche en degrés (`epsg: 4326` dans l'inventaire), sans que la demande parle de reprojection → `data.reproject` de cette couche, puis l'analyse sur `step:1.layer_id` (les étapes suivantes décalées d'un rang) ; cible : le système des autres couches vecteur en mètres de l'inventaire (Lambert-93, UTM 31N ou 32N), EPSG:2154 quand toutes sont en degrés ; l'`epsg` des couches n'est jamais effacé de ces inventaires (10.7) |
| `analyse_couche_metrique` | la même analyse (tampon ou recherche à distance) sur une couche déjà en mètres (Lambert-93, UTM 31N ou 32N), dans un inventaire qui porte aussi une couche vecteur en degrés qu'aucun plan ne cite → l'analyse seule, jamais un `data.reproject` (second essai de 11.23, constat S07) |
| `distance_azimut` | « quelle distance sépare le point … du point … », « distance et azimut entre les points … et … (UTM 31N) » → `geo.distance_azimuth` ; `unit` seulement si elle est écrite (`gon` : « en grades », `deg` : « en degrés ») ; deux points en Lambert-93 → deux `data.transform_point` vers 4326, puis le calcul sur `step:1.point` et `step:2.point` ; système écrit à chaque point ou une fois pour les deux (11.12) |
| `point_projete` | « quel point atteint-on à 3 km du point …, azimut 45° » → `geo.destination_point`, l'azimut toujours avec son unité (« 45° », « 45 degrés » → `deg` ; « 50 gr », « 50 grades » → `gon`) et la distance tels qu'écrits ; « résultat en MGRS » → `coord.convert` sur `step:1.point` (11.12) |
| `recoupement` | « recoupe la visée du point … à l'azimut 40° avec celle du point … à l'azimut 310° » → `geo.bearing_intersection`, chaque station avec son azimut ; même suite MGRS que `point_projete` (11.12) |
| `terrain_autre_systeme` | champ de vision ou ligne de vue depuis un point écrit dans un autre système que le MNT (WGS84, UTM d'une autre zone, Lambert-93 sur un MNT en UTM) → depuis 11.23, la commande de terrain avec le point **recopié tel qu'écrit**, sans `data.transform_point` : le noyau ajoute la conversion au plan avant de le montrer (11.22) ; une ligne de vue peut viser un point du système du MNT ; une référence MGRS (une fois sur quatre, alors pour chaque point), qui n'est pas un `Point`, garde `coord.convert` vers le WGS84 puis `data.transform_point` vers l'`epsg` du MNT, et la commande prend `step:k.point` (MNT à système connu seulement). Remplace le refus `NO_TERRAIN_CRS` de 5.41 (11.12) |
| `exposition` | « calcule l'exposition des versants du MNT » → `terrain.aspect`, `dem` seul (11.12, lot 2a) ; depuis 11.23, l'orientation des pentes dite sans le mot « exposition » (« dans quelle direction regardent les pentes », « vers où sont orientées les pentes ») aussi, jamais une pente reclassée (constat A01 de 11.16) |
| `ombrage` | « ombrage du MNT, soleil à l'ouest (azimut 270°), hauteur du soleil 30°, exagération verticale de 2 » → `terrain.hillshade` ; `sun_azimuth`, `sun_altitude` et `z_factor` seulement s'ils sont écrits, les angles en degrés ou en grades (`gon`), tous dans la même unité ; sans réglage écrit, `dem` seul (11.12, lot 2a) |
| `champ_de_vision_cumule` | « combien de ces points voient chaque endroit : le point …, le point … et le point … » → `terrain.viewshed_cumulative`, deux à quatre points dans le système du MNT, chacun avec son système ; hauteurs et `radius` comme `champ_de_vision` (11.12, lot 2a) ; une fois sur cinq, des points écrits dans un autre système, recopiés tels quels depuis 11.23 (la question du système, de 2026-11-08, est retirée : le noyau convertit chaque point) |
| `points_hauts` | « les 10 sommets les plus hauts du MNT, proéminence d'au moins 20 m » → `terrain.peaks` ; `max_count` (2 à 100, écrit en chiffres), `min_prominence` et `min_distance` seulement s'ils sont écrits (11.12, lot 2a) |
| `pente_seuil` | « où la pente dépasse-t-elle 15 % », « classe les pentes en moins de 5 %, de 5 % à 15 % et plus de 15 % » → `terrain.slope` dans l'unité du seuil, puis `raster.reclassify` sur `step:1` ; « dépasse, plus de n » → `{min: n, value: 1}`, « sous, moins de n » → `{max: n, value: 1}`, deux ou trois classes de valeurs 1, 2, 3 à bornes contiguës (11.12, lot 2a) |
| `cout_cumule` | « calcule le coût de déplacement cumulé depuis le point … (Lambert-93) sur le raster de coût » → `terrain.cost_distance`, `cost` recopié de la `source` du raster de coût de l'inventaire, une à trois `sources` écrites dans son système, dans l'ordre écrit, chacune avec son système (une fois sur cinq, dans un autre système, recopiées telles quelles depuis 11.23, plus de question) ; une fois sur six, un lieu nommé sans coordonnées (« depuis la mairie de X ») → question qui demande le point en X, Y et système (11.12, lot 2b-1) |
| `chemin_moindre_cout` | « trace le chemin de moindre coût du point … au point … sur la couche « Friction » » → `terrain.least_cost_path`, `start` puis `end` dans l'ordre écrit, dans le système du raster de coût (une fois sur trois, le départ, et l'arrivée une fois sur deux, dans un autre système, recopiés tels quels depuis 11.23 ; une référence MGRS garde ses deux conversions) ; un seul point écrit, l'autre nommé → question du point ; aucun raster de coût dans l'inventaire (le MNT seul, ou rien) → question qui demande d'ouvrir ou de nommer le raster de coût, jamais le MNT pris comme coût (11.12, lot 2b-1) |
| `superposition_ponderee` | « combine l'accessibilité (poids 2) et le calme (poids 1), en excluant les zones inondables » → `raster.weighted_overlay`, deux à quatre `criteria` `{raster, weight}` dans l'ordre écrit, chaque `raster` recopié de la `source` d'un critère noté de l'inventaire ; poids écrits recopiés (« poids 2 », « avec un poids de 2 pour… », « … comptant double »), sinon tous à 1 ; zéro à deux `exclusions`, masques écrits dans l'ordre écrit ; une fois sur huit, un critère nommé absent de l'inventaire → question qui demande d'ouvrir ou de nommer ce raster (11.12, lot 2b-2) |
| `reclassement_raster` | « classe la distance aux routes en moins de 100 m, de 100 à 500 m et plus de 500 m », « où le bruit dépasse 65 dB » → `raster.reclassify` directement sur la `source` du raster brut, sans `terrain.slope` ; seuil unique (`{min}` ou `{max}`, valeur 1) ou deux à trois classes à bornes contiguës, valeurs 1, 2, 3 sauf si elles sont écrites (« notées 3, 2 et 1 », une fois sur trois) ; une fois sur huit, raster absent → question du raster (11.12, lot 2b-2) |
| `style_topographique` | « mets les couches aux couleurs topographiques », « donne à la carte les couleurs d'une carte IGN » → `map.topographic_style` sans paramètre ; une fois sur trois, une ou deux couches nommées (« mets les routes et les rivières aux couleurs de l'IGN », « mets la couche des canalisations avec la sémiologie de l'IGN ») → `layers`, leurs identifiants dans l'ordre écrit (11.23) |
| `style_degrade` | « colore les communes selon leur population », « représente les parcelles par classes de surface » → `map.graduated_style` sur la couche et son champ numérique ; `method` (« en quantiles », « à intervalles égaux », « par seuils naturels » ; depuis le second essai de 11.23, la méthode est écrite une fois sur deux, avec aussi « , à intervalles égaux », « avec des classes d'égale amplitude », « avec des classes de même effectif », « par ruptures naturelles », constat ST08), `class_count` (« en 7 classes », de 2 à 12) et `ramp` (« en bleus », « dans un dégradé de verts », « en niveaux de gris », « du jaune au rouge », « du rouge au bleu ») seulement s'ils sont écrits ; une fois sur six, la couche n'a pas le champ → la question du champ (11.23) |
| `redemande` | environ 5 % du jeu : un exemple d'une autre famille vu au tour de redemande (voir plus bas) |

Depuis 5.15, `conditions_multiples` porte aussi les plans à **trois conditions** (cible
filtrée, zone et distance ; zone et deux distances), quatre à six étapes, jamais plus de six.
L'export PDF s'y ajoute une fois sur quatre, et seulement aux plans de cinq étapes ou moins
(jamais à « zone et deux distances », qui en a déjà six).

### Nouvelle passe de 5.17 (tâche A)

Les constats de 5.15 sur le jeu tenu à l'écart et sur la phrase E de `duree_reel` deviennent
des familles, jamais des règles de prompt :

- **Trajet depuis un lieu de l'inventaire.** La même tournure de trajet (« à N minutes à pied
  de ... », « situées à », « à moins de », « accessibles en ... depuis », « qu'on rejoint
  en ») rend une question quand l'origine est un lieu sans coordonnées (`question_isochrone`,
  couche du lieu présente le plus souvent, cible souvent citée, la question du
  garde-fou de 5.16 parmi les réponses), et un plan quand l'origine est un point en
  coordonnées (`isochrone_vers_cible` : `analysis.isochrone`, puis `analysis.intersect` de la
  cible avec `step:1`). Une distance en mètres reste un plan (`distance`). « À N minutes à
  pied de » nu passe par « situées à » : c'est la tournure de la phrase E.
- **Origines multiples** (décision du propriétaire, 2026-10-23, option a) : « de chaque
  gare », « de toutes les gares », « des gares » → refus affirmatif (`refus_hors_registre`) :
  l'isochrone du catalogue part d'un seul point. « D'une gare » → question (laquelle, en
  coordonnées) ; « de la gare de X » → question.
- **Plans à plusieurs conditions.** La cible filtrée l'est aussi par un nombre (`gt`, `lt`,
  `gte`, `lte`), une différence (`ne`) ou un nom (`contains`, `begins_with`), plus seulement
  par égalité. Deux formes s'ajoutent : cible filtrée et deux distances (cinq étapes :
  sélection, deux tampons, deux intersections), et distance avec inclusion dans une couche
  entière (« situées dans une zone humide », la couche prise telle quelle, intersections dans
  l'ordre de la phrase). Le poids de `conditions_multiples` passe de 9 à 13. Un test vérifie
  qu'aucune étape n'est perdue : chacune sert à une étape suivante.

**Limite de la mesure.** Ces familles visent des écarts nommés par le jeu tenu à l'écart
(H08, H17 à H23, H40), sans en lire les phrases. Viser des écarts nommés affaiblit
pourtant `ecart_reel` comme mesure aveugle : le score qui suivra mesure en partie ce qu'on a
cherché à corriger. Il faudra le lire avec cette réserve, et un jeu neuf, écrit par un autre
auteur, redonnera une mesure aveugle.

### Familles de géométrie (11.12, lot 1)

Les cinq commandes de géométrie de l'epic 11 ont chacune leur famille (`conversion_coordonnees`,
`reprojection_point`, `distance_azimut`, `point_projete`, `recoupement`), et
`terrain_autre_systeme` montre la reprojection d'un point avant une commande de terrain. Règle
de choix, celle du catalogue :

- les deux systèmes dans WGS84, UTM et MGRS → `coord.convert` ;
- un autre système en jeu (Lambert-93, Lambert II étendu, Web Mercator, ou le système d'un
  MNT) → `data.transform_point`, puis `coord.convert` si le MGRS est demandé.

Une conversion ou une reprojection est toujours une étape visible du plan (depuis 11.23, celle
d'un point de terrain est ajoutée par le noyau, et le modèle recopie le point tel qu'écrit : voir
« Passe de 11.23 »). Tout nombre du plan
(azimut, distance, précision, fuseau) est écrit dans la demande ; un `Angle` porte toujours son
`unit` ; une référence MGRS est valide (tirée par `Mgrs::from_lat_lon` du noyau) et le plan la
recopie sous sa forme canonique, sans espace. Chaque exemple passe `check_plan` puis
`guard_coordinates` sans changement.

**Refus retiré.** Le refus `NO_TERRAIN_CRS` de 5.41 (« aucune ne reprojette un point ») est
faux depuis `data.transform_point` : il n'est plus engendré, et le garde-fou
`what_is_refused_out_of_registry_is_still_missing_from_the_registry` échoue si une famille de
refus le réintroduit. Le refus de reprojection d'une **couche** est retiré à son tour en 10.7 :
`data.reproject` est au registre, les familles `reprojection_couche` et
`reprojection_avant_analyse` le remplacent, et le même garde-fou échoue si une famille, quelle
qu'elle soit, rend encore un refus qui dit qu'aucune commande ne reprojette une couche. Le
ré-entraînement qui l'apprend au modèle est groupé dans 11.23.

**Redemande.** Les plans de géométrie à une étape ne se corrompent pas (aucune couche) : le poids
de `redemande` passe de 24 à 36 pour garder environ 5 % du jeu (419 sur 8 000 à la graine 1).

Les dérivés du MNT sont faits au lot 2a (section suivante) ; les rasters de coût, de critères et
de masque (lot 2b) et l'export au format (lot 3) restent à faire avant le ré-entraînement groupé
(entrée 16).

### Familles des dérivés du MNT (11.12, lot 2a)

Quatre commandes de terrain de l'epic 11 ont chacune leur famille, bâtie sur le MNT de
l'inventaire synthétique comme `pente` et `champ_de_vision` : `exposition`
(`terrain.aspect`), `ombrage` (`terrain.hillshade`), `champ_de_vision_cumule`
(`terrain.viewshed_cumulative`) et `points_hauts` (`terrain.peaks`). Tout nombre du plan
(azimut, hauteur, exagération, nombre de sommets, proéminence, écart, rayon) est écrit dans
la demande ; un paramètre facultatif n'est au plan que s'il est écrit ; un azimut d'ombrage
se rapporte au nord du quadrillage, et une direction nommée porte son azimut (« soleil à
l'ouest (azimut 270°) »), jamais un azimut à calculer. Les points d'une vision cumulée sont
dans le système du MNT, chacun avec son système écrit.

**Refus retiré.** Le refus `NO_RASTER_THRESHOLD` (« aucune commande ne sélectionne les cellules
d'un raster par leur valeur ») est faux depuis `raster.reclassify` (11.9) : il n'est plus
engendré. `pente_seuil` le remplace par `terrain.slope`, dans l'unité du seuil (« % » →
`percent`, « degrés », « ° » → `degrees`), puis `raster.reclassify` sur `step:1` ; une cellule
hors classe devient NoData, ce qui est la sélection voulue. Le garde-fou `selects_no_cell`
échoue si un refus de ce genre revient. La statistique par zone sur un raster reste un refus
(`NO_RASTER_STATS`).

**Validateur.** Un paramètre raster de premier niveau (`dem`, `raster`) est la `source` d'un
raster de l'inventaire ou le résultat d'une étape précédente (`step:k`) ; `check_plan` vérifie
la référence et son type raster. Les éléments de liste (`criteria[i].raster`, `exclusions[i]`)
sont contrôlés depuis 11.12, lot 2b-2 (voir plus bas).

**Redemande.** Ces plans, sans couche, ne se corrompent guère (seule `pente_seuil`, à deux
étapes, perd une étape) : le poids de `redemande` reste à 36, pour 348 redemandes sur 8 000 à
la graine 1 (4,35 %).

Les rasters de coût, de critères et de masque, avec `raster.weighted_overlay`,
`terrain.cost_distance` et `terrain.least_cost_path`, forment le lot 2b, découpé en deux
(section suivante).

### Familles du raster de coût (11.12, lot 2b-1)

L'inventaire synthétique gagne un second thème raster, `cout` (« Coût de déplacement »,
« Friction », « Coût de traversée ») : un raster apporté par l'utilisateur, dont ni le nom ni
le fichier ne parlent de pente, d'altitude ou de relief. Il est cité par sa `source`, protégée
des formes de QGIS comme celle du MNT (`Builder::add_raster`), dans le système des points de
la demande (Lambert-93, UTM 31N ou 32N). Il entre aussi parmi les rasters de distraction :
une demande de terrain sans MNT reste une question sur le MNT, même si un raster de coût est
là.

Deux familles : `cout_cumule` (`terrain.cost_distance`, une à trois sources) et
`chemin_moindre_cout` (`terrain.least_cost_path`, départ puis arrivée), les points dans le
système du raster de coût, chacun avec son système, dans l'ordre écrit. La demande désigne le
raster (« sur le raster de coût », « sur la couche « Friction » »), jamais « le terrain » : le
MNT est en distraction environ une fois sur deux (posé une fois sur trois, sinon parfois tiré
parmi les rasters de distraction). Sans raster de coût, le MNT est là environ cinq fois sur
huit, aucun raster environ trois fois sur huit. Aucune pente ni aucun autre dérivé du MNT n'est pris
comme coût ; sans raster de coût dans l'inventaire, on le demande. Le refus d'itinéraire entre
deux couches (`refus`) reste : un réseau routier n'est pas un raster de coût.

**Validateur.** `cost` rejoint `dem` et `raster` : c'est la `source` d'un raster de
l'inventaire, jamais un chemin inventé ni l'id d'une couche.

**Redemande.** Ces plans à une étape, sans couche, ne se corrompent pas : à poids inchangé, les
redemandes tombaient à 3,9 %. Le poids de `redemande` passe de 36 à 44, pour 383 redemandes
sur 8 000 à la graine 1 (4,8 %).

### Familles des critères, des masques et de la superposition (11.12, lot 2b-2)

L'inventaire synthétique gagne huit thèmes raster civils, sans champ, apportés par
l'utilisateur et cités par leur `source` protégée (`Builder::add_raster`) : quatre critères
notés (accessibilité, ensoleillement, proximité des services, calme ; un groupe chacun), deux
masques (zones inondables dans le groupe `inondables`, espaces protégés dans le groupe
`znieff`) et deux rasters bruts (distance aux routes dans le groupe `voirie`, bruit dans son
groupe). Aucun nom, fichier ni identifiant ne parle de pente, de MNT, d'altitude, de relief ou
de coût. Un masque partage le groupe de la couche vecteur de même sujet : les deux ne se
rencontrent jamais dans un inventaire. Tous entrent parmi les rasters de distraction, si bien
qu'un critère ou un masque non écrit peut être là sans être cité.

Deux familles, de poids 4 : `superposition_ponderee` et `reclassement_raster` (table plus
haut). Le MNT (une fois sur quatre) et le raster de coût (une fois sur six) y sont posés en
distraction : ni l'un ni l'autre n'est jamais un critère, un masque ni le raster reclassé.
Chaque critère, masque, poids, seuil et valeur cité est écrit dans la demande. Un critère se
désigne par son sujet (« l'accessibilité », « la note de calme ») ou par le nom de sa couche
(« la couche « Tranquillité » ») ; un masque par « en excluant les zones inondables », « hors
espaces protégés », « en écartant… », « sans… ». Les unités des rasters bruts sont les mètres
(distance) et les décibels (bruit).

**Validateur.** Chaque `criteria[i].raster` et chaque `exclusions[i]` est la `source` d'un
raster de l'inventaire : ni un chemin inventé, ni l'id d'une couche, ni une étape (`step:k`).

**Redemande.** Ces plans à une étape, sans couche, ne se corrompent pas : à poids inchangé (44),
363 redemandes sur 8 000 à la graine 1 (4,5 %), dans la fourchette de 4 à 7 %.

### Famille de l'export au format (11.12, lot 3)

`layout.export_pdf` porte depuis 11.11 `page_size` (`a4` à `a0`), `orientation` (`landscape`,
`portrait`) et `grid`. La famille `export_pdf_format` (poids 4) les apprend : format seul,
orientation seule, carroyage seul ou combinés, la carte seule deux fois sur trois, sinon au
bout d'une sélection, d'une distance ou d'une zone, avec un seul export en dernière étape. Un
paramètre n'est au plan que si la demande le nomme : ni `a4` ni `landscape` écrits par défaut,
jamais `scale` (qui reste à `export_pdf_scale`). Formats écrits « au format A3 », « en A3 »,
« sur une feuille A0 », « en format A2 », « A2 paysage ».

**Carroyage** (décision du propriétaire, 2026-11-07). Toute demande de carroyage nomme son
système : Lambert-93 → `epsg: 2154`, UTM 30N, 31N ou 32N → 32630, 32631, 32632, tous les
100 m à 10 km ; degrés (de 0,01° à 5°) ou grades (de 0,5 à 20 gr) → graticule. L'intervalle
est écrit avec son unité, à la française (« 0,01° » → `0.01`), et recopié. Une fois sur six,
le carroyage ne nomme pas son système en entier. Sans aucun système (« quadrillage tous les
500 m », « grille de 1 km », « carroyage kilométrique »), la réponse est la question « Dans
quel système veux-tu le carroyage : Lambert-93, UTM avec sa zone (30N, 31N ou 32N), ou un
graticule en degrés ou en grades ? », jamais un plan. Aucune étiquette MGRS n'est promise.

**Fuseau du projet** (2026-11-08, `plan-fuseau-utm-depuis-l-emprise.md`). Un carroyage « UTM »
sans fuseau (« carroyage UTM au kilomètre », « carroyage UTM tous les 2 km », une fois sur deux
des carroyages sans système) prend le fuseau du projet que montre l'inventaire,
`project.utm_epsg` : `grid: {kind: projected, epsg: <utm_epsg>, interval}` ; « au kilomètre »
vaut 1 km. L'inventaire de ces exemples le porte trois fois sur quatre, tiré parmi 32631 (deux
fois sur quatre), 32630 et 32632 sur des couches en Lambert-93, pour que le plan le recopie au
lieu de le deviner ; sans lui, c'est la question du système. Un fuseau écrit (« UTM 31N »)
l'emporte toujours, même quand l'inventaire en porte un autre. Au jeu par défaut (graine 1,
8 000 exemples), 5 plans recopiaient le fuseau et 3 questions le demandaient : trop peu pour
l'apprendre. La famille `carroyage_utm_projet` de 11.23 (section suivante) les porte à 65 et 11.

**Refus.** Le cas « papier » de `refus_hors_registre`, qui refusait A3, A0 et portrait (« seulement
une page A4 paysage »), est remplacé par ce que l'export ne fait pas : un format hors de la
série (A5, lettre US), l'envoi à une imprimante et le nombre d'exemplaires ; « imprime la
carte » seul reste un export (5.40). Le garde-fou
`what_is_refused_out_of_registry_is_still_missing_from_the_registry` échoue si un refus de
`refus` ou de `refus_hors_registre` cite un format, une orientation ou un carroyage du catalogue, et
`paper_and_orientation_are_plans_never_refusals` (qui inverse le test de 5.40) vérifie au jeu
par défaut qu'aucun refus ne cite « A4 » et que la famille a des plans en A3, A0, portrait et
pour les deux formes de grille.

**Redemande.** À poids inchangé (44), 369 redemandes sur 8 000 à la graine 1 (4,6 %), dans la
fourchette de 4 à 7 % ; 140 exemples `export_pdf_format`, aucune famille écartée.

Les trois lots de 11.12 sont faits ; reste le ré-entraînement groupé (entrée 16).

### Passe de 11.23 (partie A) : points tels qu'écrits, style, constats de 11.16

Le générateur est mis au niveau du noyau avant le ré-entraînement groupé de 11.23. Rien ne
change de la base, de la méthode, de `DEFAULT_COUNT` ni de `DEFAULT_SEED` ; chaque exemple
passe `check_plan` puis `guard_coordinates`.

- **Points tels qu'écrits** (changement de cap du 2026-11-08, décision 3). Un point de terrain
  écrit dans un autre système que son raster est recopié tel que la phrase le donne :
  `terrain_autre_systeme`, `champ_de_vision_cumule`, `cout_cumule` et `chemin_moindre_cout` ne
  composent plus `data.transform_point`, et le noyau complète le plan (11.22) avant de le
  montrer. Les questions sur une liste de points d'un autre système sont retirées. Une ligne de
  vue ou un chemin peut mêler un point d'un autre système et un point du système du raster. Une
  référence MGRS, qui n'est pas un `Point` et que la complétion ne voit pas, garde
  `coord.convert` puis `data.transform_point`. Les familles sans raster (`distance_azimut`,
  `conversion_coordonnees`, `reprojection_point`) gardent leurs conversions : la complétion ne
  s'applique qu'aux commandes du noyau qui ont un raster.
- **Style** (11.20). Deux familles, `style_topographique` (poids 3) et `style_degrade`
  (poids 4). Le refus « Aucune commande du catalogue ne change le style d'une couche » est
  retiré : « mets les gares en rouge » rend désormais `NO_CHOSEN_COLOR` (« Le catalogue ne pose
  pas une couleur choisie sur une couche : il met les couches aux couleurs topographiques, ou
  colore une couche en dégradé selon un champ numérique. »). Le garde-fou `styles_no_layer`
  fait échouer `what_is_refused_out_of_registry_is_still_missing_from_the_registry` si une
  famille, quelle qu'elle soit, rend encore un refus qui parle de style.
- **Fuseau UTM du projet.** `carroyage_utm_projet` (poids 3, 100 exemples à la graine 1) :
  recopie de `utm_epsg` quatre fois sur huit, question sans lui une fois sur huit, fuseau écrit
  différent de celui de l'inventaire deux fois sur huit, aucun système une fois sur huit. Le
  nom du projet tiré suit son fuseau (`inventories::project_names`) : un projet de l'ouest en
  30N (Rennes, Brest, Nantes, Quimper), de l'est en 32N (Strasbourg, Colmar, Nice, Mulhouse), du
  Val-de-Marne sinon.
- **Constats de 11.16.** F03 (carroyage sans système refusé) : la question, renforcée par
  `carroyage_utm_projet`. A01 (« dans quelle direction sont tournées les pentes ») : quatre
  tournures de plus dans `exposition`. S18 (PDF qui nomme ses couches) : un cas de
  `refus_hors_registre`, `NO_LAYER_CHOICE`. S20 (lieu nommé suivi de ses coordonnées) : une
  forme de `champ_de_vision`. Les deux ratés de validation « point UTM d'un fuseau à l'autre »
  étaient des points de terrain, désormais convertis par le noyau ; `conversion_coordonnees`
  apprend en plus le passage d'un point UTM au fuseau voisin.
- **Recouvrement.** La tournure « Où la pente du terrain dépasse-t-elle … » de `pente_seuil`
  rendait Z01 d'`epic11_reel` : elle est retirée (« du MNT » à sa place), la graine ne change
  pas.

**Redemande.** Les nouveaux plans à une étape se corrompent peu : à poids inchangé, 311
redemandes (3,9 %). Le poids de `redemande` passe de 44 à 52, pour 380 redemandes sur 8 000 à
la graine 1 (4,75 %).

**Essais réels.** `plan_reel`, `terrain_reel`, `epic11_reel` et le nouvel essai `style_reel`
mesurent le **plan complété** : `ai/tests/common/mod.rs` (`completed`, `complete_with`) applique
la complétion du noyau au plan composé, le système d'un raster pris dans l'inventaire (dans le
produit, l'en-tête du GeoTIFF). S25 garde son attendu (la conversion puis le champ de vision) ;
un test sans modèle vérifie que le point recopié tel qu'écrit en est complété. `epic11_reel`
gagne quatre phrases mesurées à part, hors de sa barre (`FOREIGN_POINTS` : champ de vision
cumulé depuis deux points WGS84, coût cumulé depuis un point UTM 31N, chemin d'un point WGS84 à
un point Lambert-93, point UTM 31N vers le fuseau 32). `style_reel` : dix phrases nouvelles,
quatre mises en forme topographiques, cinq dégradés, une question de champ absent ; barre
9 / 10.

### Second essai de 11.23 : corrections du générateur

Le premier entraînement de 11.23 (validation 297/300) manque deux barres : `plan_reel` 22/25
et F03 d'`epic11_reel`. Décision du propriétaire du 2026-11-09 : corriger l'attendu de S05 et
le générateur, puis un second et dernier entraînement (règle de 11.16), mêmes base, méthode,
`DEFAULT_COUNT`, `DEFAULT_SEED`, mêmes essais et barres.

- **S05** « Mets les bâtiments en Lambert-93 » : la couche des bâtiments est en 4326 dans
  l'inventaire de l'épreuve ; l'attendu de `ai/tests/plan_reel.rs` devient `data.reproject`
  vers 2154 (le refus était périmé depuis 10.7). `tests/epreuve-modele-local/` ne change pas.
- **S07** (tampon autour de gares déjà en 2154, reprojetées quand même) : nouvelle famille
  `analyse_couche_metrique` (poids 4, 123 exemples à la graine 1), la même analyse que
  `reprojection_avant_analyse` (code commun `metric_analysis`) sur une couche en mètres, à
  côté d'une couche en degrés (`Builder::distract_in_degrees`) : aucun `data.reproject`. Au jeu
  par défaut, un `data.reproject` ne vise une couche en mètres que si la phrase écrit le
  système d'arrivée (`reprojection_couche`).
- **F03** (fuseau UTM inventé sans `utm_epsg`) : `carroyage_utm_projet` passe au poids 4 (116
  exemples) et à trois recopies pour deux questions ; au jeu par défaut, toutes familles comprises, 57
  recopies de `utm_epsg` et 32 questions sans lui (au moins la moitié, vérifié par un test).
- **S21** (« 2 m de hauteur de chaque côté », hauteur de la cible perdue) : `ligne_de_vue`
  écrit une fois sur quatre une même hauteur pour les deux bouts, qui donne `observer_height`
  et `target_height`.
- **ST08** (« à intervalles égaux » perdu) : `style_degrade` écrit la méthode une fois sur deux
  (une sur trois avant), avec quatorze tournures dont des formes entre virgules ; 45 méthodes
  au jeu par défaut, dont 10 à intervalles égaux.

`training/tests/recouvrement.rs` reste vert sans changer la graine ; `redemande` garde son poids
(379 redemandes sur 8 000).

### Les inventaires de QGIS (5.15)

Les valeurs d'un champ texte sont triées, comme le produit les rend. `unloaded_files` est
vide sauf quand la famille ouvre un fichier (`ouverture`, avec parfois un second fichier à ne
pas ouvrir) : dans QGIS, l'extension ne parcourt pas le disque. Un inventaire sur trois prend
les formes que rend `core/src/agent_inventory.rs` depuis QGIS : raster sans `feature_count`,
couche temporaire sans `source`, SCR qui n'est pas EPSG (couche ou projet sans `epsg`), table
de distraction sans géométrie (`geometry: null` sur un vecteur), projet non enregistré (sans
`name`). `training/tests/forme_inventaire.rs` vérifie que chacune de ces formes, rendue par
le produit, est vue à l'entraînement.

### La redemande (5.15)

Un exemple `redemande` part d'un exemple valide d'une autre famille dont la réponse est un
plan d'appels, et corrompt ce plan comme un modèle le corromprait : identifiant de couche
inconnu, référence `step:k` vers l'étape elle-même ou une suivante, étape retirée qui casse
une référence. La corruption n'est gardée que si `check_plan` la rejette (un identifiant
inconnu ne l'est jamais : `check_plan` ne voit pas l'inventaire, l'exemple est laissé). Le
prompt est alors celui que `cassini-ai` assemble à la redemande (`retry_turns`) : le prompt
du premier essai, la sortie rejetée en tour assistant, puis le tour « Ta réponse n'est pas
conforme : étape k : message. Redonne le plan corrigé, uniquement le JSON. » ; la réponse
attendue est le plan juste. La perte ne porte que sur cette réponse, `data.py` n'a pas changé.

Une phrase mal comprise par le modèle devient une famille ou une tournure de plus ici,
jamais une règle de prompt.

### Limite connue : « constructible »

« Constructible » se traduit par `typezone` égal à `U`, la zone urbaine du standard du
CNIG. Les zones `AUc` (à urbaniser, ouvertes) sont aussi constructibles, mais le registre
n'a ni opérateur `in` ni union de couches : une seule sélection ne peut pas couvrir les
deux. Le jeu s'en tient donc à `U`.

## Le jeu tenu à l'écart

`ai/tests/tenu_a_l_ecart/` porte 40 phrases inédites sur un inventaire de projet
d'urbanisme : 26 plans (dont la phrase de démonstration exacte de `CLAUDE.md`),
8 questions et 6 refus. C'est la barre honnête.

Règle : il a été écrit par un autre auteur que le générateur, qui ne l'a pas lu. Seuls
deux fichiers le lisent :

- `training/tests/recouvrement.rs` engendre le jeu avec la graine par défaut et vérifie
  qu'aucune phrase tenue à l'écart n'y figure telle quelle (texte normalisé : minuscules,
  sans accents ni ponctuation), ni à un nombre ou une commune près. S'il échoue, on retire
  la tournure fautive du générateur et on le signale ; on ne retouche jamais une famille
  pour coller au jeu tenu à l'écart. Depuis 5.15, le même test couvre les phrases des
  essais réels `plan_reel`, `question_reel` et `duree_reel` (qui, eux, ne sont pas tenus à
  l'écart). Il a fait retirer trois tournures : « Fais une zone tampon de N m autour des
  ... » devient « Fais-moi une zone tampon ... », « Décris la couche des ... » devient
  « Décris-moi la couche des ... », et « de la section AB » sort des tournures des parcelles
  (« en section AB », « de section AB » restent).
- `ai/tests/ecart_reel.rs` mesure le vrai modèle sur ce jeu, sans jamais imprimer le texte
  des phrases.

## Score de référence

Commande (Windows, GPU) :

```
$env:CASSINI_TEST_MODEL="C:\Users\kevin\cassini-modeles\modeles\Qwen3.5-9B-Q4_K_M.gguf"
cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture ecart_reel
```

Un plan est compté juste s'il est identique au plan attendu, sauf le titre du PDF, dont
la présence suffit (nombres comparés en valeur : `500.0` vaut `500`). Une question
attendue doit rendre une question, un refus attendu un refus. Un plan rendu qui ne passe
pas `check_plan` compte comme faux, et l'essai échoue après avoir imprimé la ligne `SCORE`.

Mesure du 2026-10-22, Qwen3.5-9B Q4_K_M sur GPU (`--features cuda`), prompt complet de
`ai/src/plan.rs` au commit `300ac5ee346d06b22abcf22b71e4189a61c9fb68` (dernier commit
qui touche ce fichier), décodage glouton (celui de `cassini-ai`), un seul passage,
344 s pour les 40 phrases (re-mesuré après les correctifs de notation : identique, 346 s ; la phrase de démo H01 est manquée) :

| Sorte | Justes |
|---|---|
| Plans | 19 / 26 |
| Questions | 7 / 8 |
| Refus | 6 / 6 |
| **Total** | **32 / 40** |

Les sept plans manqués ont tous rendu un plan (jamais une question ni un refus) : l'écart
porte sur le contenu du plan. La question manquée a rendu un refus, le défaut déjà noté
dans `deferred-work.md` (une couche absente lue comme une opération impossible). C'est
la barre que le modèle ajusté devra atteindre ou dépasser.

## Tâche 2 : entraînement reproductible et fusion en GGUF

### Le prompt court

Le modèle ajusté ne voit plus le long prompt de `ai/src/plan.rs` (catalogue et exemples) :
il voit un message système de quelques phrases (rôle, trois formes de réponse, chaînage,
question plutôt que deviner, refus seulement si aucune commande ne sait faire), puis
l'inventaire du projet et la demande, au format chatml de Qwen, réflexion coupée.

Depuis 5.15, les consignes disent que les étapes se comptent à partir de 1, comme le long
prompt (« la couche produite par l'étape k (en comptant à partir de 1) par
"step:k.layer_id" »). Le modèle livré par 5.10 a été entraîné sur l'ancien texte : il faut le
ré-entraîner (tâche B de 5.15) avant de le mesurer avec ce prompt.

Ce texte est écrit **à un seul endroit** : `training/src/prompt.rs` (`render_prompt`,
`completion`). La commande l'écrit dans le jeu :

```
cargo run -p cassini-training --release -- --format chat --seed 1 --count 8000 --out entrainement.jsonl
```

Chaque ligne est `{"prompt": "...", "completion": "..."}` : `prompt` est le texte exact vu
par le modèle, jusqu'à l'ouverture du tour assistant comprise ; `completion` est le plan en
JSON compact, `kind` en tête, suivi de `<|im_end|>`. La chaîne Python lit ces deux champs
tels quels et ne met rien en forme : un écart d'un espace entre deux mises en forme
apprendrait au modèle un format qu'il ne verrait jamais. La tâche 3 fera lire ce même
module à `ai/`.

### Une commande pour ré-entraîner

Sous WSL, avec le GPU de 8 Go, depuis la racine du dépôt, dans un dossier vide **hors du
dépôt** :

```
UV_PROJECT_ENVIRONMENT=~/.cache/cassini-entrainement/venv \
  uv run --project training/entrainement python training/entrainement/pipeline.py --sortie ~/cassini-entrainement/5-10-t2
```

(`UV_PROJECT_ENVIRONMENT` est facultatif : il place l'environnement Python sur le disque de
WSL plutôt que dans `training/entrainement/.venv`, sur le disque Windows, plus lent.)

La commande, sans intervention :

1. vérifie le GPU **avant tout téléchargement** et avant de toucher au dossier de sortie
   (au moins 7 Gio, sinon arrêt avec message) ;
2. refuse un dossier de sortie déjà rempli (rien n'est écrasé ; `--reprendre` finit un
   entraînement commencé en sautant les étapes déjà faites) ou situé dans le dépôt ;
3. engendre les jeux par `cassini-training --format chat` : graine 1 pour l'entraînement
   (8 000 exemples depuis 5.15, 5 000 avant), graine 2 pour la validation (300), dont elle retire les prompts déjà
   vus à l'entraînement ;
4. prend la base `Qwen/Qwen3.5-4B` (Apache-2.0) à la révision épinglée
   `851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a`, dans le cache de Hugging Face ;
5. écarte (et compte) tout exemple de plus de 2 048 jetons, sans jamais le tronquer ;
6. entraîne un adaptateur QLoRA : base en nf4, gradient checkpointing, graine fixe, perte
   sur la réponse seule, projections du modèle de langue seulement. Depuis le 2026-11-09, la
   base se charge et l'adaptateur se pose par **Unsloth** (`--sans-unsloth` reprend
   transformers et peft seuls), même méthode et mêmes hyperparamètres. Les couches à attention
   linéaire de Qwen3.5 passent par les noyaux Triton de `flash-linear-attention` (depuis le
   2026-11-09, voir « Accélération » plus bas ; `--noyaux-reference` reprend l'implémentation
   PyTorch de référence). La graine fixe rend les **jeux** reproductibles à l'octet, pas les
   **poids** : bitsandbytes et ces noyaux ne sont pas déterministes sur GPU ;
7. mesure le score exact sur la validation (`scoring.py`), en décodage glouton : règle de
   `ai/tests/ecart_reel.rs`, avec une exigence de plus, une question ou un refus doit avoir
   un texte non vide. Depuis le 2026-11-09, ce score se mesure **après** l'étape 8, sur le
   GGUF (le modèle fusionné, celui qui s'installe ; la base et l'adaptateur pour un adaptateur
   de domaine), par `llama-server` construit sur CUDA, en 5 min au lieu de 50 ;
   `--validation-pytorch` le mesure comme avant, par PyTorch sur l'adaptateur, avant l'étape 8 ;
8. clone llama.cpp au commit `d81235049384534c167caea52b85a694f6103d14` (celui que lie
   `cassini-ai` par llama-cpp-sys-2 0.1.160) dans `~/.cache/cassini-entrainement/`, construit
   ses outils sur CPU, convertit la base et l'adaptateur en GGUF, fusionne, puis quantifie ;
   chaque fichier s'écrit sous un nom temporaire et ne prend son nom final qu'une fois
   complet. Un test vérifie que `Cargo.lock` tire toujours llama-cpp-sys-2 0.1.160 ;
9. essaie, avec `llama-completion` (l'outil de llama.cpp qui prend un prompt brut ;
   `llama-cli` applique son propre gabarit de conversation), la première demande de
   validation qui attend un plan d'appels, par les deux chemins : la base GGUF avec
   l'adaptateur GGUF (`--lora`), et le modèle fusionné. Les deux sorties doivent se lire
   comme un plan JSON, sinon la commande s'arrête ;
10. écrit `rapport.json`, avec le commit du dépôt, l'état de l'arbre de travail (modifié ou
   non, chemins modifiés) et l'empreinte de `uv.lock`.

Dépendances épinglées par `training/entrainement/uv.lock` (Python 3.12) : torch (BSD-3),
transformers, peft, accelerate, huggingface-hub (Apache-2.0), bitsandbytes (MIT),
flash-linear-attention et fla-core (MIT), unsloth (Apache-2.0) et unsloth-zoo (LGPL-3.0,
outillage d'entraînement seulement, jamais dans le noyau ; voir l'en-tête de
`pyproject.toml` pour ses autres dépendances), et, pour compiler llama.cpp sur CUDA, les roues
nvidia-cuda-nvcc, nvidia-nvvm, nvidia-cuda-crt et nvidia-cuda-cccl (licence NVIDIA, comme
les bibliothèques nvidia-* que torch tire déjà). Rien de cela n'entre dans
la recette : les tests `python3 -I -m unittest discover -s training/entrainement/tests`
(lecture du jeu, notation, gardes de la chaîne) n'importent pas torch.

### Accélération (2026-11-09)

Quatre pistes essayées, dans l'ordre du propriétaire, chacune sur un entraînement court
(`--pas-max 20`, même jeu, graine 1, au commit `4e10efe`), contre une référence mesurée dans
les mêmes conditions (`--noyaux-reference`). Avec `--pas-max 20`, le planning cosinus tient
en 20 pas (sans échauffement) : la perte aux pas 10 et 20 ne se compare donc qu'à celle d'un
entraînement court, pas à celle d'un entraînement de 500 pas (0,74 au pas 10 pour
`regroupe-11-23b`, dont le taux monte encore). Durée par pas : moyenne des pas 11 à 20, hors
amorce (chargement, compilation Triton).

| Configuration | s / pas | Perte pas 10 | Perte pas 20 | Pic (réservé par torch) | Retenue |
|---|---|---|---|---|---|
| référence PyTorch (`--noyaux-reference`) | 25,5 | 0,4723 | 0,2754 | 6,29 Gio | — |
| 1a : règle delta de `flash-linear-attention`, convolution PyTorch | 23,1 | 0,4726 | 0,2767 | 6,50 Gio | non (gain faible) |
| **1b : règle delta et convolution de `flash-linear-attention`** | **17,8** | **0,4740** | **0,2793** | **6,24 Gio** | **oui** |
| 4 : Unsloth 2026.11.3, environnement à part (transformers 5.17.0) | 10,7 | 0,4778 | 0,2747 | 6,21 Gio | essai, puis intégré |
| **chaîne intégrée : Unsloth + 1b + validation GGUF, de bout en bout** | **10,7** | **0,4818** | **0,2765** | **6,37 Gio** | **oui (par défaut)** |
| chaîne intégrée, `--sans-unsloth` (transformers 5.17.0) | 17,9 | 0,4742 | 0,2792 | 6,50 Gio | repli |
| 2 : 1b + regroupement des exemples, 2 048 jetons par passe | > 33 | — | — | > 8 Go | non (mémoire saturée) |
| 2 : 1b + regroupement, 1 600 jetons par passe | 17,7 | 0,4735 | 0,2760 | 6,92 Gio | non (aucun gain) |

- **Piste 1.** `flash-linear-attention` 0.6.0 (MIT) est en Triton : aucun compilateur, transformers
  le prend de lui-même pour la règle delta. `causal-conv1d` (BSD-3) n'a pas de roue pour
  torch 2.14 : la roue la plus proche (torch 2.10, CUDA 13) ne se charge pas (symbole C++
  absent). La convolution passe donc par celle, en Triton, de `flash-linear-attention`,
  branchée par `install_fast_kernels` dans `pipeline.py` : c'est elle qui fait l'essentiel du
  gain, la convolution de référence calculant en float32. Contrôle sur quatre exemples, même
  adaptateur : pertes à 0,5 % près de la référence, gradient au cosinus 0,989.
- **Piste 4.** Unsloth 2026.11.3 exige transformers ≤ 5.17.0 ; la chaîne épingle 5.20.2,
  et uv refuse de résoudre les deux ensemble. Il tire aussi `unsloth_zoo`, sous **LGPL-3.0**,
  accepté par le propriétaire pour un essai seulement (outillage d'entraînement, jamais dans le
  noyau). Essai fait **hors de la chaîne**, dans un projet uv à part
  (`~/cassini-entrainement/unsloth-essai/`, transformers 5.17.0, huggingface-hub 1.33.0, le
  reste aux versions de la chaîne), par la même boucle sur le même jeu : même QLoRA nf4 à
  double quantification, mêmes 26,3 millions de paramètres, mêmes modules, et un adaptateur aux
  mêmes 352 tenseurs (la conversion GGUF ne change pas). Unsloth compile chaque couche
  (`torch.compile`, environ 1,5 min de plus au départ) et prend son checkpointing
  « unsloth », qui décharge les activations en mémoire centrale. Régime tenu sur 30 pas :
  10,7 s par pas. Décision du propriétaire, le même jour : **tout garder**. Unsloth est
  intégré à la chaîne, par défaut, au-dessus de `flash-linear-attention` et de la validation
  GGUF : `uv.lock` épingle désormais transformers 5.17.0, huggingface-hub 1.33.0, unsloth et
  unsloth-zoo 2026.11.3. L'ancien chemin reste par `--sans-unsloth`, et tourne encore avec
  transformers 5.17.0 (mesuré ci-dessus). `rapport.json` dit le chemin suivi
  (`chemin_entrainement`) et les versions (`versions`, Unsloth et fla compris). Mesure de
  bout en bout (`--pas-max 20`) : entraînement, conversion de l'adaptateur en GGUF, fusion,
  validation par `llama-server`, essai des deux chemins, tout passe ; les deux sorties de
  l'essai se lisent comme un plan. Pas encore mesuré : le score d'un modèle entraîné
  jusqu'au bout par Unsloth.
- **Piste 2.** Le regroupement gardait chaque exemple seul (positions remises à 0, longueurs
  cumulées transmises à l'attention linéaire et à la convolution ; contrôlé sur GPU : logits
  inchangés aux arrondis près, alors qu'un regroupement sans séparation les décale de 10 à 13)
  et la même perte moyenne par exemple. Mais le GPU est déjà occupé à plein par un exemple de
  750 jetons : à 1 600 jetons par passe, la durée ne bouge pas et le pic monte de 0,7 Gio ; à
  2 048, la mémoire déborde. Le code a été retiré.
- **Piste 3.** Validation de `regroupe-11-23` (300 demandes, modèle fusionné Q4_K_M) par
  `llama-server` construit sur CUDA : **296 / 300** contre 297 / 300 par PyTorch (3 désaccords
  sur 300, 277 sorties identiques au caractère près), en **5 min 22 s** contre 52 min. Le
  compilateur CUDA vient des roues NVIDIA épinglées dans `uv.lock`, posées par uv dans
  l'environnement : rien au niveau du système, pas de `sudo`. `llama-server` est construit
  une fois (environ 6 min), au même commit que les outils sur CPU, dans
  `~/.cache/cassini-entrainement/llama.cpp/build-cuda/`, et reconstruit de zéro si
  l'environnement Python change (CMake efface sa configuration quand le chemin de nvcc
  change) ; la chaîne s'arrête s'il ne voit pas le GPU, plutôt que de valider dix fois plus
  lentement sur le CPU. Il n'écoute que `127.0.0.1`, sur un port libre, le temps de la
  validation, et s'arrête avec la chaîne. Le score porte désormais sur le fichier qui
  s'installe (quantifié), et non plus sur l'adaptateur en nf4.

Estimation pour un entraînement complet (500 pas), chaîne intégrée : environ 1 h 30
d'entraînement au lieu de 3 h 30, et 5 min de validation au lieu de 50, soit environ 2 h 40
pour la commande entière au lieu de 5 h 30 (conversion et quantification inchangées).

### Ce qui sort

| Fichier | Rôle |
|---|---|
| `qwen3.5-4b-base-Q4_K_M.gguf` | la base, quantifiée, sur laquelle se charge un adaptateur |
| `cassini-composition-adaptateur-f16.gguf` | l'adaptateur LoRA de composition (environ 50 Mo) |
| `cassini-composition-4b-Q4_K_M.gguf` | base et adaptateur fusionnés, quantifiés Q4_K_M |
| `rapport.json` | révision de la base, commit de llama.cpp, graines, hyperparamètres, versions, perte, score de validation, empreintes SHA-256 |
| `validation-sorties.jsonl` | chaque sortie de validation, attendue et rendue |

Le dossier `travail/` garde les jeux engendrés et l'adaptateur au format de Hugging Face ;
les GGUF intermédiaires en f16 (près de 9 Go chacun) sont effacés à la fin.

### Un adaptateur de domaine : `--exemples-externes` (5.23)

La méthode garde la base intacte : l'adaptateur GGUF se charge sur la base GGUF. La même
commande entraîne un autre adaptateur, de domaine, sur un fichier d'exemples écrit ailleurs,
hors de ce dépôt : **même base à la même révision, mêmes graines, mêmes hyperparamètres,
même prompt court**.

```
uv run --project training/entrainement python training/entrainement/pipeline.py \
    --sortie <dossier hors dépôt> --exemples-externes <fichier.jsonl> --nom <nom>
```

- Le fichier est au format `chat` de `cassini-training` : une ligne `{"prompt", "completion"}`
  par exemple, le prompt déjà mis en forme (prompt court, chatml), la réponse finie par
  `<|im_end|>`. Il est lu et vérifié **avant** le GPU et le téléchargement : une ligne mal
  formée arrête la commande avec le nom du fichier et le numéro de la ligne.
- `--nom` : minuscules, chiffres, `-` et `_`. Un nom qui reprendrait une sortie du modèle
  civil (tout nom qui commence par `cassini-composition`) est refusé avant tout travail.
- **Mélange** : par défaut, le jeu civil engendré (`--exemples`, `--validation`) plus le fichier
  externe ; `--externe-seul` n'entraîne que sur le fichier externe.
- **Validation** : par défaut, 10 % du fichier externe, tirés par la graine de validation, sont
  mis de côté pour le score ; `--validation-externe <fichier>` donne un second fichier à la place,
  et rien n'est alors retiré du premier. Un prompt de validation vu à l'entraînement en est
  retiré et compté.
- Deux scores, séparés : `validation` (le jeu civil, quand il est mélangé, comme en 5.17) et
  `validation_externe`. Ensemble, ils montrent si l'adaptateur oublie les commandes ordinaires.
- `rapport.json` ne garde du fichier externe que son empreinte SHA-256 et ses comptes
  (`donnees.externe`) ; l'essai `llama-completion` sur un exemple externe n'y écrit ni la sortie
  ni l'attendu. Les sorties de validation externes (`validation-externe-sorties.jsonl`) restent
  dans le dossier de sortie, hors dépôt.
- `--reprendre` refuse un autre fichier externe ou un autre mélange que ceux de l'entraînement
  commencé.

| Fichier de sortie | Rôle |
|---|---|
| `qwen3.5-4b-base-Q4_K_M.gguf` | la base, identique à chaque entraînement, partagée par tous les adaptateurs |
| `<nom>-adaptateur-f16.gguf` | l'adaptateur LoRA de domaine |
| `<nom>.modele.json` | sa déclaration : base + adaptateur, prompt court |

Pas de modèle fusionné. Pour l'installer, copier l'adaptateur et sa déclaration dans le dossier
des modèles (celui que désigne `CASSINI_MODELS`, ou `models/` à côté de l'exécutable), et la
base si elle n'y est pas déjà : `cassini run model.list '{}'` liste alors la déclaration. Le
choix entre adaptateurs reviendra au sélecteur de 5.14.

### Mesure

Premier entraînement complet, le 2026-10-22, sous WSL sur la RTX 3070 Ti (8 Go), au commit
`4019c0c` plus une version antérieure des changements de la tâche 2 : avant le renommage en
anglais, avant l'essai par les deux chemins, avant l'écriture des fichiers sous nom temporaire
et avant l'arrêt des restes sans pas d'optimiseur, ce qui fait 8 exemples d'écart
(`rapport.json` de
`~/cassini-entrainement/5-10-t2/`, hors dépôt) :

| Mesure | Valeur |
|---|---|
| Exemples d'entraînement | 5 000 (graine 1), aucun écarté car trop long |
| Longueur des séquences | 742 jetons en médiane, 1 326 au plus |
| Paramètres entraînés | 26,3 millions sur 4,57 milliards (rang 16, alpha 32) |
| Pas d'optimiseur | 312 (lot 1, accumulation 16, une époque, taux 2e-4 en cosinus) |
| Perte | 0,436 au pas 10, 0,005 en fin d'entraînement |
| Pic de mémoire vidéo | 6,5 Gio (réservée par torch), sous les 8 Go |
| Durée de l'entraînement | 2 h 40 |
| Durée de la validation | 47 min (300 demandes, décodage glouton sur le GPU) |
| Durée totale de la commande | 3 h 31, conversion et quantification comprises |
| **Score de validation** | **300 / 300** : plans 250 / 250, questions 37 / 37, refus 13 / 13 |
| Essai `llama-completion` du modèle fusionné | plan JSON lisible et juste (une question) |
| Essai des deux chemins (refait à la main avec `try_model`, sur la première demande qui attend un plan d'appels) | base + adaptateur : plan juste ; modèle fusionné : plan juste |

Les noyaux rapides de l'attention linéaire de Qwen3.5 (`flash-linear-attention`,
`causal_conv1d`) ne sont pas installés : transformers prend son implémentation de
référence en PyTorch, correcte mais plus lente. C'est elle qui fixe la durée.

Le score de validation mesure ce que le modèle a appris des **familles** : la validation
est engendrée par le même générateur, avec une autre graine. Un score parfait y est donc
attendu et ne dit rien de la généralisation à des tournures inédites. La barre honnête
reste le jeu tenu à l'écart, mesuré à la tâche 4 avec le prompt court branché dans `ai/`.

## Tâche 3 : le modèle ajusté branché dans `ai/`

### La déclaration d'un modèle

Un GGUF seul est un modèle ordinaire : il reçoit le long prompt de `ai/src/plan.rs`, comme
avant (c'est le cas de Qwen3.5-9B). Un modèle qui attend autre chose se **déclare** par un
petit fichier `*.modele.json` posé à côté de ses GGUF :

```
{"model": "qwen3.5-4b-base-Q4_K_M.gguf", "adapter": "cassini-composition-adaptateur-f16.gguf", "prompt": "short"}
```

- `model` : le GGUF du modèle (la base, quand un adaptateur est nommé) ;
- `adapter` (facultatif) : l'adaptateur LoRA posé sur cette base, un seul ;
- `prompt` : `short` pour le prompt court de l'entraînement, `full` (ou absent) pour le long.

Des noms de fichiers seuls, du même dossier que la déclaration : une déclaration qui nomme
un dossier, un séparateur ou `..`, qui n'est pas du JSON lisible ou qui porte un champ
inconnu est ignorée par la liste. Une déclaration devient **une** entrée de `model.list`,
et les GGUF qu'elle nomme n'y figurent plus seuls. Une déclaration qui nomme un GGUF absent
reste listée : son chargement rend `ai.model_not_found`, avec le seul nom du fichier. Un
fichier qui n'est pas un adaptateur de cette base rend `ai.adapter`, et rien n'est gardé en
mémoire.

La chaîne d'entraînement écrit les deux déclarations dans son dossier de sortie, aussi à la
reprise (`--reprendre`) :

| Déclaration | Modèle |
|---|---|
| `cassini-composition-4b-adaptateur.modele.json` | base Q4_K_M + adaptateur, prompt court |
| `cassini-composition-4b.modele.json` | modèle fusionné, prompt court |

### Le prompt court dans `ai/`

`ai/src/plan.rs` porte une copie du texte des consignes de `training/src/prompt.rs`
(`SHORT_SYSTEM`) : le produit n'embarque pas la chaîne d'entraînement, qui n'est qu'une
dépendance de développement de `cassini-ai`. Deux tests tiennent les deux textes ensemble :

- sans modèle, pour chacun des 8 000 exemples engendrés, les tours que `ai` assemble valent
  `render_prompt`, octet pour octet, encadrement chatml compris ; et pour chaque exemple de
  redemande, la sortie rejetée jugée par `ai` donne les mêmes écarts, et les tours de
  `retry_turns` valent le prompt de redemande de l'entraînement (`render_retry_prompt`) ;
- avec le vrai modèle (`CASSINI_TEST_MODEL`, la base Q4_K_M), pour 300 exemples, les jetons
  que `ai` lit par segments sont ceux du prompt d'entraînement découpé d'un tenant par le
  découpeur de llama.cpp du même GGUF (l'égalité avec celui de Hugging Face, qui a servi à
  l'entraînement, n'est pas vérifiée par ce test).

Le prompt court se coupe au même endroit que le long : la fin des consignes est le point
d'instantané, l'inventaire vient ensuite, puis la demande. Même grammaire, même jugement
par `check_plan`, même redemande unique. L'adaptateur est initialisé une fois au chargement
et posé sur chaque contexte neuf, avant la reprise d'un instantané ; base et adaptateur sont
gardés ensemble, sous une seule clé : un autre adaptateur sur la même base fait tout
recharger et oublier les instantanés.

`cassini plan --model` prend un GGUF ou une déclaration ; sans `--model`, et dans le canal
de l'extension quand aucun modèle n'est choisi, le noyau prend la première entrée de
`model.list` sans problème, par ordre de nom (une seule règle depuis 5.8). Le sélecteur des
Réglages (5.14) choisit entre modèles.

### Installer le modèle ajusté

Copier le contenu du dossier de sortie (les trois GGUF et les deux déclarations) dans un
dossier de modèles : celui que désigne `CASSINI_MODELS`, ou `models/` à côté de
l'exécutable. Vérifier :

```
cassini run model.list '{}'
```

La liste rend les deux déclarations et aucun des trois GGUF seul. Attention : sans
`--model`, la première entrée par ordre de nom l'emporte. Dans un dossier où se trouve
aussi `Qwen3.5-9B-Q4_K_M.gguf`, c'est lui (la majuscule passe avant les minuscules).

### Démonstration

Le 2026-10-22, binaire `cassini` construit sous Windows avec `--features cassini-ai/cuda`
(RTX 3070 Ti), inventaire `tests/epreuve-modele-local/inventory.json`, phrase « Trouve les
parcelles de la zone U à moins de 500 m d'une gare » :

```
cassini plan "Trouve les parcelles de la zone U à moins de 500 m d'une gare" --inventory tests/epreuve-modele-local/inventory.json --model <dossier>/cassini-composition-4b-adaptateur.modele.json
```

| Modèle | Prompt | Résultat | Durée (chargement compris) |
|---|---|---|---|
| base + adaptateur (déclaration) | court | plan valide : sélection `typezone = U`, tampon de 500 m sur les gares, deux intersections | 32 s |
| fusionné (déclaration) | court | même plan | 24 s |
| Qwen3.5-9B (GGUF seul) | long | plan valide, mêmes étapes, intersections dans l'autre ordre | 33 s |

Le dossier de sortie de la tâche 2 a été repris par la chaîne (`--reprendre`, une minute :
les deux déclarations écrites, l'essai par les deux chemins refait) ; `model.list` sur sa
copie rend bien deux entrées. Ces durées ne sont pas une mesure : la mesure du jeu tenu à
l'écart et de la durée sur CPU est la tâche 4.

## L'inventaire vu par le modèle dans QGIS

Tâche 4a du ticket 5.10. Dans QGIS, le modèle ne reçoit pas la sortie de
`project.inventory` telle quelle : le noyau la traduit d'abord dans la forme exacte des
inventaires de l'entraînement (`core/src/agent_inventory.rs`, appelée par `plan.propose`
avant la composition).

**Ce que rapporte l'extension** (faits bruts de QGIS, tous facultatifs : une extension plus
ancienne ne les rend pas, et les clés d'avant gardent leur sens) :

| Clé | Contenu |
|---|---|
| `project_name` | nom du fichier du projet, sans extension ; `null` si le projet n'est pas enregistré |
| `layers[].kind` | `vector`, `raster` ou `other` |
| `layers[].geometry` | nom WKB de QGIS (`MultiPolygon`, `PointZ`, `NoGeometry`…) ; `null` hors vecteur |
| `layers[].feature_count` | nombre d'objets d'une couche vecteur |
| `layers[].fields` | `{name, type, values?}` : type tel que QGIS le nomme ; pour un champ ni numérique ni date, au plus 11 valeurs distinctes non nulles, triées |
| `extent.lonlat` | `{west, south, east, north, crs: "EPSG:4326"}` : l'union, en degrés WGS84, des étendues des couches reprojetées par QGIS ; une couche sans SCR valide, dont la reprojection échoue, ou dont l'étendue n'est pas finie ou sort de [-180, 180] × [-90, 90] reste hors de l'union ; la clé manque quand aucune couche n'y entre (2026-11-08) |

L'extension ne décide rien : elle lit au plus 11 valeurs par champ (une de plus que le
plafond du noyau, pour qu'il sache qu'il y en a trop), ne parcourt pas le disque et ne modifie
aucun fichier.

**Ce que décide le noyau** :

- forme `{"project": {name, epsg, utm_epsg}, "layers": [...], "unloaded_files": []}` ; par couche,
  `layer_id`, `name`, `kind`, `geometry`, `epsg`, `feature_count`, `source`, `fields` ;
- SCR : `EPSG:2154` devient `2154` ; un SCR qui n'est pas EPSG (`IGNF:LAMB93`) n'a pas de clé
  `epsg` ;
- types : `Integer`, `Integer64` et leurs voisins deviennent `integer` ; `Real`, `Double` et
  leurs voisins `number` ; tout le reste (`String`, `Date`, type inconnu) `string` ;
- valeurs : seulement sur un champ texte, 10 au plus (le jeu tenu à l'écart en montre 10 au
  plus, l'entraînement 9) ; au-delà, ou sans valeur, le champ n'a pas de clé `values` ;
- géométrie : ramenée aux six de l'entraînement (`PointZ` devient `Point`, `MultiSurface`
  `MultiPolygon`) ; sans géométrie ou inconnue, `null` ;
- raster : `geometry: null`, `fields: []`, et pas de `feature_count`, que QGIS n'a pas pour un
  raster (c'est le seul écart de forme avec le générateur, qui en tire un au hasard) ;
- `unloaded_files` vaut toujours `[]`.
- fuseau du projet (2026-11-08) : `utm_epsg` est le code EPSG du fuseau UTM normal du centre
  de `extent.lonlat` (l'emprise en degrés WGS84, reprojetée par QGIS), 326xx au nord, 327xx au
  sud ; le noyau ne convertit aucune coordonnée (AD-19). Le centre décide quand l'emprise
  chevauche deux fuseaux. Pas de clé sans `lonlat`, sans son code `EPSG:4326`, avec une valeur
  hors bornes, ouest après est, sud après nord, une emprise de plus de 12° de longitude (plus de
  deux fuseaux : couche du monde, ou antiméridien passé, de -180 à 180), ou un centre hors du
  domaine de l'UTM (80° S à 84° N) : un carroyage « UTM » sans fuseau mène alors à la question.
  Dans le générateur, `utm_epsg` est celui des couches en UTM (vecteur ou MNT), sinon 31N
  (projets du Val-de-Marne), et il manque comme l'`epsg` dans une part des formes de QGIS
  (une fois sur quatre de celles-ci) ;

`training/tests/forme_inventaire.rs` traduit une sortie représentative de `project.inventory`
et la confronte au jeu engendré (graine 1) : mêmes clés, mêmes sortes de valeurs, types et
géométries déjà vus. Si le générateur change de forme, ce test échoue et la traduction se met
à jour avec le ré-entraînement. `cassini plan --inventory` prend toujours un inventaire déjà
dans la forme apprise : il n'est pas traduit.

**Mesure sur le projet réel** (2026-10-23, QGIS 3.44.15, `projet-test-villeneuve.qgz`, sonde
jetable hors dépôt) : six couches ; la couche `plu_zones` porte `typezone` avec ses valeurs
`["N", "Nh", "U"]`. L'inventaire traduit fait 5 289 caractères, soit **2 439 jetons** du GGUF
de base (`qwen3.5-4b-base-Q4_K_M.gguf`, découpeur de llama.cpp), contre 3 196 pour la sortie
brute. C'est plus que la plus longue séquence entière vue à l'entraînement (1 326 jetons) : le
PLU réel porte 24 champs, dont des empreintes et des libellés longs. L'effet sur le modèle
ajusté se mesure à la tâche 4b.

### Budget d'inventaire (5.18)

Chaque plan exécuté ajoute ses couches de résultat au projet, donc à l'inventaire : à la recette
enchaînée de 5.17, la 4ᵉ phrase ne tenait plus dans le contexte (8 192 jetons). Le noyau borne
donc l'inventaire vu par le modèle (`core/src/agent_inventory.rs`, `within_budget_by_batches` depuis 5.19) : sous le
budget, il est rendu tel quel ; au-delà, toutes les couches de l'utilisateur restent, avec les
couches de résultat les plus récentes qui tiennent, dans l'ordre de l'inventaire. Une couche de
résultat est une couche dont le fichier source est dans le dossier de travail `<projet>.cassini/`
ou dans son dossier de rejeu `replay/` ; la plus récente est celle dont le fichier est le plus
récent (à date égale, le plus grand numéro d'étape). Rien n'est retiré de QGIS et une couche
gardée n'est pas modifiée. Une couche retirée reste donc dans le projet : un plan qui la cite
s'exécute, seul le modèle ne la voit plus.

Le budget est en **caractères** du texte JSON compact de l'inventaire, parce que le noyau ne
tokenise pas. Mesure du 2026-10-24, tokeniseur du modèle livré
(`cassini-composition-4b-Q4_K_M.gguf`), test ignoré
`the_inventory_budget_fits_the_context_with_the_real_tokenizer` de `ai/src/plan.rs` :

| Mesure | Caractères | Jetons | Jeton par caractère |
|---|---|---|---|
| inventaire de Villeneuve (6 couches) | 4 007 | 1 458 | 0,364 |
| couche d'intersection à 34 champs (parcelles × zones du PLU) | 1 999 | 724 | 0,362 |
| tout le reste au premier essai (consignes, armature, longue phrase) | — | 359 | — |
| ajout de la redemande au pire (sortie recopiée à 1 500 caractères, 8 écarts) | — | 1 106 | — |

Il reste donc 8 192 − 1 024 (réponse) − 359 − 1 106 = 5 703 jetons pour l'inventaire, soit
15 600 caractères au rapport mesuré. Le budget est fixé à **14 000 caractères** : Villeneuve
plus quatre intersections et une cinquième coupée (13 972 caractères) donnent une redemande de
6 501 jetons, plus 1 024 de réponse, soit 7 525 sur 8 192. Le budget tient tant que
l'inventaire ne dépasse pas 0,41 jeton par caractère. Le contrôle exact de `cassini-ai`
(`ai.prompt_too_long`) reste le dernier filet ; son message nomme désormais l'inventaire du
projet et conseille d'en retirer des couches. Le budget vise le prompt court du modèle livré :
le long prompt, plus lourd de son catalogue, garde son erreur typée.

**Recette enchaînée** (2026-10-24, QGIS 3.44.15, `projet-test-villeneuve.qgz`, noyau du
produit sur processeur, modèle livré seul dans `CASSINI_MODELS`, sonde jetable hors dépôt sur le
modèle de `qgis-plugin/tests/probe_demo.py`) : les dix phrases de la recette de 5.17, dans une
même session, chaque plan exécuté. Aucune `ai.prompt_too_long` (en 5.17 : phrases 4 à 10) ; le
projet passe de 6 à 18 couches. Les phrases 1 à 3 rendent les mêmes plans et les mêmes nombres
qu'en 5.17 (5 étapes, 2 429 parcelles ; une sélection, 137 zones ; 3 étapes, 2 513 parcelles,
« ces zones » toujours mal reprises, hors ticket). Les phrases 4 à 10 rendent ce qu'elles
rendaient une à une sur projet vierge en 5.17 : PDF, tampon puis intersection (40 576), question
du garde-fou, isochrone puis intersection, refus, question du modèle, refus.

Seconde recette enchaînée, après la relecture (même poste, même modèle, dix autres phrases :
démo, zones U, parcelles dans ces zones près d'une gare, tampon de 200 m, routes à 30 km/h,
bâtiments près de ces routes, altitude moyenne du MNT par zone, zones N, démo de nouveau, PDF) :
aucune `ai.prompt_too_long`, le projet passe de 6 à 19 couches (39 289 caractères bruts).
Huit plans exécutés, un refus juste (statistiques sur un raster). Constat : la phrase de démo,
juste en 1ʳᵉ position, rend en 9ᵉ position une question (« Je ne vois pas de champ de type de
zone dans la couche plu_zones »), alors que `plu_zones` et `typezone` sont dans l'inventaire
vu. L'inventaire au budget (14 000 caractères) est bien plus gros que ceux de l'entraînement
(couches citées plus une à quatre de distraction) : la justesse du modèle sur de longs
inventaires de session reste à apprendre (`deferred-work.md`).

Jeu de mesures sur GPU après le changement, modèle livré : `plan_reel` 25 / 25 (0 question),
`question_reel` 12 / 12, `ecart_reel` 33 / 40 (plans 19 / 26, questions 8 / 8, refus 6 / 6),
les mêmes qu'en 5.17 : ces essais passent par `cassini-ai` et des inventaires sous le budget.

### Ordre stable et identifiants courts (5.19)

Pour que la lecture d'une phrase reprenne l'état du moteur au plus près, l'inventaire vu par le
modèle est rangé dans un ordre stable (`stable_order`) : les couches de l'utilisateur d'abord,
dans l'ordre de QGIS, puis les couches de résultat de la plus ancienne à la plus récente (l'ordre
inverse du budget). Un plan exécuté n'ajoute donc ses couches qu'à la fin. L'entraînement mélange
l'ordre des couches : cet ordre est un ordre déjà vu.

Le modèle voit aussi un identifiant court pour chaque identifiant QGIS `nom_<uuid>` : `nom_` suivi
des 4 premiers chiffres hexadécimaux de l'uuid (`parcelles_e9ba` pour
`parcelles_e9ba733c_8a43_4717_ac73_04344c1b6bb5`), la forme engendrée pour un tiers des couches
de l'entraînement. Deux courts qui se confondent reçoivent un chiffre de plus, ensemble, jusqu'à
être uniques ; tout autre identifiant est montré tel quel. La table est faite sur toutes les
couches avant le budget, pour qu'un court ne change pas quand une couche sort de la vue. Le noyau
retraduit le plan rendu vers les identifiants complets avant de le vérifier (`plan.propose`) : le
plan gardé, montré, exécuté et journalisé ne porte que les vrais identifiants ; un court inconnu
reste tel quel et le plan est refusé comme avant. Les essais réels `plan_reel`, `question_reel`
et `ecart_reel` passent par la même table (`CASSINI_TEST_FULL_IDS=1` pour les identifiants
complets). Mesures : `mesure-duree-par-phrase.md`, « Session enchaînée dans QGIS ».

Le budget de 5.18 retire désormais **par paquets** (décision du 2026-10-29) : quand la vue dépasse
14 000 caractères, les résultats les plus anciens sortent jusqu'à descendre sous 7 000
(`within_budget_by_batches`). La décision est rejouée à chaque phrase sur les résultats présents,
du plus ancien au plus récent : les résultats gardés restent les mêmes jusqu'au dépassement
suivant, et l'invite d'avant reste un préfixe de la nouvelle.

L'invite reste celle de l'entraînement à l'octet près : `cassini-ai` coupe seulement sa lecture
juste avant la dernière clé de chaque couche, là où le découpeur de Qwen ne change aucun jeton
(prouvé au vrai tokeniseur sur 300 inventaires engendrés, 1 357 coupes, et sur les 339 redemandes
du jeu entier).

## Tâche 4 : mesures

Tâche 4b du ticket 5.10, le 2026-10-23, au commit `540f59c` plus les changements de la tâche :
les deux déclarations de la tâche 2 (`C:\Users\kevin\cassini-modeles\ajuste-5-10-t2\`, prompt
court) et, pour comparaison, Qwen3.5-9B Q4_K_M seul (long prompt). Décision du propriétaire
(2026-10-23) : un chiffre sous la barre est consigné tel quel, la tâche se ferme comme
« mesurée » et un ré-entraînement est proposé ; le ticket 5.10 reste ouvert.

### Ce qui a changé pour mesurer

- `cassini_ai::plan::resolve_model` lit un modèle d'une seule façon (GGUF seul → long prompt ;
  déclaration → base, adaptateur et prompt qu'elle nomme), pour la CLI et pour les essais. Une
  déclaration absente rend `ai.model_not_found`, une déclaration illisible
  `models.bad_declaration`, sans recopier son dossier.
- Les essais réels `ecart_reel`, `question_reel`, `plan_reel` et `duree_reel` prennent dans
  `CASSINI_TEST_MODEL` un GGUF ou une déclaration (`ai/tests/common/mod.rs`).
- `plan_reel` : au plus 2 fausses questions sur 25 pour le long prompt (concession du
  2026-10-17), au plus 1 pour le prompt court (plafond strict du ticket).
- `ecart_reel` : le notateur est une fonction (`grade`) testée sans modèle. Pour chaque phrase
  manquée, il imprime la forme de l'écart (sorte, nombre d'étapes, ordre, commande ou nom du
  paramètre à l'étape k), jamais une valeur ni le texte ; la ligne `REDEMANDES` compte les
  phrases passées par la redemande et celles qu'elle a sauvées. Les étapes « dans un autre
  ordre » se reconnaissent en remplaçant chaque `step:k` par l'étape k elle-même.
- `duree_reel` : le scénario commence par `A0`, la phrase de démo exacte de `CLAUDE.md`, et
  passe d'abord une amorce (« Décris le projet. ») qui paie seule le chargement et la lecture
  de l'invite jusqu'à l'instantané : la ligne de `A0` ne mesure que sa demande, comme une
  demande dans une session de `cassini serve` déjà ouverte.

### Parité des découpeurs

`training/entrainement/parite_decoupeurs.py` (lancée à la main, dans l'environnement `uv` de
la chaîne) découpe les prompts d'un jeu `--format chat` par le découpeur de Hugging Face de
la base (révision épinglée, `add_special_tokens=False`, comme l'entraînement) et par
`llama-tokenize` du llama.cpp du cache (`--ids --no-bos`) sur le GGUF de base :

```
UV_PROJECT_ENVIRONMENT=~/.cache/cassini-entrainement/venv \
  uv run --project training/entrainement python training/entrainement/parite_decoupeurs.py \
  --gguf ~/cassini-entrainement/5-10-t2/qwen3.5-4b-base-Q4_K_M.gguf \
  --jeu ~/cassini-entrainement/5-10-t2/travail/donnees/validation.jsonl
```

Résultat : **300 / 300 prompts identiques** (205 803 jetons), aucun écart. Avec le test de la
tâche 3 (jetons de `ai` lus par segments = découpage d'un tenant par llama.cpp), le modèle voit
dans le produit le découpage de son entraînement.

### Jeu tenu à l'écart (GPU)

`cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture ecart_reel`,
RTX 3070 Ti, décodage glouton :

| Modèle | Prompt | Plans | Questions | Refus | **Total** | Redemandes (sauvées) | Durée |
|---|---|---|---|---|---|---|---|
| base + adaptateur | court | 19 / 26 | 8 / 8 | 5 / 6 | **32 / 40** | 1 (0) | 414 s |
| fusionné | court | 20 / 26 | 8 / 8 | 5 / 6 | **33 / 40** | 2 (0) | 595 s |
| Qwen3.5-9B (référence) | long | 19 / 26 | 7 / 8 | 6 / 6 | **32 / 40** | 1 (1) | 389 s |

La référence de la tâche 1 (32/40) se retrouve à l'identique. Le modèle ajusté l'**égale**
(base + adaptateur) ou la **dépasse d'une phrase** (fusionné), avec un modèle deux fois plus
petit et un prompt bien plus court. La phrase de démo (H01) est juste pour les deux chemins ;
le 9B la manque (opérateur de la sélection).

Formes des écarts du modèle ajusté (les mêmes pour les deux chemins, à H11 et H20 près) :

| Phrase | Écart |
|---|---|
| H08 | une étape de trop (2 au lieu de 1) |
| H17, H18 | une étape de moins (3 au lieu de 4) |
| H19 | une étape de moins (5 au lieu de 6) |
| H20 | une étape de moins (adaptateur) ; plan invalide deux fois (fusionné) |
| H22 | une étape de trop (4 au lieu de 3) |
| H11 | paramètre `origin` de l'isochrone (adaptateur seulement) |
| H40 | refus attendu : un plan après redemande (adaptateur), un plan invalide deux fois (fusionné) |

Les plans de plusieurs conditions (intersections successives) sont la faiblesse : quatre
plans sur sept manqués perdent une étape. Les questions sont parfaites (8/8), là où le 9B
transforme H32 en refus.

**Valeur de la redemande** : 1 à 2 phrases sur 40 y passent, aucune n'est sauvée par le
modèle ajusté (le 9B en sauve une). Le modèle ajusté n'a jamais appris ce tour : il est gardé
(AD-23, une seule redemande), mais ne rapporte rien aujourd'hui.

### Seuil de clarification (GPU)

| Essai | base + adaptateur | fusionné | Seuil |
|---|---|---|---|
| `question_reel` (12 phrases ambiguës) | **11 / 12** questions | **11 / 12** | au moins 9 : tenu |
| `plan_reel` : fausses questions sur 25 phrases claires | **0** | **0** | au plus 1 : tenu |
| `plan_reel` : plans conformes | **16 / 25** | **16 / 25** | 80 % (20) : **sous la barre** |

`plan_reel` **échoue** donc son assertion des 80 % avec les deux déclarations (16/25) ; son
plafond de fausses questions (0, au plus 1) est tenu.

Q11 (« qui appartiennent à la commune ») rend un plan d'intersection au lieu d'une question.
Les neuf phrases manquées de `plan_reel` (S10, S15, S18, S20 à S25) attendent toutes un
**refus** : cadrage de la carte, altitude par commune (statistiques sur un raster), PDF en A3,
et six demandes de terrain (champ de vision, ligne de vue, pente) que le registre n'a pas
encore. Le modèle ajusté y rend un plan de substitution (tampon, sélection, statistiques,
isochrone) ou un plan invalide, jamais une question. Le générateur n'a aucune famille de refus
pour ces demandes : c'est à apprendre, pas à corriger dans le prompt.

### Durée sur processeur (4 fils, 4 cœurs physiques)

`duree_reel`, binaire **sans CUDA**, `CASSINI_TEST_THREADS=4`, masque d'affinité `0x55` sur le
Ryzen 7 5800X, inventaire traduit du projet réel
(`../donnees-test-cassini/inventaire-villeneuve-traduit-4b.json`, hors dépôt : 4 007 caractères,
1 458 jetons du GGUF de base, par le noyau et l'extension actuels ; la tâche 4a en comptait
2 439, chiffre pris avant les correctifs de sa relecture, et aucun filtre de longueur n'explique
l'écart). Modèle fusionné :

| id | appels | chargement (s) | lecture du prompt (s) | jetons de l'invite | jetons repris | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|---|---|---|
| amorce | 1 | 2,5 | 28,5 | 1737 | 0 | 2,3 | 17 | 33,4 | plan, 1 étape(s) |
| **A0** | 1 | 0,0 | 1,0 | 1768 | 1719 | 40,4 | 248 | **41,4** | plan, 5 étape(s) |
| A | 1 | 0,0 | 1,1 | 1772 | 1719 | 40,9 | 252 | 42,0 | plan, 5 étape(s) |
| B1 | 1 | 0,0 | 0,5 | 1740 | 1719 | 11,4 | 71 | 11,9 | plan, 1 étape(s) |
| B2 | 1 | 0,0 | 0,7 | 1751 | 1719 | 19,6 | 120 | 20,3 | plan, 2 étape(s) |
| B3 | 1 | 0,0 | 0,5 | 1740 | 1719 | 3,3 | 23 | 3,9 | plan, 1 étape(s) |
| C | 1 | 0,0 | 0,6 | 1746 | 1719 | 20,6 | 124 | 21,3 | plan, 2 étape(s) |
| D | 1 | 0,0 | 0,5 | 1740 | 1719 | 4,3 | 27 | 4,8 | question |
| E | 2 | 0,1 | 4,4 | 3685 | 3438 | 39,4 | 242 | 43,8 | erreur : agent.plan_invalid |

Moyennes hors amorce : 23,7 s. Base + adaptateur : A0 en **43,2 s** (écriture 42,0 s),
moyenne 26,8 s.

**Verdict A0 : 41,4 s, au-dessus des 30 s.** La lecture de l'invite est réglée : l'invite
entière ne fait plus que 1 737 jetons (contre 5 130 au long prompt), lue en 28,5 s une fois par
session, puis 1 s par demande. Il reste l'écriture des 248 jetons du plan à cinq étapes, à
environ 6 jetons/s, bornée par la mémoire (Qwen3.5-9B : environ 4 jetons/s au même réglage,
relevé du 2026-10-18 de `mesure-duree-par-phrase.md`, « Après le cache »). C'est la longueur du plan
écrit qu'il faut réduire (identifiants de couche longs recopiés à chaque étape, titre du PDF),
ou le débit d'écriture qu'il faut augmenter.

### Démo de bout en bout dans QGIS

Le 2026-10-23, QGIS 3.44.15 (Python de QGIS, plateforme offscreen), projet
`projet-test-villeneuve.qgz`, sonde `qgis-plugin/tests/probe_demo.py` (lancée à la main sous
`python-qgis-ltr.bat`, `CASSINI_CORE` et `CASSINI_MODELS` posés, projet en argument ; refaite après
son versionnement : même plan, PDF lisible, inventaire 0,05 s) : vrai `cassini serve` lancé par
`qgis-plugin/cassini_qgis/client.py`, avec les vrais gestes de l'extension. Noyau construit
avec `--features cassini-ai/cuda` ; `CASSINI_MODELS` sur un dossier qui ne contient que le
modèle fusionné et sa déclaration (liens durs), le dossier des modèles du poste n'est pas
touché. Phrase exacte de `CLAUDE.md` :

> Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi
> et sors-moi la carte en PDF.

| Étape | Résultat | Durée |
|---|---|---|
| `project.inventory` (geste QGIS, demandé par `plan.propose`) | 6 couches ; traduit par le noyau, identique octet pour octet à l'inventaire de la mesure CPU | **0,05 s** |
| Composition (`plan.propose`, chargement du modèle compris) | plan de 5 étapes, validé | 24,8 s |
| 1. `analysis.select_by_attribute` `typezone = U` sur `plu_zones` | 137 zones | 0,17 s |
| 2. `analysis.buffer` 500 m autour des gares | 8 objets | 0,03 s |
| 3. `analysis.intersect` parcelles × zones U | 6 792 objets | 3,7 s |
| 4. `analysis.intersect` (étape 3) × tampon (étape 2) | 2 429 objets | 1,8 s |
| 5. `layout.export_pdf`, titre « Parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi » | `30-layout.export_pdf.pdf`, 1 page, 3,2 Mo | 1,3 s |

**Réussite** : le plan enchaîne sélection de la zone U, tampon de 500 m sur les gares, deux
intersections et PDF titré, il s'exécute sans erreur, le PDF est écrit à côté du projet (`projet-test-villeneuve.cassini/`) et les cinq
étapes sont au journal. `project.inventory` ne coûte que 0,05 s sur ce projet : la mise en
cache des valeurs par session n'est pas nécessaire. Le chemin complet (`serve`, canal,
`plan.propose`, inventaire traduit, vrai modèle) est vérifié.

### Bilan contre la barre du ticket

| Barre | Mesure | |
|---|---|---|
| ≥ 32/40 sur le jeu tenu à l'écart | 32/40 (adaptateur), 33/40 (fusionné) | tenue |
| ≥ 9/12 questions | 11/12 | tenue |
| ≤ 1 fausse question sur 25 | 0 | tenue |
| `plan_reel` ≥ 80 % | 16/25, l'essai échoue | **sous la barre** (refus de terrain, cadrage, A3, raster jamais appris) |
| < 30 s par longue phrase, CPU 4 fils, à chaud | A0 : 41,4 s | **sous la barre** (écriture du plan) |
| première demande d'une session, CPU 4 fils | chargement 2,5 s + lecture complète de l'invite 28,5 s (amorce), puis A0 41,4 s : environ 72 s (somme des deux lignes, non mesurée d'un seul appel) | **sous la barre** |
| démo QGIS, phrase exacte | plan juste, exécuté, PDF | tenue |

Déclaration retenue : le **modèle fusionné** (`cassini-composition-4b.modele.json`), meilleur
d'une phrase et un peu plus rapide. Un ticket de ré-entraînement est proposé (voir
`deferred-work.md`).

## Tâche B de 5.15 : ré-entraînement et mesures

Le générateur de la tâche A (nouvelles familles, redemande, inventaires de QGIS, prompt court
« en comptant à partir de 1 ») a été entraîné par la commande unique, sans autre changement :
8 000 exemples, même base, même méthode, même révision de llama.cpp.

### Entraînement

| Mesure | Valeur |
|---|---|
| Exemples | 8 000 d'entraînement, 300 de validation, aucun écarté car trop long |
| Jetons par séquence | médiane 735, maximum 1 521 (la limite de 2 048 suffit, redemandes comprises) |
| Validation (graine 2, glouton) | 299 / 300 : plans 231 / 232, questions 37 / 37, refus 31 / 31 |
| Durée totale | 241 min (RTX 3070 Ti, WSL) |
| Essai `llama-completion` | base + adaptateur et fusionné rendent le plan attendu |

### Parité des découpeurs

- `parite_decoupeurs.py` (Hugging Face contre llama.cpp, d'un tenant) : **300 / 300**, 0 écart.
- Test de `ai` sur le vrai modèle (la base `qwen3.5-4b-base-Q4_K_M.gguf` sortie par cet
  entraînement), étendu depuis 5.15 au tour de redemande : 300 premiers essais et
  **349 redemandes** du jeu de 8 000, mêmes écarts que le jugement de `ai`, même point
  d'instantané de la tête fixe, jetons de `ai` (par segments) égaux à ceux du
  prompt d'entraînement lu d'un tenant.

### Essais réels (GPU, modèle fusionné, prompt court)

| Essai | 5.10 (fusionné) | 5.15 |
|---|---|---|
| `plan_reel` | 16 / 25 | **25 / 25**, 0 question |
| `question_reel` | 11 / 12 | **12 / 12** |
| `ecart_reel` | 33 / 40 | **32 / 40** (plans 19 / 26, questions 8 / 8, refus 5 / 6), 316 s |
| redemandes sur `ecart_reel` | 2 (0 sauvée) | 1 (0 sauvée) |

Formes des écarts du jeu tenu à l'écart :

| Phrase | Écart |
|---|---|
| H08, H23 | une question au lieu d'un plan |
| H17, H18, H20 | une étape de moins |
| H19 | paramètre `operator` de l'étape 1 |
| H22 | commande de l'étape 2 |
| H40 | un plan au lieu du refus attendu |

Sur le jeu tenu à l'écart, le modèle de 5.15 rend **une phrase juste de moins** que le
fusionné de 5.10 (32 contre 33). Les plans à plusieurs conditions restent sa faiblesse, alors que les
phrases de l'épreuve sont désormais toutes justes. Ces formes nomment des constats ; elles
ne disent rien du texte des phrases, qui n'est jamais lu.

### Durée sur processeur (4 fils, 4 cœurs physiques)

Même recette qu'en 5.10 (affinité `0x55`, `CASSINI_TEST_THREADS=4`, inventaire de
Villeneuve-le-Roi traduit) :

| id | appels | lecture du prompt (s) | écriture (s) | jetons écrits | total (s) | issue |
|---|---|---|---|---|---|---|
| amorce | 1 | 26,7 (+ chargement 2,5) | 2,0 | 17 | 31,2 | plan, 1 étape |
| A0 | 1 | 1,0 | 36,7 | 248 | 37,7 | plan, 5 étapes |
| A | 1 | 1,1 | 39,4 | 252 | 40,5 | plan, 5 étapes |
| B1 | 1 | 0,5 | 10,3 | 71 | 10,8 | plan, 1 étape |
| B2 | 1 | 0,6 | 26,4 | 180 | 27,1 | plan, 3 étapes |
| B3 | 1 | 0,5 | 2,9 | 23 | 3,5 | plan, 1 étape |
| C | 1 | 0,6 | 18,2 | 124 | 18,8 | plan, 2 étapes |
| D | 1 | 0,5 | 3,9 | 27 | 4,4 | question |
| E | 2 | 4,7 | 44,7 | 304 | 49,5 | plan, 2 étapes |

Moyennes hors amorce : 24,0 s. A0 passe de 41,4 s à 37,7 s ; la durée n'est pas un critère
de 5.15. L'exigence produit (une longue phrase en moins de 30 s sur processeur) reste
manquée par A0, A et E ; elle est suivie dans `deferred-work.md`.

**Phrase E** (« … à 10 minutes à pied de la gare de Villeneuve-le-Roi. ») : un plan valide
après redemande, au lieu de la question attendue. Le modèle **invente l'origine** de
l'isochrone (`origin` en EPSG:4326, x = 2,458, y = 48,658, à peu près Villeneuve-le-Roi) et
croise les parcelles avec la zone obtenue. La couche des gares est dans l'inventaire, mais
rien n'y donne le point en coordonnées. En 5.10, la même phrase finissait en plan invalide.

### Bilan contre la barre du ticket

| Barre | Mesure | |
|---|---|---|
| `plan_reel` ≥ 20 / 25 | 25 / 25 | tenue |
| ≤ 1 fausse question sur 25 | 0 | tenue |
| `ecart_reel` ≥ 32 / 40 | 32 / 40 | tenue, de justesse |
| `question_reel` ≥ 9 / 12 | 12 / 12 | tenue |
| phrase E rendue en question | un plan, origine inventée | **sous la barre** |

Barre manquée sur la phrase E seulement. Décision du propriétaire pour ce cas : consigner et
s'arrêter. La démo QGIS n'est pas rejouée, le modèle n'est pas installé dans `models/` ; les
sorties restent hors du dépôt, dans le dossier des modèles de la machine (`ajuste-5-15`). Une nouvelle passe sur
le générateur (isochrone dont l'origine est une couche de points présente sans le point
choisi, et plans à plusieurs conditions) fera l'objet d'un autre ticket.

## Tâche B de 5.17 : ré-entraînement et mesures

Le générateur de la tâche A de 5.17 (commit `ac11428` : trajet depuis un lieu de
l'inventaire, origines multiples, plans à plusieurs conditions) a été entraîné par la
commande unique, sans autre changement : 8 000 exemples, même base (`Qwen/Qwen3.5-4B` @
`851bf6e`), même méthode, même révision de llama.cpp (`d812350`).

### Entraînement

| Mesure | Valeur |
|---|---|
| Exemples | 8 000 d'entraînement, 300 de validation, aucun écarté car trop long |
| Jetons par séquence | médiane 749, maximum 1 659 (la limite de 2 048 suffit) |
| Perte finale | 0,0044 (500 pas) |
| Validation (graine 2, glouton) | 298 / 300 : plans 236 / 237, questions 35 / 36, refus 27 / 27 |
| Durée totale | 330 min, dont 49 min de validation (RTX 3070 Ti, WSL) |
| Essai `llama-completion` | base + adaptateur et fusionné rendent le plan attendu |
| Modèle fusionné | `cassini-composition-4b-Q4_K_M.gguf`, SHA-256 `6bde9803c06b85d796255c35d533f9f856681a8ee66049416100badf185a8084` |

La durée passe de 241 à 330 min pour le même nombre d'exemples ; les séquences sont un peu
plus longues (médiane 749 contre 735), ce qui n'explique pas tout. La cause n'est pas établie.

### Parité des découpeurs

- `parite_decoupeurs.py` : **300 / 300**, 0 écart.
- Test de `ai` sur la base `qwen3.5-4b-base-Q4_K_M.gguf` sortie par cet entraînement :
  300 premiers essais et **339 redemandes** identiques au jeton.

### Essais réels (GPU, modèle fusionné, prompt court)

Les scores de justesse se mesurent sur le GPU (décision du propriétaire du 2026-10-17) ; ils ne sont pas
refaits sur le processeur.

| Essai | 5.15 | 5.17 |
|---|---|---|
| `plan_reel` | 25 / 25, 0 question | **25 / 25**, 0 question, 0 du garde-fou |
| `question_reel` | 12 / 12 | **12 / 12**, 0 du garde-fou |
| `ecart_reel` | 32 / 40 | **33 / 40** (plans 19 / 26, questions 8 / 8, refus 6 / 6), 331 s |
| redemandes sur `ecart_reel` | 1 (0 sauvée) | 2, sur H17 et H19 (0 sauvée) |

Formes des écarts du jeu tenu à l'écart :

| Phrase | Écart en 5.17 | En 5.15 |
|---|---|---|
| H08 | une question au lieu d'un plan | même écart |
| H13 | paramètre `label_field` de l'étape 1 | juste |
| H17, H18 | une étape de moins | même écart |
| H19 | une étape de moins | paramètre `operator` de l'étape 1 |
| H20 | une étape de plus | une étape de moins |
| H22 | une étape de plus | commande de l'étape 2 |
| H23 | juste | une question au lieu d'un plan |
| H40 | juste | un plan au lieu du refus attendu |

H23 et H40 deviennent justes ; H13 recule. Les plans à plusieurs conditions restent la
faiblesse : le nombre d'étapes y est encore faux, dans les deux sens. Le gain tient à une
phrase ; il se lit avec la réserve de la tâche A : les familles ont visé des écarts nommés,
`ecart_reel` n'est plus tout à fait une mesure aveugle.

### Durée sur processeur (4 fils, 4 cœurs physiques)

Même recette qu'en 5.15 (affinité `0x55`, `CASSINI_TEST_THREADS=4`, inventaire de
Villeneuve-le-Roi traduit) :

| id | appels | lecture du prompt (s) | écriture (s) | jetons écrits | total (s) | 5.15 (s) | issue |
|---|---|---|---|---|---|---|---|
| amorce | 1 | 28,1 (+ chargement 2,5) | 2,2 | 17 | 32,8 | 31,2 | plan, 1 étape |
| A0 | 1 | 1,0 | 39,5 | 248 | 40,5 | 37,7 | plan, 5 étapes |
| A | 1 | 1,1 | 39,7 | 252 | 40,9 | 40,5 | plan, 5 étapes |
| B1 | 1 | 0,5 | 11,1 | 71 | 11,7 | 10,8 | plan, 1 étape |
| B2 | 1 | 0,7 | 28,7 | 180 | 29,4 | 27,1 | plan, 3 étapes |
| B3 | 1 | 0,5 | 3,1 | 23 | 3,7 | 3,5 | plan, 1 étape |
| C | 1 | 0,6 | 19,6 | 124 | 20,3 | 18,8 | plan, 2 étapes |
| D | 1 | 0,5 | 4,2 | 27 | 4,8 | 4,4 | question |
| E | 1 | 0,7 | 5,6 | 32 | 6,4 | 49,5 | question du modèle |

Moyenne hors amorce : 19,7 s (24,0 s en 5.15). Tout le gain vient de E ; les autres phrases
écrivent le même nombre de jetons qu'en 5.15 et prennent de 1 à 8 % de plus, écart de la
machine d'un jour à l'autre. L'exigence produit (une longue phrase en moins de 30 s sur
processeur) reste manquée par A0 et A ; elle est suivie dans `deferred-work.md`.

### La phrase E

« Trouve les parcelles à 10 minutes à pied de la gare de Villeneuve-le-Roi. », sur le même
inventaire traduit, avec la déclaration du modèle de 5.17 (`--model`) :

| Chemin | Réponse |
|---|---|
| `duree_reel` (processeur, 4 fils, tête du prompt reprise d'un instantané) | question du modèle, 32 jetons |
| `llama-completion`, prompt d'entraînement rendu par `cassini-training`, sans grammaire | question du modèle : « Quelles sont les coordonnées du point de départ (X, Y et système, par exemple Lambert-93) ? » |
| `cassini plan`, noyau du produit (processeur) ou noyau CUDA, modèle installé ou copie d'origine | question du **garde-fou** (`"source":"guard"`) |
| panneau QGIS (`cassini serve`, `plan.propose`), noyau installé | question du **garde-fou** |

Le modèle hésite entre la question et un point de départ inventé : la balance penche d'un
côté ou de l'autre selon la façon dont le prompt est calculé (fils, instantané de la tête,
noyau). La cause exacte n'est pas mesurée. Dans le produit, c'est le point inventé qui
l'emporte, et le garde-fou de 5.16 le change en question : l'utilisateur ne reçoit jamais un
plan faux, mais la réponse n'est pas apprise.

Piège de mesure : une phrase passée à `cassini.exe` par un script PowerShell enregistré en
UTF-8 **sans BOM** arrive aux accents abîmés (Windows PowerShell lit le script en ANSI). Une
première mesure de E, faite ainsi, avait donné à tort une question du modèle sur processeur.
Les scripts de mesure s'enregistrent en UTF-8 avec BOM.

Phrases de contraste, même inventaire, noyau et modèle installés (script en UTF-8 avec BOM) :

| Phrase | Réponse |
|---|---|
| « … à 10 minutes à pied du point 655200, 6849800 (Lambert-93). » | `analysis.isochrone` depuis ce point, puis `analysis.intersect` des parcelles |
| « … à 10 minutes à pied de chaque gare. » | refus : l'isochrone part d'un seul point |
| « … à 10 minutes à pied des points 655200, 6849800 et 656100, 6850400 (Lambert-93). » | question du garde-fou : `llama-completion` montre deux origines écrites dans un même objet, sortie rejetée |

### Démo de bout en bout dans QGIS

Le 2026-10-24, même sonde et même projet qu'à la tâche 4 de 5.10. **Noyau du produit**
(processeur, installé dans `%LOCALAPPDATA%\Cassini\bin`), `CASSINI_MODELS` sur un dossier qui
ne contient que le modèle fusionné et sa déclaration. Phrase exacte de `CLAUDE.md`.

| Étape | Résultat | Durée |
|---|---|---|
| `project.inventory` | 6 couches | 0,06 s |
| Composition (`plan.propose`, chargement du modèle et lecture du prompt à froid compris) | plan de 5 étapes, validé | 67,1 s |
| 1. `analysis.select_by_attribute` `typezone = U` sur `plu_zones` | 137 zones | 0,17 s |
| 2. `analysis.buffer` 500 m autour des gares | 8 objets | 0,05 s |
| 3. `analysis.intersect` parcelles × zones U | 6 792 objets | 3,9 s |
| 4. `analysis.intersect` (étape 3) × tampon (étape 2) | 2 429 objets | 1,9 s |
| 5. `layout.export_pdf`, titre « Parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi » | 1 page, 3,0 Mo, PDF lisible | 1,2 s |

**Réussite** : même plan et mêmes nombres qu'à la tâche 4 de 5.10, les cinq étapes au
journal. La composition à froid sur processeur (67 s) compte le chargement et la première
lecture du prompt ; la mesure à chaud est celle de `duree_reel`.

### Bilan contre la barre du ticket

| Barre | Mesure | |
|---|---|---|
| `plan_reel` ≥ 20 / 25 | 25 / 25 | tenue |
| ≤ 1 fausse question sur 25 | 0 | tenue |
| `ecart_reel` ≥ 32 / 40 | 33 / 40 | tenue |
| `question_reel` ≥ 9 / 12 | 12 / 12 | tenue |
| phrase E rendue en question par le modèle | question du garde-fou dans le produit ; du modèle seulement dans `duree_reel` | **manquée** |

Barre manquée sur E. Décision du propriétaire (2026-10-24) : garder quand même le modèle de
5.17, meilleur que celui de 5.15 sur tout le reste, E restant rattrapée par le garde-fou. Il est
installé dans le dossier des modèles du poste sous le nom de déclaration que le panneau a
retenu (`cassini-composition-4b.modele.json`) ; celui de 5.15 y reste sous le nom
`cassini-composition-4b-5-15`. La suite de E fera l'objet d'un autre ticket (`deferred-work.md`).

## Suite de bout en bout (ticket 5.9, 2026-11-01)

Révision `bf24d56`, commit des tests du ticket 5.9 (base `88251ab`, aucun code produit changé), modèle livré (`cassini-composition-4b.modele.json`,
prompt court), RTX 3070 Ti, paquet `--features gpu`.

- `plan_reel` (`--features cuda`) : **25 / 25**, 0 fausse question.
- `cli/tests/bout_en_bout.rs`, sur le vrai binaire : la phrase de démo exacte de `CLAUDE.md`,
  sur l'inventaire de l'épreuve (`tests/epreuve-modele-local/inventory.json`, traduit dans la forme
  de `project.inventory`), devient par `plan.propose` le plan attendu — sélection `typezone = U`
  du PLU, tampon de 500 m des gares, deux intersections, export PDF titré « Parcelles
  constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi » — en 16,5 s, chargement
  compris. Aucune socket réseau tenue par `cassini serve` après la composition. Le plan est
  exécuté par la fausse extension (5 lignes au journal), puis rejoué dans un second `cassini serve`
  dont le dossier des modèles est vide (`agent.models` y rend une liste vide) :
  `{"steps": 5, "identical": true}`, journal inchangé octet pour octet. Le rejeu se passe de l'IA.

Durées sur processeur de la même révision : `docs/mesure-duree-par-phrase.md`, section du ticket 5.9.

## Ré-entraînement de 5.39 : point en UTM et échelle de l'export PDF

Le générateur de 5.37 (famille `isochrone_utm`) et de 5.38 (famille `export_pdf_scale`) a été
entraîné par la commande unique, sans autre changement : commit `06d4608` (arbre propre),
8 000 exemples, même base (`Qwen/Qwen3.5-4B` @ `851bf6e`), même méthode, même révision de
llama.cpp (`d812350`), sortie `~/cassini-entrainement/5-39`.

### Entraînement

| Mesure | Valeur |
|---|---|
| Exemples | 8 000 d'entraînement, 300 de validation, aucun écarté car trop long |
| Jetons par séquence | médiane 745, maximum 1 499 |
| Perte finale | 0,0062 (500 pas) |
| Validation (graine 2, glouton) | 296 / 300 : plans 234 / 238, questions 33 / 33, refus 29 / 29 |
| Durée totale | 316 min, dont 48 min de validation (RTX 3070 Ti, WSL) |
| Essai `llama-completion` | base + adaptateur et fusionné rendent le plan attendu |
| Modèle fusionné | `cassini-composition-4b-Q4_K_M.gguf`, SHA-256 `b6fc49d029e258e47a916a0b89358ef046fb6453652564078085c36c03d2be37` |

### Parité des découpeurs

`parite_decoupeurs.py` : **300 / 300**, 0 écart (205 140 jetons).

### Essais réels (GPU, modèle fusionné, prompt court)

`cargo test -p cassini-ai --release --features cuda --test plan_reel --test question_reel
--test ecart_reel --test echelle_utm_reel -- --ignored`, `CASSINI_TEST_MODEL` = la déclaration
`cassini-composition-4b.modele.json` de 5.39.

| Essai | Barre | 5.17 | 5.39 |
|---|---|---|---|
| `plan_reel` | ≥ 24 / 25 | 25 / 25 | **25 / 25**, 0 question, 0 du garde-fou |
| `question_reel` | ≥ 11 / 12 | 12 / 12 | **12 / 12**, 0 du garde-fou |
| `ecart_reel` | ≥ 32 / 40 | 33 / 40 | **34 / 40** (plans 20 / 26, questions 8 / 8, refus 6 / 6), 187 s |
| `echelle_utm_reel` (nouveau) | ≥ 7 / 8 | — | **7 / 8** |

Écarts du jeu tenu à l'écart : H08 et H23 (une question au lieu d'un plan), H17, H18, H19 (une
étape de moins), H22 (une étape de plus). H13 et H20 deviennent justes ; H23 recule. Une
phrase passée par la redemande (le second essai après un plan rejeté), sans être sauvée.

`echelle_utm_reel` : les trois isochrones en UTM (31N à pied et en voiture, 32N) et quatre
exports à l'échelle (« 1/25000 », « 25 000e », « 1:50k », et « … en PDF au 1:10 000 » au bout
d'une sélection) rendent le plan attendu. **E01, « Exporte la carte au 1:25 000. », est
refusé** (« L'export PDF du catalogue produit seulement une page A4 paysage. »). Cause : toutes
les tournures de `export_pdf_scale` contiennent le mot « PDF » ; une demande d'export à
l'échelle qui ne le dit pas ressemble aux refus de format (A3) appris par `refus_hors_registre`.

### Installation

Dossier des modèles du poste : le modèle de 5.39 sous la déclaration retenue par le panneau
(`cassini-composition-4b.modele.json`, lien dur vers `ajuste-5-39`) ; celui de 5.17 gardé sous
`cassini-composition-4b-5-17` (GGUF et déclaration). `cassini run model.list` voit les quatre
modèles du dossier : Qwen3.5-9B, les ajustés de 5.15 et de 5.17, et celui de 5.39.

### Démo dans QGIS

Le 2026-11-04, sonde enchaînée sur une copie du projet réel (`projet-sonde-5-39.qgz`), noyau
installé, déclaration `cassini-composition-4b.modele.json` :

| Phrase | Résultat |
|---|---|
| phrase de démo de `CLAUDE.md` | plan de 5 étapes (sélection `typezone = U`, tampon 500 m, deux intersections, PDF titré), exécuté, PDF écrit ; 20,7 s |
| « Exporte la carte au 1:25 000. » | **refus** du modèle, comme E01 |
| « Calcule la zone atteignable en 10 minutes à pied depuis le point 456419 / 5397604 (UTM 31N). » | `analysis.isochrone` à `epsg` 32631, exécutée ; 5,9 s |
| rejeu du journal | `{"steps": 6, "identical": true}` |

Seconde sonde, sur une copie neuve (`projet-sonde-5-39b.qgz`), pour voir un export à l'échelle
au bout de la chaîne : « Sors-moi la carte en PDF au 1/25000. » rend `layout.export_pdf` à
`scale: 25000` (5,0 s), le PDF (1,9 Mo) est écrit, le centre de la vue est journalisé (clé
`replay`) et le rejeu rend `{"steps": 1, "identical": true}`.

### Bilan

Les quatre barres sont tenues ; la démo manque la phrase sans « PDF ». Décision du propriétaire
(2026-11-04) : garder le modèle de 5.39 et clore le ticket ; les tournures d'export à l'échelle
sans le mot « PDF » vont au générateur dans un ticket à part, apprises au prochain
ré-entraînement groupé (`deferred-work.md`).

## Ré-entraînement de 5.41 : commandes de terrain et export à l'échelle sans « PDF »

Le générateur de 4.6 (familles `pente`, `ligne_de_vue`, `champ_de_vision` ; refus
`NO_SLOPE`, `NO_LINE_OF_SIGHT`, `NO_VIEWSHED` retirés) et de 5.40 (tournures
d'`export_pdf_scale` sans le mot « PDF ») a été entraîné par la commande unique : même base
(`Qwen/Qwen3.5-4B` @ `851bf6e`), même méthode, même révision de llama.cpp (`d812350`),
8 000 exemples. Avant tout entraînement, les attendus de terrain de `plan_reel` ont été remis
à jour (S20 et S22 attendent un plan, S21 la question du garde-fou, hors du plafond des fausses
questions ; S23 et S24 restent des refus) et un essai neuf, `terrain_reel`, a été ajouté
(2 pentes, 2 lignes de vue, 2 champs de vision, 2 exports à l'échelle sans « PDF »).

### Deux entraînements

| | n° 1 | n° 2 (retenu) |
|---|---|---|
| Commit (arbre propre) | `a30bf85` | `f89fea4` |
| Sortie | `~/cassini-entrainement/5-41` | `~/cassini-entrainement/5-41-b` |
| Durée totale (RTX 3070 Ti, WSL) | 325 min | 281 min, dont 47 min de validation |
| Perte finale (500 pas) | 0,0048 | 0,0067 |
| Jetons par séquence | médiane 742, maximum 1 578 | médiane 742, maximum 1 540 |
| Validation (graine 2, glouton) | 299 / 300 : plans 242 / 243, questions 37 / 37, refus 20 / 20 | 299 / 300 : plans 235 / 236, questions 40 / 40, refus 24 / 24 |
| Parité des découpeurs | 300 / 300 | 300 / 300, 0 écart (203 135 jetons) |
| Essai `llama-completion` | plan attendu | base + adaptateur et fusionné rendent le plan attendu |

Le modèle n° 1 manquait trois sortes de phrases que le générateur ne montrait pas : un calcul
de terrain suivi d'une statistique par zone (S23, S24), un point de terrain écrit dans un autre
système que le MNT (S25), et « dans un rayon de X autour du point » (V02, rendu en zone tampon).
Décision du propriétaire (2026-11-04) : compléter le générateur (`refus_hors_registre` :
`NO_RASTER_STATS` après un calcul de terrain, `NO_TERRAIN_CRS` pour un point hors du système du
MNT ; `champ_de_vision` : le rayon avant le point) et ré-entraîner.

Modèle n° 2 fusionné : `cassini-composition-4b-Q4_K_M.gguf`, SHA-256
`e335f6274af331919c76ab8638431278e821849374ad40f7fa3dd914f52991a0`.

### Essais réels (GPU, modèle fusionné, prompt court)

`cargo test -p cassini-ai --release --features cuda --test plan_reel --test question_reel
--test ecart_reel --test echelle_utm_reel --test terrain_reel -- --ignored`,
`CASSINI_TEST_MODEL` = la déclaration du modèle mesuré. La colonne 5.39 reprend son modèle sur
les attendus mis à jour.

| Essai | Barre | 5.39 | 5.41 n° 1 | 5.41 n° 2 |
|---|---|---|---|---|
| `plan_reel` | ≥ 24 / 25, puis 23 / 25 (décision du propriétaire) | 22 / 25 | 22 / 25 | **23 / 25**, 0 fausse question |
| `question_reel` | ≥ 11 / 12 | 12 / 12 | 12 / 12 | **12 / 12** |
| `ecart_reel` | ≥ 32 / 40 | 34 / 40 | 34 / 40 | **34 / 40** (plans 21 / 26, questions 8 / 8, refus 5 / 6), 263 s |
| `echelle_utm_reel` | ≥ 7 / 8 | 7 / 8 | 8 / 8 | **8 / 8** (E01 « Exporte la carte au 1:25 000. » juste) |
| `terrain_reel` (nouveau) | ≥ 7 / 8 | 1 / 8 | 7 / 8 | **8 / 8** |

`plan_reel` : S23 et V02 deviennent justes. Restent **S24** (« … champ de vision …, puis sa
moyenne par commune » : le champ de vision seul, la moyenne oubliée, au lieu du refus) et
**S25** (point en WGS84 sur un MNT en Lambert-93 : un champ de vision en EPSG:4326, que le noyau
refuse ensuite à l'exécution par `terrain.crs_mismatch`). Le jeu contient ces deux refus, mais
quelques dizaines d'exemples sur 8 000 ne suffisent pas.

Écarts du jeu tenu à l'écart : H08 et H23 (une question au lieu d'un plan), H17, H18, H19 (une
étape de moins), H40 (un plan au lieu d'un refus). L'essai compte une phrase passée par la redemande sans être sauvée : elle est
l'un de ces six écarts.

**Décision du propriétaire (2026-11-05)** : après deux entraînements, accepter le modèle n° 2,
meilleur que 5.39 sur chaque essai ; le renfort des refus de S24 et S25 va au prochain
ré-entraînement groupé (`deferred-work.md`).

### Installation

Dossier des modèles du poste : le modèle n° 2 sous la déclaration retenue par le panneau
(`cassini-composition-4b.modele.json`, lien dur vers `ajuste-5-41-b`) ; celui de 5.39 gardé
sous `cassini-composition-4b-5-39` (GGUF et déclaration). `cassini run model.list` voit les
cinq modèles du dossier.

### Démo dans QGIS

Le 2026-11-05, sonde enchaînée sur une copie du projet réel avec le MNT de Villeneuve-le-Roi en
Lambert-93 (`projet-sonde-5-41.qgz`), noyau et extension installés, modèle installé : **dix
phrases sur dix** rendent le plan juste, exécuté.

| Phrase | Résultat |
|---|---|
| phrase de démo de `CLAUDE.md` | plan de 5 étapes, PDF écrit ; 17,2 s |
| « Calcule la pente du terrain en degrés. » puis « … en pourcentage. » | `terrain.slope`, rasters ouverts (0–49,3° ; 0–116 %) |
| deux lignes de vue en Lambert-93 | plateau masqué (obstacle à 2 854 m) ; pied du coteau visible, 2 m à chaque bout |
| champ de vision à 25 m de haut sur 2 km | `terrain.viewshed`, raster ouvert |
| « … dans un rayon de 800 m autour du point … » | `terrain.viewshed`, `radius` 800 m |
| « Exporte la carte au 1:25 000. » | `layout.export_pdf` à `scale: 25000` |
| zone atteignable à pied en 10 minutes | `analysis.isochrone` |
| « Sors-moi la carte au 1/10 000. » | `layout.export_pdf` à `scale: 10000` |

Compositions de 2,5 à 9,4 s hors phrase de démo.

**Le rejeu diverge** (`replay.diverged`), à l'étape 5 dans la même session, à l'étape 12 dans
une session neuve : un export rejoué ne retire de la carte que les couches à `layer_id` du
journal (5.36), alors que les rasters du noyau, ouverts par le panneau (4.3), n'y figurent pas et
ne sont pas rouverts au rejeu. Dès qu'un raster de terrain précède un export, le nombre de
couches de l'export change. Défaut du rejeu, indépendant du modèle : bogue 4.9
(`_bmad-output/initiative-cassini-mvp/epic-raster-et-terrain/bug-le-rejeu-diverge-quand-un-raster-de-terrain-precede-un-export.md`).

## Ré-entraînement de 11.16 : les quatorze commandes de l'epic 11

Le générateur de 11.12 (cinq lots : géométrie, dérivés du MNT, raster de coût, critères et
superposition, export au format) a été entraîné par la commande unique : même base
(`Qwen/Qwen3.5-4B` @ `851bf6e`), même méthode, même révision de llama.cpp (`d812350`),
8 000 exemples. Avant l'entraînement, l'attendu de S25 dans `plan_reel` est passé du refus au
plan `data.transform_point` puis `terrain.viewshed` sur `step:1.point`, et un essai neuf,
`epic11_reel`, a été ajouté : deux phrases par commande de l'epic 11 (28), plus une phrase de
carroyage sans système (F03) qui attend une question.

### L'entraînement

| | 11.16 |
|---|---|
| Commit (arbre propre) | `ce7fe05` |
| Sortie | `~/cassini-entrainement/11-16` |
| Durée totale (RTX 3070 Ti, WSL) | 323 min, dont 49 min de validation |
| Perte finale (500 pas) | 0,00697 |
| Jetons par séquence | médiane 738, maximum 1 644 ; aucun exemple écarté pour longueur |
| Validation (graine 2, glouton) | 293 / 300 : plans 236 / 241, questions 34 / 36, refus 23 / 23 |
| Parité des découpeurs | 300 / 300, 0 écart (202 154 jetons) |
| Essai `llama-completion` | base + adaptateur et fusionné rendent le plan attendu |

Le jeu de validation n'est plus celui de 5.41 (il contient les nouvelles familles) : les deux
taux ne se comparent pas. Ratés de validation : deux points UTM d'un fuseau à l'autre non
reprojetés, un carroyage sans système planifié, deux noms de raster (un identifiant de couche ou
un nom de fichier à la place de la `source`), un refus à tort (« pas de distance entre deux
couches »), une mauvaise référence d'étape (`step:0`) dans une chaîne de six étapes.

Modèle fusionné, sortie de l'entraînement `~/cassini-entrainement/11-16/cassini-composition-4b-Q4_K_M.gguf`,
SHA-256 `9dbed14d801eda2705e4f743b2a0057ccb101db414ef2995b3c0e17296c3ea6e` (même empreinte sur la
copie de `ajuste-11-16` et sur le fichier installé).

### Essais réels (GPU, modèle fusionné, prompt court)

`cargo test -p cassini-ai --release --features cuda --no-fail-fast --test plan_reel --test
question_reel --test ecart_reel --test echelle_utm_reel --test terrain_reel --test epic11_reel
-- --ignored --test-threads=1`, `CASSINI_TEST_MODEL` = la déclaration du modèle mesuré.

| Essai | Barre (celles de 5.41) | 5.41 n° 2 | 11.16 |
|---|---|---|---|
| `plan_reel` | ≥ 23 / 25 | 23 / 25 | **23 / 25**, 1 fausse question |
| `question_reel` | ≥ 11 / 12 | 12 / 12 | **12 / 12** |
| `ecart_reel` | ≥ 32 / 40 | 34 / 40 | **34 / 40** (plans 20 / 26, questions 8 / 8, refus 6 / 6), 180 s |
| `echelle_utm_reel` | ≥ 7 / 8 | 8 / 8 | **8 / 8** |
| `terrain_reel` | ≥ 7 / 8 | 8 / 8 | **8 / 8** |
| `epic11_reel` (nouveau) | chaque commande ≥ 1 / 2 | 0 / 28 | **27 / 28**, 14 / 14 commandes |

`plan_reel` : S25 devient juste. Restent **S18** (« carte PDF A3 en portrait des routes et des
gares » : l'export de toutes les couches visibles, alors que l'export ne sait pas choisir ses
couches et que le refus reste attendu) et **S20** (« depuis le château d'eau en X / Y
(Lambert-93) » : une question sur des coordonnées pourtant écrites). `epic11_reel` : **A01**
(« dans quelle direction sont tournées les pentes ») rendu en pente puis reclassement ; **F03**
(carroyage UTM sans zone) refusé (« aucune commande n'exporte de carte ») au lieu de la
question du système. **L'essai `epic11_reel` reste donc rouge** sur ce modèle : il exige aussi la
question de F03, que le tableau des cas du plan demande ; le modèle est accepté malgré cet échec
(décision ci-dessous), et l'essai garde son exigence pour le prochain ré-entraînement.

Écarts de `ecart_reel` : H08 et H11 (une question au lieu d'un plan), H13 (`label_field`), H17
(`field`), H18 et H19 (une étape de moins). H08, H17, H18 et H19 l'étaient déjà en 5.41 ; H11 et
H13 sont nouveaux, H23 et H40 sont devenus justes.

**Décision du propriétaire (2026-11-07)** : accepter le modèle, qui tient chaque barre et
apprend les quatorze commandes ; F03, A01, S18, S20 et la reprojection entre fuseaux UTM vont au
prochain ré-entraînement groupé (`deferred-work.md`).

### Installation

Dossier des modèles du poste : le modèle de 11.16 sous la déclaration retenue par le panneau
(`cassini-composition-4b.modele.json`, qui désigne `cassini-composition-4b-Q4_K_M.gguf`, lien dur
vers le fichier de `ajuste-11-16`) ; celui de 5.41 gardé
sous `cassini-composition-4b-5-41` (GGUF et déclaration). `cassini run model.list` voit les six
modèles du dossier. Noyau et extension installés déjà identiques au dépôt.

### Démo dans QGIS

Le 2026-11-07, sonde enchaînée sur une copie du projet réel (`projet-sonde-11-16.qgz`) : MNT de
Villeneuve-le-Roi en Lambert-93, plus quatre rasters de test dérivés du MNT sur sa grille, hors
du dépôt (`donnees-test-cassini/derives-11-16/`, valeurs fictives : coût = 1 + pente en degrés / 5 ;
note d'accessibilité = 10 × (1 − altitude normalisée) ; note de calme = 10 × min(1, pente en
degrés / 15 + altitude normalisée / 2), arrondies au dixième ; masque = 1 sous 40 m, sinon 0). Modèle installé,
noyau installé : **douze phrases sur douze** rendent un plan juste, exécuté, puis **le rejeu
rend `identical: true`** (29 étapes).

| Phrase | Résultat |
|---|---|
| phrase de démo de `CLAUDE.md` | plan de 5 étapes, PDF écrit ; 25,8 s |
| référence MGRS d'un point WGS84 | `coord.convert` → `31UDP5698598162` |
| distance et azimut entre deux points Lambert-93 | deux `data.transform_point` vers 4326, puis `geo.distance_azimuth` : 3 606 m, 55,9° |
| champ de vision depuis un point WGS84 | `data.transform_point` vers 2154, puis `terrain.viewshed` sur `step:1.point` |
| champ de vision cumulé de deux points, 2 km | `terrain.viewshed_cumulative` |
| orientation des versants ; relief ombré | `terrain.aspect` ; `terrain.hillshade` |
| points hauts | `terrain.peaks`, 20 sommets, couche de points ouverte |
| pente au-delà de 10 % | `terrain.slope` en pourcentage, puis `raster.reclassify` (`min: 10`) |
| chemin le moins coûteux sur le raster de coût | `terrain.least_cost_path`, 6 426 m |
| accessibilité (2) et calme (1), zones inondables exclues | `raster.weighted_overlay`, masque compris |
| export A3, carroyage Lambert-93 tous les 1 km | `layout.export_pdf` (`page_size: a3`, `grid` projeté 2154, 1 km) |

Compositions de 3,6 à 13,3 s hors phrase de démo.

Complément (relecture), même jour, seconde sonde sur une autre copie (`projet-sonde-11-16b.qgz`) :
**quatre phrases sur quatre**, rejeu `identical: true` (5 étapes).

| Phrase | Résultat |
|---|---|
| coût cumulé depuis un point Lambert-93 sur le raster de coût | `terrain.cost_distance`, 549 712 cellules atteintes |
| point d'arrivée à 2 km à l'azimut 90° d'un point WGS84 | `geo.destination_point` |
| recoupement de deux visées (45° et 315°) | `geo.bearing_intersection`, 2 601 m de chaque station |
| « que vaut la référence MGRS 31UDP 56985 98162 en latitude et longitude » | `coord.convert` → le point de la phrase MGRS de la première sonde, au millionième de degré |

La distance de la première sonde (3 606 m, 55,9°) se retrouve à la main par l'approximation
plane locale sur les deux points reprojetés (≈ 3,60 km, ≈ 55,9°).
