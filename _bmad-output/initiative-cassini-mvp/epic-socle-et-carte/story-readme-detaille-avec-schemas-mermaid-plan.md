---
title: 'README detaille avec schemas Mermaid'
type: 'chore'
ticket: '5'
created: '2026-10-14'
status: 'built'
baseline_revision: '5092dc74666cf709299595b575a739a9a88fb4a6'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le dépôt est public et n'a aucun `README.md` racine : qui l'ouvre
sur GitHub voit des dossiers sans une ligne d'explication, et ne sait ni à quoi
sert `/core` face à `/app`, ni ce qui fonctionne déjà. L'epic exige (E5) que le
dépôt explique son architecture, schémas compris.

**Approach:** écrire un `README.md` racine, en français : présentation, état réel
de l'étape 1, tableau des dossiers, démarrage rapide, et trois schémas Mermaid
rendus nativement par GitHub — l'architecture en couches, le trajet d'une demande
en langage naturel jusqu'aux commandes, le cycle de vie d'un projet.

## Boundaries & Constraints

**Always:** français (CLAUDE.md §9) ; les trois schémas demandés, et eux seuls ;
un schéma qui décrit une cible non codée est annoncé comme tel dès sa phrase
d'introduction ; toute commande citée existe telle quelle dans `app/package.json`
ou le workspace Cargo ; les principes cités (hors ligne, pas de GPL/LGPL,
l'utilisateur apporte ses données, noyau civil) reprennent CLAUDE.md sans le
contredire ; Mermaid courant seulement (`flowchart`, `sequenceDiagram`,
`stateDiagram-v2`), libellés entre guillemets droits.

**Never:** aucun badge de recette (GitHub Actions est l'entrée 1.6, à venir) ;
aucune capture d'écran ; aucune fonction prévue présentée comme disponible ;
aucune URL sous `app/src/` (la garde hors ligne y échouerait) ; aucune dépendance
nouvelle ; rien venant d'Esri (§2.1) ; ne pas réécrire les README de `/fixtures`
et `/tests` ni `docs/compiler-sous-windows.md`.

</frozen-after-approval>

## Code Map

- `README.md` -- **à créer**, racine. Seul fichier de fond de l'entrée.
- `CLAUDE.md` -- source des §1 vision, §2 principes, §3 architecture, §7 étapes.
  À résumer, jamais à recopier.
- `Cargo.toml` -- `members = ["core", "app/src-tauri"]` : `cassini-core` (moteur,
  zéro dépendance) et `cassini-app` (coquille), les crates à nommer.
- `core/src/basemap.rs` -- tout ce que le moteur sait faire : `BasemapSource`
  (`from_local_path`, `from_first_location`, `read_range`) et le refus de toute
  adresse réseau (`targets_network`). 17 tests avec `lib.rs`.
- `app/src-tauri/src/lib.rs` -- les deux seules commandes existantes,
  `engine_version` et `read_basemap` ; le registre du §4 n'est pas encore écrit.
- `app/src/main.ts`, `basemap.source.ts` -- MapLibre, protocole `pmtiles`
  enregistré avant la carte, octets lus par l'IPC et jamais par le web.
- `app/package.json` -- scripts à citer : `dev`, `check`, `fixtures`, `tauri` ;
  `app/scripts/check-offline.mjs` et `verify-protocol.mjs` sont la garde à citer.
- `docs/README.md` -- index à compléter. Les README de `/fixtures` et `/tests`
  existent : les lier, pas les redire.

## Tasks & Acceptance

**Execution:**
- [x] `README.md` -- créer, dans cet ordre : titre et présentation ; « ce qui
  marche aujourd'hui » et « ce qui est prévu » adossés aux étapes de §7 ; les
  quatre principes ; les trois schémas ; le tableau des dossiers ; le démarrage
  rapide ; la recette ; la licence et le noyau civil -- première page que voit un
  visiteur, et E5 l'exige.
- [x] `docs/README.md` -- ajouter une ligne vers `../README.md` en tête de liste
  -- pour que l'index de la documentation reste vrai.

**Acceptance Criteria:**
- Given un lecteur qui découvre le dépôt, when il a lu le README, then il peut
  dire à quoi servent `/app`, `/core`, `/docs`, `/fixtures`, `/tests` et
  `_bmad-output`, chacun ayant sa ligne dans le tableau des dossiers.
- Given la page du dépôt sur GitHub, when elle s'affiche, then les trois blocs
  Mermaid apparaissent en schémas, aucun en bloc de code ni tronqué.
- Given les trois schémas, when on les relit, then ils montrent respectivement
  les couches, le trajet d'une demande en langage naturel jusqu'aux commandes et
  le cycle de vie d'un projet, les deux derniers étant donnés pour des cibles.
- Given le README, when on vérifie chaque commande dans `app/package.json` ou le
  workspace Cargo, then elle y figure à l'identique, et rien n'est annoncé
  disponible qui soit encore prévu.
- Given la recette, when `cargo test -p cassini-core`, `cargo clippy` et
  `npm run check` tournent ensuite, then tout passe comme avant.

## Implementation Notes

Deux fichiers touchés, aucun code : `README.md` créé à la racine,
`docs/README.md` complété d'une entrée en tête de liste. Aucune dépendance
ajoutée, aucun fichier sous `app/src/` modifié.

Le README suit l'ordre exigé : présentation ; « ce qui marche aujourd'hui » et le
tableau des sept étapes de `CLAUDE.md` §7 ; les quatre principes ; les trois
schémas ; le tableau des dossiers ; le démarrage rapide ; la recette ; la licence
et le noyau civil.

Chiffres et noms repris du code, pas de mémoire : 17 tests (16 dans
`basemap.rs` + 1 dans `lib.rs`), 30 contrôles de `verify-basemap.mjs`, 24 de
`verify-protocol.mjs` — les trois relus dans la sortie de `npm run check`. Les
deux commandes nommées sont `engine_version` et `read_basemap`, les seules du
`invoke_handler` ; les crates sont `cassini-core` et `cassini-app`.

Trois décisions prises en cours de route, dans l'esprit du plan :

- **Aucun nom de produit concurrent.** `CLAUDE.md` §1 cite ArcGIS Pro comme
  repère ; le README public dit « les SIG de bureau établis ». Le « Never » de
  l'entrée est « rien venant d'Esri (§2.1) », et un README public n'a aucun
  besoin du nom pour se faire comprendre.
- **Le premier schéma marque lui-même ce qui n'existe pas.** L'entrée ne le range
  pas parmi les cibles, mais l'architecture de §3 contient des couches non
  codées : les cases prévues portent le mot « prévu » **dans leur libellé**, en
  plus de leur couleur, pour que l'information ne repose pas sur la seule teinte.
  Les schémas 2 et 3 sont annoncés comme cibles dès leur phrase d'introduction,
  comme demandé.
- **Aucune URL externe dans le README.** Les prérequis renvoient à
  `docs/compiler-sous-windows.md`, qui les porte déjà. Rien à maintenir en
  double, et aucun lien à voir mourir.

Deux ajouts au tableau des dossiers par rapport à la liste du critère
d'acceptation : `_bmad/` (suivi par git, même si ses scripts sont ignorés) et une
ligne disant que `/ai`, annoncé par §9, n'existe pas encore. Le contrôle manuel
demandait qu'aucun dossier du dépôt ne soit oublié : `git ls-files` en donne sept
à la racine, les sept sont au tableau.

Précision ajoutée au démarrage rapide, faute constatée à l'usage : `npm run dev`
ne lance que la webview, donc sans fond de carte — ses octets viennent de l'IPC.
C'est `npm run tauri dev` qui montre l'application.

Recette passée : `cargo test -p cassini-core` (17 passés), `cargo fmt --check`
(muet), `cargo clippy -p cassini-core --all-targets` (rien), `npm run check`
(garde hors ligne, 30 contrôles, 24 contrôles), et le compte des ouvertures de
bloc Mermaid du README : 3, comme attendu.

`cargo clippy --workspace` reste hors de portée de cet environnement :
`libdbus-sys` ne se construit pas sous WSL, comme déjà constaté à l'entrée 1.4 et
redit à l'entrée 1.8. L'entrée touchant deux fichiers Markdown, rien n'entre dans
la compilation de `cassini-app`.

Validation de la syntaxe Mermaid : `mermaid-cli` est écarté par les Design Notes,
donc contrôle structurel par un script jetable hors du dépôt — blocs ouverts et
fermés, guillemets droits appariés, aucun identifiant d'arête ou de `class` non
déclaré. Les trois blocs passent. Le rendu lui-même reste à voir sur GitHub,
c'est le contrôle manuel de l'entrée.

## Plan Change Log

## Review Triage Log

- **medium** -- `README.md` l. 85, « l'interface n'appelle que deux commandes ».
  Verifie : `app/src-tauri/src/lib.rs:76` en enregistre deux, mais le seul
  `invoke` de tout `app/src` est `basemap.source.ts:61` avec
  `COMMAND = "read_basemap"`, et `engine_version` n'apparait nulle part dans
  `app/src` ni dans les scripts de recette. Le moteur en expose deux, l'interface
  en appelle une. Route : patch.
- **medium** -- `README.md` l. 3-5, « s'organise en projets » au present.
  Verifie : la notion de projet arrive a l'etape 2 et le schema 3 le dit
  explicitement ; la meme phrase met « se pilotera » au futur, ce qui fait lire
  les autres verbes comme l'etat courant. L'avertissement de l. 37 vient apres
  l'introduction, donc ne la couvre pas. Contredit le « Never » du plan (« aucune
  fonction prevue presentee comme disponible »). Route : patch.
- **medium** -- `README.md` l. 271-275, portee prêtee a `check-offline.mjs`.
  Verifie : `app/scripts/check-offline.mjs:36` limite le crible a
  `SCREENED_EXTENSIONS = [".ts", ".tsx", ".js", ".mjs", ".css", ".html"]`, et le
  seul JSON lu est `STYLE_PATH` (`app/src/basemap.style.json`, l. 30). Une adresse
  reseau dans un autre `.json` sous `app/src`, ou dans une extension non listee,
  passerait sans alerte : un developpeur qui fait confiance a `npm run check`
  livrerait la reference reseau. Route : patch.
- **low** -- `README.md` l. 96 et 133, schema 1 : le sous-graphe
  `reg["Registre de commandes — Rust"]` ne porte pas « prevu » et sa case
  `existantes` est classee `fait`, alors que l. 30-31 affirme que le registre du
  §4 n'est pas ecrit. Verifie : `app/src-tauri/src/lib.rs` n'a que deux
  `#[tauri::command]` nus, sans `id`, description bilingue, schema JSON ni
  resultat type. Les deux commandes existent bien (le vert est juste), c'est le
  libelle de la couche qui annonce une cible. Correction directe du libelle.
  Route : patch.

**Résolution (2026-10-14).** Les quatre patchs sont appliqués, tous dans
`README.md`, aucun autre fichier touché :

1. Intro du schéma 1 : « le moteur expose deux commandes, et l'interface n'en
   consomme qu'une, celle qui lit les octets du fond de carte ».
2. Première phrase : « s'organisera en projets », au futur comme « se pilotera ».
3. Garde hors ligne : la portée énumère `app/index.html`, le style
   `app/src/basemap.style.json`, et les fichiers `.ts`, `.tsx`, `.js`, `.mjs`,
   `.css` et `.html` de `app/src` — ni plus, ni moins que ce que le script relit.
4. Libellé du sous-graphe : « Registre de commandes — Rust — prévu », même
   suffixe que les autres cases prévues ; `class carte,existantes,fond fait`
   inchangé, les deux commandes existant bien.

Contrôles relancés après correction : 3 blocs Mermaid, et le lint structurel des
trois blocs toujours au vert. Aucun test ne couvre un fichier Markdown, donc rien
d'autre n'a été exécuté.

## Design Notes

GitHub rend Mermaid nativement dans le Markdown : un bloc ` ```mermaid ` suffit,
sans image ni outil — d'où le format, et aucune dépendance ajoutée. Valider la
syntaxe hors ligne demanderait `mermaid-cli` : écarté, le critère de l'entrée
étant justement le rendu sur GitHub.

Deux schémas sur trois décrivent une cible — le registre et le projet arrivent à
l'étape 2, l'agent à l'étape 5. L'entrée demande de les dessiner, mais le README
ne doit pas laisser croire qu'ils tournent : la section « ce qui marche
aujourd'hui » tient le lecteur au vrai. Pas de capture d'écran non plus, le fond
livré étant un damier de recette qui montre mal le produit.

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: 17 tests passent
- `cargo fmt --check` et `cargo clippy --workspace --all-targets` -- expected:
  aucun avertissement
- `cd app && npm run check` -- expected: typage, garde hors ligne, archive et
  protocole au vert, preuve qu'aucune URL n'est entrée sous `app/src/`
- `grep -c '^```mermaid' README.md` -- expected: `3`

**Manual checks (if no CLI):**
- Aperçu Markdown de GitHub, ou poussée sur une branche : les trois schémas
  s'affichent en dessins, aucun ne reste en texte ni n'affiche d'erreur.
- Relire le tableau des dossiers avec `ls` sous les yeux : aucun dossier listé
  qui n'existe pas, aucun dossier du dépôt oublié.
