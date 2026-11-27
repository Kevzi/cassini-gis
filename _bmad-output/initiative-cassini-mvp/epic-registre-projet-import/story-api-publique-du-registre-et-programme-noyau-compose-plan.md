---
title: 'API publique du registre et programme noyau composé'
type: 'feature'
ticket: '14'
created: '2026-11-27'
status: 'built'
baseline_revision: '331cbd0ae7959f417bed610c4b982c6b740f6dfa'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-commandes-d-un-module-exterieur/change-commandes-d-un-module-exterieur.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Un intégrateur ne peut ajouter ses commandes métier qu'en forkant le dépôt : le binaire `cassini` est binaire seul, chaque point d'entrée bâtit son propre `Registry::standard()`, et rien ne refuse une collision avec le catalogue ni n'annonce un programme dérivé.

**Approach:** Une bibliothèque publique et versionnée (`cassini-core` + une crate `cassini` à lib) où un programme se compose à la compilation : registre officiel + commandes ajoutées `runs_in: core`, passé aux quatre points d'entrée ; le registre refuse au démarrage tout domaine réservé ; `system.about` et `session.hello` annoncent le programme ; la recette du catalogue devient une fonction de la bibliothèque.

## Boundaries & Constraints

**Always:** Aucun chargement dynamique. Un binaire, un registre, quatre points d'entrée identiques (AD-13). `Registry::standard()` et le comportement du programme officiel restent inchangés. Le catalogue public et `core/tests/catalogue.rs` ne changent pas de résultat. Champs d'annonce figés : nom et version du programme, version du noyau, liste des commandes ajoutées. Apache-2.0, aucune dépendance GPL/LGPL.

**Never:** Commande ajoutée déléguée à QGIS ; toucher l'extension QGIS, le panneau ou la grammaire du modèle (9.15, 5.52) ; nouvelle commande au catalogue ; casser les ~850 appels à `Registry::standard()` ; sauter en silence une commande manquante au rejeu.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Commande ajoutée | programme d'exemple + `exemple.echo` | servie par run, serve, mcp, replay ; journalisée et rejouée | — |
| Domaine réservé | ajout de `layer.foo` (ou toute famille) | refus au démarrage, message nommant l'identifiant et la famille | erreur typée, programme ne démarre pas |
| Id déjà au registre | ajout d'un id officiel | refus nommant l'id | idem |
| `runs_in` qgis ou absent | commande ajoutée déléguée | refus nommant l'id | idem |
| Rejeu croisé | journal du programme d'exemple rejoué par le programme officiel | échec nommant la commande et l'étape | `UnknownCommand`, rien n'est sauté |
| Annonce | `system.about` / `session.hello` | nom, version, version du noyau, `added_commands` | officiel : nom `cassini`, liste vide |

</frozen-after-approval>

## Code Map

