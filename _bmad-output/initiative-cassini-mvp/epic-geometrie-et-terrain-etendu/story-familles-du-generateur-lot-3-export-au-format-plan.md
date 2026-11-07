---
title: 'Familles du générateur pour les nouvelles commandes — lot 3, export au format'
type: 'feature'
ticket: '12'
created: '2026-11-07'
status: 'built'
baseline_revision: '28725d2ea0de50dae286eb4ae6f17417034bcefd'
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

**Problem:** Le générateur apprend encore au modèle à refuser « exporte en A3 », « en portrait », « en A0 » (refus `ONLY_A4` de `refus_hors_registre`), alors que `layout.export_pdf` porte depuis 11.11 `page_size`, `orientation` et `grid`. Aucun exemple n'apprend le format ni le carroyage avant le ré-entraînement groupé (entrée 16).

**Approach:** Une famille `export_pdf_format` : la carte seule ou au bout d'une recherche, avec format, orientation et/ou carroyage écrits dans la demande, recopiés dans le plan. Le cas « papier » de `refus_hors_registre` est remplacé, le test `paper_and_orientation_refusals_stay_refusals` inversé, un garde-fou échoue si le refus A4 revient, et la doc corrigée. Lot 3, dernier lot du ticket (découpage du propriétaire, 2026-11-07).

**Décisions (propriétaire, 2026-11-07) :** (1) toute demande de carroyage du jeu nomme son système (Lambert-93 → 2154, UTM 30N/31N/32N → 32630/32631/32632, degrés ou grades → graticule) ; quand il manque (« carroyage UTM » sans zone, « quadrillage de 500 m » sans système), la réponse est une question qui demande le système, jamais un plan. (2) Le cas papier de `refus_hors_registre` devient un refus de ce que l'export ne fait pas : format hors liste (A5, lettre US) et nombre d'exemplaires ou impression.

## Boundaries & Constraints

