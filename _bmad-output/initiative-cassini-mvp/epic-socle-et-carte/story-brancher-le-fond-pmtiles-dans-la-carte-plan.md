---
title: 'Brancher le fond PMTiles dans la carte'
type: 'feature'
ticket: '3'
created: '2026-10-10'
status: 'built'
baseline_revision: '3bbd816ed6b4683098601480213c7194169baaff'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la carte n'affiche qu'un polygone repère sur fond uni. L'archive PMTiles de l'entrée 1.2 existe dans `/fixtures` mais rien ne la lit, et la garde hors ligne rejette toute chaîne contenant `://` — donc le branchement la ferait échouer telle quelle.

**Approach:** servir l'archive au webview par un appel IPC Rust gardé par `cassini_core::BasemapSource`, consommé par une implémentation locale de l'interface `Source` de `pmtiles` ; puis étendre la garde hors ligne pour n'autoriser que la forme `pmtiles://<clé locale attendue>` et continuer de rejeter le réseau, URL distante imbriquée comprise.

## Boundaries & Constraints

**Always:** aucune requête réseau — le chargement passe par l'IPC Tauri, déjà autorisé par le CSP existant, jamais par `fetch`. Le chemin du fichier est validé côté Rust par `BasemapSource::depuis_chemin_local`, qui devient la garde réelle du chemin de chargement au lieu d'un verrou inutilisé. La clé de la source du style est identique octet pour octet à ce que renvoie `getKey()`. Le polygone `repere` reste dans le style : `verify-basemap.mjs` le lit pour contrôler l'emprise. La licence de toute dépendance promue va dans le message de commit (CLAUDE.md §8).

**Never:** pas de protocole `asset` de Tauri, pas d'origine HTTP ajoutée au CSP. Pas de `new Protocol({ metadata: true })` : la bibliothèque y documente une requête HTTP supplémentaire. Pas de `setRTLTextPlugin`, pas de `glyphs`, `sprite` ni couche `symbol`. Ne pas modifier le format de l'archive ni l'encodeur de `generate-basemap.mjs`. Pas de registre de commandes : c'est l'étape 2.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Affichage nominal | application lancée, archive en place, machine sans réseau | le damier s'affiche aux zooms 10 à 14, `#etat` indique « Carte prête — hors ligne » | aucune erreur attendue |
| Lecture de plage | `offset` et `longueur` dans les bornes du fichier | exactement `longueur` octets renvoyés, bruts | — |
| Plage hors fichier | `offset + longueur` dépasse la fin | erreur nommant décalage, longueur et taille du fichier | erreur, jamais de troncature silencieuse ni de panique |
| Tuile absente | tuile hors emprise, ou zoom hors de 10–14 | la carte ne dessine rien à cet endroit | retour vide, aucune exception, `#etat` inchangé |
| Archive absente | fichier renommé ou supprimé au démarrage | la fenêtre s'ouvre, `#etat` nomme le fichier manquant | `BasemapError::FichierAbsent` remontée lisible |
| Garde — URL imbriquée | style portant `pmtiles://https://exemple.fr/f.pmtiles` | la garde échoue en nommant la faute | code de sortie non nul |
| Garde — clé inconnue | style portant `pmtiles://une-autre-cle` | la garde échoue : cette clé deviendrait un `FetchSource` | code de sortie non nul |
| Garde — code TypeScript | adresse réseau introduite dans `app/src/*.ts` | la garde échoue | code de sortie non nul |

## Decisions

- **L'archive de recette est embarquée dans l'installeur** en ressource Tauri — décidé par le propriétaire le 2026-10-10. Motif : la carte affiche un fond dès la première ouverture et l'entrée 1.4 devient testable sans manipulation ; l'archive est un damier synthétique produit par le dépôt, sans aucune donnée géographique tierce, donc CLAUDE.md §2.4 n'est pas engagé. L'application résout ce chemin par le répertoire de ressources, avec repli sur le fichier du dépôt en développement.
- **Plan conservé entier** malgré ses ~3 800 tokens, décidé par le propriétaire le 2026-10-10 : la longueur porte des faits d'investigation, et le périmètre reste une seule entrée de ticket.

