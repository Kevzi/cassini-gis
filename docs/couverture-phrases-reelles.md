# Couverture du registre par des phrases réelles : mesure de départ de l'epic 10 (tickets 7.11 et 8)

Mesure du 2026-11-20, sur le registre du dépôt à cette date, y compris les fichiers de `raster.zonal_statistics` (entrée 12), en cours de livraison. Jeu : `ai/tests/couverture_reelle/phrases.json`, 44 phrases tirées de sources publiques (documentation et manuel de formation de QGIS, questions de forum publiques, supports de cours publics), réécrites et non recopiées ; chaque phrase garde type, référence, licence, thème et traitement. Les références désignent un document ou un thème, pas une adresse de page vérifiée. Les phrases sont rapportées à l'inventaire de l'épreuve (`ai/tests/par_commande/inventory.json`). Harnais : `ai/tests/couverture_reelle_reel.rs`, qui écrit lui-même ce rapport (couverture, colonne agent, ligne du taux de plans conformes). Modèle mesuré : `cassini-composition-4b-Q4_K_M.gguf` (prompt court, sans adaptateur) installé dans `%LOCALAPPDATA%\Cassini\bin\models`, non ré-entraîné (le ré-entraînement est groupé après le dernier ticket de l'epic). La part « couverte par le registre » ne dépend pas du modèle, seule la colonne « Agent local » en dépend.

Le ticket 10.8 a ajouté le jeu entier à la garde anti-fuite de l'entraînement (`training/tests/recouvrement.rs`, source `couverture_reelle`, 44 phrases). La garde a trouvé trois phrases (CV03, CV21, CV22) qui tombaient à un nombre ou une commune près sur des phrases du jeu engendré, d'une autre source d'essai ou d'une famille ; 10.8 les a reformulées (même sens, même attendu). La colonne « Agent local » de CV03, CV21 et CV22 a été mesurée sur la nouvelle formulation (le modèle a tourné après la reformulation).

## Taux de départ de l'epic 10

- **Référence de l'entrée 17** : `plan_reel` 44/48 plans conformes, mesuré le 2026-11-16 (séance `continu-5-61b`), avant les commits de l'epic 10, barre 38/48. C'est la valeur à égaler ou dépasser.
- **Relecture du 2026-11-20** sur le catalogue courant et le modèle installé : 41/48 (85,4 %). Écarts : S15 et S23 (leur attendu est devenu `raster.zonal_statistics` avec l'entrée 12, le modèle n'a pas appris la commande), S24 (plan invalide : la sortie du champ de vision n'a pas de champ `viewshed`), PL04, PL08, PL17 et PL22 (confusion de commandes voisines, question non voulue, géocodage). Seuls S15 et S23 sont attribuables avec certitude au catalogue ; la baisse de 44 à 41 n'est pas analysée plus avant ici. Le ré-entraînement de fin d'epic est ce qui doit la rattraper.

## Couverture

Part couverte par le registre actuel (plan valide ou question justifiée) : 34/44, soit 77,3 % (33/44, 75,0 % à la mesure d'origine du 7.11, avant que `raster.zonal_statistics` couvre CV26).

Part couverte et bien rendue par l'agent local : 21/44, soit 47,7 %. Réponses conformes à l'attendu, refus compris : 28/44, soit 63,6 %. Taux de plans conformes de l'épreuve `plan_reel` : 41/48, soit 85,4 %.

**Verdict sur l'objectif de 70 %** (décision du 2026-10-17 de l'epic) : la couverture par le registre est déjà de 77,3 %, soit au-dessus de l'objectif avant même la fin de l'epic. Ce qui reste loin est la part rendue correctement par l'agent (47,7 %), qui dépend du modèle et du ré-entraînement, pas de nouvelles primitives. L'objectif est tenu sur la couverture ; il reste à le tenir sur l'agent à l'entrée 17.

## Constats

- Quatre phrases qui donnent un point en notation « 5.90 E 45.90 N » (CV14, CV18, CV28, CV29) déclenchent la question du garde-fou des coordonnées au lieu d'un plan : le registre les couvre, c'est la lecture de la notation E/N qui manque. Constat consigné pour l'entrée 17 (ou 10.17, ou un ticket de garde-fou) ; les phrases sont laissées telles quelles.
- Confusions de commandes voisines : décompte par zone, longueurs de lignes et reclassement (CV03, CV04, CV17) sortent un autre plan valide ; fusion par champ (CV09) est refusée alors que `analysis.dissolve` existe.
- Terrain sans point ni coordonnées, pente, ombrage, points hauts et style gradué (CV05, CV06, CV11, CV19) : l'agent répond par une question alors que la phrase suffit.
- CV35 (géocodage d'une liste d'adresses) : couverte depuis `geocode.ban` (entrée 16), attendu devenu `geocode.ban` puis `layer.add` ; PL22 de l'épreuve, qui demandait le géocodage d'une adresse isolée, est remplacée par un refus encore vrai (plus court chemin) ; depuis `network.shortest_path` (entrée 13), PL22 devient à son tour un refus encore vrai (bassin versant, `watershed`), EU34 un plan (`network.shortest_path`) et trois phrases par commande (CHE1 à CHE3) sont ajoutées, sans famille d'exemples ni ré-entraînement (ticket 17). Mesure de couverture non rejouée ici, modèle non ré-entraîné.
- CV26 (altitude moyenne par commune) : couverte depuis `raster.zonal_statistics`, mais l'agent refuse encore ; le modèle ne sera pas ré-entraîné avant la fin de l'epic.
- Trois phrases non couvertes reçoivent un plan inventé au lieu d'un refus ou d'une question (CV37 vectorisation, CV39 étiquettes, CV42 calculatrice raster) : à surveiller, un plan inventé pour une commande absente est le pire cas.
- Aucune phrase qui donne ses coordonnées n'est comptée en « question justifiée » : les quatre phrases E/N attendent un plan.

## Ordre recommandé des primitives restantes

Proposition d'après la couverture, à valider par le propriétaire (étape humaine de l'entrée 8) ; `tickets.toml` n'est pas modifié. Commandes manquantes aux 10 phrases non couvertes : geocode (CV35), polygonize (CV37), label (CV39), clip (CV40), raster_calculator (CV42), heatmap (CV38), profile (CV41), watershed (CV36), radio_coverage (CV44), download (CV43, jamais : hors ligne).

1. Terminer l'entrée 12 (zonal, CV26) : déjà le plus demandé des restants du voie unique.
2. Géocodage (entrée 16) : seule primitive de la voie unique qu'une phrase réelle demande (CV35), et PL22 de l'épreuve le demande aussi ; le sortir de sa place en bout de voie (derrière 15) serait un gain de couverture net.
3. Vectorisation d'un raster (`raster.polygonize`, ajoutée par le changement de cap du 2026-11-05) : CV37.
4. Entrées à créer si la couverture doit encore monter : découpage par masque (CV40), calculatrice raster (CV42), étiquettes (CV39). Carte de chaleur et profil (CV38, CV41) ensuite ; hydrologie et couverture radio restent hors périmètre (V2).
5. En queue : atlas (15). Le plus court chemin (13) et la préparation de réseau (14) sont faits ; la préparation a ses trois phrases de `par_commande` (PRE1 en plan avec l'isochrone, « prépare le réseau de chemins avant l'isochrone », PRE2 en plan, PRE3 en question), hors des 44 phrases réelles : aucune de ces 44 ne la demande ni ne demande l'atlas ; le jeu est petit (44), donc ce n'est pas la preuve qu'ils sont inutiles, mais aucune mesure ne les soutient.

Le reclassement d'un raster (entrée 25) est à `dropped` selon la décision du 2026-11-05 (calculé par le noyau, epic 11) ; CV17 est comptée couverte par ce motif.

## Rejouer la mesure

Hors ligne : `cargo test -p cassini-ai --test couverture_reelle_reel` (la couverture est recalculée). Avec modèle : lancer d'abord `plan_reel` (`cargo test -p cassini-ai --features gpu --test plan_reel -- --ignored --nocapture`, ligne `SCORE n/48 plans conformes`), puis le harnais avec `CASSINI_TEST_MODEL`, `CASSINI_PLANS_CONFORMES=n/48` (le chiffre relevé) et `CASSINI_COUVERTURE_RAPPORT=<fichier>` qui reçoit le rapport.

## Phrase par phrase

| Id | Source | Phrase | Attendu | Commande manquante | Agent local |
|---|---|---|---|---|---|
| CV01 | QGIS Training Manual (docs.qgis.org), CC BY-SA 4.0 (zone tampon autour de points) | Fais une zone de 500 mètres autour de chaque gare. | plan : analysis.buffer | — | OK Plan |
| CV02 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (sélection par expression sur un champ) | Sélectionne les parcelles dont la surface dépasse 1000 m². | plan : analysis.select_by_attribute | — | OK Plan |
| CV03 | QGIS Training Manual (docs.qgis.org), CC BY-SA 4.0 (compter les points dans les polygones) | Dénombre les gares situées dans chaque commune. | plan : analysis.count_points_in_polygon | — | ÉCART OtherPlan |
| CV04 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (somme des longueurs de lignes par polygone) | Quelle est la longueur totale de routes dans chaque commune ? | plan : analysis.sum_line_lengths | — | ÉCART OtherPlan |
| CV05 | QGIS Training Manual (docs.qgis.org), CC BY-SA 4.0 (pente d'un MNT) | Calcule la pente à partir du modèle numérique de terrain. | plan : terrain.slope | — | ÉCART Question |
| CV06 | Documentation QGIS en français (docs.qgis.org/fr), CC BY-SA 4.0 (ombrage d'un MNT) | Fais-moi un ombrage du relief. | plan : terrain.hillshade | — | ÉCART Question |
| CV07 | supports de cours et de TP de SIG publics des universités françaises ; thème repris, formulation réécrite (mise en page et export PDF) | Exporte la carte en PDF avec le titre « Zonage communal ». | plan : layout.export_pdf | — | OK Plan |
| CV08 | Documentation QGIS en français (docs.qgis.org/fr), CC BY-SA 4.0 (ajout d'une couche vecteur) | Ouvre le fichier ecoles.geojson dans le projet. | plan : layer.add | — | OK Plan |
| CV09 | QGIS User Guide, algorithmes de traitement (docs.qgis.org), CC BY-SA 4.0 (regrouper des entités par attribut) | Fusionne les zones du PLU qui ont le même type de zone. | plan : analysis.dissolve | — | ÉCART Refusal |
| CV10 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (changement de système de coordonnées) | Reprojette les parcelles en WGS 84. | plan : data.reproject | — | OK Plan |
| CV11 | QGIS Training Manual (docs.qgis.org), CC BY-SA 4.0 (symbologie graduée) | Colorie les parcelles par tranches de surface. | plan : map.graduated_style | — | ÉCART Question |
| CV12 | QGIS User Guide, algorithmes de traitement (docs.qgis.org), CC BY-SA 4.0 (plus proche voisin) | Pour chaque commune, trouve la gare la plus proche. | plan : analysis.nearest | — | OK Plan |
| CV13 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (jointure d'un tableau sur un champ) | Rattache les prix de mutation aux parcelles grâce à l'identifiant de parcelle. | plan : analysis.join_table | — | OK Plan |
| CV14 | supports de cours et de TP de SIG publics des universités françaises ; thème repris, formulation réécrite (distance et azimut entre deux points) | Quelle distance et quel azimut entre le point 5.90 E 45.90 N et le point 6.10 E 45.80 N ? | plan : geo.distance_azimuth | — | ÉCART Question |
| CV15 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (conversion MGRS vers WGS 84) | Convertis le point MGRS 32TLR1234567890 en latitude et longitude. | plan : coord.convert | — | OK Plan |
| CV16 | QGIS Training Manual (docs.qgis.org), CC BY-SA 4.0 (sélection par distance à un objet) | Quels bâtiments sont à moins de 100 m d'une route nationale ? | plan : analysis.buffer + analysis.intersect | — | OK Plan |
| CV17 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (reclassification d'un raster) | Reclasse le modèle de terrain en trois classes : moins de 500 m, de 500 à 1000 m, plus de 1000 m. | plan : raster.reclassify | — | ÉCART OtherPlan |
| CV18 | supports de cours et de TP de SIG publics des universités françaises ; thème repris, formulation réécrite (chemin de moindre coût) | Trouve le chemin de moindre coût du point 6.10 E 45.90 N au point 6.20 E 45.85 N sur la couche de coût de déplacement. | plan : terrain.least_cost_path | — | ÉCART Question |
| CV19 | QGIS User Guide, algorithmes de traitement (docs.qgis.org), CC BY-SA 4.0 (sommets d'un MNT) | Donne-moi les points hauts du relief. | plan : terrain.peaks | — | ÉCART Question |
| CV20 | Documentation QGIS en français (docs.qgis.org/fr), CC BY-SA 4.0 (liste des couches du projet) | Qu'est-ce qui est ouvert dans mon projet ? | plan : project.inventory | — | OK Plan |
| CV21 | Documentation QGIS en français (docs.qgis.org/fr), CC BY-SA 4.0 (suppression d'une couche) | Enlève du projet la couche des routes. | plan : layer.remove | — | OK Plan |
| CV22 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (exposition d'un MNT) | Quelle est l'orientation des versants ? | plan : terrain.aspect | — | OK Plan |
| CV23 | QGIS Training Manual (docs.qgis.org), CC BY-SA 4.0 (analyse multicritère) | Superpose la note d'accessibilité et la note de calme, à 60 % et 40 %. | plan : raster.weighted_overlay | — | OK Plan |
| CV24 | Documentation QGIS en français (docs.qgis.org/fr), CC BY-SA 4.0 (conversion de format vecteur) | Enregistre les communes en GeoPackage. | plan : data.convert | — | OK Plan |
| CV25 | Documentation QGIS en français (docs.qgis.org/fr), CC BY-SA 4.0 (description d'une couche) | Décris la couche des bâtiments : champs et nombre d'entités. | plan : layer.describe | — | OK Plan |
| CV26 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (statistiques zonales d'un raster) | Donne l'altitude moyenne de chaque commune d'après le modèle de terrain. | plan : raster.zonal_statistics | — | ÉCART Refusal |
| CV27 | QGIS Training Manual (docs.qgis.org), CC BY-SA 4.0 (statistiques par catégorie) | Quelle est la surface totale des parcelles pour chaque section ? | plan : analysis.statistics_by_category | — | OK Plan |
| CV28 | QGIS User Guide, algorithmes de traitement (docs.qgis.org), CC BY-SA 4.0 (zones de desserte sur un réseau) | Quelles zones atteint-on en 10 minutes de voiture depuis le point 6.12 E 45.90 N en suivant les routes ? | plan : analysis.isochrone | — | ÉCART Question |
| CV29 | supports de cours et de TP de SIG publics des universités françaises ; thème repris, formulation réécrite (analyse de visibilité) | Que voit-on à 5 km à la ronde depuis le point 6.13 E 45.90 N, un observateur à 2 m du sol ? | plan : terrain.viewshed | — | ÉCART Question |
| CV30 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (zone tampon sans couche ni distance) | Fais une zone tampon. | question justifiée | — | OK Question |
| CV31 | QGIS Training Manual (docs.qgis.org), CC BY-SA 4.0 (zone tampon sur une couche absente du projet) | Fais une zone tampon de 200 m autour des hôpitaux. | question justifiée | — | OK Question |
| CV32 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (filtre sur un champ qui n'existe pas) | Garde les bâtiments les plus anciens. | question justifiée | — | OK Question |
| CV33 | supports de cours et de TP de SIG publics des universités françaises ; thème repris, formulation réécrite (proximité à un objet absent, distance non donnée) | Garde les parcelles proches de la mairie. | question justifiée | — | OK Question |
| CV34 | Documentation QGIS en français (docs.qgis.org/fr), CC BY-SA 4.0 (visibilité sans point d'observation) | Montre la visibilité depuis le sommet. | question justifiée | — | OK Question |
| CV35 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (géocodage d'adresses) | Géocode cette liste d'adresses et place-les sur la carte. | non couverte | geocode | OK Question |
| CV36 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (hydrologie, bassin versant) | Calcule le bassin versant de ce ruisseau. | non couverte | watershed, hydrology | OK Question |
| CV37 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (vectorisation d'une image) | Transforme les bâtiments de cette image satellite en polygones. | non couverte | polygonize, vectorize | ÉCART Plan |
| CV38 | supports de cours et de TP de SIG publics des universités françaises ; thème repris, formulation réécrite (carte de chaleur) | Fais une carte de chaleur des accidents de la route. | non couverte | heatmap | OK Question |
| CV39 | QGIS Training Manual (docs.qgis.org), CC BY-SA 4.0 (étiquettes de la carte) | Mets en étiquette le nom de chaque commune sur la carte. | non couverte | label | ÉCART Plan |
| CV40 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (découpage d'un raster par un masque) | Découpe le modèle de terrain selon l'emprise de la commune. | non couverte | clip | OK Question |
| CV41 | QGIS User Guide, algorithmes de traitement (docs.qgis.org), CC BY-SA 4.0 (profil en long) | Fais un profil d'altitude le long de la route nationale. | non couverte | profile | OK Refusal |
| CV42 | gis.stackexchange.com, question récurrente, CC BY-SA 4.0 ; thème repris, formulation réécrite (calculatrice raster, différence de deux MNT) | Soustrais le relevé de terrain de 2019 au modèle de terrain actuel. | non couverte | raster_calculator | ÉCART Plan |
| CV43 | supports de cours et de TP de SIG publics des universités françaises ; thème repris, formulation réécrite (téléchargement de données (hors ligne par principe)) | Télécharge le cadastre de la commune. | non couverte | download | OK Refusal |
| CV44 | Documentation QGIS en français (docs.qgis.org/fr), CC BY-SA 4.0 (couverture radio) | Calcule la couverture radio de cette antenne. | non couverte | radio_coverage | OK Question |
