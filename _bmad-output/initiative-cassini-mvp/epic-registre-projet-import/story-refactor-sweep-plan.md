---
title: 'Refactor sweep : nettoyage de fin d''epic 2'
type: 'refactor'
ticket: '9'
created: '2026-10-15'
status: 'built'
baseline_revision: '629d811340362c724648402de272d2277a213220'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: []
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'epic 2 laisse une lecture du journal écrite trois fois, une documentation du contrat devenue fausse (règle 10, README, commentaire de la coquille) et des constats de revue dispersés dans huit plans sans bilan unique.

**Approach:** un nettoyage sans changement de comportement : factoriser la lecture des lignes du journal, remettre à jour la règle 10, le README et le commentaire de `app/src-tauri/src/lib.rs`, puis écrire dans ce plan le sort de chaque constat différé ou rejeté pendant l'epic. Périmètre fixé ici, rien d'autre.

</frozen-after-approval>

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/journal.rs` -- une seule fonction rend les lignes non vides d'un texte de journal ; `count_lines`, `journal_ids` et `Replay::load` s'en servent -- supprime le triple codage, comportement identique
- [ ] `docs/regle-de-redaction-des-commandes.md` -- règle 10 : la normalisation des unités est faite dans le harnais de l'epic 8 (`canonical`, entrée 8.3) ; dire ce que le noyau compare et ne compare pas
- [ ] `README.md` -- état des étapes et schéma : registre, journal, `run`, `serve`, `replay` faits ; MCP et agent restent prévus ; nombre de tests relu par la recette
- [ ] `app/src-tauri/src/lib.rs` -- commentaire d'en-tête sans numéro d'étape (constat différé de 1.9)
- [ ] ce plan, `## Implementation Notes` -- une ligne par constat différé ou rejeté des huit plans de l'epic : traité ici, ou écarté avec la raison

**Acceptance Criteria:**
- Given le journal d'un projet, when on le compte, l'énumère ou le rejoue, then les résultats sont ceux d'avant le changement et les tests de `core` et `cassini` passent sans modification.
- Given la règle 10, le README et le commentaire de la coquille, when on les relit, then aucun ne décrit comme futur ce que l'epic a livré.
- Given les constats des huit plans, when on lit les notes, then chacun est traité ou écarté par écrit.

## Implementation Notes

Oneshot : changement de moins de cent lignes, sans comportement nouveau. Aucune dépendance ajoutée.

Fichiers touchés : `core/src/journal.rs` (`content_lines`), `docs/regle-de-redaction-des-commandes.md` (règle 10), `README.md` (état, schéma, 110 tests), `app/src-tauri/src/lib.rs` (commentaire). `count_lines` lit toujours des octets, passés en `from_utf8_lossy`, pour ne pas changer son comportement sur un journal non UTF-8.

**Sort de chaque constat de l'epic** (tous les « rejeté » sont déjà motivés dans les Review Triage Log des plans) :

- 2.1, `register` écrase un id en double : traité en 2.3, entrée close dans `deferred-work.md`.
- 2.1, `std::env::args()` panique sur un argument non UTF-8 : écarté, appelé par des programmes.
- 2.2, `test_echo` listé en release ; test de tri ; panic de `build.rs` vu à la main : écartés, cosmétiques.
- 2.3, détection de géométries heuristique, doublons, numérotation du doc : écartés, limites connues et décrites.
- 2.4, `PlanError` sans `Display`, double désérialisation, tableaux de `normalize` : écartés, sans défaut démontré.
- 2.5, chemin absolu par `result`, journal relu en entier, `.cassini` créé sans écriture : écartés, conformes au plan ; les commandes de l'epic 9 rendent des noms relatifs (AD-10).
- 2.6, ligne tronquée, `replay/` jamais vidé, repère `out:` : écartés ; **à reprendre si l'epic 9 écrit des sorties**, la politique « sauter ou refuser » une ligne tronquée restant à décider.
- 2.7, ligne sans borne de taille, hello sur projet verrouillé : écartés, pair local de confiance.
- 2.8, commandes `test.*` hors du binaire : limite connue, couverte dans `core`.
- Report de 1.9 (commentaire de `lib.rs`) : traité ici. Les constats 1.4/1.5/1.6 (contrôle de `dist/`, ids HTML, clippy sous WSL, `rust-toolchain.toml`) sont de l'epic 1 : restent dans `deferred-work.md`.

Relecture par le développeur seul (aucun sous-agent lancé : la session n'en autorise pas sans demande explicite).

## Review Triage Log

Rien de relevé : les 110 tests, `cargo fmt` et `cargo clippy` sont propres, `cassini-app` compile, le comportement du journal est inchangé.

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy -p cassini-core -p cassini --all-targets --locked -- -D warnings` -- expected: aucun avertissement
