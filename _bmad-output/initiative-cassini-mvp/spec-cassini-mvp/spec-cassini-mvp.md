---
id: SPEC-cassini-mvp
companions:
  - ../../../CLAUDE.md
  - ../research-sig-francais-hors-ligne-pour-la-defense/research-sig-francais-hors-ligne-pour-la-defense.md
sources: []
---

> **Contrat canonique.** Cette spec et les fichiers listés dans `companions:` forment le contrat complet de ce qu'il faut construire, tester et valider. Les titres de sections et les clés `intent` / `success` restent en anglais parce que les skills BMad en aval les lisent ; le contenu est en français, conformément à CLAUDE.md §9.
>
> **Périmètre de ce dépôt :** le **noyau civil** de Cassini, publié sous Apache-2.0. Toute fonction spécialement conçue pour l'usage militaire vit dans un dépôt privé distinct et n'est pas spécifiée ici.

# Cassini — MVP (noyau civil)

## Why

Une vision à réaliser. Les bureaux d'études en urbanisme et environnement, les collectivités et les étudiants dépendent d'ArcGIS Pro pour des travaux courants : organiser un projet, charger leurs couches, les croiser, sortir une carte. Ils paient une licence propriétaire et travaillent dans un outil qui suppose une connexion et un compte, alors que leurs données sont déjà sur leur disque et que le travail n'a besoin de rien d'autre. Cassini existe pour que ces usages se fassent dans un logiciel libre, de bureau, organisé en projets comme ArcGIS, qui fonctionne entièrement hors ligne, et qui se pilote en langage naturel plutôt qu'à travers une arborescence de menus — ce que les modèles de langage locaux rendent possible aujourd'hui seulement. L'utilisateur reste propriétaire de ses données : rien n'est téléchargé, rien n'est envoyé.

Le fonctionnement totalement déconnecté ouvre un second débouché, sur postes isolés du réseau. La recherche de domaine a montré que ce n'est pas le calcul de terrain qui y est rare — l'intervisibilité sur modèle d'élévation existe dans les systèmes embarqués français depuis au moins 2011, et deux éditeurs tiennent déjà ce créneau — mais bien l'interface : aucun SIG de bureau hors ligne piloté par une IA locale n'a été identifié. Cassini se positionne donc en **complément** d'un existant installé, pas en remplaçant.

## Capabilities