- `core/src/registry.rs` -- `Registry` (entries, duplicates), `standard()` via `inventory`, `Command`, `RunsIn` ; ajouter constante des familles, `ProgramInfo` et ajout contrôlé.
- `core/src/contract.rs` -- `is_valid_id`, `check_contract` ; réutiliser, ne pas changer ses codes.
- `core/src/commands/system_about.rs` -- unité sans contexte ; doit lire l'identité du programme.
- `core/src/serve.rs` (`hello` ~l.903, résultat ~l.1028) -- ajouter les champs d'annonce ; `core/src/serve/` pour les tests.
- `core/src/journal/mod.rs` (`Replay::load` ~l.1771) -- refuse déjà `UnknownCommand` avec l'étape : ne pas toucher, le prouver par test.
- `core/tests/catalogue.rs` -- parse/normalize/confront à extraire dans `core/src/recipe.rs` (public) ; le test devient un appel mince.
- `cli/src/main.rs` (`run` l.63, `serve` l.127, `replay` l.523, `plan` l.683, `Registry::standard()` aux l.92/137/528/683) et `cli/src/mcp.rs` (l.437, worker l.472, handler l.537) -- faire passer un programme au lieu de `standard()`.
- `cli/Cargo.toml`, `Cargo.toml` (members) -- `cli/src/lib.rs` nouveau ; nouveau membre `tests/programme-exemple`.
- `cli/tests/{parity,end_to_end,mcp}.rs`, `cli/tests/support/` -- modèles pour les tests du programme d'exemple (`CARGO_BIN_EXE_*`).

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/registry.rs` -- `CATALOGUE_FAMILIES`, `ProgramInfo{name,version}`, `Registry::with_added(info, commandes) -> Result<_, ProgramError>` refusant famille, collision et `runs_in` ≠ core ; `registry.program()` et `added_commands()` -- refus au démarrage nommant l'id
- [ ] `core/src/commands/system_about.rs`, `core/src/serve.rs` -- annoncer `program_name`, `program_version`, `core_version`, `added_commands` -- AD-6/AD-13, figés pour 9.15 et 5.52
- [ ] `core/src/lib.rs`, `core/src/recipe.rs`, `core/tests/catalogue.rs` -- recette publique `check_catalogue(&Registry, texte)` ; test officiel inchangé en résultat -- recette paramétrée
- [ ] `cli/src/lib.rs`, `cli/src/main.rs`, `cli/src/mcp.rs` -- `run_program(&Program, args, io…)` ; `main` appelle avec le programme officiel ; mcp reconstruit le programme dans son worker -- quatre entrées, un registre
- [ ] `tests/programme-exemple/` (+ membre du workspace) -- binaire qui enregistre `exemple.echo`, avec son catalogue et sa recette -- preuve par l'exemple
- [ ] `tests/programme-exemple/tests/*.rs`, `core/src/registry.rs` tests -- couvrir la matrice : run/serve/mcp/replay, journal et rejeu, refus, rejeu croisé, annonce, recette paramétrée verte
- [ ] `docs/` -- page « API publique du registre » : surface, version, exigence AD-1 pour l'intégrateur

**Acceptance Criteria:**
- Given le programme d'exemple, when `exemple.echo` passe par run, serve, mcp et replay, then le résultat est identique, journalisé et rejoué à l'identique.
- Given un identifiant dans une famille du catalogue, when le programme démarre, then il refuse en nommant l'identifiant.
- Given un journal du programme d'exemple, when le programme officiel le rejoue, then l'échec nomme la commande manquante.
- Given le programme officiel, when `cargo test` passe, then la recette du catalogue est verte et sa version paramétrée passe sur l'exemple.
- Given `session.hello`, when le programme est officiel, then `added_commands` est vide et le nom est `cassini`.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (thorough, 4 lentilles). Aucun intent_gap ni bad_plan ; 5 correctifs `patch`.

| # | Constat | Verdict | Route | Preuve |
|---|---------|---------|-------|--------|
| 1 | Un installeur qui n'inscrit rien, ou plusieurs commandes, est sauté en silence (`registry.rs:854`, `continue` ; `pop_first` laisse le reste) | medium | patch | Lu : contredit « rien n'est sauté » ; la commande manque aussi à `added_commands` |
| 2 | `CATALOGUE_FAMILIES` n'est testée que dans un sens (chaque famille listée existe) ; une famille officielle nouvelle resterait libre | medium | patch | Lu : la boucle de test `registry.rs:1153` ne vérifie pas l'inverse |
| 3 | Nom ` cassini` ou `cassini ` accepté (trim seulement pour le vide, `registry.rs:845`) ; `is_official` en égalité stricte | medium | patch | Lu : un programme dérivé s'annonce visuellement comme l'officiel |
| 4 | Collision des noms d'outil MCP (`.`→`_`, `mcp.rs:68`) : `layer_add.x` masque `layer.add_x` | medium | patch | Lu : aucune vérification ; refus par `DuplicateId`, sans nouvelle variante |
| 5 | `inventory::submit!` contourne `with_added` ; contrat non vérifié sur les commandes ajoutées | low | patch | Lu : correctif réduit à la doc et à un `check_contract` dans le test de l'exemple, sans surface publique |
| 6 | Version de programme vide acceptée | low | rejeté | Rapport peu probable, correctif = nouvelle branche d'erreur |
| 7 | Espaces de noms de protocole (`session`, `agent`…) non réservés | low | rejeté | Les commandes passent par `command.run` avec un id en paramètre ; l'intention liste 14 familles |
| 8 | Le registre est composé trois fois sur le chemin mcp | low | rejeté | Documenté « appelable plusieurs fois » ; aucun défaut démontré |
| 9 | Registre composé avant l'aiguillage : usage et `model-check` d'un programme refusé sortent en 1 | false | rejeté | Refuser au démarrage sur tous les points d'entrée est l'intention |
| 10 | `[lib]` et `[[bin]]` de même nom (collision `cargo doc`) | low | rejeté | Rare, correctif = configuration en plus |
| 11 | Journalisation vérifiée par `run` seulement, pas par serve/mcp ; `replies[1]` positionnel ; ménage des tests ; négatifs de recette sur params/required | low | rejeté | Journal commun à la session, risque négligeable |
| 12 | `REGISTRY_API_VERSION` absente de about/hello | low | rejeté | Champs d'annonce figés par le plan |
| 13 | Sortie du programme officiel élargie (3 champs) et fiche `system.about` modifiée | false | à signaler | Additif, imposé par le critère d'annonce ; à confirmer par le propriétaire |
| 14 | Diff incomplet (`recipe.rs`, `catalogue.rs` absents), artefacts du patch | false | rejeté | Limite de l'outil de relecture : déplacement pur, test vert |
| 15 | Mise en garde sur `plan` / `serve_with_composer` avec commandes ajoutées | low | rejeté | Grammaire du modèle : ticket 5.52 |

## Design Notes

Ajout par instance, pas par `inventory` : la composition est explicite, vérifiable au démarrage et compatible avec `standard()`. `RunsIn` d'une commande ajoutée doit valoir `Core` explicitement (le défaut `None` est refusé). Seuil de version : `REGISTRY_API_VERSION` distinct de `PROTOCOL_VERSION`.

## Verification

**Commands:**
- `cargo test -p cassini-core --test catalogue` -- expected: vert, inchangé
- `cargo test --workspace` -- expected: vert, y compris `programme-exemple`
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement
