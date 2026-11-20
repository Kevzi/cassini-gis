---
type: epic
title: "L'anglais mesuré sur le modèle actuel"
parent: initiative-international
covers: [INT-1]
after: []
assignee: ""
risk: low
---

# L'anglais mesuré sur le modèle actuel

## Description

Avant de ré-entraîner quoi que ce soit, on veut un chiffre : que fait le modèle actuel d'une demande en anglais ? L'épic écrit un jeu de phrases anglaises équivalent au jeu français `par_commande`, le fait passer au modèle installé par le même chemin que le français, et conclut par un verdict écrit qui dit quelle suite est justifiée. Aucun ré-entraînement, aucune modification du modèle ni du noyau.

## Outcome

Le propriétaire connaît, commande par commande, l'écart entre l'anglais et le français sur le modèle actuel, et sait laquelle des suites (ré-entraînement bilingue, autre modèle de base, rien) se justifie.

## Requirements

- E1 (INT-1) : un jeu de phrases anglaises, une jumelle par phrase française de `par_commande`, avec les mêmes plans attendus.
- E2 (INT-1) : une mesure du taux de plans corrects en anglais et en français sur le modèle installé, par le même code et la même barre.
- E3 (INT-1) : un verdict écrit, chiffré, avec une recommandation pour l'épic 2.

## Done when

1. Chaque phrase de `par_commande/phrases.json` a une jumelle anglaise dont le plan attendu est identique (mêmes commandes, mêmes paramètres, mêmes valeurs).
2. Le test de validité des plans attendus passe sur le jeu anglais sans modèle, comme sur le français.
3. Un rapport donne, pour l'anglais et le français, le taux de plans corrects par commande avec le modèle installé.
4. Le verdict écrit nomme l'écart global et recommande la suite.

## Boundaries

Cet épic suit la frontière de la mesure : il ne modifie ni le modèle, ni le noyau, ni la grammaire, ni le prompt. Il n'est pas un ré-entraînement (épic 2). Le jeu anglais vit à côté du jeu français et n'entre pas dans les données d'entraînement (règle de recouvrement `training/tests/recouvrement`).

## References

- parent — `_bmad-output/initiative-international/initiative-international.md`, Requirements INT-1
- input — `ai/tests/par_commande/phrases.json`, `inventory.json`, `ai/tests/par_commande_reel.rs` (le test jumeau, garde `CASSINI_TEST_MODEL`)
- constraint — `docs/regle-de-redaction-des-commandes.md`, règle 8
- input — `_bmad-output/initiative-cassini-mvp/epic-epreuve-du-modele-local/` (harnais et verdict de l'épic 8, modèle de verdict)
- input — `docs/entrainement-du-modele.md` (barres de mesure et recouvrement)

## Notes

- Assumption: le jeu anglais réutilise le même `inventory.json` (noms de couches et de champs français, tels que l'utilisateur les aurait) ; un utilisateur anglophone aurait aussi des noms anglais, ce que le jeu ne couvre pas dans cet épic. À confirmer au verdict.
- Decision: 2026-11-20 — pas de ticket « Refactor sweep » : l'épic n'a que trois tickets et ne touche pas au code de production.
