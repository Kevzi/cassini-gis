---
title: 'README detaille avec schemas Mermaid'
type: 'chore'
ticket: '5'
created: '2026-10-14'
status: 'ready-for-dev'
baseline_revision: ''
route: 'full'
route_source: 'auto'
risk: 'low'
review: ''
review_source: ''
lenses_ran: []
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
- [ ] `README.md` -- créer, dans cet ordre : titre et présentation ; « ce qui
  marche aujourd'hui » et « ce qui est prévu » adossés aux étapes de §7 ; les
  quatre principes ; les trois schémas ; le tableau des dossiers ; le démarrage
  rapide ; la recette ; la licence et le noyau civil -- première page que voit un
  visiteur, et E5 l'exige.
- [ ] `docs/README.md` -- ajouter une ligne vers `../README.md` en tête de liste
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

## Plan Change Log

## Review Triage Log

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
