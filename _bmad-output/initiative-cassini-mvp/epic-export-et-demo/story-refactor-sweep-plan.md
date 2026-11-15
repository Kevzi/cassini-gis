---
title: 'Refactor sweep de l''epic export et démo'
type: 'chore'
ticket: '7'
created: '2026-11-15'
status: 'built'
baseline_revision: 'df7af9f3a9279d1a02c8ecada196ee69a7572151'
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

**Problem:** l'epic export et démo est fini (tickets 1, 4, 5, 6, 9 faits ; 2 et 3 abandonnés) et le ticket 7 demande de traiter ou consigner les constats de relecture reportés, la sonde devant rester OK sur les fixtures.

**Approach:** classer chaque constat reporté (sonde, noyau, extension, décision du propriétaire), mettre à jour la seule entrée devenue caduque, consigner le tri dans ce plan, puis rejouer la sonde sur les fixtures. Aucun code de la sonde ne change : elle n'a ni TODO ni constat ouvert.

</frozen-after-approval>

## Implementation Notes

Route `oneshot` : le travail est un tri de constats et une preuve de non-régression, bien sous 100 lignes.

**Tri des constats reportés** (`deferred-work.md`, source 7.9 et pente cachée) :

| # | Constat | Lieu | Sort |
|---|---------|------|------|
| 1 | Un résultat raster est rangé sous les vecteurs pleins de l'utilisateur | extension, `layout_export.py` | consigné, hors sonde (entrée « RÉSOLU » du 2026-11-13, reste vrai) |
| 2 | Session rouverte en milieu de plan : `plan_start` vaut `None`, aucun `focus` | noyau, `journal/mod.rs` | consigné, hors sonde |
| 3 | Légende des MNT « Band 1 (Gray) » hors export de plan | extension, décision du propriétaire | consigné, à trancher |
| 4 | Résultat ligne d'un seul trait `#3d0026` | extension + noyau | consigné, aucune démo ne rend de ligne |
| 5 | Noms de résultats en français seulement | noyau | consigné |
| 6 | `result_label` coupe les titres en casse de titre | noyau | consigné, risque faible |
| 7 | Sorties des plans précédents sous leur nom technique | noyau | à mettre à jour : traité en grande partie par f5bd91c (2026-11-13), reste le cas du constat 2 |
| 8 | Dessin du focus vérifié seulement par sonde manuelle | tests | consigné, pas de QGIS en CI |

Rejets déjà justifiés dans les journaux de triage des plans 6.1, 6.3, 6.4 et 6.5 : conservés tels quels.

**Mesures du 2026-11-15 :**
- `python -I -m unittest discover -s qgis-plugin/tests` : 853 tests OK.
- `probe_demo.py` sur les fixtures avec le noyau installé (`%LOCALAPPDATA%\Cassini\bin`, construit le 13 novembre à 09:35) : **ECHEC** sur la pente (« sorties du plan 1 encore dans le PDF »). Cause : ce noyau précède f5bd91c (13 novembre, 18:22), qui cache les sorties des plans précédents.
- Même sonde avec le noyau local reconstruit à jour (`C:\ct-5-57g`) : **OK**, rejeu identique sur 9 étapes, 0 connexion réseau vue sur 80 relevés.
- Suite à décider par le propriétaire : réinstaller un noyau à jour (rien n'est installé par ce ticket).

**Tâches :**
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- nouvelle entrée en fin de fichier précisant le constat 7 (le fichier est en ajout seul : l'ancienne entrée n'est pas éditée) : traitée par f5bd91c, reste le cas de la session rouverte.
- sonde de démo -- aucune modification : relancer `qgis-plugin/tests/probe_demo.py` sur `fixtures/demo/` et consigner le verdict dans ce plan.

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: tous les tests passent (dont `test_demo_verdict.py`)
- `probe_demo.py` sur les fixtures (Python de QGIS 3.44, modèle installé, noyau reconstruit à jour : le noyau installé précède f5bd91c et rend ECHEC, voir les mesures et `deferred-work.md`) -- expected: verdict OK, rejeu identique

**Manual checks:**
- `deferred-work.md` : l'entrée du constat 7 dit ce qui est traité et ce qui reste.

## Review Triage Log

Relecture `quick`, 8 constats.

| Verdict | Constat | Suite |
|---|---|---|
| medium | La vérification citait le noyau installé, qui rend ECHEC sur la pente | corrigé : vérification précisée, entrée à décider ajoutée à `deferred-work.md` |
| low | L'évidence de l'entrée ajoutée disait « noyau ou extension » pour toutes les lignes | corrigé : les lignes 3 et 8 y sont nommées |
| low, rejeté | Doublon avec l'entrée de la session rouverte | voulu : l'entrée ajoutée marque la précision, le fichier est en ajout seul |
| low, rejeté | Ligne 1 du tri portée comme consignée alors que son entrée dit RÉSOLU | la ligne dit ce qui reste vrai (raster sous les vecteurs pleins) |
| low, rejeté | Plans 6.1 à 6.5 cités hors `context` | cosmétique, les plans sont dans le même dossier |
| false | Ancienne entrée non marquée caduque | le fichier est en ajout seul, l'entrée ajoutée la précise |
| false | Tests Rust non lancés | ce ticket ne touche aucun code Rust |
| false | Commit scindé | les deux fichiers partent dans un seul commit |
