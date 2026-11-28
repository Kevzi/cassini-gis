---
title: 'Grammaire limitee aux commandes connues du modele'
type: 'feature'
ticket: '52'
created: '2026-11-28'
status: 'built'
baseline_revision: '92dccd7d913b409542215b4a63a6b8a5843948d8'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter','edge-case-hunter','verification-gap','intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/change-commandes-d-un-module-exterieur/change-commandes-d-un-module-exterieur.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Un programme noyau dérivé (2.14) ajoute des commandes que le modèle local n'a jamais apprises ; si la grammaire les offre, le modèle les compose sans justesse, sur la composition des plans (risque high).

**Approach:** La déclaration du modèle (`*.modele.json`) gagne un champ facultatif nommant les commandes ajoutées qu'il connaît. La grammaire et le prompt n'incluent une commande ajoutée que si la déclaration la nomme ; un plan qui nomme une commande exclue est rejeté. Catalogue de session, formulaire du panneau et MCP ne changent pas.

## Boundaries & Constraints

**Always:** Les commandes du registre officiel (`added_commands()` vide) restent offertes comme aujourd'hui. Les anciennes déclarations sans le champ restent valides et ne connaissent aucune commande ajoutée. Le même filtre sert le prompt, la grammaire et le préchauffage (`offered`). Le noyau reste sans GPL/LGPL, sans réseau.

**Décision (2026-11-28, tranchée au mieux du projet, §4.4 du changement) :** un GGUF seul, sans déclaration (mode « long prompt », essais, sans promesse), n'a rien qui limite le catalogue : il voit toutes les commandes ajoutées, prompt et grammaire. Dès qu'une déclaration existe, seules les commandes ajoutées qu'elle nomme sont offertes, quel que soit son style de prompt.

