---
title: 'Serveur MCP : cassini mcp expose le registre a un client tiers'
type: 'feature'
ticket: '7'
created: '2026-10-30'
status: 'built'
baseline_revision: 'dbbf886bfa6b208a58a0976d0806bcd496a71b15'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** CAP-8 n'est pas livré : aucune IA externe ne peut piloter Cassini. Le binaire n'a que `serve`, `run`, `replay` (AD-13 en prévoit un quatrième, `mcp`).

**Approach:** Ajouter la sous-commande `cassini mcp [--project <fichier>]`, un serveur MCP sur l'entrée et la sortie standard (AD-3), qui expose les commandes du registre comme outils MCP avec leurs schémas engendrés, en exécute une à la demande du client par le même chemin que `cassini run`, et ne donne jamais d'ordre au client.

## Boundaries & Constraints

**Always:**
- Un seul registre (AD-13) : chaque outil est une commande de `Registry::standard()`, nom = `id`, description = `description_fr` puis `description_en`, `inputSchema` = `params_schema`, `outputSchema` = `output_schema`. Aucun outil propre au serveur MCP, aucune méthode du protocole Cassini (AD-6, `session.*`, `plan.*`…) exposée.
- Exécution par le même chemin que `run` : sans `--project`, `Router::new(&registry).call` ; avec `--project`, `Session::open` puis `session.call` (journalisé, AD-9/AD-10). Même validation des paramètres par le noyau.
- Le serveur répond seulement (AD-3) : il n'émet aucune requête vers le client (ni sampling, ni elicitation, ni roots).
- Stdio seul : aucune socket, aucun port, aucune feature réseau (AD-14). `cargo deny check` reste vert.
- `cassini serve`, `run`, `replay` inchangés.

**Never:**
- Ni transport HTTP/SSE, ni authentification, ni feature `transport-io` de rmcp (elle tire `tokio/net`).
- Aucune composition de plan ni appel au modèle local dans `mcp` (AD-23, hors règle : l'IA externe appelle les commandes une à une).
- Ne pas toucher `core` au-delà de ce qui est strictement nécessaire (idéalement rien).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Découverte | `tools/list` | une entrée par commande exposée, schémas du registre | — |
| Appel nominal | `tools/call system.about {}` | `structuredContent` = sortie de `cassini run system.about`, plus le même JSON en texte | — |
| Paramètres invalides | `tools/call system.about {"x":1}` | résultat `isError: true`, texte = `ErrorReport` JSON (`registry.invalid_params`) | pas de plantage |
| Outil inconnu | `tools/call foo.bar` | erreur JSON-RPC de paramètres invalides | serveur reste vivant |
| Commande `runs_in: qgis` | absente de `tools/list` ; `tools/call` | erreur JSON-RPC comme un outil inconnu | serveur reste vivant |
| Projet absent | `cassini mcp --project manque.qgz` | message d'erreur sur stderr, code 1, avant tout échange | — |
| Fin de flux | le client ferme stdin | le processus sort avec code 0 | — |

## Decisions (propriétaire, 2026-10-30)

- **Dépendances :** `rmcp = "=3.8.0"` (Apache-2.0), `default-features = false, features = ["server"]`, et `tokio` (MIT) avec `rt` et `io-std` seulement. `deny.toml` passe l'interdiction de `tokio` en `wrappers = ["rmcp", "cassini"]` et interdit la feature `net` de tokio (`[[bans.features]]`). Le tableau de pile de l'architecture passe de rmcp 3.5.1 à 3.8.0.
- **Commandes déléguées à QGIS :** non exposées. Le catalogue MCP ne liste que les commandes `runs_in: core` ; un appel à une commande `qgis` est traité comme un outil inconnu.
- **Taille du plan :** gardée entière.

</frozen-after-approval>

## Code Map

- `cli/src/main.rs:60-115` -- `run()` aiguille les sous-commandes ; ajouter `mcp` ici, réutiliser la logique `--project` de `parse_run` et l'exécution `Router` / `Session` des lignes 83-95. `USAGE` (l.14) à compléter.
- `cli/src/main.rs:117` -- `serve()` : modèle d'une boucle stdio ; ne pas modifier.
- `cli/src/mcp.rs` (nouveau) -- le serveur : implémente `rmcp::ServerHandler` à la main (`get_info` avec la capacité `tools` seule, `list_tools`, `call_tool`), sans les macros (catalogue dynamique). Transport : le couple `(tokio::io::stdin(), tokio::io::stdout())` passé à `serve_server` (feature `transport-async-rw`). Runtime `tokio` `current_thread`.
- `core/src/registry.rs:333` -- `Registry::specs()` et `spec(id)` : source du catalogue ; `CommandSpec` (l.30) porte `params_schema`, `output_schema`, `runs_in`.
- `core/src/exec.rs` -- `Router::call` ; `core/src/journal/mod.rs:128,144,210` -- `Session` (verrou sur le projet, `call` renvoie `ErrorReport`). `Session` contient un `Box<dyn Executor>` non `Send` : la tenir dans un fil dédié qui reçoit les appels par canal `std::sync::mpsc` et répond par canal, le `ServerHandler` (qui doit être `Send + Sync`) ne tenant que l'émetteur.
- `cli/tests/parity.rs` -- modèle de test qui lance le vrai binaire et compare `run`/`serve` ; à étendre à `mcp`.
- `cli/tests/hors_ligne.rs` -- garde PE sans DLL réseau (Windows) ; ne pas modifier, doit rester vert.
- `deny.toml` -- garde AD-14.

## Tasks & Acceptance

**Execution:**
- [x] `cli/Cargo.toml`, `deny.toml`, `Cargo.lock` -- ajouter les dépendances décidées ci-dessus, avec la ligne de licence en commentaire comme dans `core/Cargo.toml`.
- [x] `cli/src/mcp.rs` -- serveur MCP (catalogue, appel, erreurs selon la matrice) et fil d'exécution.
- [x] `cli/src/main.rs` -- sous-commande `mcp`, `USAGE`, test unitaire d'usage invalide.
- [x] `cli/tests/mcp.rs` -- lance le binaire : handshake d'un client (révision 2025-11-25 avec `initialize`), `tools/list`, `tools/call system.about` comparé à `cassini run system.about`, paramètres invalides, outil inconnu, appel avec `--project` qui écrit le journal, fin de flux → code 0 ; vérifie qu'aucun message sortant n'est une requête (pas de `method` avec `id`).
- [x] `cli/tests/parity.rs` -- ajouter `mcp` à la parité pour chaque commande `core` et ses exemples.
- [x] `_bmad-output/.../architecture-noyau-et-facade-qgis.md` -- tableau de pile (version de rmcp) et ligne Deferred « Révision de MCP servie par rmcp » marquée vérifiée.
- [x] `README.md` -- section courte : brancher un client MCP (`cassini mcp --project <projet.qgz>`).

**Acceptance Criteria:**
- Given un client MCP tiers configuré pour lancer `cassini mcp`, when il se connecte, then il liste les commandes et exécute `system.about` dont il reçoit le résultat.
- Given aucun client MCP, when l'extension QGIS lance `cassini serve`, then tous les tests existants de `serve` et la parité passent sans changement.
- Given la recette, when `cargo deny check` et le test hors ligne tournent, then ils passent.

## Implementation Notes

- `cli/src/mcp.rs` : `ServerHandler` écrit à la main (`get_info`, `list_tools`, `call_tool`), catalogue construit une fois depuis `Registry::standard()`. `call_tool` est servi sur place (`std::future::ready`) : il envoie l'appel au fil d'exécution par `std::sync::mpsc` et attend sa réponse ; le fil ne dépend pas du runtime, donc pas d'interblocage, et les appels passent un à la fois dans l'ordre. Le fil tient le registre et, avec `--project`, la `Session` (ouverte avant tout échange : un échec sort sur stderr, code 1). Après la fin du flux, `shutdown_background` du runtime (le fil bloquant de lecture de stdin n'est pas attendu), puis `join` du fil d'exécution, qui libère le verrou du projet.
- Le client ferme l'entrée avant `initialize` : `ServerInitializeError::ConnectionClosed`, traité comme une fin normale (code 0).
- `rmcp` 3.8.0 garde sa liste de révisions par défaut (2024-11-05 à 2026-07-28) : `initialize` jusqu'à 2025-11-25, métadonnées par requête pour 2026-07-28. Pas de `get_tool` : aucune validation de schéma par rmcp, la seule validation est celle du noyau.
- Client de test partagé : `cli/tests/support/mcp_client.rs`, inclus par `#[path]` dans `mcp.rs` et `parity.rs` ; il lit une réponse avant d'envoyer la requête suivante.
- Environnement : `cli` se construit sous Windows (PowerShell depuis WSL, `LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`) ; `cargo deny check` aussi.

## Plan Change Log

- 2026-10-30 — `deny.toml` : `tokio-util` ajouté aux `wrappers` de `tokio` (`["rmcp", "tokio-util", "cassini"]`). `tokio-util` est une dépendance obligatoire de rmcp (`transport-async-rw`, tirée par `server`) et parent direct de tokio : sans lui, `cargo deny check bans` refuse tokio. La feature `net` reste interdite (`[[bans.features]]`, vérifié en la posant à la main : `feature-banned`).
- 2026-10-30 — `Cargo.lock` : tokio passé de 1.53.2 à 1.54.1 (`cargo update -p tokio --precise 1.54.1`), pour l'avis RUSTSEC-2026-0386 (`AsyncFd`, Unix) que `cargo deny check advisories` levait dès que tokio entrait dans le graphe du noyau.

## Review Triage Log

### Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment)

Verdicts : high 0, medium 3, low 9, false 2, maybe-false 2.

| Verdict | Route | Constat | Preuve / action |
|---|---|---|---|
| medium | patch | Un `QuitReason::JoinError` (tâche du service paniquée) est rendu comme une sortie propre, code 0 | `waiting()` rend `Ok(QuitReason::JoinError)` (rmcp service.rs:1185) et `map(\|_\| ())` l'efface ; le mapper en erreur |
| medium | patch | Panique du fil d'exécution cachée : `internal_error` ensuite, puis sortie 0 ; `opened` fermé sans message → code 1 muet | `let _ = thread.join()` ignore `Err` ; rendre `io_failure` |
| medium | patch | `deny.toml` admet tokio sous `tokio-util` sans restreindre qui tire `tokio-util` : `core` ou `ai` pourraient tirer tokio | ajouter `{ crate = "tokio-util", wrappers = ["rmcp"] }` |
| low | patch | Le serveur tient le verrou du projet pour toute sa vie : QGIS sur le même projet passe en lecture seule (et inversement, chaque appel MCP rend `journal.read_only`) ; rien ne le dit | comportement identique à `run` ; documenter dans le README |
| low | patch | Chemin `--project` : code d'erreur des paramètres invalides non vérifié | ajouter l'assertion `registry.invalid_params` |
| low | patch | Aucun test n'assure que chaque schéma d'entrée et de sortie exposé est de type `object` (exigé par MCP) | assertion sur chaque outil de `tools/list` |
| low | patch | Ligne d'état du README muette sur la portée (commandes du noyau seulement) ; exemple de configuration supposant `cassini` dans le PATH | préciser |
| low | patch | Message « Outil inconnu » pour une commande QGIS : l'IA ne sait pas pourquoi | message : outil inconnu ou réservé à l'extension QGIS |
| maybe-false, medium si vrai | defer | Noms d'outils avec un point (`system.about`) possiblement refusés par certains clients | la spec MCP 2025-11-25 admet `.` ; tranché par la recette hitl avec le client du propriétaire |
| low | defer | Révisions 2024-11-05 et 2026-07-28 annoncées mais non testées | rmcp épinglé `=3.8.0` ; ne régresse qu'à une montée de version voulue |
| medium | defer | Appel bloquant : ni `ping` ni annulation pendant une commande longue | sans effet sur les deux commandes du noyau ; à traiter avec les commandes de terrain (étape 6) |
| low | reject | `run()` reçoit des flux injectés que `mcp` ignore | `mcp` sert toujours les flux du processus ; tests par le binaire |
| low | reject | Client de test sans délai, stderr jeté ; dossiers temporaires non nettoyés ; `run` sans `CASSINI_MODELS` | confort de test, motif déjà présent dans `parity.rs` |
| low | reject | Pas de test de rejeu d'un journal écrit par MCP | même `Session::call` que `run`, rejeu déjà prouvé sur ce chemin |
| false | reject | Lock : `syn 3.0.6` → `syn 2.0.119` sur une crate existante | résolution de cargo, lock vérifié par `--locked` ; aucune version de crate directe changée hors tokio |
| false | reject | Métadonnées du plan incohérentes (`ticket: '7'`, journal vide) | `ticket` = `id` de `find` par règle ; journal rempli à cette passe |
| maybe-false | reject | Premier message autre que `initialize` → sortie 1 sans réponse | rmcp gère la révision sans `initialize` ; si vrai, faible |

## Design Notes

Le serveur n'a pas d'état propre : il traduit un appel MCP en appel du registre et une réponse du registre en résultat MCP. Une erreur de commande est un résultat d'outil (`isError: true`), pas une erreur de protocole, pour que l'IA externe lise le message et corrige ses paramètres ; seule une demande hors catalogue est une erreur de protocole.

## Verification

**Commands:**
- `cargo test -p cassini --locked` -- expected: tout vert, `mcp.rs` et `parity.rs` compris
- `cargo clippy -p cassini --all-targets -- -D warnings` et `cargo fmt --all --check` -- expected: aucun avertissement
- `cargo deny check` -- expected: ok

**Manual checks (if no CLI):**
- Recette hitl : le propriétaire branche un client MCP de son choix sur `cassini mcp` et exécute `system.about`.
