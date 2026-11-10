# CLAUDE.md — Cassini

> Ce fichier est la source de vérité du projet. Lis-le en entier avant chaque session.
> Si une demande contredit ce fichier, signale-le avant d'agir.
>
> Contrat machine : `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/spec-cassini-mvp.md`.
> Dossier de recherche initial : hors de ce dépôt, dans le dépôt privé `cassini-defense-prive`
> (`recherche/research-sig-francais-hors-ligne-pour-la-defense/`), à côté de ce dossier.
> Virage du 2026-10-14, qui explique l'organisation actuelle :
> `_bmad-output/initiative-cassini-mvp/change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md`.
>
> Contrat d'architecture, qui fixe les invariants de la frontière entre le noyau
> et l'extension, et ne se re-décide pas en passant :
> `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`.

## 1. Vision

Cassini est l'**interface en langage naturel d'un SIG, qui fonctionne 100 % hors
ligne**. L'utilisateur formule sa demande en français ; Cassini la traduit en une
suite d'opérations qu'il affiche, fait valider, exécute et journalise pour pouvoir
les rejouer. **L'utilisateur apporte ses propres données** : Cassini n'en
télécharge, n'en catalogue et n'en redistribue aucune.

Phrase de démo : « Trouve les parcelles constructibles du PLU à moins de 500 m
d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF. » — sur un projet où
l'utilisateur a déjà ouvert ses fichiers.

Public cible : bureaux d'études (urbanisme, environnement), collectivités,
étudiants — en particulier l'utilisateur occasionnel, qui sait ce qu'il veut mais
pas où cliquer. Second débouché : les postes isolés du réseau, défense et
sécurité comprises, servis par un **module séparé et privé** (voir §2.8), le
présent dépôt restant un noyau civil généraliste.

**Le concurrent est QGIS, pas ArcGIS Pro.** QGIS est libre, hors ligne, organisé
en projets, mûr depuis vingt ans et déjà installé chez les utilisateurs visés.
Reconstruire sa carte, son import, ses analyses et son export PDF serait perdre
des années pour arriver à moins bien. Ce qui n'existe nulle part, c'est un SIG de
bureau hors ligne piloté par une IA locale. Le calcul de terrain n'est pas rare —
l'intervisibilité sur modèle d'élévation existe dans les systèmes embarqués
français depuis au moins 2011, et QGIS la fait gratuitement par GRASS. Le fossé
concurrentiel est l'agent hors ligne, pas la géométrie.

**Conséquence sur la construction.** Au MVP, Cassini est une **extension QGIS
posée sur un noyau indépendant** : QGIS exécute la carte, l'ouverture des
fichiers, le projet, les analyses vectorielles et l'export PDF ; le noyau porte le
registre de commandes, l'agent, le journal rejouable et un moteur de terrain en
licence permissive. L'application de bureau autonome est reportée en V2 — son
code existant reste au dépôt comme fondation.

## 2. Principes non négociables

1. **Clean-room.** On travaille uniquement à partir de la documentation publique
   d'Esri et du comportement observable. Interdit : copier du code, des icônes,
   des noms de produits, des presets ou des assets d'Esri. Lire un format publié
   (Shapefile) est permis ; lire ou écrire un format de projet Esri (`.aprx`,
   `.mxd`) ne l'est pas.
2. **Pas de GPL ni de LGPL dans le noyau**, y compris par liaison statique et y
   compris transitivement. Conséquence directe : ni GEOS, ni l'extension
   `spatial` de DuckDB qui le lie statiquement. Licences autorisées dans le
   noyau : MIT, Apache-2.0, BSD.
3. **L'extension QGIS est en GPL**, QGIS l'étant. Elle vit séparée du noyau, le
   noyau ne lie jamais QGIS, et les deux communiquent **à distance de bras, par
   processus séparé**. Passer par QGIS ne lève pas le principe 2 : QGIS embarque
   GDAL et GEOS de son côté, l'extension les utilise dans le processus de QGIS,
   le noyau ne les lie jamais. Le canal ne porte que des messages publics et
   documentés — identifiants, chemins, paramètres à schéma JSON, résultats typés —
   et jamais une structure de données interne : la FAQ de la FSF réserve le cas
   d'un échange assez intime pour que deux programmes n'en fassent qu'un.
