---
type: epic
title: "Analyses vectorielles et historique rejouable"
parent: initiative-cassini-mvp
covers: [CAP-3, CAP-7]
after: []
assignee: ""
risk: medium
---

# Analyses vectorielles et historique rejouable

## Description

Les cinq analyses vectorielles du MVP, et la trace qui permet de refaire exactement le même travail : chaque exécution de commande est journalisée dans le projet et se rejoue.

## Outcome

L'utilisateur croise ses couches pour répondre à une question de terrain, et peut prouver comment il y est arrivé ; le signal est que rejouer l'historique sur les mêmes données redonne les mêmes résultats.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities.

## Done when

1. CAP-3 : zone tampon, intersection, sélection par attribut, statistiques par zone et isochrones s'exécutent sur un projet, chacune avec son test.
2. CAP-7 : rejouer l'historique d'un projet sur les mêmes données importées redonne des résultats identiques, vérifié par comparaison.
3. Le motif d'interface de lancement d'une commande et de saisie de ses paramètres est posé et réutilisable par l'epic 4.
4. L'historique reste lisible par un humain dans le dossier du projet.

## Boundaries

Frontière par capacité : les analyses sur géométries vectorielles, et l'historique. Pas de raster ni de terrain, c'est l'epic 4. Pas d'agent, c'est l'epic 5.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — `CLAUDE.md` §2.2 pas de GPL ni LGPL : la géométrie passe par la crate `geo`, jamais par GEOS

## Notes

- Cet epic possède le motif d'interface de lancement d'une commande, que l'epic 4 attend.
- Unknown: quel moteur de routage pour les isochrones, sous quelle licence, et quels attributs minimaux le réseau routier apporté doit porter — question ouverte de la spec.
