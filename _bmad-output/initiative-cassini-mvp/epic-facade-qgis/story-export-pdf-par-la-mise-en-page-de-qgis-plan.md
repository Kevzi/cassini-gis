---
title: 'Export PDF par la mise en page de QGIS'
type: 'feature'
ticket: '12'
created: '2026-10-16'
status: 'built'
baseline_revision: '97d51dc825a03578d750894c50e146a3959f5507'
route: 'full'
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

**Problem:** CAP-5 n'a pas sa commande : la phrase de démo finit par « sors-moi la carte en PDF », et rien ne produit ce PDF.

**Approach:** `layout.export_pdf`, déclarée par le noyau, exécutée par un geste de l'extension qui monte une mise en page QGIS (carte, titre, légende, barre d'échelle, flèche du nord, sources) et l'exporte dans le fichier `out` nommé par le noyau (AD-10).

## Boundaries & Constraints

**Always:** paramètres neutres et typés ; `title` seul paramètre saisi (texte réellement libre) ; sortie `out` nommée par le noyau, `.output_ext(".pdf")` ; la carte montre les couches visibles du projet, cadrée sur leur emprise réunie en SCR du projet ; « sources » = noms de fichiers des couches visibles (jamais de chemin absolu, jamais d'URI à identifiants : `source_name` de `inventory.py`) ; mise en page A4 paysage, construite en mémoire, jamais ajoutée au projet ; couches et projet en lecture seule ; description et exemples FR/EN ; liste `sources` du résultat bornée par `maxItems` (AD-8).

**Never:** pas de gabarit `.qpt`, pas de choix de format, d'orientation, de couches ou d'emprise (plus tard) ; pas d'export d'image ni de SVG ; pas de lecture du canevas de la carte (`iface`) ; pas de modification de `ProcessingGesture`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Export | projet avec couches visibles, `title` | PDF à `out`, résultat `{file, page_count: 1, layer_count, sources[≤50], truncated}` | — |
| Aucune couche visible | projet vide ou tout masqué | — | `layout.empty` |
| Titre vide | `title` blanc | — | `layout.bad_params` |
| Source non fichier | couche mémoire, base distante | absente de `sources`, couche tout de même dessinée | — |
| Écriture refusée | `out` illisible ou export en erreur | aucun fichier partiel | `layout.export_failed` |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_zonal_statistics.rs` -- gabarit (`Delegated`, `output_params`, `output_ext`, exemples, tests de schéma, borne `maxItems`) ; `mod.rs` inclut `commands_mods.rs` engendré : module déclaré tout seul.
- `qgis-plugin/cassini_qgis/layout_export.py` (nouveau) -- geste `LayoutExportGesture` sur le modèle de `_ReadGesture` (`inventory.py`) : fond injectable, ordonnanceur, `done` une seule fois. Réutiliser `QgisBackend.layers()`, `extent_of`, `source_of`, `source_name`, `project_crs`. Export synchrone (la mise en page de QGIS ne tourne pas hors fil principal) : commande `cancellable(false)`.
- `qgis-plugin/cassini_qgis/plugin.py` -- `registry.register("layout.export_pdf", ...)` près de `analysis.register_all`.
- `qgis-plugin/tests/test_analysis.py` (`FakeBackend`), `probe_zonal_statistics.py` -- gabarits des tests et de la sonde réelle.
- `qgis-plugin/README.md` -- recette manuelle. Ne pas toucher `form.py` : `title` est un texte libre ordinaire.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/layout_export_pdf.rs` -- déclarer la commande, sortie `{file, page_count, layer_count, sources[≤50], truncated}`, tests de schéma -- contrat noyau
- [ ] `qgis-plugin/cassini_qgis/layout_export.py`, `plugin.py` -- fond réel (mise en page : carte, titre, légende, barre d'échelle, flèche du nord, sources ; export PDF), geste, nettoyage du fichier partiel -- traduction dans l'extension
- [ ] `qgis-plugin/tests/test_layout_export.py` -- cas de la matrice avec doublure -- preuve
- [ ] `qgis-plugin/tests/probe_layout_export.py` -- sonde QGIS réelle : les cinq éléments présents dans la mise en page construite, PDF qui commence par `%PDF` -- preuve
- [ ] `qgis-plugin/README.md` -- recette manuelle

**Acceptance Criteria:**
- Given le projet de test Villeneuve ouvert dans QGIS, when `layout.export_pdf` est lancé avec un titre, then un PDF apparaît dans le dossier de travail et s'ouvre dans un lecteur standard avec titre, légende, échelle, flèche du nord et sources.
- Given aucune couche visible, when la commande est lancée, then elle refuse par `layout.empty` sans fichier.
- Given le noyau seul, when `cargo test` tourne, then tout passe sans QGIS.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Les « cinq éléments » annoncés par le fond sont écrits en dur, la sonde ne peut pas échouer | medium | patch (corrigé) | `export_pdf` relit maintenant `layout.items()` par type, texte du titre/sources et chemin de la flèche ; sonde réelle OK. |
| SCR du projet invalide : emprises non reprojetées | low | rejeté | QGIS donne toujours un SCR au projet ; correctif = branche en plus. |
| Étiquette des sources et légende à taille fixe, coupées si beaucoup de couches | low | rejeté | Cosmétique, à voir à la recette manuelle ; mise en page adaptative hors plan. |
| `_discard(out)` supprime un fichier préexistant | low | rejeté | `out` est nommé par le noyau pour cette étape (AD-10) ; même nettoyage que `ProcessingGesture`. |
| La sonde n'ouvre pas le PDF (page blanche passerait) ; légende/sources non vérifiées | low | rejeté | Aucun moteur de rendu ; relu en recette manuelle. |
| « six » vs « cinq » éléments dans le docstring | low | rejeté | Libellé : titre + quatre autres ; cosmétique. |
| Titre blanc non refusé côté noyau | low | rejeté | Le geste rend `layout.bad_params` (testé) ; le schéma noyau ne porte pas `minLength` par choix de la règle 1. |

## Design Notes

Éléments QGIS : `QgsLayoutItemMap`, `QgsLayoutItemLabel` (titre, sources), `QgsLayoutItemLegend`, `QgsLayoutItemScaleBar`, `QgsLayoutItemPicture` (flèche du nord, via `setNorthMode`), export par `QgsLayoutExporter.exportToPdf`. Le fond rend la liste des types d'éléments posés, pour que la sonde et les tests prouvent les cinq.

## Verification

**Commands:**
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propre
- `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` -- expected: OK
- Sonde réelle sur `../donnees-test-cassini/projet-test-villeneuve.qgz` -- expected: PDF valide, cinq éléments

**Manual checks (if no CLI):**
- Dans QGIS, `layout.export_pdf` : ouvrir le PDF et voir les cinq éléments.
