---
id: SPEC-cassini-mvp
companions:
  - ../../../CLAUDE.md
  - dossier de recherche initial, hors de ce dépôt (dépôt privé)
  - ../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md
  - catalogue-des-commandes.md
sources: []
---

> **Contrat canonique.** Cette spec et les fichiers listés dans `companions:` forment le contrat complet de ce qu'il faut construire, tester et valider. Les titres de sections et les clés `intent` / `success` restent en anglais parce que les skills BMad en aval les lisent ; le contenu est en français, conformément à CLAUDE.md §9.
>
> **Périmètre de ce dépôt :** le **noyau civil** de Cassini, publié sous Apache-2.0, plus l'extension QGIS qui lui sert d'interface au MVP. Toute fonction spécialement conçue pour l'usage militaire vit dans un dépôt privé distinct et n'est pas spécifiée ici.

# Cassini — MVP (noyau civil et façade QGIS)

## Why

Une vision à réaliser. Les bureaux d'études en urbanisme et environnement, les collectivités et les étudiants disposent déjà d'un SIG de bureau libre, hors ligne et organisé en projets : QGIS. Ce qui leur manque n'est ni une carte, ni des analyses, ni un export PDF — c'est l'accès à tout cela. L'outil suppose qu'on sait déjà où cliquer : un utilisateur occasionnel, qui sait ce qu'il veut mais pas comment le faire, reste bloqué devant une arborescence de menus et de boîtes de dialogue. Cassini existe pour que la demande se formule en français et se traduise en une suite d'opérations vérifiables, **sur une machine qui ne téléphone jamais à l'extérieur** — ce que les modèles de langage locaux rendent possible aujourd'hui seulement. L'utilisateur reste propriétaire de ses données : rien n'est téléchargé, rien n'est envoyé.

Le fonctionnement totalement déconnecté ouvre un second débouché, sur postes isolés du réseau. La recherche de domaine a montré que ce n'est pas le calcul de terrain qui y est rare — l'intervisibilité sur modèle d'élévation existe dans les systèmes embarqués français depuis au moins 2011, et QGIS la fait gratuitement par GRASS — mais bien l'interface : aucun SIG de bureau hors ligne piloté par une IA locale n'a été identifié. Le fossé concurrentiel est donc l'agent hors ligne, pas la géométrie ni la visibilité.

Conséquence assumée sur la construction : Cassini ne reconstruit pas ce que QGIS fait déjà. Au MVP, **QGIS exécute** la carte, l'ouverture des fichiers, le projet, les analyses vectorielles et l'export PDF ; le noyau Cassini porte le registre de commandes, l'agent, le journal rejouable et un moteur de terrain en licence permissive. L'application de bureau autonome est reportée en V2.

## Capabilities

