---
type: epic
title: "Refonte du panneau : le fil de la demande"
parent: initiative-cassini-mvp
covers: [CAP-13, CAP-4]
after: []
assignee: ""
risk: medium
---

# Refonte du panneau : le fil de la demande

## Description

Le propriétaire trouve le panneau Cassini « un peu serré et pas assez ergonomique » (2026-11-08). Sur des captures réelles à 340 px, le plan, le résultat et le journal tiennent dans des cadres de quatre ou cinq lignes qui défilent, alors que le bas du panneau reste vide. La demande s'efface dès l'envoi, et rien ne dit quoi faire ensuite. Le design « Fil de la demande » (commit f5811ce) refait la mise en page : la demande reste en bulle, les réponses de Cassini s'enchaînent en cartes, la saisie est fixée en bas, et une ligne « À vous » dit quand agir. Cet epic le construit dans l'extension seulement.

## Outcome

Un utilisateur occasionnel tape une phrase, voit sa demande rester affichée, comprend à chaque instant quoi faire (attendre, valider, reformuler), et lit plan et résultat sans faire défiler de petits cadres. Ça se vérifie sur captures du vrai panneau dans QGIS, comparées à la maquette.

## Requirements

La source est la spec UX `../ux-panneau-cassini/DESIGN.md` et `../ux-panneau-cassini/EXPERIENCE.md` (statut `final`, 2026-11-08), avec la maquette `../ux-panneau-cassini/mockups/panneau-fil.html`. CAP-13 (façade QGIS) pour le panneau, CAP-4 (agent en langage naturel) pour le parcours demande → plan → résultat.

## Done when

1. Les six états du fil (repos, composition, plan proposé, exécution, résultat, erreur d'étape) s'affichent dans le vrai panneau comme sur la maquette, en thème clair et en thème sombre de QGIS, à 340 px, sans cadre à hauteur fixe ni texte coupé.
2. La demande reste visible jusqu'au résultat ; la ligne « À vous », le repère « Étape k sur n en cours » et « Reformuler la demande » guident chaque étape, au clavier comme à la souris.
3. Le journal se rejoue depuis son repli, et le panneau dit si le résultat est identique.
4. Les tests de l'extension et la garde de l'extension passent ; aucune logique métier n'entre dans l'extension (AD-4).

## Boundaries

Extension QGIS seulement (`qgis-plugin/`), Qt par `qgis.PyQt`, QGIS 3.44 et 4.x. Le noyau, le générateur et le modèle ne changent pas ; le rejeu passe par l'appel du noyau qui existe déjà. Les points reportés du journal UX (mot « noyau », bulle collée en haut, paramètres dans « Détails techniques », export du journal, choix du modèle) restent hors périmètre.

## References

- design — `../ux-panneau-cassini/DESIGN.md`, `../ux-panneau-cassini/EXPERIENCE.md`, `../ux-panneau-cassini/mockups/panneau-fil.html`
- reference — `../ux-panneau-cassini/.memlog.md`, décisions du 2026-11-08
- constraint — `../architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md`, AD-4, AD-5, AD-11, AD-18, AD-21, AD-23, AD-27
- reference — `../plan-bug-etapes-du-plan-en-langage-courant.md` (lot 1, libellés courts déjà livrés, 20a52f4)
- tool — `%TEMP%\capture-panneau\capture.py` (captures du vrai panneau sous le Python de QGIS)

## Notes

- Decision: 2026-11-08 — propriétaire : refonte du panneau avant le fuseau UTM et le ré-entraînement 11.23 ; elle ne touche pas au modèle.
- Decision: 2026-11-08 — trois entrées : le fil et ses cartes, puis le guidage, puis le rejeu et le mode expert ; pas de ticket de nettoyage final (epic de trois entrées).
- Unknown: les couleurs du thème sombre de la maquette sont approchées ; elles sont à relever sur un vrai QGIS « Night Mapping » pendant 12.1.
