# CLAUDE.md — Cassini

> Ce fichier est la source de vérité du projet. Lis-le en entier avant chaque session.
> Si une demande contredit ce fichier, signale-le avant d'agir.
>
> Contrat machine : `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/spec-cassini-mvp.md`.
> Dossier de recherche : `_bmad-output/initiative-cassini-mvp/research-sig-francais-hors-ligne-pour-la-defense/`.

## 1. Vision

Cassini est un SIG (système d'information géographique) open source, de bureau,
qui fonctionne **100 % hors ligne**, **s'organise en projets** comme ArcGIS, et
se pilote **en langage naturel** grâce à un agent IA intégré. Il vise à remplacer
ArcGIS Pro pour les usages courants. **L'utilisateur apporte ses propres
données** : Cassini n'en télécharge, n'en catalogue et n'en redistribue aucune.

Phrase de démo : « Trouve les parcelles constructibles du PLU à moins de 500 m
d'une gare à Villeneuve-le-Roi et sors-moi la carte en PDF. » — sur un projet où
l'utilisateur a déjà importé ses fichiers.

Public cible : bureaux d'études (urbanisme, environnement), collectivités,
étudiants. Second débouché : les postes isolés du réseau, défense et sécurité
comprises, servis par un **module séparé et privé** (voir §2.7), le présent dépôt
restant un noyau civil généraliste.

Positionnement : **complément** d'un existant installé, pas remplaçant. Le calcul
de terrain n'est pas rare — l'intervisibilité sur modèle d'élévation existe dans
les systèmes embarqués français depuis au moins 2011. Ce qui n'existe pas, c'est
un SIG de bureau hors ligne piloté par une IA locale.

## 2. Principes non négociables

1. **Clean-room.** On travaille uniquement à partir de la documentation publique
   d'Esri et du comportement observable. Interdit : copier du code, des icônes,
   des noms de produits, des presets ou des assets d'Esri. Lire un format publié
   (Shapefile) est permis ; lire ou écrire un format de projet Esri (`.aprx`,
   `.mxd`) ne l'est pas.
2. **Pas de GPL ni de LGPL**, dans le projet comme dans ses dépendances, y
   compris par liaison statique et y compris transitivement. Conséquence
   directe : ni GEOS, ni l'extension `spatial` de DuckDB qui le lie
   statiquement. Licences autorisées : MIT, Apache-2.0, BSD.
3. **Hors ligne total.** Aucune fonction du MVP n'utilise le réseau. Le réseau ne
   sert qu'à brancher une IA externe, en option.
4. **L'utilisateur apporte ses données.** Cassini ne télécharge rien et ne
   modifie jamais les fichiers d'origine : l'import écrit une copie dans le
   projet.
5. **Agent-first.** Toute action utilisateur passe par une commande du registre
   (voir §4). Aucune logique métier dans l'interface.
6. **Indépendant des fournisseurs d'IA.** Le code ne dépend d'aucune IA en
   particulier. IA locale par défaut, IA externes via MCP.
7. **Séparation civil / défense.** Ce dépôt public ne contient **aucune fonction
   spécialement conçue pour l'usage militaire**. Celles-là vivent dans un dépôt
   privé distinct, et aucune fonction de fonction réservée au module privé n'est écrite
   avant réponse écrite à une demande de classement (formulaire [retiré],
   Bureau de la réglementation, du classement, du double-usage et de la sécurité
   industrielle). Motif : la charnière du contrôle de la liste commune des
   équipements militaires de l'UE est l'expression *specially designed for
   military use*, et l'article L.2332-1 du Code de la défense soumet le commerce
   de matériels de guerre à une autorisation (AFCI).
8. **Petit et solide.** Mieux vaut 5 fonctions fiables que 50 à moitié finies.

## 3. Architecture

```
┌─────────────────────────────────────────────┐
│  Interface (Tauri v2 + TypeScript + MapLibre)│
└──────────────────┬──────────────────────────┘
                   │ appels de commandes
┌──────────────────▼──────────────────────────┐
│  Registre de commandes (Rust)                │
│  ← utilisé par : UI, CLI, agent local, MCP   │
└──────┬───────────────┬───────────────┬──────┘
       │               │               │
┌──────▼─────┐  ┌──────▼──────┐  ┌─────▼───────┐
│ Moteur géo │  │ Stockage    │  │ Couche IA   │
│ geo (Rust) │  │ du projet   │  │ llama.cpp   │
│ terrain    │  │ GeoParquet, │  │ (local) +   │
│ (Rust)     │  │ PMTiles     │  │ serveur MCP │
└────────────┘  └─────────────┘  └─────────────┘
```

- **App de bureau** : Tauri v2 (Windows, macOS, Linux).
- **Moteur** : Rust, **sans GDAL ni GEOS par défaut**. Géométrie vectorielle avec
  la crate `geo` (MIT), projections avec `proj` (MIT). Lecture GeoPackage via
  SQLite. Analyse de terrain reprise ou réimplémentée depuis `whitebox_next_gen`
  (MIT OR Apache-2.0), qui est en Rust.
  Si GDAL devient nécessaire, sa licence est un agrégat : auditer la
  configuration de compilation pilote par pilote avant adoption.
- **Carte** : MapLibre GL JS (BSD-3). Fonds de carte hors ligne au format
  PMTiles (BSD-3).
- **Données du projet** : stockées localement en GeoParquet.
- **IA locale** : modèle open source (Mistral ou Qwen, quantifié GGUF) servi par
  llama.cpp, embarqué avec l'app.
- **IA externe (optionnel)** : serveur MCP qui expose le registre de commandes,
  pour Claude, ChatGPT, Gemini, etc.

## 4. Registre de commandes

Chaque fonction est une commande unique, décrite une seule fois :

- `id` (ex. `analysis.buffer`, `terrain.viewshed`)
- description courte en français et en anglais (lue par l'IA)
- paramètres typés avec schéma JSON
- résultat typé
- implémentation en Rust
- au moins un test

L'interface, la CLI, l'agent local et le serveur MCP appellent **les mêmes
commandes**. Aucune fonction ne doit exister ailleurs que dans le registre.

Chaque exécution est journalisée dans l'historique du projet pour pouvoir être
rejouée (reproductibilité).

## 5. Données : apportées par l'utilisateur

Cassini ne télécharge rien. L'utilisateur importe les fichiers qu'il possède
déjà, et l'import ne modifie jamais l'original.

**Formats d'import visés au MVP** (vecteur) :

| Format | Remarque |
|---|---|
| GeoPackage (`.gpkg`) | Base SQLite ; couvre aussi le profil de défense du DGIWG, qui se lit comme un GeoPackage ordinaire |
| Shapefile (`.shp`) | Spécification publique ; lecture sans code Esri |
| GeoJSON | — |
| CSV à coordonnées | Colonnes X/Y, système précisé à l'import |
| GeoParquet | Format de stockage interne du projet |

**Raster** : modèles numériques d'élévation pour l'analyse de terrain (§6.4).

**Systèmes de coordonnées** : Lambert-93 (EPSG:2154) et WGS84 au minimum ;
reprojection à l'import quand la couche diffère du projet.

Les sources publiques françaises (cadastre Etalab, Géoportail de l'Urbanisme,
BAN, IGN BD TOPO, ADMIN EXPRESS) restent la matière première naturelle des
utilisateurs, mais leur téléchargement est **hors périmètre** : au plus une page
de documentation indiquant où les trouver, sans code.

## 6. Périmètre du MVP

1. **Projet** : créer, ouvrir, enregistrer un projet qui conserve couches, ordre,
   symbologie, étendue, mise en page et historique.
2. **Import** : ajouter au projet les fichiers de l'utilisateur, reprojetés si
   besoin, sans toucher aux originaux.
3. **Carte** : afficher des couches vectorielles et un fond de carte hors ligne.
4. **Analyses** : zone tampon, intersection, sélection par attribut,
   statistiques par zone, isochrones à pied et en voiture (graphe routier
   apporté par l'utilisateur) ; et analyse de terrain sur MNT — champ de vision,
   ligne de vue, intervisibilité, pente, exposition, ombrage, zones masquées,
   points dominants, franchissabilité.
5. **Agent IA** : une demande en langage naturel devient une suite de commandes,
   affichée étape par étape, que l'utilisateur peut valider et rejouer.
6. **Export** : carte PDF avec titre, légende, échelle, flèche du nord et sources.

**Hors périmètre (V2 et plus)** : toute fonction spécialement conçue pour l'usage
militaire (elle va au dépôt privé), téléchargement ou redistribution de données,
formats de projet Esri, formats et symbologies de défense DTED, NITF et APP-6,
édition avancée, 3D, IA sur images satellite, plugins, collaboration, version web.

## 7. Étapes

- **Étape 1** : squelette Tauri, carte MapLibre, fond PMTiles hors ligne.
- **Étape 2** : registre de commandes, notion de projet, import des formats
  vecteur.
- **Étape 3** : analyses vectorielles et historique rejouable.
- **Étape 4** : chaîne raster, MNT et analyse de terrain.
- **Étape 5** : agent IA local (llama.cpp) et serveur MCP.
- **Étape 6** : export PDF et scénario de démo complet.
- **Étape 7** : vidéo de démo, README, publication GitHub.

Ne commence pas une étape tant que la précédente n'est pas testée et commitée.

## 8. Règles de travail pour Claude

- Le propriétaire du projet ne code pas lui-même : explique chaque changement
  en 2–3 phrases simples, en français.
- **Une tâche courte par session.** Si une tâche paraît grosse, propose de la
  découper avant de coder.
- Avant d'ajouter une dépendance, demande et donne sa licence. Licences
  autorisées : MIT, Apache-2.0, BSD. Pas de GPL/LGPL, même par liaison statique.
- Chaque nouvelle commande vient avec son test.
- Après chaque étape qui marche : lancer les tests, puis proposer un commit
  avec un message clair.
- Ne jamais inventer une API de données : vérifier la documentation officielle
  de la source.
- Ne jamais copier quoi que ce soit d'Esri (voir §2.1).
- **Ne jamais pousser dans ce dépôt public une fonction conçue pour l'usage
  militaire** (voir §2.7).
- Si quelque chose ne marche pas après deux essais, s'arrêter et expliquer le
  problème au lieu d'empiler des correctifs.

## 9. Conventions

- Code et noms de variables en anglais, commentaires et docs utilisateur en
  français.
- Rust : `cargo fmt` et `cargo clippy` sans avertissement.
- TypeScript : mode strict.
- Structure du dépôt :

```
/app          interface Tauri + TypeScript
/core         moteur Rust et registre de commandes
/ai           couche IA (llama.cpp, serveur MCP)
/fixtures     jeux de données de test, légers, pour la recette
/docs         documentation, parité fonctionnelle, roadmap
/tests        tests d'intégration et scénario de démo
```

## 10. Licence

Noyau public : Apache-2.0 (permet un usage commercial et institutionnel). Le
module défense, s'il voit le jour, est un dépôt privé distinct dont la licence
et les conditions de commercialisation dépendent de la réponse au classement
(voir §2.7).
