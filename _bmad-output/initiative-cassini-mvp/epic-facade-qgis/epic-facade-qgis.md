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

Les exigences sont portées par la spec, une capacité à la fois ; les entrées citent les `CAP-N`. La source est la spec de l'initiative,
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
- spine — `../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-1 à AD-5, AD-7, AD-12, AD-14, AD-17, AD-18, AD-22 et la séquence de la phrase de démonstration

## Notes

- Decision: le canal local est l'entrée et la sortie standard d'un processus enfant lancé par l'extension, en JSON-RPC 2.0, un message par ligne, dans les deux sens ; aucune socket, aucun port, aucun fichier d'échange. Tranché par le contrat d'architecture, AD-2 et AD-3 (propriétaire, 2026-10-14).
- Decision: une seule extension pour QGIS 3.44 LTR à 4.99, tout accès à Qt passant par le shim `qgis.PyQt`. Contrat d'architecture, AD-18 (propriétaire, 2026-10-14).
- Unknown: la surface de l'API PyQGIS réellement nécessaire, à fixer geste par geste avec les commandes déléguées. Question ouverte de la spec.
- Unknown: le dessin du panneau. L'expérience devient un panneau ancré dans un outil d'expert, alors que l'argument de vente est qu'un opérateur non cartographe obtient sa réponse en parlant. À esquisser avant la première entrée, par `bmad-ux` comme le reporte le contrat d'architecture.
- Decision: l'extension est en GPL, QGIS l'étant ; ce n'est pas un choix mais une conséquence, et le module défense s'appuiera donc sur le noyau et non sur elle (2026-10-14).
- Open question: la confirmation juridique de la frontière GPL est à obtenir avant toute vente, pas avant d'écrire du code.
- Waits on epic 2 because: l'extension ne peut consommer le registre avant que son contrat existe.
- Decision: les ordres du noyau portent des chemins absolus, décidés par le noyau ; le journal garde ses chemins relatifs (2026-10-15). Exception volontaire à AD-9 pour le seul `order.execute`, car le noyau et l'extension sont sur la même machine et l'extension a besoin du chemin réel pour écrire.
- Source conflict: AD-9 — « aucun chemin absolu sur le canal » vs `core/src/serve.rs` et `docs/canal-local.md` (2.11), où `order.execute` porte des chemins absolus, et `session.hello` le chemin absolu du projet (AD-7). Tranché par la Decision ci-dessus ; AD-9 reste à amender dans le contrat d'architecture.
- Decision: le panneau est esquissé par `bmad-ux` avant la story 4 ; le traceur (story 1) garde un panneau minimal jetable (2026-10-15).
- Decision: découpage en 14 entrées sur une seule piste Python, au-dessus des 8 à 12 habituels ; un refactor sweep et une suite de bout en bout ferment l'épic (2026-10-15).
