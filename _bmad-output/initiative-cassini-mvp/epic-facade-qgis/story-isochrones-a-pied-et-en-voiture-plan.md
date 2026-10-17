---
title: 'Isochrones à pied et en voiture'
type: 'feature'
ticket: '11'
created: '2026-10-17'
status: 'built'
baseline_revision: '7829f94f4f969ecdc962efa5206f6cd7868288cd'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/regle-de-redaction-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** CAP-3 n'a pas son dernier verbe : « ce qui est atteignable en N minutes depuis ce point ». Sans lui, la phrase de démo (« à moins de 500 m d'une gare ») n'a pas d'équivalent en temps de trajet.

**Approach:** décision du propriétaire (2026-10-16) : l'utilisateur apporte son réseau routier, une couche de lignes choisie dans la liste. `analysis.isochrone` (noyau : déclaration seule, AD-22) est exécutée par l'exécuteur générique : service area de QGIS depuis un point d'origine, puis enveloppe convexe des tronçons atteints, écrite dans le `.gpkg` fourni par le noyau. Ajouter à l'exécuteur une étape suivante facultative (`then`) de la table.

## Boundaries & Constraints

**Always:** paramètres neutres (règle 5) : `layer_id` (réseau, lignes), `origin {x, y, epsg}`, `mode` ∈ `walk`, `car`, `duration {value, unit}` avec `unit` ∈ `s`, `min`, strictement positive ; `out` rempli par le noyau, `.output_ext(".gpkg")`. Vitesses fixes du geste (marche 5 km/h, voiture 50 km/h), sans champ de vitesse du réseau, tronçons parcourus dans les deux sens. Réseau en SCR métrique ; l'origine est reprojetée par QGIS dans le SCR du réseau. Étape intermédiaire en sortie temporaire, jamais de fichier annexe. Tâche annulable, entrée en lecture seule, hors ligne.

**Never:** pas de téléchargement ni de réseau routier fourni par Cassini ; pas d'algorithme choisi par le modèle (table fermée, AD-23) ; pas de champ de sens ou de vitesse ; pas de géocodage de l'origine ; pas de réécriture du canal, du journal ni de `layer.add`.

</frozen-after-approval>

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Marche | grille de rues 100 m en EPSG:2154, origine au centre, `walk`, 3 min | polygone d'une entité ; emprise ≈ 250 m autour de l'origine (± tolérance) | — |
| Voiture | même réseau, `car`, 3 min | emprise plus grande que la marche | — |
| Durée en s | `{180, "s"}` | identique à 3 min | — |
| Sans réseau | `layer_id` absent du projet | — | `layer.not_found` |
| Couche non linéaire | réseau en polygones ou points | — | `analysis.bad_params` (dit de choisir une couche de lignes) |
| SCR en degrés | réseau en EPSG:4326 | — | `analysis.crs_not_metric` |
| Origine hors réseau | point à plusieurs km | — | `analysis.origin_off_network` |
| Durée invalide | `value` ≤ 0 | — | refusée par le noyau avant l'ordre |
| Annulation | arrêt en cours | aucun fichier partiel | `qgis.cancelled` |

## Code Map