</frozen-after-approval>

## Code Map

- `app/src/main.ts:5,15,23,38` -- style importé en JSON puis casté ; carte créée sur `[2.415,48.735]` zoom 12 ; `on("error")` est le seul chemin d'erreur existant. Enregistrer le protocole **avant** la création de la carte.
- `app/src/basemap.style.json` -- `version: 8`, source `repere` (GeoJSON en ligne), couches `fond`, `repere-surface`, `repere-contour` ; aucun `glyphs` ni `sprite`. Ajouter une source `raster` et sa couche **sous** `repere-surface`.
- `app/scripts/check-offline.mjs:17-40` -- `parcourir()` rejette toute chaîne contenant `://` ou commençant par `//` ; lignes 38-40 contrôlent `glyphs`/`sprite`. À extraire en fonction pure exportée, puis étendre.
- `core/src/basemap.rs:50,66,75` -- `depuis_chemin_local` (refuse vide, `://`, `//`, fichier absent), `chemin()`, `designe_le_reseau` ; `BasemapError` à trois variantes ; 5 tests en place, à suivre comme modèle.
- `app/src-tauri/src/lib.rs:8-18` -- unique commande `version_du_moteur` dans `invoke_handler`, aucun plugin enregistré.
- `app/src-tauri/tauri.conf.json` -- `app.security.csp` porte déjà `ipc:` et `http://ipc.localhost` : **ne pas y toucher**. `bundle` n'a pas de `resources`.
- `app/package.json` -- `pmtiles ^4.5.0` en `devDependencies` (BSD-3-Clause) ; `check` enchaîne `tsc --noEmit`, `check-offline.mjs`, `verify-basemap.mjs`.
- `app/scripts/generate-basemap.mjs:21-32` -- `CHEMIN_SORTIE`, `EMPRISE`, `CENTRE`, `ZOOM_MIN`, `ZOOM_MAX`, `ZOOM_CENTRE`, `COTE_TUILE = 256`. Lire ces constantes, ne rien y écrire.
- `app/node_modules/pmtiles/dist/esm/index.js` -- `Protocol.tilev4` prend `url.substr(10)` comme clé et, si `tiles.get(clé)` est vide, construit `new PMTiles(clé)` donc un `FetchSource` **qui part sur le réseau**. `Source` = `{ getBytes(offset, length, signal?, etag?): Promise<{data: ArrayBuffer}>, getKey(): string }`.
- `app/tsconfig.json` -- `strict`, plus `noUncheckedIndexedAccess` et `exactOptionalPropertyTypes` : l'implémentation de `Source` doit déclarer ses paramètres optionnels exactement comme l'interface.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/basemap.rs` -- ajouter `BasemapSource::lire_plage(offset, longueur)` avec une variante d'erreur pour une plage hors fichier, et ses tests -- le moteur garde la lecture du disque, l'interface n'y touche jamais (CLAUDE.md §2.5)
- [x] `app/src-tauri/src/lib.rs` -- exposer une commande qui résout le chemin de l'archive, le valide par `depuis_chemin_local`, et renvoie les octets en `tauri::ipc::Response` -- `Response` traverse l'IPC en binaire, là où un `Vec<u8>` serait sérialisé en tableau JSON
- [x] `app/src/basemap.source.ts` -- **à créer.** Implémenter `Source` de `pmtiles` au-dessus de `invoke`, avec la clé littérale partagée avec le style exportée depuis ce seul endroit
- [x] `app/src/main.ts` -- enregistrer le protocole et la source avant la création de la carte, et faire remonter dans `#etat` un échec de lecture de l'archive
- [x] `app/src/basemap.style.json` -- ajouter la source `raster` en `pmtiles://`, `tileSize: 256`, bornes de zoom 10–14, et sa couche sous le repère
- [x] `app/scripts/check-offline.mjs` -- extraire l'analyse en fonction pure, n'autoriser que la clé attendue derrière `pmtiles://`, rejeter URL imbriquée et clé inconnue, étendre le crible à `app/src/*.ts`, et ajouter des cas négatifs en mémoire qui mettent la garde en échec exprès -- sans ça rien ne prouve qu'elle sache échouer (constat V1 de l'entrée 1.2)
- [x] `app/scripts/verify-protocole.mjs` -- **à créer.** Piloter le vrai `Protocol` de `pmtiles` avec une `Source` Node lisant le fichier, et couvrir les lignes de la matrice vérifiables sans fenêtre : TileJSON issu de l'en-tête, tuile connue rendue en PNG, tuile hors emprise vide, plage hors fichier refusée -- la fenêtre Tauri ne peut pas s'ouvrir dans cet environnement, c'est l'entrée 1.4 ; ce script est la preuve du branchement d'ici là
- [x] `app/package.json` -- promouvoir `pmtiles` en `dependencies` (BSD-3-Clause 4.5.0, transitif `fflate` MIT) et chaîner `verify-protocole.mjs` dans `check`
- [x] `app/src-tauri/tauri.conf.json`, `fixtures/README.md`, `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- déclarer l'archive en ressource de l'installeur, documenter d'où l'application la lit, et clore l'entrée reportée A10 tranchée par l'investigation

**Acceptance Criteria:**
- Given `npm run check` sur un dépôt propre, when la recette s'exécute, then typage strict, garde hors ligne avec ses cas négatifs, vérification du fixture et vérification du protocole passent toutes au vert.
- Given `cargo test -p cassini-core && cargo clippy -p cassini-core --all-targets -- -D warnings && cargo fmt --all --check`, when la recette Rust s'exécute, then elle est verte et la lecture de plage est couverte.
- Given le diff complet de l'entrée, when on le relit, then `app/src-tauri/tauri.conf.json` ne contient aucune origine réseau nouvelle dans `csp` et aucune clé `assetProtocol`.
- Given l'application construite, when on cherche d'où vient l'archive, then un seul endroit du code porte son chemin et un seul endroit porte sa clé.

## Implementation Notes

**Ce que le style déclare, et ce que MapLibre en fait.** La source est de la
forme `url: "pmtiles://<clé>"`, et non `tiles: [...]`. Vérifié dans
`maplibre-gl-dev.mjs` : `RasterTileSource` ne retient de son style que `url`,
`scheme` et `tileSize`, puis `loadTileJson` calcule
`pick(extend(tileJSON, options), [...])` — donc les options du style **écrasent**
le TileJSON. `tiles` et `bounds` viennent de l'en-tête de l'archive (constat
A10), tandis que `tileSize: 256` et les bornes 10–14 écrites dans le style sont
celles qui s'appliquent. Les deux sont donc cohérentes et toutes deux utiles.

**Le chemin du rendu raster.** `makeRequest` dirige toute adresse non
`http(s):`/`file:` vers `config.REGISTERED_PROTOCOLS`, sur le fil principal, puis
`doImageRequest` convertit la réponse par `arrayBufferToImageBitmap`. La
`Uint8Array` que renvoie `Protocol.tilev4` passe donc telle quelle, et une tuile
absente (`data: null`) est traitée comme une absence, pas comme une erreur. Rien
à ajouter au CSP : les octets ne transitent par aucune URL d'image.

**Piège désamorcé.** `getHeaderAndRoot` de la bibliothèque demande toujours les
16 384 premiers octets, sans regarder la taille du fichier. Comme le moteur
refuse une plage hors fichier plutôt que de la tronquer, une archive plus courte
que 16 Ko ne se chargerait pas du tout. `verify-protocole.mjs` contrôle donc
explicitement cette taille plancher ; l'archive actuelle fait 26 316 octets.

**Un seul endroit par chose, vérifié par la recette.** La clé vit dans
`app/src/basemap.source.ts` (`CLE_DU_FOND`) ; `check-offline.mjs` la relit par
expression régulière plutôt que de la redéclarer, parce qu'un script Node ne peut
pas importer un module TypeScript. Le chemin vit dans `app/src-tauri/src/lib.rs`
(`CHEMIN_DE_LARCHIVE`), choisi pour servir à la fois de cible de ressource et de
repli de développement. `verify-protocole.mjs` contrôle que ces valeurs
s'accordent avec le style, avec `tauri.conf.json` et avec le nom de commande,
donc une divergence fait échouer `npm run check` au lieu de casser en silence.

**La recette Rust de la coquille.** `cargo test`/`clippy` ne sont lancés que sur
`cassini-core`, comme le prévoit le plan : les dépendances de `cassini-app`
n'avaient jamais été téléchargées dans cet environnement et sa compilation
demanderait la chaîne GTK/WebKit. `cargo fmt --all --check` couvre tout de même
le formatage de `app/src-tauri/src/lib.rs`. Les API employées ont donc été
vérifiées sur le code de `tauri 2.12.2` lui-même : `impl<T: IpcResponse, E:
Into<InvokeError>> ResultKind for Result<T, E>` autorise `Result<Response,
String>`, et `scripts/ipc-protocol.js` résout un corps non-JSON en
`response.arrayBuffer()`, donc `invoke<ArrayBuffer>` est le bon type côté
interface.

## Plan Change Log

- **Dépendance ajoutée, hors de ce que la Verification annonçait** :
  `@tauri-apps/api` ^2.12.2 (Apache-2.0 OR MIT, aucune dépendance transitive).
  La tâche `app/src/basemap.source.ts` demande une implémentation « au-dessus de
  `invoke` », or le paquet qui fournit `invoke` n'était pas installé — le Code
  Map ne relevait que `pmtiles` dans `app/package.json`. La ligne de vérification
  « aucune autre dépendance ajoutée » n'est donc pas tenue à la lettre. La seule
  façon de l'éviter aurait été d'appeler `window.__TAURI_INTERNALS__.invoke`
  directement, c'est-à-dire de dépendre d'une interface interne non documentée
  pour économiser un paquet officiel de quelques kilo-octets : l'arbitrage a été
  tranché en faveur du paquet. À signaler au propriétaire, avec la licence, comme
  l'exige CLAUDE.md §8.
- **`@tauri-apps/api` approuvée par le propriétaire le 2026-10-10**, après lui
  avoir donné la licence (Apache-2.0 OR MIT, aucune dépendance transitive) comme
  l'exige CLAUDE.md §8. Motif retenu : c'est le paquet officiel qui porte
  `invoke`, et toutes les étapes suivantes en dépendront. L'alternative écartée
  était `window.__TAURI_INTERNALS__.invoke`, interface interne non documentée.
- **`pmtiles` passe en `dependencies`** comme prévu (BSD-3-Clause 4.5.0,
  transitif `fflate` 0.8.3 MIT, vérifiés dans les paquets installés).
- **Deux variantes d'erreur ajoutées à `BasemapError`** au lieu d'une seule :
  `PlageHorsFichier` comme demandé, mais aussi `LectureImpossible`, parce qu'une
  lecture de disque peut échouer pour d'autres raisons qu'une plage fausse et que
  l'enum dérive `PartialEq`, donc ne peut pas porter un `std::io::Error`.

## Review Triage Log

**Passe 1 — `bmad-review`, lentille `quick` épinglée, exécutée par un sous-agent en contexte vierge (première revue réellement indépendante de l'epic).** 8 constats. Verdicts : 0 haut, 5 moyens, 1 bas, 1 faux, 1 reporté. Une première tentative a été coupée par la limite de session sans rendre de constat ; elle n'a rien produit à trier.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Q1 — `app/src/main.ts` : à l'archive absente, `#etat` finit par afficher « Carte prête — hors ligne » au lieu de nommer le fichier manquant. La ligne de matrice « Archive absente » n'est donc pas tenue | moyen | corrigé | **Vérifié dans `node_modules/maplibre-gl/dist/maplibre-gl.mjs`** : le catch du chargement de TileJSON est `catch(e){this._tileJSONRequest=null,this._loaded=!0,dn(e)||this.fire(new z(nt(e)))}` — la source est marquée chargée **avant** l'émission de l'erreur. Puis `this.on("error",()=>{this._sourceErrored=this._source.loaded()})`, `loaded(){if(this._sourceErrored)return!0}`, et `this.loaded()&&!this._loaded&&(this._loaded=!0,this.fire(new Yn("load")))`. L'ordre est garanti : l'erreur passe, puis `load` l'écrase |
| Q2 — `app/src/basemap.source.ts` : le rappel `signalerEchec` n'apporte rien d'observable, et son commentaire avance une raison fausse | moyen | corrigé | Même cause que Q1 : `demander` relance la même erreur, MapLibre la transforme en événement `error`, et le gestionnaire de `main.ts` écrit le même texte dans le même élément juste après. Le message du rappel n'est visible que le temps d'une micro-tâche |
| Q3 — `app/src-tauri/src/lib.rs` : la cause nommée dans l'erreur est toujours le chemin de développement, jamais celui des ressources, et elle grave le chemin absolu de la machine de compilation dans un texte montré à l'utilisateur | moyen | corrigé | Lecture directe : les candidats sont empilés dans l'ordre ressources puis développement, et `derniere_erreur` est réécrite à chaque tour, donc la dernière gagne. `env!("CARGO_MANIFEST_DIR")` est résolu à la compilation, donc le chemin du constructeur — nom d'utilisateur compris — part dans le binaire |
| Q4 — `app/src-tauri/src/lib.rs` : la nouvelle commande n'a aucun test et aucune recette ne la compile, contre CLAUDE.md §8 | moyen | corrigé | `cargo check -p cassini-app` échoue ici (`libdbus-sys` panique, `pkg-config` absent), confirmé par exécution. Le plan restreignait `cargo test`/`clippy` à `cassini-core`, donc l'ordre des candidats et l'enveloppe `Response` n'étaient tenus par rien. Les API ont été auditées à la main sur `tauri-2.12.2`, sans trouver de rupture, mais rien dans le dépôt ne l'attraperait |
| Q5 — `app/scripts/check-offline.mjs` : la garde annonce surveiller « le code de l'interface » alors qu'elle ne lit que les `.ts` du premier niveau de `app/src` | moyen | corrigé | Lecture directe : `readdirSync` non récursif filtré sur `.ts`. Hors crible : `app/src/style.css`, que `main.ts` importe et où `@import url(https://…)` est une vraie requête réseau, plus `app/index.html` et tout futur sous-dossier. Le relecteur a par ailleurs prouvé empiriquement que ce qui **est** dans le crible échoue bien |
| Q6 — `app/scripts/verify-protocole.mjs` : les contrôles « plage hors fichier » éprouvent le double écrit dans le script, pas le lecteur de production, et recopient à la main le message de `BasemapError::PlageHorsFichier` | bas | corrigé | Lecture directe : `source.getBytes` est la classe `SourceFichier` déclarée dans le même fichier. Si `lire_plage` se mettait à tronquer, les 25 contrôles resteraient verts. La vraie couverture de cette ligne de matrice, ce sont les tests unitaires du moteur, qui passent |
| Q7 — le critère « un seul endroit du code porte son chemin et un seul porte sa clé » est littéralement faux | faux | rejeté | Le correctif consisterait à réécrire le critère du plan de cette construction, ce que le triage rejette par règle. Sur le fond, l'intention est tenue autrement : le chemin apparaît dans trois fichiers et la clé dans deux, mais `verify-protocole.mjs` recoupe chaque paire, donc une divergence fait échouer `npm run check` |
| Q8 — CLAUDE.md §9 exige des noms de code en anglais ; tout le code du dépôt est nommé en français, et cette entrée en ajoute environ 400 lignes | moyen, non causé par cette entrée | reporté | Relevé juste : la ligne de base est déjà entièrement française (`depuis_chemin_local`, `designe_le_reseau`, `tuilePng`…), donc l'entrée ne crée pas l'écart, elle l'étend. Le correctif toucherait CLAUDE.md ou renommerait tout le dépôt : décision du propriétaire, hors d'une entrée de branchement de carte. Voir `deferred-work.md` |

