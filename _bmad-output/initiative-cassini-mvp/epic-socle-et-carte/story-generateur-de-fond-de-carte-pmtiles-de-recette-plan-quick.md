# Revue indépendante — lentille Quick — story 1.2 de Cassini

> Prompt autonome. Il ne référence aucun fichier : tout ce dont tu as besoin est
> recopié ci-dessous. À faire tourner dans une session vierge, idéalement sur un
> autre modèle, puis à recoller dans la session Cassini.

Tu es le relecteur. Lis le plan ci-dessous — il porte les critères d'acceptation
et l'intention. Lis ensuite les règles du dépôt. Lis enfin le diff unifié : c'est
le changement à relire.

Signale les critères d'acceptation non tenus, les règles enfreintes et les bugs.
Pour chaque constat : l'emplacement, ce qui ne va pas, et la preuve recoupée avec
le code alentour. Rends une liste Markdown de constats, sans gravité, sans
priorité, sans classement.

N'invoque aucun skill et ne lance aucun sous-agent — c'est toi le relecteur.
Rends tes constats en texte dans ton message final.

---

## 1. Le plan (critères d'acceptation et intention)

````markdown
---
title: 'Generateur de fond de carte PMTiles de recette'
type: 'feature'
ticket: '2'
created: '2026-10-09'
status: 'in-review'
baseline_revision: '541abc39722ac2608af83a015c9c8441cc2d2d95'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'entrée 1.3 doit afficher un fond de carte PMTiles, mais le projet ne télécharge aucune donnée (CLAUDE.md §2.4) et `/fixtures` est vide. Sans fichier de recette, ni la carte ni la garde hors ligne ne sont vérifiables.

**Approach:** écrire un générateur qui fabrique localement une archive PMTiles v3 valide, à tuiles PNG, couvrant la zone de Villeneuve-le-Roi aux zooms 10 à 14, et un vérificateur qui la relit avec la bibliothèque `pmtiles` officielle pour prouver sa conformité.

## Boundaries & Constraints

**Always:** le générateur ne fait aucune requête réseau et n'a aucune dépendance tierce — Node seul, avec son `zlib` intégré. L'archive produite respecte la spécification PMTiles v3 au point d'être relue par la bibliothèque `pmtiles` sans adaptation. Le fichier produit est versionné dans `/fixtures` (exception déjà posée dans `.gitignore`) et pèse moins de 200 Ko. Toute dépendance ajoutée voit sa licence écrite dans le message de commit (CLAUDE.md §8).

**Never:** ne pas télécharger de données, même une seule tuile. Ne pas produire de tuiles vectorielles MVT : hors périmètre, et le PNG suffit à prouver l'affichage. Ne pas toucher à `app/src/basemap.style.json` ni à `app/src/main.ts` — le branchement dans la carte est l'entrée 1.3. Ne pas ajouter de dépendance Rust : rien de ceci n'entre dans le moteur.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Génération nominale | `npm run fixtures` sur un dépôt propre | `fixtures/basemap-villeneuve-le-roi.pmtiles` écrit, en-tête de 127 octets, magie `PMTiles`, version 3 | Aucune erreur attendue |
| Relecture par la bibliothèque | l'archive produite | `pmtiles` lit l'en-tête, les métadonnées et rend chaque tuile annoncée aux zooms 10 à 14 | Échec du script si une tuile annoncée est absente |
| Identifiant de tuile connu | z=12, x=3423, y=1763 | l'identifiant calculé vaut exactement 19078479, valeur donnée par la spécification | Échec si l'écart est non nul |
| Hors de l'emprise | une tuile en dehors de la boîte de Villeneuve-le-Roi | absente de l'archive, et la lecture renvoie « pas de tuile » sans planter | Retour vide, pas d'exception |
| Zoom hors bornes | z=9 ou z=15 | absent de l'archive, bornes `min_zoom` et `max_zoom` de l'en-tête respectées | Retour vide |
| Déterminisme | deux générations consécutives | deux fichiers identiques octet pour octet | Échec de la vérification si l'empreinte diffère |

</frozen-after-approval>

## Code Map

- `app/package.json` -- ajouter `pmtiles` en `devDependencies` (BSD-3-Clause, v4.5.0, licence relevée le 2026-10-09) et deux scripts, `fixtures` et `verify:fixtures`. L'entrée 1.3 promouvra `pmtiles` en dépendance de production quand l'application l'importera.
- `app/scripts/generate-basemap.mjs` -- **à créer.** Le générateur. Node pur, zéro dépendance tierce, `zlib` intégré pour le gzip du répertoire et `zlib.deflateSync` pour les PNG.
- `app/scripts/verify-basemap.mjs` -- **à créer.** Le vérificateur, seul endroit qui importe `pmtiles`.
- `app/scripts/check-offline.mjs` -- modèle à suivre pour le style : Node ESM, échec par `process.exit(1)` avec un message explicite. Ne pas le modifier.
- `app/src-tauri/icons/` -- les icônes du dépôt ont été produites par un encodeur PNG minimal (IHDR, IDAT `zlib.deflate`, IEND, filtre 0, RGBA 8 bits) ; réutiliser cette approche pour les tuiles.
- `fixtures/README.md` -- remplir le tableau des jeux de test avec le fichier produit, sa nature, son origine « généré » et son utilisateur, l'entrée 1.3.
- `app/src/main.ts` et `app/src/basemap.style.json` -- **ne pas toucher**, c'est l'entrée 1.3.

## Tasks & Acceptance

**Execution:**
- [ ] `app/package.json` -- ajouter `pmtiles` en `devDependencies` (BSD-3-Clause, v4.5.0) et les scripts `fixtures` et `verify:fixtures`, puis chaîner `verify:fixtures` dans `check` -- en premier, parce que le vérificateur de la tâche suivante importe ce paquet et que la recette doit protéger le fixture
- [ ] `app/scripts/generate-basemap.mjs` -- écrire l'encodeur PMTiles v3 : varint little-endian, identifiant de tuile par courbe de Hilbert, répertoire racine, métadonnées JSON, section de tuiles ; puis l'encodeur PNG des tuiles et l'assemblage du fichier -- c'est le livrable de l'entrée
- [ ] `app/scripts/verify-basemap.mjs` -- relire l'archive avec `pmtiles` et couvrir les six scénarios de la matrice d'entrées-sorties : en-tête, bornes de zoom, présence de chaque tuile annoncée, identifiant de référence de la spécification, tuile hors emprise, et déterminisme par double génération -- la preuve que l'archive est conforme, pas seulement qu'elle existe, et chaque commande vient avec son test (CLAUDE.md §8)
- [ ] `fixtures/README.md` -- documenter le fichier produit, l'emprise, les zooms couverts et la commande qui le régénère -- sans ça le fixture est un binaire orphelin dans le dépôt

**Acceptance Criteria:**
- Given un dépôt fraîchement cloné et `npm install` fait, when on lance `npm run fixtures`, then `fixtures/basemap-villeneuve-le-roi.pmtiles` est créé et pèse moins de 200 Ko.
- Given l'archive générée, when on lance `npm run check`, then la vérification passe au vert et affiche le nombre de tuiles et les zooms couverts.
- Given l'archive générée, when on l'ouvre avec la bibliothèque `pmtiles`, then son en-tête annonce le type de tuile PNG, la compression interne gzip, les tuiles non compressées, et les zooms 10 à 14.
- Given le générateur, when on l'exécute hors réseau, then il réussit — il ne fait aucune requête.
- Given une tuile corrompue ou tronquée introduite à la main dans l'archive, when on lance la vérification, then elle échoue avec un code de sortie non nul.

## Implementation Notes

**Conversion de Hilbert validée avant tout le reste**, comme le plan l'exigeait : la valeur de référence de la spécification (`z12/3423/1763 → 19078479`) est retrouvée, ainsi que les cinq premières valeurs de la table du §4.1. Le `rot` de l'algorithme classique prend `n`, pas `s` — se tromper là donne un ordre faux qui passe au zoom 1 et casse aux zooms supérieurs.

**Tuiles en gris + alpha (type de couleur PNG 4) au lieu de RVBA.** Mesuré pendant l'implémentation : le damier en RVBA pesait 1 351 octets par tuile contre 685 en gris + alpha, pour un rendu identique ici. Avec une cellule de 64 px, l'archive finale fait 26 Ko pour 35 tuiles, loin sous le plafond de 200 Ko du plan.

**L'emprise ne produit que 35 tuiles**, pas la centaine estimée au plan : la boîte de Villeneuve-le-Roi est petite (z10:1, z11:1, z12:4, z13:9, z14:20). Un seul répertoire racine suffit donc largement, et le garde-fou des 16 384 premiers octets n'est jamais approché.

**Le contrôle d'intégrité PNG a dû être refait.** La première version ne vérifiait que la signature et la présence d'`IEND` : l'inversion d'un octet au milieu d'un IDAT passait au vert, et seul le contrôle de déterminisme l'attrapait. `pngValide` parcourt maintenant les morceaux, recalcule chaque CRC avec `zlib.crc32` et décompresse l'IDAT ; la corruption est désormais nommée à la tuile près (`z14/8299/5646 : CRC du morceau IDAT invalide`). Deux contrôles indépendants la détectent.

**Dépendance transitive déclarée (CLAUDE.md §8) :** `pmtiles` 4.5.0 est en BSD-3-Clause et entraîne `fflate` 0.8.3, en **MIT**, relevé sur le paquet installé. Aucune autre dépendance n'entre.

**Le calcul d'identifiant est recoupé avec la bibliothèque**, pas seulement avec la valeur de référence : `zxyToTileId` de `pmtiles` est comparé au nôtre sur les 35 tuiles. Juge indépendant plutôt qu'auto-attestation.

**Déterminisme confirmé par construction :** `zlib.gzipSync` de Node écrit un MTIME nul, donc la sortie est stable d'une exécution à l'autre. Vérifié par le contrôle de double génération.

**Vérifié hors périmètre :** le diff ne touche ni `app/src/main.ts`, ni `app/src/basemap.style.json`, ni `core/`, ni `app/src-tauri/`, conformément au « Never » du plan. Le branchement dans la carte reste l'entrée 1.3.

**23 contrôles** dans le vérificateur, les six lignes de la matrice couvertes, toutes passées.

## Plan Change Log

## Review Triage Log

## Design Notes

**Le piège de la spécification.** Le pseudocode de l'annexe A.1 de la spécification PMTiles v3 **omet le nombre d'entrées**, alors que le §4.2 l'exige comme première partie du répertoire et que la procédure de décodage du §4.3 le lit en premier. Le répertoire s'encode donc dans cet ordre : nombre d'entrées, puis les identifiants delta-encodés, puis les longueurs de série, puis les longueurs, puis les décalages — chaque valeur en varint little-endian.

**Identifiant de tuile.** `tile_id = (4^z - 1) / 3 + hilbert(2^z, x, y)`. Le premier terme est le cumul des tuiles des zooms inférieurs ; le second est la position sur la courbe de Hilbert d'ordre `z`. L'ordre attendu au zoom 1 est `(0,0)=1, (0,1)=2, (1,1)=3, (1,0)=4`, ce qui correspond à l'algorithme `xy2d` classique à rotations. La valeur de référence de la spécification, `z=12, x=3423, y=1763 → 19078479`, sert de test et doit être vérifiée **avant** d'écrire la moindre tuile.

**En-tête.** 127 octets : magie `PMTiles` sur 7 octets, version `3`, puis huit entiers non signés 64 bits little-endian (décalage et longueur du répertoire racine, des métadonnées, des répertoires feuilles, des tuiles), trois compteurs 64 bits, puis `clustered = 1`, compression interne `2` (gzip), compression des tuiles `1` (aucune, le PNG est déjà compressé), type de tuile `2` (PNG), zoom minimum et maximum, positions minimale, maximale et centrale sur 8 octets chacune — longitude puis latitude, chacune multipliée par 10 000 000 et écrite en entier signé 32 bits little-endian — et le zoom central.

**Décalages contigus.** Les tuiles étant écrites dans l'ordre des identifiants, `clustered` vaut 1 et chaque décalage sauf le premier s'encode en `0` quand il suit exactement la tuile précédente. Un seul répertoire racine suffit : l'emprise retenue produit bien moins d'entrées que la limite de 16 257 octets compressés.

**Emprise et zooms.** La boîte `[2.37, 48.71]`–`[2.46, 48.76]` est celle du repère déjà présent dans `app/src/basemap.style.json`, et la carte s'ouvre sur `[2.415, 48.735]` au zoom 12 d'après `app/src/main.ts`. Les zooms 10 à 14 encadrent donc l'ouverture par défaut. Au-delà de l'emprise, l'archive est vide, ce qui est assumé et documenté : c'est un fixture, pas un fond de production.

**Contenu des tuiles.** Un damier avec bordure et diagonale suffit à voir que le déplacement et le zoom fonctionnent. Pas de texte : rendre des chiffres demanderait une police embarquée, hors périmètre.

**Pourquoi Node et pas Rust.** Node est déjà requis pour construire l'interface, son `zlib` est intégré, et c'est le même interpréteur que la bibliothèque `pmtiles` qui sert de juge. Un générateur Rust imposerait des dépendances de compression au moteur, dont CLAUDE.md §3 exige qu'il reste sans dépendance.

## Verification

**Commands:**
- `cd app && npm install` -- expected: `pmtiles` installé, aucune autre dépendance ajoutée
- `cd app && npm run fixtures` -- expected: `fixtures/basemap-villeneuve-le-roi.pmtiles` écrit, taille affichée, moins de 200 Ko
- `cd app && npm run check` -- expected: typage strict, garde hors ligne et vérification du fixture tous au vert
- `cd app && npm run fixtures && sha256sum ../fixtures/*.pmtiles && npm run fixtures && sha256sum ../fixtures/*.pmtiles` -- expected: deux empreintes identiques (déterminisme)
- `cargo test -p cassini-core && cargo clippy -p cassini-core --all-targets -- -D warnings && cargo fmt --all --check` -- expected: le moteur reste intact et vert

````

---

## 2. Les règles du dépôt (CLAUDE.md à la racine)

````markdown
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

````

---

## 3. Le changement à relire (diff unifié)

````diff
diff --git a/_bmad-output/initiative-cassini-mvp/epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md b/_bmad-output/initiative-cassini-mvp/epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md
new file mode 100644
index 0000000..e7dc038
--- /dev/null
+++ b/_bmad-output/initiative-cassini-mvp/epic-socle-et-carte/story-generateur-de-fond-de-carte-pmtiles-de-recette-plan.md
@@ -0,0 +1,99 @@
+---
+title: 'Generateur de fond de carte PMTiles de recette'
+type: 'feature'
+ticket: '2'
+created: '2026-10-09'
+status: 'in-progress'
+baseline_revision: '541abc39722ac2608af83a015c9c8441cc2d2d95'
+route: 'full'
+route_source: 'auto'
+risk: 'medium'
+review: ''
+review_source: ''
+lenses_ran: []
+review_loop_iteration: 0
+context: []
+---
+
+<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">
+
+## Intent
+
+**Problem:** l'entrée 1.3 doit afficher un fond de carte PMTiles, mais le projet ne télécharge aucune donnée (CLAUDE.md §2.4) et `/fixtures` est vide. Sans fichier de recette, ni la carte ni la garde hors ligne ne sont vérifiables.
+
+**Approach:** écrire un générateur qui fabrique localement une archive PMTiles v3 valide, à tuiles PNG, couvrant la zone de Villeneuve-le-Roi aux zooms 10 à 14, et un vérificateur qui la relit avec la bibliothèque `pmtiles` officielle pour prouver sa conformité.
+
+## Boundaries & Constraints
+
+**Always:** le générateur ne fait aucune requête réseau et n'a aucune dépendance tierce — Node seul, avec son `zlib` intégré. L'archive produite respecte la spécification PMTiles v3 au point d'être relue par la bibliothèque `pmtiles` sans adaptation. Le fichier produit est versionné dans `/fixtures` (exception déjà posée dans `.gitignore`) et pèse moins de 200 Ko. Toute dépendance ajoutée voit sa licence écrite dans le message de commit (CLAUDE.md §8).
+
+**Never:** ne pas télécharger de données, même une seule tuile. Ne pas produire de tuiles vectorielles MVT : hors périmètre, et le PNG suffit à prouver l'affichage. Ne pas toucher à `app/src/basemap.style.json` ni à `app/src/main.ts` — le branchement dans la carte est l'entrée 1.3. Ne pas ajouter de dépendance Rust : rien de ceci n'entre dans le moteur.
+
+## I/O & Edge-Case Matrix
+
+| Scenario | Input / State | Expected Output / Behavior | Error Handling |
+|----------|--------------|---------------------------|----------------|
+| Génération nominale | `npm run fixtures` sur un dépôt propre | `fixtures/basemap-villeneuve-le-roi.pmtiles` écrit, en-tête de 127 octets, magie `PMTiles`, version 3 | Aucune erreur attendue |
+| Relecture par la bibliothèque | l'archive produite | `pmtiles` lit l'en-tête, les métadonnées et rend chaque tuile annoncée aux zooms 10 à 14 | Échec du script si une tuile annoncée est absente |
+| Identifiant de tuile connu | z=12, x=3423, y=1763 | l'identifiant calculé vaut exactement 19078479, valeur donnée par la spécification | Échec si l'écart est non nul |
+| Hors de l'emprise | une tuile en dehors de la boîte de Villeneuve-le-Roi | absente de l'archive, et la lecture renvoie « pas de tuile » sans planter | Retour vide, pas d'exception |
+| Zoom hors bornes | z=9 ou z=15 | absent de l'archive, bornes `min_zoom` et `max_zoom` de l'en-tête respectées | Retour vide |
+| Déterminisme | deux générations consécutives | deux fichiers identiques octet pour octet | Échec de la vérification si l'empreinte diffère |
+
+</frozen-after-approval>
+
+## Code Map
+
+- `app/package.json` -- ajouter `pmtiles` en `devDependencies` (BSD-3-Clause, v4.5.0, licence relevée le 2026-10-09) et deux scripts, `fixtures` et `verify:fixtures`. L'entrée 1.3 promouvra `pmtiles` en dépendance de production quand l'application l'importera.
+- `app/scripts/generate-basemap.mjs` -- **à créer.** Le générateur. Node pur, zéro dépendance tierce, `zlib` intégré pour le gzip du répertoire et `zlib.deflateSync` pour les PNG.
+- `app/scripts/verify-basemap.mjs` -- **à créer.** Le vérificateur, seul endroit qui importe `pmtiles`.
+- `app/scripts/check-offline.mjs` -- modèle à suivre pour le style : Node ESM, échec par `process.exit(1)` avec un message explicite. Ne pas le modifier.
+- `app/src-tauri/icons/` -- les icônes du dépôt ont été produites par un encodeur PNG minimal (IHDR, IDAT `zlib.deflate`, IEND, filtre 0, RGBA 8 bits) ; réutiliser cette approche pour les tuiles.
+- `fixtures/README.md` -- remplir le tableau des jeux de test avec le fichier produit, sa nature, son origine « généré » et son utilisateur, l'entrée 1.3.
+- `app/src/main.ts` et `app/src/basemap.style.json` -- **ne pas toucher**, c'est l'entrée 1.3.
+
+## Tasks & Acceptance
+
+**Execution:**
+- [ ] `app/package.json` -- ajouter `pmtiles` en `devDependencies` (BSD-3-Clause, v4.5.0) et les scripts `fixtures` et `verify:fixtures`, puis chaîner `verify:fixtures` dans `check` -- en premier, parce que le vérificateur de la tâche suivante importe ce paquet et que la recette doit protéger le fixture
+- [ ] `app/scripts/generate-basemap.mjs` -- écrire l'encodeur PMTiles v3 : varint little-endian, identifiant de tuile par courbe de Hilbert, répertoire racine, métadonnées JSON, section de tuiles ; puis l'encodeur PNG des tuiles et l'assemblage du fichier -- c'est le livrable de l'entrée
+- [ ] `app/scripts/verify-basemap.mjs` -- relire l'archive avec `pmtiles` et couvrir les six scénarios de la matrice d'entrées-sorties : en-tête, bornes de zoom, présence de chaque tuile annoncée, identifiant de référence de la spécification, tuile hors emprise, et déterminisme par double génération -- la preuve que l'archive est conforme, pas seulement qu'elle existe, et chaque commande vient avec son test (CLAUDE.md §8)
+- [ ] `fixtures/README.md` -- documenter le fichier produit, l'emprise, les zooms couverts et la commande qui le régénère -- sans ça le fixture est un binaire orphelin dans le dépôt
+
+**Acceptance Criteria:**
+- Given un dépôt fraîchement cloné et `npm install` fait, when on lance `npm run fixtures`, then `fixtures/basemap-villeneuve-le-roi.pmtiles` est créé et pèse moins de 200 Ko.
+- Given l'archive générée, when on lance `npm run check`, then la vérification passe au vert et affiche le nombre de tuiles et les zooms couverts.
+- Given l'archive générée, when on l'ouvre avec la bibliothèque `pmtiles`, then son en-tête annonce le type de tuile PNG, la compression interne gzip, les tuiles non compressées, et les zooms 10 à 14.
+- Given le générateur, when on l'exécute hors réseau, then il réussit — il ne fait aucune requête.
+- Given une tuile corrompue ou tronquée introduite à la main dans l'archive, when on lance la vérification, then elle échoue avec un code de sortie non nul.
+
+## Implementation Notes
+
+## Plan Change Log
+
+## Review Triage Log
+
+## Design Notes
+
+**Le piège de la spécification.** Le pseudocode de l'annexe A.1 de la spécification PMTiles v3 **omet le nombre d'entrées**, alors que le §4.2 l'exige comme première partie du répertoire et que la procédure de décodage du §4.3 le lit en premier. Le répertoire s'encode donc dans cet ordre : nombre d'entrées, puis les identifiants delta-encodés, puis les longueurs de série, puis les longueurs, puis les décalages — chaque valeur en varint little-endian.
+
+**Identifiant de tuile.** `tile_id = (4^z - 1) / 3 + hilbert(2^z, x, y)`. Le premier terme est le cumul des tuiles des zooms inférieurs ; le second est la position sur la courbe de Hilbert d'ordre `z`. L'ordre attendu au zoom 1 est `(0,0)=1, (0,1)=2, (1,1)=3, (1,0)=4`, ce qui correspond à l'algorithme `xy2d` classique à rotations. La valeur de référence de la spécification, `z=12, x=3423, y=1763 → 19078479`, sert de test et doit être vérifiée **avant** d'écrire la moindre tuile.
+
+**En-tête.** 127 octets : magie `PMTiles` sur 7 octets, version `3`, puis huit entiers non signés 64 bits little-endian (décalage et longueur du répertoire racine, des métadonnées, des répertoires feuilles, des tuiles), trois compteurs 64 bits, puis `clustered = 1`, compression interne `2` (gzip), compression des tuiles `1` (aucune, le PNG est déjà compressé), type de tuile `2` (PNG), zoom minimum et maximum, positions minimale, maximale et centrale sur 8 octets chacune — longitude puis latitude, chacune multipliée par 10 000 000 et écrite en entier signé 32 bits little-endian — et le zoom central.
+
+**Décalages contigus.** Les tuiles étant écrites dans l'ordre des identifiants, `clustered` vaut 1 et chaque décalage sauf le premier s'encode en `0` quand il suit exactement la tuile précédente. Un seul répertoire racine suffit : l'emprise retenue produit bien moins d'entrées que la limite de 16 257 octets compressés.
+
+**Emprise et zooms.** La boîte `[2.37, 48.71]`–`[2.46, 48.76]` est celle du repère déjà présent dans `app/src/basemap.style.json`, et la carte s'ouvre sur `[2.415, 48.735]` au zoom 12 d'après `app/src/main.ts`. Les zooms 10 à 14 encadrent donc l'ouverture par défaut. Au-delà de l'emprise, l'archive est vide, ce qui est assumé et documenté : c'est un fixture, pas un fond de production.
+
+**Contenu des tuiles.** Un damier avec bordure et diagonale suffit à voir que le déplacement et le zoom fonctionnent. Pas de texte : rendre des chiffres demanderait une police embarquée, hors périmètre.
+
+**Pourquoi Node et pas Rust.** Node est déjà requis pour construire l'interface, son `zlib` est intégré, et c'est le même interpréteur que la bibliothèque `pmtiles` qui sert de juge. Un générateur Rust imposerait des dépendances de compression au moteur, dont CLAUDE.md §3 exige qu'il reste sans dépendance.
+
+## Verification
+
+**Commands:**
+- `cd app && npm install` -- expected: `pmtiles` installé, aucune autre dépendance ajoutée
+- `cd app && npm run fixtures` -- expected: `fixtures/basemap-villeneuve-le-roi.pmtiles` écrit, taille affichée, moins de 200 Ko
+- `cd app && npm run check` -- expected: typage strict, garde hors ligne et vérification du fixture tous au vert
+- `cd app && npm run fixtures && sha256sum ../fixtures/*.pmtiles && npm run fixtures && sha256sum ../fixtures/*.pmtiles` -- expected: deux empreintes identiques (déterminisme)
+- `cargo test -p cassini-core && cargo clippy -p cassini-core --all-targets -- -D warnings && cargo fmt --all --check` -- expected: le moteur reste intact et vert
diff --git a/app/package-lock.json b/app/package-lock.json
index 4a69d2c..e523351 100644
--- a/app/package-lock.json
+++ b/app/package-lock.json
@@ -13,6 +13,7 @@
       },
       "devDependencies": {
         "@tauri-apps/cli": "^2.12.1",
+        "pmtiles": "^4.5.0",
         "typescript": "^5.6.0",
         "vite": "^5.4.0"
       }
