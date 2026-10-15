---
title: 'Ajouter une commande sans toucher un fichier partagé'
type: 'feature'
ticket: '2'
created: '2026-10-15'
status: 'built'
baseline_revision: '8083c60b2d5f50c19a53ae8e198ef89a136fbe08'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** aujourd'hui une commande s'inscrit à la main dans `Registry::standard()` (`core/src/registry.rs`), et un fichier neuf demande une ligne `mod` dans `core/src/lib.rs`. Les epics 9, 5 et 4 ajoutent des commandes en parallèle : ces deux fichiers partagés seraient des points de collision (Done when 2).

**Approach:** chaque commande vit dans un fichier de `core/src/commands/` et s'inscrit elle-même par le crate `inventory` (licence donnée le 2026-10-15) ; un `build.rs` déclare les modules de ce dossier, donc ni `lib.rs` ni le registre ne changent quand on ajoute un fichier. `Registry::standard()` lit ce que les commandes ont inscrit. `system.about` quitte `registry.rs` pour son propre fichier.

## Boundaries & Constraints

**Always:**
- Seule dépendance ajoutée : `inventory` (MIT OR Apache-2.0). `build.rs` n'emploie que la bibliothèque standard.
- L'ordre des commandes dans le registre ne dépend pas de l'ordre d'inscription ni de l'ordre des fichiers sur le disque.
- Un nom de fichier qui n'est pas un identifiant Rust valide fait échouer la construction avec un message français nommant le fichier : jamais ignoré en silence (AD-21).
- La preuve de l'objectif est dans cette story même : la commande de test est un fichier neuf, et le diff ne touche ni `lib.rs` ni `registry.rs` pour elle.
- Le binaire `cassini` voit les commandes du noyau, sans quoi l'éditeur de liens les écarte : un test de la CLI le vérifie.

**Never:**
- Pas de validation du contrat d'une commande (2.3), pas de refus d'id en double (2.3, voir `deferred-work.md`), pas de nouvelle commande de production.
- La commande de test n'est jamais compilée hors des tests.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|--------------|---------------------|------------------|
| Commande ajoutée | un fichier neuf dans `core/src/commands/` qui inscrit `test.echo` | `Registry::standard()` la liste et l'appelle, sans modification d'un fichier existant | — |
| Commande existante | `system.about`, déplacée dans `commands/system_about.rs` | même résultat qu'en 2.1 | — |
| Depuis le binaire | `cassini run system.about` | même sortie qu'en 2.1 | — |
| Nom de fichier invalide | `commands/system-about.rs` | construction refusée, message français nommant le fichier | `build.rs` échoue |
| Dossier avec `mod.rs` | `commands/geo/mod.rs` | module `geo` déclaré, ses sous-modules à sa charge | — |

</frozen-after-approval>

## Code Map

- `core/src/registry.rs` -- `Registry::standard()` inscrit `SystemAbout` à la main ; `AboutParams`, `AboutOutput` et `SystemAbout` y vivent : à déplacer. `Registry::register` et `Command` ne changent pas.
- `core/src/lib.rs` -- `pub mod basemap; pub mod registry;` : un seul ajout, `pub mod commands;`, une fois pour toutes.
- `core/Cargo.toml` -- ajouter `inventory` ; le commentaire de licences cite déjà la décision de 2.1.
- `cli/src/main.rs` -- `run` appelle `Registry::standard()` : son test `about_prints_versions` prouve déjà que l'éditeur de liens garde `system.about` ; garder ce test tel quel.
- Nouveaux : `core/build.rs`, `core/src/commands/mod.rs` (généré par inclusion, voir Design Notes), `core/src/commands/system_about.rs`, `core/src/commands/test_echo.rs`.
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- id en double : traité en 2.3, ne pas y toucher ici.

## Tasks & Acceptance

