---
type: epic
title: "Agent en langage naturel et serveur MCP"
parent: initiative-cassini-mvp
covers: [CAP-4, CAP-8]
after: []
assignee: ""
risk: high
---

# Agent en langage naturel et serveur MCP

## Description

La couche qui fait la différence du produit : une phrase en français devient une suite d'appels de commandes, affichée avant exécution, validée par l'utilisateur, rejouable. Et le même registre est exposé par un serveur MCP pour une IA externe, au choix de l'utilisateur.

## Outcome

Un utilisateur qui sait ce qu'il veut mais pas comment le faire dans un SIG obtient quand même le résultat, et comprend ce que la machine a fait ; le signal est la phrase de démo transformée en plan validé puis exécuté.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities.

## Done when

1. CAP-4 : une demande en français produit un plan d'appels de commandes affiché étape par étape **dans le panneau Cassini de QGIS**, que l'utilisateur valide avant exécution et rejoue ensuite à l'identique.
2. L'IA ne calcule rien elle-même : elle choisit des commandes et leurs paramètres, le moteur fait le calcul. Vérifié en rejouant un plan sans l'IA et en obtenant le même résultat.
3. Le modèle local tourne sans aucun accès réseau, modèle embarqué compris.
4. CAP-8 : un client MCP tiers découvre les commandes et en exécute une ; sans IA externe branchée, l'application garde toutes ses fonctions.
5. Quand l'agent ne sait pas relier un mot de l'utilisateur à une couche ou un champ, il pose une question au lieu de deviner.

## Boundaries

Frontière par capacité : la couche IA et le pont MCP, dans le noyau. Aucune logique métier n'est ajoutée ici : l'agent ne fait que composer des commandes existantes du registre. Le panneau où le plan s'affiche appartient à l'epic 9 ; cet epic le remplit, il ne le dessine pas.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — `CLAUDE.md` §2.6 indépendance des fournisseurs d'IA, §2.3 hors ligne total

## Notes

- Unknown: le modèle GGUF pèse plusieurs Go — installeur ou téléchargement au premier lancement ? La seconde option casse le hors ligne sur poste isolé. Question ouverte de la spec.
- Unknown: quel modèle exactement, quelle quantification, et quelle configuration minimale de RAM et de CPU. Question ouverte de la spec.
- Assumption: l'agent reçoit le catalogue des commandes et un inventaire du projet, jamais les données elles-mêmes ; c'est l'epic 8 qui valide ou casse cette hypothèse.
- Decision: l'epic 8 passe avant celui-ci et lui livre le modèle, la quantification et la forme de sollicitation retenus ; les deux questions ouvertes ci-dessus y seront tranchées (2026-10-14).
- Waits on epic 8 because: le modèle et la forme de sollicitation sortent de l'épreuve.
- Waits on epic 9 because: l'agent a besoin d'un panneau où afficher son plan et le faire valider.
- Waits on epic 2 because: l'agent ne compose que des commandes du registre.
