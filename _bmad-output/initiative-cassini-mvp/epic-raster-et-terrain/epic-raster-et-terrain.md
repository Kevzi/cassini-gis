---
type: epic
title: "Chaine raster et analyse de terrain"
parent: initiative-cassini-mvp
covers: [CAP-11]
after: []
assignee: ""
risk: high
---

# Chaine raster et analyse de terrain

## Description

Le second pilier du moteur : importer un modèle numérique d'élévation, et répondre aux questions de relief — ce qui est visible, ce qui est à l'abri des vues, la pente, les points dominants, la franchissabilité.

## Outcome

L'utilisateur interroge le relief de sa zone d'étude sans quitter son projet ni sa machine ; le signal est que chacune des neuf commandes de terrain donne un résultat vérifiable par un test, hors ligne.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities.

## Done when

1. CAP-11 : les neuf commandes de terrain répondent sur un MNT importé, chacune avec son test.
2. Un MNT s'importe comme une couche du projet et s'affiche en ombrage sur la carte.
3. Aucune dépendance GPL ni LGPL n'est entrée avec la chaîne raster, et la provenance du code de visibilité est documentée commit par commit.
4. Les résultats de visibilité sont reproductibles : même MNT, mêmes paramètres, même sortie.

## Boundaries

Frontière par capacité : le raster, les MNT et les calculs de relief. **Cet epic possède l'import raster**, l'epic 2 ne portant que le vecteur. Aucune fonction de fonction réservée au module privé : elle est hors de ce dépôt.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities
- constraint — `CLAUDE.md` §2.7 séparation civil/défense : les commandes de terrain sont des fonctions standard de SIG, et rien de plus n'entre ici
- research — `../research-sig-francais-hors-ligne-pour-la-defense/research-sig-francais-hors-ligne-pour-la-defense.md`, section D4

## Notes

- Unknown: `whitebox_next_gen` expose-t-il son viewshed comme bibliothèque réutilisable, et à quel niveau de maturité — version 0.1.0, projet conduit en open core. À trancher par un cas d'essai compilé avant de s'engager ; la recherche recommande un spike.
- Decision: GRASS et SAGA sont exclus car sous GPL ; les algorithmes de visibilité sont repris du code MIT de Whitebox ou réimplémentés depuis la littérature publique (2026-10-09).
