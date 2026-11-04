# Taxonomie des tâches SIG des professionnels français et correspondance QGIS / GDAL

## 1. Contexte et cadre d'analyse

La présente étude établit la taxonomie opérationnelle des traitements géospatiaux réalisés au quotidien par les bureaux d'études (urbanisme, aménagement, environnement), les collectivités territoriales (communes, intercommunalités, départements) et les étudiants ou géomaticiens en France. L'objectif principal est d'éclairer la conception d'un logiciel SIG de bureau piloté en langage naturel (français), en identifiant pour chaque besoin métier :

1. Les opérations géométriques, attributaires et raster sous-jacentes ;
2. La chaîne d'algorithmes exacte au sein de la boîte à outils de traitement QGIS (versions 3.40 LTR à 3.44) et de GDAL/OGR ;
3. Les formulations réelles et le vocabulaire métier utilisés par les praticiens ;
4. Le statut des licences logicielles et les dépendances vers des moteurs tiers (GRASS GIS, SAGA GIS, extensions).

Conformément aux exigences méthodologiques du cahier des charges initial, chaque analyse s'appuie sur des références institutionnelles et documentaires primaires, notamment la documentation officielle de QGIS, le Manuel Prestataire du Géoportail de l'Urbanisme (GPU), le Fascicule 1 ZAN du Ministère de la Transition Écologique et du Cerema, le Plan d'activités IGN 2026, ainsi que les retours d'expérience du forum communautaire GeoRezo.

---

## 2. Tableau synthétique des tâches SIG métier

Le tableau ci-dessous recense l'ensemble des tâches SIG identifiées à travers les 5 axes d'analyse (Urbanisme & Foncier, Environnement, Collectivités & Services Urbains). Pour chaque tâche, la fréquence est qualifiée sur la base des cahiers des clauses techniques particulières (CCTP) de marchés publics, des programmes de formation CNFPT/QGIS et des sujets récurrents sur GeoRezo.

### 2.1. Axe 1 — Urbanisme, foncier et aménagement