- `core/src/commands/analysis_buffer.rs` -- modèle : `Distance` positive, `Delegated`, `.output_ext`. `analysis_zonal_statistics.rs` -- modèle du résultat `layer_id/name/feature_count/crs`.
- `core/src/commands/mod.rs` -- déclarer le module ; `core/src/commands/analysis_isochrone.rs` (nouveau).
- `qgis-plugin/cassini_qgis/analysis.py` -- `ALGORITHMS`, `ProcessingGesture._begin/_on_finished/_load`, `QgisBackend.run_algorithm`, `translate_*` : y ajouter l'entrée, `then`, et la garde `geometry` pour les lignes (`Line`).
- `qgis-plugin/cassini_qgis/form.py` -- `*_layer_id` donne la liste ; `mode` (enum) donne une liste ; ne pas modifier.
- `qgis-plugin/tests/test_analysis.py` (doublure `FakeBackend`), `probe_zonal_statistics.py` -- modèles de test et de sonde QGIS réelle.
- `fixtures/README.md` -- y inscrire le réseau synthétique ; `docs/regle-de-redaction-des-commandes.md`, `qgis-plugin/README.md` -- noms reconnus, recette manuelle.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_isochrone.rs`, `mod.rs` -- déclarer `analysis.isochrone` (exemples, `.cancellable`, `.output_ext(".gpkg")`, tests : déléguée, durée positive, `mode` fermé, sortie `.gpkg`) -- verbe du noyau
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- entrée de table `native:serviceareafrompoint` (`translate_isochrone` : origine, stratégie « plus rapide », vitesse du mode, coût en secondes) + `then` `native:convexhull` sur le résultat temporaire ; erreur `analysis.origin_off_network` si aucune ligne atteinte -- seul fichier qui nomme QGIS
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- exécuteur : enchaîner l'étape `then`, une seule sortie écrite, annulation et nettoyage inchangés
- [ ] `fixtures/reseau-grille-2154.geojson` (+ README) -- grille de rues synthétique, EPSG:2154
- [ ] `qgis-plugin/tests/test_analysis.py`, `probe_isochrone.py` -- cas de la matrice avec doublure ; sonde sous le vrai QGIS (emprise attendue)
- [ ] docs -- règle 11 (`mode`), recette manuelle

**Acceptance Criteria:**
- Given la grille de rues chargée, when `analysis.isochrone` est lancée depuis le panneau (réseau choisi dans la liste, `mode` dans une liste), then une couche d'un polygone à l'emprise attendue s'ajoute au projet, sans réseau.
- Given aucune couche de lignes choisie, when la commande est lancée, then elle refuse par une erreur typée, sans fichier.
- Given le noyau seul, when `cargo test` tourne, then tout passe sans QGIS.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| # | Finding | Verdict | Route | Evidence |
|---|---------|---------|-------|----------|
| 1 | README étape 5 : `` d'un chemin remplacé par un caractère de contrôle (`15<BS>in`) | medium | patch (corrigé) | Octet 0x08 trouvé dans la ligne 151 ; remplacé par `in`, plus aucun 0x08. |
| 2 | « carré de 500 m » alors que l'enveloppe est un losange | low | patch (corrigé) | Seule l'emprise fait 500 m ; textes du README, des fixtures et de la sonde reformulés. |
| 3 | Schéma `range(min = 0.0)` inclusif alors que la durée est strictement positive | low | rejeté | Même motif que `analysis.buffer` ; le noyau refuse 0 à la désérialisation, correction non directe. |
| 4 | Passage de la couche temporaire non confirmé sous QGIS réel | false | rejeté | La sonde réelle de l'implémenteur finit `RÉSULTAT : OK` (emprise 652250-652750 × 6850250-6850750). |
| 5 | `mod.rs` listé au plan mais absent du diff | false | rejeté | `build.rs` déclare les modules ; aucun défaut. |
| 6 | Test « either step » plus faible que son nom | low | rejeté | Le correctif ajoute des branches de test sans défaut de produit. |

## Design Notes

L'enveloppe convexe surestime l'isochrone sur un réseau en étoile ou à rivière (elle comble les creux) : limite assumée au MVP, notée dans la description de la commande ; une enveloppe concave serait une amélioration ultérieure. L'origine se saisit en coordonnées, faute de sélecteur de point sur la carte : à reprendre en suite (règle 11).

## Verification

**Commands:**
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propre
- `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` -- expected: OK
- `python-qgis-ltr.bat qgis-plugin/tests/probe_isochrone.py` -- expected: `RÉSULTAT : OK`

**Manual checks (if no CLI):**
- Dans QGIS, `analysis.isochrone` : liste des couches, liste `mode`, aucun champ `out` ; polygone cohérent autour de l'origine.
