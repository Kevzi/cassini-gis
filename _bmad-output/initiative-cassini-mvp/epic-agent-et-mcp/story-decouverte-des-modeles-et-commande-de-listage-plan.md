---
title: 'Découverte des modèles et commande de listage'
type: 'feature'
ticket: '2'
created: '2026-10-17'
status: 'built'
baseline_revision: '51703a174cc711859bdb4e289b25c6db066dea51'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/regle-de-redaction-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le noyau ne sait pas où sont les modèles GGUF, et l'extension, la CLI et l'agent ne doivent pas chacun l'apprendre (AD-15). Aujourd'hui une commande du registre ne peut pas échouer avec un code typé : `Command::run` rend toujours un résultat.

**Approach:** une commande du registre `model.list`, en Rust dans `core/` (simple parcours de dossier, sans `cassini-ai`), cherche le dossier de modèles et rend la liste des `.gguf`. Pour qu'elle rende une erreur typée, `Command::run` devient faillible, avec un code `domaine.cause` (AD-21).

## Boundaries & Constraints

**Always:** ordre de recherche fixe et unique : variable d'environnement `CASSINI_MODELS` (dossier), puis `models/` à côté de l'exécutable du noyau. Premier dossier **désigné** qui n'existe pas = erreur, jamais de repli silencieux sur le suivant. Dossier trouvé mais sans GGUF = succès, liste vide. Un modèle = un fichier régulier dont l'extension est `gguf` (sans tenir compte de la casse), trié par nom ; résultat `{folder, models: [{file, size_bytes}]}`, nom de fichier seul. Erreur `models.no_folder` (message français : où chercher, comment désigner un dossier) ; une erreur de lecture du dossier donne `models.unreadable`. `core/` ne dépend pas de `cassini-ai` ni d'aucune nouvelle crate. Commande `runs_in: core`, non annulable, avec exemple et test.

**Never:** aucun téléchargement, aucun accès réseau ; ne pas charger ni ouvrir un GGUF (la liste vient des noms et des tailles) ; aucune logique de modèle dans `qgis-plugin/` ; pas de fichier de configuration (hors périmètre, AD-15 le tolère par « réglage ») ; pas de parcours récursif.

</frozen-after-approval>

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Deux modèles | dossier avec `a.gguf`, `B.GGUF`, `notes.txt` | deux entrées triées, `size_bytes` exacts | — |
| Dossier vide | dossier existant sans GGUF | `models: []`, succès | — |
| Aucun dossier | pas de `CASSINI_MODELS`, pas de `models/` à côté | — | `models.no_folder` |
| Réglage cassé | `CASSINI_MODELS` vers un dossier absent | — | `models.no_folder`, sans repli |
| Sous-dossier `x.gguf/` | répertoire nommé comme un modèle | ignoré | — |
| Paramètre inconnu | `{"x": 1}` | — | `registry.invalid_params` |

## Code Map

- `core/src/registry.rs` -- `trait Command` l.~133 : `run` rend `Result<Output, CommandFailure>` ; nouvelle struct `CommandFailure {code, message}` ; `register()` l.~250 propage l'échec ; nouvelle variante `RegistryError::Failed {code, message}` (code() et Display l.~150-190, même rendu que `Delegated`).
- `core/src/commands/{system_about,test_echo,test_commands}.rs` -- adapter `run` en `Ok(..)` (4 impls) ; ne rien changer d'autre.
- `core/src/commands/model_list.rs` (nouveau, auto-déclaré par `build.rs`) -- `ModelList`, fonction pure `discover(setting: Option<&str>, exe_dir: Option<&Path>)` testable sans toucher l'environnement.
- `core/src/serve.rs` l.~445 et `core/src/journal/mod.rs` l.~243 -- lire comment `Delegated` y est converti ; `Failed` doit donner le même `ErrorReport` `{code, message}` et être journalisé comme une erreur ordinaire. `core/src/error.rs` : `From<&RegistryError>` suffit.
- `core/src/contract.rs` -- `check_contract` ne doit rien signaler pour `model.list` ; les compteurs de commandes des tests (`cli/tests/*`, `qgis-plugin/tests/*`) sont à relever si un chiffre en dur casse.
- Ne pas toucher : `ai/`, `qgis-plugin/` (le panneau affichera la liste plus tard, ticket 5.3+).

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/registry.rs` + 4 impls existantes -- `CommandFailure`, `RegistryError::Failed`, `run` faillible -- une commande peut échouer avec un code typé
- [ ] `core/src/commands/model_list.rs` -- commande `model.list`, `discover`, tests de la matrice -- découverte et listage
- [ ] `core/src/{serve,journal}` -- vérifier que l'erreur passe par le canal et le journal ; test dans `model_list.rs` via `Registry::standard().call` -- code `models.no_folder` rendu tel quel
- [ ] `docs/` -- une note : ordre de recherche, variable `CASSINI_MODELS`, dossier `models/` -- recette

**Acceptance Criteria:**
- Given `CASSINI_MODELS` vers un dossier avec deux GGUF, when `model.list` s'exécute par le registre, then les deux sont listés, triés, avec leur taille.
- Given aucun dossier de modèles, when `model.list` s'exécute, then l'erreur a le code `models.no_folder`, sans repli ni téléchargement.
- Given le dépôt, when `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` et `cargo deny check` tournent, then tout passe et `core/` ne dépend pas de `cassini-ai`.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| Finding | Verdict | Route | Evidence |
|---|---|---|---|
| lien `.gguf` cassé fait échouer toute la liste (`metadata` suivi d'un `?`) | medium | patch | corrigé : l'entrée est ignorée |
| `CASSINI_MODELS` non Unicode ou vide retombe sur `models/` | low | rejete | cas rare ; une variable vide vaut « non posée » par convention |
| `cargo fmt --check` échouerait | false | rejete | `cargo fmt --check` passe |
| pas de test de `models.unreadable` | low | rejete | difficile à provoquer, garde ajoutée seulement pour un test |
| `Failed` non vérifié dans le canal et le journal | false | rejete | `serve.rs:572` convertit toute `RegistryError` par `ErrorReport::from` ; le journal ne traite que `Cancelled` |
| `set_var` dans un test multi-fils | low | rejete | seul test à poser la variable ; la parité tourne en processus séparé |

## Design Notes

`discover` prend ses entrées en arguments : le test des cas de la matrice n'a besoin ni de l'environnement ni de l'exécutable. Un seul test de bout en bout pose `CASSINI_MODELS` (variable de processus, donc un seul test y touche). Dossier désigné absent = erreur franche, pour qu'un réglage faux ne soit jamais masqué par le dossier d'installation.

## Verification

**Commands:**
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo deny check` -- expected: propre
