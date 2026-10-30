---
title: "Relais : cassini mcp pilote la session QGIS quand elle l'autorise"
type: 'feature'
ticket: '25'
created: '2026-10-30'
status: 'built'
baseline_revision: 'ee9da302a08d0d6937065b91a9cfb35920c1cb45'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
  - '{project-root}/docs/canal-local.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Depuis 5.24 et 5.28, la session QGIS ouvre le canal d'IA externe et exécute les `command.run` d'un relais, mais `cassini mcp` ne s'y branche jamais : il sert seul le registre du noyau, sans QGIS, tient le verrou du projet toute sa vie, et bloque son runtime pendant un appel (ni `ping` ni annulation, report de 5.7).

**Approach:** À chaque `tools/list` et `tools/call`, `cassini mcp` choisit son mode (AD-13) : **relais** si le canal d'IA externe de l'utilisateur répond à `external.attach` (catalogue et schémas pris dans la session, commandes QGIS comprises, appels transmis par `command.run`), **autonome** sinon (5.7, verrou pris le temps d'un appel). Il annonce tout changement de mode ou de catalogue par `notifications/tools/list_changed` et sert ses appels sans bloquer son runtime.

## Boundaries & Constraints

**Always:**
- Relais : ne décide rien, ne lit jamais son propre registre pour le catalogue ; outil = entrée `available` du catalogue d'`external.attach` (nom `id`, description fr puis en, schémas de la session). N'emploie que `external.attach`, `command.run`, `request.cancel` (AD-6) ; aucun chemin n'y part (AD-9) : `--project` est ignoré en relais.
- Un appel commencé en relais ne finit jamais en autonome : session perdue en plein appel → résultat `isError` `external.session_lost`.
- Traduction (AD-29) : `notifications/cancelled` → `request.cancel` de l'`id` du `command.run` ; erreur AD-21 → résultat `isError` (texte = `ErrorReport` JSON, comme 5.7) ; outil hors catalogue du mode courant → erreur de protocole.
- Autonome : `Session::open` puis fermeture à chaque appel (`--project`) ; projet introuvable toujours refusé au démarrage, code 1.
- `get_info` annonce `tools.listChanged`. Le runtime n'attend jamais un appel de façon bloquante : `ping` et annulation sont servis pendant une commande longue, dans les deux modes.
- Stdio côté client, canal d'IA externe côté session, rien d'autre (AD-14) ; `cargo deny check`, `hors_ligne.rs`, tests de `serve`, `canal_externe` et `parity.rs` restent verts.

**Never:**
- Aucune dépendance nouvelle ; seule la feature `sync` de tokio est ajoutée (même crate, pas de nouvelle crate). Ni `macros`, ni `net`.
- Rien dans `core` sauf correctif strictement nécessaire ; rien dans l'extension ni le panneau (5.26) ; pas de `external.approve`.
- Pas de sondage périodique du canal : le mode se décide aux requêtes, et à la réception de `external.session_lost`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Autonome | aucun canal | `tools/list` = commandes `core` (5.7) | — |
| Bascule en relais | canal allumé, `tools/list` | catalogue de la session, commandes QGIS annoncées comprises ; `list_changed` émis | — |
| Appel QGIS | `tools/call layer.describe` | `order.execute` dicté à l'extension, résultat structuré, journal de la session `"origin":"external"` | erreur de l'extension → `isError` |
| Occupé | session qui compose un plan | `isError` `session.busy` | — |
| Annulation | `notifications/cancelled` sur un appel annulable en cours | `request.cancel` envoyé ; étape journalisée annulée | — |
| Ping | `ping` pendant un appel en cours | réponse immédiate | — |
| Interrupteur coupé | pendant un appel | `isError` `external.session_lost`, jamais réexécuté en autonome ; `list_changed` ; requête suivante en autonome | — |
| Autonome, verrou | `--project P`, après un appel | `cassini run --project P` écrit le journal pendant que `mcp` vit | — |
| Versions différentes | `external.attach` → `session.protocol_mismatch` | `tools/list` = catalogue autonome ; chaque `tools/call` rend `isError` `session.protocol_mismatch` avec le message qui nomme le binaire à mettre à jour ; rien n'est exécuté | — |

## Decisions (proposées le 2026-10-30, à confirmer à l'approbation)

- Versions de protocole différentes : refus visible (cas de la matrice) plutôt que repli silencieux en autonome, pour que l'IA dise à l'utilisateur quoi mettre à jour.
- Plan gardé entier (~2 300 jetons), comme 5.24 et 5.28 : le runtime non bloquant est nécessaire à l'annulation en relais.

</frozen-after-approval>

## Code Map

- `cli/src/mcp.rs` -- serveur 5.7. `CassiniMcp` gagne le mode ; `call_tool`/`list_tools` deviennent de vrais `async` (attente d'un `tokio::sync::oneshot`, sélection manuelle avec `ctx.ct.cancelled()` par `std::future::poll_fn`, sans `macros`). `worker` ouvre `Session` à chaque `Job`. `catalogue(registry)` reste pour l'autonome ; ajouter `catalogue_from_session(&[Value])` (même forme de `Tool`). `get_info` : `enable_tool_list_changed()`. Garder `parse`, `io_failure`, la sortie 0/1.
- `cli/src/mcp/relay.rs` (nouveau) -- connexion : `cassini_core::external::{connect, ChannelName::for_user}` (`core/src/external/mod.rs:108`, déjà prévu pour 5.25), `external.attach {protocol_version: PROTOCOL_VERSION}` (`core/src/registry.rs:16`), fil lecteur qui route les réponses par `id` vers des `oneshot` en attente et, sur `external.session_lost` ou fin de flux, solde toutes les attentes par `external.session_lost` et marque la connexion morte ; écriture sous `Mutex` sur un `try_clone`. `shutdown()` à la sortie.
- `rmcp-3.8.0` -- chaque requête est une tâche (`service.rs:1586`) : une attente non bloquante libère la boucle. `RequestContext { ct, peer }` ; `Peer::notify_tool_list_changed()` (`service/server.rs:253`), appelé aussi depuis le fil lecteur par un `tokio::runtime::Handle` gardé.
- `core/src/serve/external.rs:357` -- `attach` : rend `commands` (entrées `CommandSpec` sérialisées + `available`, `serve.rs:855`). Ne pas modifier.
- `docs/canal-local.md:246` -- section du canal d'IA externe (forme d'`external.attach`, `command.run`, `request.cancel`, `session_lost`).
- `cli/tests/mcp.rs`, `cli/tests/support/mcp_client.rs` -- client de test (une requête à la fois) : lui ajouter l'envoi sans attente, la lecture des notifications et `ping`. Modèle de session simulée : `cli/tests/end_to_end.rs:443` (vrai `cassini serve` avec `CASSINI_EXTERNAL_TAG`, test jouant l'extension sur stdin/stdout, `session.hello` annonçant `layer.describe` et `analysis.buffer`).
- `cli/tests/parity.rs` -- parité autonome, ne doit pas bouger.

