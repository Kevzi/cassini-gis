---
title: "L'extinction du canal d'IA externe attend que chaque relais ait fini sa file de lignes"
type: 'bugfix'
ticket: '43'
created: '2026-11-06'
status: 'built'
baseline_revision: 'f9a29f95fd4db04d886abbced5211834b31bd89e'
route: 'oneshot'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['quick', 'blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le test `a_relay_that_stops_reading_does_not_hold_the_switch_off` (`core/tests/canal_externe.rs`) dépasse 1,5 s sous Windows chargé. La mesure (2026-11-06, Windows, test instrumenté) montre que la cause est dans le noyau et non la seule charge : l'échéance commune des adieux tient bien 0,5 s, mais `Conn::finish` attend ensuite chaque fil lecteur, qui traite encore toutes les lignes déjà lues de son relais (une réponse de catalogue par `external.attach`) ; ces attentes, faites l'une après l'autre, ajoutent 0,27 s au repos et 0,7 à 0,9 s sous charge, sans borne : un relais qui inonde le canal retient l'extinction aussi longtemps qu'il veut, contre AD-4.

**Approach:** Chaque connexion porte un drapeau « retirée », levé par la boucle quand elle retire la connexion (fermeture du canal ou débranchement) ; le fil lecteur le regarde avant chaque ligne et s'arrête sans la traiter. L'extinction ne dépend plus alors que de l'échéance commune, et le test garde son seuil, qui sépare l'échéance commune (≈ 0,5 s) d'une échéance par connexion (≈ 2 s pour quatre).

**Décision du propriétaire (2026-11-06, après la relecture quick) :** le seuil du test passe de 1,5 s à 1 s, pour qu'il casse aussi, sous charge, si les lignes en file sont de nouveau traitées.

</frozen-after-approval>

## Tasks & Acceptance

**Execution:**
- [x] `core/src/serve/external.rs` -- ajouter à `Conn` un `Arc<AtomicBool>` partagé avec le fil lecteur créé dans `start_conn` ; le lever dans `close` dès que les connexions sont tirées de la table (avant les adieux) et au début de `finish` ; dans la boucle du lecteur, après chaque `read_until` réussi, sortir sans traiter la ligne ni envoyer `Msg::Detached` si le drapeau est levé -- borne le temps de `finish` à une ligne en cours au plus.
- [x] `core/tests/canal_externe.rs` -- corriger le commentaire faux « elles sont traitées » (les lecteurs ne sont pas ordonnés avec l'appel de l'extension) et dire ce que le test garantit : l'extinction tient l'échéance commune même quand les relais ont encore des lignes en file. Seuil resserré à 1 s (décision du propriétaire).

**Acceptance Criteria:**
- Given quatre relais muets ayant envoyé 300 `external.attach` chacun, when l'interrupteur est éteint, then la réponse arrive en moins de 1 s, y compris sous Windows chargé (16 boucles actives), sur 5 exécutions de suite.
- Given le garde-fou du lecteur retiré, sous Windows chargé, when le test tourne, then il échoue (mutation, annulée ensuite).
- Given l'échéance des adieux remise temporairement par connexion (`deadline` calculée dans la boucle de `finish`), when le test tourne, then il échoue (vérification par mutation, annulée ensuite).
- Given les autres tests de `canal_externe` (débranchement d'un relais, fermeture de session, adieux), when la recette tourne, then ils passent sous Linux et Windows.

## Implementation Notes

Route oneshot : ~30 lignes dans deux fichiers, cause localisée par la mesure.

Mesure de départ (Windows, instrumentation retirée) : au repos `close` = 0,79 s (attente 0,51 s, lecteurs 0,145 + 0,080 + 0,027 + 0,013 s) ; sous charge 1,20 à 1,41 s, lecteurs jusqu'à 0,34 s chacun. Arrêt du flux, écrivains, acceptation : quelques millisecondes.

Fichiers : `core/src/serve/external.rs` (drapeau `retired` de `Conn`, levé dans `close` avant les adieux et dans `finish`, lu par le lecteur après chaque `read_until`), `core/tests/canal_externe.rs` (commentaires, seuil à 1 s). Écart au texte de la tâche : le drapeau est lu après tout retour de `read_until`, fin de flux et erreur comprises ; sans effet, `hub.detached` ne fait rien pour une connexion déjà retirée.

Résultats (2026-11-06) :
- AC 1 : Windows, 16 boucles actives, 5/5 verts sous le seuil de 1 s, extinction 0,52 à 0,54 s (avant : 1,20 à 1,41 s).
- Mutation du garde-fou (lecture du drapeau retirée), Windows chargé, seuil 1 s : 3/3 en échec (1,27 à 1,49 s) ; annulée.
- AC 2 : échéance par connexion remise à la main → 2,06 s, le test échoue ; mutation annulée.
- AC 3 : `canal_externe` 32/32 sous Linux et sous Windows (Windows deux fois de suite, sans charge ; une passe faite sous 32 boucles de charge en a fait échouer 10 sur leur attente de 10 s, sans rapport avec le correctif) ; `cassini-core` complet 588/0 sous Linux ; fmt et clippy propres.

## Verification

**Commands:**
- `cargo test -p cassini-core --test canal_externe` (Linux) -- expected: tous verts
- même commande sous Windows (`cargo.exe`, `CARGO_TARGET_DIR` à part), le test visé 5 fois sous charge -- expected: 5/5 sous 1,5 s
- `cargo fmt --check` et `cargo clippy -p cassini-core --all-targets -- -D warnings` -- expected: propres

## Review Triage Log

Relecture `quick` (1 lentille), 6 remarques :
- medium — relecture `quick` sur une entrée `high` du canal, contraire à CLAUDE.md §8 : la voie oneshot l'impose ; proposée au propriétaire (relecture `thorough`).
- medium — le test ne prouve pas l'effet du drapeau : rien ne garantit qu'il reste des lignes en file à l'extinction ; avant correction, le test passait déjà sous charge locale (1,20 à 1,41 s). Correctif non simple (plus de lignes = plus de mémoire, ou seuil plus serré = moins de marge) : question posée au propriétaire.
- low (corrigé) — commentaire du test « chaque lecteur garde des lignes en file … quelle que soit la charge » : il dit maintenant que le test ne le garantit pas.
- low (corrigé) — commentaire « l'extension reste servie pendant que les lecteurs traitent ces lignes » : affirmait un ordre ; remplacé.
- low (corrigé) — doc de `Conn::retired` : une ligne en cours finit encore ; dit maintenant « au plus la ligne en cours ».
- false — lecture du drapeau aussi sur fin de flux et erreur : `Msg::Detached` n'a d'effet que pour une connexion encore dans la table ; consigné dans les notes.
- low (corrigé) — tâches non cochées, critères sans preuve : résultats ajoutés aux notes.

Relecture `thorough` (blind-hunter, edge-case-hunter, verification-gap, intent-alignment) :
- low (corrigé) — seuil de 1 s contraire au texte figé « le test garde son seuil » (4 lentilles) : décision du propriétaire, ajoutée au bloc figé, tâche et critère mis à jour.
- medium (différé) — le test ne prouve pas le garde-fou au repos (avant correctif : 0,79 s, sous le seuil) ; il ne le prouve que sous charge (mutation 3/3 en échec) : inscrit dans deferred-work.md.
- false — course entre la lecture du drapeau et `reserve` : `calls::run` libère l'occupation quand la connexion a disparu (`core/src/serve/calls.rs:41`), comme `serve.rs:759` et `calls.rs:210` ; déjà le cas avant ce changement.
- false — lignes reçues abandonnées sans réponse : avant le changement, leurs réponses partaient déjà vers une connexion retirée de la table (`Shared::send` ne trouve rien) ; l'IA externe ne voit pas de différence.
- false — petite fenêtre entre le vidage de la table et `retire()` : mêmes réponses perdues, pas d'effet visible.
- low (corrigé) — commentaire du test qui inversait la raison du seuil : réécrit.
- low (corrigé) — fin de flux silencieuse après retrait, à dire dans le code : commentaire du lecteur complété.
- low (rejeté) — `retire()` sans effet sur le chemin `detached` : sans danger ; un seul point d'entrée garde le code simple.
- low (rejeté) — 1 s pourrait rester fragile sous Windows chargé : 0,52 à 0,54 s mesurées sous 16 boucles ; le contrôle reste la recette GitHub (hors ticket).
- low (rejeté) — `SeqCst` plus fort que nécessaire : identique aux drapeaux `stop` et `ask` voisins.
- low (corrigé) — compte rendu du plan en retard (statut, critères, journal) : mis à jour.
