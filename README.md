# Cassini

Cassini est l'**interface en langage naturel d'un SIG** — un système
d'information géographique — et elle fonctionne **entièrement hors ligne**.
L'utilisateur formule sa demande en français ; Cassini la traduit en une suite
d'opérations qu'elle affiche, fait valider, exécute, puis journalise pour qu'on
puisse les rejouer. Elle s'adresse aux bureaux d'études en urbanisme et en
environnement, aux collectivités et aux étudiants — en particulier à
l'utilisateur occasionnel, qui sait ce qu'il veut mais pas où cliquer.

**L'utilisateur apporte ses propres données.** Cassini n'en télécharge, n'en
catalogue et n'en redistribue aucune.

La phrase de démo, sur un projet où l'utilisateur a déjà ouvert ses fichiers :

> « Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à
> Villeneuve-le-Roi et sors-moi la carte en PDF. »

Elle passe aujourd'hui dans QGIS, sur processeur et sans réseau : le modèle local
compose le plan, l'utilisateur le valide, QGIS l'exécute et le PDF sort. Ce qui
reste avant la publication est listé plus bas.

**Cassini ne refait pas QGIS.** QGIS est libre, hors ligne, organisé en projets
et mûr depuis vingt ans : reconstruire sa carte, son ouverture de fichiers, ses
analyses et son export PDF serait perdre des années pour arriver à moins bien.
Ce qui n'existe nulle part, c'est un SIG de bureau hors ligne piloté par une IA
locale. Au MVP, Cassini est donc une **extension QGIS posée sur un noyau
indépendant** : QGIS exécute la carte, l'ouverture des fichiers, le projet, les
analyses vectorielles et l'export PDF ; le noyau porte le registre de commandes,
l'agent, le journal rejouable et un moteur de terrain et de géométrie en licence
permissive, écrit d'après les publications.

Ce découpage date du 2026-10-14 et remplace un plan où Cassini était une
application de bureau complète ; le raisonnement est écrit en entier dans
[la proposition de changement](_bmad-output/initiative-cassini-mvp/change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md).
L'application autonome n'est pas abandonnée, elle est **reportée en V2**, et le
code déjà écrit pour elle reste au dépôt.

La frontière entre le noyau et l'extension est fixée par
[le contrat d'architecture](_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md),
en décisions numérotées (AD-1 à AD-31) ; ce README en résume ce qui change
la forme du produit.

Depuis le 2026-11-05, la liste des commandes est **figée** dans
[le catalogue des commandes](_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md) :
pour chaque commande — faite, planifiée ou réservée — son identifiant, ses
paramètres et son résultat. Un identifiant ou un paramètre n'y change que par un
changement de cap, pour ne plus avoir à ré-entraîner le modèle à chaque besoin
découvert. La recette confronte le catalogue au registre.
[Le changement de cap](_bmad-output/initiative-cassini-mvp/change-geometrie-et-terrain-avant-la-demo/change-geometrie-et-terrain-avant-la-demo.md)
qui l'a décidé place aussi la géométrie et le terrain étendu **avant la démo**.

## Ce qui marche aujourd'hui

Le projet a franchi les étapes 1 à 6 de `CLAUDE.md` §7, et l'essentiel de
l'étape 6 bis : l'agent local tourne dans QGIS, et le noyau calcule lui-même le
terrain et la géométrie. Concrètement :

- **Le noyau** (`core/`, crate `cassini-core`) : le **registre de commandes**,
  où chaque commande s'inscrit seule et dont le registre contrôle lui-même le
  contrat (id, descriptions française et anglaise, schémas, exécutant, test) ; le
  **journal rejouable**, écrit à côté du projet QGIS en ajout seul sous verrou
  exclusif ; la validation des plans composés par l'agent, dont le garde-fou qui
  refuse une coordonnée que la demande n'a pas fournie.
- **Le binaire `cassini`** (`cli/`) : `run` exécute une commande, `replay` rejoue
  un journal, `serve` ouvre le canal local de l'extension, `plan` compose un plan
  depuis une phrase en ligne de commande, `model-check` vérifie qu'un modèle
  répond, `mcp` expose le registre à une IA externe par le protocole MCP. Un même
  appel donne le même résultat par `run`, par `serve` et par `mcp`, ce que
  contrôle un test sur le vrai binaire.
- **L'extension QGIS** (`qgis-plugin/`) : un panneau ancré dans QGIS 3.44 et
  4.x. On y écrit une demande en français, le plan proposé s'affiche étape par
  étape, on le valide ou on l'arrête, et le journal dit en une phrase ce que
  chaque étape a fait. Un mode expert lance une commande du registre à la main ;
  les Réglages choisissent la langue et le modèle.
- **Les commandes exécutées par QGIS**, toutes déclarées au registre :

  | Commande | Ce qu'elle fait |
  |---|---|
  | `layer.add` | ouvre un GeoPackage, un Shapefile, un GeoJSON, un CSV à coordonnées ou un GeoParquet, sans toucher le fichier |
  | `project.inventory`, `layer.describe` | décrivent le projet et ses couches, pour l'utilisateur et pour l'agent |
  | `layer.remove` | retire une couche du projet, sans effacer ni modifier son fichier ; une couche déjà absente n'est pas une erreur |
  | `analysis.buffer` | zone tampon |
  | `analysis.intersect` | intersection de deux couches |
  | `analysis.select_by_attribute` | sélection par attribut |
  | `analysis.zonal_statistics` | statistiques par zone |
  | `analysis.isochrone` | isochrones à pied et en voiture sur un réseau |
  | `layout.export_pdf` | carte en PDF avec titre, légende, échelle, flèche du nord et sources |
  | `data.transform_point` | reprojette un seul point vers un code EPSG, par exemple un point Lambert-93 vers WGS84 avant une conversion MGRS |

  Deux commandes de plus, `layer.open_raster` et `layer.open_vector`, ouvrent dans
  le projet un raster ou un GeoJSON que le noyau vient d'écrire. Elles ne sont
  **dictées que par le noyau**, après l'étape qui a produit le fichier, et ne sont
  offertes ni au modèle, ni à une IA externe (AD-30).
- **Les commandes calculées par le noyau**, en Rust, testées sans QGIS :

  | Commande | Ce qu'elle fait |
  |---|---|
  | `terrain.slope` | pente d'un MNT, en degrés ou en pourcentage |
  | `terrain.aspect`, `terrain.hillshade` | exposition et ombrage d'un MNT |
  | `terrain.line_of_sight` | ligne de vue entre deux points, courbure et réfraction comprises |
  | `terrain.viewshed` | champ de vision depuis un point |
  | `terrain.viewshed_cumulative` | nombre d'observateurs qui voient chaque cellule |
  | `terrain.peaks` | points hauts d'un MNT, par proéminence, rendus en GeoJSON |
  | `raster.reclassify`, `raster.weighted_overlay` | reclassement d'un raster, superposition pondérée de plusieurs critères |
  | `terrain.cost_distance`, `terrain.least_cost_path` | coût de déplacement cumulé, et chemin de moindre coût rendu en GeoJSON |
  | `coord.convert` | conversion d'une coordonnée entre WGS84, UTM et MGRS |
  | `geo.distance_azimuth` | distance géodésique, azimut et contre-azimut entre deux points |
  | `geo.destination_point` | point placé à un azimut et une distance |
  | `geo.bearing_intersection` | recoupement de deux visées |
  | `system.about`, `model.list` | version du noyau, modèles présents sur le poste |

  Aucune ligne n'en vient de GRASS, de Whitebox, de GDAL ni de QGIS : chaque calcul
  est écrit d'après sa publication (Horn pour la pente, Franklin et Ray pour la
  visibilité, Karney pour la géodésie, la spécification de la NGA pour MGRS), et
  les GeoTIFF sont lus et écrits sans GDAL. Comparés à QGIS et à GRASS sur un vrai
  MNT, les résultats concordent et le noyau répond 2,9 à 124 fois plus vite —
  écarts, temps et sources dans
  [`docs/comparaison-terrain-qgis.md`](docs/comparaison-terrain-qgis.md). Les
  coordonnées sont contrôlées contre des jeux de référence publics (GeographicLib,
  NGA).
