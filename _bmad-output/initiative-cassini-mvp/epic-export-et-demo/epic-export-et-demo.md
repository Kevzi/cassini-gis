---
type: epic
title: "Scenario de demo de bout en bout"
parent: initiative-cassini-mvp
covers: []
after: []
assignee: ""
risk: medium
---

# Scénario de démo de bout en bout

## Description

Le scénario qui prouve que l'ensemble tient : la phrase de démo tapée dans le
panneau Cassini de QGIS, sur une machine débranchée, exécutée jusqu'au PDF, puis
rejouée à l'identique depuis le journal. Rien de neuf n'est construit ici — tout
ce que la démo utilise existe déjà dans les epics 9, 5 et 4. Cet epic assemble et
transforme la démonstration en suite de tests.

## Outcome

Le propriétaire peut faire la démonstration devant quelqu'un sans crainte, et la
refaire à l'identique ; le signal est le signal de succès de la spec, la démo
complète hors ligne puis rejouée après fermeture et réouverture du projet.

## Requirements

Source : la spec de l'initiative, `../spec-cassini-mvp/spec-cassini-mvp.md`, section
Success signal, et `CLAUDE.md` §1 (phrase de démo).

- D1 : la phrase de démo de `CLAUDE.md` §1 passe de bout en bout dans QGIS, par le vrai
  modèle local et le vrai noyau, sans aucune sortie réseau. (Success signal)
- D2 : la séquence se rejoue à l'identique depuis le journal après fermeture et réouverture du
  projet. (Success signal)
- D3 : le PDF porte titre, légende, échelle, flèche du nord et sources. (CAP-10, vu dans la démo)
- D4 : la démonstration passe par au moins une commande de terrain du noyau. (Done when 4)
- D5 : le scénario est une suite exécutable qui rend un verdict, sur fixtures synthétiques par
  défaut et sur les données réelles de l'utilisateur à la demande. (Done when 5)

## Done when

1. La phrase de démo de `CLAUDE.md` §1 passe de bout en bout dans QGIS, panneau
   Cassini ouvert, sur des données apportées, machine sans réseau.
2. La même séquence se rejoue à l'identique depuis le journal après fermeture et
   réouverture du projet.
3. Le PDF obtenu porte titre, légende, échelle, flèche du nord et sources.
4. Une démonstration sur poste isolé passe aussi par au moins une commande de
   terrain du noyau, pour montrer ce que l'extension seule ne fait pas.
5. Le scénario est une suite de tests exécutables, pas une marche à suivre
   manuelle.

## Boundaries

Frontière par activité : l'assemblage et la mise sous test du scénario. Aucune
nouvelle commande, aucune nouvelle analyse, aucun nouveau morceau d'interface. Ce
qui manquerait ici est une entrée de l'epic 9, 5 ou 4, pas de celui-ci.

## References

- parent — `_bmad-output/initiative-cassini-mvp/initiative-cassini-mvp.md`, section Requirements
- spec — `../spec-cassini-mvp/spec-cassini-mvp.md`, section Success signal
- constraint — `CLAUDE.md` §1 phrase de démo, §6 périmètre du MVP

## Notes

- Decision: l'export PDF n'appartient plus à cet epic ; il est exécuté par le moteur de mise en page de QGIS et appartient à l'epic 9 (2026-10-14).
- Decision (2026-11-05, propriétaire) : « parcelle constructible » se lit comme une parcelle qui touche une zone de type U du PLU (inception de l'epic 6 ; tranche l'ancienne question ouverte de la spec).
- Decision (2026-11-05, propriétaire) : la démo tourne sur des fixtures synthétiques commitées par défaut, et sur `../donnees-test-cassini/projet-test-villeneuve.qgz` (hors dépôt) à la demande ; le produit ne distribue toujours aucune donnée.
- Decision (2026-11-05, propriétaire) : le MNT de démo est une copie en EPSG:2154 du MNT de test, reprojetée une fois dans QGIS (AD-19 : le noyau ne reprojette pas), rangée dans `../donnees-test-cassini/`, hors dépôt.
- Decision (2026-11-05, propriétaire) : la demande de terrain de la démo est la pente du MNT en degrés, ajoutée à la carte ; aucune coordonnée à fournir.
- Decision (2026-11-05, propriétaire) : la suite de démo est une sonde commitée, lancée sous le Python de QGIS sur le poste du propriétaire (par Claude), pas dans la CI, qui n'a ni QGIS ni modèle ; la CI garde ses tests actuels.
- Tracer bullet : entree 1 (la sonde juge la phrase de demo sur fixtures).
- Decision (2026-11-05) : les entrees 1 a 5 s'enchainent en file (1→2→3→4→5) parce qu'elles modifient toutes `qgis-plugin/tests/probe_demo.py` ; c'est une collision, pas un simple ordre.
- Decision (2026-11-05, proprietaire) : le hors-ligne se prouve deux fois — la sonde surveille les sockets du noyau (entree 5), et le proprietaire fait la demo machine debranchee (entree 6) ; les connexions propres a QGIS ne sont pas surveillees par la sonde.
- Decision (2026-11-05, proprietaire) : les phrases de repetition A a E font partie de l'entree 6 de cet epic.
- Waits on epic 11 because: changement de cap du 2026-11-05 — la géométrie et le terrain étendu passent avant la démo, et la démo part d'un catalogue des commandes figé ; elle peut montrer une commande de l'epic 11 en plus de la pente, sans obligation.
- Waits on epic 9 because: la démo enchaîne des commandes exécutées par QGIS, export PDF compris.
- Waits on epic 5 because: c'est l'agent qui transforme la phrase en plan et le fait valider.
- Waits on epic 4 because: la démonstration sur poste isolé passe par une commande de terrain du noyau.
- Decision: le scénario de répétition, à reprendre à l'inception de cet epic, tourne sur `../donnees-test-cassini/projet-test-villeneuve.qgz` (données réelles hors dépôt), en recette manuelle par le propriétaire après l'entrée 5.6 : A, la phrase de démo complète ; B1 → B2 → B3, la même demande en trois fois (« garde les zones U du PLU », « zone tampon de 500 m autour des gares », « croise les parcelles avec ces deux résultats et fais la carte ») ; C, bâtiments à moins de 50 m d'une route ; D, parcelles près des écoles (aucune couche : une question attendue) ; E, parcelles à 10 minutes à pied de la gare de Villeneuve-le-Roi (coordonnées inconnues du modèle : question ou refus attendu). Critère de durée : moins de 30 s par phrase (entrée 5.12) (2026-10-17).