@@ -1145,6 +1146,13 @@
         "@esbuild/win32-x64": "0.21.5"
       }
     },
+    "node_modules/fflate": {
+      "version": "0.8.3",
+      "resolved": "https://registry.npmjs.org/fflate/-/fflate-0.8.3.tgz",
+      "integrity": "sha512-tbZNuJrLwGUp3zshBtdy4W+ORxZuIh8a5ilyIEQDC5rY1f3U20JMry0Ll3WBzU58EZKsEuJFXhb5gwv8CsPvgA==",
+      "dev": true,
+      "license": "MIT"
+    },
     "node_modules/fsevents": {
       "version": "2.3.3",
       "resolved": "https://registry.npmjs.org/fsevents/-/fsevents-2.3.3.tgz",
@@ -1264,6 +1272,16 @@
       "dev": true,
       "license": "ISC"
     },
+    "node_modules/pmtiles": {
+      "version": "4.5.0",
+      "resolved": "https://registry.npmjs.org/pmtiles/-/pmtiles-4.5.0.tgz",
+      "integrity": "sha512-CBeD4SoUluFziGdy/8k7FOjQxQy486n+929W/tWophauvMICpkZ26vGBWhDt/6b1FZQWNaf4jFdT/5+q0Wii5w==",
+      "dev": true,
+      "license": "BSD-3-Clause",
+      "dependencies": {
+        "fflate": "^0.8.2"
+      }
+    },
     "node_modules/postcss": {
       "version": "8.5.29",
       "resolved": "https://registry.npmjs.org/postcss/-/postcss-8.5.29.tgz",
diff --git a/app/package.json b/app/package.json
index 190a4d9..c20377f 100644
--- a/app/package.json
+++ b/app/package.json
@@ -8,14 +8,17 @@
     "dev": "vite",
     "build": "tsc --noEmit && vite build",
     "preview": "vite preview",
-    "check": "tsc --noEmit && node scripts/check-offline.mjs",
-    "tauri": "tauri"
+    "check": "tsc --noEmit && node scripts/check-offline.mjs && node scripts/verify-basemap.mjs",
+    "tauri": "tauri",
+    "fixtures": "node scripts/generate-basemap.mjs",
+    "verify:fixtures": "node scripts/verify-basemap.mjs"
   },
   "dependencies": {
     "maplibre-gl": "^6.13.0"
   },
   "devDependencies": {
     "@tauri-apps/cli": "^2.12.1",
+    "pmtiles": "^4.5.0",
     "typescript": "^5.6.0",
     "vite": "^5.4.0"
   }