4. **Le noyau tourne sans QGIS.** Il se lance, s'exécute et se teste seul. C'est
   ce qui fonde l'argument juridique du principe 3 et ce qui rend la V2
   autonome possible. Toute dépendance du noyau envers QGIS est un bug.
5. **Hors ligne total.** Aucune fonction du MVP n'utilise le réseau. Le réseau ne
   sert qu'à brancher une IA externe, en option. L'installeur en ligne (§3,
   « Distribution ») n'est pas une fonction du produit : programme séparé, il
   télécharge une seule fois, pendant l'installation, puis le noyau et
   l'extension ne touchent plus jamais au réseau.
6. **L'utilisateur apporte ses données.** Cassini ne télécharge rien et ne
   modifie jamais les fichiers d'origine.
7. **Agent-first.** Toute action utilisateur passe par une commande du registre
   (voir §4). Aucune logique métier dans l'extension.
8. **Indépendant des fournisseurs d'IA.** Le code ne dépend d'aucune IA en
   particulier. IA locale par défaut, IA externes via MCP.
9. **Séparation civil / défense.** Ce dépôt public ne contient **aucune fonction
   spécialement conçue pour l'usage militaire**. Celles-là vivent dans un dépôt
   privé distinct, qui s'appuiera sur le **noyau** et non sur l'extension, cette
   dernière étant en GPL. Aucune décision, aucun calendrier ni aucun détail sur ce
   module privé n'est écrit dans un fichier versionné de ce dépôt : ils vivent dans
   des notes locales non versionnées (`CLAUDE.local.md`). Sa commercialisation est
   subordonnée aux autorisations que la réglementation impose.
10. **Petit et solide.** Mieux vaut 5 fonctions fiables que 50 à moitié finies.

## 3. Architecture

```
┌────────────────────────────────────────────────────┐
│  QGIS  (GPL — fourni, pas écrit ici)               │
│  carte, formats, projet, analyses, mise en page    │
│  ┌──────────────────────────────────────────────┐  │
│  │  Extension Cassini (Python, GPL)             │  │
│  │  panneau + exécutant des gestes QGIS,        │  │
│  │  aucune logique métier, aucune décision      │  │
│  └───────────────────┬──────────────────────────┘  │
└──────────────────────┼─────────────────────────────┘
                       │  JSON-RPC 2.0, un message par ligne,
                       │  sur l'entrée et la sortie standard,
                       │  dans les deux sens. Processus enfant
                       │  lancé par l'extension. Aucun réseau.
┌──────────────────────▼─────────────────────────────┐
│  Noyau Cassini (Rust, Apache-2.0)                  │
│  ┌──────────────────────────────────────────────┐  │
│  │  /cli — binaire cassini                      │  │
│  │  serve · mcp · run · replay                  │  │
│  └───────────────────┬──────────────────────────┘  │
│  ┌───────────────────▼──────────────────────────┐  │
│  │  /core — Registre de commandes               │  │
│  │  la boucle d'exécution vit ici               │  │
│  └──┬────────────────┬────────────────┬────────┘   │
│    │                 │                 │           │
│  ┌──▼────────┐ ┌──────▼──────┐ ┌────────▼───────┐  │
│  │ Terrain   │ │ Journal     │ │ /ai — modèle   │  │
│  │ MNT, /core│ │ rejouable   │ │ GGUF lié au    │  │
│  │           │ │ /core       │ │ processus      │  │
│  └───────────┘ └─────────────┘ └────────────────┘  │
└────────────────────────────────────────────────────┘
```

- **Extension** : Python et PyQGIS. Un panneau ancré dans QGIS. Aucune logique
  métier : elle affiche, elle demande, elle exécute ce que le registre décide.