**Always:** chaque exemple passe `validate` avec sa demande ; `page_size` parmi `a4`…`a0`, `orientation` parmi `landscape`/`portrait`, `grid` dans l'une des deux formes du catalogue ; un paramètre n'apparaît que si la demande le nomme (jamais `a4` ni `landscape` écrits par défaut, sauf s'ils sont écrits) ; intervalle de grille écrit avec son unité (`m`, `km` ; `deg`, `gon`) et recopié ; vocabulaire civil (§2.9) ; part des redemandes entre 4 et 7 %.

**Never:** nombre d'exemplaires ou impression papier comme plan ; étiquettes MGRS promises ; `scale` tiré dans cette famille (reste à `export_pdf_scale`) ; changement du noyau, de `cassini-ai`, du prompt ou de l'extension ; entraînement ou mesure (entrée 16) ; `DEFAULT_COUNT`, `DEFAULT_SEED` changés.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Format | « Exporte la carte en PDF au format A3 » | `{title, page_size: "a3"}` | — |
| Orientation | « Sors la carte en portrait » | `{title, orientation: "portrait"}` | — |
| Carroyage projeté | « … en A3 avec un carroyage Lambert-93 tous les 1 km » | `{title, page_size: "a3", grid: {kind: projected, epsg: 2154, interval: {value: 1, unit: km}}}` | — |
| Graticule | « … avec un graticule tous les 0,01° » | `grid: {kind: degrees, interval: {value: 0.01, unit: deg}}` | grades → `unit: gon` |
| Système absent | « … avec un carroyage UTM au kilomètre » (sans zone), « quadrillage tous les 500 m » | question qui demande le système du carroyage | — |
| Hors fiche | « Exporte la carte en A5 », « imprime-la en 3 exemplaires » | refus affirmatif (format hors liste, ni exemplaires ni impression) | — |
| Au bout d'une recherche | « Trouve les écoles … et sors-moi la carte en A3 portrait » | étapes de recherche puis un seul `layout.export_pdf` avec `page_size`, `orientation` | — |

</frozen-after-approval>

## Code Map

- `training/src/families.rs` -- `FAMILIES` (l. 113–122, `export_pdf` poids 6, `export_pdf_scale` poids 3) : ajouter `export_pdf_format` (poids 4) après `export_pdf_scale`. Patrons : `export_pdf_scale` (l. 1635, carte seule 2 fois sur 3 sinon `draft_filter_eq`/`draft_distance`/`draft_zone` + `open_find`, `MAX_DRAFT_STEPS`), `pdf_clause` (l. 1589), `pdf`/`pdf_at` (l. 325–332), `azimuth_words`/`angle_words` (l. 4005) pour degrés et grades, `Dist::json` (l. 509).
- `training/src/families.rs` -- `refus_hors_registre` (l. 2476), cas `roll == 1` (l. 2505–2514) et `ONLY_A4` (l. 2463) : remplacer par le refus hors fiche (décision 2), raison(s) nommée(s) en constante(s) ; commentaire l. 2452 (« Le papier A3 vit dans `refus_hors_registre` ») à corriger.
- `training/src/families.rs` -- test `what_is_refused_out_of_registry_is_still_missing_from_the_registry` (l. 5921) : retirer la note l. 5935–5937, ajouter un volet qui échoue si un refus de `refus`/`refus_hors_registre` cite « A4 paysage » ou un format/orientation (patron `reprojects_no_point`).
- `training/src/lib.rs` -- `paper_and_orientation_refusals_stay_refusals` (l. 773) : inverser — au jeu par défaut, aucun refus ne cite « A4 », et `export_pdf_format` a des plans pour A3, A0, portrait et chacune des deux formes de grille ; `export_pdf_scale` ne nomme toujours ni papier ni orientation.
- `training/src/lib.rs` -- `validate` : aucun changement attendu (pas de raster) ; `check_plan` du noyau contrôle les types.
- Catalogue `spec-cassini-mvp/catalogue-des-commandes.md` l. 583–625 ; `core/src/commands/layout_export_pdf.rs` (description lue par le modèle, l. 156).
- `docs/entrainement-du-modele.md` -- ligne 104 (refus « PDF en A3, A0 ou portrait »), table des familles (ligne `export_pdf_format`), section « 11.12, lot 3 » après le lot 2b-2 (l. ~255), phrase « Reste l'export au format » remplacée.
- `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- l. 731–745 : clore les trois entrées (refus `ONLY_A4`, garde-fou perdu, lot 3).

## Tasks & Acceptance

**Execution:**
- [x] `training/src/families.rs` -- famille `export_pdf_format` (format seul, orientation seule, carroyage seul ou combinés ; un tiers au bout d'une recherche ; 1 sur 8 carroyage sans système → question) et son poids -- apprendre les trois paramètres.
- [x] `training/src/families.rs` -- cas papier de `refus_hors_registre` remplacé, `ONLY_A4` retiré, garde-fou rétabli -- ne plus refuser ce que le registre sait faire.
- [x] `training/src/families.rs` (tests) -- clés attendues seules, valeurs du catalogue, chaque valeur retrouvée écrite dans la demande, intervalle et unité recopiés, un seul export par plan -- vérifier sans modèle.
- [x] `training/src/lib.rs` -- test inversé -- verrouiller le remplacement au jeu par défaut.
- [x] `docs/entrainement-du-modele.md`, `deferred-work.md` -- doc et entrées closes.

**Acceptance Criteria:**
- Given le jeu par défaut (graine 1, 8 000 exemples), when il est engendré, then `export_pdf_format` garde au moins un exemple, aucune famille n'est rejetée au résumé, et aucun exemple n'a pour raison « seulement une page A4 paysage ».
- Given `cargo test -p cassini-training`, `cargo clippy -p cassini-training -p cassini-core --all-targets -- -D warnings` et `cargo fmt --check`, when ils tournent, then tout est vert.

## Implementation Notes

- Famille `export_pdf_format` (poids 4) : `grid_words`, `page_words`, `distance_step_text`, `decimal_fr`, `number_json`, question `GRID_SYSTEM_QUESTION` ; refus `NO_PAGE_SIZE` (A5, lettre US) et `NO_PRINTING` (imprimante, exemplaires) à la place d'`ONLY_A4` ; garde-fou `refuses_a_known_page`.
- « Imprime la carte au 1:25 000 » reste un export (5.40) ; seuls « à l'imprimante » et « en N exemplaires » sont refusés.
- Graine 1, 8 000 exemples : `export_pdf_format` 140, aucune famille écartée, 369 redemandes (4,6 %), poids de `redemande` inchangé.
- Test ajouté à la relecture de la matrice : `a_page_out_of_the_series_and_printing_stay_refusals` (ligne « Hors fiche »).

## Plan Change Log

## Review Triage Log

Passe 1 (quick, 2026-11-07) : high 0, medium 0, low 5, false 0, maybe-false 0, plus 1 constat vérifié sans défaut.

| # | Constat | Verdict | Route | Preuve / action |
|---|---|---|---|---|
| 1 | « Imprime » : plan dans `export_pdf_scale`, refus avec « exemplaires » ; ligne de doc « PDF … imprimé » refusé, contradictoire | low | patch | Le code sépare bien imprimante/exemplaires (refus) et « imprime la carte » (export, 5.40) ; doc reformulée dans la table et la section du lot 3. |
| 2 | `refuses_a_known_page` ne contient aucun mot de grille alors que son commentaire parle de carroyage | low | patch | « carroyage », « quadrillage », « graticule » ajoutés ; refus existants toujours admis. |
| 3 | Test : zone UTM contrôlée par `contains("utm") && contains(zone)`, trop lâche | low | patch | La zone doit suivre « utm » (« utm {z} », « utm zone {z} »). |
| 4 | `GRID_SYSTEM_QUESTION` ne propose pas les grades | low | patch | « … ou un graticule en degrés ou en grades ? », doc alignée. |
| 5 | `page_words_and_grid_steps_are_written_with_their_unit` n'appelle pas `page_words` | low | patch | Quatre cas de `page_words` contrôlés. |
| 6 | Chiffres de la doc (140, 369, 4,6 %) non vérifiés | false | rejet | Relancé après correctifs : 140 gardés, 369 redemandes, aucune famille écartée. |

## Design Notes

Formats écrits : « au format A3 », « en A3 », « sur une feuille A0 », « A2 paysage » ; orientations : « en portrait », « à la française » (portrait), « à l'italienne » (paysage), « paysage » écrit → `landscape`. Carroyage : « avec un carroyage Lambert-93 tous les 1 km », « quadrillage UTM 31N de 1 000 m », « graticule de 0,5° », « carroyage géographique tous les 10 grades ». Valeur décimale écrite à la française (« 0,01° ») et recopiée en `0.01`.

## Verification

**Commands:**
- `cargo test -p cassini-training` -- expected: tous verts, nouveaux tests compris
- `cargo run -p cassini-training --release` -- expected: 8 000 exemples, aucune famille rejetée, redemandes entre 4 et 7 %
- `cargo clippy -p cassini-training -p cassini-core --all-targets -- -D warnings && cargo fmt --check` -- expected: aucun avertissement
