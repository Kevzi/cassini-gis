---
type: epic
title: "Facade QGIS : extension, canal local, commandes executees par QGIS"
parent: initiative-cassini-mvp
covers: [CAP-13, CAP-9, CAP-10, CAP-3, CAP-5]
after: []
assignee: ""
risk: high
---

# Façade QGIS : extension, canal local, commandes exécutées par QGIS

## Description

L'interface du MVP. Une extension QGIS en Python ouvre un panneau Cassini,
dialogue avec le noyau lancé comme **processus séparé**, et fait exécuter par
QGIS tout ce qui touche aux données : ouvrir les fichiers de l'utilisateur,
inventorier le projet, lancer les analyses vectorielles, sortir le PDF. Cet epic
est ce qui remplace l'application autonome au MVP, et il porte la frontière
juridique entre le noyau Apache-2.0 et l'extension GPL.

## Outcome

L'utilisateur pilote Cassini sans quitter QGIS, là où ses données sont déjà
ouvertes ; le signal est une commande du registre lancée depuis le panneau, qui
produit son résultat dans le projet, sur une machine sans réseau.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities, CAP-13, CAP-9,
CAP-10, CAP-3 et CAP-5.

## Done when

1. CAP-13 : une extension installable ouvre un panneau Cassini, joint le noyau
   lancé en processus séparé, et exécute une commande du registre de bout en bout.
2. **Le noyau se lance, s'exécute et passe ses tests sans QGIS installé** — c'est
   ce qui fonde l'argument de distance de bras sur la GPL, donc c'est un contrôle
   de recette et pas une intention.
3. CAP-9 et CAP-10 : une commande fait ouvrir par QGIS un fichier de chacun des
   cinq formats du MVP, et une autre dresse l'inventaire du projet ; le fichier
   d'origine est inchangé, vérifié par empreinte.
4. CAP-3 : zone tampon, intersection, sélection par attribut, statistiques par
   zone et isochrones s'exécutent par le cadre de traitement de QGIS, chacune
   appelée comme commande du registre et couverte par un test.
5. CAP-5 : le PDF produit par le moteur de mise en page de QGIS porte titre,
   légende, échelle, flèche du nord et sources.
6. La garde hors ligne couvre aussi le Python de l'extension : aucune adresse
   réseau n'y passe la recette.

## Boundaries

Frontière par unité livrée : l'extension, le canal local, et les commandes du
registre dont l'exécution est déléguée à QGIS. Le contrat du registre lui-même
appartient à l'epic 2. L'agent appartient à l'epic 5. Les commandes de terrain
sont exécutées par le noyau et appartiennent à l'epic 4. Aucune logique métier
n'est écrite dans le Python.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-13, CAP-9, CAP-10, CAP-3, CAP-5 et section Constraints
- change — `../change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md`, sections 3.2 et 3.3
- constraint — `CLAUDE.md` §2.3 l'extension est en GPL et vit séparée, §2.4 le noyau tourne sans QGIS, §2.7 agent-first

## Notes

- Unknown: la forme du canal local entre l'extension et le noyau — entrée et sortie standard, socket local, ou fichiers d'échange. Elle porte l'argument juridique du §2.3, donc elle se tranche avant la première entrée de cet epic. Question ouverte de la spec.
- Unknown: la version plancher de QGIS visée et la surface de l'API PyQGIS réellement nécessaire. Question ouverte de la spec.
- Unknown: le dessin du panneau. L'expérience devient un panneau ancré dans un outil d'expert, alors que l'argument de vente est qu'un opérateur non cartographe obtient sa réponse en parlant. À esquisser avant la première entrée.
- Decision: l'extension est en GPL, QGIS l'étant ; ce n'est pas un choix mais une conséquence, et le module défense s'appuiera donc sur le noyau et non sur elle (2026-10-14).
- Open question: la confirmation juridique de la frontière GPL est à obtenir avant toute vente, pas avant d'écrire du code.
- Waits on epic 2 because: l'extension ne peut consommer le registre avant que son contrat existe.
