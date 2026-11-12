---
title: "Changement de cap — mise en évidence du résultat dans le PDF d'un plan"
date: 2026-11-12
status: approuvé
approved_by: party d'approbation (John, Winston, Amelia, Mary, Sally), par délégation du propriétaire, 2026-11-12 soir
---

# Changement de cap — mise en évidence du résultat dans le PDF d'un plan

## 1. Problème

La répétition de la démo d'Annecy (2026-11-12, après 9.16) produit un PDF complet où la
réponse ne se voit pas : emprise de tout le projet (le département), résultat confondu avec
les routes, légende aux noms techniques d'étapes (« 4-analysis.intersect »), couches
intermédiaires et intitulés de bande « Band 1 (Gray) » des MNT. Ticket 7.9 (epic 7), avant la
vidéo (7.6).

## 2. Décision

`layout.export_pdf` reçoit un paramètre caché `focus`, rempli par le noyau pour un export
lancé dans un plan (comme `out`, `grid.epsg` et `work_epsg`), absent de la grammaire du
modèle, du formulaire et du client MCP. Il porte : les couches de résultat (sorties du plan
non consommées par une étape suivante) avec leur nom en clair et leur style de mise en
évidence, les couches intermédiaires à cacher, les couches de cadrage, et
`hide_band_labels`. Le geste QGIS l'applique à la seule mise en page (clone en mémoire du
résultat, jamais ajouté au projet). Un export hors plan, ou d'un plan sans couche produite,
n'a pas de `focus` et garde son comportement. `focus` est journalisé et relu au rejeu, jamais
recalculé.

## 3. Ce qui ne change pas

Identifiants et paramètres visibles du catalogue ; texte lu par le modèle (descriptions,
doc des paramètres, exemples, prompt) — décision du propriétaire du 2026-11-12 ; couches de
l'utilisateur dans le projet ; aucune étape ajoutée au plan (AD-23) ; aucune décision dans
l'extension (AD-5) ; rejeu identique (AD-11).

## 4. Propositions d'édition

- **Catalogue** (`spec-cassini-mvp/catalogue-des-commandes.md`, fiche `layout.export_pdf`) :
  ajouter la ligne `focus` (rempli par le noyau, caché) et son paragraphe ; à faire par 7.9.
- **Contrat d'architecture** : précision datée sous AD-24 (paramètre rempli par le noyau pour
  la mise en page d'un plan) ; à faire par 7.9.
- **Tickets** : 7.9 existant ; aucun autre ticket touché.

## 5. Portée

Mineure : un ticket, un paramètre caché, aucun ré-entraînement. Risque élevé (rejeu) :
relecture thorough.