- **L'agent local** (`ai/`, crate `cassini-ai`) : un modèle quantifié au format
  GGUF, chargé **dans le processus du noyau** par llama.cpp lié en bibliothèque —
  jamais un serveur HTTP. Le modèle retenu est un Qwen3.5-4B (Apache-2.0) ajusté sur
  le registre de Cassini, choisi dans les Réglages du panneau ; il pose une question au lieu de deviner quand la demande
  est ambiguë, et enchaîne plusieurs phrases dans une même session sans saturer son
  contexte. Il connaît les commandes du MVP et les trois premières de terrain ; les
  commandes ajoutées depuis le 2026-11-05 lui seront apprises en une seule fois, au
  ré-entraînement groupé qui clôt l'étape 6 bis. D'ici là, c'est le Qwen3.5-9B non
  ajusté, avec le registre entier dans son prompt, qui les compose.
- **La chaîne d'entraînement** (`training/`) : le jeu d'exemples est engendré
  depuis le registre lui-même, l'entraînement et la fusion en GGUF se refont d'une
  seule commande, et un jeu tenu à l'écart mesure le modèle. Détail dans
  [`docs/entrainement-du-modele.md`](docs/entrainement-du-modele.md).
- **Le verrou hors ligne**, vérifié autrement que par lecture de code : la table
  d'imports du binaire Windows ne nomme aucune bibliothèque réseau, l'extension
  n'importe aucun module réseau, et `cargo deny` refuse tout client HTTP, toute
  couche TLS et toute licence non permissive dans le noyau.

**La durée par phrase.** L'exigence est qu'une longue phrase soit résolue en moins
de 30 s sur un processeur de bureau ordinaire. Sur dix phrases enchaînées dans une
session QGIS, bridée à quatre cœurs, huit sur neuf passent ; la phrase de démo, en
cours de session, reste juste au-dessus (32 s au mieux). Quand le poste a une carte
graphique dédiée, le noyau s'en sert seul, par Vulkan (NVIDIA, AMD et Intel), et
retombe sur le processeur sans elle, sans aucun réglage. Une carte intégrée n'est
jamais prise. Sur une RTX 3070 Ti, les neuf phrases passent sous 30 s, et la phrase de
démo en cours de session tombe à 14 s. Les relevés sont dans
[`docs/mesure-duree-par-phrase.md`](docs/mesure-duree-par-phrase.md).

