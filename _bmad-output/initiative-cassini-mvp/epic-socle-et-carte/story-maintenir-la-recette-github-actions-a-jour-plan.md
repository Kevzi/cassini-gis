---
title: 'Maintenir la recette GitHub Actions a jour'
type: 'chore'
ticket: '11'
created: '2026-11-05'
status: 'built'
baseline_revision: '73460f669144a4f50dfac2a404a48ce1a1db0de2'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La recette utilise `actions/checkout@v4`, `actions/setup-node@v4` et `actions/setup-python@v5`, qui tournent sur Node.js 20 que GitHub déprécie, et l'étiquette `ubuntu-latest`, qui va basculer vers Ubuntu 26.

**Approach:** Passer ces trois actions à leur version majeure actuelle sur Node.js 24 (v7 pour les trois), et remplacer `ubuntu-latest` par `ubuntu-24.04` partout, matrice du job extension comprise ; ne rien changer à ce que la recette contrôle. Décisions du propriétaire (2026-11-05) : figer ubuntu-24.04, Ubuntu 26 plus tard par un ticket à part ; `windows-latest` reste tel quel ; `EmbarkStudios/cargo-deny-action` (Docker) n'est pas touchée.

</frozen-after-approval>

## Implementation Notes

Voie courte : une dizaine de lignes dans `.github/workflows/recette.yml`. Vérifié le 2026-11-05 par l'API GitHub : checkout v7.0.1, setup-node v7.1.0 et setup-python v7.1.0 déclarent `using: node24`. Changements cassants de v5 à v7 relus (cache automatique de setup-node, retrait de `pip-install` dans setup-python, identifiants de checkout dans un fichier à part, refus des PR de fork en `pull_request_target`) : aucun ne touche les options de la recette (`node-version`, `cache: npm`, `cache-dependency-path`, `python-version`). La preuve finale est une recette verte sans annotation après poussée sur main.

## Verification

**Commands:**
- `grep -nE "ubuntu-latest|@v4|setup-python@v5" .github/workflows/*.yml` -- expected: aucune ligne
- Recette GitHub après poussée sur main -- expected: cinq jobs verts, aucune annotation Node.js 20 ni ubuntu-latest, image ubuntu-24.04 dans les journaux Linux

## Review Triage Log

Passe 1 (quick). high 0, medium 0, low 1, false 1, maybe-false 0, plus un constat de vérification.

- vérification, non un défaut — la preuve du `verify` (recette verte sans annotation, image ubuntu-24.04) ne se voit qu'après poussée sur main ; le ticket ne passe pas à `done` avant.
- false — plan en mode 100755 : `core.fileMode=false`, l'index l'enregistre en 100644.
- low, defer — commentaire d'en-tête de `recette.yml` (« rien d'autre que les actions officielles de GitHub ») inexact depuis l'ajout de cargo-deny-action ; antérieur à ce ticket.
