---
type: epic
title: "Registre de commandes et journal rejouable"
parent: initiative-cassini-mvp
covers: [CAP-6, CAP-7]
after: []
assignee: ""
risk: high
---

# Registre de commandes et journal rejouable

## Description

L'épine dorsale du produit. Chaque fonction est décrite une seule fois — id,
descriptions en français et en anglais, schéma JSON de paramètres, résultat typé
— et appelable à l'identique par l'extension QGIS, la CLI, l'agent local et un
client MCP. Chaque exécution est journalisée à côté du projet QGIS, de façon à
pouvoir être rejouée. Trois epics consomment ce contrat : la façade, l'agent et
le terrain.

## Outcome

Une fonction ajoutée au registre devient immédiatement disponible aux quatre
points d'entrée sans y écrire une ligne ; le signal est qu'un même appel donne le
même résultat depuis la CLI et depuis le panneau QGIS, et que le rejouer depuis
le journal le redonne encore.

## Requirements

Source : la spec de l'initiative, `../spec-cassini-mvp/spec-cassini-mvp.md`,
section Capabilities, CAP-6 et CAP-7.

- R1 : une commande du registre porte un `id`, des descriptions courtes en
  français et en anglais, des paramètres à schéma JSON engendré depuis les types
  Rust, un résultat typé, un exécutant déclaré (`runs_in` : `core` ou `qgis`),
  et au moins un test. (CAP-6 ; AD-6)
- R2 : ajouter une commande ne modifie aucun fichier partagé. (CAP-6 ; Done when 2)
- R3 : un même appel de commande donne le même résultat depuis la CLI et depuis
  le canal local, le noyau tournant sans QGIS. (CAP-6 ; AD-3, AD-7, AD-13)
- R4 : toute exécution est journalisée, en ajout seul et par un seul auteur, dans
  un fichier texte lisible posé à côté du projet, sans chemin absolu. (CAP-7 ;
  AD-9, AD-10, AD-11)
- R5 : rejouer le journal sur les mêmes fichiers redonne des résultats
  identiques, y compris après fermeture et réouverture. (CAP-7 ; AD-11)

## Done when

1. CAP-6 : chaque commande porte id, descriptions fr et en, schéma JSON de
   paramètres, résultat typé, **soit une implémentation Rust soit une délégation
   déclarée à QGIS**, et au moins un test.
2. **Ajouter une commande ne modifie aucun fichier partagé**, pour que les
   epics 9, 5 et 4 en ajoutent en parallèle sans se marcher dessus.
3. Un même appel de commande donne le même résultat depuis la CLI et depuis le
   canal local, sans QGIS installé pour la CLI.
4. CAP-7 : rejouer le journal d'un projet sur les mêmes fichiers redonne des
   résultats identiques, y compris après fermeture et réouverture, vérifié par
   comparaison.
5. Le journal reste lisible par un humain, dans un fichier posé à côté du projet
   QGIS et non dans un format opaque.

## Boundaries

Frontière par unité livrée : le contrat du registre, son exécution côté noyau, et
le journal. Pas d'extension QGIS — c'est l'epic 9, qui consomme ce contrat. Pas
d'agent — c'est l'epic 5. Pas de commande de terrain — c'est l'epic 4. Pas de
format de projet Cassini : le projet est un projet QGIS, le noyau n'en invente
pas.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-6 et CAP-7
- change — `../change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md`, section 2.3 : pourquoi le format de projet disparaît et le journal reste
- constraint — `CLAUDE.md` §2.4 le noyau tourne sans QGIS, §2.7 agent-first, §4 registre de commandes
- spine — `../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-6, AD-8 à AD-11, AD-13, AD-21, AD-22 et Consistency Conventions

## Notes

- Cet epic possède la décision que les epics 9, 5 et 4 attendent : le contrat du registre de commandes.
- Decision: le format de projet Cassini est abandonné au MVP ; le projet est un projet QGIS et le noyau écrit son journal à côté (2026-10-14).
- Decision: une commande déclare son exécutant — Rust dans le noyau, ou délégation à QGIS — jamais implicitement et jamais les deux (2026-10-14).
- Decision: la forme des descriptions et des schémas suit le verdict de l'epic 8, signé le 2026-10-15 (oui sous conditions) : `epic-epreuve-du-modele-local/verdict-epreuve-du-modele-local.md`, section « Ce que l'epic 2 en retient » (2026-10-15).
- Decision: cet epic livre le contrat et sa mécanique, pas le catalogue : les vraies commandes arrivent avec l'epic 9, pour que les epics 9, 5 et 4 en ajoutent en parallèle (2026-10-15).
- Decision: la commande du traceur est `system.about`, une commande Rust de diagnostic (version du noyau et du protocole), inventée par cet epic et conservée ensuite pour `session.hello` (propriétaire, 2026-10-15).
- Decision: le gate sur l'epic 1 est ramené à ses entrées 1.1 (workspace, verrou hors ligne) et 1.6 (recette) : l'epic 2 n'attend pas la passe de nettoyage 1.7 (propriétaire, 2026-10-15).
- Decision: une passe de nettoyage et une suite de bout en bout ferment l'epic (propriétaire, 2026-10-15).
- Decision: à l'inception, le propriétaire a délégué les choix ouverts (« fait le meilleur choix »). Retenus le 2026-10-15 : `--project` sur la CLI, porté par l'entrée 5 ; `serve` journalise par la même boucle que `run` (entrée 7 après 5) ; un module de commandes de test, propriétaire unique l'entrée 5 ; le type du plan et la référence entre étapes, propriétaire l'entrée 4 ; l'ordre du noyau vers l'extension et l'annulation (AD-3, AD-4, AD-12) livrés ici, entrée 11, pour que l'epic 9 ne les invente pas ; la lecture du registre (descriptions, schémas, valeurs d'exemple) dans la réponse à `session.hello`. AD-8 est contrôlé par l'entrée 3 ; AD-10 par l'entrée 5.
- Unknown: le mécanisme d'enregistrement sans fichier partagé (entrée 2) reste à confirmer par le propriétaire avant cette story : le `unknown` de l'entrée donne l'orientation proposée.
- Decision: QGIS possède les systèmes de coordonnées du vecteur et exécute la reprojection ; le noyau ne reprojette pas et toute coordonnée traverse le canal avec son code EPSG. MGRS est hors MVP. Contrat d'architecture, AD-19, et non-objectif de la spec (2026-10-14).
- Waits on epic 1 because: le registre a besoin du workspace, du verrou hors ligne dans le moteur et de la recette.
- Waits on epic 8 because: la forme des descriptions de commandes se rédige d'après ce que l'épreuve du modèle local aura retenu.