| Domaine | Tâche SIG | Fréquence & Preuve | Données d'entrée typiques | Opérations géospatiales | Algorithme(s) QGIS / GDAL exact(s) et fournisseur | Chaîne d'opérations (si multi-étapes) | Source principale |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Urbanisme** | Identification des dents creuses & gisements fonciers (ZAN) | **Élevée** (Priorité nationale ZAN, guides Cerema, CCTP PLUi) | Cadastre Etalab (PCI), BD TOPO bâti (IGN), Zonage PLU (CNIG) | Buffer sur le bâti, Soustraction géométrique, Filtre de surface | `native:buffer`<br>`native:difference`<br>`native:extractbyattribute` | 1. Tampon 20m autour des bâtiments (`native:buffer`)<br>2. Différence avec le cadastre (`native:difference`)<br>3. Filtre zone U et surface > 300m² (`native:extractbyattribute`) | Cerema ZAN Fascicule 1 / Code de l'urbanisme Art. L.101-2-1 |
| **Urbanisme** | Analyse de constructibilité par accessibilité (ex. gare/bus) | **Élevée** (Offres d'emploi géomaticien, études de densité) | Parcelles cadastrales, Réseau de transport (BD TOPO / GTFS) | Zone tampon / Isochrone, Intersection, Filtre attributaire | `native:buffer`<br>`native:extractbylocation`<br>`native:extractbyattribute` | 1. Tampon de 500m autour des gares (`native:buffer`)<br>2. Extrait par localisation (`native:extractbylocation`)<br>3. Filtre attributaire zone U/AU (`native:extractbyattribute`) | CCTP Études d'Aménagement / Documentation QGIS Vector Overlay |
| **Urbanisme** | Contrôle topologique et préparation à la publication GPU | **Élevée** (Exigence légale GPU, Manuel Prestataire GPU v4.3) | Couches PLU/PLUi au standard CNIG (ZONE_URBA, PRESCRIPTION) | Correction des géométries, Vérification des auto-intersections | `native:fixgeometries`<br>`native:checkvalidity` | 1. Diagnostic de validité (`native:checkvalidity`)<br>2. Correction automatique (`native:fixgeometries`)<br>3. Polygonisation le cas échéant (`native:polygonize`) | Manuel Prestataire GPU v4.3 / Standard CNIG PLU |
| **Foncier** | Observatoire des prix et transactions immobilières (DVF) | **Moyenne** (Études de marché, Safer, Agences d'urbanisme) | Demandes de Valeurs Foncières (DVF), Parcelles PCI | Jointure attributaire sur IDPAR, Cartographie thématique / Agrégation | `native:joinattributestable`<br>`native:aggregate` | 1. Jointure de la table DVF sur les parcelles (`native:joinattributestable`)<br>2. Agrégation par commune ou section (`native:aggregate`) | Données Foncieres Cerema / DGFIP DVF |
| **Urbanisme** | Instruction du Droit des Sols (ADS) — Extrait parcellaire | **Élevée** (Pratique quotidienne des services instructeurs) | Cadastre PCI, SUP, Servitudes, Périmètres de protection | Sélection par numéro de parcelle, Intersection SUP/PLU, Export PDF | `native:extractbyattribute`<br>`native:intersection` | 1. Sélection de la parcelle demandée (`native:extractbyattribute`)<br>2. Intersection avec SUP et zones PLU (`native:intersection`) | Code de l'urbanisme / Forum GeoRezo QGIS |

### 2.2. Axe 2 — Environnement, risques et milieu naturel

| Domaine | Tâche SIG | Fréquence & Preuve | Données d'entrée typiques | Opérations géospatiales | Algorithme(s) QGIS / GDAL exact(s) et fournisseur | Chaîne d'opérations (si multi-étapes) | Source principale |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Environnement** | Évaluation environnementale & Croisement d'impacts | **Élevée** (Études d'impact réglementaires, inventaires DREAL) | Emprise projet, ZNIEFF I/II, Natura 2000, Parcs nationaux | Intersection spatiale, Calcul de surface d'empiétement | `native:intersection`<br>`native:fieldcalculator` | 1. Intersection géométrique (`native:intersection`)<br>2. Recalcul de la surface en hectares (`native:fieldcalculator` `$area / 10000`) | Code de l'environnement / Guide Cerema Évaluation Environnementale |
| **Environnement** | Pré-localisation des zones humides & Hydrologie raster | **Moyenne** (Bureau d'études environnement, SAGE / Agences d'eau) | Modèle Numérique de Terrain (RGE ALTI IGN) | Calcul de pente, Indice d'humidité topographique (TWI), Ombres | `gdal:slope`<br>`saga:wetnessindex` (SAGA)<br>`gdal:contour` | 1. Calcul de pente (`gdal:slope`)<br>2. Indice d'humidité TWI (`saga:wetnessindex`)<br>3. Génération des courbes de niveau 1m (`gdal:contour`) | IGN RGE ALTI / Documentation SAGA GIS & QGIS |
| **Risques** | Analyse d'exposition aux risques naturels (PPRI) | **Élevée** (Plan de Prévention des Risques, DDTM) | Zonage PPRI (Aléa/Enjeu), Bâtiments BD TOPO, Cadastre | Intersection spatiale, Statistiques de bâti exposé | `native:extractbylocation`<br>`native:countpointsinpolygon` | 1. Extrait des bâtiments en zone d'aléa fort (`native:extractbylocation`)<br>2. Dénombrement et agrégation par section (`native:countpointsinpolygon`) | DGPR / Ministère Transition Écologique |
| **Occupation du Sol** | Statistiques d'artificialisation par zone (OCS GE / Corine) | **Élevée** (Observatoire ZAN, Suivi SRADDET) | OCS GE (IGN), Limites communales / EPCI (Admin Express) | Découpage raster/vecteur, Statistiques zonales | `native:zonalstatisticsfb`<br>`native:clip` | 1. Découpage de l'OCS GE sur l'EPCI (`native:clip`)<br>2. Calcul des statistiques zonales (`native:zonalstatisticsfb`) | IGN OCS GE Socle / Cerema ZAN |

### 2.3. Axe 3 — Collectivités territoriales et services urbains

| Domaine | Tâche SIG | Fréquence & Preuve | Données d'entrée typiques | Opérations géospatiales | Algorithme(s) QGIS / GDAL exact(s) et fournisseur | Chaîne d'opérations (si multi-étapes) | Source principale |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **Services Urbains** | Audit de couverture Défense Extérieure Contre l'Incendie (DECI) | **Élevée** (Référentiel DECI, SDIS, Services techniques municipaux) | Point d'Eau Incendie (PEI / Hydrants), Bâtiments BD TOPO | Zone tampon 200m/400m, Différence géométrique (hors couverture) | `native:buffer`<br>`native:difference` | 1. Tampon de 200m autour des PEI conformes (`native:buffer`)<br>2. Différence avec le bâti pour isoler les bâtiments hors couverture (`native:difference`) | Règlement Départemental DECI / SDIS 44 |
| **Adresses** | Géocodage de fichiers d'adresses bruts | **Élevée** (Communes, BAN, Déploiement Fibre / Enquêtes) | Fichier CSV/XLS d'adresses, Base Adresse Nationale (BAN) | Appariement textuel / spatial, Géocodage par lots | `native:batchnominatimgeocoder` ou Plugin BAN Geocoder | 1. Nettoyage de la chaîne d'adresse<br>2. Appariement géographique sur l'API BAN via script ou plugin | Etalab Base Adresse Nationale / Forum GeoRezo |
| **Statistiques** | Analyse socio-démographique à l'IRIS / Carreaux INSEE | **Moyenne** (Politique de la ville, Intercommunalités, INSEE) | Carreaux INSEE 200m (Filosofi), IRIS, Équipements publics | Jointure spatiale, Agrégation de données démographiques | `native:joinattributesbylocation`<br>`native:aggregate` | 1. Jointure des carreaux sur les équipements (`native:joinattributesbylocation`)<br>2. Somme et moyenne par quartier/IRIS (`native:aggregate`) | INSEE Filosofi / CNFP Training QGIS |
| **Réseaux** | Maintenance et découpage du réseau de voirie / eau | **Moyenne** (Services voirie/assainissement, Bureaux VRD) | Tronçons de voies BD TOPO, Points de coupure / Nœuds | Accrochage topologique, Découpage par lignes | `native:snapgeometries`<br>`native:splitwithlines` | 1. Accrochage topologique des lignes (`native:snapgeometries`)<br>2. Découpage aux intersections (`native:splitwithlines`) | Standard CNIG Réseaux / Documentation QGIS Geometry |

---

## 3. Formulations en langage naturel et expressions types

Pour nourrir le module de compréhension du langage naturel (NLP/LLM) qui traduira la demande de l'utilisateur en pipeline QGIS, les requêtes types des praticiens français sont répertoriées ci-dessous avec leurs variantes et paramètres optionnels.

### 3.1. Urbanisme et Foncier (Dents creuses, ZAN et Constructibilité)

* **Formulations types** :
  1. *« Trouve les dents creuses de plus de 300 m² dans la zone U du PLU. »*
  2. *« Extrais les parcelles non bâties situées à moins de 500 mètres de la gare d'Orléans et sors la carte en PDF. »*
  3. *« Identifie les gisements fonciers mutables de plus de 500 m² hors zones humides et hors EBC. »*
  4. *« Filtre les parcelles cadastrales de la commune de Pessac qui sont constructibles et libres de toute servitude. »*
  5. *« Liste les parcelles agricoles consommées par la ZAC du Vallon depuis 2021. »*
* **Variantes d'exécution** :
  * *Filtre strict de distance* : « à moins de X mètres de... » ➔ exige `native:buffer` suivi de `native:extractbylocation`.
  * *Filtre réglementaire* : « en zone U et AU » ➔ exige un filtre attributaire `CODE_ZONE IN ('U', 'AU')`.
  * *Action de sortie* : « et exporte la liste en Excel / sors le plan en PDF » ➔ exige l'appel au composeur d'impression QGIS ou à un export de couche.

### 3.2. Environnement et Risques

* **Formulations types** :
  1. *« Croise l'emprise du tracé avec la couche Natura 2000 et donne la surface d'impact en hectares. »*
  2. *« Quels sont les bâtiments situés dans la zone rouge du PPRI de la vallée du Rhône ? »*
  3. *« Calcule les pentes supérieures à 15% sur le MNT et génère les courbes de niveau tous les 5 mètres. »*
  4. *« Donne le pourcentage de surfaces artificialisées par commune sur l'ensemble de l'EPCI. »*
  5. *« Extrais les zones humides inventoriées qui intersectent le périmètre de l'étude d'impact. »*

### 3.3. Protection Incendie (DECI) et Services Urbains

* **Formulations types** :
  1. *« Liste tous les bâtiments résidentiels situés à plus de 200 mètres d'un poteau incendie fonctionnel. »*
  2. *« Trace les zones de couverture de 200m et 400m autour des hydrants de la commune. »*
  3. *« Géocode le fichier Excel des abonnés à l'eau sur la Base Adresse Nationale. »*
  4. *« Calcule la population totale vivant dans les carreaux INSEE à moins de 10 minutes d'un équipement sportif. »*

---

## 4. Glossaire métier et traduction opérationnelle

Le tableau suivant établit la correspondance directe entre le vocabulaire métier des praticiens français et l'opération SIG logicielle à déclencher dans QGIS.

| Terme métier | Définition & Sens opérationnel en urbanisme / aménagement | Opération SIG & Filtre logique associé |
| :--- | :--- | :--- |
| **Constructible** | Parcelle située dans une zone urbaine (U) ou à urbaniser (AU) du PLU/PLUi, non grevée d'inconstructibilité stricte. | `CODE_ZONE IN ('U', 'AU')` sur la couche `ZONE_URBA` du CNIG, croisé avec l'exclusion des zones `N`, `A`, `EBC` et servitudes de type `PPRi`. |
| **Dent creuse** | Espace non bâti de superficie limitée, enserré entre des constructions existantes au sein du tissu urbain consolidé. | `Difference(Parcelles_Zone_U, Buffer(Bâti, 20m))` filtré sur `Surface >= Surface_Min`. |
| **EBC (Espace Boisé Classé)** | Zone boisée protégée au titre de l'article L.113-1 du Code de l'urbanisme, entraînant un rejet de tout changement d'affectation. | Filtre attributaire `TYPEINF = '01'` (Prescription CNIG) ➔ Opération de soustraction (`native:difference`). |
| **OAP (Orientation d'Aménagement)** | Secteur à enjeu d'aménagement spécifique (mixité, densité, TVB) encadré par le PLU. | Sélection surfacique `TYPEINF = '06'` ➔ Croisement par intersection avec le cadastre. |
| **Zone U / AU / A / N** | Typologie réglementaire des zones PLU : U (Urbaine), AU (À Urbaniser), A (Agricole), N (Naturelle et forestière). | Filtres attributaires directs sur le champ `TYPEZONE` du standard CNIG (`'U'`, `'AU'`, `'A'`, `'N'`). |
| **DECI (200 m)** | Périmètre de sécurité incendie légal imposant une ressource en eau à moins de 200m (ou 400m) des constructions. | Zone tampon `native:buffer` de 200m autour des hydrants ➔ Soustraction `native:difference` du bâti. |
| **ENAF** | Espaces Naturels, Agricoles et Forestiers au sens du suivi de la loi Climat et Résilience (ZAN). | Couverture du sol OCS GE (`CS2` Végétation ou `CS1.2` Sols nus naturels) hors zones bâties et stabilisées. |

---

## 5. Algorithmes tiers et extensions spécifiques

Certaines analyses SIG complexes demandées par les professionnels ne sont pas couvertes par les algorithmes C++ natifs de QGIS ni par GDAL. Elles nécessitent de faire appel aux fournisseurs tiers intégrés (GRASS GIS, SAGA GIS) ou à des extensions (plugins) Python spécifiques.

### 5.1. Traitements dépendants de GRASS GIS ou SAGA GIS

* **Nettoyage topologique complexe de réseaux et de polylignes** :
  * *Algorithme* : `grass:v.clean` (GRASS GIS)
  * *Usage* : Élimination des dangles (brins morts), cassure des lignes aux intersections, simplification sans rompre la topologie.
  * *Licence* : GNU GPL v2+.
* **Analyse hydrologique et calcul du Topographic Wetness Index (TWI)** :
  * *Algorithme* : `saga:wetnessindex` (SAGA GIS) ou `grass:r.watershed` (GRASS GIS)
  * *Usage* : Pré-localisation des zones humides potentielles à partir d'un MNT (RGE ALTI IGN) par calcul d'accumulation de flux et de pente localisée.
  * *Licence* : GNU GPL v2 / LGPL.
* **Ombrage avancé et analyse d'insolation réelle** :
  * *Algorithme* : `saga:analyticalhillshading` (SAGA GIS)
  * *Usage* : Calcul de l'insolation et du rayonnement solaire pour les études photovoltaïques ou microclimatiques urbaines.
  * *Licence* : GNU GPL v2.

### 5.2. Extensions (Plugins) QGIS incontournables dans le contexte français

* **Extension Cadastre (Plugin Cadastre DGFIP)** :
  * *Fonction* : Importation et structuration des fichiers MAJIC (données littérales des propriétaires) et PCI Vectoriel (géométrie) dans une base PostgreSQL/PostGIS ou SpatiaLite.
  * *Licence* : GNU GPL v2.
* **Extension BAN Geocoder (ou Geocoding BAN)** :
  * *Fonction* : Géocodage par lots de fichiers d'adresses en s'appuyant sur l'API de la Base Adresse Nationale (BAN / Etalab).
  * *Licence* : GNU GPL v2.
* **Extension QLR Loader / Géoportail** :
  * *Fonction* : Chargement automatique des flux WMS/WFS du Géoportail de l'Urbanisme et de l'IGN.
  * *Licence* : GNU GPL v2.

---

## 6. Analyse des évolutions QGIS (versions 3.40 LTR à 3.44)

Pour garantir la pérennité du moteur d'exécution QGIS piloté par le logiciel, les évolutions techniques observées entre les versions QGIS 3.40 LTR (octobre 2024) et QGIS 3.44 (octobre 2026) doivent être prises en compte :

1. **Généralisation de `native:zonalstatisticsfb`** :
   L'algorithme de statistiques zonales a été refondu sous la dénomination *Feature Based* (`native:zonalstatisticsfb`), offrant des performances de calcul en mémoire 5 à 10 fois supérieures sur les rasters de grande taille (ex. MNT RGE ALTI ou OCS GE).
2. **Harmonisation des paramètres GDAL (`CREATION_OPTIONS`)** :
   À partir de QGIS 3.40, tous les algorithmes d'export raster GDAL (ex. `gdal:translate`, `gdal:warp`) utilisent la clé normalisée `CREATION_OPTIONS` à la place de l'ancienne nomenclature `CREATE_OPTIONS`.
3. **Intégration native des nuages de points LiDAR HD** :
   De nouveaux algorithmes d'extraction et de conversion de nuages de points (`qgis:pointcloudextraction`, `qgis:pointcloudconversion`) ont été intégrés au Processing Framework pour traiter directement les fichiers `.copc.laz` issus du programme national LiDAR HD de l'IGN.
4. **Exécution autonome via `qgis_process`** :
   Le moteur en ligne de commande `qgis_process` permet d'exécuter l'intégralité des chaînes d'algorithmes (C++, GDAL, GRASS) en tâche de fond sans instancier l'interface graphique QGIS, optimisant le fonctionnement du logiciel SIG bureau en mode piloté.

---

## 7. Lacunes, incertitudes et limites des données

Conformément aux exigences de rigueur du cahier des charges, certaines zones d'ombre subsistent en raison de la nature des données publiques et des pratiques professionnelles :

* **Mesure exacte des trames vertes et bleues (TVB) à l'échelle parcellaire** :
  Il n'existe pas d'algorithme unique et standardisé pour cartographier la continuité écologique à la parcelle. Les pratiques varient fortement d'un bureau d'études à l'autre (combinaison personnalisée de graph-theory et d'indices de perméabilité sous GRASS ou SAGA).
* **Délais d'actualisation des Fichiers Fonciers Cerema / MAJIC** :
  La comptabilisation de la consommation d'espaces dans les Fichiers Fonciers présente un décalage temporel de 1 à 2 ans par rapport aux travaux réels sur le terrain (notamment pour les phases d'aménagement des ZAC).
* **Données sur les réseaux enterrés (Eau / Assainissement)** :
  Les données précises de réseaux sous standard CNIG RAEP sont restreintes par les gestionnaires pour des raisons de protection des infrastructures. Les algorithmes de topologie de réseaux ne peuvent être testés que sur des jeux de données de démonstration.

---

## 8. Répertoire des sources documentaires

1. **QGIS Project** (2026). *Processing providers and algorithms — QGIS Documentation (v3.44 / 3.40 LTR)*. Éditeur : QGIS Project. URL : docs.qgis.org.
2. **CNIG — Conseil National de l'Information Géolocalisée** (2024). *Standard CNIG du Plan Local d'Urbanisme (PLU/PLUi) v2024-01*. Éditeur : CNIG. URL : cnig.gouv.fr.
3. **Cerema & Ministère de la Transition Écologique** (2025). *Zéro Artificialisation Nette (ZAN) — Fascicule 1 : Définir et observer la consommation d'espaces et l'artificialisation des sols*. Éditeur : Ministère de la Transition Écologique / Cerema.
4. **Géoportail de l'Urbanisme (GPU)** (2024). *Manuel prestataire et délégataire du GPU (v4.3)*. Éditeur : IGN / GPU.
5. **IGN — Institut National de l'Information Géographique et Forestière** (2026). *Plan d'activités IGN 2026 : Données souveraines et appui aux politiques publiques*. Éditeur : IGN.
6. **Forum GeoRezo** (2026). *Echanges et discussions autour du SIG bureautique libre QGIS et de la géomatique en France*. Éditeur : GeoRezo.
