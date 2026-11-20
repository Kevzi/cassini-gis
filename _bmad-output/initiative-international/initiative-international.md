---
type: initiative
title: "Cassini à l'international"
parent: none
covers: [INT-1, INT-2, INT-3, INT-4]
after: []
assignee: ""
risk: medium
---

# Cassini à l'international

## Description

Aujourd'hui Cassini se parle en français : le modèle local est entraîné et mesuré sur des phrases françaises, les messages du noyau sont en français, la documentation et la démo aussi. Cette initiative rend Cassini utilisable hors de France, **en anglais d'abord** : on commence par mesurer ce que le modèle actuel sait faire en anglais, puis on décide comment l'améliorer, puis on traduit ce qui reste. Le noyau n'a rien de propre à la France (WGS84, UTM et MGRS y sont déjà) ; ce qui l'est, c'est le modèle, les messages, les textes et la démo.

## Outcome

Un utilisateur anglophone, hors de France, formule sa demande en anglais, obtient un plan correct, le valide et l'exécute hors ligne ; le signal est le taux de plans corrects en anglais, mesuré avec la même barre et les mêmes commandes qu'en français.

## Requirements

- INT-1 : savoir ce que le modèle actuel donne en anglais, par une mesure comparable à celle du français.
- INT-2 : le modèle compose des plans corrects à partir de demandes en anglais, à moins de 5 points du français sur les mêmes commandes.
- INT-3 : les messages destinés à l'utilisateur, côté noyau, existent en anglais comme en français.
- INT-4 : la documentation, le site et la démo existent en anglais, la démo sur un territoire hors de France.

## Done when

1. Un rapport chiffré donne le taux de plans corrects en anglais et en français, par commande, avec le même modèle.
2. Sur la même barre, l'anglais est à moins de 5 points du français sur les commandes du catalogue.
3. Une erreur du noyau s'affiche dans la langue réglée dans le panneau.
4. Le README et la page d'accueil existent en anglais.
5. La démo filmée tourne sur un territoire hors de France, avec des données apportées par l'utilisateur.

## Boundaries

Cette initiative suit la frontière de la langue et du public, pas celle d'un module. L'anglais seulement : ni espagnol, ni allemand, ni autre langue pour l'instant. Comme en France, aucune donnée n'est téléchargée ni redistribuée : l'utilisateur apporte les siennes (principe 6 de CLAUDE.md). Elle ne porte aucune fonction pour un usage militaire. Chemin traceur : une phrase anglaise de `par_commande` arrive au modèle actuel et son plan est jugé avec les mêmes règles que sa jumelle française.

- Touch point: ré-entraînement du modèle ajusté — ticket 17 de l'épic catalogue étendu (initiative cassini-mvp), à grouper avec l'anglais ; owner: epic-cassini-comprend-l-anglais
- Touch point: panneau QGIS — le réglage de langue existe déjà ; owner: epic-cassini-comprend-l-anglais

## References

- constraint — CLAUDE.md, §1 (public français) et §2.6 (l'utilisateur apporte ses données) ; changement de cap ci-dessous
- constraint — `docs/regle-de-redaction-des-commandes.md`, règle 8 : « l'anglais n'est pas établi »
- constraint — `docs/modele-hugging-face/README.md` : les autres langues ne sont pas évaluées
- input — `ai/tests/par_commande/phrases.json` et `inventory.json`, le jeu français de référence

## Notes

- Decision: 2026-11-20 — le propriétaire veut s'attaquer à l'international, anglais d'abord. C'est un changement de cap par rapport à CLAUDE.md §1, qui décrit un public français (bureaux d'études, collectivités) et une démo à Annecy ; CLAUDE.md n'est pas modifié ici, le propriétaire le fera ou demandera de le faire.
- Decision: 2026-11-20 — dossier séparé de l'initiative cassini-mvp ; `active_initiative` reste cassini-mvp pour que `bmad build next` continue le catalogue étendu.
- Decision: 2026-11-20 — anglais seulement pour l'instant.
- Decision: 2026-11-20 — barre de réussite : l'anglais à moins de 5 points du français sur les mêmes commandes, pas une valeur fixe.
- Decision: 2026-11-20 — seul le premier épic est détaillé maintenant ; les deux autres attendent le verdict de la mesure.
- Unknown: si le modèle actuel comprend déjà bien l'anglais ou s'il faut un ré-entraînement bilingue ou un autre modèle de base ; l'épic 1 y répond, l'épic 2 en dépend.
- Waits on epic-cassini-comprend-l-anglais because: la démo hors de France et la documentation décrivent un produit qui comprend l'anglais.