**Execution:**
- [ ] `core/Cargo.toml` -- ajouter `inventory` -- licence MIT OR Apache-2.0 donnée par le propriétaire
- [ ] `core/build.rs` -- lister `src/commands/*.rs` (sauf `mod.rs`) et `src/commands/*/mod.rs`, trier, refuser un nom invalide avec un message français, écrire dans `OUT_DIR` un fichier `commands_mods.rs` de `#[path = "…"] pub mod nom;` ; `cargo:rerun-if-changed=src/commands` -- supprime le fichier partagé de déclaration
- [ ] `core/src/commands/mod.rs` -- `include!(concat!(env!("OUT_DIR"), "/commands_mods.rs"));` -- seul fichier qui lit la liste générée
- [ ] `core/src/registry.rs` -- type `Registration` (une fonction qui reçoit `&mut Registry`), `inventory::collect!`, `Registry::standard()` parcourt les inscriptions ; retirer `SystemAbout` et ses types -- le registre ne connaît plus aucune commande
- [ ] `core/src/commands/system_about.rs` -- déplacer `system.about` et son `inventory::submit!`
- [ ] `core/src/commands/test_echo.rs` -- commande `test.echo` sous `#[cfg(test)]`, en fichier neuf, jamais dans le produit
- [ ] `core/src/lib.rs` -- `pub mod commands;` -- dernière édition d'un fichier partagé
- [ ] tests -- `test.echo` apparaît dans `Registry::standard()` et répond ; `system.about` donne le même résultat qu'en 2.1 ; la liste est triée ; le test de la CLI de 2.1 passe toujours

**Acceptance Criteria:**
- Given un fichier neuf dans `core/src/commands/`, when on compile et teste, then sa commande est au registre sans qu'un fichier existant ait été modifié.
- Given le binaire `cassini` compilé en release, when on lance `cassini run system.about`, then il répond comme en 2.1.
- Given un fichier `system-about.rs`, when on compile, then la construction échoue avec un message français qui le nomme.

## Implementation Notes

Implémenté par un sous-agent, complété par moi. Ajout hors plan : la logique de `build.rs` est dans `core/build_support.rs`, incluse sous `cfg(test)` par `core/src/commands/mod.rs` (et non par `lib.rs`), parce que `build.rs` ne passe pas par `cargo test` et que deux lignes de la matrice (nom invalide, dossier `mod.rs`) n'avaient sinon aucun test. Le linker ne supprime pas `system.about` en release : `linkme` n'est pas nécessaire ; la recette GitHub vérifie maintenant cette commande en release. `lib.rs` n'a reçu que `pub mod commands;`. 29 tests du noyau, 5 de la CLI.

## Plan Change Log

## Review Triage Log

Relecture quick, 10 constats.
- medium, patch : seconde édition de `lib.rs` (inclusion de `build_support`) contre la règle du plan ; déplacée dans `commands/mod.rs`, `lib.rs` n'a plus que `pub mod commands;`.
- medium, patch : un dossier sans `mod.rs` était ignoré en silence ; erreur nommant le dossier, test ajouté (AD-21).
- medium, patch : les mots réservés de Rust passaient pour des noms valides ; refusés, test ajouté.
- low, patch : la suggestion de nom du message pouvait elle-même être invalide ; proposée seulement si valide.
- low, patch : le chemin de `#[path]` n'était pas échappé ; formaté par `{:?}`.
- medium, patch : le test de `test.echo` vivait dans `registry.rs` ; déplacé dans `test_echo.rs`.
- medium, patch : aucun contrôle automatique en release ; étape ajoutée à `recette.yml`.
- low, rejeté : `test_echo` figure dans la liste générée en release ; cosmétique, le fichier est vide hors test.
- low, rejeté : `standard_registry_is_sorted_by_id` ne teste que la `BTreeMap` ; le tri est structurel, un test plus fin n'ajouterait rien.
- low, rejeté : le chemin « construction échoue » n'est testé qu'au niveau de la fonction ; le panic de `build.rs` a été vu à la main (`system-about.rs`).

## Design Notes

`inventory` collecte, au chargement du binaire, les inscriptions faites dans chaque fichier ; le registre les lit une seule fois dans `standard()`. Le risque connu est l'éditeur de liens, qui peut écarter un fichier objet qu'aucun symbole ne référence : dans une crate bibliothèque liée dans `cassini`, il faut donc un test qui passe par le binaire (celui de la CLI) et, en release, la vérification de l'acceptation. Si le risque se réalise, le repli est `linkme`, de même licence, sans toucher la forme de `register`.

```rust
// dans un fichier de core/src/commands/
inventory::submit! { Registration(|r| r.register(SystemAbout)) }
```

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement
- `cargo run -q --release -p cassini -- run system.about` -- expected: `{"core_version":"0.1.0","protocol_version":1}`
- `git diff --stat main` -- expected: ni `core/src/lib.rs` ni `core/src/registry.rs` ne changent à cause de `test.echo` (leurs seules éditions sont celles listées aux tâches)

**Manual checks (if no CLI):**
- Renommer à la main un fichier de `commands/` en `x-y.rs` et vérifier le message de la construction, puis le remettre.