- **Noyau** : Rust, **sans GDAL ni GEOS**. Analyse de terrain reprise ou
  réimplémentée depuis `whitebox_next_gen` (MIT OR Apache-2.0), qui est en Rust.
  La géométrie vectorielle et la lecture des formats sont déléguées à QGIS au
  MVP ; la crate `geo` ne redevient nécessaire qu'avec la V2 autonome.
- **Canal local** : tranché le 2026-10-14. L'extension lance le binaire du noyau
  comme **processus enfant** et dialogue par son entrée et sa sortie standard, en
  JSON-RPC 2.0, un message par ligne, **dans les deux sens**. Aucune socket
  réseau, aucun port, aucun fichier d'échange ; seule exception, le canal local
  d'IA externe, ouvert à la demande de l'utilisateur (AD-29, 2026-10-30).
- **Qui décide** : le noyau tient la boucle. Il compose le plan, le fait valider,
  puis dicte une étape à la fois et journalise la réponse. L'extension n'enchaîne
  jamais deux commandes d'elle-même.
- **Plancher QGIS** : une seule extension pour les deux séries,
  `qgisMinimumVersion=3.44` et `qgisMaximumVersion=4.99`, tout accès à Qt passant
  par le shim `qgis.PyQt`. Aucun import direct de PyQt5 ni de PyQt6.
- **Distribution** : l'extension au dépôt d'extensions de QGIS, sans binaire et
  sous 25 Mo — le dépôt l'exige ; le binaire du noyau et le modèle dans un paquet
  Cassini installé à part, que l'extension localise dans un ordre fixé, sans
  jamais rien télécharger. Ce paquet passe par **deux portes** (2026-11-10) :
  un installeur léger en ligne, qui télécharge une seule fois, pendant
  l'installation, le noyau depuis une release GitHub et le modèle depuis
  Hugging Face, à des adresses fixes, en vérifiant l'empreinte SHA-256 de chaque
  fichier avant de l'installer ; et un installeur complet hors ligne, un seul
  fichier qui contient tout, pour les postes isolés du réseau. Windows et Linux
  d'abord, macOS plus tard. Seul l'installeur, programme séparé, télécharge :
  jamais le noyau ni l'extension (AD-17). Les mises à jour suivent la même
  frontière : « Mettre à jour Cassini », lancé à la main, est le même programme
  que l'installeur en ligne et lit un manifeste des versions publié dans la
  release GitHub ; un nouveau modèle s'installe à côté des anciens, jamais à leur
  place.
- **IA locale** : modèle open source (Mistral ou Qwen, quantifié GGUF) chargé
  **dans le processus du noyau** par llama.cpp lié en bibliothèque — jamais un
  serveur HTTP, qui rouvrirait une interface réseau.
- **IA externe (optionnel)** : serveur MCP qui expose le registre de commandes.
- **Reporté en V2** : la coquille Tauri v2 et la carte MapLibre, sous `/app`.
  Testées et conservées, hors du chemin critique.

## 4. Registre de commandes

Chaque fonction est une commande unique, décrite une seule fois :

