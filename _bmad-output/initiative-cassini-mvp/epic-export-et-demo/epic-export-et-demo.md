---
type: epic
title: "Export PDF et scenario de demo"
parent: initiative-cassini-mvp
covers: [CAP-5]
after: []
assignee: ""
risk: medium
---

# Export PDF et scenario de demo

## Description

La sortie présentable à un tiers, et le scénario qui prouve que l'ensemble tient : la phrase de démo exécutée de bout en bout sur une machine débranchée.

## Outcome

L'utilisateur remet à son client une carte propre produite par Cassini ; le signal est le signal de succès de la spec, la démo complète hors ligne puis rejouée.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities.

## Done when

1. CAP-5 : le PDF porte titre, légende, échelle, flèche du nord et sources, et s'ouvre dans un lecteur standard.
2. La phrase de démo de CLAUDE.md §1 passe de bout en bout sur des données apportées, sur une machine sans réseau.
3. La même séquence se rejoue à l'identique depuis l'historique après fermeture et réouverture du projet.
4. Le scénario de démo est une suite de tests exécutables, pas une marche à suivre manuelle.

## Boundaries

Frontière par capacité : la mise en page d'export et l'assemblage du scénario. Pas de nouvelle analyse : tout ce que la démo utilise existe déjà.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — `CLAUDE.md` §1 phrase de démo, §6.6 export

## Notes

- Unknown: la définition de « parcelle constructible » en termes de champs du Géoportail de l'Urbanisme, dont la phrase de démo dépend. Question ouverte de la spec.
- Unknown: avec quelles données la démo tourne, l'application n'en distribuant aucune. Question ouverte de la spec.
