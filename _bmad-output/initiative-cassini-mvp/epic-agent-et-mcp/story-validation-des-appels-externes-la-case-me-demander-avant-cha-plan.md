---
title: "Validation des appels externes : la case « Me demander avant chaque appel »"
type: 'feature'
ticket: '29'
created: '2026-10-30'
status: 'done'
baseline_revision: '106bfa75b0d6f246f9bf5a24f6d6357bca6583fa'
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

**Problem:** Depuis 5.28, la case `ask` allumée refuse tout appel d'un relais : l'utilisateur prudent ne peut pas laisser l'IA externe agir en validant chaque appel (AD-29, Validation).

**Approach:** Case allumée, le noyau demande l'accord de l'extension par `external.approve {call_id, command, params}` avant d'exécuter ; il attend sans cesser de lire (AD-4), au plus 120 s, annulable. Accord : chemin de 5.28 inchangé. Toute autre issue : `external.not_approved`, rien d'exécuté.

## Boundaries & Constraints

**Always:**
- L'attente occupe la session : tout autre demandeur (relais ou extension) reçoit `session.busy` ; `session.external` reste servi.
- `external.call` `started` part **après** l'accord : un appel refusé n'apparaît ni au fil du panneau ni au journal.
- Seul `{"approved": true}` vaut accord ; refus, réponse d'erreur, réponse malformée, délai, annulation → `external.not_approved` (message qui dit la cause).
- Annulation pendant l'attente par `request.cancel` du relais (son `id`) ou de l'extension (`call_id`), commande annulable ou non : rien n'est encore exécuté ; réponse `{"cancelled": true}`.
- Session coupée (interrupteur éteint, fin d'entrée) ou relais parti pendant l'attente : rien n'est exécuté, aucune réponse au relais, qui reçoit `external.session_lost` (5.28).
- Délai de 120 s par défaut, réglable par les tests via l'API publique de `serve`.
- Contrôles de 5.28 (commande, disponibilité, chemins AD-9) **avant** la demande : un appel refusé d'office ne dérange pas l'utilisateur.
- Erreurs AD-21 ; tests de `serve`, `canal_externe`, `hors_ligne.rs`, `cargo deny check` et pytest restent verts.

**Never:**
- Ni dépendance, ni tokio, ni méthode hors AD-6 (`external.approve` y figure déjà).
- Rien dans l'extension réelle ni le panneau (Autoriser / Refuser : 5.26), ni dans `cli/src/mcp.rs`.
- Pas de file d'appels en attente d'accord.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Accord | case allumée, extension répond `{approved:true}` | `started`, exécution, `finished`, journal `external`, résultat au relais | — |
| Refus | `{approved:false}` | rien d'exécuté ni journalisé, aucun `external.call` | `external.not_approved` |
| Erreur de l'extension | réponse `error` (ex. méthode inconnue) | idem | `external.not_approved` |
| Délai | pas de réponse avant le délai (court en test) | `request.cancel {id}` de la demande vers l'extension | `external.not_approved` |
| Annulation | `request.cancel` du relais ou de l'extension (`call_id`) | `{cancelled:true}`, `request.cancel` de la demande vers l'extension | `external.not_approved` au relais |
| Occupé | autre relais ou `command.run` de l'extension pendant l'attente | `session.busy` aussitôt | — |
| Session coupée | interrupteur éteint ou fin d'entrée pendant l'attente | rien d'exécuté, aucune réponse | `external.session_lost` |
| Case éteinte | `ask:false` | aucun `external.approve`, chemin de 5.28 | — |

</frozen-after-approval>

## Code Map

- `core/src/serve/calls.rs` -- `call` (l.46) : retirer le refus systématique (l.53-62) ; après les contrôles et l'attribution de `call_id`, poser `link.current` **avant** l'attente puis, si `ask`, appeler la nouvelle `approve(...)`, et seulement ensuite notifier `started`. `run` (l.25) répond déjà rien si `input_closed`, et `hub.send` ne part pas vers une connexion retirée : la perte n'a pas besoin d'un cas à part.
- `core/src/serve.rs` -- `Link` (l.166) gagne `approve_timeout: Duration` ; `pub const APPROVE_TIMEOUT` (120 s) à côté de `CANCEL_GRACE` (l.48) ; `serve_with` (l.289) et `serve_with_channel` (l.272, seul appelant : les tests) prennent ce délai. Modèle de la boucle d'attente : `Channel::execute` (l.664-781) — même traitement de `Msg::Detached`, `Msg::Relay Run` (`release_for` + `busy`), `Eof`/`Failed`, `classify`. Réutiliser `Current::answers` (l.126), `cancel_notification` (l.615), `busy`, `respond`, `external::session_external`, `link.caller_lost()` (l.185), le compteur `next_order` pour l'identifiant `core-<n>` de la demande. Ne pas toucher `request_cancel` (l.572) : il exige un ordre annulable, alors qu'ici toute attente s'annule.
- `core/src/serve/external.rs` -- rien à changer : le fil lecteur a déjà réservé l'occupation, qui couvre l'attente.
- `core/tests/canal_externe.rs` -- `Session::start_with` (l.157) passe le délai ; ajouter un constructeur à délai court. `with_the_ask_box_on_nothing_is_run` (l.841) devient l'accord et la case éteinte. `Session::expect_order`, `reply`, `Relay::run/cancel`, `code_of`, `assert_silent`, `journal` servent tels quels.
- `qgis-plugin/cassini_qgis/client.py:499` -- lecture seule : `external.approve` reçoit `-32601` (donc refus), un `request.cancel` d'id inconnu est ignoré.
- `docs/canal-local.md` -- l.276-278 (`session.external`), l.335 (`command.run` relais), l.365 (Occupation), l.480 (table d'erreurs) : décrire `external.approve` et ses issues.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/serve.rs` -- `APPROVE_TIMEOUT`, champ `approve_timeout` du `Link`, paramètre de `serve_with` et `serve_with_channel` -- délai réglable par les tests.
- [x] `core/src/serve/calls.rs` -- `approve` : demande, boucle d'attente, retrait de la demande ; `call` réordonné -- cœur du ticket.
- [x] `core/tests/canal_externe.rs` -- un test par ligne de la matrice, client brut du canal et extension simulée.
- [x] `docs/canal-local.md` -- section `external.approve` (noyau vers extension), issues, mise à jour de la case et de la table d'erreurs.

**Acceptance Criteria:**
- Given un appel refusé puis un appel accordé, when on lit le journal, then seule l'étape accordée y figure, avec `"origin":"external"`.
- Given une demande retirée (délai ou annulation), when l'extension répond ensuite à l'ancienne demande, then la réponse est ignorée et la session sert l'appel suivant.
- Given la recette sous Linux et Windows, when `cargo test -p cassini-core`, `hors_ligne.rs`, `cargo deny check` et pytest tournent, then ils passent.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Relecture `thorough` (blind-hunter, edge-case-hunter, verification-gap, intent-alignment), 2026-10-30. Comptes : high 0, medium 2, low 7, false 5, rejetés low 3.

| # | Lentille | Constat | Verdict | Preuve | Suite |
|---|---|---|---|---|---|
| 1 | BH, ECH | Entrée illisible (`Msg::Failed`) pendant l'attente : le relais reçoit une réponse d'erreur `external.session_lost` avant l'avis de perte | low | `run` ne se tait que sur `input_closed` ; la doc promet « aucune réponse » | patch (`run` se tait aussi si `failure`) |
| 2 | BH, ECH | Une écriture ratée dans la boucle d'attente ne l'arrête pas : attente jusqu'à 120 s sur une sortie morte | medium | retours de `send_or_fail` ignorés, contrairement à `Channel::execute` | patch |
| 3 | BH | Toute réponse d'erreur dit « mettez-la à jour » | low | seul `-32601` signale une extension ancienne | patch |
| 4 | BH, ECH | Case décochée pendant l'attente : comportement non dit | low | la demande en cours attend toujours sa réponse ; rien n'est exécuté sans accord | patch (doc) |
| 5 | BH, VG | Liste des méthodes `session.busy` copiée dans `approve` et `answer_during_order` | low | une méthode ajoutée d'un côté seulement répondrait « méthode inconnue » | patch (aide commune) |
| 6 | VG, BH | Aucun test d'une réponse rejetée par `classify` portant l'id de la demande | medium | les réponses testées sont toutes bien formées ; l'inverser ferait attendre 120 s sans test rouge | patch (test) |
| 7 | VG, BH | `plan.execute`, `session.hello`, `agent.models` non testés occupés pendant l'attente | low | la boucle du test n'en couvre que quatre | patch (test) |
| 8 | BH | « contrôles avant la demande » testé pour la seule commande inconnue | low | ni chemin refusé ni commande indisponible, case cochée | patch (test) |
| 9 | BH | En-tête du test et paragraphe « Occupation » mal tenus | low | « des deux tickets » pour trois ; ligne non coupée | patch |
| 10 | BH | Ligne illisible pendant l'attente, bras `RelayCall::Run` non testés | low | bras inatteignable (le fil lecteur refuse d'abord) ; écho d'erreur déjà testé en 5.28 | rejeté |
| 11 | BH | Le délai de 120 s peut dépasser celui du client MCP ; non réglable | low | 120 s fixé par AD-29 ; le relais (5.25) attend sans délai propre | rejeté |
| 12 | BH | `lost()` réutilise le code public `external.session_lost` | false | après le n° 1, cette erreur ne part jamais (relais parti, entrée fermée ou panne) | rejeté |
| 13 | BH | Aucune trace des appels refusés | false | « rien n'est journalisé » est voulu par l'intention (matrice) | rejeté |
| 14 | BH | Paramètre de test dans la signature publique de `serve_with_channel` | false | décidé par le plan (« réglable par les tests via l'API publique ») ; seuls les tests l'appellent | rejeté |
| 15 | IA | Le problème utilisateur n'est résolu qu'avec le panneau | false | Boundaries : Autoriser / Refuser dans 5.26 | rejeté |
| 16 | IA | Perte pendant l'attente : `external.session_lost` et non `not_approved` | false | la matrice gelée prévoit `external.session_lost` pour la session coupée | rejeté |
| 17 | VG | Bras `RelayCall::Run` inatteignable | low | refusé par prudence, comme dans `Channel::execute` | rejeté |

## Design Notes

```json
{"jsonrpc":"2.0","id":"core-4","method":"external.approve","params":{"call_id":"external-3","command":"layer.describe","params":{"layer_id":"l1"}}}
{"jsonrpc":"2.0","id":"core-4","result":{"approved":true}}
```

Les `params` montrés sont ceux du relais (relatifs, AD-9). Boucle d'attente : `recv_timeout` jusqu'à l'échéance ; en tête de chaque tour, `caller_lost()` ou `input_closed` → fin sans exécution. Retrait (délai, annulation, perte) : `request.cancel {id: "core-<n>"}` vers l'extension, pour que le panneau ferme sa question ; une réponse tardive à une demande retirée tombe dans « réponse sans ordre en vol », déjà ignorée par `handle_line`.

## Verification

**Commands:**
- `cargo test -p cassini-core --locked` (Linux) puis sous Windows (PowerShell, `CARGO_TARGET_DIR=C:\ct`) -- expected: tout vert, `canal_externe` compris
- `cargo test -p cassini --test hors_ligne` sous Windows -- expected: vert
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo deny check` -- expected: ok
- `python -m pytest qgis-plugin/tests` -- expected: vert