**La revue a payé son coût.** Q1 est un défaut que ni l'implémenteur ni moi n'avions vu : la fenêtre aurait annoncé « Carte prête » sans aucun fond de carte. C'est exactement le genre de succès silencieux sur échec qu'un produit dont la promesse est la vérifiabilité ne peut pas se permettre. Et c'est le premier constat de l'epic trouvé par un relecteur qui n'avait pas écrit le code.

## Design Notes

**Pourquoi l'IPC plutôt que le protocole `asset` de Tauri.** Le protocole `asset` sait répondre aux requêtes de plage (`206`, `Content-Range`, vérifié dans `crates/tauri/src/protocol/asset.rs`), mais il exige la fonctionnalité `protocol-asset`, une portée `assetProtocol`, et une origine HTTP (`asset:` ou `http://asset.localhost`) dans `connect-src`. Ouvrir une origine HTTP dans le CSP d'une application dont la promesse est l'absence de réseau est le mauvais côté de l'arbitrage. L'IPC est déjà autorisé, n'ajoute aucune fonctionnalité, et place la garde Rust sur le chemin réel de chargement. Coût : quelques allers-retours pour 26 Ko.

**Le mode de défaillance à craindre.** `Protocol.tilev4` traite ce qui suit `pmtiles://` comme une clé de dictionnaire. Clé absente ⇒ `new PMTiles(chaîne)` ⇒ `FetchSource` ⇒ requête réseau, sans un mot dans la console ; et `pmtiles://https://exemple.fr/f.pmtiles` lui est une URL valable. La garde doit donc connaître la clé attendue, pas seulement reconnaître le schéma. C'est ce qui porte le risque à `high` : une erreur ici casse en silence la promesse centrale du produit et ne se voit pas sur un poste connecté.