**Conservé pour la V2** (`app/`) : une coquille de bureau Tauri v2 qui affiche
une carte MapLibre sur un fond PMTiles lu sur le disque, sans aucune requête
réseau. Elle marche et reste testée, mais elle est hors du chemin critique du MVP
depuis le 2026-10-14.

## Ce qui est prévu

Les sept étapes du projet, telles que les fixe `CLAUDE.md` §7. Aucune étape ne
commence avant que la précédente ne soit testée.

| Étape | Ce qu'elle apporte | État |
|---|---|---|
| 1 | Socle technique, verrou hors ligne, recette automatique | **faite** |
| 2 | Épreuve du modèle local sur un registre factice | **faite** : verdict « oui sous conditions », dans `_bmad-output/initiative-cassini-mvp/epic-epreuve-du-modele-local/` |
| 3 | Registre de commandes et journal rejouable | **faite**, suite de bout en bout comprise : exécuter, fermer, rouvrir, rejouer, comparer |
| 4 | Façade QGIS — extension, canal local, commandes exécutées par QGIS | **faite**, suite de bout en bout comprise : ouvrir, analyser, exporter, rejouer |
| 5 | Agent IA local et serveur MCP | **faite**, suite de bout en bout comprise : l'agent tourne dans QGIS, sur carte graphique ou sur processeur ; le serveur MCP (`cassini mcp`) relaie vers la session QGIS quand l'utilisateur l'autorise, et n'expose sinon que les commandes du noyau |
| 6 | Analyse de terrain dans le noyau — pente, ligne de vue, champ de vision | **faite**, comparaison à QGIS et à GRASS comprise |
| 6 bis | Géométrie et terrain étendu, catalogue des commandes figé | **en cours** : catalogue figé, quatorze commandes faites (tableaux ci-dessus) ; restent le format, l'orientation et le carroyage de la mise en page, l'apprentissage des nouvelles commandes par le modèle, le ménage de fin d'epic et la démonstration de bout en bout dans QGIS |
| 7 | Scénario de démo complet, puis publication : dépôt d'extensions QGIS, installeurs, vidéo | à venir |

Un catalogue étendu est déjà découpé pour l'après-démo (epic 10), et ses contrats
sont déjà figés au catalogue : reprojection et conversion de couches, plus proche
voisin, plus court chemin sur un réseau, atlas PDF, statistiques zonales sur un
MNT, vectorisation et alignement de rasters, courbes de niveau, géocodage sur la
Base Adresse Nationale. Des identifiants sont aussi réservés, sans paramètres
encore, pour l'hydrologie (écoulement, bassin versant, zone inondable) et la
couverture radio d'un émetteur.

Le noyau fournit des briques générales et ne porte aucun paramètre propre à un
usage particulier — classes de véhicules, seuils métier, choix d'un emplacement :
ces recettes se composent au-dessus de lui.

Limite connue de la phrase de démo : « constructible » se traduit aujourd'hui par
la zone urbaine `U` du standard du CNIG ; les zones `AUc` ne sont pas encore
couvertes, faute d'opérateur « parmi » dans le registre.

Formats ouverts par QGIS pour le compte du registre : GeoPackage, Shapefile,
GeoJSON, CSV à coordonnées, GeoParquet. Systèmes de coordonnées : Lambert-93
(EPSG:2154) et WGS84 au minimum, avec reprojection quand la couche diffère du
projet — reprojection exécutée par QGIS, jamais par le noyau. Le noyau ne
convertit que des **coordonnées ponctuelles**, entre WGS84, UTM et MGRS, par des
formules publiées ; tout autre système, et toute couche, restent à QGIS (AD-19).
Raster : modèles numériques d'élévation et rasters de coût ou de critères en
GeoTIFF à une bande. Aucune parité avec QGIS n'est visée
ni revendiquée.

Cassini se distribuera en deux artefacts : l'extension, légère et sans binaire,
par le dépôt d'extensions QGIS ; le noyau et le modèle d'IA, par un paquet Cassini
installé à part. Une clé USB portant les deux suffira sur un poste qui n'a jamais
vu le réseau (AD-17).

## Quatre principes non négociables

1. **Hors ligne total.** Aucune fonction du produit n'utilise le réseau. Il ne
   servira qu'à brancher une IA externe, en option, et jamais par défaut.
2. **Le noyau sans GPL ni LGPL ; l'extension en GPL par conséquence.** Le noyau
   n'admet que MIT, Apache-2.0 et BSD, y compris par liaison statique et y
   compris transitivement — d'où l'exclusion de GEOS. L'extension QGIS, elle, est
   forcément en GPL puisque QGIS l'est : elle vit **séparée** du noyau, le noyau
   ne lie jamais QGIS, et les deux communiquent **à distance de bras, par
   processus séparé**. La licence de chaque dépendance ajoutée est écrite dans le
   message de commit qui l'ajoute.
3. **Le noyau tourne sans QGIS.** Il se lance, s'exécute et se teste seul : c'est
   ce qui fonde le principe précédent et ce qui rend la V2 autonome possible.
   Toute dépendance du noyau envers QGIS est un bug.
