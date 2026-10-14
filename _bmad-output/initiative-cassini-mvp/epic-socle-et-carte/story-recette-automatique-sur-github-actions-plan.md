---
title: "Recette automatique sur GitHub Actions"
type: 'chore'
ticket: '6'
created: '2026-10-14'
status: 'built'
baseline_revision: 'ddef1e5280bc00d5c7db6907f2d62e595db4280e'
route: 'oneshot'
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

**Problem:** La recette — tests du noyau, `cargo fmt`, `cargo clippy`, typage
strict et garde hors ligne — ne tourne qu'à la main ; rien n'empêche une poussée
de casser l'un des cinq contrôles sans que personne le voie (epic 1, E4).

**Approach:** Un workflow GitHub Actions unique, `.github/workflows/recette.yml`,
lancé à chaque poussée, qui enchaîne les cinq contrôles tels que le README les
décrit et échoue dès que l'un d'eux échoue. Un emplacement nommé y est réservé
au contrôle du Python de l'extension, que l'epic 9 remplira. Le README cesse de
dire que la recette se lance à la main.

</frozen-after-approval>

## Tasks & Acceptance

**Execution:**
- [x] `.github/workflows/recette.yml` -- créer le workflow : déclencheurs `push`,
  `pull_request`, `workflow_dispatch` ; job `noyau` (Ubuntu) : `cargo fmt --all
  --check`, `cargo test -p cassini-core --locked`, puis dépendances système de
  Tauri v2 et `cargo clippy --workspace --all-targets --locked -- -D warnings` ;
  job `interface` : Node, `npm ci` et `npm run check` dans `app` ; job réservé
  `extension`, en commentaire, pour l'epic 9 -- les cinq contrôles automatiques,
  le point d'accroche prévu.
- [x] `README.md` -- section « La recette » et tableau des étapes : la recette
  tourne à chaque poussée sur GitHub Actions -- le README dit vrai.

**Acceptance Criteria:**
- Given une poussée saine, when le workflow tourne, then les jobs passent au vert.
- Given un contrôle cassé (mise en forme, test, avertissement clippy, erreur de
  type, adresse réseau dans le style), when la commande correspondante tourne,
  then elle sort en erreur, ce qui fait échouer son job.
- Given le noyau, when il est testé, then aucune dépendance système de Tauri
  n'est requise pour `fmt` et `test` (principe 4).

## Implementation Notes

Route oneshot : un fichier YAML et quelques lignes de README, sans code produit.

- Deux jobs plutôt qu'un : `interface` ne dépend pas de Rust, et `noyau` lance
  `fmt` et les tests avant d'installer les bibliothèques de Tauri (principe 4).
- `mkdir -p app/dist` avant clippy : `tauri::generate_context!` exige que
  `frontendDist` (`../dist`) existe à la compilation ; il est ignoré par git.
- Seules les actions officielles `actions/checkout@v4` et `actions/setup-node@v4`
  (MIT) ; pas d'action de cache tierce (`Swatinem/rust-cache` est sous LGPL).
- Point d'accroche de l'epic 9 : job `extension` en commentaire.
- Preuve locale du rouge, chaque contrôle cassé exprès puis restauré : `fmt` →
  1, test → 101, clippy (`-p cassini-core`) → 101, `tsc` → 2, adresse
  `https://` dans le style → 1. Clippy sur tout le workspace n'a pas pu tourner
  en local (bibliothèques webkit absentes du WSL) : le premier run GitHub le
  prouvera.
- Fichiers : `.github/workflows/recette.yml` (créé), `README.md`.

## Review Triage Log

Revue `quick`, passe 1 : 2 medium, 2 low, 1 rejetée.

- medium, patch -- tableau du README sans `--locked`, `--all` ni `-D warnings` :
  clippy à la main sortait 0 avec des avertissements. Commandes alignées sur le workflow.
- medium, patch -- `cancel-in-progress` annulait aussi les runs de `main`, seule
  branche poussée : un commit pouvait rester sans verdict. Annulation hors `main` seulement.
- low, defer -- Rust non figé (`rust-toolchain.toml` absent) : un nouveau clippy
  peut rougir le run sans changement de code. Figer la version touche aussi le
  poste local, décision à part ; inscrit dans `deferred-work.md`.
- low, patch -- cases du plan non cochées. Cochées.
- low, rejetée -- « le README annonce une recette jamais vue tourner » : le
  workflow tourne par construction dès la poussée ; le premier run vert reste la
  vérification manuelle prévue, signalée au propriétaire.

## Verification

**Commands:**
- `cargo fmt --all --check`, `cargo test -p cassini-core --locked` -- succès en local.
- `npm run check` dans `app` -- succès en local.
- Casser tour à tour chaque contrôle en local -- chaque commande sort en erreur.

**Manual checks (if no CLI):**
- Après fusion et poussée de `main` (sur accord du propriétaire) : le run
  GitHub Actions est vert.
