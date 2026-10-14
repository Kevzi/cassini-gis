---
type: epic
title: "Scenario de demo de bout en bout"
parent: initiative-cassini-mvp
covers: []
after: []
assignee: ""
risk: medium
---

# Scénario de démo de bout en bout

## Description

Le scénario qui prouve que l'ensemble tient : la phrase de démo tapée dans le
panneau Cassini de QGIS, sur une machine débranchée, exécutée jusqu'au PDF, puis
rejouée à l'identique depuis le journal. Rien de neuf n'est construit ici — tout
ce que la démo utilise existe déjà dans les epics 9, 5 et 4. Cet epic assemble et
transforme la démonstration en suite de tests.

## Outcome

Le propriétaire peut faire la démonstration devant quelqu'un sans crainte, et la
refaire à l'identique ; le signal est le signal de succès de la spec, la démo
complète hors ligne puis rejouée après fermeture et réouverture du projet.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Success signal.

## Done when

1. La phrase de démo de `CLAUDE.md` §1 passe de bout en bout dans QGIS, panneau
   Cassini ouvert, sur des données apportées, machine sans réseau.
2. La même séquence se rejoue à l'identique depuis le journal après fermeture et
   réouverture du projet.
3. Le PDF obtenu porte titre, légende, échelle, flèche du nord et sources.
4. Une démonstration sur poste isolé passe aussi par au moins une commande de
   terrain du noyau, pour montrer ce que l'extension seule ne fait pas.
5. Le scénario est une suite de tests exécutables, pas une marche à suivre
   manuelle.

## Boundaries

Frontière par activité : l'assemblage et la mise sous test du scénario. Aucune
nouvelle commande, aucune nouvelle analyse, aucun nouveau morceau d'interface. Ce
qui manquerait ici est une entrée de l'epic 9, 5 ou 4, pas de celui-ci.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Success signal
- constraint — `CLAUDE.md` §1 phrase de démo, §6 périmètre du MVP

## Notes

- Decision: l'export PDF n'appartient plus à cet epic ; il est exécuté par le moteur de mise en page de QGIS et appartient à l'epic 9 (2026-10-14).
- Unknown: la définition de « parcelle constructible » en termes de champs du Géoportail de l'Urbanisme, dont la phrase de démo dépend. Question ouverte de la spec.
- Unknown: avec quelles données la démo tourne, le produit n'en distribuant aucune. Question ouverte de la spec.
- Waits on epic 9 because: la démo enchaîne des commandes exécutées par QGIS, export PDF compris.
- Waits on epic 5 because: c'est l'agent qui transforme la phrase en plan et le fait valider.
- Waits on epic 4 because: la démonstration sur poste isolé passe par une commande de terrain du noyau.