- `id` (ex. `analysis.buffer`, `terrain.viewshed`)
- description courte en français et en anglais (lue par l'IA)
- paramètres typés avec schéma JSON
- résultat typé
- **soit** une implémentation en Rust dans le noyau, **soit** une délégation
  déclarée à QGIS — jamais implicite, jamais les deux
- au moins un test

L'extension QGIS, la CLI, l'agent local et le serveur MCP appellent **les mêmes
commandes**. Aucune fonction ne doit exister ailleurs que dans le registre.

Chaque exécution est journalisée dans un journal Cassini écrit à côté du projet
QGIS, pour pouvoir être rejouée (reproductibilité).

## 5. Données : apportées par l'utilisateur

Cassini ne télécharge rien. L'utilisateur ouvre les fichiers qu'il possède déjà,
et Cassini ne modifie jamais l'original.

**Formats visés au MVP** (vecteur), tous lus par QGIS :

| Format | Remarque |
|---|---|
| GeoPackage (`.gpkg`) | Base SQLite ; couvre aussi le profil de défense du DGIWG, qui se lit comme un GeoPackage ordinaire |
| Shapefile (`.shp`) | Spécification publique ; lecture sans code Esri |
| GeoJSON | — |
| CSV à coordonnées | Colonnes X/Y, système précisé à l'ouverture |
| GeoParquet | — |

**Raster** : modèles numériques d'élévation pour l'analyse de terrain (§6).

**Systèmes de coordonnées** : Lambert-93 (EPSG:2154) et WGS84 au minimum ;
reprojection quand la couche diffère du projet.

Les sources publiques françaises (cadastre Etalab, Géoportail de l'Urbanisme,
BAN, IGN BD TOPO, ADMIN EXPRESS) restent la matière première naturelle des
utilisateurs, mais leur téléchargement est **hors périmètre** : au plus une page
de documentation indiquant où les trouver, sans code.

Pour les essais à la main, un jeu de données réelles (Villeneuve-le-Roi, Natural Earth,
modèle d'élévation) vit **hors du dépôt**, dans `../donnees-test-cassini/` : il n'est jamais
commité ni redistribué, et `LISEZMOI.txt` y donne les sources et les licences. Les tests
automatiques n'utilisent que des fixtures synthétiques de `/fixtures`.

## 6. Périmètre du MVP

Ce que **le noyau** porte :

1. **Registre de commandes** : le contrat décrit en §4, et les quatre points
   d'entrée qui l'appellent à l'identique.
2. **Journal rejouable** : toute exécution journalisée à côté du projet QGIS, et
   rejouable à l'identique après fermeture et réouverture.
3. **Agent IA** : une demande en langage naturel devient une suite de commandes,
   affichée étape par étape, que l'utilisateur valide et peut rejouer.
4. **Serveur MCP** : le même registre exposé à une IA externe, en option.
5. **Analyse de terrain** : **champ de vision**, **ligne de vue** entre deux
   points, **pente** (epic 4), puis exposition, ombrage, champ de vision cumulé,
   points hauts, surface de coût et chemin de moindre coût, superposition
   pondérée (epic 11). **Géométrie** : conversion de coordonnées WGS84, UTM et
   MGRS, distance et azimut, point par azimut et distance, recoupement (epic 11).
   En Rust, sans aucune dépendance GPL ou LGPL — c'est la seule base sur
   laquelle le module défense privé pourra être construit. Les identifiants et
   paramètres de toutes les commandes sont figés dans le catalogue des commandes
   (changement de cap du 2026-11-05) ; ils n'y changent que par un nouveau
   changement de cap.

Ce que **QGIS exécute**, orchestré par le registre à travers l'extension :

6. **Carte et fond de carte**, **ouverture des fichiers**, **projet**,
   **analyses vectorielles** (zone tampon, intersection, sélection par attribut,
   statistiques par zone, isochrones), **export PDF** avec titre, légende,
   échelle, flèche du nord et sources.

**Hors périmètre (V2 et plus)** : l'application de bureau autonome (carte
MapLibre, chaîne vectorielle propre, format de projet Cassini) ; hydrologie et
couverture radio (après la démo, identifiants réservés au catalogue) ; les
paramètres propres à un usage particulier (classes de véhicules, seuils métier),
que le noyau ne porte pas ; toute fonction spécialement
conçue pour l'usage militaire (elle va au dépôt privé) ; téléchargement ou
redistribution de données ; formats de projet Esri ; formats et symbologies de
défense DTED, NITF et APP-6 ; édition avancée ; 3D ; IA sur images satellite ;
système de plugins hébergé par Cassini ; collaboration ; version web. Aucune
parité avec QGIS n'est visée ni revendiquée.

## 7. Étapes

Séquence décidée le 2026-10-14. Elle remplace l'ancienne, qui finissait par
l'agent.

- **Étape 1** : socle technique, verrou hors ligne, recette automatique. *Fait
  pour l'essentiel.*
- **Étape 2** : **épreuve du modèle local** — un modèle quantifié sait-il
  transformer une phrase française en suite de commandes typées correcte, sur un
  registre factice ? Ne dépend de rien, et conditionne tout le reste.
- **Étape 3** : registre de commandes et journal rejouable. L'épine dorsale.
- **Étape 4** : façade QGIS — extension, canal local, commandes exécutées par
  QGIS.
- **Étape 5** : agent IA local (llama.cpp) et serveur MCP.
- **Étape 6** : analyse de terrain dans le noyau.
- **Étape 6 bis** : géométrie et terrain étendu (epic 11), catalogue des
  commandes figé (changement de cap du 2026-11-05).
- **Étape 7** : scénario de démo complet, puis publication (dépôt d'extensions
  QGIS, installeurs, vidéo, README).

Ne commence pas une étape tant que la précédente n'est pas testée et commitée.

## 8. Règles de travail pour Claude

- Le propriétaire du projet ne code pas lui-même : explique chaque changement
  en 2–3 phrases simples, en français.
- **Une tâche courte par session.** Si une tâche paraît grosse, propose de la
  découper avant de coder.
- Avant d'ajouter une dépendance, demande et donne sa licence. Dans le noyau :
  MIT, Apache-2.0, BSD seulement. Pas de GPL/LGPL, même par liaison statique.
  L'extension, elle, est déjà en GPL.
- Chaque nouvelle commande vient avec son test.
- Après chaque étape qui marche : lancer les tests, puis proposer un commit
  avec un message clair.
- Ne jamais inventer une API de données : vérifier la documentation officielle
  de la source.
- Ne jamais copier quoi que ce soit d'Esri (voir §2.1).
- **Ne jamais pousser dans ce dépôt public une fonction conçue pour l'usage
  militaire** (voir §2.9).
- Sur GitHub, on ne pousse que `main` ; les branches d'entrée restent locales.
- Les invariants d'architecture sont fixés dans le contrat d'architecture cité en
  tête de ce fichier, sous des références stables (`AD-1` à `AD-31`). On les cite,
  on ne les re-décide pas : une décision qui en contredit une se signale avant
  d'agir.
- Entrées de risque `high` (rejeu, canal, analyses) : relecture à plusieurs lentilles (`thorough`), pas la seule relecture `quick` (rétro de l'epic 9, A6).
- Ne construire un ticket de fermeture (refactor sweep, suite de bout en bout) qu'une fois construites toutes les entrées qu'il attend dans `after` (rétro de l'epic 9, A1).
- Si quelque chose ne marche pas après deux essais, s'arrêter et expliquer le
  problème au lieu d'empiler des correctifs.

## 9. Conventions

- Code et noms de variables en anglais, commentaires et docs utilisateur en
  français.
- Rust : `cargo fmt` et `cargo clippy` sans avertissement.
- TypeScript : mode strict.
- Python de l'extension : contrôlé par la recette, au même titre que le reste.
- Structure du dépôt :

```
/core         crate cassini-core : registre de commandes, terrain, journal
/cli          crate cassini : binaire unique — serve, mcp, run, replay
/qgis-plugin  extension QGIS (Python, GPL) — l'interface du MVP
/ai           crate cassini-ai : modèle local lié, planification
/app          coquille Tauri + carte MapLibre — reporté en V2, conservé
/fixtures     jeux de données de test, légers, pour la recette
/docs         documentation, feuille de route
/tests        tests d'intégration et scénario de démo
```

Le binaire vit dans `/cli` parce qu'il a besoin du noyau **et** de la couche IA,
alors que `/core` ne doit pas dépendre de `/ai` : le module défense privé et la V2
autonome doivent pouvoir prendre le registre et le terrain sans l'IA.

## 10. Licence

- **Noyau** (`/core`, `/ai`) : Apache-2.0. Usage commercial et institutionnel
  permis.
- **Extension** (`/qgis-plugin`) : GPL, comme QGIS. Ce n'est pas un choix mais
  une conséquence.
- **Module défense**, s'il voit le jour : dépôt privé distinct, construit sur le
  noyau et non sur l'extension. Sa licence et ses conditions de commercialisation
  dépendent de la régularisation du classement, à faire avant toute vente (voir §2.9).
