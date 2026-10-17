---
title: 'Traceur : le noyau charge le modèle local et répond à une phrase, sans réseau'
type: 'feature'
ticket: '1'
created: '2026-10-17'
status: 'ready-for-dev'
baseline_revision: 'efa849f9046ac8430ef6a11607d9fc6948ead2f8'
route: 'full'
route_source: 'auto'
risk: 'high'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context:
  - '{project-root}/docs/regle-de-redaction-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** l'epic 5 n'a aucun code : pas de crate `ai/`, pas de `llama-cpp-2`, et la compilation de llama.cpp sous Windows est le risque principal de l'epic. Sans un chemin de bout en bout prouvé, tout le reste repose sur une hypothèse.

**Approach:** créer le crate `cassini-ai` (Apache-2.0) qui lie `llama-cpp-2` **dans le processus du noyau** (AD-15, jamais de serveur ni de HTTP), le brancher à `cli/` par une sous-commande `model-check`, charger le GGUF Qwen3.5-9B Q4_K_M et rendre le texte d'une phrase fixe. Poser la garde AD-14 (`cargo deny`) qui accepte cette dépendance sans ouvrir de porte au réseau.

## Boundaries & Constraints

**Always:** `core/` ne dépend jamais de `ai/` ; `cli/` dépend des deux. Dépendance `llama-cpp-2 = "=0.1.159"` (MIT OR Apache-2.0, llama.cpp MIT), `default-features = false` (CPU, sans openmp ni GPU), `Cargo.lock` commité. Décodage glouton (température 0), contexte 8192, réflexion de Qwen3.5 coupée (`<think>\n\n</think>\n\n` ajouté après le tour assistant, prompt de forme `tests/epreuve-modele-local/harness.py`). Le fichier GGUF reste hors git (`ai/models/`). Aucune connexion réseau, aucune fonction de téléchargement. `deny.toml` : licences MIT, Apache-2.0, BSD, ISC, Unicode-3.0, Unlicense ; `bans` sur les clients HTTP, piles async et TLS ; portée limitée au noyau (`app/src-tauri` exclu). Le binaire final n'importe ni `ws2_32` ni `winhttp` (cpp-httplib est compilé dans `llama-common` mais écarté au lien) : un test le vérifie.

**Never:** pas de `llama-server` ni de socket ; pas de GPL ni de LGPL ; pas de commande du registre ici (la découverte de modèles est le ticket 5.2) ; pas de prompt F6 complet ni de plan (ticket 5.3) ; pas de modification de `core/` ; pas de téléchargement de modèle par le code.

</frozen-after-approval>

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Réponse | `cassini model-check --model <gguf> --prompt "Dis bonjour."` | texte non vide du modèle sur la sortie standard, code 0 | — |
| Modèle absent | chemin inexistant | — | message clair, code non nul, aucun panic |
| Fichier invalide | fichier qui n'est pas un GGUF | — | erreur de chargement typée, code non nul |
| Prompt vide | `--prompt ""` | — | refusé avant le chargement |
| Arguments manquants | pas de `--model` | — | usage affiché, code non nul |
| Hors ligne | exécution sans réseau | même résultat | — |

## Code Map

- `Cargo.toml` -- workspace : ajouter `ai` aux `members` ; `ai/Cargo.toml` (nouveau, `cassini-ai`, héritage `.workspace = true`).
- `ai/src/lib.rs` (nouveau) -- `complete(model: &Path, prompt: &str) -> Result<String, AiError>` : init du backend (logs coupés), chargement, contexte 8192, tokenisation, décodage glouton borné, arrêt sur fin de génération.
- `cli/src/main.rs` -- `run()` l.58 et `USAGE` l.10-13 : troisième branche `model-check` avant `parse_run` ; tests en bas du même fichier. `cli/Cargo.toml` -- ajouter `cassini-ai`.
- `core/Cargo.toml` -- ne pas toucher.
- `deny.toml` (nouveau), `.github/workflows/recette.yml` -- étape `cargo deny` et couverture de `ai` (`-p cassini-ai`), installation de cmake et libclang sur ubuntu.
- `tests/epreuve-modele-local/harness.py` -- forme du prompt chatml et réflexion coupée à reprendre ; `docs/` -- note de build Windows (LIBCLANG_PATH, chemin court).
- Fait vérifié : build à froid de `llama-cpp-2` en release CPU ≈ 1 min 20 s avec `LIBCLANG_PATH=C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native`, CMake 4.2.2 et VS Build Tools 2022 ; un `target/` à chemin trop long fait échouer CMake (MAX_PATH), d'où `CARGO_TARGET_DIR` court si besoin.

## Tasks & Acceptance

**Execution:**
- [ ] `Cargo.toml`, `ai/Cargo.toml`, `ai/src/lib.rs` -- crate `cassini-ai` et `complete()` avec erreurs typées (`AiError`) -- cœur du traceur
- [ ] `cli/Cargo.toml`, `cli/src/main.rs` -- sous-commande `model-check`, usage, tests des lignes de la matrice qui ne chargent pas le modèle -- point d'entrée
- [ ] `deny.toml`, `.github/workflows/recette.yml` -- garde AD-14 : licences, bans réseau, `cargo deny check` limité au noyau ; installer `cargo-deny` sur le poste -- verrou hors ligne
- [ ] `ai/tests/` ou `cli` -- test d'intégration qui lance le vrai modèle quand `CASSINI_TEST_MODEL` est posé (ignoré sinon), et test que le binaire n'importe pas `ws2_32`/`winhttp` -- preuve hors réseau
- [ ] `docs/` -- recette Windows : variables, chemin court, taille de `target/`

**Acceptance Criteria:**
- Given le GGUF dans `ai/models/` et aucune connexion, when `cassini model-check --model ai/models/Qwen3.5-9B-Q4_K_M.gguf --prompt "Dis bonjour."` tourne, then une réponse non vide est affichée et le code de sortie est 0.
- Given le dépôt, when `cargo deny check` et `cargo test` tournent, then tout passe et `core/` ne dépend pas de `cassini-ai`.
- Given les lignes d'erreur de la matrice, when elles sont lancées, then chacune rend un message clair et un code non nul, sans panic.

## Implementation Notes

## Plan Change Log

## Review Triage Log

## Design Notes

La réponse du modèle n'est qu'un texte libre : aucun plan, aucun schéma ici. `complete()` est volontairement la plus petite surface qui prouve la chaîne (compilation, chargement, décodage) ; la forme du prompt F6, la contrainte de sortie et la validation viennent au 5.3 sans casser cette signature. Un profil `dev` lent à l'inférence est prévisible : la preuve se fait en `--release`.

## Verification

**Commands:**
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propre
- `cargo deny check` -- expected: licences et bans OK
- `cargo build --release -p cassini` puis `cassini model-check --model ai/models/Qwen3.5-9B-Q4_K_M.gguf --prompt "Dis bonjour."` -- expected: texte non vide, code 0

**Manual checks (if no CLI):**
- Débrancher le réseau et relancer `model-check` : même résultat.
