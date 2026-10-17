---
type: initiative
title: "L'interface en langage naturel d'un SIG, hors ligne"
parent: none
covers: [CAP-1, CAP-3, CAP-4, CAP-5, CAP-6, CAP-7, CAP-8, CAP-9, CAP-10, CAP-11, CAP-13]
after: []
assignee: ""
risk: high
---

# L'interface en langage naturel d'un SIG, hors ligne

## Description

Cassini permet à un bureau d'études, une collectivité ou un étudiant de faire son
travail cartographique courant en **formulant sa demande en français** plutôt
qu'en naviguant dans des menus, sur une machine qui ne téléphone jamais à
l'extérieur. Au MVP, Cassini est une **extension QGIS posée sur un noyau
indépendant** : QGIS exécute la carte, l'ouverture des fichiers, le projet, les
analyses vectorielles et l'export PDF ; le noyau porte le registre de commandes,
l'agent, le journal rejouable et un moteur de terrain en licence permissive. La
spec porte les capacités, les contraintes et les non-objectifs ; cette initiative
les livre.

## Outcome

Un utilisateur qui sait ce qu'il veut mais pas comment le faire dans un SIG
obtient le résultat quand même ; le signal est celui de la spec, la phrase de
démo exécutée de bout en bout dans QGIS sur une machine débranchée du réseau,
puis rejouée à l'identique depuis le journal.

## Requirements

Le contrat est la spec : `spec-cassini-mvp/spec-cassini-mvp.md`, sections
Capabilities (CAP-1, CAP-3 à CAP-11, CAP-13), Constraints et Non-goals. Les
epics citent ses identifiants dans leur `covers`.

## Done when

1. CAP-1, CAP-3 à CAP-11 et CAP-13 fonctionnent depuis QGIS, sur une machine sans
   accès réseau, et non derrière un drapeau de fonctionnalité.
2. La phrase de démo de CLAUDE.md §1 passe de bout en bout sur des données
   apportées par l'utilisateur, et se rejoue à l'identique depuis le journal
   après fermeture et réouverture du projet.
3. Aucune dépendance GPL ou LGPL dans l'arbre de dépendances **du noyau**,
   liaison statique comprise, et la licence de chaque dépendance est tracée.
   L'extension, elle, est en GPL par construction.
4. **Le noyau se lance, s'exécute et passe ses tests sans QGIS installé.**
5. Aucune requête réseau n'est émise en usage normal, ni par le noyau ni par
   l'extension, vérifié autrement que par lecture de code.
6. L'extension s'installe depuis le gestionnaire d'extensions de QGIS, et le
   noyau s'empaquette sur Windows, macOS et Linux.
7. Le dépôt public ne contient aucune fonction spécialement conçue pour l'usage
   militaire.

## Boundaries

Frontière par capacité : un seul dépôt, un seul propriétaire, des epics découpés
sur les capacités de la spec. Hors périmètre : tout ce que listent les Non-goals
de la spec, et en particulier l'application de bureau autonome (reportée en V2,
son code conservé sous `/app`) et toute fonction spécialement conçue pour l'usage
militaire, qui vit dans un dépôt privé distinct.

Chemin traceur à travers les epics : un fichier GeoPackage ouvert dans un projet
QGIS, filtré par une commande du registre lancée depuis le panneau Cassini, et
sorti en PDF — il traverse les epics 2, 9 et 5.

- Touch point : QGIS — plateforme hôte, consommée par son API PyQGIS et son cadre
  de traitement, jamais modifiée ; owner: epic-facade-qgis
- Touch point : dépôt d'extensions QGIS — canal de distribution principal ;
  owner: epic-publication
- Touch point : llama.cpp — modèle local embarqué et servi, consommé sans
  modification ; owner: epic-agent-et-mcp
- Touch point : `whitebox_next_gen` — moteur d'analyse de terrain en Rust, repris
  ou réimplémenté à partir de son code MIT OR Apache-2.0 ; owner: epic-raster-et-terrain
- Touch point : GitHub Actions — recette automatique sur chaque poussée, Rust,
  TypeScript et Python de l'extension ; owner: epic-socle-et-carte
- Touch point : dépôt privé du module défense — rien n'y est écrit depuis ici, et
  il s'appuiera sur le noyau et non sur l'extension ; owner: aucun epic d'ici

## References

- spec — `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — la même spec, section Constraints : hors ligne total, pas de GPL ni LGPL dans le noyau, frontière GPL de l'extension, séparation civil/défense, agent-first
- change — `_bmad-output/initiative-cassini-mvp/change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md` : le virage du 2026-10-14 et son analyse d'impact
- source de vérité — `CLAUDE.md`, §2 principes, §3 architecture, §6 périmètre, §7 étapes, §8 règles de travail, §9 conventions
- research — dossier de recherche initial, hors de ce dépôt (dépôt privé `cassini-defense-prive`, `recherche/research-sig-francais-hors-ligne-pour-la-defense/`), sections Recommandations et D4

## Notes

- Decision: au MVP, QGIS porte tout l'habillage (carte, ouverture des fichiers, projet, analyses vectorielles, export PDF) et le noyau ne porte que le registre, l'agent, le journal et le terrain. L'application autonome est reportée en V2 (propriétaire, 2026-10-14).
- Decision: le concurrent est QGIS, pas ArcGIS Pro ; le fossé est l'interface en langage naturel hors ligne, pas la géométrie ni la visibilité (2026-10-14).
- Decision: l'épreuve du modèle local (epic 8) passe en premier, avant le registre, parce que la promesse unique du produit était planifiée en dernier (2026-10-14).
- Decision: l'extension est en GPL, QGIS l'étant ; le noyau reste Apache-2.0, ne lie jamais QGIS et communique à distance de bras par processus séparé. Le module défense s'appuiera sur le noyau (2026-10-14).
- Decision: CAP-11 est réduite à trois commandes de terrain au MVP ; les six autres sont reportées (2026-10-14).
- Decision: l'epic 3, analyses et historique, est retiré ; CAP-3 part à l'epic 9 et CAP-7 à l'epic 2. L'id 3 n'est jamais réattribué (2026-10-14).
- Decision: le découpage initial en sept epics alignés sur les sept étapes de CLAUDE.md §7 est abandonné au profit de la séquence du virage (2026-10-14, remplace la décision du 2026-10-09).
- Decision: DuckDB et son extension `spatial` sont écartés au profit d'une chaîne Rust, GEOS étant en LGPL-2.1 (2026-10-09).
- Decision: le dépôt public reste un noyau civil ; aucune fonction conçue pour l'usage militaire n'y entre (2026-10-09).
- Decision: tout code et tout test passent par les skills BMad — `bmad-build` par entrée, `bmad-code-review` ou `bmad-walkthrough` pour les revues. Rien n'est codé en direct hors d'un ticket (demande du propriétaire, 2026-10-09).
- Decision: sur GitHub, on ne pousse que `main` ; les branches d'entrée restent locales (propriétaire, 2026-10-14).
- Parked: CAP-12 n'est portée par aucun epic d'ici : elle appartient au module privé, hors de ce dépôt, au su du propriétaire.
- Open question: la forme du canal local entre l'extension et le noyau porte l'argument juridique de la frontière GPL. À régler par `bmad-architecture` avant l'epic 9.
- Unknown: la maturité de `whitebox_next_gen` comme bibliothèque réutilisable ; l'epic 4 l'attend et doit la trancher avant de s'engager.
- Unknown: le chemin de livraison du modèle GGUF sur un poste isolé, le dépôt d'extensions QGIS n'acceptant pas plusieurs Go. Les epics 5 et 7 l'attendent.
- Decision: ajout de l'epic 10, catalogue étendu par les algorithmes de traitement de QGIS, après la démo (epic 6) : le propriétaire veut à terme un outil complet, sans parité revendiquée ; objectif d'au moins 70 % des demandes réelles couvertes (2026-10-17).
- Waits on epic 1 because: le registre a besoin du workspace, du verrou hors ligne et de la recette.
- Waits on epic 8 because: la forme des descriptions de commandes, puis l'agent lui-même, dépendent du verdict de l'épreuve.
- Waits on epic 2 because: toute commande ajoutée ensuite a besoin du contrat du registre.
- Waits on epic 9 because: l'agent et la démo ont besoin du panneau et des commandes exécutées par QGIS.
