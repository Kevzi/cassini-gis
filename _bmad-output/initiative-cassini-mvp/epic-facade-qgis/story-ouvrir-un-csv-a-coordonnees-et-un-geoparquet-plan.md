---
title: 'Ouvrir un CSV a coordonnees et un GeoParquet'
type: 'feature'
ticket: '6'
created: '2026-10-16'
status: done
baseline_revision: '2a88c98fc3dfc7cfe166d67c77c9f33108d90382'
route: 'oneshot'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/regle-de-redaction-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `layer.add` n'ouvre que GeoPackage, Shapefile et GeoJSON ; le CSV à coordonnées et le GeoParquet, deux formats visés au MVP (CLAUDE.md §5), sont refusés.

**Approach:** étendre `layer.add` : `.parquet` passe par OGR comme les autres ; `.csv` passe par le fournisseur `delimitedtext` de QGIS (colonnes `X` et `Y` par défaut, champs `x_field`/`y_field` optionnels, séparateur `,` ou `;`), avec le SCR **obligatoire** via `crs` (un CSV n'en porte jamais). Mêmes garanties : lecture seule, fichier inchangé, SCR explicite. L'inconnue sur GeoParquet est levée : les données de test s'ouvrent dans QGIS 3.44.15 (LISEZMOI.txt) ; si le pilote manque ailleurs, la couche est invalide → `layer.invalid_source`.

</frozen-after-approval>

## Implementation Notes

Oneshot (~60 lignes, deux fichiers de code). Décisions : CSV sans `crs` → `layer.unknown_crs` avant toute ouverture ; pas de gestion de la virgule décimale, ni de WKT, ni de choix du séparateur (hors périmètre).

Verifie sur QGIS 3.44.15 reel : gares-2154.csv (8 points, EPSG:2154), villes.csv, villes.parquet (243, EPSG:4326), fichiers inchanges. Correction trouvee en route : l'URI delimitedtext casse si delimiteur et SCR sont encodes en pourcent ; seuls les noms de colonnes le sont.

## Review Triage Log

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 | medium | patch (fait) | Docs schemars de `path` et `crs` perimees : le modele aurait cru .csv/.parquet refuses. |
| 2 | medium | patch (fait) | Test `Boom` sans le parametre `csv` : levait TypeError au lieu de RuntimeError. |
| 3 | low | patch (fait) | Regex de noms de colonnes trop stricte : remplacee par encodage pourcent + `layer.invalid_column`. |
| 4 | low | rejete | Double de test ne couvrant pas l'URI : couverte par l'essai sur le vrai QGIS ci-dessus. |
| 5 | low | rejete | `crs` requis pour CSV seulement cote extension : voulu (AD-19, le noyau ne connait pas les formats). |
| 6 | low | rejete | `x_field`/`y_field` ignores hors CSV : inoffensif, ajouterait du code. |
