---
id: 12
type: bug
title: "Le rejeu dans la meme session diverge sur une lecture du projet sans dire pourquoi"
parent: epic-registre-projet-import
covers: ["R5"]
after: []
assignee: ""
refined: true
hitl: false
risk: high
severity: P2
---

# Le rejeu dans la même session diverge sur une lecture du projet sans dire pourquoi

## Description

Un journal qui porte une lecture du projet (`project.inventory`) est rejoué en fin de session. Le rejeu s'arrête dès l'étape 1 sur `replay.diverged`, avec un motif muet : « la commande project.inventory ne redonne pas le même résultat ». L'inventaire rejoué voit l'état du moment : les couches posées depuis par la session, la légende rangée par `layer.arrange`, une couche retirée par `layer.remove`. Il ne peut pas redonner ce que l'inventaire d'origine a vu.

Décision tranchée en party le 2026-11-10, à confirmer par le propriétaire : la lecture reste journalisée, rejouée et comparée (AD-11) ; le rejeu identique promis part de l'état d'ouverture du projet (R5 : « y compris après fermeture et réouverture »). Le défaut à corriger est double : le motif ne dit ni la cause ni le remède, et la recette d'Annecy rejoue dans la session au lieu de rejouer après réouverture.

## Reproduction

1. Poste Windows, QGIS 3.44.15, modèle installé du 2026-11-10.
2. `qgis-plugin/tests/probe_recette.py` sur `donnees-test-cassini/annecy/projet-test-annecy.qgz` avec `qgis-plugin/tests/recette/annecy.json` (20 phrases, la première demande l'inventaire).
3. Obtenu, en fin de session : `replay.diverged` à l'étape 1, `project.inventory`. Journal : `projet-test-annecy-recette-20261110-160135.cassini/journal.jsonl`, 29 lignes.
4. Attendu : un motif qui nomme une couche posée par une exécution précédente et dit de rouvrir le projet sans l'enregistrer ; après réouverture, le rejeu rend `identical: true`.

## Cause Hypothesis

Le rejeu compare l'empreinte du résultat rejoué à celle du journal. Dans la même session, le résultat rejoué porte les couches de l'exécution d'origine (leurs ids sont ceux du journal), et le motif de divergence ne regarde pas ce qu'il contient. La sonde appelle `journal.replay` sans rouvrir la copie du projet.

## Acceptance Criteria

1. **Le motif nomme la cause et le remède**
   **Given** un journal dont une lecture du projet est rejouée dans la session où des couches de l'exécution d'origine sont encore au projet
   **When** le rejeu diverge sur cette ligne
   **Then** l'erreur reste `replay.diverged` à cette étape, et son message nomme une couche posée par une exécution précédente et dit de fermer puis rouvrir le projet sans l'enregistrer avant de rejouer
2. **Une vraie divergence garde le motif ordinaire**
   **Given** un rejeu en session neuve dont une lecture voit un nombre d'objets différent ou une couche sans équivalent
   **When** le rejeu diverge
   **Then** le message est le motif ordinaire, sans le remède de la réouverture
3. **La recette rejoue après réouverture**
   **Given** la sonde de recette
   **When** les phrases sont jouées
   **Then** elle rejoue d'abord dans la session (verdict indicatif imprimé), puis rouvre la copie du projet sans l'enregistrer et rejoue ; son code de sortie suit ce second rejeu
4. **Tests cover the condition found and fixed**
   **Given** les tests du noyau
   **When** ils tournent
   **Then** un test échoue sur le défaut et passe sur le correctif (motif nommé en même session), et les tests de rejeu existants (inventaire après couches créées, nombre d'objets différent, couche sans équivalent) restent verts
5. **Or: no change is needed, with proof**
   **Given** la reproduction
   **When** elle est rejouée sur le code courant
   **Then** le motif dit déjà la cause, preuve consignée dans Notes — ceci remplace 1 à 4

## Boundaries

- Ni « ne pas comparer » ni « ne pas journaliser » les lectures : AD-11 et le précédent du 2026-11-09 (« comparer moins que le résultat » interdit).
- Aucun paramètre ni code d'erreur nouveau au catalogue ; protocole inchangé.

## References

- parent — `_bmad-output/initiative-cassini-mvp/epic-registre-projet-import/epic-registre-projet-import.md`, R5
- architecture — `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-11
- constat — `_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-recette-qgis-d-annecy-portee-a-20-phrases-barre-16-20-plan.md`
- précédent — `_bmad-output/initiative-cassini-mvp/plan-bug-rejeu-mise-en-forme-et-couches-creees.md`

## Notes

- Decision (2026-11-10, tranchée en party, à confirmer par le propriétaire) : rejeu identique depuis l'état d'ouverture ; en même session, la divergence d'une lecture est honnête et doit être expliquée. Option laissée au propriétaire : rendre la lecture identique en même session en passant `leave_out` à `project.inventory` (couches et étendue sans celles d'une exécution précédente) — paramètre nouveau, donc changement de cap.
- Contrôle hors du ticket (risque high) : recette QGIS d'Annecy réelle avec un noyau construit depuis la branche, puis relecture `thorough`.
