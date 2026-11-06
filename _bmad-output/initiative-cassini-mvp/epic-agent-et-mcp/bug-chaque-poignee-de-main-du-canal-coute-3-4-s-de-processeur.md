---
id: 44
type: bug
title: "Chaque poignee de main du canal coute 3,4 s de processeur dans les tests, et la recette GitHub depasse ses delais"
parent: epic-agent-et-mcp
covers: []
after: []
assignee: ""
refined: true
hitl: false
risk: high
severity: P3
---

# Chaque poignée de main du canal coûte 3,4 s de processeur dans les tests, et la recette GitHub dépasse ses délais

## Description

La recette GitHub passe au rouge sur des tests du canal local et du canal d'IA externe qui
attendent une réponse du noyau : la réponse arrive après le délai du test (10 s pour
`serve::tests`). Cinq des six dernières recettes sont rouges pour cette raison, sans défaut
du produit. Les tests doivent répondre en millisecondes, et la recette doit rester verte
sans allonger aucun délai.

## Reproduction

1. Recette GitHub du 2026-11-06 sur `76c7a12` (run 42304300147), job « Noyau Rust — fmt,
   tests, clippy », étape « Tests du noyau » (ubuntu-24.04, 4 cœurs) : trois tests
   échouent, `serve::tests::a_cancelled_step_is_skipped_by_the_replay`,
   `a_composer_error_is_passed_on_and_nothing_is_kept`,
   `a_failing_step_stops_the_plan_with_its_step_number`, tous par
   « le noyau devait écrire: Timeout ». Les tests de la bibliothèque y durent 146 s (128 s
   à la recette verte précédente). Runs rouges voisins : 42270010992
   (`closing_the_input_warns_the_relays_and_removes_the_channel`), 42246405465
   (`a_relay_that_stops_reading_does_not_hold_the_switch_off`).
2. En local, un seul test trivial du canal,
   `serve::tests::an_order_is_sent_and_its_answer_becomes_the_result`, lancé seul : 3,33 s
   de mur et 3,48 s de processeur.
3. Mesure par étapes dans ce test : démarrage 5 ms, `session.hello` 3,43 s, un ordre et sa
   réponse 3 ms, fermeture 1 ms. Dans le noyau, le traitement de `session.hello` (ouverture
   du journal, catalogue) prend 10 ms : les 3,4 s sont passées à écrire la réponse.
4. Durées par test (`--report-time`, 4 fils, en local) : 542 tests, 229 s cumulées, dont
   225 s pour les tests `serve::tests`, chacun de 3,8 à 11 s. Les tests du terrain et de la
   géométrie prennent moins de 0,5 s chacun.
5. Attendu : une poignée de main de quelques millisecondes, et des tests du canal qui ne
   consomment presque pas de processeur.

## Cause Hypothesis

La réponse à `session.hello` contient le catalogue des commandes, environ 107 Ko depuis
l'epic 11. Le noyau l'écrit par le formatage d'une valeur JSON, qui produit des milliers de
petites écritures. L'écrivain des tests (`ChanWriter`, dans les tests de `serve` et dans
`core/tests/canal_externe.rs`) ajoute chaque morceau à son tampon, puis cherche une fin de
ligne depuis le début du tampon : le coût est quadratique en la taille de la ligne. Chaque
commande ajoutée au registre allonge le catalogue et aggrave le coût. Sur 4 cœurs, ces
tests se disputent le processeur et dépassent leurs délais. En production, la sortie
standard est tamponnée et ne souffre pas du coût quadratique, mais elle reçoit les mêmes
milliers de petites écritures.

## Acceptance Criteria

1. **La poignée de main est rapide**
   **Given** un test du canal local qui ouvre une session et envoie `session.hello`
   **When** il tourne en local, seul
   **Then** la réponse arrive en moins de 100 ms, et le test entier coûte moins de 0,5 s de
   processeur
2. **Les tests du noyau consomment dix fois moins**
   **Given** les tests de la bibliothèque `cassini-core`
   **When** ils tournent en local sur 4 fils (`--test-threads=4`)
   **Then** le temps de processeur du binaire de test passe de 230 s (mesuré le 2026-11-06,
   59,6 s de mur) à moins de 23 s, chiffres avant et après relevés dans le plan
3. **Les messages du canal sont inchangés**
   **Given** les tests existants du canal local, du canal d'IA externe et de `cassini mcp`
   **When** ils tournent
   **Then** ils passent tous sans modifier aucun message attendu : chaque message reste une
   ligne JSON complète, suivie d'une fin de ligne
4. **Tests du défaut**
   **Given** la suite de tests
   **When** elle tourne
   **Then** un test échoue si l'écriture d'un message redevient quadratique ou découpée en
   milliers de morceaux, et passe sur la correction
5. **La recette GitHub reste verte**
   **Given** la correction poussée sur `main`
   **When** la recette GitHub tourne
   **Then** les jobs Linux et Windows passent les tests du noyau sans dépasser de délai, sur
   au moins deux exécutions
6. **Ou : aucune correction n'est nécessaire, preuve à l'appui**
   **Given** la reproduction
   **When** elle est rejouée sur le code actuel
   **Then** la poignée de main coûte déjà quelques millisecondes, avec la mesure consignée
   dans les Notes — ceci remplace 1 à 5

## Boundaries

- Aucun délai de test n'est allongé.
- Le protocole du canal (AD-2, AD-3) et le contenu de `session.hello` ne changent pas : le
  catalogue reste entier dans la réponse.
- Aucune dépendance ajoutée.

## References

- parent — `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/epic-agent-et-mcp.md`
- contrat d'architecture — `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-2, AD-3
- contrat des messages — `docs/canal-local.md`
- tickets voisins — 5.42 et 5.43 dans `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/tickets.toml`

## Notes

- Decision: 2026-11-06 — propriétaire : ticket créé et traité tout de suite, en optimisant
  le code plutôt qu'en allongeant les délais.
- Open question: les échecs de 5.42 et 5.43 (canal d'IA externe) ont-ils la même cause ? À
  mesurer après la correction ; s'ils restent, ils gardent leur ticket.
- Risque high : canal local (CLAUDE.md, relecture thorough) ; contrôle hors du ticket : la
  recette GitHub verte sur plusieurs exécutions.
