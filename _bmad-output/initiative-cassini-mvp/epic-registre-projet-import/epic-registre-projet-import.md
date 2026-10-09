---
type: epic
title: "Registre de commandes, projet et import"
parent: initiative-cassini-mvp
covers: [CAP-6, CAP-10, CAP-9, CAP-1]
after: []
assignee: ""
risk: high
---

# Registre de commandes, projet et import

## Description

Le cœur du produit : chaque fonction décrite une seule fois dans un registre, un projet qui se crée, s'ouvre et s'enregistre en conservant son état, et l'import des fichiers vecteur que l'utilisateur possède déjà. Cet epic livre aussi la part de CAP-1 qui dépend du projet : afficher ses couches.

## Outcome

L'utilisateur ouvre un projet, y glisse ses fichiers, les voit sur la carte, et retrouve tout à l'identique en réouvrant ; le signal est qu'aucune de ces actions n'existe ailleurs que comme commande du registre.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities.

## Done when

1. CAP-6 : chaque commande porte id, descriptions fr et en, schéma JSON de paramètres, résultat typé, implémentation Rust et au moins un test.
2. **Ajouter une commande ne modifie aucun fichier partagé**, pour que deux epics puissent en ajouter en parallèle sans se marcher dessus.
3. CAP-9 : les cinq formats vecteur s'importent, se reprojettent si besoin, et le fichier d'origine est inchangé sur le disque, vérifié par empreinte.
4. CAP-10 : fermer puis réouvrir un projet, y compris déplacé sur un autre poste, restitue couches, ordre, symbologie et étendue à l'identique.
5. Un même appel de commande donne le même résultat depuis l'interface et depuis la CLI.

## Boundaries

Frontière par capacité : le registre, le projet, l'import vecteur et l'affichage des couches. **Le raster et les MNT appartiennent à l'epic 4.** Les analyses appartiennent à l'epic 3, l'agent à l'epic 5.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — `CLAUDE.md` §2.4 données apportées, §2.5 agent-first, §4 registre de commandes

## Notes

- Cet epic possède les deux décisions que les epics 3 à 6 attendent : le contrat du registre de commandes, et le format de projet sur disque.
- Decision: l'epic reste entier plutôt que coupé en registre-et-projet puis import-et-affichage, choix du propriétaire (2026-10-09).
- Unknown: quels systèmes de coordonnées au-delà de Lambert-93 et WGS84, et si MGRS entre au MVP — question ouverte de la spec.
