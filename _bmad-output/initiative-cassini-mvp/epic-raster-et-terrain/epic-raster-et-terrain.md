---
type: epic
title: "Analyse de terrain dans le noyau"
parent: initiative-cassini-mvp
covers: [CAP-11]
after: []
assignee: ""
risk: high
---

# Analyse de terrain dans le noyau

## Description

Les trois commandes de relief du MVP — champ de vision depuis un point, ligne de
vue entre deux points, pente — implémentées **en Rust dans le noyau**, sans
aucune dépendance GPL ou LGPL. QGIS sait déjà faire du champ de vision par GRASS,
mais GRASS est en GPL : un moteur de terrain en licence permissive dans le noyau
est la seule base sur laquelle le module défense privé pourra être construit.
C'est la raison d'être de cet epic, et la raison pour laquelle il reste dans le
noyau au lieu d'être délégué.

## Outcome

L'utilisateur interroge le relief de sa zone d'étude depuis le panneau Cassini,
et le calcul se fait dans un code dont la licence n'interdit rien ; le signal est
que chacune des trois commandes donne un résultat vérifiable par un test, hors
ligne, avec une provenance de code documentée.

## Requirements

À compléter à l'inception de cet epic. La source est la spec de l'initiative,
`../spec-cassini-mvp/spec-cassini-mvp.md`, section Capabilities, CAP-11.

## Done when

1. CAP-11 : champ de vision, ligne de vue entre deux points et pente répondent
   sur un modèle numérique d'élévation ouvert dans le projet, chacune comme
   commande du registre et chacune avec son test.
2. Aucune dépendance GPL ni LGPL n'est entrée dans le noyau avec la chaîne
   raster, et la provenance du code de visibilité est documentée commit par
   commit.
3. Les résultats sont reproductibles : même MNT, mêmes paramètres, même sortie,
   vérifié par comparaison.
4. Les trois commandes s'exécutent sans QGIS, depuis la CLI, sur un MNT donné en
   chemin de fichier.
5. Un résultat du noyau se compare à celui de QGIS par GRASS sur le même MNT, et
   l'écart est expliqué ou nul.

## Boundaries

Frontière par unité livrée : les trois commandes de terrain du noyau et la
lecture du MNT dont elles ont besoin. **L'ouverture du MNT comme couche du projet
et son affichage en ombrage appartiennent à QGIS**, donc à l'epic 9. Les six
autres commandes de terrain sont hors MVP. Aucune fonction de choix
d'fonction réservée : elle est hors de ce dépôt.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, CAP-11 et section Non-goals
- change — `../change-facade-qgis-avant-app-autonome/change-facade-qgis-avant-app-autonome.md`, section 2.4 : pourquoi trois commandes et pas neuf
- constraint — `CLAUDE.md` §2.2 pas de GPL ni LGPL dans le noyau, §2.9 séparation civil/défense
- research — `../research-sig-francais-hors-ligne-pour-la-defense/research-sig-francais-hors-ligne-pour-la-defense.md`, section D4

## Notes

- Decision: réduit de neuf commandes à trois au MVP (champ de vision, ligne de vue, pente). Motif : en mode extension QGIS fait déjà le reste, mais par GRASS qui est en GPL ; trois commandes suffisent à établir une base en licence permissive pour le module défense. Les six autres sont reportées, pas abandonnées (propriétaire, 2026-10-14).
- Decision: GRASS et SAGA restent exclus du noyau car sous GPL ; les algorithmes de visibilité sont repris du code MIT de Whitebox ou réimplémentés depuis la littérature publique (2026-10-09).
- Unknown: `whitebox_next_gen` expose-t-il son champ de vision comme bibliothèque réutilisable, et à quel niveau de maturité — version 0.1.0, projet conduit en open core. À trancher par un cas d'essai compilé avant de s'engager ; la recherche recommande un spike.
- Waits on epic 2 because: les trois commandes sont des commandes du registre et ont besoin de son contrat.
