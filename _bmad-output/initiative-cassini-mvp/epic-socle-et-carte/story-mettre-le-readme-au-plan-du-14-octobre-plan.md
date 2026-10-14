---
title: 'Mettre le README au plan du 14 octobre'
type: 'chore'
ticket: '9'
created: '2026-10-14'
status: 'built'
baseline_revision: '374f84235571849809f9b403d0fcb6b2641f061c'
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

**Problem:** le `README.md` racine, écrit à l'entrée 1.5, décrit le plan
abandonné le 2026-10-14 : les sept anciennes étapes, une architecture dont
l'interface est Tauri et MapLibre, et un cycle de vie de projet qui suppose un
format de projet Cassini. Le dépôt est public : pousser en l'état afficherait une
feuille de route morte sur la page d'accueil.

**Approach:** remettre le README en accord avec le nouveau `CLAUDE.md` —
présentation, ce qui marche et ce qui est prévu, les trois schémas redessinés, le
tableau des dossiers, le démarrage rapide, la licence — en gardant tel quel tout
ce qui reste vrai.

## Boundaries & Constraints

**Always:** français (CLAUDE.md §9) ; trois schémas, en Mermaid courant
(`flowchart`, `sequenceDiagram`, `stateDiagram-v2`), libellés entre guillemets
droits ; un schéma qui décrit une cible le dit dès sa phrase d'introduction ; les
principes cités suivent le nouveau §2 et distinguent le noyau de l'extension ;
toute commande citée existe à l'identique dans `app/package.json` ou le workspace
Cargo.

