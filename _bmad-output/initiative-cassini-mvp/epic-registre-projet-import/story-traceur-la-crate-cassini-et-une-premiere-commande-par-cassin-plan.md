---
title: 'Traceur : la crate cassini et une première commande par cassini run'
type: 'feature'
ticket: '1'
created: '2026-10-15'
status: 'built'
baseline_revision: '6a04be445b4e56f42b8d59b2fe5d2ef88d7d5d1e'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le registre de commandes n'existe pas : `core` n'a que le fond de carte, et il n'y a ni crate `cli` ni binaire `cassini`. Rien ne prouve encore qu'une commande typée, à schémas engendrés, s'appelle de bout en bout depuis le terminal sans QGIS.

**Approach:** le chemin le plus mince. Un type `Command` et un `Registry` dans `cassini-core`, une seule commande Rust `system.about` (version du noyau et du protocole), et une crate `cassini` (`/cli`) dont la sous-commande `run <id> [json]` appelle le registre et imprime le résultat en JSON. Les schémas de paramètres et de résultat sont engendrés par `schemars`, jamais écrits à la main (AD-6).

## Boundaries & Constraints

**Always:**
- Dépendances et licences données par le propriétaire le 2026-10-15 : `schemars` (MIT), `serde` et `serde_json` (MIT OR Apache-2.0). Aucune autre dépendance, `clap` compris : l'analyse des arguments est écrite à la main avec la bibliothèque standard.
- Le noyau ne dépend ni de QGIS ni de `cassini-ai` (AD-1) ; `core` reste compilable et testable seul.
- Schémas en JSON Schema 2020-12, dialecte vérifié par un test ; commande en `domaine.verbe`, anglais, snake_case ; descriptions fr et en ; `runs_in` valant `core`.
- Résultat sur la sortie standard, traces et erreurs sur la sortie d'erreur (AD-2) ; une erreur porte un code `domaine.cause` stable, un message français et sort avec le code 1 (AD-21).
- Code et noms en anglais, commentaires et messages utilisateur en français ; `cargo fmt` et `cargo clippy` sans avertissement.

**Never:**
- Pas d'enregistrement automatique des commandes (entrée 2.2), pas de validation de schéma ni de plan (2.3, 2.4), pas de journal ni de projet (2.5), pas de `serve`, `mcp` ni `replay`.
- Aucune commande autre que `system.about`, aucun accès au réseau, aucune modification du fond de carte ni de `app/`.

## I/O & Edge-Case Matrix

| Scénario | Entrée | Comportement attendu | Gestion d'erreur |
|----------|--------|---------------------|------------------|
| Appel normal | `cassini run system.about` | JSON `{"core_version":"0.1.0","protocol_version":1}` sur la sortie standard, code 0 | — |
| Commande inconnue | `cassini run foo.bar` | rien sur la sortie standard ; sur la sortie d'erreur, JSON `{"code":"registry.unknown_command","message":"…"}`, code 1 | message français nommant l'id |
| Paramètres invalides | `cassini run system.about '{"x":1}'` | idem avec `registry.invalid_params` | champ inconnu refusé |
| JSON illisible | `cassini run system.about '{'` | idem avec `registry.invalid_params` | — |
| Sans argument | `cassini run` | usage en français sur la sortie d'erreur, code 2 | — |

</frozen-after-approval>

## Code Map

- `Cargo.toml` -- `members = ["core", "app/src-tauri"]` : y ajouter `cli`.
- `core/Cargo.toml` -- aucune dépendance aujourd'hui (commentaire à mettre à jour) : ajouter `serde`, `serde_json`, `schemars`.
- `core/src/lib.rs` -- `pub mod basemap;` et `VERSION` (`env!("CARGO_PKG_VERSION")`) ; y déclarer `pub mod registry;`. `basemap.rs` donne le style : enums d'erreur typées par module avec `Display`, commentaires français.
- `app/src-tauri/Cargo.toml` -- dépend déjà de `serde` et `serde_json` (`1`) : mêmes versions, `Cargo.lock` partagé.
- `.github/workflows/recette.yml` -- l'étape « Tests du noyau » lance `cargo test -p cassini-core --locked` : y ajouter `-p cassini`. Clippy couvre déjà tout le workspace.
- `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md` -- AD-6, AD-13, AD-21 : source des règles de commande et d'erreur.

## Tasks & Acceptance

