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

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities, CAP-6 et CAP-7.

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

## Notes

- Cet epic possède la décision que les epics 9, 5 et 4 attendent : le contrat du registre de commandes.
- Decision: le format de projet Cassini est abandonné au MVP ; le projet est un projet QGIS et le noyau écrit son journal à côté (2026-10-14).
- Decision: une commande déclare son exécutant — Rust dans le noyau, ou délégation à QGIS — jamais implicitement et jamais les deux (2026-10-14).
- Unknown: la forme des descriptions et des schémas de paramètres dépend du verdict de l'epic 8, qui mesure ce qu'un modèle local sait lire. À relire avant l'inception.
- Unknown: quels systèmes de coordonnées au-delà de Lambert-93 et WGS84, et si MGRS entre au MVP — question ouverte de la spec.
- Waits on epic 1 because: le registre a besoin du workspace, du verrou hors ligne dans le moteur et de la recette.
- Waits on epic 8 because: la forme des descriptions de commandes se rédige d'après ce que l'épreuve du modèle local aura retenu.
