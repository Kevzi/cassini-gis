# CLAUDE.md — Cassini

> Ce fichier est la source de vérité du projet. Lis-le en entier avant chaque session.
> Si une demande contredit ce fichier, signale-le avant d'agir.
>
> Contrat machine : `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/spec-cassini-mvp.md`.
> Dossier de recherche : `_bmad-output/initiative-cassini-mvp/research-sig-francais-hors-ligne-pour-la-defense/`.
> Virage du 2026-10-14, qui explique l'organisation actuelle :
> `_bmad-output/initiative-cassini-mvp/change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md`.

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
   le noyau ne les lie jamais.
4. **Le noyau tourne sans QGIS.** Il se lance, s'exécute et se teste seul. C'est
   ce qui fonde l'argument juridique du principe 3 et ce qui rend la V2
   autonome possible. Toute dépendance du noyau envers QGIS est un bug.
5. **Hors ligne total.** Aucune fonction du MVP n'utilise le réseau. Le réseau ne
   sert qu'à brancher une IA externe, en option.
6. **L'utilisateur apporte ses données.** Cassini ne télécharge rien et ne
   modifie jamais les fichiers d'origine.
7. **Agent-first.** Toute action utilisateur passe par une commande du registre
   (voir §4). Aucune logique métier dans l'extension.
8. **Indépendant des fournisseurs d'IA.** Le code ne dépend d'aucune IA en
   particulier. IA locale par défaut, IA externes via MCP.
9. **Séparation civil / défense.** Ce dépôt public ne contient **aucune fonction
   spécialement conçue pour l'usage militaire**. Celles-là vivent dans un dépôt
   privé distinct, qui s'appuiera sur le **noyau** et non sur l'extension, cette
   dernière étant en GPL. Aucune fonction de fonction réservée au module privé n'est
   écrite avant réponse écrite à une demande de classement (formulaire
   [retiré], Bureau de la réglementation, du classement, du double-usage et de la
   sécurité industrielle). Motif : la charnière du contrôle de la liste commune
   des équipements militaires de l'UE est l'expression *specially designed for
   military use*, et l'article L.2332-1 du Code de la défense soumet le commerce
   de matériels de guerre à une autorisation (AFCI).
10. **Petit et solide.** Mieux vaut 5 fonctions fiables que 50 à moitié finies.

## 3. Architecture

```
┌──────────────────────────────────────────────┐
│  QGIS  (GPL — fourni, pas écrit ici)         │
│  carte, formats, projet, analyses, mise en page│
│  ┌────────────────────────────────────────┐  │
│  │  Extension Cassini (Python, GPL)       │  │
│  │  panneau de conversation, appels       │  │
│  └──────────────┬─────────────────────────┘  │
└─────────────────┼────────────────────────────┘
                  │ canal local, processus séparé
┌─────────────────▼────────────────────────────┐
│  Noyau Cassini (Rust, Apache-2.0)            │
│  ┌────────────────────────────────────────┐  │
│  │  Registre de commandes                 │  │
│  │  ← UI, CLI, agent local, serveur MCP   │  │
│  └──┬──────────────┬──────────────┬───────┘  │
│     │              │              │          │
│  ┌──▼───────┐ ┌────▼──────┐ ┌─────▼───────┐  │
│  │ Terrain  │ │ Journal   │ │ Couche IA   │  │
│  │ (MNT)    │ │ rejouable │ │ llama.cpp + │  │
│  │          │ │           │ │ serveur MCP │  │
│  └──────────┘ └───────────┘ └─────────────┘  │
└──────────────────────────────────────────────┘
```

- **Extension** : Python et PyQGIS. Un panneau ancré dans QGIS. Aucune logique
  métier : elle affiche, elle demande, elle exécute ce que le registre décide.
- **Noyau** : Rust, **sans GDAL ni GEOS**. Analyse de terrain reprise ou
  réimplémentée depuis `whitebox_next_gen` (MIT OR Apache-2.0), qui est en Rust.
  La géométrie vectorielle et la lecture des formats sont déléguées à QGIS au
  MVP ; la crate `geo` ne redevient nécessaire qu'avec la V2 autonome.
- **Canal local** : forme non encore tranchée (question ouverte de la spec). Elle
  porte l'argument juridique du §2.3, donc elle se décide avant l'extension.
- **IA locale** : modèle open source (Mistral ou Qwen, quantifié GGUF) servi par
  llama.cpp.
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

## 6. Périmètre du MVP

Ce que **le noyau** porte :

1. **Registre de commandes** : le contrat décrit en §4, et les quatre points
   d'entrée qui l'appellent à l'identique.
2. **Journal rejouable** : toute exécution journalisée à côté du projet QGIS, et
   rejouable à l'identique après fermeture et réouverture.
3. **Agent IA** : une demande en langage naturel devient une suite de commandes,
   affichée étape par étape, que l'utilisateur valide et peut rejouer.
4. **Serveur MCP** : le même registre exposé à une IA externe, en option.
5. **Analyse de terrain**, trois commandes et pas neuf : **champ de vision**,
   **ligne de vue** entre deux points, **pente**. En Rust, sans aucune
   dépendance GPL ou LGPL — c'est la seule base sur laquelle le module défense
   privé pourra être construit.

Ce que **QGIS exécute**, orchestré par le registre à travers l'extension :

6. **Carte et fond de carte**, **ouverture des fichiers**, **projet**,
   **analyses vectorielles** (zone tampon, intersection, sélection par attribut,
   statistiques par zone, isochrones), **export PDF** avec titre, légende,
   échelle, flèche du nord et sources.

**Hors périmètre (V2 et plus)** : l'application de bureau autonome (carte
MapLibre, chaîne vectorielle propre, format de projet Cassini) ; les six autres
commandes de terrain (intervisibilité multi-points, exposition, ombrage, zones
masquées, points dominants, franchissabilité) ; toute fonction spécialement
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
/core         noyau Rust : registre de commandes, terrain, journal
/qgis-plugin  extension QGIS (Python, GPL) — l'interface du MVP
/ai           couche IA (llama.cpp, serveur MCP)
/app          coquille Tauri + carte MapLibre — reporté en V2, conservé
/fixtures     jeux de données de test, légers, pour la recette
/docs         documentation, feuille de route
/tests        tests d'intégration et scénario de démo
```

## 10. Licence

- **Noyau** (`/core`, `/ai`) : Apache-2.0. Usage commercial et institutionnel
  permis.
- **Extension** (`/qgis-plugin`) : GPL, comme QGIS. Ce n'est pas un choix mais
  une conséquence.
- **Module défense**, s'il voit le jour : dépôt privé distinct, construit sur le
  noyau et non sur l'extension. Sa licence et ses conditions de commercialisation
  dépendent de la réponse au classement (voir §2.9).
