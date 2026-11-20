---
title: 'Atlas PDF (layout.export_atlas)'
type: 'feature'
ticket: '15'
created: '2026-11-20'
status: 'built'
baseline_revision: '5256ef8fab1b3ed15d85b26152a1dd3a3d422778'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Cassini n'exporte qu'une carte d'une page ; il n'a pas de série (une page par entité d'une couche), et `layout.export_pdf` dessine aussi les couches distantes du projet (WMS, WMTS, XYZ, tuiles vectorielles distantes), ce qui ouvre une requête réseau.

**Approach:** Ajouter `layout.export_atlas`, livré par un genre `layout` de la table fermée du geste générique, qui monte la même mise en page que `layout.export_pdf` et la fait exporter par `native:atlaslayouttopdf`. Les deux exports ne dessinent que des couches locales ; les couches distantes sont écartées et nommées (`skipped_layers`, plan).

## Boundaries & Constraints

**Always:** Identifiants et paramètres du catalogue inchangés (`title`, `coverage_layer_id`, `label_field`, `out` ; résultat `file`, `page_count`, `layer_count`, `skipped_layers`, `sources`, `truncated`) ; chemin `out` dicté par le noyau (AD-10) ; une ligne de la table fermée, pas de commande passe-plat (AD-24) ; aucune requête réseau (AD-14) ; le noyau reste sans QGIS, avec un test Rust.

**Never:** Aucune expression de filtre ou de tri émise (AD-23) ; `label_field` est vérifié contre les champs de la couche, jamais recopié tel quel dans une expression. Pas de `SummaryRow`, de GRASS ni de famille d'entraînement (ticket 17). Aucune modification du rendu de `layout.export_pdf` en dehors de l'écart des couches distantes ; pas de réécriture de `layout.export_pdf` en `native:printlayouttopdf` (sa mise en page reste montée à la main).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Atlas | couverture de 3 entités, couches locales | un PDF de 3 pages au chemin du noyau, `page_count` 3 | Aucune erreur |
| Couche distante | projet avec une couche XYZ visible | couche écartée, nommée dans `skipped_layers`, aucune requête réseau | Aucune erreur ; avertissement au plan |
| Export simple | `layout.export_pdf` avec une couche WMS | même règle, `skipped_layers` rempli | Aucune erreur |
| Que du distant | toutes les couches visibles distantes | rien à dessiner | `layout.empty`, aucun fichier |
| Couverture vide | couche sans entité | refus nommé | `layer.empty`, aucun fichier |
| Champ inconnu | `label_field` absent de la couche | refus avant tout lancement | `analysis.unknown_field` |
| Échec QGIS | algorithme en erreur | fichier retiré | `layout.export_failed` |

</frozen-after-approval>

## Code Map

- `core/src/commands/layout_export_pdf.rs` -- modèle de la commande (params, `spec()`, `.runs_in(Qgis)`, `.output_ext(".pdf")`, tests) ; y ajouter `skipped_layers` au résultat.
- `core/src/commands/layout_export_atlas.rs` -- à créer ; s'enregistre seule (`build.rs`) ; `title` non vide, passe `leave_out`/`replay` comme l'export simple si le schéma l'exige.
- `qgis-plugin/cassini_qgis/layout_export.py` -- `LayoutBackend.visible_layers` (383) : filtrer le distant ; `export_pdf` (469-742) : sortir le montage dans une méthode partagée ; `LayoutExportGesture` (1026) : `skipped_layers`.
- `qgis-plugin/cassini_qgis/analysis.py` -- table fermée (824-910), `ProcessingGesture` (1812), `_run_step` (1969), `QgisBackend.run_algorithm` (1462) : genre `layout`.
- `qgis-plugin/cassini_qgis/delegated.py:66` -- table des ids ; `tests/test_delegated.py` vérifie qu'elle égale les commandes `runs_in: qgis` du noyau.
- `qgis-plugin/cassini_qgis/{summary.py:566,texts.py:162}` -- phrase de résumé FR/EN, modèle `_export_pdf`.
- `qgis-plugin/tests/probe_prepare.py`, `probe_layout_export.py` -- modèles de sonde sous QGIS 3.44.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md:2262-2292` -- fiche à passer de `planifiee` à livrée.
- À ne pas changer : `core/src/focus.rs`, `layout_export.py` pour la grille, le focus et les marges.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/cassini_qgis/layout_export.py` -- `visible_layers` écarte les couches distantes (fournisseurs `wms`, `wmts`/`xyz`, `vectortile`, `arcgis*`, `wfs`) et en rend les noms ; le résultat de l'export simple porte `skipped_layers` -- règle commune aux deux exports
- [ ] `qgis-plugin/cassini_qgis/layout_export.py` -- extraire le montage de la mise en page en une méthode partagée -- l'atlas réutilise titre, légende, échelle, flèche du nord, sources
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- genre `layout` et ligne `layout.export_atlas` : mise en page nommée ajoutée au gestionnaire le temps de l'appel (retirée dans un `finally`), `native:atlaslayouttopdf` avec `LAYERS` locales seules, `label_field` contrôlé -- AD-24
- [ ] `qgis-plugin/cassini_qgis/delegated.py` -- inscrire `layout.export_atlas` -- table fermée
- [ ] `core/src/commands/layout_export_atlas.rs`, `layout_export_pdf.rs` -- commande, `skipped_layers`, tests -- registre
- [ ] `qgis-plugin/cassini_qgis/{summary,texts}.py`, `plan` du noyau -- phrase de résumé et avertissement des couches écartées dans le plan -- AD-8
- [ ] `qgis-plugin/tests/probe_atlas.py`, `tests/test_analysis.py`, `test_layout_export.py`, `test_summary.py`, `test_result_conformance.py`, `fixtures/resultats-extension.json` -- sonde sur trois entités et couche XYZ, tests unitaires des cas de la matrice
- [ ] `catalogue-des-commandes.md`, `README.md`, `qgis-plugin/README.md`, `ai/tests/par_commande/phrases.json` -- fiche, table de commandes, phrases de planification (l'agent compose un plan valide)

**Acceptance Criteria:**
- Given un atlas sur trois entités synthétiques, when `layout.export_atlas` s'exécute, then un seul PDF de trois pages est écrit au chemin nommé par le noyau.
- Given un projet avec une couche XYZ visible, when l'un ou l'autre export s'exécute, then la couche est écartée, nommée dans `skipped_layers`, et aucune requête réseau n'est émise.
- Given la recette hors ligne, when elle tourne, then elle reste verte ; `cargo fmt`, `cargo clippy` sans avertissement, `cargo test` vert.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| # | Verdict | Route | Evidence |
|---|---------|-------|----------|
| 1 README : octet 0x08 dans le chemin de la sonde | low | patch | Réel (4 lignes, dont 2 ajoutées) ; remplacé par `in`. |
| 2 Pas de note au plan pour `layout.export_pdf` | low | defer | Réel, mais la note allonge l'étiquette courte et casse 2 tests existants ; annulé, consigné. |
| 3 `delegated.py` non modifié | false | rejeté | `table_rows` copie `analysis.ALGORITHMS` ; la ligne est reprise ; 1125 tests verts. |
| 4 `skipped_layers` après le focus | false | rejeté | Les résultats d'un plan sont des sorties de Cassini, jamais distantes. |
| 5 Fuite de la mise en page montée | false | rejeté | `_finish` retire toujours `_mounted` ; rien n'est ajouté avant `addLayout`. |
| 6 `is_remote_source` : `/vsizip//vsicurl`, `WMS:` | low | patch | Réel ; préfixes `wms:`/`wmts:`/`wfs:` et `/vsicurl` n'importe où. Bases réseau (postgres…) : maybe-false, consigné. |
| 7 `details` inutilisé dans l'atlas | low | rejeté | Négligeable, aucune conséquence. |
| 8 EU33 : `missing_params` sort de la garde « rougit » | low | rejeté | Un refus d'option d'une commande livrée ne peut pas rougir à l'arrivée au registre. |
| 9 ATL2 | - | rejeté | Le constat conclut que c'est couvert. |

## Design Notes

L'export simple monte sa mise en page à la main (`QgsLayout` jamais ajouté au gestionnaire) ; le ticket le croyait déjà fondé sur `native:printlayouttopdf`. Décision : ne pas le changer. L'algorithme d'atlas exige une mise en page nommée du gestionnaire : le genre `layout` l'y pose le temps de l'appel, avec la carte pilotée par l'atlas, puis la retire.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace` -- expected: aucun avertissement, tests verts
- `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_atlas.py` -- expected: `RÉSULTAT : OK`
- `python -m unittest discover qgis-plugin/tests` -- expected: tests verts

**Manual checks (if no CLI):**
- Rendre une page du PDF d'atlas avec Ghostscript et la regarder (titre, légende, échelle, flèche, sources).
