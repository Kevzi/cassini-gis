---
title: 'Passe de nettoyage'
type: 'chore'
ticket: '7'
created: '2026-10-15'
status: 'built'
baseline_revision: 'df8cc21dfe4daedf5aa93079162ff65833ca9045'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `deferred-work.md` garde des constats de l'epic 1 restés sans issue : deux contrôles absents de la recette, une commande de README que WSL ne sait pas exécuter, un Rust non figé, un numéro d'étape périmé dans un commentaire, et des reports sans décision écrite.

**Approach:** une passe, sans nouvelle fonction : on ajoute les deux contrôles à la recette, on corrige README, commentaire et chaîne d'outils, puis on clôt chaque entrée concernée dans `deferred-work.md` par une ligne `status` et `resolution`, sans toucher `/app` ni `/fixtures` autrement que par ces contrôles.

## Boundaries & Constraints

**Always:**
- Les deux contrôles sont des scripts Node sans dépendance nouvelle, hors ligne, dans `app/scripts/`, qui échouent avec un message en français nommant l'id ou le fichier fautif.
- Contrôle des ids : chaque `id="x"` de `app/index.html` doit être visé par `main.ts` (`#x` ou `"x"` passé à `container`) ou par `style.css` (`#x`), et chaque `#x` ou `container: "x"` visé doit exister dans `index.html`. Il entre dans `npm run check`.
- Contrôle de `dist/` : après `npm run build`, chaque adresse `/assets/...` nommée par `dist/index.html` existe dans `dist/`, ainsi que chaque `.js`, `.mjs` et `.css` référencé depuis ces fichiers. Une commande `npm run verify:dist` ; la CI la lance après `npm run build`, en étape distincte de `npm run check`.
- `rust-toolchain.toml` fige `1.93.1` avec `rustfmt` et `clippy`. Le noyau reste à `rust-version = "1.89"`.
- Chaque entrée de `deferred-work.md` sans `status` en reçoit un : `close` avec `resolution` quand la passe la règle, sinon `decision` en une phrase qui dit que le report est maintenu et pourquoi. Aucune entrée n'est supprimée.

**Never:**
- Pas de suppression de `/app` ni de `/fixtures`, pas de nouvelle dépendance, pas de changement de comportement de l'application.
- Pas d'ouverture de l'application construite par la recette (demande un environnement graphique, reste reporté).
- Pas de décision à la place du propriétaire sur la protection des données ou la vectorisation d'images : ces deux entrées gardent leur report, avec la mention de qui l'a décidé.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|--------------|---------------------|------------------|
| Ids concordants | `index.html`, `main.ts`, `style.css` actuels | contrôle vert | — |
| Id orphelin | `main.ts` vise `#carte`, absent du HTML | échec nommant `carte` | message français, sortie non nulle |
| Id mort | un id du HTML visé par personne | échec nommant l'id | idem |
| `dist/` complet | build à jour | contrôle vert | — |
| Asset manquant | `dist/index.html` nomme un fichier absent | échec nommant l'adresse | idem |
| `dist/` absent | aucun build | échec disant de lancer `npm run build` | idem |

</frozen-after-approval>

## Code Map

- `app/index.html`, `app/src/main.ts` (l. 26, 74), `app/src/style.css` -- ids `map` et `status`, déjà concordants ; le commentaire `main.ts:16` cite « Étape 1 » (à reformuler sans numéro).
- `app/scripts/check-offline.mjs` -- modèle de script de recette (exports, messages) à suivre ; ne pas le modifier.
- `app/scripts/verify-ids.mjs`, `app/scripts/verify-dist.mjs` -- nouveaux ; testables en important une fonction pure et en la nourrissant de textes, sur le modèle de `verify-basemap.mjs`.
- `app/package.json` -- `check` + `verify:ids`, `verify:dist`.
- `.github/workflows/recette.yml` -- job `interface` : étapes `npm run build` puis `npm run verify:dist` ; l'étape `rustup component add` devient redondante avec le fichier de chaîne d'outils, la laisser.
- `README.md` (l. 358, tableau de la recette) -- note : `--workspace` demande `libdbus` ; sous WSL, `cargo clippy -p cassini-core -p cassini --all-targets --locked -- -D warnings`. Le compte de contrôles et la liste de `npm run check` y sont mis à jour.
- `rust-toolchain.toml` (racine) -- nouveau.
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- ajouter `status` et `resolution` aux entrées ouvertes ; ne pas réécrire le reste.
- `app/src-tauri/src/lib.rs` -- l'en-tête ne cite plus d'étape : rien à faire, le noter dans la résolution.

## Tasks & Acceptance

**Execution:**
- [ ] `app/scripts/verify-ids.mjs` -- contrôle des ids, fonction pure + tests des quatre lignes de la matrice qui le concernent -- constat 1.8
- [ ] `app/scripts/verify-dist.mjs` -- contrôle des assets de `dist/`, fonction pure + tests -- constat 1.4
- [ ] `app/package.json`, `.github/workflows/recette.yml` -- brancher les deux contrôles -- recette
- [ ] `rust-toolchain.toml`, `README.md`, `app/src/main.ts` -- chaîne d'outils figée, note WSL, commentaire sans numéro d'étape
- [ ] `deferred-work.md` -- clore ou motiver chaque entrée ouverte

**Acceptance Criteria:**
- Given le dépôt après la passe, when on lance `npm run check`, `npm run build` puis `npm run verify:dist`, then les trois passent et les contrôles échouent bien sur les cas faux de la matrice.
- Given `deferred-work.md`, when on le relit, then aucune entrée n'est ouverte sans `status`.
- Given `cargo fmt --all --check`, `cargo clippy` et `cargo test` sous `rust-toolchain.toml`, then ils passent comme avant.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Revue `quick` faite par moi-même : le lancement du relecteur indépendant a été refusé par le classificateur de permissions, et je n'ai pas contourné ce refus.

Passe 1 : 0 high, 0 medium, 2 low, 0 false.

| Constat | Verdict | Route | Evidence |
|---|---|---|---|
| `tsTargets` lit toute chaîne `"#xxx"` comme un id : une couleur hexadécimale mise entre guillemets dans `main.ts` ferait échouer le contrôle des ids | low | rejeté | Réel mais l'échec est bruyant et nomme la chaîne ; aucune couleur de ce genre dans `main.ts` aujourd'hui ; l'éviter demande un analyseur. |
| Critère « `npm run check` passe » non tenu sur ce poste : `verify-basemap.mjs` échoue sous Node 24 | low | defer | Antérieur à 1.7 (même échec sans son diff) ; l'archive vient de Node 22 et la CI en utilise un. Reporté dans `deferred-work.md` avec ce qui le trancherait. |

## Design Notes

Décisions prises sans interroger (passe déléguée) : `1.93.1` est la version utilisée ici et prouvée par la recette ; le contrôle de `dist/` reste séparé de `check` pour que `check` ne demande pas de build ; la protection des données, la vectorisation d'images et la déduplication PMTiles restent reportées, puisque leurs entrées disent déjà à quel moment les rouvrir.

## Verification

**Commands:**
- `npm run check` (depuis `app`) -- expected: tout vert, `verify-ids` compris
- `npm run build && npm run verify:dist` (depuis `app`) -- expected: vert
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