CAP-2 a été retirée (préparation des données d'une commune par code INSEE) : l'identifiant n'est jamais réattribué. CAP-12 (fonction réservée au module privé) est spécifiée hors de ce dépôt, dans le module défense privé, et n'est pas développée avant réponse écrite à la demande de classement.

- **CAP-1** — Carte
  - **intent:** l'utilisateur affiche ses couches vectorielles par-dessus un fond de carte, sans aucun accès réseau.
  - **success:** l'application ouvre un projet et affiche ses couches avec un fond de carte lu depuis un fichier local, sur une machine déconnectée du réseau, et la navigation (zoom, déplacement) reste fluide sur les données d'une commune entière.

- **CAP-3** — Analyses vectorielles
  - **intent:** l'utilisateur répond à une question spatiale en combinant des analyses élémentaires sur les couches de son projet.
  - **success:** zone tampon, intersection, sélection par attribut, statistiques par zone et isochrones à pied et en voiture s'exécutent chacune sur les couches d'un projet et donnent un résultat vérifiable par un test, hors ligne.

- **CAP-4** — Agent en langage naturel
  - **intent:** l'utilisateur formule sa demande en français et obtient le travail fait, sans connaître les commandes.
  - **success:** sur un projet où les couches nécessaires sont déjà importées, la phrase de démo de CLAUDE.md §1 produit une suite de commandes affichée étape par étape, que l'utilisateur valide avant exécution et peut rejouer ensuite à l'identique.

- **CAP-5** — Export PDF
  - **intent:** l'utilisateur sort une carte présentable à un tiers depuis ce qu'il voit à l'écran.
  - **success:** le PDF produit porte titre, légende, échelle, flèche du nord et mention des sources, et s'ouvre dans un lecteur PDF standard.

- **CAP-6** — Registre de commandes
  - **intent:** chaque fonction du produit est décrite une seule fois et appelable à l'identique par l'interface, la CLI, l'agent local et un client MCP.
  - **success:** chaque commande porte un `id`, une description courte en français et en anglais, des paramètres à schéma JSON, un résultat typé, une implémentation Rust et au moins un test ; aucune logique métier n'existe hors du registre, et un même appel donne le même résultat depuis les quatre points d'entrée.

- **CAP-7** — Historique rejouable
  - **intent:** l'utilisateur retrouve et rejoue ce qui a été fait dans un projet, pour vérifier ou refaire un travail.
  - **success:** toute exécution de commande est journalisée dans le projet, et rejouer l'historique sur les mêmes données importées redonne les mêmes résultats.

- **CAP-8** — Serveur MCP
  - **intent:** une IA externe choisie par l'utilisateur pilote Cassini à travers le même registre de commandes, en option.
  - **success:** un client MCP tiers découvre les commandes et en exécute une ; l'application garde toutes ses fonctions quand aucune IA externe n'est branchée.

- **CAP-9** — Import des données de l'utilisateur
  - **intent:** l'utilisateur ajoute à son projet les fichiers géographiques qu'il possède déjà, sans préparation préalable ni outil tiers.
  - **success:** un fichier des formats retenus au MVP — GeoPackage (y compris un GeoPackage conforme au profil de défense du DGIWG, qui se lit comme un GeoPackage ordinaire), Shapefile, GeoJSON, CSV à coordonnées, GeoParquet — est importé dans un projet, reprojeté si son système de coordonnées diffère de celui du projet, visible dans la carte et utilisable par CAP-3 et CAP-11 ; le fichier d'origine de l'utilisateur est inchangé sur le disque.

- **CAP-10** — Projet
  - **intent:** l'utilisateur regroupe un travail dans un projet qu'il retrouve à l'identique en le réouvrant, et qu'il peut déplacer ou archiver.
  - **success:** créer, ouvrir et enregistrer un projet conserve les couches, leur ordre, leur symbologie, l'étendue de la carte, la mise en page d'export et l'historique des commandes ; réouvrir un projet déplacé sur un autre poste le restitue à l'identique.

- **CAP-11** — Analyse de terrain
  - **intent:** l'utilisateur interroge le relief de sa zone d'étude : ce qui est visible depuis un point, ce qui est à l'abri des vues, la pente, l'exposition, les points dominants, la franchissabilité.
  - **success:** à partir d'un modèle numérique d'élévation importé, chacune des commandes de terrain — champ de vision, ligne de vue entre deux points, intervisibilité entre plusieurs points, pente, exposition, ombrage, zones masquées, points dominants, franchissabilité — s'exécute hors ligne et donne un résultat vérifiable par un test. Ce sont des fonctions standard de SIG, présentes dans les outils libres existants, et non des fonctions spécialement conçues pour un usage militaire.

## Constraints

- Le dépôt public ne contient aucune fonction spécialement conçue pour l'usage militaire. Le module défense est un dépôt privé distinct. Fondement : la charnière du contrôle de la liste commune des équipements militaires de l'UE est l'expression *specially designed for military use*.
- Aucune fonction de fonction réservée au module privé n'est écrite avant réponse écrite à une demande de classement (formulaire [retiré], Bureau de la réglementation, du classement, du double-usage et de la sécurité industrielle).
- Cassini ne télécharge, ne catalogue et ne redistribue aucune donnée géographique. Cadastre, PLU, BAN, BD TOPO, ADMIN EXPRESS et fonds de carte sont apportés par l'utilisateur.
- Hors ligne total : aucune fonction du MVP n'utilise le réseau. Le réseau ne sert qu'à brancher une IA externe, en option.
- Aucune dépendance GPL ou LGPL, y compris par liaison statique et y compris transitivement. En conséquence directe : ni GEOS, ni l'extension `spatial` de DuckDB qui le lie statiquement. Toute dépendance à licence agrégée, GDAL en particulier, est auditée avant adoption.
- Le moteur est une chaîne Rust : `georust/geo` pour la géométrie vectorielle, lecture GeoPackage via SQLite, analyse de terrain reprise ou réimplémentée depuis `whitebox_next_gen` (MIT OR Apache-2.0).
- Clean-room : seules la documentation publique d'Esri et le comportement observable sont admis comme sources. Copier du code, des icônes, des noms de produits, des presets ou des assets d'Esri est interdit. Lire un format publié (Shapefile) est permis ; lire ou écrire un format de projet Esri ne l'est pas.
- La licence d'une dépendance est demandée et donnée avant son ajout. Licences autorisées : MIT, Apache-2.0, BSD.
- Agent-first : toute action utilisateur passe par une commande du registre. Aucune logique métier dans l'interface.
- Indépendance des fournisseurs d'IA : le code ne dépend d'aucune IA en particulier. IA locale par défaut, IA externes via MCP.
- Les fichiers d'origine de l'utilisateur ne sont jamais modifiés ni déplacés.
- Petit et solide : à effort égal, la fiabilité passe avant le nombre de fonctions.
- Application de bureau sur Windows, macOS et Linux.
- Chaque nouvelle commande est livrée avec au moins un test.
- Aucune API ni aucun schéma de données n'est supposé : tout est vérifié dans la documentation officielle de la source.
- Une étape de la feuille de route ne commence pas avant que la précédente soit testée et commitée.
- Une tâche courte par session de travail ; une tâche qui paraît grosse est découpée avant d'être codée.
- Code et noms en anglais, commentaires et documentation utilisateur en français. Rust sans avertissement `cargo fmt` ni `clippy`, TypeScript en mode strict.
- Licence du noyau public : Apache-2.0.

## Non-goals

- Toute fonction spécialement conçue pour l'usage militaire, dans ce dépôt.
- Téléchargement, catalogue ou redistribution de données géographiques.
- Lecture ou écriture des formats de projet Esri (`.aprx`, `.mxd`).
- Formats et symbologies de défense au MVP : DTED, NITF, APP-6. Le profil GeoPackage du DGIWG, lui, est couvert par CAP-9 puisqu'il se lit comme un GeoPackage ordinaire.
- Édition avancée de données.
- 3D.
- IA appliquée aux images satellite.
- Système de plugins.
- Travail collaboratif.
- Version web.
- Parité fonctionnelle mesurée avec ArcGIS Pro : seuls les usages courants de CAP-1, CAP-3 à CAP-5 et CAP-9 à CAP-11 sont visés.

## Success signal

Sur une machine débranchée du réseau, l'utilisateur crée un projet, y importe ses propres fichiers (cadastre, zonage du PLU, gares), puis tape « Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF. » Cassini affiche la suite de commandes qu'il va exécuter, l'utilisateur la valide, et il obtient un PDF titré, légendé, à l'échelle, avec flèche du nord et sources — puis rejoue la même séquence à l'identique depuis l'historique du projet, après avoir fermé et réouvert le projet.

## Assumptions

- Les étapes de CLAUDE.md §7 sont l'ordre de livraison et deviendront les epics du découpage.
- Un projet est un dossier sur le disque, déplaçable et versionnable, et non un fichier binaire unique, puisque CAP-7 exige un historique lisible et rejouable.
- L'import écrit une copie en GeoParquet dans le projet, comme ArcGIS importe dans sa géodatabase, pour ne pas abîmer des livrables clients.
- « Remplacer ArcGIS Pro pour les usages courants » signifie couvrir CAP-1, CAP-3 à CAP-5 et CAP-9 à CAP-11, sans objectif de parité mesurée.
- Villeneuve-le-Roi reste le territoire de référence de la démo, avec des données apportées par l'utilisateur et non livrées.

## Open Questions

- **Maturité du moteur de terrain.** `whitebox_next_gen` expose-t-il son viewshed comme bibliothèque réutilisable, et à quel niveau de maturité ? Le projet est en version 0.1.0 et conduit en open core. À trancher par compilation d'un cas d'essai avant de s'engager sur CAP-11.
- **Fond de carte.** D'où vient-il, puisque rien n'est téléchargé : apporté par l'utilisateur en PMTiles ou MBTiles, livré léger avec l'application, ou aucun fond par défaut ? CAP-1 en dépend.
- **Connaissance des schémas français sans les données.** Garde-t-on la capacité à reconnaître un export cadastre Etalab ou un zonage GPU et à proposer la symbologie et le nommage de champs correspondants ?
- **Systèmes de coordonnées.** Lambert-93 (EPSG:2154) et WGS84 suffisent-ils, ou faut-il aussi les Lambert zones et NTF (EPSG:27572) pour les fichiers anciens ? Et MGRS entre-t-il au MVP, sa conversion étant de l'arithmétique peu coûteuse ?
- **Jeu de recette.** Avec quelles données la démo et les tests tournent-ils, puisque l'application n'en distribue aucune : des données fabriquées à la main, ou un échantillon réel sous licence ouverte versé dans le dépôt ?
- **Isochrones.** Quel réseau routier l'utilisateur doit-il apporter, et quels attributs minimaux sont exigés ? Et quel moteur de routage, sous quelle licence MIT, Apache-2.0 ou BSD ?
- **Définition de « parcelle constructible ».** En termes de quels champs du Géoportail de l'Urbanisme ? La couverture du PLU y est partielle et les schémas varient d'une commune à l'autre, alors que la phrase de démo repose sur cette définition.
- **Livraison du modèle local.** Le modèle GGUF pèse plusieurs Go : part-il dans l'installeur, ou est-il téléchargé au premier lancement ? La seconde option contredit « hors ligne total » sur un poste isolé.
- **Modèle et configuration minimale.** Mistral ou Qwen, quelle quantification, et quelle configuration minimale de RAM et de CPU sont visées pour l'agent local ?
- **Classement.** La demande déposée par [retiré] est le préalable à tout développement de CAP-12. Tant qu'elle n'a pas reçu de réponse écrite, le périmètre du module défense reste indéterminé.