4. **L'utilisateur apporte ses données, et ce dépôt reste civil.** Cassini ne
   télécharge rien et ne modifie jamais le fichier d'origine. Ce dépôt public ne
   contient aucune fonction spécialement conçue pour l'usage militaire — voir
   [Licence](#licence-et-noyau-civil).

Les principes complets, dont la règle de travail en salle blanche, sont dans
[`CLAUDE.md`](CLAUDE.md) §2, qui est la source de vérité du projet.

## Comment c'est construit

### L'architecture en couches

Les cases vertes sont écrites et testées ; la coquille de la V2, conservée, est
hors du chemin du MVP.

```mermaid
flowchart TB
    subgraph qgis["QGIS — GPL — fourni, pas écrit ici"]
        panneau["Panneau Cassini — extension Python — fait"]
        moteurQgis["Carte, fichiers, projet, analyses vectorielles, export PDF"]
    end

    canal["Canal local — JSON-RPC 2.0 sur l'entrée et la sortie standard, dans les deux sens, aucun réseau — fait"]

    subgraph noyau["Noyau Cassini — Rust, Apache-2.0 — tourne sans QGIS"]
        binaire["Binaire cassini, sous cli/ — run, replay, serve, plan, mcp faits"]
        registre["Registre de commandes, qui tient la boucle d'exécution — fait"]
        binaire --> registre
        terrain["Terrain et géométrie — MNT, coûts, coordonnées, GeoTIFF et GeoJSON sans GDAL — fait"]
        journal["Journal rejouable, écrit à côté du projet QGIS — fait"]
        ia["Couche IA — modèle local llama.cpp, lié au processus — fait"]
        registre --> terrain
        registre --> journal
        binaire --> ia
        ia --> registre
    end

    v2["Coquille Tauri et carte MapLibre, sous app/ — reporté en V2, conservé"]

    panneau <--> canal
    canal <--> binaire
    panneau -->|"délégation déclarée"| moteurQgis
    registre -.->|"servira en V2"| v2

    classDef fait fill:#dcf2e3,stroke:#2f7a46,color:#13331f
    classDef fourni fill:#e7eefb,stroke:#45659c,color:#1a2740
    classDef reporte fill:#f6eede,stroke:#9c7a45,color:#3d2f1a
    class panneau,canal,binaire,registre,journal,ia,terrain fait
    class moteurQgis fourni
    class v2 reporte
```

Trois conséquences de ce découpage :

- **Aucune logique métier dans l'extension.** L'extension QGIS, la ligne de
  commande, l'agent local et le serveur MCP appellent *les mêmes* commandes
  du registre.
- **Chaque commande déclare son exécutant.** Soit une implémentation en Rust dans
  le noyau, soit une délégation déclarée à QGIS — jamais implicite, jamais les
  deux.
- **Indépendance vis-à-vis des fournisseurs d'IA.** Le code ne dépend d'aucune IA
  en particulier : IA locale par défaut, IA externes par MCP.

Le canal local (AD-2, AD-3) : l'extension lance le binaire du noyau comme
processus enfant et dialogue avec lui par son entrée et sa sortie standard, en
JSON-RPC 2.0, un message par ligne, dans les deux sens. Aucune socket, aucun port,
aucun fichier d'échange. Ce qui porte l'argument de licence du principe 2, c'est
la **nature** de ce qui circule : des identifiants, des chemins relatifs, des
paramètres à schéma JSON et des résultats typés — jamais une couche entière ni une
structure interne (AD-8, AD-16). Le contrat des messages est dans
[`docs/canal-local.md`](docs/canal-local.md).

### Le trajet d'une demande en langage naturel

Le panneau demande, le noyau décide et dicte une étape à la fois (AD-5). La branche
« commande portée par le noyau » sert au terrain et à la géométrie. Quand elle écrit
un fichier — un raster, ou un GeoJSON de points ou de ligne —, le noyau dicte
aussitôt à QGIS l'étape qui l'ouvre dans le projet.

```mermaid
sequenceDiagram
    autonumber
    actor Humain
    participant Panneau as Panneau Cassini dans QGIS
    participant Agent as Agent IA local du noyau
    participant Registre as Registre de commandes du noyau
    participant QGIS as QGIS pour les couches et la mise en page
    participant Journal as Journal posé à côté du projet QGIS

    Humain->>Panneau: une demande en français
    Panneau->>Agent: la demande, par le canal local
    Agent->>Registre: lecture des commandes disponibles et de l'inventaire du projet
    Registre-->>Agent: identifiants, descriptions, paramètres typés
    Agent-->>Panneau: un plan ordonné, ou une question
    Panneau-->>Humain: le plan affiché étape par étape
    Humain->>Panneau: validation, ou arrêt
    Panneau->>Registre: le plan validé
    loop le noyau dicte chaque étape, une à la fois
        alt commande portée par le noyau
            Registre->>Registre: calcul en Rust, raster ou GeoJSON écrit
            Registre->>Panneau: ordre d'ouvrir le fichier produit
            Panneau->>QGIS: couche ajoutée au projet
        else commande déléguée à QGIS
            Registre->>Panneau: ordre de l'étape, paramètres typés
            Panneau->>QGIS: exécution par le cadre de traitement
            QGIS-->>Panneau: couche ajoutée, ou mise en page produite
            Panneau-->>Registre: résultat typé de l'étape
        end
        Registre->>Journal: étape journalisée
        Registre->>Panneau: avancement affiché
    end
    Panneau-->>Humain: projet QGIS à jour, et journal rejouable
```

L'agent ne fait rien d'autre que **choisir des commandes du registre** : il n'a
pas d'accès direct à QGIS ni au disque, et son plan est validé contre les schémas
des commandes avant d'être montré ; rien de ce qu'il produit n'est exécuté comme
du code (AD-23). Un plan invalide est refusé, jamais réparé en silence.
L'extension n'enchaîne jamais deux commandes d'elle-même. Toute exécution est
journalisée par le noyau seul (AD-11), pour que la même demande puisse être
rejouée à l'identique.

### Le cycle de vie d'un projet QGIS

Le projet appartient à QGIS : Cassini n'a pas de format de projet et n'en aura
pas au MVP. Ce qu'il écrit, c'est son journal, posé à côté du fichier de projet,
et les couches produites par les analyses — couches de QGIS, rasters et GeoJSON
du noyau —, dans un dossier `<projet>.cassini/`.

```mermaid
stateDiagram-v2
    direction TB
    state "QGIS ouvert, aucun projet" as aucun
    state "Projet QGIS et couches de l'utilisateur" as projet
    state "Plan proposé par l'agent" as plan
    state "Commande en cours d'exécution" as execution
    state "Mise en page de QGIS" as miseEnPage

    [*] --> aucun
    aucun --> projet : créer ou ouvrir un projet QGIS
    projet --> projet : ouvrir des fichiers, ordonner, symboliser, cadrer
    projet --> plan : une demande en français, au panneau Cassini
    plan --> projet : plan refusé, rien n'est exécuté
    plan --> execution : plan validé, étape par étape
    execution --> projet : résultat ajouté, étape écrite au journal
    projet --> miseEnPage : préparer titre, légende, échelle, nord, sources
    miseEnPage --> projet : revenir à la carte
    miseEnPage --> [*] : exporter en PDF, enregistrer, fermer
    projet --> [*] : enregistrer et fermer

    note right of execution
        Le journal Cassini vit à côté du fichier de projet QGIS.
        Il n'est pas le projet. Il garde la suite des commandes et
        leurs paramètres, pour rejouer une analyse après réouverture.
    end note
```

## Les dossiers du dépôt

| Dossier | À quoi il sert |
|---|---|
| `core/` | Le noyau, crate `cassini-core` : registre de commandes, commandes, journal, canal local, validation des plans, terrain et géodésie, lecture et écriture des GeoTIFF, écriture des GeoJSON. Les tests unitaires sont à côté du code qu'ils couvrent. |
| `ai/` | La couche IA, crate `cassini-ai` : le modèle local chargé dans le processus du noyau par llama.cpp, la composition du plan, le choix des fils de calcul. `cassini-core` n'en dépend jamais (AD-13). |
| `cli/` | Le binaire unique `cassini`. Il vit à part parce qu'il a besoin du noyau **et** de la couche IA. Ses tests d'intégration lancent le vrai binaire. |
| `qgis-plugin/` | L'extension QGIS en Python, sous GPL : le panneau, l'exécutant des gestes QGIS, et ses tests. Installation et recettes manuelles dans [`qgis-plugin/README.md`](qgis-plugin/README.md). |
| `training/` | La chaîne d'entraînement du modèle : générateur d'exemples tiré du registre (Rust), entraînement et notation (Python). Ne tourne jamais en CI, sauf ses tests. |
| `app/` | **Reporté en V2, conservé exprès.** La coquille Tauri et la carte MapLibre, avec ses scripts de recette et le générateur de fond de carte. |
| `docs/` | La documentation. Index dans [`docs/README.md`](docs/README.md). |
| `fixtures/` | Les jeux de test, légers et fabriqués par le dépôt lui-même — aucune donnée géographique tierce. Voir [`fixtures/README.md`](fixtures/README.md). |
| `tests/` | Réservé aux tests qui traversent plusieurs briques ; les suites de bout en bout vivent pour l'instant dans `cli/tests/` et `qgis-plugin/tests/`. |
| `_bmad-output/` | La planification : spécification et catalogue des commandes, contrat d'architecture, changements de cap, epics, tickets, et le plan écrit de chaque entrée de travail. |
| `_bmad/` | L'outillage de planification qui produit ce qui précède. |

À la racine : [`CLAUDE.md`](CLAUDE.md), la source de vérité du projet ;
`Cargo.toml`, le workspace (`core`, `ai`, `cli`, `training`, `app/src-tauri`) ;
`deny.toml`, la garde des licences et des dépendances ; `LICENSE`.

Les essais à la main se font sur un jeu de données réelles (Villeneuve-le-Roi,
Natural Earth, modèle d'élévation) qui vit **hors du dépôt**, dans
`../donnees-test-cassini/`, et n'est jamais commité ni redistribué.

## Démarrage rapide

**Prérequis** : Rust stable avec `cargo` (la version est figée par
`rust-toolchain.toml`), CMake et libclang pour compiler llama.cpp. Sous Windows,
la marche à suivre est dans
[`docs/modele-local-sous-windows.md`](docs/modele-local-sous-windows.md) ; la
coquille V2 demande en plus Node.js et les prérequis de Tauri v2, détaillés dans
[`docs/compiler-sous-windows.md`](docs/compiler-sous-windows.md).

Sous WSL (ou un Linux nu), la construction de llama.cpp demande libclang et les
en-têtes de gcc, que `bindgen` ne trouve pas seul :

```bash
sudo apt install build-essential cmake clang libclang-dev
export LIBCLANG_PATH=/usr/lib/llvm-18/lib        # le dossier qui contient libclang.so
export BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include
```

Les numéros de version (`llvm-18`, gcc `13`) sont ceux du poste : `ls /usr/lib/llvm-*`
et `ls /usr/lib/gcc/x86_64-linux-gnu` les donnent. `--workspace` demande en plus
`libdbus-1-dev` et les bibliothèques GTK de la coquille Tauri (`app`) ; sans elles,
s'en tenir à `-p cassini-core -p cassini-ai -p cassini`.

Le noyau se teste seul, sans QGIS et sans modèle — c'est le principe 3 en
pratique :

```bash
cargo test -p cassini-core -p cassini-ai -p cassini
```

Pour essayer Cassini dans QGIS :

1. Construire le noyau : `cargo build --release -p cassini` donne un noyau processeur
   d'un seul fichier. Le paquet livré, qui se sert aussi de la carte graphique, se
   construit avec `--features gpu` et le SDK Vulkan : c'est un dossier, `cassini.exe` et
   ses bibliothèques — voir [`docs/paquet-du-noyau.md`](docs/paquet-du-noyau.md).
2. Poser le noyau dans `%LOCALAPPDATA%\Cassini\bin` (Windows) ou
   `~/.local/share/cassini/bin` (Linux), et un modèle `.gguf` dans le dossier
   `models/` voisin — voir [`docs/dossier-des-modeles.md`](docs/dossier-des-modeles.md).
3. Copier `qgis-plugin/cassini_qgis` dans le dossier des extensions de QGIS, puis
   activer *Cassini* — détail dans [`qgis-plugin/README.md`](qgis-plugin/README.md).
4. Ouvrir un projet QGIS avec ses couches, et écrire une demande dans le panneau.

Rien n'est téléchargé à aucun moment : le modèle se copie à la main, comme le
noyau.

### Brancher un client MCP

Une IA externe, si on en veut une, pilote Cassini par `cassini mcp`, un serveur MCP
sur l'entrée et la sortie standard — aucun port, aucune socket. C'est l'application de
l'IA qui lance le binaire ; Cassini ne contient aucune ligne propre à un fournisseur.
Toutes les fiches ci-dessous lancent la même commande : le chemin complet du binaire
(paquet Windows : `C:/Users/<vous>/AppData/Local/Cassini/bin/cassini.exe`), suivi de
`mcp`. On peut ajouter `--project C:/chemin/vers/projet.qgz` : il ne sert qu'en mode
autonome, pour journaliser chaque appel à côté du projet, comme `cassini run`.

Les outils portent le nom de la commande, le point remplacé par un souligné :
`analysis_buffer` pour `analysis.buffer`, `system_about` pour `system.about`. C'est le même
nom pour tous les clients, certains refusant le point. Le journal et le rejeu gardent le nom
à point de la commande ; un client qui appelle encore l'ancien nom est servi.

**Claude Desktop** ([documentation](https://modelcontextprotocol.io/docs/develop/connect-local-servers)).
Réglages › Développeur › *Modifier la configuration* ouvre `claude_desktop_config.json`
(`%APPDATA%\Claude\` ; l'application du Microsoft Store le range sous
`%LOCALAPPDATA%\Packages\Claude_<suffixe>\LocalCache\Roaming\Claude\`). Y mettre, puis
quitter complètement Claude Desktop et le rouvrir :

```json
{
  "mcpServers": {
    "cassini": {
      "command": "C:/Users/<vous>/AppData/Local/Cassini/bin/cassini.exe",
      "args": ["mcp"]
    }
  }
}
```

**ChatGPT pour ordinateur et Codex CLI** ([documentation](https://learn.chatgpt.com/docs/extend/mcp)).
Les deux lisent la même configuration, `%USERPROFILE%\.codex\config.toml` : ce qui est
branché dans l'un l'est dans l'autre. Dans l'application : Réglages › *MCP servers* ›
*Add server*, type **STDIO**, commande et argument ci-dessus, puis *Restart*. En ligne de
commande : `codex mcp add cassini -- C:/Users/<vous>/AppData/Local/Cassini/bin/cassini.exe mcp`.
Ou directement dans le fichier :

```toml
[mcp_servers.cassini]
command = "C:/Users/<vous>/AppData/Local/Cassini/bin/cassini.exe"
args = ["mcp"]
```

Ajouter `default_tools_approval_mode = "approve"` sous `[mcp_servers.cassini]` évite une
confirmation à chaque appel, pour ce serveur seulement. Codex ne pense pas toujours de
lui-même à Cassini : commencer la demande par « Dans mon projet QGIS ouvert, avec les
outils Cassini : ». L'application web de ChatGPT ne lance pas de programme sur le poste :
elle ne peut pas servir.

**Gemini CLI** ([documentation](https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md)).
`gemini mcp add -s user cassini C:/Users/<vous>/AppData/Local/Cassini/bin/cassini.exe mcp`,
ou à la main dans `%USERPROFILE%\.gemini\settings.json`, la même entrée `mcpServers` que
pour Claude Desktop. Ajouter `"trust": true` à cette entrée évite une confirmation à chaque
appel ; les autres outils de Gemini restent soumis à confirmation. Constaté lors de la
recette du 31 octobre 2026 (pas dans la documentation de Google) : la 0.27 appelait des
modèles retirés, d'où une version récente (`npm install -g @google/gemini-cli@latest`) ;
la connexion par compte personnel gratuit était refusée (« no longer supported for Gemini
Code Assist for individuals »), et une clé d'API de Google AI Studio, posée dans
`GEMINI_API_KEY`, a servi à la place.
Les avertissements « unknown format "uint64" ignored » au démarrage sont sans effet.

**Antigravity CLI** (`agy`, successeur de Gemini CLI). Constaté lors de la recette du
31 octobre 2026 avec `agy` 1.4.12 (pas dans une documentation de Google) : le serveur se
déclare dans `%USERPROFILE%\.gemini\config\mcp_config.json`, la même entrée `mcpServers`
que pour Claude Desktop, et ses outils s'autorisent dans
`%USERPROFILE%\.gemini\antigravity-cli\settings.json` :

```json
{
  "permissions": {
    "allow": ["mcp(cassini/*)"]
  }
}
```

Antigravity préfixe chaque outil par `mcp_cassini_` et refuse tout nom hors
`^[a-zA-Z0-9_-]{1,64}$` ; les noms sans point de Cassini y passent. Les outils de Cassini sont bien chargés, mais en `agy -p` le
modèle commence par une commande shell (`RunCommand`), refusée sans demande dans ce mode, et
le tour s'arrête sans aucun appel à Cassini. Préférer une session interactive, ou, en
`agy -p`, interdire les commandes système pour la seule session :
`agy --settings <fichier.json> -p "<demande>"`, le fichier contenant
`{"permissions": {"allow": ["mcp(cassini/*)"], "deny": ["command(*)"]}}`. Le modèle se
rabat alors sur les outils de Cassini.

**Claude Code** ([documentation](https://code.claude.com/docs/en/mcp)) : `claude mcp add cassini -- C:/Users/<vous>/AppData/Local/Cassini/bin/cassini.exe mcp`.

**La recette, avec n'importe lequel de ces clients :**

1. Ouvrir le projet dans QGIS, puis dans les Réglages du panneau Cassini, cocher
   « Autoriser une IA externe » : le bandeau « Une IA externe peut agir sur ce projet »
   apparaît.
2. Lancer le client (ou le redémarrer s'il était ouvert) et vérifier qu'il voit les
   outils de Cassini — dont `project_inventory`, `analysis_buffer`, `layout_export_pdf`.
3. Lui donner la phrase de démo. Chaque appel défile au journal du panneau, marqué
   « IA externe », et les couches produites apparaissent sur la carte.
4. Couper l'IA externe par **Couper** dans le bandeau.

Le panneau n'offre pas encore de bouton de rejeu. La sonde
`qgis-plugin/tests/probe_external.py` joue la même recette sans écran, sous le Python de
QGIS, pendant qu'un client en ligne de commande donne la phrase. Elle travaille sur une
copie du projet, puis rouvre cette copie et rejoue le journal, qui doit rendre
`identical: true` (mode d'emploi en tête du fichier).

Le serveur a **deux modes**, choisis à chaque demande du client (liste des outils,
appel d'un outil) :

- **Relais**, quand une session QGIS a allumé le canal d'IA externe dans le panneau :
  `cassini mcp` s'y branche et pilote cette session. Les outils sont alors les
  commandes disponibles de la session, **commandes exécutées par QGIS comprises**
  (`layer_describe`, `analysis_buffer`…), avec leurs schémas ; chaque appel est
  exécuté et journalisé par la session, avec l'origine `external`, sous les yeux de
  l'utilisateur. Le relais ne décide rien et n'exécute rien lui-même ; `--project`
  n'est pas transmis à la session, et aucun chemin absolu ne passe sur le canal,
  dans aucun sens.
- **Autonome** sinon : seules les commandes du noyau sont exposées (`system_about`,
  `model_list`), sans QGIS.

Tout changement de mode ou de catalogue est annoncé au client par
`notifications/tools/list_changed`, qu'il suit en relisant la liste des outils. Un
appel commencé en relais ne finit jamais en autonome : si l'utilisateur coupe le
canal ou ferme QGIS pendant l'appel, le client reçoit l'erreur
`external.session_lost`, et la demande suivante est servie en autonome. Une session
qui parle une autre version du protocole refuse chaque appel par
`session.protocol_mismatch`, dont le message nomme le binaire à mettre à jour. Le
serveur ne compose aucun plan : l'IA externe appelle les commandes une à une. Il
reste réactif pendant un appel long : `ping` y répond. En relais, l'annulation du
client est transmise à la session ; en autonome, la réponse de l'appel annulé est
abandonnée, mais la commande va jusqu'au bout (et, avec `--project`, est
journalisée).

En mode autonome avec `--project`, le serveur ne prend le verrou du journal du projet
**que le temps d'un appel**, comme `cassini run` : il n'empêche jamais QGIS de
s'ouvrir en écriture sur le même projet. Si QGIS tient le verrou, un appel autonome
répond `journal.read_only`.

## La recette

Le workflow [`.github/workflows/recette.yml`](.github/workflows/recette.yml) la
lance à chaque poussée ; chaque contrôle se lance aussi à la main. Si l'un échoue,
le run passe au rouge.

| Contrôle | Ce qu'il vérifie |
|---|---|
| `cargo deny check` | Licences permissives seulement dans le noyau ; aucun client HTTP, aucune couche asynchrone, aucune couche TLS (AD-14). |
| `cargo fmt --all --check` | La mise en forme du Rust. |
| `cargo test -p cassini-core -p cassini-ai -p cassini -p cassini-training --locked` | Les tests du noyau, de la couche IA, du binaire et du générateur d'exemples, dont le refus de toute source réseau, la parité entre `run` et `serve`, le rejeu du journal, la suite de bout en bout et la confrontation du catalogue des commandes au registre. |
| `cargo run --release -p cassini -- run system.about` | Le binaire de release connaît bien les commandes du noyau. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Zéro avertissement, coquille Tauri comprise. |
| `cargo test --release -p cassini --test hors_ligne`, sous Windows | Ni le binaire ni les bibliothèques posées à côté de lui n'importent une bibliothèque réseau. Avec `--features gpu` (à la main, la CI n'a pas le SDK Vulkan), il vérifie aussi le contenu du paquet. |
| `python -I qgis-plugin/tests/check_extension.py` puis les tests de `qgis-plugin/tests`, sous Linux et Windows | L'extension n'importe ni module réseau ni PyQt direct, tient sous la taille permise, et ses tests passent sans QGIS. |
| les tests de `training/entrainement/tests` | La chaîne d'entraînement : lecture du jeu, notation, gardes — sans GPU. |
| `npm run check`, `npm run build` et `npm run verify:dist`, depuis `app` | La coquille V2 : typage strict, garde hors ligne de l'interface, fond de carte de recette. |

Sous WSL, `--workspace` demande `libdbus` (dépendance de la coquille Tauri) :
lancer `cargo clippy -p cassini-core -p cassini-ai -p cassini --all-targets --locked -- -D warnings`.

Deux familles de contrôles ne tournent pas en CI, parce qu'elles demandent ce que
la CI n'a pas :

- **Les sondes sous le vrai QGIS** (`qgis-plugin/tests/probe_*.py`), lancées par
  `python-qgis-ltr.bat` : elles enchaînent les commandes dans un vrai projet, le
  ferment, le rouvrent et rejouent le journal. `probe_style.py` met en forme des
  copies des projets de Villeneuve-le-Roi et d'Annecy (`map.topographic_style`,
  `map.graduated_style`), exporte le PDF et rejoue. Elle passe par les deux
  commandes que le noyau dicte à l'extension, `layer.set_style` et
  `layer.arrange`, dont les refus sont `qgis.layer_missing`, `style.invalid`,
  `style.field_missing`, `style.geometry_mismatch` et `arrange.invalid`. Elle
  demande `../donnees-test-cassini/` avec les deux projets de test, Villeneuve-le-Roi
  et Annecy ; elle se lance avec `QT_QPA_PLATFORM=windows`, et ses PDF se
  regardent rendus en image.
- **Les mesures du vrai modèle** (`ai/tests/*_reel.rs`) : justesse des plans
  (`plan_reel`), des questions (`question_reel`), score sur le jeu tenu à l'écart
  (`ecart_reel`) et durée par phrase (`duree_reel`). Elles demandent un modèle
  posé par `CASSINI_TEST_MODEL`.

Un garde-fou qu'on n'a jamais vu refuser quelque chose ne garde rien : d'où les
contrôles qui échouent exprès.

## Licence et noyau civil

Trois régimes, et celui de l'extension n'est pas un choix :

- **Le noyau, la couche IA, le binaire et la coquille de bureau** — `core/`,
  `ai/`, `cli/`, `training/`, `app/` : **Apache-2.0**, voir [`LICENSE`](LICENSE).
  L'usage commercial et institutionnel est permis, et aucune dépendance GPL ou
  LGPL n'y est admise, même par liaison statique.
- **L'extension QGIS** — `qgis-plugin/` : **GPL**, comme QGIS. C'est une
  conséquence de l'hôte, pas une préférence. Elle vit séparée du noyau, qui ne lie
  jamais QGIS et reste utilisable sans lui.
- **Le module défense**, s'il voit le jour : **dépôt privé distinct**, bâti sur le
  noyau et non sur l'extension — précisément parce que l'extension est en GPL. Sa
  licence et ses conditions de commercialisation dépendent de la réponse à une
  demande de classement.

Ce dépôt est le **noyau civil** du projet, généraliste. Il ne contient, et ne
contiendra, **aucune fonction spécialement conçue pour l'usage militaire** : la
charnière du contrôle à l'exportation des équipements militaires de l'Union
européenne est précisément l'expression *specially designed for military use*, et
l'article L.2332-1 du Code de la défense soumet le commerce de matériels de guerre
à autorisation. Les fonctions de cette nature, si elles voient le jour, vivront
dans le dépôt privé cité ci-dessus, et leur diffusion est subordonnée aux
autorisations que la réglementation impose.

Sont également hors du périmètre de ce dépôt : le téléchargement ou la
redistribution de données, les formats de projet propriétaires, et les formats et
symbologies de défense.