diff --git a/app/scripts/generate-basemap.mjs b/app/scripts/generate-basemap.mjs
new file mode 100644
index 0000000..de1d5eb
--- /dev/null
+++ b/app/scripts/generate-basemap.mjs
@@ -0,0 +1,299 @@
+// Générateur du fond de carte PMTiles de recette.
+//
+// Cassini ne télécharge aucune donnée (CLAUDE.md §2.4), donc le fond de carte
+// qui sert à la recette est **fabriqué** ici : une archive PMTiles v3 à tuiles
+// PNG couvrant la commune de Villeneuve-le-Roi, aux zooms 10 à 14.
+//
+// Aucune dépendance tierce : Node seul, avec son `zlib` intégré. Aucune requête
+// réseau. Le fichier produit est déterministe, octet pour octet.
+//
+// Spécification suivie : https://github.com/protomaps/PMTiles/blob/main/spec/v3/spec.md
+// Attention : le pseudocode de son annexe A.1 omet le nombre d'entrées en tête
+// du répertoire, alors que le §4.2 l'exige et que le décodage du §4.3 le lit en
+// premier. Voir `encoderRepertoire`.
+
+import { writeFileSync, mkdirSync } from "node:fs";
+import { dirname, join } from "node:path";
+import { fileURLToPath, pathToFileURL } from "node:url";
+import zlib from "node:zlib";
+
+const ICI = dirname(fileURLToPath(import.meta.url));
+export const CHEMIN_SORTIE = join(ICI, "..", "..", "fixtures", "basemap-villeneuve-le-roi.pmtiles");
+
+/** Emprise du repère déjà présent dans `app/src/basemap.style.json`. */
+export const EMPRISE = { ouest: 2.37, sud: 48.71, est: 2.46, nord: 48.76 };
+/** Centre d'ouverture de la carte, aligné sur `app/src/main.ts`. */
+export const CENTRE = { lon: 2.415, lat: 48.735 };
+export const ZOOM_MIN = 10;
+export const ZOOM_MAX = 14;
+export const ZOOM_CENTRE = 12;
+
+const COTE_TUILE = 256;
+const COTE_CELLULE = 64;
+
+// ---------------------------------------------------------------- varint
+
+/** Écrit un entier non signé en varint little-endian base 128. */
+function ecrireVarint(octets, valeur) {
+  let v = BigInt(valeur);
+  if (v < 0n) throw new Error(`varint négatif interdit : ${valeur}`);
+  while (v >= 128n) {
+    octets.push(Number((v & 127n) | 128n));
+    v >>= 7n;
+  }
+  octets.push(Number(v));
+}
+
+// ------------------------------------------------- identifiant de tuile
+
+function pivoter(n, x, y, rx, ry) {
+  if (ry === 0) {
+    if (rx === 1) {
+      x = n - 1 - x;
+      y = n - 1 - y;
+    }
+    return [y, x];
+  }
+  return [x, y];
+}
+
+/** Position de (x, y) sur la courbe de Hilbert d'ordre `n` = 2^z. */
+export function indiceHilbert(n, x, y) {
+  let d = 0;
+  for (let s = n / 2; s > 0; s = Math.floor(s / 2)) {
+    const rx = (x & s) > 0 ? 1 : 0;
+    const ry = (y & s) > 0 ? 1 : 0;
+    d += s * s * ((3 * rx) ^ ry);
+    [x, y] = pivoter(n, x, y, rx, ry);
+  }
+  return d;
+}
+
+/**
+ * Identifiant PMTiles d'une tuile : le cumul des tuiles des zooms inférieurs,
+ * plus la position sur la courbe de Hilbert du zoom courant.
+ *
+ * Valeur de référence de la spécification : (12, 3423, 1763) vaut 19078479.
+ */
+export function identifiantTuile(z, x, y) {
+  if (!Number.isInteger(z) || z < 0 || z > 26) throw new Error(`zoom hors bornes : ${z}`);
+  const n = 2 ** z;
+  if (!Number.isInteger(x) || x < 0 || x >= n) throw new Error(`x hors bornes au zoom ${z} : ${x}`);
+  if (!Number.isInteger(y) || y < 0 || y >= n) throw new Error(`y hors bornes au zoom ${z} : ${y}`);
+  return (4 ** z - 1) / 3 + indiceHilbert(n, x, y);
+}
+
+// --------------------------------------------------- emprise vers tuiles
+
+const lonVersX = (lon, z) => Math.floor(((lon + 180) / 360) * 2 ** z);
+
+const latVersY = (lat, z) => {
+  const r = (lat * Math.PI) / 180;
+  return Math.floor(((1 - Math.log(Math.tan(r) + 1 / Math.cos(r)) / Math.PI) / 2) * 2 ** z);
+};
+
+/** Les tuiles couvrant l'emprise, du zoom minimum au maximum, triées par identifiant. */
+export function tuilesCouvrantes() {
+  const liste = [];
+  for (let z = ZOOM_MIN; z <= ZOOM_MAX; z += 1) {
+    const n = 2 ** z;
+    const x0 = Math.max(0, lonVersX(EMPRISE.ouest, z));
+    const x1 = Math.min(n - 1, lonVersX(EMPRISE.est, z));
+    // Le nord donne le plus petit y : l'axe des tuiles descend.
+    const y0 = Math.max(0, latVersY(EMPRISE.nord, z));
+    const y1 = Math.min(n - 1, latVersY(EMPRISE.sud, z));
+    for (let x = x0; x <= x1; x += 1) {
+      for (let y = y0; y <= y1; y += 1) {
+        liste.push({ z, x, y, id: identifiantTuile(z, x, y) });
+      }
+    }
+  }
+  liste.sort((a, b) => a.id - b.id);
+  return liste;
+}
+
+// -------------------------------------------------------------------- PNG
+
+function morceauPng(type, donnees) {
+  const prefixe = Buffer.alloc(8);
+  prefixe.writeUInt32BE(donnees.length, 0);
+  prefixe.write(type, 4, "ascii");
+  const crc = Buffer.alloc(4);
+  // Le CRC couvre le type et les données, pas la longueur.
+  crc.writeUInt32BE(zlib.crc32(Buffer.concat([prefixe.subarray(4), donnees])), 0);
+  return Buffer.concat([prefixe, donnees, crc]);
+}
+
+/**
+ * Une tuile : damier en niveaux de gris avec bordure claire. Le damier change
+ * de phase selon (x + y) pour que deux tuiles voisines s'inversent, ce qui rend
+ * le déplacement et l'alignement des coutures visibles à l'œil.
+ *
+ * Gris + alpha (type de couleur PNG 4) plutôt que RVBA : deux fois moins
+ * d'octets pour un rendu identique ici, ce qui tient le fixture sous 200 Ko.
+ */
+export function tuilePng(x, y) {
+  const phase = (x + y) % 2;
+  const lignes = [];
+  for (let py = 0; py < COTE_TUILE; py += 1) {
+    const ligne = Buffer.alloc(1 + COTE_TUILE * 2); // octet de filtre 0, puis (gris, alpha)
+    let o = 1;
+    for (let px = 0; px < COTE_TUILE; px += 1) {
+      const damier = (Math.floor(px / COTE_CELLULE) + Math.floor(py / COTE_CELLULE) + phase) % 2 === 0;
+      const bord = px < 2 || py < 2 || px > COTE_TUILE - 3 || py > COTE_TUILE - 3;
+      ligne[o] = bord ? 190 : damier ? 30 : 46;
+      ligne[o + 1] = 255;
+      o += 2;
+    }
+    lignes.push(ligne);
+  }
+  const ihdr = Buffer.alloc(13);
+  ihdr.writeUInt32BE(COTE_TUILE, 0);
+  ihdr.writeUInt32BE(COTE_TUILE, 4);
+  ihdr[8] = 8; // profondeur de bit
+  ihdr[9] = 4; // type de couleur : gris + alpha
+  return Buffer.concat([
+    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
+    morceauPng("IHDR", ihdr),
+    morceauPng("IDAT", zlib.deflateSync(Buffer.concat(lignes), { level: 9 })),
+    morceauPng("IEND", Buffer.alloc(0)),
+  ]);
+}
+
+// ------------------------------------------------------------- répertoire
+
+/**
+ * Encode un répertoire : nombre d'entrées, identifiants delta-encodés,
+ * longueurs de série, longueurs, puis décalages — chacun en varint.
+ *
+ * Le nombre d'entrées est exigé par le §4.2 et lu en premier par le §4.3, mais
+ * absent du pseudocode de l'annexe A.1. L'oublier produit une archive illisible.
+ */
+export function encoderRepertoire(entrees) {
+  if (entrees.length === 0) throw new Error("un répertoire doit contenir au moins une entrée");
+  const octets = [];
+  ecrireVarint(octets, entrees.length);
+  let dernierId = 0;
+  for (const e of entrees) {
+    ecrireVarint(octets, e.id - dernierId);
+    dernierId = e.id;
+  }
+  for (const e of entrees) ecrireVarint(octets, e.serie);
+  for (const e of entrees) ecrireVarint(octets, e.longueur);
+  let octetSuivant = 0;
+  for (const [i, e] of entrees.entries()) {
+    // Un décalage contigu avec la tuile précédente s'écrit 0 ; sinon décalage + 1.
+    if (i > 0 && e.decalage === octetSuivant) ecrireVarint(octets, 0);
+    else ecrireVarint(octets, e.decalage + 1);
+    octetSuivant = e.decalage + e.longueur;
+  }
+  return Buffer.from(octets);
+}
+
+// ---------------------------------------------------------------- en-tête
+
+function ecrirePosition(tampon, decalage, lon, lat) {
+  tampon.writeInt32LE(Math.round(lon * 1e7), decalage);
+  tampon.writeInt32LE(Math.round(lat * 1e7), decalage + 4);
+}
+
+/** L'en-tête de 127 octets, disposition du §3.1 de la spécification. */
+export function encoderEntete(c) {
+  const h = Buffer.alloc(127);
+  h.write("PMTiles", 0, "ascii");
+  h[7] = 3;
+  const u64 = [
+    c.decalageRepertoireRacine, c.longueurRepertoireRacine,
+    c.decalageMetadonnees, c.longueurMetadonnees,
+    c.decalageRepertoiresFeuilles, c.longueurRepertoiresFeuilles,
+    c.decalageTuiles, c.longueurTuiles,
+    c.nbTuilesAdressees, c.nbEntreesTuiles, c.nbContenusTuiles,
+  ];
+  u64.forEach((v, i) => h.writeBigUInt64LE(BigInt(v), 8 + i * 8));
+  h[96] = 1; // groupé par identifiant
+  h[97] = 2; // compression interne : gzip
+  h[98] = 1; // compression des tuiles : aucune, le PNG est déjà compressé
+  h[99] = 2; // type de tuile : PNG
+  h[100] = ZOOM_MIN;
+  h[101] = ZOOM_MAX;
+  ecrirePosition(h, 102, EMPRISE.ouest, EMPRISE.sud);
+  ecrirePosition(h, 110, EMPRISE.est, EMPRISE.nord);
+  h[118] = ZOOM_CENTRE;
+  ecrirePosition(h, 119, CENTRE.lon, CENTRE.lat);
+  return h;
+}
+
+// -------------------------------------------------------------- assemblage
+
+export function construireArchive() {
+  const tuiles = tuilesCouvrantes();
+  const blobs = [];
+  const entrees = [];
+  let decalage = 0;
+  for (const t of tuiles) {
+    const blob = tuilePng(t.x, t.y);
+    blobs.push(blob);
+    entrees.push({ id: t.id, serie: 1, longueur: blob.length, decalage });
+    decalage += blob.length;
+  }
+  const donneesTuiles = Buffer.concat(blobs);
+
+  const metadonnees = zlib.gzipSync(
+    Buffer.from(
+      JSON.stringify({
+        name: "Cassini — fond de recette, Villeneuve-le-Roi",
+        description:
+          "Fond de carte fabrique localement pour la recette. Aucune donnee reelle, aucun telechargement.",
+        attribution: "Cassini, jeu de test genere",
+        type: "baselayer",
+        format: "png",
+        generated_by: "app/scripts/generate-basemap.mjs",
+      }),
+    ),
+    { level: 9 },
+  );
+  const repertoire = zlib.gzipSync(encoderRepertoire(entrees), { level: 9 });
+
+  const decalageRepertoireRacine = 127;
+  const decalageMetadonnees = decalageRepertoireRacine + repertoire.length;
+  const decalageTuiles = decalageMetadonnees + metadonnees.length;
+
+  if (decalageMetadonnees > 16384) {
+    throw new Error(
+      `le répertoire racine dépasse les 16 384 premiers octets (${decalageMetadonnees}) : il faudrait des répertoires feuilles`,
+    );
+  }
+
+  const entete = encoderEntete({
+    decalageRepertoireRacine,
+    longueurRepertoireRacine: repertoire.length,
+    decalageMetadonnees,
+    longueurMetadonnees: metadonnees.length,
+    decalageRepertoiresFeuilles: decalageTuiles,
+    longueurRepertoiresFeuilles: 0,
+    decalageTuiles,
+    longueurTuiles: donneesTuiles.length,
+    nbTuilesAdressees: entrees.length,
+    nbEntreesTuiles: entrees.length,
+    nbContenusTuiles: entrees.length,
+  });
+
+  return { archive: Buffer.concat([entete, repertoire, metadonnees, donneesTuiles]), tuiles };
+}
+
+function principal() {
+  const { archive, tuiles } = construireArchive();
+  mkdirSync(dirname(CHEMIN_SORTIE), { recursive: true });
+  writeFileSync(CHEMIN_SORTIE, archive);
+  const parZoom = {};
+  for (const t of tuiles) parZoom[t.z] = (parZoom[t.z] ?? 0) + 1;
+  console.log(`Fond de recette écrit : ${CHEMIN_SORTIE}`);
+  console.log(
+    `  ${tuiles.length} tuiles, zooms ${ZOOM_MIN} à ${ZOOM_MAX} (${Object.entries(parZoom)
+      .map(([z, n]) => `z${z}:${n}`)
+      .join(", ")})`,
+  );
+  console.log(`  ${(archive.length / 1024).toFixed(1)} Ko, aucune requête réseau`);
+}
+
+if (import.meta.url === pathToFileURL(process.argv[1] ?? "").href) principal();
diff --git a/app/scripts/verify-basemap.mjs b/app/scripts/verify-basemap.mjs
new file mode 100644
index 0000000..129050f
--- /dev/null
+++ b/app/scripts/verify-basemap.mjs
@@ -0,0 +1,224 @@
+// Vérificateur du fond de carte PMTiles de recette.
+//
+// Le générateur écrit un format binaire à la main : « le fichier existe » ne
+// prouve rien. Ici l'archive est relue par la bibliothèque `pmtiles` officielle,
+// qui sert de juge indépendant, et chaque scénario de la matrice du plan de la
+// story 1.2 est couvert.
+//
+// Exécuté par `npm run check`, donc une archive cassée fait échouer la recette.
+
+import { readFileSync, existsSync } from "node:fs";
+import zlib from "node:zlib";
+import { PMTiles, Compression, TileType, zxyToTileId } from "pmtiles";
+import {
+  CHEMIN_SORTIE,
+  EMPRISE,
+  ZOOM_MIN,
+  ZOOM_MAX,
+  identifiantTuile,
+  tuilesCouvrantes,
+  construireArchive,
+} from "./generate-basemap.mjs";
+
+const SIGNATURE_PNG = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
+
+/**
+ * Valide un PNG en parcourant ses morceaux et en recalculant leur CRC, puis en
+ * décompressant l'IDAT. Se contenter de la signature et d'`IEND` laisserait
+ * passer un octet corrompu au milieu des données : c'est arrivé pendant
+ * l'écriture de cette story.
+ */
+function pngValide(octets) {
+  if (!octets.subarray(0, 8).equals(SIGNATURE_PNG)) return "signature absente";
+  let i = 8;
+  const vus = [];
+  const idat = [];
+  while (i + 12 <= octets.length) {
+    const longueur = octets.readUInt32BE(i);
+    const type = octets.subarray(i + 4, i + 8).toString("ascii");
+    const fin = i + 8 + longueur;
+    if (fin + 4 > octets.length) return `morceau ${type} tronqué`;
+    const donnees = octets.subarray(i + 8, fin);
+    const crcLu = octets.readUInt32BE(fin);
+    const crcCalcule = zlib.crc32(Buffer.concat([octets.subarray(i + 4, i + 8), donnees]));
+    if (crcLu !== crcCalcule) return `CRC du morceau ${type} invalide`;
+    vus.push(type);
+    if (type === "IDAT") idat.push(donnees);
+    i = fin + 4;
+    if (type === "IEND") break;
+  }
+  if (i !== octets.length) return "octets en trop après IEND";
+  for (const attendu of ["IHDR", "IDAT", "IEND"]) {
+    if (!vus.includes(attendu)) return `morceau ${attendu} manquant`;
+  }
+  try {
+    zlib.inflateSync(Buffer.concat(idat));
+  } catch {
+    return "IDAT ne se décompresse pas";
+  }
+  return null;
+}
+
+let echecs = 0;
+const resultats = [];
+
+function verifier(nom, condition, detail = "") {
+  if (condition) {
+    resultats.push(`  ok   ${nom}`);
+  } else {
+    echecs += 1;
+    resultats.push(`  ÉCHEC ${nom}${detail ? ` — ${detail}` : ""}`);
+  }
+}
+
+/** Source PMTiles lisant un tampon en mémoire : aucun accès réseau, aucun fichier temporaire. */
+class SourceTampon {
+  constructor(tampon, cle) {
+    this.tampon = tampon;
+    this.cle = cle;
+  }
+
+  getKey() {
+    return this.cle;
+  }
+
+  async getBytes(decalage, longueur) {
+    const tranche = this.tampon.subarray(decalage, decalage + longueur);
+    return {
+      data: tranche.buffer.slice(tranche.byteOffset, tranche.byteOffset + tranche.byteLength),
+    };
+  }
+}
+
+/** Décompression explicite par `zlib`, pour ne pas dépendre des globales de la plateforme. */
+async function decompresser(donnees, compression) {
+  if (compression === Compression.None) return donnees;
+  if (compression === Compression.Gzip) {
+    return new Uint8Array(zlib.gunzipSync(Buffer.from(donnees)));
+  }
+  throw new Error(`compression non gérée dans la vérification : ${compression}`);
+}
+
+async function principal() {
+  // --- Scénario : génération nominale -------------------------------------
+  if (!existsSync(CHEMIN_SORTIE)) {
+    console.error(`Vérification du fond de recette : ÉCHEC, fichier absent.\n  Lance d'abord : npm run fixtures`);
+    process.exit(1);
+  }
+  const archive = readFileSync(CHEMIN_SORTIE);
+  verifier("l'archive dépasse 127 octets", archive.length > 127, `${archive.length} octets`);
+  verifier("la magie vaut PMTiles", archive.subarray(0, 7).toString("ascii") === "PMTiles");
+  verifier("la version de spécification vaut 3", archive[7] === 3, `lu : ${archive[7]}`);
+  verifier("l'archive pèse moins de 200 Ko", archive.length < 200 * 1024, `${(archive.length / 1024).toFixed(1)} Ko`);
+
+  // --- Scénario : relecture par la bibliothèque ---------------------------
+  const pmtiles = new PMTiles(new SourceTampon(archive, CHEMIN_SORTIE), undefined, decompresser);
+  const entete = await pmtiles.getHeader();
+  verifier("la bibliothèque lit l'en-tête", entete.specVersion === 3);
+  verifier(
+    "l'en-tête occupe exactement 127 octets",
+    entete.rootDirectoryOffset === 127,
+    `le répertoire racine commence à ${entete.rootDirectoryOffset}`,
+  );
+  verifier("le type de tuile est PNG", entete.tileType === TileType.Png, `lu : ${entete.tileType}`);
+  verifier("la compression interne est gzip", entete.internalCompression === Compression.Gzip);
+  verifier("les tuiles ne sont pas compressées", entete.tileCompression === Compression.None);
+  verifier("l'archive est groupée par identifiant", entete.clustered === true);
+
+  // --- Scénario : zoom hors bornes ---------------------------------------
+  verifier("le zoom minimum annoncé est correct", entete.minZoom === ZOOM_MIN, `lu : ${entete.minZoom}`);
+  verifier("le zoom maximum annoncé est correct", entete.maxZoom === ZOOM_MAX, `lu : ${entete.maxZoom}`);
+  const sousBorne = await pmtiles.getZxy(ZOOM_MIN - 1, 0, 0);
+  verifier("une tuile sous le zoom minimum est absente sans exception", sousBorne === undefined);
+  const surBorne = await pmtiles.getZxy(ZOOM_MAX + 1, 0, 0);
+  verifier("une tuile au-delà du zoom maximum est absente sans exception", surBorne === undefined);
+
+  // --- Emprise annoncée ---------------------------------------------------
+  const proche = (a, b) => Math.abs(a - b) < 1e-6;
+  verifier(
+    "l'emprise annoncée est celle du repère de la carte",
+    proche(entete.minLon, EMPRISE.ouest) &&
+      proche(entete.minLat, EMPRISE.sud) &&
+      proche(entete.maxLon, EMPRISE.est) &&
+      proche(entete.maxLat, EMPRISE.nord),
+    `lu : ${entete.minLon},${entete.minLat} → ${entete.maxLon},${entete.maxLat}`,
+  );
+
+  // --- Métadonnées --------------------------------------------------------
+  const metadonnees = await pmtiles.getMetadata();
+  verifier("les métadonnées se relisent et annoncent le format png", metadonnees?.format === "png");
+
+  // --- Scénario : identifiant de tuile connu -----------------------------
+  verifier(
+    "l'identifiant de référence de la spécification est retrouvé",
+    identifiantTuile(12, 3423, 1763) === 19078479,
+    `calculé : ${identifiantTuile(12, 3423, 1763)}`,
+  );
+  const tuiles = tuilesCouvrantes();
+  const desaccords = tuiles.filter((t) => identifiantTuile(t.z, t.x, t.y) !== zxyToTileId(t.z, t.x, t.y));
+  verifier(
+    "notre calcul d'identifiant concorde avec celui de la bibliothèque sur les 35 tuiles",
+    desaccords.length === 0,
+    desaccords.length > 0 ? `${desaccords.length} désaccords, dont z${desaccords[0].z}/${desaccords[0].x}/${desaccords[0].y}` : "",
+  );
+  verifier(
+    "le nombre de tuiles annoncé par l'en-tête correspond",
+    entete.numAddressedTiles === tuiles.length,
+    `en-tête : ${entete.numAddressedTiles}, attendu : ${tuiles.length}`,
+  );
+
+  // --- Chaque tuile annoncée est servie et est un PNG valide -------------
+  let servies = 0;
+  let malFormees = 0;
+  let premierProbleme = null;
+  for (const t of tuiles) {
+    const reponse = await pmtiles.getZxy(t.z, t.x, t.y);
+    if (!reponse) continue;
+    servies += 1;
+    const octets = Buffer.from(reponse.data);
+    const probleme = pngValide(octets);
+    if (probleme) {
+      malFormees += 1;
+      if (!premierProbleme) premierProbleme = `z${t.z}/${t.x}/${t.y} : ${probleme}`;
+    }
+  }
+  verifier("chaque tuile annoncée est servie", servies === tuiles.length, `${servies} sur ${tuiles.length}`);
+  verifier(
+    "chaque tuile servie est un PNG valide, CRC et IDAT compris",
+    malFormees === 0,
+    `${malFormees} mal formées${premierProbleme ? `, dont ${premierProbleme}` : ""}`,
+  );
+
+  // --- Scénario : hors de l'emprise --------------------------------------
+  const horsEmprise = await pmtiles.getZxy(12, 0, 0);
+  verifier("une tuile hors de l'emprise est absente sans exception", horsEmprise === undefined);
+
+  // --- Scénario : déterminisme -------------------------------------------
+  const refaite = construireArchive().archive;
+  verifier(
+    "deux générations donnent le même fichier, octet pour octet",
+    refaite.equals(archive),
+    `${refaite.length} contre ${archive.length} octets`,
+  );
+
+  // --- Rapport ------------------------------------------------------------
+  const parZoom = {};
+  for (const t of tuiles) parZoom[t.z] = (parZoom[t.z] ?? 0) + 1;
+  console.log(
+    echecs === 0
+      ? `Vérification du fond de recette : OK, ${resultats.length} contrôles.`
+      : `Vérification du fond de recette : ÉCHEC, ${echecs} contrôle(s) sur ${resultats.length}.`,
+  );
+  for (const l of resultats) if (echecs > 0 || l.startsWith("  ÉCHEC")) console.log(l);
+  console.log(
+    `  ${tuiles.length} tuiles, zooms ${ZOOM_MIN} à ${ZOOM_MAX} (${Object.entries(parZoom)
+      .map(([z, n]) => `z${z}:${n}`)
+      .join(", ")}), ${(archive.length / 1024).toFixed(1)} Ko`,
+  );
+  if (echecs > 0) process.exit(1);
+}
+
+principal().catch((e) => {
+  console.error(`Vérification du fond de recette : ÉCHEC, exception inattendue.\n  ${e.message}`);
+  process.exit(1);
+});
diff --git a/fixtures/README.md b/fixtures/README.md
index 6f5496b..1917ff4 100644
--- a/fixtures/README.md
+++ b/fixtures/README.md
@@ -13,7 +13,39 @@ avec une exception pour ce dossier.
 
 | Fichier | Nature | Origine | Utilisé par |
 |---|---|---|---|
-| _(vide pour l'instant)_ | — | — | — |
+| `basemap-villeneuve-le-roi.pmtiles` | Archive PMTiles v3, tuiles PNG en niveaux de gris, 35 tuiles, 26 Ko | **Généré**, aucun téléchargement : `app/scripts/generate-basemap.mjs` | L'affichage du fond de carte hors ligne (entrée 1.3), et la recette |
 
-Le fond de carte PMTiles de test arrive à la tâche 2 de l'étape 1 : il sera
-**généré** par un script, pas téléchargé.
+## Le fond de carte de recette
+
+C'est un damier, pas une carte. Il sert à prouver trois choses : que l'archive
+est conforme à la spécification PMTiles v3, que MapLibre sait l'afficher depuis
+un fichier local, et qu'aucune requête réseau n'est nécessaire pour voir une
+carte à l'écran.
+
+- **Emprise** : `2.37, 48.71` → `2.46, 48.76`, soit la commune de
+  Villeneuve-le-Roi, la même boîte que le repère de `app/src/basemap.style.json`.
+- **Zooms** : 10 à 14. En dehors, l'archive est vide — c'est assumé. La carte
+  s'ouvre au zoom 12 sur `2.415, 48.735`.
+- **Hors de l'emprise** : aucune tuile. Un lecteur doit répondre « pas de
+  tuile », pas planter.
+
+### Régénérer
+
+```bash
+cd app
+npm run fixtures          # réécrit le .pmtiles
+npm run verify:fixtures   # le relit avec la bibliothèque pmtiles et le valide
+```
+
+La génération est **déterministe** : deux exécutions produisent le même fichier,
+octet pour octet, ce que la vérification contrôle. `npm run check` lance la
+vérification, donc une archive cassée fait échouer la recette.
+
+### Ce que la vérification contrôle
+
+22 contrôles, dont : la magie et la version de l'en-tête, le type de tuile, les
+compressions, les bornes de zoom et l'emprise annoncées, la relecture des
+métadonnées, la concordance de notre calcul d'identifiant de tuile avec celui de
+la bibliothèque sur les 35 tuiles, la valeur de référence donnée par la
+spécification (`z12/3423/1763 → 19078479`), la validité PNG de chaque tuile avec
+ses CRC, l'absence de tuile hors emprise et hors zooms, et le déterminisme.
diff --git a/fixtures/basemap-villeneuve-le-roi.pmtiles b/fixtures/basemap-villeneuve-le-roi.pmtiles
new file mode 100644
index 0000000..44a82cf
Binary files /dev/null and b/fixtures/basemap-villeneuve-le-roi.pmtiles differ

````
