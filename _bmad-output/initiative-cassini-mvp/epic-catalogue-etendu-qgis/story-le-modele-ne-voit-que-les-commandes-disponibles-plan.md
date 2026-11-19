---
title: 'Le modèle ne voit que les commandes disponibles'
type: 'feature'
ticket: '5'
created: '2026-11-19'
status: 'built'
baseline_revision: '3681a2a0bf9275f3661025833824ee33732e29c5'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le prompt fixe et la grammaire de l'agent listent tout le registre, que l'extension ait annoncé ou non chaque commande (10.4) : le modèle peut proposer une commande indisponible, et le catalogue étendu allonge la tête du prompt, donc la durée d'une phrase sur processeur (critère de 30 s à 4 cœurs).

**Approach:** Le noyau passe à l'agent l'ensemble des commandes disponibles (la règle `is_available` de `serve.rs`). `ai` n'y décrit que celles-là, dans le prompt fixe et dans la grammaire, en ordre stable : deux sessions qui annoncent le même ensemble donnent la même tête de prompt, donc le même préfixe repris par le cache de 5.12.

## Boundaries & Constraints

**Always:** texte de tête déterministe pour un ensemble donné (ordre du registre) ; `data.reproject` suit la règle commune ; la tête reste identique pour deux inventaires (test existant à conserver) ; la mesure à 4 cœurs se consigne dans `docs/mesure-duree-par-phrase.md`, avec les commandes de son mode opératoire.

**Never:** toucher `SHORT_SYSTEM`, `judge`, `retry_turns`, les quatre `SLOT_*`, les coupes d'inventaire (5.19) ou le critère de 30 s ; ajouter une clé explicite de cache (le préfixe de jetons fait la clé) ; modifier le garde `verify` de `plans.rs` ; réseau.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Tout annoncé | toutes les commandes disponibles | tête et grammaire comme aujourd'hui | — |
| Commande indisponible | `data.reproject` non annoncée | absente du prompt fixe et de la grammaire | un plan qui la nomme reste refusé par `verify` |
| Même ensemble, deux sessions | mêmes commandes annoncées | tête identique octet pour octet, préfixe repris par le cache | — |
| Ensembles différents | un jeu annoncé en moins | têtes différentes, la première phrase repart de zéro | — |
| Ensemble vide de commandes QGIS | aucune annonce QGIS | seules les commandes du noyau sont décrites | — |

</frozen-after-approval>

## Code Map

- `ai/src/plan.rs` -- `offered(registry)` (~l.117), `plan_grammar`/`plan_grammar_built`, `fixed_system_text` (~l.381), `first_turns_anchored`, `compose_plan*` (l.652-760) : filtrer par l'ensemble disponible ; tests l.~1395-1860 dont `the_fixed_head_is_the_same_for_two_inventories_…`.
- `core/src/serve/plans.rs` -- trait `Composer` (l.31), `propose` (l.130) : transmettre l'ensemble ; `verify` (l.104) inchangé.
- `core/src/serve.rs` -- `is_available` (l.988), `catalogue` (l.1035), `FixedComposer` de test (l.1926) : calculer l'ensemble et adapter les implémentations de test.
- `cli/src/main.rs` -- `ModelComposer::compose` (l.269), `ChosenModel::compose_cut` (l.382) : relayer l'ensemble jusqu'à `compose_plan_anchored_cut`.
- `ai/tests/cache_reel.rs`, `ai/tests/duree_reel.rs` -- essais réels ignorés (`CASSINI_TEST_MODEL`, `CASSINI_TEST_THREADS`) ; ne pas changer leur contrat.
- `docs/mesure-duree-par-phrase.md` -- y consigner la mesure.
- `core/src/commands/data_reproject.rs` -- `RunsIn::Qgis` : sans changement.

## Tasks & Acceptance

**Execution:**
- [ ] `ai/src/plan.rs` -- `offered` prend l'ensemble disponible ; prompt fixe et grammaire n'en décrivent que les membres ; tests de la matrice (absence dans `cached_head` et grammaire, égalité octet pour octet, ensembles différents)
- [ ] `core/src/serve/plans.rs`, `core/src/serve.rs`, `cli/src/main.rs` -- `Composer::compose` reçoit l'ensemble, calculé par `is_available` ; adapter `FixedComposer`
- [ ] `ai/src/lib.rs` -- test (sans modèle) que deux têtes identiques donnent le même préfixe repris par `Checkpoints::best`, et qu'une tête différente ne le reprend pas
- [ ] `docs/mesure-duree-par-phrase.md` -- mesure avec le catalogue complet à 4 cœurs (`ProcessorAffinity=0x55`), consignée avec date et commande

**Acceptance Criteria:**
- Given une commande non annoncée, when l'agent compose un plan, then elle n'apparaît ni dans le prompt fixe ni dans la grammaire.
- Given deux sessions annonçant le même ensemble, when on construit leur tête de prompt, then elle est identique octet pour octet et le cache reprend le préfixe.
- Given le catalogue complet à 4 cœurs, when on mesure une phrase, then la durée est consignée dans le document de mesure.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (lentille : quick). Verdicts : 0 high, 0 medium, 3 low, 3 false.

| Verdict | Route | Constat et preuve |
|---|---|---|
| false | rejeté | « La mesure n'exerce pas le changement » : le ticket demande la mesure du catalogue complet à 4 cœurs, soit l'ensemble entier annoncé, que `duree_reel` compose sans filtre ; le doc dit lui-même qu'un ensemble réduit n'est pas mesuré. |
| false | rejeté | `mesure-4.txt` non commité : le nom n'apparaît que dans la commande PowerShell du mode opératoire (`docs/mesure-duree-par-phrase.md:685`), aucun fichier n'est attendu. |
| false | rejeté | Écart de frontmatter du plan entre diff et disque : tenue de comptes, le diff a été écrit avant le passage à `in-review`. |
| low | rejeté | `ChosenModel::compose` passe `None` (tout le registre) : c'est le chemin de la CLI hors session, sans `hello` donc sans ensemble annoncé ; le chemin de session (`ModelComposer::compose`) passe l'ensemble réel. |
| low | rejeté | Disponibilité qui change entre le prompt et `verify` au cours d'un même `propose` : `verify` reste le garde, et le plan refusé l'est avant affichage ; correctif = ajouter de la complexité pour un cas improbable. |
| low | rejeté | `#[allow(clippy::too_many_arguments)]` et test à jetons fictifs : lint demandé propre par CLAUDE.md §9 et satisfait ; test sans modèle voulu par le plan. |

## Design Notes

L'ensemble passe en argument, sans clé de cache : le moteur est lié au processus, la clé reste le préfixe de jetons. Deux ensembles qui alternent dans un même processus s'évincent (un seul `SLOT_HEAD`) ; une session n'en annonce qu'un, donc le cas est accepté.

## Verification

**Commands:**
- `cargo test -p cassini-core --locked --lib` -- expected: succès
- `cargo test -p cassini-ai --lib plan::` et `cargo test -p cassini-cli` -- expected: succès (variables d'environnement du poste, voir la mémoire des outils locaux)
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement

**Manual checks (if no CLI):**
- Mesure réelle : `duree_reel` à 4 cœurs, résultat dans `docs/mesure-duree-par-phrase.md`, sous 30 s hors première phrase.