**Never:** aucun badge de recette (l'entrée 1.6 est à venir) ; aucune capture ;
aucune fonction prévue présentée comme disponible ; aucune URL sous `app/src/` ;
aucune dépendance nouvelle ; rien venant d'Esri (§2.1) ; ne pas réécrire
`fixtures/README.md`, `tests/README.md`, `docs/README.md` ni
`docs/compiler-sous-windows.md` ; **ne pas effacer la trace du travail livré** —
la coquille Tauri et la carte MapLibre existent et marchent, elles sont reportées
en V2, pas supprimées.

</frozen-after-approval>

## Code Map

- `README.md` -- **à modifier**, 305 lignes, seul fichier de fond. À reprendre :
  présentation (l. 3-19), « ce qui marche » (21-42), « ce qui est prévu » (44-61),
  les quatre principes (63-78), les trois schémas (89-136, 151-174, 185-205), le
  tableau des dossiers (211-227), le démarrage rapide (229-256), la licence
  (289-305). **La section « la recette » (258-287) reste juste : ne pas y toucher.**
- `CLAUDE.md` -- la nouvelle source : §1 vision, §2 les dix principes dont 2, 3 et
  4 portent la frontière de licence, §3 le schéma d'architecture, §6 le périmètre
  scindé entre noyau et QGIS, §7 la séquence d'étapes, §9 la structure, §10 les
  trois licences. À résumer, jamais à recopier.
- `core/src/basemap.rs`, `app/src-tauri/src/lib.rs`, `app/src/basemap.source.ts`
  -- inchangés ici. Faits à préserver dans « ce qui marche » : deux commandes
  exposées (`engine_version`, `read_basemap`), une seule appelée par l'interface,
  17 tests, refus de toute adresse réseau à la construction.
- `_bmad-output/initiative-cassini-mvp/change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md`
  -- le pourquoi du virage, si une phrase d'explication manque.

## Tasks & Acceptance

**Execution:**
- [x] `README.md` -- réécrire les sections listées au Code Map -- c'est la
  première page que voit un visiteur, et elle annonce un plan abandonné.

**Acceptance Criteria:**
- Given un lecteur qui découvre le dépôt, when il a lu le README, then il peut
  dire que QGIS exécute la carte, l'ouverture des fichiers, le projet, les
  analyses et le PDF, que le noyau porte le registre, l'agent, le journal et le
  terrain, et que rien de l'extension n'existe encore.
- Given la page du dépôt sur GitHub, when elle s'affiche, then les trois blocs
  Mermaid apparaissent en schémas, aucun en bloc de code ni tronqué.
- Given les trois schémas, when on les relit, then le premier montre QGIS
  au-dessus du noyau avec le canal local entre les deux, le deuxième fait passer
  la demande par le panneau Cassini de QGIS, le troisième montre la vie d'un
  projet QGIS avec le journal Cassini à côté.
- Given le tableau des dossiers, when on le compare à `git ls-files` à la racine,
  then chaque dossier suivi a sa ligne, `/qgis-plugin` est annoncé comme pas
  encore créé, et `/app` est marqué reporté en V2 et conservé exprès.
- Given la licence, when on la lit, then elle distingue le noyau Apache-2.0,
  l'extension en GPL — par conséquence et non par choix — et le module défense
  privé.

## Implementation Notes

`README.md` passe de 305 à 364 lignes. Un seul fichier touché ; aucun code, aucune
dépendance.

Ce qui a été réécrit : la présentation (Cassini est l'interface en langage
naturel d'un SIG, le concurrent est QGIS, extension sur noyau indépendant, virage
du 2026-10-14 avec lien vers la proposition de changement) ; « ce qui marche
aujourd'hui », qui ouvre désormais sur « rien de l'extension n'existe » et ferme
sur « la carte et la coquille sont hors du chemin critique mais elles restent » ;
le tableau des sept étapes, repris de `CLAUDE.md` §7 ; les quatre principes, où
le deuxième porte la frontière noyau / extension et le troisième « le noyau
tourne sans QGIS » ; les trois schémas ; le tableau des dossiers ; le démarrage
rapide ; la licence, qui énumère les trois régimes.

Ce qui est resté tel quel : la section « la recette » dans son intégralité
(contrôlée identique au `git show HEAD:README.md`, au saut de ligne final près),
les deux précisions `npm run dev` / `npm run tauri dev` / `npm run fixtures`, les
liens vers `docs/`, `fixtures/` et `tests/`, et le paragraphe de fond sur le
*specially designed for military use*.

Deux choix de forme pris pour que GitHub rende les schémas :

- les alias de `sequenceDiagram` ne contiennent plus de virgule — le lexer de
  Mermaid exclut `,` et `;` du jeton d'acteur, et un alias virgulé aurait cassé
  le bloc entier ;
- la note du `stateDiagram-v2` ne contient plus de deux-points ;
- dans le schéma 1, les deux liens pointillés partent du noyau vers la case
  `app/` et non l'inverse : dagre range la source au-dessus de la cible, et des
  flèches partant d'`app/` l'auraient hissée entre QGIS et le noyau, là où le
  critère d'acceptation demande QGIS au-dessus du noyau avec le canal entre eux ;
- toujours dans le schéma 1, le chemin `panneau --> canal --> registre` est
  orienté de haut en bas, l'aller-retour étant porté par le libellé du nœud
  `canal` : une arête bidirectionnelle `<-->` suffit à faire ignorer le
  `direction` du sous-graphe QGIS, et la grappe récupérait alors le canal dans sa
  propre bande verticale au lieu de le laisser entre les deux grappes.

Le libellé `## Licence et noyau civil` est conservé pour que l'ancre
`#licence-et-noyau-civil`, citée par le principe 4, reste valide.

## Plan Change Log

## Review Triage Log

Passe 1, lentille quick : 6 medium ou low retenus, 2 reportes, 0 faux, 0 bouclage.
La lentille a reellement rendu les trois blocs Mermaid avec mermaid 10.9.8 et
11.17.2 : AC2 est atteint, les trois rendent sans erreur.

- **medium** -- `README.md` l. 121, `direction LR` dans `subgraph qgis` est inerte,
  et le canal local ne se place pas entre les deux grappes. Verifie : la lentille a
  rendu le bloc et releve des rangs identiques avec et sans la ligne `direction`,
  `canal` centre a y=113, dans la bande verticale de la grappe QGIS et non dans
  l'ecart entre les deux. Mermaid ignore la `direction` d'un sous-graphe des qu'un
  de ses noeuds pointe hors du sous-graphe, ce que fait `panneau <--> canal`. AC3
  partiellement non atteint. Route : patch.
- **medium** -- `README.md` l. 114, « a ce jour, une seule case est faite » sous-estime
  le travail livre. Verifie : le meme schema porte la case `v2` et les l. 40-60 disent
  que ce code est ecrit, marche et porte 17 tests (16 dans `basemap.rs`, 1 dans
  `lib.rs`). Compter les seules cases classees `fait` contredit le Never du plan,
  « ne pas effacer la trace du travail livre ». Route : patch.
- **medium** -- `README.md` l. 259, la ligne `core/` presente du code prevu comme
  present : « Toute la logique metier y vit -- registre, journal, terrain ». Verifie :
  `core/src` ne contient que `lib.rs` et `basemap.rs`, et un `grep` de registry,
  journal, terrain et viewshed n'y renvoie rien. Seule affirmation au present sans
  marqueur dans tout le README. Contredit le Never « aucune fonction prevue presentee
  comme disponible ». Route : patch.
- **medium** -- `README.md`, « l'interface » designe deux composants differents.
  Verifie : l. 48, 55 et 60 l'emploient pour la webview Tauri, l. 176 et 272 pour le
  panneau QGIS. Un lecteur de la l. 176 n'a aucun moyen de savoir lequel est vise.
  Route : patch.
- **low** -- `README.md` l. 313, « Les 17 tests du moteur » garde le mot abandonne
  alors que le reste du document dit « noyau ». Verifie par grep : seules les l. 26
  (moteur de terrain, voulu, repris de CLAUDE.md §1) et 313 portent encore le mot. Le
  plan gelait la section 258-287 pour ne pas perdre de faits verifies ; renommer un mot
  n'en perd aucun, donc le gel n'est pas viole. Route : patch.
- **low** -- `README.md` l. 339, « Ce depot portera deux licences » est suivi de trois
  puces. Verifie : l. 341-351 portent noyau Apache-2.0, extension GPL, module defense.
  Le decompte est defendable puisque la troisieme est un autre depot, mais la phrase et
  la liste se contredisent telles qu'ecrites. Route : patch.
- **defer** -- `README.md` l. 315 conseille `cargo clippy --workspace --all-targets`,
  qui echoue sous WSL sur `libdbus-sys`. Verifie et connu aux entrees 1.4, 1.5 et 1.8.
  Anterieur a cette entree, ecrit a l'entree 1.5 : non cause par ce changement.
- **defer** -- `app/src-tauri/src/lib.rs:4-5` dit que le registre arrive « (etape 2) »
  alors que la nouvelle sequence le place a l'etape 3. Verifie dans le fichier. Devenu
  faux au commit de replanification, qui est la base de cette entree, et dans un fichier
  que ce diff ne touche pas : non cause par ce changement.

### Résolution (2026-10-14)

Les six entrées ci-dessus, toutes corrigées dans `README.md` et nulle part ailleurs :

1. **Schéma 1, direction inerte** — `direction LR` supprimé du sous-graphe QGIS
   (Mermaid l'ignorait, un nœud du sous-graphe pointant au dehors) et les deux
   arêtes `<-->` remplacées par `panneau --> canal` puis `canal --> registre`,
   l'aller-retour passant dans le libellé du nœud `canal`. Le canal occupe
   désormais son propre rang entre les deux grappes, ce que demande l'AC3.
2. **« une seule case est faite »** — reformulé : une seule case *du noyau* est
   faite, et la case `app/` est livrée et fonctionne, mais elle est reportée en
   V2. L'ancienne phrase effaçait les 17 tests décrits vingt lignes plus haut.
3. **Ligne `core/` du tableau des dossiers** — « toute la logique métier y vit »
   passe au futur : registre, journal et terrain sont annoncés comme encore à
   écrire. C'était la seule affirmation au présent sans marqueur du document.
4. **« interface » ambigu** — les deux emplois qui désignaient le panneau QGIS
   disent maintenant « le panneau » ; les cinq autres, qui désignent bien la
   webview ou le produit, sont intacts.
5. **« les 17 tests du moteur »** — seul le mot `moteur` devient `noyau`. La
   section « la recette » ne diffère de `HEAD` que par ce mot, contrôlé au `diff`.
6. **Annonce de la licence** — « deux licences » suivi de trois puces devient
   « trois régimes, et celui de l'extension n'est pas un choix ».

## Design Notes

Le schéma 3 change de sujet, pas seulement de contenu : il décrit désormais le
cycle de vie d'un projet **QGIS** avec le journal Cassini posé à côté, le format
de projet Cassini n'existant plus au plan. C'est la conséquence la plus visible
du virage et elle doit se saisir sans lire la spec.

Le piège est de surcorriger. Deux tiers du README restent vrais : la recette, les
chiffres (17 tests, 30 et 24 contrôles), le démarrage rapide du noyau, la
distinction entre `npm run dev` et `npm run tauri dev`. Les toucher perdrait des
faits vérifiés à l'entrée 1.5 sans rien gagner.

## Verification

**Commands:**
- `cargo test -p cassini-core` -- expected: 17 tests passent
- `cargo fmt --check` et `cargo clippy -p cassini-core --all-targets` -- expected:
  aucun avertissement (`--workspace` échoue sous WSL sur `libdbus-sys`, limite
  connue aux entrées 1.4, 1.5 et 1.8, sans rapport avec du Markdown)
- `cd app && npm run check` -- expected: tout au vert
- `grep -c '^```mermaid' README.md` -- expected: `3`
- `git ls-files | cut -d/ -f1 | sort -u` -- expected: chaque dossier retourné a sa
  ligne dans le tableau des dossiers

**Manual checks (if no CLI):**
- Aperçu Markdown de GitHub, ou poussée sur une branche : les trois schémas
  s'affichent en dessins, aucun ne reste en texte ni n'affiche d'erreur.
- Relire « ce qui marche aujourd'hui » en face de `app/src-tauri/src/lib.rs` et de
  `core/src/basemap.rs` : aucune affirmation qui n'y soit vérifiable.
