---
title: 'Un message du canal, une seule écriture'
type: 'bugfix'
ticket: '44'
created: '2026-11-06'
status: 'built'
baseline_revision: '7cda98bdaab818d38eb91ac7c1f2c82e31f6b95c'
route: 'oneshot'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La réponse à `session.hello` (catalogue d'environ 107 Ko) est écrite par le
formatage d'une valeur JSON, en milliers de petites écritures ; l'écrivain des tests relit tout
son tampon à chaque morceau (coût quadratique). Chaque poignée de main coûte 3,4 s de
processeur, 225 s des 229 s des tests du noyau, et la recette GitHub sur 4 cœurs dépasse ses
délais.

**Approach:** Le noyau sérialise chaque message du canal en une chaîne, fin de ligne comprise,
puis l'écrit d'un seul `write_all` ; même règle pour la ligne envoyée à un relais d'IA externe.
Les écrivains de test ne cherchent la fin de ligne que dans les octets nouveaux. Un test
échoue si un message redevient découpé.

</frozen-after-approval>

## Implementation Notes

Voie courte : environ 60 lignes sur trois fichiers, cause mesurée avant le plan (voir le
ticket, Reproduction). Contraintes du ticket : aucun délai de test allongé, protocole et
contenu de `session.hello` inchangés (AD-2, AD-3), aucune dépendance ajoutée.

Fichiers visés :
- `core/src/serve.rs` — `Link::send` (`writeln!(out, "{message}")`) ; `ChanWriter` des tests.
- `core/src/serve/external.rs` — fil d'écriture d'un relais (`writeln!(write_half, "{line}")`,
  deux écritures par ligne).
- `core/tests/canal_externe.rs` — `ChanWriter`, même motif quadratique.

Mesure de départ (2026-11-06, local, 16 cœurs, `--test-threads=4`) : binaire de test de
`cassini-core` 59,6 s de mur, 230 s de processeur ; test seul
`serve::tests::an_order_is_sent_and_its_answer_becomes_the_result` 3,33 s de mur, 3,48 s de
processeur.

Fait (2026-11-07) :
- `core/src/serve.rs` : nouvelle fonction `line_of` (message sérialisé d'un bloc, fin de ligne
  comprise) ; `Link::send` l'écrit d'un seul `write_all`. `ChanWriter` des tests découpe les
  seuls octets reçus. Nouveau test
  `serve::tests::each_message_leaves_in_a_single_write_with_its_end_of_line` : sur l'ancienne
  écriture, il échoue avec 22 889 écritures pour 2 messages ; il passe sur la correction.
- `core/src/serve/external.rs` : la ligne d'un relais et sa fin partent en un `write_all`.
- `core/tests/canal_externe.rs` : même `ChanWriter` linéaire.

Mesures après (même poste, même commande) : binaire de test de `cassini-core` 3,80 s de mur,
5,00 s de processeur (avant : 59,6 s et 230 s) ; test seul 0,07 s de mur, 0,02 s de processeur
(avant : 3,33 s et 3,48 s). `core/tests/canal_externe.rs` : 32 tests en 5,5 s sous Linux, 0,8 s
sous Windows.

Recette locale : `cargo test -p cassini-core --locked` vert sous WSL ; `cargo test -p
cassini-core -p cassini-ai -p cassini --locked` vert sous Windows (la couche IA ne compile pas
sous WSL faute de libclang) ; `cargo fmt --all --check` et `cargo clippy -p cassini-core
--all-targets --locked -- -D warnings` sans avertissement. Le critère 5 du ticket (recette
GitHub verte deux fois) se constate après la poussée.

## Verification

**Commands:**
- `cargo test -p cassini-core -p cassini-ai -p cassini --locked` -- expected: tout passe
- `/usr/bin/time -f "%e %U" <binaire de test cassini-core> -q --test-threads=4` -- expected: processeur < 23 s
- `cargo clippy -p cassini-core -p cassini-ai -p cassini --all-targets --locked -- -D warnings` et `cargo fmt --all --check` -- expected: aucun avertissement

## Review Triage Log

Passe 1 (relecture complète, quatre lentilles) : 1 medium, 4 low, 0 false, 0 maybe-false.

- medium, patch — `cli/src/mcp/relay.rs` `Relay::write` formatait une valeur JSON sur une
  connexion sans tampon, un appel système par morceau (même cause, autre sens du canal) :
  sérialisé d'un bloc puis un seul `write_all`. Signalé par trois lentilles.
- low, patch — le commentaire de `line_of` laissait croire que la production payait les
  secondes ; la sortie standard regroupe les morceaux, le coût était celui de l'écrivain des
  tests. Commentaire corrigé.
- low, patch — l'assertion `> 20` du nouveau test devient l'égalité avec le nombre de commandes
  du registre.
- low, patch — l'écrivain de test linéaire n'avait pas de test propre : ajout de
  `the_test_writer_rebuilds_lines_whatever_the_pieces` (ligne en trois morceaux, deux lignes
  d'un coup, appel vide, fin sans retour).
- low, rejeté — aucun test ne compte les écritures du fil d'écriture d'un relais côté noyau :
  la ligne y était déjà une chaîne, l'écart n'était que d'un appel système, et le tester
  demanderait une couture dans le fil.
- low, rejeté — `ChanWriter` copié dans deux fichiers : antérieur au ticket, les deux copies
  sont corrigées.
- low, rejeté — pas de garde automatique du temps de processeur : le test du nombre
  d'écritures est le garde, la mesure reste dans ce plan.
- low, rejeté — le test ne vérifie pas `flush` : inchangé par le diff, toujours appelé.

Après les correctifs : `cargo test -p cassini-core` vert sous WSL (544 tests de
bibliothèque) ; `cargo test` et `cargo clippy -D warnings` sur `cassini-core`, `cassini-ai`,
`cassini` verts sous Windows ; `cargo fmt --all --check` propre.
