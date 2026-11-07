---
title: 'Reprojection d''un point par QGIS (data.transform_point)'
type: 'feature'
ticket: '4'
created: '2026-11-06'
status: done
baseline_revision: 'cc5926d3a5388a367bdf8b4cb4f8cba86b142d7d'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la géodésie du noyau (`coord.convert`, `geo.*`) refuse tout point hors WGS84/UTM en nommant `data.transform_point`, qui n'existe pas encore : un point Lambert-93 ne peut pas rejoindre la géodésie, ni un point WGS84 le système d'un MNT.

**Approach:** déclarer au registre la commande déléguée `data.transform_point` exactement comme sa fiche du catalogue (§6), et l'exécuter dans l'extension par la reprojection de QGIS (`QgsCoordinateTransform`, contexte de transformation du projet) ; le panneau affiche le point reprojeté.

## Boundaries & Constraints

**Always:** paramètres `point: Point`, `target_epsg: integer` et résultat `point: Point` (dont `epsg` = `target_epsg`), exactement ceux de la fiche ; erreur `data.unknown_crs` (AD-21, message en français) quand QGIS ne connaît pas le code source ou cible, en nommant le code fautif ; la reprojection est celle de QGIS, jamais du noyau (AD-19) ; même code source et cible → le point rendu tel quel ; fiche et §4 du catalogue passés en `faite` dans le même commit ; tests de l'extension sans QGIS par un fond doublure, plus une sonde sous le vrai QGIS.

**Never:** aucune formule de projection dans le noyau ni dans l'extension ; aucune couche créée, aucun fichier écrit (`out` absent) ; aucune reprojection insérée en silence avant une autre commande ; pas de famille d'entraînement ni de ré-entraînement (entrées 12 et 16) ; pas de recette de la table fermée (entrée 17) ; aucune dépendance nouvelle.

