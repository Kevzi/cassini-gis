---
title: 'Passer les noms du code en anglais'
type: 'refactor'
ticket: '8'
created: '2026-10-13'
status: 'built'
baseline_revision: '3914405bcda6c54b22deee227f2b81cc1dcd592d'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Environ 230 identifiants du code sont en français (variantes
d'erreur, fonctions, constantes, noms de tests, ids HTML, un nom de fichier)
alors que CLAUDE.md §9 exige du code en anglais. Le dépôt est public et chaque
entrée livrée rend le renommage plus coûteux.

**Approach:** Renommage pur à comportement identique : les identifiants passent
en anglais, la prose française ne bouge pas. Les sept noms qui traversent une
frontière — commande Tauri, clés d'arguments IPC, constante relue par regex, ids
HTML et CSS, ids de couches du style, nom de fichier de script — changent des
deux côtés dans le même geste, aucun compilateur ne les reliant.

## Boundaries & Constraints

**Always:**
- Comportement observable inchangé : mêmes textes de fenêtre mot pour mot, mêmes
  messages d'erreur, mêmes fichiers lus et écrits.
- La prose reste en français : commentaires, messages d'`expect` et d'`assert!`,
  libellés affichés par les scripts de recette, docs (CLAUDE.md §9).
- Un nom de frontière change partout dans le même commit, regex comprises.
- Les documents du dépôt qui citent un identifiant renommé sont mis à jour.
- La valeur de la clé du protocole passe elle aussi en anglais :
  `cassini-fond-de-recette` devient `cassini-basemap-fixture`, du même octet
  dans `basemap.source.ts:23` et `basemap.style.json:7` (décision du
  propriétaire, 2026-10-13).

**Never:**
- Aucun changement de logique, de dépendance, de format de fichier, ni d'ordre
  des champs sérialisés. Pas de test ajouté, pas de refactorisation d'occasion.
- Pas de renommage des noms imposés de l'extérieur : scheme `pmtiles://`, clés
  de `tauri.conf.json`, métadonnées PMTiles, variables `CARGO_*`, crates
  `cassini-core` et `cassini-app`, lieu `villeneuve-le-roi`.
- Pas de traduction de `docs/compiler-sous-windows.md` ni de son nom : doc
  utilisateur.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Carte au démarrage | fixture présente | carte montée dans son conteneur, tuiles lues par plages, zéro requête réseau | aucune erreur attendue |
| Fixture absente ou plage hors fichier | archive supprimée, ou plage au-delà de la taille | refus avant lecture | messages `fond de carte introuvable …` et plage à trois nombres, inchangés |
| Garde hors ligne | adresse `http` glissée dans le style ou le code | la recette échoue | les 16 cas négatifs restent refusés à l'identique |

</frozen-after-approval>

## Code Map

- `core/src/basemap.rs` — ~60 identifiants FR : variantes et champs de
  `BasemapError`, `depuis_chemin_local`, `lire_plage`, `designe_le_reseau`,
  16 noms de tests. Les 8 messages `Display` ne bougent pas.
- `core/src/lib.rs` — un nom de test.
- `app/src-tauri/src/lib.rs` — 11 identifiants dont **4 contrats** :
  `CHEMIN_DE_LARCHIVE`, `version_du_moteur`, `lire_le_fond_de_carte` et ses
  paramètres `decalage` / `longueur`, sérialisés par leur nom dans l'IPC.
  **Ce paquet ne compile pas ici** (`libdbus-sys` absent) — voir Design Notes.
- `app/src/basemap.source.ts` — l'autre bout : `COMMANDE`, `CLE_DU_FOND` et
  **sa valeur**, `SourceFondDeCarte`, charge utile `{ decalage, longueur }`.
  La valeur doit rester identique à celle de `basemap.style.json:7` ;
  `check-offline.mjs` et `verify-protocole.mjs` la comparent octet pour octet.
- `app/src/main.ts`, `app/index.html`, `app/src/style.css` — 10 noms, ids `carte`
  et `etat`, variables `--fond` / `--texte` / `--bord` ; un seul bloc.
- `app/src/basemap.style.json` — ids `fond`, `fond-tuiles`, `repere`,
  `repere-surface`, `repere-contour`, lus en dur par `verify-protocole.mjs:169`
  et `verify-basemap.mjs:141`.
- `app/scripts/*.mjs` (4 fichiers) — ~1030 occurrences, 93 % de l'effort, toutes
  internes **sauf** les constantes exportées importées entre scripts
  (`CHEMIN_SORTIE`, `EMPRISE`, `construireArchive`, `identifiantTuile`,
  `tuilesCouvrantes`, `cleDuFond`, `CHEMIN_DU_STYLE`) et les deux regex de
  `verify-protocole.mjs`.
- `app/package.json` — script `verify:protocole` et deux chemins vers le fichier
  renommé. `fixtures/README.md` — quatre citations à corriger (dont la clé), prose intacte.
- Ne pas toucher : `main.rs`, `build.rs`, les `Cargo.toml`, `tauri.conf.json`,
  `vite.config.ts`, `tsconfig.json`, `vite-env.d.ts`.

## Tasks & Acceptance

**Execution:**
- [x] `app/src-tauri/src/lib.rs` + `app/src/basemap.source.ts` + `app/scripts/verify-protocole.mjs` — renommer d'un bloc les 4 noms qui traversent l'IPC et les 2 regex qui les lisent — un décalage casse le chargement sans erreur de compilation
- [x] `app/index.html` + `app/src/style.css` + `app/src/main.ts` — renommer les ids, variables CSS et sélecteurs qui les visent — aucun test ne couvre le DOM, un id orphelin ne se voit qu'à l'ouverture de la fenêtre
- [x] `app/src/basemap.style.json` + `verify-protocole.mjs` + `verify-basemap.mjs` — renommer les ids de source et de couches et les accès codés en dur
- [x] `app/src/basemap.source.ts` + `app/src/basemap.style.json` — passer la valeur de la clé à `cassini-basemap-fixture` dans les deux fichiers d'un seul geste — les deux scripts de recette comparent les deux octet pour octet et refusent le moindre écart
- [x] `core/src/basemap.rs` + `core/src/lib.rs` — renommer les ~61 identifiants Rust restants, tests compris
- [x] `app/scripts/*.mjs` — renommer les identifiants internes des 4 scripts et propager aux imports croisés
- [x] `git mv app/scripts/verify-protocole.mjs app/scripts/verify-protocol.mjs` + `app/package.json` — renommer le fichier et le script npm en `verify:protocol`
- [x] `fixtures/README.md` — mettre à jour les quatre citations, prose inchangée

**Acceptance Criteria:**
- Given le dépôt renommé, when on lance la recette complète, then elle passe au vert aux mêmes compteurs qu'avant : 17 tests Rust, 16 cas négatifs, 30 puis 24 contrôles.
- Given le code source, when on cherche les racines françaises hors prose dans `core/src`, `app/src`, `app/scripts`, `app/index.html` et `app/package.json`, then plus aucun identifiant français n'apparaît.
- Given le diff du commit, when on l'examine, then aucune ligne de texte affiché à l'utilisateur n'a changé, aucun commentaire n'a été traduit, et aucun fichier hors du Code Map n'est touché.
- Given l'application compilée sous Windows, when la fenêtre s'ouvre, then la carte s'affiche et « Chargement… », « Carte prête — hors ligne » et « Fond de carte indisponible : … » sont identiques mot pour mot.

## Implementation Notes

Renommage pur, fait dans l'ordre des contrats. Les quatre noms qui traversent
l'IPC (`ARCHIVE_PATH`, `engine_version`, `read_basemap`, `offset` / `length`)
ont changé des deux côtés dans le même geste que les deux regex de
`verify-protocol.mjs` qui les relisent. La valeur de la clé du protocole est
passée à `cassini-basemap-fixture` du même octet dans `basemap.source.ts` et
`basemap.style.json`.

Correspondances principales :

| Avant | Après |
|---|---|
| `BasemapError::{SourceReseau, CheminVide, FichierAbsent, PlageHorsFichier, AucunEmplacement, LectureImpossible}` | `{NetworkSource, EmptyPath, MissingFile, RangeOutsideFile, NoLocation, ReadFailed}` |
| `depuis_chemin_local`, `depuis_le_premier_emplacement`, `lire_plage`, `chemin`, `designe_le_reseau` | `from_local_path`, `from_first_location`, `read_range`, `path`, `targets_network` |
| `CHEMIN_DE_LARCHIVE`, `version_du_moteur`, `lire_le_fond_de_carte`, `decalage`/`longueur` | `ARCHIVE_PATH`, `engine_version`, `read_basemap`, `offset`/`length` |
| `CLE_DU_FOND`, `COMMANDE`, `SourceFondDeCarte` | `BASEMAP_KEY`, `COMMAND`, `BasemapSource` |
| ids `carte`, `etat` ; `--fond`, `--texte`, `--bord` | `map`, `status` ; `--background`, `--text`, `--border` |
| couches `fond`, `fond-tuiles`, `repere-surface`, `repere-contour` ; sources `fond-pmtiles`, `repere` | `background`, `basemap-tiles`, `marker-fill`, `marker-outline` ; `basemap-pmtiles`, `marker` |
| `CHEMIN_SORTIE`, `EMPRISE`, `identifiantTuile`, `tuilesCouvrantes`, `construireArchive`, `cleDuFond`, `CHEMIN_DU_STYLE` | `OUTPUT_PATH`, `EXTENT`, `tileId`, `coveringTiles`, `buildArchive`, `basemapKey`, `STYLE_PATH` |

Deux décisions prises en cours de route, hors liste explicite du plan mais dans
son esprit :

- la propriété GeoJSON `nom` du repère devient `name` (aucun code ne la lit, et
  elle vit dans un fichier du Code Map) ;
- la source de style en mémoire `fond` des cas négatifs de `check-offline.mjs`
  devient `basemap` ; ses traces ne sont imprimées qu'en cas d'échec, et ces cas
  ne vérifient que le nombre de fautes.

Non renommés, exprès : les noms de fichiers temporaires des tests Rust
(`cassini-fond-ok.pmtiles`, etc.), parce que le plan exige « mêmes fichiers lus
et écrits » ; la clé de test `une-autre-cle` et `une-cle-non-enregistree`, qui
sont des données de cas négatifs ; `docs/compiler-sous-windows.md`.

## Plan Change Log

## Review Triage Log

### Passe 1 — lentille `quick` (2026-10-13)

Verdicts : 0 high, 1 medium, 3 low, 1 false, 0 maybe-false. Routes : 1 patch (groupe de 2 constats), 1 defer, 3 rejets.

| Constat | Verdict | Route | Évidence |
|---|---|---|---|
| `deferred-work.md:15-17` — l'entrée « Réconcilier CLAUDE.md §9 » que cette entrée résout reste ouverte | medium | patch | Vérifié : l'entrée n'a ni `status:` ni `resolution:`, là où l'entrée A10 (l. 12-13) porte les deux. Le renommage la résout. Laisser l'entrée ouverte fait mentir le suivi. |
| `deferred-work.md:13` — chemin périmé `app/scripts/verify-protocole.mjs` | low | patch | Vérifié : le fichier est renommé par cette entrée ; même cause racine que ci-dessus (le fichier de travail reporté n'a pas été touché), donc groupé. |
| `basemap.style.json:19` + `check-offline.mjs:166` — renommage de deux valeurs sérialisées (`nom`→`name`, source `fond`→`basemap`) contre un « Never » gelé, non journalisé | false | rejet | Réfuté sur ses deux moitiés. Le « Never » gelé interdit un changement de *format de fichier* et d'*ordre des champs sérialisés* — pas le nom d'une clé, et le fichier reste du GeoJSON. Et le `## Plan Change Log` est, par le commentaire du gabarit lui-même, une section de l'étape 4 : l'implémenteur a journalisé au bon endroit, les Implementation Notes. Impact comportemental nul, vérifié indépendamment : aucune lecture de `properties.nom`/`name`, et les cas négatifs n'éprouvent que le nombre de fautes. |
| `npm run build` listé en vérification alors qu'il ne peut pas tourner ici | low | rejet | Réel et confirmé (`node_modules/@rollup` ne contient que les binaires `win32`, installés depuis Windows), mais le correctif consiste à modifier le plan de ce build. La limite est déjà écrite dans `docs/compiler-sous-windows.md:110-111`. |
| AC4 non vérifiable et aucun contrôle ne relie les ids de `index.html` à `main.ts` et `style.css` | low | defer | La moitié « défaut » est réfutée : les trois côtés concordent, vérifié à la main (`id="map"`/`id="status"` ↔ `#status`/`container:"map"` ↔ `#map`/`#status`), et aucun `var()` ne pend. Le manque de garde est réel mais antérieur à cette entrée, et le plan interdit d'ajouter un test ici. |
| `basemap.rs` — la raison écrite pour garder les noms de fichiers temporaires français est inexacte | low | rejet | Réel : ces fichiers sont créés et supprimés par les tests dans `temp_dir()`, la règle « mêmes fichiers lus et écrits » ne les couvre pas ; la vraie raison est qu'ils sont des données de test, pas des identifiants. Correctif = modifier le plan de ce build, donc rejeté. |

## Design Notes

Le compilateur Rust et `tsc` rattrapent toutes les fautes de renommage **à
l'intérieur** d'un langage. La seule difficulté est la poignée de noms reliés
par du texte plutôt que par un type — d'où l'ordre des tâches, contrats d'abord,
chacun avec ses deux bouts dans la même tâche.

Le piège : `verify-protocole.mjs` lit le nom de la commande dans le TypeScript,
puis le cherche dans le Rust.

```js
const commande = codeSourceTs.match(/const COMMANDE = "([^"]+)"/)[1];
// …puis vérifie `fn <commande>(` et sa présence dans generate_handler![…]
```

Renommer `COMMANDE` sans toucher la regex fait échouer la recette avec un
message trompeur ; renommer la regex sans renommer la commande Rust laisse
passer une application qui ne charge plus rien.

Le paquet Tauri ne compilant pas ici, ses 11 renommages se prouvent par grep :
zéro occurrence restante de chaque ancien nom dans tout le dépôt hors
`_bmad-output`.

## Verification

**Commands:**
- `cargo fmt --all --check` — expected: silencieux ; vérifie la syntaxe même du paquet Tauri non compilable
- `cargo clippy -p cassini-core --all-targets -- -D warnings` — expected: aucun avertissement
- `cargo test -p cassini-core` — expected: 17 passed, 0 failed
- `cd app && npm run check` — expected: `tsc` muet, « Garde hors ligne : OK … 16 cas négatifs », « fond de recette : OK, 30 contrôles », « protocole : OK, 24 contrôles »
- `cd app && npm run build` — expected: `tsc --noEmit` puis `vite build` sans erreur
- `for s in 'Chargement…' 'Carte prête — hors ligne' 'Fond de carte indisponible : ' 'Erreur de carte : '; do grep -rqF "$s" app/index.html app/src/ || echo "MANQUANT: $s"; done` — expected: aucune sortie. (La version précédente de ce contrôle, un `git diff | grep`, ne pouvait pas rendre une sortie vide : une ligne renommée qui porte aussi le texte apparaît forcément des deux côtés du diff.)
- `git grep -nE "<ancien_nom>" -- ':!_bmad-output'` pour chaque nom renommé du paquet Tauri — expected: aucune sortie

**Manual checks (if no CLI):**
- La compilation de `cassini-app` et l'ouverture de la fenêtre restent hors de portée de cet environnement (voir entrée 1.4) : à revérifier au prochain lancement Windows, ou automatiquement quand l'entrée 1.6 aura posé la recette sur GitHub Actions.
