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

La cible, en une phrase — et c'est bien une cible, pas l'état actuel :

> « Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à
> Villeneuve-le-Roi et sors-moi la carte en PDF. »

**Cassini ne refait pas QGIS.** QGIS est libre, hors ligne, organisé en projets
et mûr depuis vingt ans : reconstruire sa carte, son ouverture de fichiers, ses
analyses et son export PDF serait perdre des années pour arriver à moins bien.
Ce qui n'existe nulle part, c'est un SIG de bureau hors ligne piloté par une IA
locale. Au MVP, Cassini est donc une **extension QGIS posée sur un noyau
indépendant** : QGIS exécute la carte, l'ouverture des fichiers, le projet, les
analyses vectorielles et l'export PDF ; le noyau porte le registre de commandes,
l'agent, le journal rejouable et un moteur de terrain en licence permissive.

Ce découpage date du 2026-10-14 et remplace un plan où Cassini était une
application de bureau complète ; le raisonnement est écrit en entier dans
[la proposition de changement](_bmad-output/initiative-cassini-mvp/change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md).
L'application autonome n'est pas abandonnée, elle est **reportée en V2**, et le
code déjà écrit pour elle reste au dépôt.

La frontière entre le noyau et l'extension est fixée par
[le contrat d'architecture](_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md),
en vingt-trois décisions numérotées de AD-1 à AD-23 ; ce README en résume ce qui
change la forme du produit.

Le projet en est à l'**étape 1 sur 7** : le socle technique et le verrou hors
ligne. La section suivante dit exactement ce que cela couvre.

## Ce qui marche aujourd'hui

Rien de l'extension QGIS n'existe encore — le dossier `qgis-plugin/` n'est pas
créé. Ce qui est écrit, testé et conservé, c'est le socle, plus la coquille de
bureau désormais reportée en V2 :

- Un workspace Rust : `cassini-core`, le noyau ; `cassini`, le binaire de la
  CLI ; `cassini-app`, la coquille de bureau en Tauri v2.
- Le **registre de commandes** : chaque commande s'inscrit seule, sans toucher un
  fichier partagé, et le registre contrôle son contrat (id, descriptions
  française et anglaise, schémas, exécutant, test). Une seule commande de
  production aujourd'hui, `system.about`.
- Le **journal rejouable**, écrit à côté du projet en ajout seul sous verrou
  exclusif, et le binaire `cassini` : `run` exécute une commande, `replay`
  rejoue le journal, `serve` ouvre le canal local en JSON-RPC sur l'entrée et la
  sortie standard. Un même appel donne le même résultat par `run` et par
  `serve`, ce que contrôle un test sur le vrai binaire. `mcp` n'existe pas
  encore.
- Une fenêtre qui s'ouvre sous Windows et affiche une carte MapLibre GL JS,
  navigable au zoom et au déplacement.
- Un fond de carte au format PMTiles **lu sur le disque** : le noyau en sert les
  octets par l'IPC de Tauri, l'interface ne connaît jamais le chemin du fichier,
  et aucune requête réseau n'intervient à aucun moment.
- Deux commandes exposées par la coquille, et deux seulement : `engine_version`
  et `read_basemap`. L'interface n'appelle que la seconde ; la coquille ne passe
  pas encore par le registre de commandes.
- Le verrou hors ligne vit dans le noyau — `BasemapSource` refuse à la
  construction toute source qui désigne le réseau — et il est doublé d'une garde
  qui relit l'interface fichier par fichier.
- Un générateur **déterministe** de l'archive PMTiles de recette, et deux
  vérificateurs qui la relisent : conformité de l'archive, puis chemin complet de
  chargement de la carte.
- Une recette qui tient en cinq commandes : 110 tests du noyau et de la CLI, `cargo fmt`,
  `cargo clippy`, et le contrôle de l'interface — lancée par GitHub Actions à
  chaque poussée.

La carte et la coquille sont **hors du chemin critique du MVP** depuis le
2026-10-14, mais elles marchent et elles restent : c'est la fondation de la V2
autonome. Tout le reste de ce README — agent IA, serveur MCP, extension, terrain —
décrit une **cible**, pas une fonction disponible.

## Ce qui est prévu

Les sept étapes du projet, telles que les fixe `CLAUDE.md` §7. Aucune étape ne
commence avant que la précédente ne soit testée.

| Étape | Ce qu'elle apporte | État |
|---|---|---|
| 1 | Socle technique, verrou hors ligne, recette automatique | **faite pour l'essentiel** ; la recette tourne à chaque poussée |
| 2 | Épreuve du modèle local — un modèle quantifié sait-il transformer une phrase française en suite de commandes typées correcte, sur un registre factice | **faite** : verdict « oui sous conditions », dans `_bmad-output/initiative-cassini-mvp/epic-epreuve-du-modele-local/` |
| 3 | Registre de commandes et journal rejouable — l'épine dorsale | **en cours** : contrat, journal, `run`, `replay` et `serve` faits ; reste la suite de bout en bout |
| 4 | Façade QGIS — extension, canal local, commandes exécutées par QGIS | à venir |
| 5 | Agent IA local (llama.cpp) et serveur MCP | à venir |
| 6 | Analyse de terrain dans le noyau — champ de vision, ligne de vue, pente | à venir |
| 7 | Scénario de démo, puis publication : dépôt d'extensions QGIS, installeurs, vidéo | le présent README est pris par avance, le dépôt étant déjà public |

Formats que QGIS ouvrira pour le compte du registre : GeoPackage, Shapefile,
GeoJSON, CSV à coordonnées, GeoParquet. Systèmes de coordonnées : Lambert-93
(EPSG:2154) et WGS84 au minimum, avec reprojection quand la couche diffère du
projet — reprojection exécutée par QGIS, jamais par le noyau (AD-19). Aucune
parité avec QGIS n'est visée ni revendiquée.

Une seule extension servira QGIS 3.44 LTR et la série 4.x (AD-18). Cassini se
distribuera en deux artefacts : l'extension, légère et sans binaire, par le
dépôt d'extensions QGIS ; le noyau et, en option, le modèle d'IA, par un paquet
Cassini installé à part. Une clé USB portant les deux suffira sur un poste qui
n'a jamais vu le réseau (AD-17).

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

**Schéma d'une cible, pour l'essentiel.** QGIS existe et fonctionne, mais rien de
ce qui le relie à Cassini n'est écrit. Du noyau, une seule case est faite — la
lecture du fond de carte. La case `app/`, elle, est livrée et fonctionne, mais
elle est reportée en V2. Les cases marquées « prévu » n'existent pas encore dans
le dépôt, et `qgis-plugin/` pas davantage.

```mermaid
flowchart TB
    subgraph qgis["QGIS — GPL — fourni, pas écrit ici"]
        panneau["Panneau Cassini — extension Python — prévu"]
        moteurQgis["Carte, fichiers, projet, analyses vectorielles, export PDF"]
    end

    canal["Canal local — JSON-RPC 2.0 sur l'entrée et la sortie standard, dans les deux sens, aucun réseau — côté noyau fait"]

    subgraph noyau["Noyau Cassini — Rust, Apache-2.0 — tourne sans QGIS"]
        binaire["Binaire cassini, sous cli/ — run, replay, serve faits ; mcp prévu"]
        registre["Registre de commandes, qui tient la boucle d'exécution — fait"]
        binaire --> registre
        terrain["Terrain sur MNT — champ de vision, ligne de vue, pente — prévu"]
        journal["Journal rejouable, écrit à côté du projet QGIS — fait"]
        ia["Couche IA — modèle local llama.cpp, lié au processus — prévu"]
        fond["Lecture du fond de carte, refus du réseau — fait"]
        registre --> terrain
        registre --> journal
        binaire --> ia
        ia --> registre
    end

    v2["Coquille Tauri et carte MapLibre, sous app/ — reporté en V2, conservé"]

    panneau <--> canal
    canal <--> binaire
    panneau -->|"délégation déclarée"| moteurQgis
    fond -.->|"sert aujourd'hui"| v2
    registre -.->|"servira en V2"| v2

    classDef fait fill:#dcf2e3,stroke:#2f7a46,color:#13331f
    classDef prevu fill:#f2f2f2,stroke:#8d8d8d,color:#333333
    classDef fourni fill:#e7eefb,stroke:#45659c,color:#1a2740
    classDef reporte fill:#f6eede,stroke:#9c7a45,color:#3d2f1a
    class fond,canal,binaire,registre,journal fait
    class panneau,terrain,ia prevu
    class moteurQgis fourni
    class v2 reporte
```

Trois conséquences de ce découpage, qui valent dès maintenant :

- **Aucune logique métier dans l'extension.** L'extension QGIS, la ligne de
  commande, l'agent local et le serveur MCP appelleront *les mêmes* commandes du
  registre.
- **Chaque commande déclare son exécutant.** Soit une implémentation en Rust dans
  le noyau, soit une délégation déclarée à QGIS — jamais implicite, jamais les
  deux.
- **Indépendance vis-à-vis des fournisseurs d'IA.** Le code ne dépendra d'aucune
  IA en particulier : IA locale par défaut, IA externes par MCP.

Le canal local est tranché (AD-2, AD-3) : l'extension lance le binaire du noyau
comme processus enfant et dialogue avec lui par son entrée et sa sortie
standard, en JSON-RPC 2.0, un message par ligne, dans les deux sens. Aucune
socket, aucun port, aucun fichier d'échange. Ce qui porte l'argument de licence
du principe 2, c'est la **nature** de ce qui circule : des identifiants, des
chemins relatifs, des paramètres à schéma JSON et des résultats typés — jamais
une couche entière ni une structure interne (AD-8, AD-16).

Le noyau lit lui-même les modèles numériques d'élévation dont le terrain a
besoin ; QGIS ne lui passe qu'un chemin de fichier et une emprise (AD-20).

### Le trajet d'une demande en langage naturel

**Schéma d'une cible.** Ni l'extension, ni le registre, ni l'agent ne sont
écrits : le registre arrive à l'étape 3, la façade QGIS à l'étape 4, l'agent à
l'étape 5. Ce qui est déjà fixé, c'est la forme de l'échange — le panneau
demande, le noyau décide et dicte une étape à la fois (AD-5).

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
    Agent->>Registre: lecture des commandes disponibles
    Registre-->>Agent: identifiants, descriptions, paramètres typés
    Agent-->>Panneau: un plan ordonné de commandes et de paramètres
    Panneau-->>Humain: le plan affiché étape par étape
    Humain->>Panneau: validation, ou correction d'une étape
    Panneau->>Registre: le plan validé
    loop le noyau dicte chaque étape, une à la fois
        alt commande portée par le noyau
            Registre->>Registre: calcul de terrain, en Rust
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
du code (AD-23). L'extension n'enchaîne jamais deux commandes d'elle-même. Toute
exécution est journalisée par le noyau seul (AD-11), pour que la même demande
puisse être rejouée à l'identique.

### Le cycle de vie d'un projet QGIS

**Schéma d'une cible.** Le projet appartient à QGIS : Cassini n'a pas de format
de projet et n'en aura pas au MVP. Ce qu'il écrit, c'est son journal, posé à côté
du fichier de projet.

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

Le projet, ses couches, leur ordre, leur symbologie, l'étendue et la mise en page
sont enregistrés par QGIS, dans le format de QGIS. Le journal Cassini ne double
pas ce travail : il garde la trace des commandes exécutées, et c'est lui qui rend
une analyse rejouable.

## Les dossiers du dépôt

| Dossier | À quoi il sert |
|---|---|
| `core/` | Le noyau, crate `cassini-core`, sans aucune dépendance. Toute la logique métier y vivra — registre, journal et terrain sont encore à écrire — et les tests unitaires sont à côté du code qu'ils couvrent. |
| `app/` | **Reporté en V2, conservé exprès.** La coquille de bureau et la carte : `app/src` la webview — TypeScript strict, MapLibre ; `app/src-tauri` la coquille Rust qui ouvre la fenêtre et relaie les appels ; `app/scripts` les scripts de recette et le générateur de fond de carte. Hors du chemin critique du MVP depuis le 2026-10-14, mais testé et gardé comme fondation de l'application autonome. |
| `docs/` | La documentation. Index dans [`docs/README.md`](docs/README.md), dont la marche à suivre pour compiler sous Windows. |
| `fixtures/` | Les jeux de test, légers et fabriqués par le dépôt lui-même — aucune donnée géographique tierce. Voir [`fixtures/README.md`](fixtures/README.md). |
| `tests/` | Les tests qui traversent plusieurs briques, à venir avec le registre et l'extension. Ne contient encore que son [`README.md`](tests/README.md). |
| `_bmad-output/` | La planification : spécification, dossier de recherche, epics, propositions de changement, et le plan écrit de chaque entrée de travail. |
| `_bmad/` | L'outillage de planification qui produit ce qui précède. Ses scripts sont réinstallables, donc ignorés par git ; `skills-lock.json`, à la racine, en verrouille les versions. |

À la racine : [`CLAUDE.md`](CLAUDE.md), la source de vérité du projet ;
`Cargo.toml`, le workspace à deux membres (`core` et `app/src-tauri`) ; `LICENSE`.

Trois dossiers annoncés par `CLAUDE.md` §9 n'existent pas encore :

| Dossier | À quoi il servira |
|---|---|
| `cli/` | Le binaire unique `cassini`, crate du même nom, avec ses sous-commandes `serve` (le canal de l'extension), `mcp`, `run` et `replay`. Il vit à part parce qu'il a besoin du noyau **et** de la couche IA, alors que `cassini-core` ne dépend jamais de `cassini-ai` (AD-13, et la direction des dépendances du contrat). |
| `qgis-plugin/` | L'extension QGIS en Python, **pas encore créée**, qui arrivera à l'étape 4 et portera le panneau du MVP. |
| `ai/` | La couche IA, crate `cassini-ai` : le modèle local chargé dans le processus du noyau, jamais servi par HTTP (AD-15). Elle arrivera à l'étape 5. |

## Démarrage rapide

**Prérequis** : Rust stable avec `cargo`, Node.js avec `npm`, et les prérequis
système de Tauri v2. Sous Windows, tout est détaillé pas à pas dans
[`docs/compiler-sous-windows.md`](docs/compiler-sous-windows.md), preuve de
fonctionnement réseau coupé comprise.

Le noyau se teste seul, sans rien installer d'autre que Rust — c'est le principe
3 en pratique :

```bash
cargo test -p cassini-core
```

La coquille de bureau reportée en V2, depuis le dossier `app` :

```bash
npm install
npm run check        # typage, garde hors ligne, archive et protocole
npm run tauri dev    # ouvre la fenêtre Cassini avec son fond de carte
```

Deux précisions qui font gagner du temps :

- `npm run dev` ne lance que la webview Vite, **sans le noyau** : le fond de
  carte y reste indisponible, puisque ses octets viennent de l'IPC de Tauri. Pour
  voir l'application, c'est `npm run tauri dev`.
- `npm run fixtures` régénère l'archive PMTiles de `fixtures/`. La génération est
  déterministe : deux exécutions produisent le même fichier, octet pour octet.

Il n'y a encore rien à installer dans QGIS : l'extension arrive à l'étape 4.

## La recette

Cinq commandes, que le workflow
[`.github/workflows/recette.yml`](.github/workflows/recette.yml) lance à chaque
poussée, et qui se lancent aussi à la main. Si l'une échoue, le run passe au
rouge. Un emplacement y est réservé au contrôle du Python de l'extension QGIS,
qui arrivera avec elle.

| Commande | Ce qu'elle contrôle |
|---|---|
| `cargo test -p cassini-core -p cassini --locked` | Les 110 tests du noyau et de la CLI, dont le refus de toute source réseau, la parité entre `run` et `serve` et le rejeu du journal. |
| `cargo fmt --all --check` | La mise en forme du Rust. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Zéro avertissement, sur les deux crates et leurs tests. |
| `npm run check`, depuis `app` | Le typage strict, puis quatre vérifications décrites ci-dessous. |
| `npm run build` puis `npm run verify:dist`, depuis `app` | Chaque fichier que le paquet construit désigne existe dans `dist/`. |

Sous WSL, `--workspace` demande `libdbus` (dépendance de la coquille Tauri) : lancer
`cargo clippy -p cassini-core -p cassini --all-targets --locked -- -D warnings`.
La version de Rust est figée à 1.93.1 par `rust-toolchain.toml`.

Ce que `npm run check` enchaîne après `tsc` :

- `scripts/check-offline.mjs` — la **garde hors ligne** côté interface : elle
  échoue si la moindre adresse réseau apparaît dans `app/index.html`, dans le
  style `app/src/basemap.style.json`, ou dans les fichiers `.ts`, `.tsx`, `.js`,
  `.mjs`, `.css` et `.html` de `app/src` — une feuille de style est une porte de
  sortie aussi directe qu'un `fetch`. Seule exception, nommée : le schéma
  `pmtiles://` suivi de la clé locale attendue, qu'elle relit dans le code plutôt
  que de la recopier.
- `scripts/verify-basemap.mjs` — 30 contrôles sur l'archive PMTiles : en-tête,
  tuiles, emprise, zooms, déterminisme, et quatre contrôles qui mettent le
  validateur en échec exprès.
- `scripts/verify-protocol.mjs` — 24 contrôles sur le chemin de chargement de la
  carte, et la preuve qu'une clé non enregistrée partirait bien sur le réseau,
  c'est-à-dire ce que la garde précédente existe pour empêcher.
- `scripts/verify-ids.mjs` — chaque id de `app/index.html` est visé par `main.ts` ou
  `style.css`, et chaque id visé existe dans le HTML.

Un garde-fou qu'on n'a jamais vu refuser quelque chose ne garde rien : d'où les
contrôles qui échouent exprès.

## Licence et noyau civil

Trois régimes, et celui de l'extension n'est pas un choix :

- **Le noyau et la coquille de bureau** — `core/`, `app/` et, quand ils
  existeront, `cli/` et `ai/` : **Apache-2.0**, voir [`LICENSE`](LICENSE).
  L'usage commercial et institutionnel est permis, et aucune dépendance GPL ou
  LGPL n'y est admise, même par liaison statique.
- **L'extension QGIS** — `qgis-plugin/`, pas encore créée : **GPL**, comme QGIS.
  C'est une conséquence de l'hôte, pas une préférence. Elle vit séparée du noyau,
  qui ne lie jamais QGIS et reste utilisable sans lui.
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
