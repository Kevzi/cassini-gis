---
title: "Exécution des appels du relais : la session QGIS exécute ce que l'IA externe demande"
type: 'feature'
ticket: '28'
created: '2026-10-30'
status: 'built'
baseline_revision: '7e2bb23bbb868de6444e9e434dfd7d9ed52e180a'
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

**Problem:** Depuis 5.24, un relais se branche et reçoit le catalogue, mais son `command.run` est « méthode inconnue » : l'IA externe voit la session QGIS sans pouvoir y agir (AD-29).

**Approach:** Le noyau de la session exécute le `command.run` d'un relais par `Session::call`, commandes QGIS dictées à l'extension comme une étape (AD-5), sous un état d'occupation unique valable dans les deux sens ; chaque appel est annoncé à l'extension (`external.call`), annulable, journalisé avec l'origine `external`, et ne rend jamais de chemin absolu.

## Boundaries & Constraints

**Always:**
- Un seul état d'occupation par processus. Occupants : composition et exécution de plan, `command.run` et `journal.replay` de l'extension, appel externe. Un plan proposé non tranché occupe pour le relais seulement ; l'extension peut le remplacer par `plan.propose` et garde son `command.run`. Occupé : `session.busy` aussitôt, rien en file.
- Le relais reçoit `session.busy`, `external.attach` et ses erreurs **aussitôt**, composition comprise : son fil lecteur répond lui-même (AD-4).
- Relais : `external.attach`, `command.run`, `request.cancel` seulement (AD-6), commandes du catalogue disponible.
- AD-9 : un paramètre de chemin (`path_params`) absolu ou sortant du projet (`..`) → `external.path_refused`, rien n'est exécuté ni journalisé ; aucune réponse au relais ni `external.call` ne porte de chemin absolu.
- Erreurs AD-21 ; le rejeu ignore l'origine ; tests de `serve`, `canal_externe`, `hors_ligne.rs`, `cargo deny check` et pytest restent verts.

**Never:**
- Ni dépendance, ni tokio, ni méthode hors AD-6 ; pas de file d'appels.
- Rien dans `cli/src/mcp.rs` (5.25), le panneau (5.26) ni l'extension, qui ignore déjà `external.call`.
- Pas de `external.approve` (ticket à part, Decisions).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Appel noyau | relais : `command.run {command:"system.about"}` | `external.call` `started` puis `finished` vers l'extension ; résultat au relais ; journal `"origin":"external"` | avant `session.hello` : `session.not_started` |
| Appel QGIS | commande `qgis` annoncée | `order.execute` à l'extension, réponse rendue au relais et journalisée | non annoncée : indisponible |
| Occupé, côté relais | composition, ordre ou plan non tranché | `session.busy` aussitôt | — |
| Occupé, côté extension | appel externe en cours | `command.run`, `plan.*`, `journal.replay` : `session.busy` | — |
| Case `ask` allumée | `session.external {ask:true}` | rien n'est exécuté | `external.not_approved` |
| Chemin | `layer.add {path:"/abs/x.gpkg"}` ou `"../x"` | rien n'est exécuté | `external.path_refused` |
| Annulation | `request.cancel` du relais (son `id`) ou de l'extension (`call_id`) | ordre annulé (AD-12), étape journalisée annulée | autre id, autre connexion : `request.unknown` |
| Perte de session | interrupteur coupé ou fin de stdin en plein appel | annulé si annulable, sinon fini et journalisé ; `external.session_lost`, aucune réponse | — |
| Relais parti | déconnexion en plein appel | même traitement, réponse abandonnée, session libérée | — |
| Rejeu | journal avec étapes `external` | rejoué à l'identique | — |

## Decisions (propriétaire, 2026-10-30)

- Ticket coupé : la validation (`external.approve`, 120 s, refus ou délai) part au report (`deferred-work.md`), en ticket avant 5.26. D'ici là, `ask` allumé refuse tout appel externe par `external.not_approved`.

</frozen-after-approval>

## Code Map

- `core/src/serve/external.rs` -- `Hub`. `on_line` (l.202) quitte la boucle : le fil lecteur de `start_conn` (l.332) traite chaque ligne (`attach`, lecture, `session.busy`, méthode inconnue) avec un état partagé `Arc` : `conns`, catalogue (`RefCell` → `Mutex`), occupation, `ask`. Il ne verse dans la file que le `command.run` dont il a **réservé** l'occupation, et `request.cancel`. Il répond par la table des connexions, sans garder de `Sender` (sinon `finish` attendrait tout `FAREWELL`).
- `core/src/serve.rs` -- `Link::side` (l.147) rend à l'appelant le `Detached` du relais en cours ; `run_id` (l.135) devient l'appel en cours `{source, id, call_id}` ; `request_cancel` (l.471) apparie par source ; `handle_line` (l.409) réserve et libère pour `command.run`, `plan.propose`, `plan.execute`, `journal.replay` de l'extension ; boucle (l.277) : le `command.run` d'un relais va à `serve/calls.rs`. `Channel::execute` (l.565) : `Detached` du relais en cours ou interrupteur coupé → annulation si annulable ; réponses vers leur source.
- `core/src/serve/plans.rs` -- `propose`, `execute`, `reject` tiennent `plan_pending` à jour.
- `core/src/serve/calls.rs` (nouveau) -- appel externe : connexion encore inscrite, `ask`, chemins, `call_id` `external-<n>`, `external.call`, `Session::call_from`, résultat par `relativize` puis chemins absolus restants réduits au nom de fichier, réponse ou abandon.
- `core/src/journal/mod.rs:210` -- `call` délègue à `call_from(id, params, origin: Option<&str>)` qui ajoute `"origin"` ; `Replay` ne lit que des clés nommées.
- `core/src/registry.rs` -- `path_params`, `cancellable`, `RegistryError::Cancelled`.
- `core/tests/canal_externe.rs` -- `Session`, `Relay` ; `system.about` (noyau), `layer.describe` (QGIS), `layer.add` (chemin), `analysis.buffer` (annulable) ; `Composer` est public pour un compositeur factice.
- `docs/canal-local.md:242` -- section du canal d'IA externe, table des erreurs.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/journal/mod.rs`, `core/src/journal/tests.rs` -- origine ; test de rejeu d'un journal qui la porte.
- [x] `core/src/serve/external.rs` -- état partagé, lecteur qui répond, réservation.
- [x] `core/src/serve.rs`, `core/src/serve/plans.rs` -- occupation unique, appel en cours par source, annulation, `Detached` et interrupteur en plein ordre.
- [x] `core/src/serve/calls.rs` -- appel externe de bout en bout.
- [x] `core/tests/canal_externe.rs` -- chaque ligne de la matrice, compositeur factice pour le plan non tranché.
- [x] `docs/canal-local.md` -- `command.run` et `request.cancel` du relais, `external.call`, occupation, erreurs.

**Acceptance Criteria:**
- Given un appel QGIS lent du relais, when l'extension demande `command.run`, then `session.busy` ; et réciproquement pour le relais pendant un `command.run` de l'extension.
- Given une session qui a servi des appels externes, when `journal.replay` tourne, then le rejeu est identique et les lignes gardent `"origin":"external"`.
- Given la recette sous Linux et Windows, when `cargo test -p cassini-core`, `hors_ligne.rs`, `cargo deny check` et pytest tournent, then ils passent.

## Implementation Notes

- `plan_pending` est tenu à jour dans `handle_line` (après chaque méthode de l'extension, d'après
  `state.pending`) plutôt que dans chacune des trois fonctions de `plans.rs`, qui n'a pas changé :
  un seul point, qui couvre aussi les échecs de `plan.propose`.
- `Msg::Line` ne porte plus de source (seule l'extension en verse) ; les relais versent
  `Msg::Relay(conn, RelayCall::{Run, Cancel})`. `Link::side` disparaît : la boucle et
  `Channel::execute` traitent chacun ces messages, et `Link::caller_lost` détecte, au début de
  chaque tour d'attente d'un ordre, un relais de l'appel en cours débranché ou coupé par
  l'interrupteur (annulation si annulable).
- Un `command.run` de relais sans `id` (notification) est ignoré : il ne pourrait être ni
  répondu ni annulé.
- Les refus avant exécution (occupation, `ask`, chemin, commande inconnue ou indisponible,
  paramètres mal formés) ne sont pas annoncés par `external.call`.
- Chemins rendus : `relativize` puis, pour tout texte qui est un chemin absolu, son nom de
  fichier ; dans un message, les dossiers du projet et de travail (deux écritures de
  séparateurs) deviennent `.`. Un chemin absolu hors de ces dossiers, au milieu d'un message,
  n'est pas réduit (risque résiduel, seul le texte de l'extension peut en porter).
- `session.not_started` côté relais n'est pas atteignable par l'API publique (l'interrupteur
  exige une session) : gardé par prudence, non testé de bout en bout.
- Fin d'entrée en plein appel QGIS : comme tout ordre, l'étape est annulée même non annulable
  (plus rien ne peut répondre à l'ordre) ; elle est journalisée `cancelled`.

## Plan Change Log

## Review Triage Log

Relecture `thorough` (blind-hunter, edge-case-hunter, verification-gap, intent-alignment), 2026-10-30. Comptes : high 0, medium 3, low 8, false 6, rejetés low 7.

| # | Lentille | Constat | Verdict | Preuve | Suite |
|---|---|---|---|---|---|
| 1 | BH, ECH, VG | Bras `Msg::Relay Run` de `Channel::execute` répond `session.busy` sans libérer la réservation, qui est forcément celle de ce relais | low | inatteignable aujourd'hui (toute méthode qui dicte un ordre est dans `occupies`), mais une méthode ajoutée figerait la session ; correction directe | patch |
| 2 | ECH | `handle_line` libère l'occupation avant de poser `plan_pending` : un relais réserve dans l'intervalle et s'exécute pendant qu'un plan attend | medium | `link.external.release()` puis `set_plan_pending(...)`, deux verrous distincts ; le fil lecteur réserve entre les deux | patch (poser `plan_pending` d'abord) |
| 3 | BH, ECH, IA | Un chemin absolu hors des dossiers du projet et de travail, cité au milieu d'un message, part au relais (AD-9) | medium | `scrub_text` ne remplace que `known_dirs` ; une erreur d'extension sur une couche de l'utilisateur hors projet le cite | patch |
| 4 | BH, ECH | Remplacement des dossiers sans frontière (`Projet code2` → `.2`), et projet à la racine (`/`, `C:\`) qui transforme chaque séparateur | low | `text.replace(dir, ".")` ; `project.parent()` vaut la racine | patch (frontière, racines ignorées) |
| 5 | BH | Une valeur de résultat absolue sous le dossier du projet perd ses sous-dossiers (`donnees/x.gpkg` → `x.gpkg`) | low | `scrub` ne relativise que le dossier de travail | patch |
| 6 | ECH | `looks_absolute` prend `A: voir a/b` pour un chemin et tronque le texte ; `/` seul devient vide | low | test de lettre sans séparateur | patch (séparateur exigé pour la réduction) |
| 7 | VG | Aucun test ne vérifie qu'un résultat réussi d'appel externe porteur de chemins absolus n'en garde aucun | medium | les résultats réussis des tests n'ont aucun chemin | patch (test) |
| 8 | VG | Aucun test : un relais ne peut pas annuler l'appel de l'extension | low | pas de `relay.cancel` pendant un `command.run` de l'extension | patch (test) |
| 9 | VG | Aucun test : `journal.replay` et `plan.execute` de l'extension occupent pour le relais | low | `occupies` non couvert pour ces deux méthodes | patch (test) |
| 10 | BH | Le relais ne distingue pas « plan en attente de l'utilisateur » de « commande en cours » | low | même message `session.busy` ; cas courant (plan proposé non tranché pendant que l'IA appelle) ; correction d'un message | patch |
| 11 | BH | `docs/canal-local.md` nomme `Session::call` au lieu de `call_from` | low | correction directe | patch |
| 12 | BH | `session.hello` hors `occupies` : un relais réservé tournerait sur un autre projet | false | un second `session.hello` rend `session.already_started` : le projet ne change jamais dans une session | rejeté |
| 13 | BH | `request.cancel` d'un relais attend la fin d'une composition, ou vise un appel encore en file | false | pendant une composition le relais n'a aucun appel ; un appel en file est lu avant l'annulation qui le suit (même file), et l'annulation est alors servie dans l'ordre | rejeté |
| 14 | ECH | Chemin imbriqué dans un objet échappe au refus | false | `path_params` ne désigne que des paramètres de premier niveau (registre) | rejeté |
| 15 | ECH | `plans.rs` annoncé modifié par le plan, inchangé | false | écart consigné dans Implementation Notes ; `plan_pending` tenu dans `handle_line` ; le corriger éditerait le plan | rejeté |
| 16 | IA | Le diff ne touche ni `cassini mcp` ni le panneau : la lecture « bout en bout » de l'intention n'est pas montrée | false | l'intention vise le noyau ; relais et panneau sont 5.25 et 5.26 | rejeté |
| 17 | IA | Asymétrie de l'occupation (plan non tranché, réservation en file) non dite par l'intention | false | décision du propriétaire écrite dans Boundaries | rejeté |
| 18 | BH | Relais débranché juste après son `command.run` : l'appel noyau tourne et est journalisé | low | le `Detached` suit le `Run` dans la file ; l'étape réelle est journalisée, seule la réponse se perd ; corriger demande une nouvelle garde | rejeté |
| 19 | BH | Politique incohérente devant un verrou empoisonné | low | n'arrive qu'après une panique d'un fil qui tient le verrou | rejeté |
| 20 | BH, ECH | Casse des chemins Windows dans les messages | low | les messages sont bâtis avec les chemins que le noyau a donnés, même écriture ; le n° 3 réduit de toute façon le reste | rejeté |
| 21 | BH, ECH | Lien symbolique du projet qui pointe dehors | low | lien posé par l'utilisateur dans son propre projet ; la garde demanderait un accès disque | rejeté |
| 22 | BH | Test Python que l'extension ignore `external.call` | low | `_handle_call` (client.py l.530) ne répond qu'aux requêtes à `id` : une notification inconnue est ignorée | rejeté |
| 23 | BH | Ordre des contrôles incomplet dans la doc ; entrée du report sans `status` ; doc du format du journal | low | l'ordre n'est pas un contrat ; les entrées du report prennent `status` à la création du ticket ; aucune doc ne décrit les clés du journal | rejeté |
| 24 | BH | `..` refusé même s'il reste dans le projet | low | plus strict qu'exigé, sans danger, dit par le message d'erreur | rejeté |

## Design Notes

Réservation : `Mutex<Occupancy { busy: Option<Source>, plan_pending: bool }>`. Le relais réserve si `busy` est vide et qu'aucun plan n'attend ; l'extension si `busy` est vide. Une réservation de relais encore en file passe avant l'extension, qui reçoit `session.busy`. Toujours libérer, erreur comprise ; un `command.run` réservé dont la connexion a disparu est libéré sans exécution.

```json
{"method":"external.call","params":{"call_id":"external-3","event":"started","command":"layer.describe","params":{"layer_id":"l1"}}}
{"method":"external.call","params":{"call_id":"external-3","event":"finished","command":"layer.describe","result":{},"duration_ms":41}}
```
`finished` porte `result` ou `error {code, message}`.

## Verification

**Commands:**
- `cargo test -p cassini-core --locked` (Linux) puis sous Windows (PowerShell, `CARGO_TARGET_DIR=C:\ct`) -- expected: tout vert, `canal_externe` compris
- `cargo test -p cassini --test hors_ligne` sous Windows -- expected: vert
- `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check`, `cargo deny check` -- expected: ok
- `python -m pytest qgis-plugin/tests` -- expected: vert