## Tasks & Acceptance

**Execution:**
- [x] `cli/Cargo.toml` -- feature `sync` de tokio, commentaire de licence à jour.
- [x] `cli/src/mcp/relay.rs` -- connexion, poignée de main, appel, annulation, perte.
- [x] `cli/src/mcp.rs` -- choix du mode par requête, `list_changed`, appels asynchrones, verrou par appel, tests unitaires (catalogue tiré d'un `attach`, réponse d'erreur → `isError`).
- [x] `cli/tests/support/mcp_client.rs`, `cli/tests/mcp.rs` -- chaque ligne de la matrice avec un vrai `cassini serve` derrière le canal, nom isolé par `CASSINI_EXTERNAL_TAG`.
- [x] `docs/canal-local.md`, `README.md` -- le relais vu de `cassini mcp` : deux modes, `list_changed`, `--project` ignoré en relais.

**Acceptance Criteria:**
- Given une session QGIS simulée qui allume le canal, when un client MCP liste puis appelle `layer.describe`, then il reçoit le résultat de l'extension et la session a journalisé l'étape avec l'origine `external`.
- Given le canal éteint en plein usage, when le client rappelle un outil du noyau, then il est servi en autonome après avoir reçu `list_changed`.
- Given la recette sous Linux et Windows, when `cargo test -p cassini --locked`, `cargo test -p cassini-core --locked` et `cargo deny check` tournent, then ils passent.

## Implementation Notes

- Mode choisi par `CassiniMcp::decide` sous un `tokio::sync::Mutex` : connexion vivante gardée, sinon `Relay::attach` dans `spawn_blocking` (connexion et poignée de main bloquantes, poignée de main bornée à 5 s). Le catalogue servi est comparé (sérialisé) au dernier servi ; `list_changed` part s'il diffère. Le fil lecteur du relais, à la perte, ne fait que lancer une tâche sur le `Handle` gardé : elle oublie le relais s'il est encore le courant, pose le catalogue autonome comme dernier annoncé et envoie `list_changed` (un seul envoi, quel que soit l'ordre avec la requête suivante).
- `notifications/cancelled` devient une **notification** `request.cancel {id}` (la réponse du `command.run` dit l'issue, et rmcp abandonne de toute façon la réponse d'une requête annulée).
- Erreur sans `ErrorReport` dans `data` (erreur de protocole de la session, ex. `-32602`) : rendue comme erreur de protocole MCP de même code.
- Versions différentes : le catalogue reste l'autonome et tout `tools/call`, quel que soit l'outil, rend `isError` `session.protocol_mismatch` (le refus passe avant le contrôle du catalogue).
- Tests d'intégration : « Occupé » est provoqué par un `command.run` de l'extension en vol (ordre `analysis.buffer` non répondu) plutôt que par une composition de plan, qui exigerait un vrai modèle ; l'annulation vise `analysis.buffer` (`layer.describe` n'est pas annulable, les commandes `test.*` ne sont pas dans le binaire) ; « Versions différentes » ne peut pas venir d'un vrai `cassini serve` (même protocole) : le test tient lui-même le canal et répond `session.protocol_mismatch`. Le client de test pose toujours un `CASSINI_EXTERNAL_TAG` inutilisé, pour qu'une vraie session QGIS du poste ne bascule pas les tests autonomes en relais.
- Recette sous Linux (WSL) : la construction de `llama-cpp-sys-2` exige libclang (`LIBCLANG_PATH`) et les en-têtes de gcc (`BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`) ; `cargo clippy --workspace` y échoue sur `app` (libdbus absent), indépendamment du ticket : clippy de `cassini`, `cassini-core`, `cassini-ai` sous Linux, de tout l'espace sous Windows.

## Plan Change Log

## Review Triage Log

Relecture `thorough` (blind-hunter, edge-case-hunter, verification-gap, intent-alignment), 2026-10-30. Comptes : high 0, medium 1, low 13, false 6, maybe-false 0.

| # | Lentille | Constat | Verdict | Preuve | Suite |
|---|---|---|---|---|---|
| 1 | BH, ECH, IA | Catalogue du relais figé tant que la connexion vit : un changement de catalogue n'est pas annoncé | false | le catalogue n'est posé qu'à `session.hello` (`serve.rs:843`), une seule fois par session (`session.already_started`), et le canal exige une session : il ne change qu'avec la connexion | rejeté |
| 2 | BH, ECH | Mode `Mismatch` : un outil hors catalogue autonome rend `-32602` « aucune session QGIS… » au lieu de `session.protocol_mismatch` | medium | contredit la matrice figée (« chaque `tools/call` ») et masque le binaire à mettre à jour | patch |
| 3 | VG | Refus `journal.read_only` d'un appel autonome (verrou tenu ailleurs) non testé | low | aucun test de `mcp` ne cherche `read_only` ; une régression vers un appel non journalisé passerait | patch (test) |
| 4 | VG | Session muette à `external.attach` : borne de 5 s non testée | low | seul `recv_timeout` protège `decide` ; aucun test d'un canal muet | patch (test) |
| 5 | BH, ECH, VG | Contrôles « jamais réexécuté » et `take_notifications == 0` ne voient que les messages déjà lus | low | `session.unread` / `unread` ne contiennent que ce qu'un `wait_for` a tiré ; commentaire mal placé | patch |
| 6 | BH | Version de protocole écrite en dur (`2`) dans les tests du relais | low | une montée de version casserait tous les tests par un `session.hello` refusé | patch |
| 7 | BH, VG | Docs : « annulation servie dans les deux modes » alors qu'en autonome la commande va au bout | low | `call_autonomous` n'abandonne que l'attente | patch (doc) |
| 8 | BH | README : « aucun chemin ne part vers l'IA externe » dépasse AD-9 | low | la session rend des chemins relatifs ; seul l'absolu est exclu | patch (doc) |
| 9 | BH | Docs muettes : appels concurrents en relais → `session.busy` ; erreur sans `ErrorReport` → erreur de protocole | low | règle écrite seulement dans les notes du plan | patch (doc) |
| 10 | VG, IA | `ping` et annulation pendant un appel autonome non testés | low | aucune commande `core` du binaire n'est assez lente pour tenir un appel en vol | defer (avec les commandes de terrain) |
| 11 | BH, ECH | Écriture bloquante vers la session sur le fil du runtime | low | la session lit chaque connexion dans un fil dédié qui ne s'arrête pas (5.24, 5.28) ; tampon plein improbable ; correctif = fil écrivain | rejeté |
| 12 | BH, ECH | Chaque requête sans relais retente branchement et poignée de main (5 s au plus, verrou de `Modes` tenu) | low | une session qui accepte sans répondre n'arrive que figée ; correctif = cache d'échec | rejeté |
| 13 | ECH | `connect` sans échéance (file pleine, tube occupé) | low | instance de tube recréée après chaque `accept` ; rare | rejeté |
| 14 | BH, ECH | Relais perdu entre poignée de main et `on_lost` servi comme vivant | low | fenêtre de quelques microsecondes ; l'appel rend `external.session_lost` sans rien exécuter ; correctif = garde nouvelle | rejeté |
| 15 | BH, ECH | `try_lock` à la sortie peut sauter `relay.shutdown()` ; `Drop` joint le fil lecteur depuis une tâche | low | `Drop` ferme la connexion d'abord, le lecteur rend aussitôt la main | rejeté |
| 16 | ECH | Annulation pendant `decide` : l'appel part quand même | low | il est aussitôt annulé (`ct` déjà annulé) ; non annulable, il va au bout comme tout appel | rejeté |
| 17 | ECH | Annulation d'un appel autonome en file : il s'exécute quand même | low | commandes `core` instantanées ; correctif = jeton dans `Job` | rejeté |
| 18 | ECH | Erreur de session avec `data.code` sans `message` rendue en erreur de protocole | low | la session envoie toujours un `ErrorReport` complet | rejeté |
| 19 | ECH | Fin du flux du canal (QGIS tué) en plein appel non testée de bout en bout | low | même branche que `session_lost` dans `read_loop`, couverte par le test unitaire du relais | rejeté |
| 20 | BH | `--project` ignoré en relais sans avertissement | false | décision figée du plan (AD-9 : aucun chemin vers la session) | rejeté |
| 21 | BH | `call_relay` attend sans fin après une annulation | false | la session répond toujours : `registry.cancelled` après au plus `CANCEL_GRACE`, sinon la fin de commande, sinon `session_lost` | rejeté |
| 22 | IA | L'allumage du canal n'est découvert qu'à la requête suivante, pas poussé | false | « pas de sondage » est une limite figée ; AD-13 décide le mode à chaque requête | rejeté |
| 23 | BH | `structuredContent` non objet non contrôlé ; entrées de catalogue mal formées | low | toutes les sorties du registre sont des objets (schemars) | rejeté |
| 24 | BH | Plan inachevé (journal de tri vide, décisions « à confirmer ») | false | journal rempli à cette passe ; décisions approuvées avec le plan le 2026-10-30 | rejeté |
| 25 | VG | Test « projet introuvable » lancé sans `CASSINI_EXTERNAL_TAG` | low | refus avant tout échange, sans effet | rejeté |
| 26 | IA | « Occupé » provoqué par un `command.run` de l'extension, pas par une composition | false | même refus `session.busy` côté relais ; la composition est couverte par `canal_externe` (5.28), le binaire n'a pas de compositeur factice | rejeté |

## Design Notes

Choix du mode, à chaque requête : connexion vivante → relais ; sinon tentative `connect` + `attach` (échec rapide sans canal) → relais ou autonome. Le catalogue servi est comparé au dernier annoncé ; s'il diffère, `list_changed` part. Le fil lecteur ne fait qu'aiguiller : il ne décide aucun mode.

```text
tools/call (relais) → id = n ; command.run {command, params} → attente(oneshot n | ct.cancelled)
  ct.cancelled → request.cancel {id:n}, puis attente de la réponse n
  réponse result → structured ; error.data ErrorReport → isError ; perte → isError external.session_lost
```

## Verification

**Commands:**
- `cargo test -p cassini --locked` puis sous Windows (PowerShell, `CARGO_TARGET_DIR=C:\ct`) -- expected: tout vert, `mcp.rs`, `parity.rs`, `end_to_end.rs`, `hors_ligne.rs` compris
- `cargo test -p cassini-core --locked` -- expected: vert
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo deny check` -- expected: ok
