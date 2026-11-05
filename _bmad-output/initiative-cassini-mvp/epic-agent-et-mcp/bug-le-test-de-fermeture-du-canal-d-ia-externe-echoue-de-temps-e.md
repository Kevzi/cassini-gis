---
id: 42
type: bug
title: "Le test de fermeture du canal d'IA externe echoue de temps en temps sous Linux"
parent: epic-agent-et-mcp
covers: []
after: []
assignee: ""
refined: true
hitl: false
risk: low
severity: P3
---

# Le test de fermeture du canal d'IA externe echoue de temps en temps sous Linux

## Description

Un test du canal d'IA externe ferme une session QGIS alors que deux relais d'IA externe y sont branchés. Chaque relais doit recevoir
`external.session_lost` (raison `session_ended`), puis la fin du flux, et le canal doit
disparaître. Environ une recette sur huit, dans le job Linux, un relais voit la connexion se
fermer sans avoir reçu ce message. La recette passe alors au rouge sans vrai défaut connu, et
une relance la remet au vert.

## Reproduction

1. Pousser sur `main` et laisser tourner la recette (`.github/workflows/recette.yml`, job
   « Noyau Rust — fmt, tests, clippy », étape « Tests du noyau », image ubuntu-24.04).
2. Constaté sur 30 recettes jusqu'au 2026-11-05 : 4 échecs de ce test, tous dans ce job Linux,
   aucun dans le job Windows qui lance pourtant les mêmes tests du noyau. Recettes en échec :
   41227782609 et 41244719816 (2026-10-31), 41798468347 (2026-11-04), 42041089313
   (2026-11-05, verte à la relance).
3. Message, à chaque fois : « la connexion s'est fermée » — le relais du test reçoit la fin
   du flux au lieu du message d'adieu.
4. Attendu : les deux relais reçoivent `external.session_lost`, puis la fin du flux.
5. En local, sous Linux : lancer ce seul test 500 fois de suite (voir Notes pour son nom),
   par exemple avec `for i in $(seq 500); do cargo test -p cassini-core --test canal_externe -- --exact <nom du test> || break; done`.
   Pas encore fait au 2026-11-05.

## Cause Hypothesis

Le second relais (`quiet`) se connecte juste avant la fermeture de la session, sans s'attacher.
Sous Linux, la connexion peut aboutir côté relais avant que la session l'ait enregistrée. Dans
ce cas, la fermeture ne la prévient pas, et le relais ne voit que la fin du flux. Deux lectures
restent possibles : le test n'attend pas que la session ait pris la connexion (défaut du test
seul), ou une connexion acceptée mais pas encore enregistrée au moment de la fermeture ne reçoit
jamais l'adieu (vraie course dans le noyau, sans perte de données : l'IA externe voit une
coupure franche au lieu d'un adieu).

## Acceptance Criteria

1. **Chaque relais connecté reçoit l'adieu**
   **Given** une session QGIS ouverte avec le canal d'IA externe activé, un relais attaché et
   un relais seulement connecté
   **When** la session se ferme
   **Then** chacun des deux relais reçoit `external.session_lost` avec la raison
   `session_ended`, puis la fin du flux, et le canal n'existe plus
2. **Le test ne dépend plus de l'ordre d'arrivée**
   **Given** le test lancé 500 fois de suite sous Linux
   **When** il tourne
   **Then** il passe à chaque fois, et la cause trouvée (le test ou le noyau) est écrite dans
   Notes
3. **Tests cover the condition found and fixed**
   **Given** les tests du noyau, lancés par la CI sous Linux et sous Windows
   **When** ils tournent
   **Then** un test couvre la connexion qui arrive juste avant la fermeture, et les autres tests
   du canal d'IA externe restent verts
4. **Or: no change is needed, with proof**
   **Given** la reproduction
   **When** elle est rejouée sur le code courant
   **Then** le test passe 500 fois de suite sous Linux, la raison des échecs passés est
   démontrée (par exemple un défaut de la machine de CI), et 30 recettes d'affilée sont vertes
   sur ce test, preuve consignée dans Notes — ceci remplace 1 à 3

## Boundaries

- Must not change: le protocole du canal d'IA externe (messages, raisons, numéro de version),
  le refus des clients distants, et l'extinction qui ne doit pas attendre un relais muet.

## References

- parent — `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/epic-agent-et-mcp.md`
- ticket d'origine du canal — `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-canal-d-ia-externe-cote-session-le-noyau-de-qgis-accepte-un-plan.md`
- recette en échec — https://github.com/Kevzi/cassini-gis/actions/runs/42041089313

## Notes

- Constaté le 2026-11-05 en suivant la recette du ticket 1.11 ; l'échec ne vient pas de 1.11.
- Test concerné : `closing_the_input_warns_the_relays_and_removes_the_channel`, dans
  `core/tests/canal_externe.rs` ; le message vient de la lecture d'une ligne par le relais du
  test (ligne 360 au 2026-11-05).
- Decision (2026-11-05, propriétaire) : sévérité P3 et risque low, car l'utilisateur ne perd ni donnée ni résultat ; le
  coût principal est une recette rouge sans raison.