CAP-2 a été retirée (préparation des données d'une commune par code INSEE) : l'identifiant n'est jamais réattribué. CAP-12 est spécifiée hors de ce dépôt, dans le module privé.

- **CAP-1** — Affichage hors ligne vérifié
  - **intent:** l'utilisateur travaille sur ses couches et son fond de carte sans qu'aucune fonction du produit n'accède au réseau.
  - **success:** des couches et un fond de carte lus depuis des fichiers locaux s'affichent sur une machine déconnectée, et une vérification automatique — et non une lecture de code — prouve qu'aucune requête réseau n'est émise sur le chemin nominal, côté noyau comme côté extension.

- **CAP-3** — Analyses vectorielles
  - **intent:** l'utilisateur répond à une question spatiale en combinant des analyses élémentaires sur les couches de son projet.
  - **success:** zone tampon, intersection, sélection par attribut, statistiques par zone et isochrones à pied et en voiture sont chacune une commande du registre, **exécutée par le cadre de traitement de QGIS**, et donnent un résultat vérifiable par un test, hors ligne.

- **CAP-4** — Agent en langage naturel
  - **intent:** l'utilisateur formule sa demande en français et obtient le travail fait, sans connaître les commandes.
  - **success:** sur un projet où les couches nécessaires sont déjà ouvertes, la phrase de démo de CLAUDE.md §1 produit une suite de commandes affichée étape par étape dans le panneau Cassini de QGIS, que l'utilisateur valide avant exécution et peut rejouer ensuite à l'identique.

- **CAP-5** — Export PDF
  - **intent:** l'utilisateur sort une carte présentable à un tiers depuis ce qu'il voit à l'écran.
  - **success:** une commande du registre fait produire par le moteur de mise en page de QGIS un PDF portant titre, légende, échelle, flèche du nord et mention des sources, au format et à l'orientation demandés, avec un carroyage en option, qui s'ouvre dans un lecteur PDF standard.

- **CAP-6** — Registre de commandes
  - **intent:** chaque fonction du produit est décrite une seule fois et appelable à l'identique par l'extension QGIS, la CLI, l'agent local et un client MCP.
  - **success:** chaque commande porte un `id`, une description courte en français et en anglais, des paramètres à schéma JSON, un résultat typé, une implémentation Rust ou une délégation déclarée à QGIS, et au moins un test ; aucune logique métier n'existe hors du registre, et un même appel donne le même résultat depuis les quatre points d'entrée.

- **CAP-7** — Journal rejouable
  - **intent:** l'utilisateur retrouve et rejoue ce qui a été fait, pour vérifier ou refaire un travail.
  - **success:** toute exécution de commande est journalisée dans un journal Cassini écrit à côté du projet QGIS, et rejouer ce journal sur les mêmes fichiers redonne les mêmes résultats, y compris après fermeture et réouverture du projet ; le journal ne contient aucun chemin absolu, ce qui le garde rejouable après déplacement du dossier (CAP-10).

- **CAP-8** — Serveur MCP
  - **intent:** une IA externe choisie par l'utilisateur pilote Cassini à travers le même registre de commandes, en option.
  - **success:** un client MCP tiers découvre les commandes et en exécute une ; quand une session QGIS est ouverte et que l'utilisateur l'autorise, il pilote cette session et chaque appel est visible dans le panneau ; vérifié avec un client de chacune des familles Claude, GPT et Gemini ; le produit garde toutes ses fonctions quand aucune IA externe n'est branchée (changement du 2026-10-30).

- **CAP-9** — Ouverture des données de l'utilisateur
  - **intent:** l'utilisateur ajoute à son projet les fichiers géographiques qu'il possède déjà, sans préparation préalable ni outil tiers.
  - **success:** une commande du registre fait ouvrir par QGIS un fichier des formats retenus au MVP — GeoPackage (y compris un GeoPackage conforme au profil de défense du DGIWG, qui se lit comme un GeoPackage ordinaire), Shapefile, GeoJSON, CSV à coordonnées, GeoParquet — la couche apparaît dans le projet, est reprojetée si son système diffère de celui du projet, et devient utilisable par CAP-3 et CAP-11 ; le fichier d'origine de l'utilisateur est inchangé sur le disque.

- **CAP-10** — Projet
  - **intent:** l'utilisateur regroupe un travail dans un projet qu'il retrouve à l'identique en le réouvrant, et qu'il peut déplacer ou archiver.
  - **success:** le projet est un projet QGIS ; une commande du registre en dresse l'inventaire (couches, ordre, étendue, système de coordonnées) et le journal Cassini de CAP-7 vit à côté du fichier de projet ; déplacer l'ensemble sur un autre poste restitue le projet et son journal.

- **CAP-11** — Analyse de terrain
  - **intent:** l'utilisateur interroge le relief de sa zone d'étude : ce qui est visible depuis un point, ce qui se voit entre deux points, la pente.
  - **success:** à partir d'un modèle numérique d'élévation ouvert dans le projet, chacune des trois commandes de terrain du MVP — champ de vision, ligne de vue entre deux points, pente — s'exécute **dans le noyau Rust**, hors ligne, sans aucune dépendance GPL ou LGPL, et donne un résultat vérifiable par un test. Ce sont des fonctions standard de SIG, présentes dans les outils libres existants, et non des fonctions spécialement conçues pour un usage militaire.

- **CAP-13** — Façade QGIS
  - **intent:** l'utilisateur pilote Cassini depuis QGIS, sans quitter l'outil où ses données sont déjà ouvertes.
  - **success:** une extension QGIS installable ouvre un panneau Cassini, se connecte au noyau lancé comme **processus séparé**, et exécute une commande du registre de bout en bout en affichant son résultat dans le projet ; et le noyau se lance, s'exécute et se teste **sans QGIS installé**.

- **CAP-15** — Géométrie et terrain étendu *(ajoutée le 2026-11-05, avant la démo)*
  - **intent:** l'utilisateur convertit et mesure des coordonnées, et interroge le relief au-delà des trois commandes de CAP-11.
  - **success:** conversion de coordonnées ponctuelles entre WGS84, UTM et MGRS, distance et azimut géodésiques, point par azimut et distance, recoupement, exposition, ombrage, champ de vision cumulé depuis plusieurs points, points hauts, surface de coût cumulé et chemin de moindre coût, reclassement et superposition pondérée de rasters : chacune est une commande du registre exécutée **dans le noyau Rust**, hors ligne, sans dépendance GPL ou LGPL, testée sans QGIS sur des jeux de référence publics ou des fixtures synthétiques ; le modèle de composition les enchaîne dans un plan valide ; la recette hors ligne de CAP-1 reste verte. Ce sont des fonctions standard de SIG et de topographie ; les paramètres propres à un usage particulier ne sont pas portés par le noyau.

- **CAP-14** — Catalogue étendu *(après le MVP)*
  - **intent:** l'utilisateur obtient en langage naturel une réponse à la plupart de ses questions SIG courantes, au-delà des cinq analyses de CAP-3 : comptage et somme pondérés dans un polygone, plus proche voisin et matrice de distances, plus court chemin sur réseau, statistiques par catégorie et longueurs, reprojection, conversion de format, atlas PDF, statistiques zonales d'un raster — moyenne, minimum et maximum des altitudes d'un MNT par polygone compris —, préparation d'un réseau d'itinéraires à partir de chemins bruts, géocodage d'adresses sur un fichier de la Base Adresse Nationale qu'il apporte, et, ajoutés le 2026-11-04 d'après la recherche sur les tâches des professionnels, différence de couches, surface des entités, jointure d'une table, fusion par champ, reclassement d'un raster et courbes de niveau.
  - **success:** chaque primitive retenue est une commande du registre **exécutée par le cadre de traitement de QGIS** — sauf le géocodage, commande du noyau testée sans QGIS —, avec un test hors ligne, que l'agent compose dans un plan valide ; sur un jeu de phrases réelles d'utilisateurs, au moins 70 % des demandes sont couvertes (un plan valide ou une question justifiée), mesuré et consigné ; la recette hors ligne de CAP-1 reste verte.

## Constraints

- L'extension QGIS est sous GPL, QGIS l'étant. Elle vit séparée du noyau ; le noyau reste sous Apache-2.0, ne lie jamais QGIS, et communique avec l'extension à distance de bras, par processus séparé.
- Le noyau doit rester pleinement utilisable sans QGIS. C'est ce qui fonde l'argument de distance de bras sur la GPL et ce qui rend l'application autonome de V2 possible.
- Aucune dépendance GPL ou LGPL **dans le noyau**, y compris par liaison statique et y compris transitivement : ni GEOS, ni l'extension `spatial` de DuckDB qui le lie statiquement. Passer par QGIS ne lève pas cette interdiction : QGIS embarque GDAL et GEOS de son côté, l'extension les utilise dans le processus de QGIS, le noyau ne les lie jamais.
- Le module défense ne peut pas être construit dans l'extension, celle-ci étant en GPL : il s'appuiera sur le noyau Apache-2.0.
- Le dépôt public ne contient aucune fonction spécialement conçue pour l'usage militaire. Le module défense est un dépôt privé distinct. Fondement : la charnière du contrôle de la liste commune des équipements militaires de l'UE est l'expression *specially designed for military use*.
- Cassini ne télécharge, ne catalogue et ne redistribue aucune donnée géographique. Cadastre, PLU, BAN, BD TOPO, ADMIN EXPRESS et fonds de carte sont apportés par l'utilisateur.
- Hors ligne total : aucune fonction du MVP n'utilise le réseau. Le réseau ne sert qu'à brancher une IA externe, en option.
- Le canal entre l'extension et le noyau ne porte que des messages publics et documentés : identifiants, chemins relatifs, paramètres à schéma JSON, résultats typés, et quelques géométries ponctuelles en GeoJSON. Jamais une couche entière, jamais une structure de données interne : c'est la condition de l'argument de distance de bras, la FAQ de la FSF réservant le cas d'un échange assez intime pour que deux programmes n'en fassent qu'un.
- Le plan composé par le modèle local est une entrée non fiable : il ne contient que des appels de commandes du registre, validés contre leurs schémas, et rien de ce que le modèle produit n'est exécuté comme du code ou une expression.
- L'extension fonctionne sur QGIS 3.44 LTR et sur la série 4.x avec un seul code : les bureaux d'études et collectivités visés restent des années sur une version LTR.
- Distribution en deux artefacts : l'extension au dépôt d'extensions QGIS, sans binaire et sous 25 Mo ; le noyau et le modèle dans un paquet Cassini installé à part. Une clé USB portant les deux suffit à installer Cassini sur un poste qui n'a jamais vu le réseau, et rien n'est jamais téléchargé pour réparer une installation.
- Le noyau est une chaîne Rust : les trois commandes de terrain y sont réimplémentées, le portage depuis `whitebox_next_gen` (MIT OR Apache-2.0) restant permis avec attribution, et le noyau lit lui-même le modèle numérique d'élévation. La géométrie vectorielle et la lecture des formats sont déléguées à QGIS au MVP ; `georust/geo` ne redevient nécessaire qu'avec l'application autonome de V2.
- Toute dépendance à licence agrégée, GDAL en particulier, est auditée avant adoption dans le noyau.
- Clean-room : seules la documentation publique d'Esri et le comportement observable sont admis comme sources. Copier du code, des icônes, des noms de produits, des presets ou des assets d'Esri est interdit. Lire un format publié (Shapefile) est permis ; lire ou écrire un format de projet Esri ne l'est pas.
- La licence d'une dépendance est demandée et donnée avant son ajout. Licences autorisées dans le noyau : MIT, Apache-2.0, BSD.
- Agent-first : toute action utilisateur passe par une commande du registre. Aucune logique métier dans l'extension.
- Indépendance des fournisseurs d'IA : le code ne dépend d'aucune IA en particulier. IA locale par défaut, IA externes via MCP.
- Les fichiers d'origine de l'utilisateur ne sont jamais modifiés ni déplacés.
- Petit et solide : à effort égal, la fiabilité passe avant le nombre de fonctions.
- Le noyau se compile sur Windows, macOS et Linux ; l'extension cible les plateformes où QGIS tourne.
- Chaque nouvelle commande est livrée avec au moins un test.
- Tout seuil d'une commande de CAP-14 (distance réglementaire, pente, surface minimale) est un paramètre, jamais une constante.
- Les fournisseurs de traitement sous GPL (GRASS, extensions tierces) ne s'exécutent que dans le processus de QGIS, jamais liés au noyau. Le géocodeur intégré de QGIS interroge un service en ligne : il est exclu.
- Le géocodage est calculé par le noyau, l'appariement d'adresses étant un raisonnement et non un geste QGIS. Il ne lit que des CSV (adresses, Base Adresse Nationale) désignés par chemin relatif, écrit un seul CSV de résultat dans le dossier de travail sans toucher aux originaux, et ne reprojette pas : coordonnées `lon`/`lat` de la BAN en EPSG:4326, la couche étant ajoutée ensuite par QGIS.
- Les statistiques zonales d'un raster, MNT compris, sont calculées par QGIS et non par le noyau : le calcul exige les polygones, que le canal ne transporte jamais. CAP-11 garde ses trois commandes de terrain ; les autres calculs de terrain relèvent de CAP-15.
- Le catalogue des commandes (`catalogue-des-commandes.md`) fige l'identifiant, les paramètres et le résultat de chaque commande, faite, planifiée ou réservée ; un identifiant ou un paramètre n'y change que par un changement de cap (2026-11-05).
- Aucune API ni aucun schéma de données n'est supposé : tout est vérifié dans la documentation officielle de la source.
- Une étape de la feuille de route ne commence pas avant que la précédente soit testée et commitée.
- Une tâche courte par session de travail ; une tâche qui paraît grosse est découpée avant d'être codée.
- Code et noms en anglais, commentaires et documentation utilisateur en français. Rust sans avertissement `cargo fmt` ni `clippy`, TypeScript en mode strict, extension Python contrôlée par la recette.
- Licence du noyau public : Apache-2.0.

## Non-goals

- Application de bureau autonome au MVP : la coquille Tauri, la carte MapLibre et une chaîne vectorielle propre sont reportées en V2. Le code déjà écrit reste au dépôt comme fondation.
- Toute fonction spécialement conçue pour l'usage militaire, dans ce dépôt.
- Hydrologie (écoulement, bassin versant, zone inondable) et couverture radio au MVP : identifiants réservés au catalogue, reportées après la démo. Les paramètres de mobilité propres à un véhicule : jamais dans le noyau, qui fournit surface de coût et chemin de moindre coût. (Changement du 2026-11-05 : l'intervisibilité multi-points, l'exposition, l'ombrage et les points dominants passent à CAP-15 ; les zones masquées sont rendues par le champ de vision.)
- Parité fonctionnelle avec QGIS, mesurée ou revendiquée. Cassini ajoute une interface, il ne remplace pas l'outil.
- Système de plugins **hébergé par** Cassini. À ne pas confondre avec le fait que Cassini **est** une extension au MVP.
- Téléchargement, catalogue ou redistribution de données géographiques.
- Lecture ou écriture des formats de projet Esri (`.aprx`, `.mxd`).
- Formats et symbologies de défense au MVP : DTED, NITF, APP-6. Le profil GeoPackage du DGIWG, lui, est couvert par CAP-9 puisqu'il se lit comme un GeoPackage ordinaire.
- Rejeu d'un journal sans QGIS au MVP : les commandes déléguées exigent QGIS pour être rejouées ; une étape impossible est nommée en échec, jamais sautée.
- Édition avancée de données.
- 3D.
- IA appliquée aux images satellite.
- Travail collaboratif.
- Version web.

## Success signal

Sur une machine débranchée du réseau, l'utilisateur ouvre QGIS sur un projet contenant ses propres fichiers (cadastre, zonage du PLU, gares), ouvre le panneau Cassini et tape « Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF. » Le panneau affiche la suite de commandes qu'il va exécuter, l'utilisateur la valide, et il obtient un PDF titré, légendé, à l'échelle, avec flèche du nord et sources — puis rejoue la même séquence à l'identique depuis le journal du projet, après l'avoir fermé et réouvert.

## Assumptions

- Le projet est un projet QGIS ; le noyau n'invente pas de format de projet et écrit son journal à côté.
- L'ordre de livraison est la séquence décidée le 2026-10-14 : socle et recette, épreuve du modèle local, registre et journal, façade QGIS, agent et MCP, terrain, géométrie et terrain étendu (epic 11, ajouté le 2026-11-05), démo, publication — celle que porte CLAUDE.md §7.
- Le dérisquage précède l'engagement : qu'un modèle local quantifié sache composer une suite de commandes typées correcte est vérifié sur un registre factice avant que le registre réel ne soit écrit.
- « Compléter QGIS pour les usages courants » signifie couvrir par le registre CAP-3, CAP-5, CAP-9 à CAP-11 et CAP-15 au MVP, puis CAP-14 après le MVP, sans objectif de parité. Le signal de succès ci-dessus reste celui du MVP ; CAP-14 porte son propre seuil de couverture.
- Villeneuve-le-Roi reste le territoire de référence de la démo, avec des données apportées par l'utilisateur et non livrées.

## Open Questions

- **Confirmation de la frontière GPL.** Le raisonnement de distance de bras est à faire confirmer avant toute vente, pas avant d'écrire du code.
- **Surface de l'API PyQGIS** réellement nécessaire, à fixer geste par geste avec les commandes déléguées.
- **Modèle et configuration minimale.** Mistral ou Qwen, quelle quantification, et quelle configuration minimale de RAM et de CPU sont visées pour l'agent local ?
- **Dessin du panneau.** L'expérience devient un panneau ancré dans QGIS, outil dense et d'expert, alors que l'argument de vente est qu'un opérateur non cartographe obtient sa réponse en parlant. À esquisser avant d'ouvrir l'epic de façade.
- **Maturité de la lecture du relief.** `wbgeotiff` 0.1.2 et `wbraster` 0.2.1 lisent-ils de façon fiable les modèles d'élévation visés ? Bibliothèques jeunes, à trancher par un cas d'essai compilé en tête de l'epic terrain, avant de s'engager sur CAP-11.
- **Connaissance des schémas français sans les données.** Garde-t-on la capacité à reconnaître un export cadastre Etalab ou un zonage GPU et à proposer la symbologie et le nommage de champs correspondants ?
- **Systèmes des modèles d'élévation.** Le noyau lit le MNT dans son propre système et refuse une coordonnée exprimée dans un autre : quels systèmes doit-il savoir lire — Lambert-93 et WGS84 seuls, ou aussi les Lambert zones et NTF (EPSG:27572) des fichiers anciens ? *Tranché le 2026-11-03 (inception de l'epic terrain) : tout système projeté métrique doté d'un code EPSG ; un MNT en degrés est refusé.*
- **Jeu de recette.** Avec quelles données la démo et les tests tournent-ils, puisque l'application n'en distribue aucune : des données fabriquées à la main, ou un échantillon réel sous licence ouverte versé dans le dépôt ?
- **Phrases réelles de mesure de CAP-14.** D'où vient le jeu qui mesure le seuil de 70 % — collecte auprès d'utilisateurs, communauté QGIS ? À fournir par le propriétaire avant l'inception de l'epic 10, au plus tard à la publication (epic 7).
- **Définition de « parcelle constructible ».** En termes de quels champs du Géoportail de l'Urbanisme ? La couverture du PLU y est partielle et les schémas varient d'une commune à l'autre, alors que la phrase de démo repose sur cette définition.