**Never:** Ne pas toucher à `serve::catalogue`, au formulaire du panneau ni aux outils MCP. Pas de commande ajoutée codée en dur. Pas de ré-entraînement ni de changement du générateur d'entraînement. Une commande nommée par la déclaration mais absente du registre est ignorée sans erreur.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Déclaration sans le champ | registre dérivé (`exemple.echo`) | `exemple.echo` absent de la grammaire et du prompt ; un plan qui la nomme est rejeté | rejet par la validation du plan |
| Déclaration qui la nomme | `commands: ["exemple.echo"]` | présente dans grammaire et prompt | — |
| Déclaration nomme une inconnue | `commands: ["x.y"]` | ignorée, rien d'ajouté | pas d'erreur |
| GGUF seul | pas de déclaration, registre dérivé | commande ajoutée offerte (long prompt) | — |
| Registre officiel | `added_commands()` vide | grammaire identique à avant | — |
| Catalogue, MCP | dans tous les cas | `exemple.echo` toujours listée | — |
| Ancienne déclaration | JSON sans le champ | acceptée | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/model_list.rs` -- `ModelDeclaration` (:50, `deny_unknown_fields`, 4096 octets max) : ajouter `commands: Vec<String>` en `#[serde(default)]` ; `read_declaration` (:132) ; test de l'exemple du propriétaire (:633) ; `model.list` expose la déclaration (:187).
- `ai/src/plan.rs` -- `offered(registry, available)` (:127) : point d'accroche unique de `plan_grammar_for`, `fixed_system_text` et du préchauffage ; `resolve_model` (:846) jette aujourd'hui la déclaration hors empreintes ; `compose_plan_anchored_cut` (:766) ; tests ≈1600-1770 (`registry()`, `available_without`).
- `core/src/registry.rs` -- `added_commands()` (:835), `with_added` (:844) ; modèle de test (:1154). À réutiliser, pas à changer.
- `cli/src/lib.rs` -- `ChosenModel` (≈480), `choose` (:554), `compose_cut` (:489-520), `warm_turns` (≈387) : faire porter la liste connue.
- `core/src/serve.rs` (`available`, `catalogue`), `cli/src/mcp.rs` (`tools()` :154) -- ne pas toucher.
- `tests/programme-exemple` -- fournit `exemple.echo` ; `cargo test -p programme-exemple` sans GGUF.
- `docs/canal-local.md` (:228-252), `docs/dossier-des-modeles.md` -- décrire le champ.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/model_list.rs` -- champ `commands` facultatif, test d'ancienne et de nouvelle déclaration -- format de la déclaration
- [ ] `ai/src/plan.rs` -- `offered` reçoit l'ensemble connu (`Option`), exclut les commandes ajoutées non nommées ; `resolve_model` le rend -- filtre unique
- [ ] `cli/src/lib.rs` -- `ChosenModel` porte l'ensemble connu jusqu'à `compose_cut` et `warm_turns` -- même filtre partout
- [ ] `ai/src/plan.rs` (tests) -- cas de la matrice avec `Registry::with_added` et `exemple.echo` -- preuve sans GGUF
- [ ] `docs/canal-local.md`, `docs/dossier-des-modeles.md` -- champ `commands` et règle du GGUF seul

**Acceptance Criteria:**
- Given une déclaration sans le champ, when le plan nomme `exemple.echo`, then la grammaire ne l'accepte pas et `check_plan` rejette le plan.
- Given la même commande, when la déclaration la nomme, then elle figure dans la grammaire et le prompt.
- Given les deux cas, then `exemple.echo` reste au catalogue de session et aux outils MCP.
- Given le registre officiel, then la grammaire est identique à celle d'avant le changement.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment) : 1 medium, 2 low corrigés, le reste rejeté ou reporté.

| Constat | Verdict | Route | Preuve / action |
|---|---|---|---|
| Câblage `offered_set` (compose, warmup, warm_inventory) sans test avec registre dérivé | medium | patch | test `offered_set_hides_added_commands_the_model_does_not_know` dans `cli/src/lib.rs` |
| Paragraphe `commands` coupe une liste dans `docs/canal-local.md` | low | patch | déplacé après la liste |
| `let mut run` inutile (clippy) | low | patch | corrigé |
| Le rejet est dans `judge`, pas dans `Registry::check_plan` (écart avec la critère du plan) | low | rejeté | `check_plan` ne connaît pas l'ensemble offert et MCP doit accepter ces commandes ; le rejet vise le modèle local, ce que dit l'intention |
| GGUF seul voit tout (défaut permissif) | low | rejeté | décision du plan, §4.4, écrite dans les docs |
| Identifiant mal écrit dans `commands` ignoré en silence | low | rejeté | décidé au plan ; le correctif ajoute un diagnostic hors périmètre |
| Préchauffage sur le modèle par défaut (`choose(None)`) | low | defer | conception d'avant 5.52 |
| `unwrap_or_else` jamais atteint | low | rejeté | inoffensif, `Some` entre, `Some` sort |
| `judge` ne garde que les commandes ajoutées | false | rejeté | voulu : les autres sont filtrées par la grammaire existante |
| `gen` mot réservé (édition 2024) | false | rejeté | édition 2021 du workspace, les tests compilent |
| Formatage fmt | false | rejeté | `cargo fmt --check` passe |
| Filtre « unique par convention » | low | rejeté | seuls appelants produit : les 3 chemins de `cli`, tous filtrés ; `ai/src/lib.rs:983` est un test |
| Message « pas offerte » partagé, exclusion par `available` non testée | low | rejeté | correctif ajoute de la complexité pour un cas improbable |
| Ancien binaire rejette un champ `commands` (`deny_unknown_fields`) | low | rejeté | rétrogradation hors périmètre |
| Dépendances de test `schemars`/`serde` (`ai`, `cli`) | low | signalé | MIT OR Apache-2.0, déjà dans l'arbre, tests seuls : à valider par le propriétaire (§8) |

## Verification

**Commands:**
- `cargo test -p cassini-core -p cassini-ai -p cassini -p programme-exemple` -- expected: tout vert, sans GGUF
- `cargo fmt --check` et `cargo clippy --all-targets` -- expected: sans avertissement