**Constat A10 tranché.** Avec `metadata: false`, `Protocol.tilev4` construit `bounds`, `minzoom` et `maxzoom` depuis l'en-tête ; `getTileJson`, seul lecteur des métadonnées JSON, n'est appelé que si `metadata` est vrai. Rien à ajouter au générateur ; l'entrée reportée peut être close.

**Tuiles de 256 px.** `COTE_TUILE = 256` dans le générateur, alors que MapLibre prend 512 par défaut pour une source raster : sans `tileSize: 256` explicite, l'échelle est fausse d'un facteur deux.

## Verification

**Commands:**
- `cd app && npm install` -- expected: `pmtiles` en dépendance de production, aucune autre dépendance ajoutée
- `cd app && npm run check` -- expected: typage strict, garde hors ligne (positifs et négatifs), fixture et protocole tous au vert
- `cargo test -p cassini-core && cargo clippy -p cassini-core --all-targets -- -D warnings && cargo fmt --all --check` -- expected: vert, lecture de plage couverte
- `cd app && npm run build` -- expected: `tsc --noEmit` puis `vite build` sans erreur
- `grep -rn '://' app/src/ app/src-tauri/tauri.conf.json` -- expected: uniquement la source `pmtiles://` avec la clé attendue, et les origines IPC déjà présentes du CSP

**Manual checks (if no CLI):**
- L'affichage réel des tuiles et le relevé de trafic réseau nul demandent une fenêtre Tauri, que cet environnement ne peut pas ouvrir : périmètre de l'entrée 1.4. `verify-protocole.mjs` couvre le contrat de chargement sans fenêtre.
