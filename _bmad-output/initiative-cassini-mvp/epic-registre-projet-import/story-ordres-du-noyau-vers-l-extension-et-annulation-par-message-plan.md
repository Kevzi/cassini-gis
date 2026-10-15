---
title: "Ordres du noyau vers l'extension et annulation par message"
type: 'feature'
ticket: '11'
created: '2026-10-15'
status: 'built'
baseline_revision: '5cc23616ed2514b7cbc0e5a21c7aee5f2e2cc7b5'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le canal `cassini serve` ne sait que répondre (AD-3 le veut symétrique) : le noyau ne peut pas demander à l'extension d'exécuter une commande `qgis`, ni l'utilisateur arrêter un travail en cours sans tuer le processus (AD-12). La boucle lit et répond sur un seul fil : un ordre en vol l'empêcherait de lire l'annulation (AD-4).

**Approach:** un fil lit l'entrée et transmet les messages ; la boucle principale reste seule à écrire. Une commande `qgis` annoncée part en ordre `order.execute` vers l'extension, dont la réponse revient sur le même flux ; `request.cancel` annule une requête en vol par son `id`, le noyau journalise l'étape comme annulée et supprime ses sorties partielles.

## Boundaries & Constraints

**Always:**
- Ordre du noyau : requête `order.execute`, `params` `{command, params}` (paramètres déjà résolus : chemins absolus décidés par le noyau, AD-10), son propre `id` (chaîne `core-<n>`). L'extension répond `result` (la valeur de la commande) ou `error` portant un `ErrorReport` dans `data`.
- `request.cancel`, `params` `{id}`, existe dès la version 1 (`PROTOCOL_VERSION` reste 1) et vaut dans les deux sens. Reçu comme requête, il répond `{cancelled: true}` ; reçu comme notification, il ne répond pas.
- Annuler une requête en vol, `command.run` d'une commande déclarée `cancellable`, envoie à l'extension `request.cancel` (notification) sur l'ordre en cours, puis attend sa réponse au plus `CANCEL_GRACE` (5 s). La requête annulée répond l'erreur `registry.cancelled`, message en français.
- Étape annulée : une ligne de journal `{step, id, params, inputs, cancelled: true, versions}` sans `result` ni `error`, `next_step` avance ; les fichiers de sortie de l'étape (`output_params`) sont supprimés du dossier de travail.
- Rejeu : une étape `cancelled` est sautée, sans exécution ni comparaison.
- Une seule étape à la fois (AD-5) : un `command.run` ou `session.hello` pendant un ordre en vol répond `session.busy`, immédiatement. Toute autre requête reçoit sa réponse normale, sans attendre l'ordre.
- Entrée fermée pendant un ordre : l'étape est annulée comme ci-dessus, puis le serveur se termine proprement.
- Une réponse à un `id` inconnu ou déjà soldé est ignorée. Aucun chemin absolu dans les messages (AD-9).

**Never:**
- Aucune commande `core` annulable : l'annulation d'une commande Rust en processus n'est pas livrée ; `request.cancel` sur une commande non annulable répond `request.not_cancellable`, sur un `id` inconnu `request.unknown`, sans rien changer.
- Pas d'exécutant QGIS Python (épic 9), pas d'agent, pas de MCP, pas de socket, pas de nouvelle dépendance, pas de changement de `PROTOCOL_VERSION`.
- Aucun côté ne tue l'autre ; aucune lecture bloquante qui empêche d'écouter l'annulation.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|--------------|---------------------|------------------|
| Ordre | `command.run test.qgis_probe` annoncée | le noyau écrit `order.execute`, le client répond, `command.run` rend ce résultat | — |
| Ordre en erreur | le client répond une erreur | `command.run` rend cette erreur, étape journalisée en erreur | — |
| Annulation | `request.cancel` sur le `command.run` en vol | notification d'annulation à l'extension, `command.run` répond `registry.cancelled`, journal `cancelled`, sorties supprimées | — |
| Annulation sans réponse | l'extension ne répond jamais | au bout de `CANCEL_GRACE`, même issue | — |
| Non annulable | cancel sur une commande `cancellable: false` | rien ne change | `request.not_cancellable` |
| Id inconnu | cancel sur un `id` soldé | rien ne change | `request.unknown` |
| Occupé | `command.run` pendant un ordre | refus immédiat | `session.busy` |
| Entrée fermée | fin du flux pendant l'ordre | étape annulée, serveur terminé, verrou libéré | — |

</frozen-after-approval>

## Code Map

