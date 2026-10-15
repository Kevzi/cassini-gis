---
title: 'Validation des paramètres, erreurs typées et exécutant abstrait'
type: 'feature'
ticket: '4'
created: '2026-10-15'
status: 'built'
baseline_revision: 'b543b01d760330dd3bb269e42ce1f91771f51a0f'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le registre ne sait appeler qu'une commande Rust : aucune commande déléguée à QGIS ne peut s'inscrire, les erreurs n'ont pas la forme de AD-21 (code, message, étape), et il n'existe aucun type de plan ni de référence entre étapes que le rejeu (2.6) et l'agent (epic 5) puissent reprendre.

**Approach:** une commande déléguée s'inscrit avec ses types et ses schémas ; un routeur valide les paramètres puis confie l'appel à un exécutant (Rust en processus, ou QGIS, ici un bouchon) ; un type `Plan` et une vérification statique refusent, avant tout envoi, une étape qui cite une sortie inexistante ou d'un autre type.

## Boundaries & Constraints

**Always:**
- Les paramètres sont validés par les types Rust engendrant le schéma (`serde`, `deny_unknown_fields`), jamais par un second validateur.
- Toute erreur se rend en `{code, message, step?}` : code `domaine.cause` stable, message en français, `step` = numéro d'étape à partir de 1 (AD-21).
- La référence est la chaîne exacte `step:<k>`, valeur d'un paramètre de premier niveau, désignant toute la sortie de l'étape `k` ; `k` doit précéder l'étape qui la cite.
- Deux types sont compatibles si leurs schémas, `$ref` développés, `$schema`, `title` et `description` ôtés, sont égaux.
- Aucune dépendance ajoutée.

**Never:**
- Pas de journal (2.5), de rejeu (2.6), de canal `serve` (2.7), ni d'exécution d'un plan entier.
- Pas de vraie commande de production ; les commandes de test vivent dans les tests de chaque module, jamais dans le registre global.
- Pas de référence à un champ d'une sortie (`step:1.x`) ni de référence imbriquée.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|--------------|---------------------|------------------|
| Paramètres invalides | champ inconnu ou mal typé | `registry.invalid_params`, rien exécuté | message français |
| Commande QGIS sans exécutant | `runs_in` = `qgis`, bouchon | `registry.unavailable` nommant la commande | message français |
| Commande QGIS, exécutant branché | exécutant de test | résultat de l'exécutant | — |
| Plan valide | `step:1` d'un type compatible | `check_plan` accepte | — |
| Étape fantôme | `step:3` dans un plan de 2 | `plan.bad_reference`, étape fautive | refusé avant envoi |
| Type incompatible | sortie objet citée comme chaîne | `plan.type_mismatch` | refusé avant envoi |
| Commande inconnue dans le plan | id absent | `plan.unknown_command` | refusé avant envoi |
| Refus | `Plan::Refusal` | accepté, rien à exécuter | — |

</frozen-after-approval>

## Code Map

- `core/src/registry.rs` -- `Entry` (gagne `validate` et un `handler` optionnel), `register` (modèle de `register_delegated`), `RegistryError` (ajouter `Unavailable`, code `registry.unavailable`), `call` ; ne pas toucher `standard`, `check_contract`.
- `core/src/error.rs` -- nouveau : `ErrorReport {code, message, step}` (sérialisable) construit depuis `RegistryError` et `PlanError`.
- `core/src/exec.rs` -- nouveau : trait `Executor`, `InProcess`, `QgisStub`, `Router` (branche un exécutant QGIS).
- `core/src/plan.rs` -- nouveau : `Plan`, `PlannedCall`, `PlanError`, `Registry::check_plan`, comparaison de schémas.
- `core/src/contract.rs` -- ajouter `contract.runs_in_mismatch` (implémentation Rust déclarée `qgis`, ou déléguée déclarée `core`).
- `core/src/lib.rs` -- trois `pub mod`, seule édition.
- `cli/src/main.rs` -- rendre les erreurs par `ErrorReport` et passer par `Router` ; garder la sortie actuelle.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/registry.rs` -- trait `Delegated` (types + `spec()`), `register_delegated`, `validate_params(id, &Value)`, variante `Unavailable` -- une commande QGIS s'inscrit avec ses schémas sans implémentation Rust
- [ ] `core/src/error.rs` -- `ErrorReport` et conversions -- une seule forme d'erreur (AD-21)
- [ ] `core/src/exec.rs` -- `Executor`, `InProcess`, `QgisStub`, `Router::call` : id inconnu, validation, puis exécutant selon `runs_in` -- sépare Rust et QGIS
- [ ] `core/src/plan.rs` -- `check_plan` : commande connue, références `step:k` avec `k` ≥ 1 et < étape courante, types compatibles ; étape sans référence validée par `validate_params` -- plan refusé avant envoi
- [ ] `core/src/contract.rs`, `core/src/lib.rs` -- règle `runs_in_mismatch`, déclarations de modules
- [ ] `cli/src/main.rs` -- `ErrorReport` et `Router` -- même sortie qu'avant
- [ ] tests dans `exec.rs`, `plan.rs`, `contract.rs` -- une ligne de la matrice chacune, avec registres de test locaux

**Acceptance Criteria:**
- Given une commande aux paramètres invalides, when on appelle le routeur, then le code est stable, le message français, et le résultat n'a rien exécuté (compteur de l'exécutant à 0).
- Given un plan citant une sortie inexistante ou de mauvais type, when on lance `check_plan`, then il est refusé avec l'étape fautive, sans appel d'exécutant.
- Given une commande `qgis` et le bouchon, when on l'appelle, then l'erreur d'indisponibilité nomme la commande.

## Design Notes

Choix faits sans interrogation (le propriétaire a délégué, voir l'epic) : la référence `step:k` du verdict de l'epic 8 est reprise telle quelle, isolée dans `plan.rs` pour être remplaçable ; le typage de sortie se lit dans le schéma de sortie déjà engendré, sans type de couche inventé avant l'epic 9 ; une étape qui porte une référence n'est validée qu'à l'exécution, quand la valeur est résolue (2.5/2.6).

```
{"kind":"calls","calls":[{"command":"layer.add","params":{"path":"mnt.tif"}},
                         {"command":"terrain.slope","params":{"raster":"step:1"}}]}
{"kind":"refusal","reason":"Demande hors du catalogue."}
```

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement
- `cargo run -q --release -p cassini -- run system.about` -- expected: `{"core_version":"0.1.0","protocol_version":1}`

## Review Triage Log

| Finding | Verdict | Route | Evidence |
|---|---|---|---|
| Étape à référence : `validate_params` sauté, littéraux voisins non vérifiés | low | rejeté | Choix documenté dans les Design Notes (validation à l'exécution) ; corriger ajouterait une validation partielle. |
| `step:01` accepté comme `step:1` | low | patch (fait) | `parse_reference` refuse désormais un zéro initial ; cas ajouté au test de l'étape fantôme. |
| `normalize` sensible à l'ordre des tableaux | low | rejeté | Schémas engendrés de façon déterministe ; garde supplémentaire pour un cas non démontré. |
| `PlanError` sans `Display` | low | rejeté | Rendu passe par `ErrorReport` ; cosmétique. |
| `runs_in` absent routé vers InProcess | false | rejeté | `check_contract` signale déjà `contract.runs_in` ; inatteignable sur un registre contrôlé. |
| Double désérialisation des paramètres | low | rejeté | Travail dupliqué sans défaut ; correction ajoutée sans gain. |