**Execution:**
- [ ] `core/Cargo.toml` -- ajouter `serde` (derive), `serde_json`, `schemars` ; retirer « aucune dépendance » du commentaire en citant la décision de licence -- CLAUDE.md §8
- [ ] `core/src/registry.rs` -- créer : `PROTOCOL_VERSION: u32 = 1` ; `RunsIn` (`Core`) ; `CommandSpec` (id, description_fr, description_en, runs_in, schémas de paramètres et de résultat engendrés) ; trait `Command` (types `Params` et `Output` à schémas engendrés, `spec()`, `run()`) ; `RegistryError` en énumération typée avec `code()` en `registry.unknown_command` ou `registry.invalid_params` et un message français ; `Registry` (`new`, `register`, `spec`, `call(id, json) -> Result<Value, RegistryError>`, `standard()` qui porte `system.about`) -- le contrat de AD-6, à la taille minimale
- [ ] `core/src/lib.rs` -- déclarer le module `registry` -- exposer l'API
- [ ] `core/src/registry.rs` (tests) -- schémas en 2020-12 ; `system.about` rend la bonne forme ; id inconnu, champ inconnu et JSON illisible donnent le bon code -- au moins un test par commande
- [ ] `cli/Cargo.toml` et `cli/src/main.rs` -- créer la crate `cassini` (binaire `cassini`, dépend de `cassini-core` et de `serde_json`) ; `run(args, out, err) -> i32` testable, `main` ne fait que l'appeler ; sous-commande `run <id> [json]` -- le premier point d'entrée du registre (AD-13)
- [ ] `cli/src/main.rs` (tests) -- les cinq lignes de la matrice, sorties et codes de sortie vérifiés
- [ ] `Cargo.toml` -- ajouter `cli` aux `members` -- la crate fait partie du workspace
- [ ] `.github/workflows/recette.yml` -- tester aussi `-p cassini` sans QGIS -- la recette couvre le nouveau point d'entrée

**Acceptance Criteria:**
- Given une machine sans QGIS, when on lance `cargo run -p cassini -- run system.about`, then la version du noyau et celle du protocole s'impriment en JSON et le code de sortie est 0.
- Given le registre standard, when on lit les schémas de `system.about`, then ils ont été engendrés depuis les types Rust et déclarent le dialecte 2020-12.
- Given un appel invalide, when la commande échoue, then rien n'est écrit sur la sortie standard et l'erreur porte un code stable avec un message français.

## Implementation Notes

Implémenté par un sous-agent, vérifié par moi : 26 tests, `cargo fmt`, clippy sur tout le workspace, `cassini run system.about`. `schemars` en `1` (le 1.x engendre le 2020-12, vérifié par test). `rustfmt` installé sur le poste pendant la vérification. Fichiers : `core/src/registry.rs`, `core/src/lib.rs`, `core/Cargo.toml`, `cli/`, `Cargo.toml`, `Cargo.lock`, `.github/workflows/recette.yml`.

## Plan Change Log

## Review Triage Log

Relecture quick, 9 constats.
- medium, patch : l'échec de sérialisation du résultat était rendu en `registry.invalid_params` ; variante `ResultNotSerializable` et code `registry.result_not_serializable` ajoutés.
- medium, patch : le message « français » incluait le texte anglais de serde ; la phrase est française et le détail technique est entre parenthèses.
- medium, patch : `unwrap_or(Value::Null)` sur le schéma pouvait enregistrer une commande sans schéma ; remplacé par un `expect` qui échoue fort (AD-21).
- medium, patch : un résultat non écrit sur la sortie standard rendait quand même 0 ; rend désormais 1.
- low, patch : version `0.1.0` écrite en dur dans le test de la CLI ; remplacée par `cassini_core::VERSION`.
- low, patch : le test de schéma ne vérifiait pas `additionalProperties: false` ; assertion ajoutée.
- medium, différé : un id en double remplace silencieusement la commande existante (`register`). 2.2 change l'enregistrement et 2.3 en contrôle le contrat : entrée dans `deferred-work.md`.
- low, rejeté : `std::env::args()` panique sur un argument non UTF-8 ; la correction ajouterait plus de code qu'elle n'en vaut pour un outil appelé par des programmes.
- false : « le test JSON illisible du registre manque » ; `Registry::call` prend déjà un `Value`, la ligne de la matrice est couverte par le test de la CLI, comme le prévoit la matrice.

## Design Notes

Le trait `Command` porte les types de paramètres et de résultat pour que les schémas viennent du compilateur ; le registre en garde une version à types effacés (`serde_json::Value` en entrée et en sortie) pour ne rien savoir des commandes. 2.2 changera seulement la façon d'inscrire une commande, 2.3 ajoutera les contrôles du contrat : l'API de `register` et de `call` ne doit pas bouger.

```rust
// forme visée (esquisse, pas le code final)
trait Command { type Params: DeserializeOwned + JsonSchema; type Output: Serialize + JsonSchema;
    fn spec() -> CommandSpec; fn run(&self, params: Self::Params) -> Self::Output; }
```

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement
- `cargo run -q -p cassini -- run system.about` -- expected: `{"core_version":"0.1.0","protocol_version":1}`, la forme exacte de la matrice

**Manual checks (if no CLI):**
- `Cargo.lock` : le premier `cargo build` le met à jour avec les nouvelles dépendances ; il doit être commité avant que `--locked` passe.