- `core/src/serve.rs` -- boucle unique en écriture ; `thread::scope` + `mpsc` pour la lecture. Ajouter l'exécutant `Channel` (ordre + attente en pompant le canal), `request.cancel`, `session.busy`. Réutiliser `handle_line`, `error_response`, `RpcError`.
- `core/src/exec.rs` -- `Executor` inchangé ; le routeur reçoit l'exécutant du canal par `Session::with_qgis`.
- `core/src/registry.rs` -- `RegistryError::Cancelled` (code `registry.cancelled`) ; `cancellable` déjà dans `CommandSpec`.
- `core/src/journal.rs` -- `Session::call` : sur `Cancelled`, ligne `cancelled: true`, suppression des sorties de l'étape (noms par `output_file`) ; `Replay::run` saute ces lignes. Ne pas toucher au verrou ni au format des autres lignes.
- `core/src/error.rs` -- aucun changement (`From<RegistryError>` couvre `Cancelled`).
- `cli/src/main.rs` -- `serve` : le flux d'entrée doit être `Send` (`&mut (dyn BufRead + Send)`) ; usage inchangé.
- `docs/` -- ajouter `canal-local.md` : les trois méthodes (`order.execute`, `request.cancel`, erreurs `session.busy`, `registry.cancelled`) — le contrat que l'épic 9 consomme.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/registry.rs` -- variante `Cancelled(String)`, code et message -- cause stable de l'annulation
- [ ] `core/src/journal.rs` -- étape annulée : ligne, nettoyage des sorties, saut au rejeu -- AD-12
- [ ] `core/src/serve.rs` -- lecteur en fil, exécutant du canal, `order.execute`, `request.cancel`, `session.busy`, fin sur entrée fermée -- AD-3, AD-4, AD-12
- [ ] `cli/src/main.rs` -- types `Send` pour l'entrée -- le fil de lecture
- [ ] `core/src/serve.rs` (tests) -- un client scripté (canaux `mpsc`, aucun processus) pour chaque ligne de la matrice ; l'ordre vers une commande dont le client répond hors d'ordre
- [ ] `docs/canal-local.md` -- le contrat des messages avec un exemple par méthode

**Acceptance Criteria:**
- Given un client de test qui répond à un ordre, when il envoie `command.run`, then il reçoit l'ordre, y répond, et le résultat est celui de sa réponse.
- Given un ordre en vol, when le client annule l'`id` de sa requête, then la requête répond `registry.cancelled`, le journal porte l'étape `cancelled` et le dossier de travail n'a aucun fichier de sortie de cette étape.
- Given un ordre en vol, when le client ne répond ni ne lit plus rien, then le serveur ne bloque pas et se termine à la fermeture de l'entrée, verrou libéré.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 0 medium, 6 low, 2 false.

| Constat | Verdict | Route | Evidence |
|---|---|---|---|
| `serve` attend la fin de lecture de l'entrée si la sortie casse alors que l'entrée reste ouverte (fil joint en fin de portée) | low | rejeté | Réel mais l'extension ferme les deux tubes en sortant ; le corriger demande un fil détaché `'static`, donc de la complexité. |
| Fin d'entrée pendant un ordre avec sortie aussi cassée : `serve` rend une erreur | false | rejeté | Une sortie cassée est précisément le cas où l'erreur est attendue ; sortie saine, la fin d'entrée est propre (test dédié). |
| Réponse avec `error` non objet rejetée en requête invalide | false | rejeté | Un message mal formé reçoit une erreur JSON-RPC ; l'ignorance du plan vise les réponses bien formées à un id soldé. |
| `delegated_error` invente `order.failed` et garde le code de l'extension tel quel | low | rejeté | Réel, sans conséquence démontrée ; l'extension est un pair de confiance, la correction ajoute une règle de validation. |
| `RegistryError::Delegated` et `code() -> &str` hors Code Map ; `plan.rs` a un `_` fourre-tout | low | rejeté | Écart nécessaire pour rendre le code de l'extension ; `validate_params` ne rend que deux causes. |
| `request.cancel` hors ordre répond `request.unknown`, docs muettes sur le sens extension vers noyau | low | rejeté | Seul `command.run` est annulable et il n'est en vol que pendant un ordre : comportement conforme au plan. |
| `ReplayReport.steps` compte les étapes rejouées, pas les lignes | low | rejeté | Voulu : une étape sautée n'est pas rejouée ; aucun appelant ni test de 2.10 ne casse. |
| `answer_during_order` ne passe pas par `dispatch` | low | rejeté | Équivalent aujourd'hui (trois méthodes) ; aucun défaut présent. |

## Design Notes

Choix faits sans interrogation (épic délégué) : l'annulation attend la réponse de l'extension au plus 5 s avant de nettoyer, pour ne pas supprimer un fichier qu'elle écrit encore ; `request.cancel` porte le même nom dans les deux sens pour garder la symétrie d'AD-3 ; une étape annulée n'écrit ni `result` ni `error`, pour que le rejeu la saute sans ambiguïté.

```json
{"jsonrpc":"2.0","id":"core-1","method":"order.execute","params":{"command":"test.qgis_probe","params":{"layer":"l"}}}
{"jsonrpc":"2.0","method":"request.cancel","params":{"id":"core-1"}}
```

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement
