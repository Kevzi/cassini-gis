---
type: initiative
title: "Un SIG de bureau libre, hors ligne, piloté en langage naturel"
parent: none
covers: [CAP-1, CAP-3, CAP-4, CAP-5, CAP-6, CAP-7, CAP-8, CAP-9, CAP-10, CAP-11]
after: []
assignee: ""
risk: high
---

# Un SIG de bureau libre, hors ligne, piloté en langage naturel

## Description

Cassini permet à un bureau d'études, une collectivité ou un étudiant de faire son
travail cartographique courant dans un logiciel libre qui ne demande ni compte ni
connexion : il ouvre un projet, y importe ses propres fichiers, croise ses
couches, interroge le relief, et sort une carte en PDF — en formulant ses
demandes en français plutôt qu'en naviguant dans des menus. La spec porte les
capacités, les contraintes et les non-objectifs ; cette initiative les livre.

## Outcome

Un utilisateur qui sait ce qu'il veut mais pas comment le faire dans un SIG
obtient le résultat quand même ; le signal est celui de la spec, la phrase de
démo exécutée de bout en bout sur une machine débranchée du réseau, puis rejouée
à l'identique.

## Requirements

Le contrat est la spec : `spec-cassini-mvp/spec-cassini-mvp.md`, sections
Capabilities (CAP-1, CAP-3 à CAP-11), Constraints et Non-goals. Les epics citent
ses identifiants dans leur `covers`.

## Done when

1. CAP-1, CAP-3 à CAP-11 fonctionnent sur une application installée, sur une
   machine sans accès réseau, et non derrière un drapeau de fonctionnalité.
2. La phrase de démo de CLAUDE.md §1 passe de bout en bout sur des données
   apportées par l'utilisateur, et se rejoue à l'identique depuis l'historique
   après fermeture et réouverture du projet.
3. Aucune dépendance GPL ou LGPL dans l'arbre de dépendances livré, liaison
   statique comprise, et la licence de chaque dépendance est tracée.
4. Aucune requête réseau n'est émise par l'application en usage normal, vérifié
   autrement que par lecture de code.
5. L'application se compile et s'installe sur Windows, macOS et Linux.
6. Le dépôt public ne contient aucune fonction spécialement conçue pour l'usage
   militaire.

## Boundaries

Frontière par capacité : un seul dépôt, un seul propriétaire, des epics découpés
sur les capacités de la spec et alignés sur les sept étapes de CLAUDE.md §7, qui
sont la préférence de découpage du propriétaire. Hors périmètre : tout ce que
listent les Non-goals de la spec, et en particulier toute fonction spécialement
conçue pour l'usage militaire, qui vit dans un dépôt privé distinct.

Chemin traceur à travers les epics : un fichier GeoPackage importé dans un
projet, affiché sur un fond de carte hors ligne, filtré par une analyse, et sorti
en PDF — il traverse les epics 1, 2, 3 et 6.

- Touch point : llama.cpp — modèle local embarqué et servi, consommé sans
  modification ; owner: epic-agent-et-mcp
- Touch point : `whitebox_next_gen` — moteur d'analyse de terrain en Rust, repris
  ou réimplémenté à partir de son code MIT OR Apache-2.0 ; owner: epic-raster-et-terrain
- Touch point : GitHub Actions — recette automatique sur chaque poussée ;
  owner: epic-socle-et-carte
- Touch point : dépôt privé du module défense — rien n'y est écrit depuis ici ;
  owner: aucun epic de cette initiative

## References

- spec — `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — la même spec, section Constraints : hors ligne total, pas de GPL ni LGPL, séparation civil/défense, agent-first
- source de vérité — `CLAUDE.md`, §2 principes, §3 architecture, §7 étapes, §8 règles de travail, §9 conventions
- research — `_bmad-output/initiative-cassini-mvp/research-sig-francais-hors-ligne-pour-la-defense/research-sig-francais-hors-ligne-pour-la-defense.md`, sections Recommandations et D4

## Notes

- Decision: le découpage en epics suit les sept étapes de CLAUDE.md §7, choix du propriétaire du projet (2026-10-09).
- Decision: DuckDB et son extension `spatial` sont écartés au profit d'une chaîne Rust, GEOS étant en LGPL-2.1 (2026-10-09).
- Decision: le dépôt public reste un noyau civil ; aucune fonction conçue pour l'usage militaire n'y entre (2026-10-09).
- Parked: CAP-12, le fonction réservée au module privé, n'est porté par aucun epic d'ici : il appartient au module défense privé et attend la réponse à une demande de classement ([retiré]), au su du propriétaire.
- Decision: le contrat du registre de commandes et le format de projet sur disque sont décidés dans l'epic 2, qui les possède ; les epics 3, 4, 5 et 6 l'attendent (2026-10-09).
- Unknown: d'où vient le fond de carte livré ou apporté, question ouverte de la spec ; l'epic 1 en a besoin et travaille d'ici là sur un fond généré pour la recette.
- Unknown: la maturité de `whitebox_next_gen` comme bibliothèque réutilisable ; l'epic 4 l'attend et doit la trancher avant de s'engager.
- Decision: tout code et tout test passent par les skills BMad — `bmad-build` par entrée, `bmad-code-review` ou `bmad-walkthrough` pour les revues. Rien n'est codé en direct hors d'un ticket (demande du propriétaire, 2026-10-09).
- Waits on epic 1 because: le registre et le projet ont besoin du workspace, de la coquille et de la recette.
- Waits on epic 2 because: toute commande ajoutée ensuite a besoin du contrat du registre et du format de projet.
