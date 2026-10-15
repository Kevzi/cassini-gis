---
title: 'Le canal local : cassini serve, session.hello et commandes disponibles'
type: 'feature'
ticket: '7'
created: '2026-10-15'
status: 'built'
baseline_revision: '7f55dd57f6dd595adc73fdbfa47eb6403cf83c0e'
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

**Problem:** le noyau n'a aucun canal vers l'extension QGIS (AD-2, AD-3) : ni l'extension ni l'agent ne peuvent lire le registre, ni appeler une commande par la même boucle et le même journal que `cassini run`.

**Approach:** `cassini serve` parle JSON-RPC 2.0 sur l'entrée et la sortie standard, un message par ligne. `session.hello` ouvre la session du projet et rend le catalogue (descriptions, schémas, exemples, disponibilité) ; `command.run` exécute une commande par `Session::call`.

## Boundaries & Constraints

**Always:**
- `session.hello` est la première requête ; `params` : `protocol_version` (entier), `commands` (ids annoncés), `project` (chemin absolu du fichier de projet, AD-7). Toute autre méthode avant elle : `session.not_started`. Un second hello : `session.already_started`.
- Version différente de `PROTOCOL_VERSION` : refus `session.protocol_mismatch`, message en français disant quoi mettre à jour (l'extension si elle est en retard, le paquet du noyau si elle est en avance). Aucune session ouverte.
- Le catalogue rend chaque commande : `id`, descriptions fr/en, `runs_in`, `cancellable`, schémas, `examples`, `available`. `available` = commande `core`, ou `qgis` annoncée. Une commande `qgis` non annoncée est refusée par `command.run` en `registry.unavailable`, avant toute exécution.
- `command.run` (`{command, params}`) passe par `Session::call` : même validation, même journal que `run`. Résultat = la valeur rendue par la commande.
- Erreurs : `error.data` porte l'`ErrorReport` (AD-21) ; erreurs de protocole en codes JSON-RPC standard (-32700, -32600, -32601, -32602), erreurs métier en -32000.
- Sortie standard : messages seuls. Pas de réponse à une notification. L'entrée fermée termine le serveur proprement ; une sortie qui n'accepte plus d'écriture le termine en échec.
- Aucun chemin absolu dans les messages d'erreur (AD-9).

**Never:**
- Pas d'ordre du noyau vers l'extension ni d'annulation (entrée 11), pas d'agent, pas de MCP, pas de socket ni de port, pas de nouvelle dépendance.
- Une commande `qgis` annoncée reste servie par le bouchon (`registry.unavailable`) : aucun exécutant QGIS réel ici.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|--------------|---------------------|------------------|
| Branchement | hello version 1, `commands: []`, projet absolu | catalogue ; `test.qgis_probe` `available:false`, `system.about` `true` | — |
| Appel | `command.run system.about` | résultat de la commande, une ligne de journal | — |
| Version différente | hello version 99 | refus nommant la mise à jour | `session.protocol_mismatch` |
| Appel avant hello | `command.run` en premier | refus | `session.not_started` |
| QGIS non annoncée | `command.run test.qgis_probe` | refus, rien n'est journalisé | `registry.unavailable` |
| QGIS annoncée | même appel, annoncée | le bouchon répond | `registry.unavailable` |
| Message illisible | ligne non JSON | réponse `id: null`, le serveur continue | -32700 |
| Chemin relatif | `project: "p.qgz"` | refus | -32602 |
| Entrée fermée | fin du flux | serveur terminé, verrou libéré | — |

</frozen-after-approval>

## Code Map

- `core/src/serve.rs` (nouveau) -- `serve(registry, input, output)` : boucle ligne à ligne, hello, catalogue, `command.run`. Déclarer dans `core/src/lib.rs`.
- `core/src/registry.rs` -- `CommandSpec` : champ `examples: Vec<Value>` (sérialisé) et `.example(Value)` ; `Registry::specs()` public (ordre de l'id). `RunsIn`, `PROTOCOL_VERSION` réutilisés.
- `core/src/journal.rs` -- `Session::open`, `Session::call` réutilisés tels quels. Ne pas toucher au format du journal.
- `core/src/error.rs` -- `ErrorReport` réutilisé ; codes `session.*` ajoutés dans `serve.rs`, sans changer `ErrorReport`.
- `core/src/commands/system_about.rs`, `test_commands.rs` -- déclarer un exemple (`{}`, `{"path": "a"}`).
- `cli/src/main.rs` -- sous-commande `serve` (aucun argument), `run` reçoit l'entrée ; usage mis à jour.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/registry.rs` -- `examples`, `.example()`, `specs()`, test : tout exemple déclaré valide ses paramètres
- [ ] `core/src/commands/` -- exemples sur `system.about` et `test.read_file`
- [ ] `core/src/serve.rs` -- boucle JSON-RPC, hello, catalogue, `command.run`, erreurs
- [ ] `cli/src/main.rs` -- sous-commande `serve`, entrée injectable
- [ ] tests dans `serve.rs` et `main.rs` -- une ligne de la matrice chacune ; le test de parcours : hello, `system.about`, ligne de journal présente

**Acceptance Criteria:**
- Given un projet de test et un client qui envoie hello puis `system.about`, when `cassini serve` tourne, then le catalogue porte les schémas, la réponse porte les versions, et le journal compte une ligne.
- Given un hello de version différente, when le noyau répond, then le message nomme ce qu'il faut mettre à jour et aucune commande ne s'exécute ensuite.
- Given une entrée fermée, when la boucle se termine, then le verrou du projet est libre pour une autre session.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 1 medium, 3 low, 1 false.

| Constat | Verdict | Route | Evidence |
|---|---|---|---|
| `hello` valide `commands` et `project` avant la version : un hello d'une version différente aux paramètres autres donne -32602 au lieu de `session.protocol_mismatch` | medium | patch | Réel (`serve.rs`, ordre des contrôles). Version lue et comparée en premier ; test ajouté. |
| Objet malformé sans `id` : aucune réponse | low | patch | Réel (`id_value?`). Réponse à identifiant nul (JSON-RPC 2.0), correction de quelques lignes ; test ajouté. |
| Hello sur un projet verrouillé : session en lecture seule non signalée | low | rejeté | `command.run` répond ensuite `journal.read_only`, qui nomme la cause ; signaler le mode ajoute une surface publique au résultat de hello, que le plan ne prévoit pas. |
| Ligne sans borne de taille | low | rejeté | Pair local de confiance (processus enfant de l'extension) ; borner demande une politique de refus que le plan n'a pas. |
| Disponibilité non vérifiée pour un id absent du registre | false | rejeté | L'appel tombe sur `registry.unknown_command`, comme `cassini run`. |

## Design Notes

Choix faits sans interrogation (epic délégué) : `serve` ne prend pas de projet en argument, car AD-7 le fait venir de `session.hello` ; le catalogue liste toutes les commandes avec un drapeau plutôt que de cacher les indisponibles, pour que l'agent sache ce qui existe. Exemple de ligne :
`{"jsonrpc":"2.0","id":1,"method":"session.hello","params":{"protocol_version":1,"commands":[],"project":"C:/p/p.qgz"}}`

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement
