---
title: "Un relais connecté juste avant la fermeture reçoit l'adieu"
type: 'bugfix'
ticket: '42'
created: '2026-11-06'
status: 'built'
baseline_revision: 'b30280996e5112d79f6be6e130b54b407db4fd63'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Quand la session ferme le canal d'IA externe, elle débloque son fil d'acceptation en se connectant à elle-même. Sous Unix, la file d'attente des connexions est dans l'ordre d'arrivée : un relais connecté juste avant, pas encore accepté, passe avant la connexion de réveil ; le fil l'accepte, voit l'arrêt et le referme sans `external.session_lost`. C'est l'échec intermittent de `closing_the_input_warns_the_relays_and_removes_the_channel` (vraie course du noyau, pas défaut du test).

**Approach:** Le signal d'arrêt du canal porte la raison (`Lost`). Une connexion acceptée après l'arrêt reçoit l'adieu correspondant, écrit d'un seul coup par le fil d'acceptation, puis est refermée. Un test unitaire déterministe couvre le relais en file au moment de l'arrêt.

</frozen-after-approval>

## Implementation Notes

Route oneshot : environ 60 lignes dans un seul fichier, `core/src/serve/external.rs`.

- `Open::stop` passe de `Arc<AtomicBool>` à `Arc<OnceLock<Lost>>` ; `close(why)` fait `stop.set(why)` avant le réveil.
- `accept_loop` : quand l'arrêt est posé et qu'une connexion vient d'être acceptée, elle reçoit `session_lost(why)` en un seul `write_all` (ligne terminée par `\n`), puis `shutdown`. La connexion de réveil reçoit aussi l'adieu : l'écriture échoue ou se perd, sans effet. Une connexion neuve a un tampon vide (Unix ≥ 200 Ko, tube Windows 64 Ko) : l'écriture d'une ligne ne bloque pas, l'extinction n'attend pas un relais muet.
- Limite connue et acceptée : seule la première connexion en file est prise. Un deuxième relais en file derrière elle, ou arrivé après le réveil, voit une coupure franche — comme avant, sans perte de donnée.
- Test unitaire dans `external.rs` (`mod tests`) : ouvrir un `Listener`, connecter un client (en file, pas accepté), poser l'arrêt `SessionEnded`, lancer `accept_loop` : le client lit `external.session_lost` raison `session_ended`, puis la fin du flux. Déterministe sous Unix (file d'attente) comme sous Windows (instance en attente).
- Ne change pas : le protocole (messages, raisons, version), le refus des clients distants, l'échéance commune `FAREWELL` des adieux.

- Cause confirmée : c'est une vraie course du noyau, pas un défaut du test. La contre-épreuve le montre : sans l'envoi de l'adieu, le nouveau test échoue (« EOF while parsing »), c'est-à-dire la fin du flux reçue au lieu de l'adieu, comme en recette. Avec l'envoi, il passe.
- Fichiers touchés : `core/src/serve/external.rs` (correctif, test, doc de `close`) et `core/src/external/mod.rs` (doc de `wake`).
- Ran 500 times in a row on Linux (WSL2): `closing_the_input_warns_the_relays_and_removes_the_channel` passed every time.

## Plan Change Log

## Review Triage Log

Relecture rapide (quick), 1 passage : 0 high, 0 medium, 3 low, 1 maybe-false.
- low, patch : la doc de `wake` (`core/src/external/mod.rs`) disait que la connexion de réveil était refermée aussitôt ; elle reçoit maintenant l'adieu d'abord. Commentaire corrigé.
- low, patch : la doc de `Hub::close` promettait l'adieu à chaque relais sans dire la limite acceptée (deuxième relais en file). Commentaire complété.
- low, rejeté : le test unitaire passe par `accept_loop` sans passer par `Hub::close`. Le chemin `close` est déjà couvert par le test d'intégration, joué 500 fois.
- maybe-false, defer : sous Windows, `wake` peut attendre ~5 s quand un relais a pris l'instance en attente. Le défaut existait avant ce changement ; il est noté dans `deferred-work.md`.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy -p cassini-core --all-targets -- -D warnings` -- expected: aucun avertissement
- `cargo test -p cassini-core` -- expected: tout vert, nouveau test compris
- `for i in $(seq 500); do cargo test -q -p cassini-core --test canal_externe -- --exact closing_the_input_warns_the_relays_and_removes_the_channel || break; done` -- expected: 500 passages
