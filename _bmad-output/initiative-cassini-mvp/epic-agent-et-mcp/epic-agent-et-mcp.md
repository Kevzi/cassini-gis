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

La source est la spec de l'initiative, `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-4 et CAP-8,
citées telles quelles par les entrées de `tickets.toml` (inception du 2026-10-17).

## Done when

1. CAP-4 : une demande en français produit un plan d'appels de commandes affiché étape par étape **dans le panneau Cassini de QGIS**, que l'utilisateur valide avant exécution et rejoue ensuite à l'identique.
2. L'IA ne calcule rien elle-même : elle choisit des commandes et leurs paramètres, le moteur fait le calcul. Vérifié en rejouant un plan sans l'IA et en obtenant le même résultat.
3. Le modèle local tourne sans aucun accès réseau, modèle embarqué compris.
4. CAP-8 : un client MCP tiers découvre les commandes et en exécute une ; sans IA externe branchée, l'application garde toutes ses fonctions.
5. Quand l'agent ne sait pas relier un mot de l'utilisateur à une couche ou un champ, il pose une question au lieu de deviner.
6. Sur un poste sans carte graphique (le Ryzen 7 5800X du propriétaire bridé à 4 cœurs), une longue phrase rend son plan en moins de 30 s dès la deuxième demande.

## Boundaries

Frontière par capacité : la couche IA et le pont MCP, dans le noyau. Aucune logique métier n'est ajoutée ici : l'agent ne fait que composer des commandes existantes du registre. Le panneau où le plan s'affiche appartient à l'epic 9 ; cet epic le remplit, il ne le dessine pas.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — `CLAUDE.md` §2.6 indépendance des fournisseurs d'IA, §2.3 hors ligne total

## Notes

- Unknown: le modèle GGUF pèse plusieurs Go — installeur ou téléchargement au premier lancement ? La seconde option casse le hors ligne sur poste isolé. Question ouverte de la spec. Tranché : livré dans le paquet Cassini, jamais téléchargé (en-tête de `tickets.toml`, AD-17).
- Unknown: quel modèle exactement, quelle quantification, et quelle configuration minimale de RAM et de CPU. Question ouverte de la spec. Tranché pour le modèle : Qwen3.5-9B Q4_K_M (épreuve 8.6) ; la configuration minimale reste à mesurer (entrées 11, 12 et 10).
- Assumption: l'agent reçoit le catalogue des commandes et un inventaire du projet, jamais les données elles-mêmes ; c'est l'epic 8 qui valide ou casse cette hypothèse.
- Decision: l'epic 8 passe avant celui-ci et lui livre le modèle, la quantification et la forme de sollicitation retenus ; les deux questions ouvertes ci-dessus y seront tranchées (2026-10-14).
- Waits on epic 8 because: le modèle et la forme de sollicitation sortent de l'épreuve.
- Waits on epic 9 because: l'agent a besoin d'un panneau où afficher son plan et le faire valider.
- Waits on epic 2 because: l'agent ne compose que des commandes du registre.
- Decision: exigence du propriétaire, une longue phrase est résolue en moins de 30 s sur un poste sans carte graphique ; machine de mesure, son Ryzen 7 5800X bridé à 4 cœurs. Entrées 11 (mesure de référence) et 12 (modèle chargé une fois, préfixe du prompt en cache) ajoutées avant le panneau, et l'entrée 10 attend la mesure (2026-10-17).
- Decision: la répétition de démo sur le projet réel Villeneuve-le-Roi appartient à l'epic 6, qui en reçoit le scénario dans ses Notes, et non à cet epic (2026-10-17).
- Decision: l'entrée 10 (modèle ajusté) reste hors du ménage final (8) : longue et faite à la main, elle peut finir après la suite de bout en bout ; elle reste nécessaire même si l'entrée 12 tient les 30 s, pour les postes plus modestes et le seuil strict de clarification (2026-10-17).
- Decision: phrase E du scénario, isochrone d'origine inventée par le modèle de 5.15 : l'entrée 16 (garde-fou du noyau sur la provenance des coordonnées) la règle à coup sûr, puis l'entrée 17 (nouvelle passe sur le générateur et ré-entraînement) apprend au modèle à poser la question. AD-23 est étendu aux coordonnées par bmad-architecture avant le build de 16 ; 15 reste sur sa branche jusqu'à 16 et la recette du propriétaire dans QGIS. Comme l'entrée 10, les entrées 15 et 17 (entraînement long, fait à la main) restent hors du ménage final (8) ; 16 y entre (2026-10-23).
