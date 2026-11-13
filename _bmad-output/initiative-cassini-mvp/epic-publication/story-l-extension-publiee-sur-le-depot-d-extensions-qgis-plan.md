---
title: "L'extension publiée sur le dépôt d'extensions QGIS"
type: 'feature'
ticket: '10'
created: '2026-11-13'
status: 'built'
baseline_revision: 'e6cc90484e50ae794f19f68751a266f9bec0cad7'
route: 'oneshot'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `metadata.txt` est incomplet pour le dépôt d'extensions QGIS (version 0.0.1, `experimental=True`, `email`, `repository`, `tracker` vides, `homepage` et `icon` absents, « (traceur) » dans la description) et l'archive de 7.2 ne contient pas de LICENSE.

**Approach:** Compléter `metadata.txt` (version 0.1.0, `experimental=False`, adresses fixes de la décision Q8, contact cassini-sig@proton.me, icône déjà dans le paquet), ajouter le texte de la GPL-3.0 au paquet, et faire vérifier ces exigences par la garde de l'extension et par le script d'archive. La soumission au dépôt (compte OSGeo, approbation) reste l'étape humaine du ticket.

</frozen-after-approval>

## Implementation Notes

Voie oneshot : un fichier de métadonnées, un texte de licence, deux garde-fous et leurs tests, moins de 100 lignes. Le LICENSE est la GPL-3.0 (l'extension est GPL, le LICENSE de la racine est l'Apache-2.0 du noyau), placé dans `cassini_qgis/` pour entrer dans l'archive.

## Verification

**Commands:**
- `python -I qgis-plugin/tests/check_extension.py` -- expected: propre
- `python -I -m unittest discover -s qgis-plugin/tests -p "test_check_extension.py"` et `python -I -m unittest discover -s installer/tests` -- expected: OK

## Review Triage Log

- medium, patché : le script d'archive ne vérifiait pas LICENSE, icône ni clés ; `verifier_publication` ajoutée et appelée par `main`, avec tests.
- low, rejeté : valeurs exactes de Q8 non comparées, icône hors paquet, LICENSE vide, test par sous-chaîne ; peu probables et le correctif ajouterait plus qu'une simple correction.
- low, corrigé : le test de l'archive ne fige plus `version=0.1.0`.