**Decisions (propriétaire, 2026-11-06):** le geste est un geste dédié, inscrit au registre des gestes (`plugin.py`) comme `layer.add` et `layout.export_pdf`, et non une ligne de `ALGORITHMS` : un point n'est ni une couche ni un fichier ; c'est ce registre que l'entrée 17 confrontera au noyau. Un point que QGIS ne sait pas reprojeter (hors domaine, résultat non fini) rend `data.transform_failed`, ajoutée au texte de la fiche (identifiant et paramètres inchangés). Le plan est gardé entier (~2 000 tokens).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| WGS84 → Lambert-93 | `{x: 3, y: 46.5, epsg: 4326}`, `target_epsg: 2154` (origine de la projection) | `{x: 700000, y: 6600000, epsg: 2154}` à 1 cm près (sonde QGIS) | — |
| Lambert-93 → WGS84 | le point ci-dessus, retour | `{x: 3, y: 46.5, epsg: 4326}` à 1e-9° près | — |
| Même système | `epsg` = `target_epsg` | point inchangé | — |
| Cible inconnue | `target_epsg: 99999` | — | `data.unknown_crs`, nomme EPSG:99999 |
| Source inconnue | `point.epsg: 99999` | — | `data.unknown_crs` |
| Hors domaine | point que QGIS ne sait pas reprojeter (exception, résultat non fini) | — | `data.transform_failed` |
| Paramètres invalides | `target_epsg` absent, négatif ou non entier ; champ en trop | — | `registry.invalid_params` (noyau) |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_intersect.rs` -- modèle d'une commande déléguée : `impl Delegated`, `CommandSpec::new::<P,O>(id, fr, en).runs_in(RunsIn::Qgis).example(..).tests(&[..])`, `inventory::submit!` + `register_delegated`. Pas de `output_params` ici (aucun fichier).
- `core/src/point.rs` -- `Point {x, y, epsg: u32}` (marque `x-cassini: point`) : à réutiliser en paramètre et en résultat ; un résultat `point` est déjà repris par `step:` ; ne rien changer.
- `core/src/commands/geo_point.rs` l. 54 -- message `geo.unsupported_crs` qui nomme déjà `data.transform_point` : inchangé.
- `core/src/commands/mod.rs` -- modules déclarés par `build.rs` : ajouter le fichier suffit.
- `core/tests/catalogue.rs` -- une fiche `planifiee` au registre fait échouer la recette : passer la fiche en `faite`.
- `qgis-plugin/cassini_qgis/layout_export.py` l. 63 -- `reproject_point` : même appel (`QgsCoordinateReferenceSystem`, `QgsCoordinateTransform(.., QgsProject.instance())`), à reprendre dans le fond du nouveau geste, sans toucher `layout_export.py`.
- `qgis-plugin/cassini_qgis/layer_add.py` -- forme d'un geste : fond `QgisBackend` seul à importer `qgis.core`, ordonnanceur injectable, `_finish` une seule fois, `cancel()` → `qgis.cancelled`.
- `qgis-plugin/cassini_qgis/plugin.py` l. 179-185, `qgis-plugin/tests/probe_external.py` l. 141-146 -- inscription des gestes : ajouter la ligne aux deux.
- `qgis-plugin/cassini_qgis/summary.py` (`_point` l. 379, `_SENTENCES` l. 440), `texts.py` (FR l. ~142, EN l. ~305), `tests/test_summary.py` -- `_point` ne sait rendre que 4326 et UTM : ajouter le cas d'un autre système (« EPSG:2154 x, y »).
- `training/`, `ai/` -- ne pas modifier.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/data_transform_point.rs` -- params, résultat, spec déléguée (descriptions FR/EN : « reprojette un seul point vers un code EPSG, par QGIS ; relie un point Lambert-93 à coord.convert/geo.*, ou un point WGS84 au système d'un MNT »), exemples, tests : déléguée (`registry.unavailable` sans QGIS), schéma accepte/refuse la matrice des paramètres invalides.
- [x] `qgis-plugin/cassini_qgis/point_transform.py` -- le geste dédié et son fond QGIS (seul à importer `qgis.core`) : codes source et cible validés, reprojection, `data.unknown_crs` / `data.transform_failed`.
- [x] `qgis-plugin/tests/test_point_transform.py` -- fond doublure : cas de la matrice, erreurs nommées, annulation, une seule réponse.
- [x] `qgis-plugin/tests/probe_point_transform.py` -- sonde sous le vrai QGIS : origine Lambert-93 à 1 cm, retour à 1e-9°, code inconnu.
- [x] `qgis-plugin/cassini_qgis/plugin.py`, `qgis-plugin/tests/probe_external.py` -- inscrire le geste.
- [x] `qgis-plugin/cassini_qgis/summary.py`, `texts.py`, `tests/test_summary.py` -- phrase FR/EN « Point reprojeté en EPSG:2154 : … ».
- [x] `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- fiche déplacée en §5 et passée en `faite`, §4 aussi, erreurs précisées.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when on lance `cargo test --workspace`, then la recette du catalogue passe avec la fiche `faite`.
- Given le serveur MCP lancé sans QGIS, when on liste ses outils, then `data.transform_point` n'y figure pas (déléguée, servie seulement par une session QGIS, `cli/tests/mcp.rs` l. 67).
- Given QGIS 3.44 et l'extension installée, when la sonde reprojette l'origine du Lambert-93, then elle rend (700000, 6600000) à 1 cm près, et un code inconnu rend `data.unknown_crs`.

## Implementation Notes

- Rendu du panneau : la phrase nomme déjà le code (« Point reprojeté en EPSG:2154 : x 700000 ; y 6600000 »), donc `_point(..., any_system=True)` rend un autre système par x, y seuls, sans répéter « EPSG:2154 » ; au millimètre au-delà de 1000 (mètres), à sept décimales sinon (degrés). Une cible WGS84 ou UTM reprend les formes de `geo.*`. Sans `any_system`, `_point` reste strict (un point 2154 rendu par `geo.*` tombe toujours au repli).
- Sonde, cas hors domaine : un point Lambert-93 très lointain (1e14 m) rend sous PROJ un résultat fini près du pôle, pas une erreur ; la sonde prend donc une latitude de 95° (EPSG:4326 → 2154), refusée par PROJ (« Invalid coordinate ») → `data.transform_failed`.
- Le geste refuse aussi un code inconnu quand source et cible sont égales (les codes sont vérifiés avant le raccourci « même système »).
- Vérification sous Windows (WSL n'a ni libclang ni GTK pour llama.cpp et Tauri) : `cargo clippy --all-targets -- -D warnings` et `cargo test --workspace` verts (`CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH` posé), `cargo fmt --check` vert, `python -I -m unittest discover -s qgis-plugin/tests` : 408 tests verts, garde de l'extension propre, sonde QGIS 3.44.15 : 6/6.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — high 0, medium 1, low 2, false 0, maybe-false 1 ; rejeté en low 1.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Forme nue « x, y » : en français, virgule décimale et séparateur se confondent (« 652000,25, 6853000,5 ») | medium | patch | `summary.py` `_point(any_system=True)` ; valeurs étiquetées `x … ; y …` |
| Système géographique autre que 4326 rendu longitude d'abord, sans lettre | low | patch | même racine d'affichage : l'étiquette `x`/`y` lève l'ambiguïté |
| Seuil de 1000 : une coordonnée projetée < 1000 m rendue à 7 décimales | low | rejeté | cas rare (systèmes locaux), affichage seul ; la correction ajoute une règle par système |
| `QgsCoordinateTransform.isValid()` non vérifié : une transformation invalide rendrait le point inchangé | maybe-false | defer | avec deux SCR valides, QGIS replie sur une autre opération (fallback) ; à établir par une sonde sur un projet dont le contexte nomme une grille absente |

## Design Notes

Référence de la sonde : l'origine du Lambert-93 (φ0 = 46°30' N, λ0 = 3° E) a par définition E = 700 000 m, N = 6 600 000 m (décret n° 2000-1276, paramètres IGN). Le passage WGS84 → RGF93 est la transformation nulle de l'EPSG ; aucun jeu de référence tiers n'est requis.

## Verification

**Commands:**
- `cargo test --workspace` -- expected: tout vert, dont `catalogue`
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert

**Manual checks (if no CLI):**
- `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_point_transform.py` -- expected: toutes les vérifications passent.
