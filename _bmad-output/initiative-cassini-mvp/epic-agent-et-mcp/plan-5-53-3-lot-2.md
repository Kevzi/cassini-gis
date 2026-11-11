---
title: 'echelle_utm_reel et terrain_reel portées à 40-50 phrases écrites à la main, refus compris (livrable 3 de 5.53, lot 2)'
type: 'feature'
ticket: '' # livrable de 5.53, plan rattaché à story-barre-de-mesure-par-commande-phrases-ecrites-a-la-main-plan.md
created: '2026-11-11'
status: 'built'
baseline_revision: '9ad0716d1a7d3297a345181b029addf47163bc4a'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/plan-5-53-3-barres-elargies.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `echelle_utm_reel` et `terrain_reel` n'ont que 8 phrases : un raté pèse 12,5 %,
trop pour juger un oubli à une phrase près ; aucune n'attend de refus.

**Approach:** comme le lot 1, porter chaque barre à 40-50 phrases écrites à la main, dont au
moins 3 refus, sources existantes intactes, phrases protégées par `recouvrement`.

Décisions du propriétaire (2026-11-11, D1-D6 du lot 1, adaptées) :
- D1 : phrases neuves dans `ai/tests/echelle_utm_reel/phrases.json` et
  `ai/tests/terrain_reel/phrases.json`, schéma de `par_commande/phrases.json` sans `command`.
- D2 : élargies en place : chaque test garde sa constante `SENTENCES` et son `expected`
  intacts, lit en plus son JSON ; un seul `SCORE` sur le total.
- D3 : au moins 3 refus attendus par barre, chacun avec `missing` ; la barre par commande
  n'est pas touchée.
- D4 : seuils proportionnels, arrondis vers le bas : 7/8 rapporté au total
  (`common::proportional(total, 7, 8)`, 44 → 38).
- D5 : `ecart_reel` reste à 40.
- D6 : ce plan est le lot 2 ; le lot 3 (`epic11_reel`, `style_reel`, puis remesure du parent)
  reste dans `deferred-work.md`.

## Boundaries & Constraints

**Always:** vocabulaire civil (§2.9) ; phrases neuves, jamais engendrées ni recopiées d'une autre
barre ; chaque attendu passe `common::invalid` (dont garde-fou et complétion sans effet) ;
inventaire de l'épreuve tel quel ; aucune phrase ne nomme un fichier non ouvert ; le
propriétaire relit les phrases avant le commit ; une seule réponse défendable par phrase (correctifs du journal du lot 1 appliqués d'office).

**Never:** modifier `tests/epreuve-modele-local/`, le générateur (`training/src/`), la barre par
commande ou les barres des lots 1 et 3 ; envoyer une phrase de mesure au jeu d'entraînement ;
charger un modèle ou utiliser le GPU ; toucher `~/cassini-worktrees/reentrainement-5-49` ou
`CASSINI-nuit` ; pousser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Barre élargie | test sans modèle | vert, total 40-50, ≥ 3 refus | — |
| Attendu invalide | plan attendu qui échoue un contrôle | rouge, nommant l'id | — |
| Refus devenu faux | commande de `missing` ajoutée au registre | rouge, nommant l'id | — |
| Phrase engendrée ou recopiée | au jeu par défaut, ou dans deux barres | `recouvrement` rouge | identifiant seul |
| Sources gelées | `git diff` sur l'épreuve et les `SENTENCES` | vide | — |

</frozen-after-approval>

## Code Map

- `ai/tests/echelle_utm_reel.rs` -- `SENTENCES` (8), `expected`, seuil `right >= 7` ; pas de complétion (commandes QGIS seules) : la garder ainsi.
- `ai/tests/terrain_reel.rs` -- `SENTENCES` (8), `expected`, `common::completed`, `no_export_sentence_names_pdf` (préfixe `X`), `the_dem_is_the_source_of_the_inventory_raster` à garder.
- `ai/tests/plan_reel.rs:160-330` -- modèle à suivre : `epreuve()` (anciennes phrases mises au schéma JSON), `all_phrases`, `answers`, tests sans modèle (total, refus, `invalid`, `naming_unloaded_files`).
- `ai/tests/common/mod.rs` -- `read_phrases`, `invalid`, `same_calls`, `proportional`, `naming_unloaded_files` : réutiliser, ne pas modifier sauf manque prouvé.
- `training/tests/recouvrement.rs` -- `HAND_WRITTEN` (comptes), `real_trials_by_source`, somme de `the_real_trials_are_read_whole`.
- `core/src/point.rs:240,364` -- systèmes lus par le garde-fou : « Lambert-93 », « WGS 84 », « UTM 31N / 31 N / zone 31 Nord / UTM31N », code EPSG écrit. « fuseau 31 » ou « UTM 31 » sans hémisphère : question.
- Schémas : `terrain_slope.rs` (`unit` degrees|percent), `terrain_line_of_sight.rs` (`observer_height`, `target_height`, `curvature`, `refraction`), `terrain_viewshed.rs` (`radius` ≤ 2 500 m sur le MNT 5 m), `layout_export_pdf.rs` (`scale`, `page_size` a4…a0, `orientation`, `title`), `analysis_isochrone.rs` (`mode` walk|car, `duration` s|min).

## Tasks & Acceptance

**Execution:**
- [x] `ai/tests/echelle_utm_reel/phrases.json` -- ~36 phrases `EU01…` (total 40-50) : exports à une échelle seuls, sous toutes ses écritures, dont page, orientation ou titre ; exports au bout d'une sélection, d'un tampon ou d'une isochrone ; isochrones depuis un point UTM 31N ou 32N ; 4 refus (Design Notes).
- [x] `ai/tests/terrain_reel/phrases.json` -- ~36 phrases `TR01…` : pentes, lignes de vue, champs de vision (points en Lambert-93), exports à une échelle sans « PDF », dont après une étape de terrain ; 4 refus.
- [x] `ai/tests/echelle_utm_reel.rs`, `ai/tests/terrain_reel.rs` -- lire `SENTENCES` et le JSON, `answers` comme `plan_reel`, un seul `SCORE`, seuil D4, en-tête ; essai renommé, préfixe gardé ; tests sans modèle : `invalid` sur tout, total 40-50, ≥ 3 refus, aucun fichier non ouvert nommé, aucun export de `terrain_reel` ne dit « PDF » (par la commande attendue, pas l'id), notation (`answers` : titre, nombres, sortes) et seuil en valeurs fixes (38/44), jamais la formule recopiée ; `SCORE … réponses attendues`.
- [x] `training/tests/recouvrement.rs` -- deux entrées de plus dans `HAND_WRITTEN` (`echelle_utm_reel`, `terrain_reel`), sources Rust renommées `…:sentences`, comptes, somme, en-tête.
- [x] `docs/entrainement-du-modele.md` -- section « Barres élargies » : deux lignes de tableau, refus, seuils.
- [x] `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- entrée du lot 2 : `resolution`.

**Acceptance Criteria:**
- Given `cargo test -p cassini-ai --test echelle_utm_reel --test terrain_reel` sans modèle, then vert, 40-50 phrases chacune.
- Given `cargo test -p cassini-training --test recouvrement`, then vert (y compris contre le générateur actuel, livrable 4 compris).
- Given la recette locale complète, then verte.

## Design Notes

Doutes tranchés en panel (testeur, géomaticien, architecte, utilisateur occasionnel) :

- **Comparaison** : une seule règle, `common::same_calls`, pour les 8 anciennes et les neuves (titre libre non vide sauf titre attendu, entier écrit en flottant accepté) ; `comparable` local supprimé. Seul écart : `10.0` vaut `10` ; les totaux changeant, la barre rend déjà `non comparable`. Refus attendu = `AgentError::Refused` ; une question est un échec, sans plafond à part.
- **Refus** — mots-clés d'identifiants planifiés ou réservés du catalogue : atlas par commune (`atlas`, `layout.export_atlas`), itinéraire le plus court entre deux points UTM (`shortest_path`), temps de trajet entre deux points (`distance_matrix`), extension d'une crue sur le MNT (`flood`, `terrain.flood_extent`), accumulation d'écoulement (`flow_accum`, `terrain.flow_accumulation`) ; mots-clés anglais choisis, sans identifiant au catalogue : export en image PNG ou JPEG (`export_image`, `image`), cubature déblai/remblai (`cut_fill`, `volume`), rayonnement solaire annuel (`solar`, `insolation`). Écartés : isochrone en bus ou à vélo (valeur d'énumération, aucun mot-clé ne garde) ; « envoie à l'imprimante » (« imprime » vaut export depuis E04, X02).
- **Tournures retenues** (topographie civile) : échelle « 1:25 000 », « 1/25 000e », « au 25 000e », « échelle 1 : 2 500 », « 1:20k », une en toutes lettres (« au vingt-cinq millième ») ; jamais « 1 cm pour 250 m » (calcul, pas une forme apprise). Mise en page sans « PDF » : planche, mise en page, impression, tirage, édition. Terrain : déclivité, inclinaison, pente en % ou en degrés ; intervisibilité, visibilité réciproque, vue dégagée, masque du relief ; bassin visuel, zone d'influence visuelle, depuis un belvédère, un château d'eau, un mât. UTM : « E 457 200 / N 5 397 600 », nord écrit avant l'est (1-2), « EPSG:32631 », un relevé GPS « UTM 31N (WGS 84) ».
- **Pièges des attendus** : points de terrain en Lambert-93 seuls (un autre système ajoute une conversion, rouge à `invalid` ; c'est le rôle de `FOREIGN_POINTS`) ; jamais une valeur égale au défaut (œil 1,6 m, cible 0 m, réfraction 0,13, « à hauteur d'homme ») ; rayon ≤ 2 500 m ; durées en `min` (« une demi-heure », « un quart d'heure », « une heure » → 30, 15, 60) ; jamais « zone nord », « écoles », « hydrographie » (fichiers non ouverts) ; une phrase qui ne diffère d'une autre que par un nombre ou une commune est une copie pour `masked`.
- **Collision avec le générateur** : reformuler la phrase de barre (jamais mesurée), même attendu, générateur intact (précédent PL14, PL19).
- **Comptes visés** : 8 + 36 = 44 par barre, seuil 38/44 (86,4 %).

## Verification

Environnement : `LIBCLANG_PATH` vers un `libclang.so`, `BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`.

**Commands:**
- `cargo fmt --all --check && cargo clippy -p cassini-core -p cassini-ai -p cassini -p cassini-training --all-targets -- -D warnings` -- sans avertissement
- `cargo test -p cassini-core -p cassini-ai -p cassini -p cassini-training --locked` -- vert
- `python3 -I -m unittest discover -s training/entrainement/tests` -- vert
- `git diff 9ad0716 -- tests/epreuve-modele-local/ training/src/ ai/tests/par_commande/ ai/tests/plan_reel/ ai/tests/question_reel/` -- vide ; `SENTENCES` et `expected` des deux `.rs` inchangés

**Manual checks:**
- Un attendu faussé, un refus dont la commande existe : le test nomme la phrase.

## Implementation Notes

- `echelle_utm_reel` : 8 + 36 = 44 (32 plans neufs : 16 exports seuls, dont 3 titres exacts (un 4e
  au bout d'une sélection), pages a0-a3 et portrait, jamais une valeur par défaut écrite ;
  8 exports au bout d'une sélection, d'un tampon ou d'une isochrone ; 8 isochrones UTM 31N/32N).
  Refus EU33-EU36 : `atlas` ; `shortest_path` ; `export_image`, `image` ; `distance_matrix`.
  Seuil 7/8 du total, 38/44. Toujours sans complétion. Essai renommé
  `echelle_utm_reel_at_least_7_of_8_of_the_answers_match`.
- `terrain_reel` : 8 + 36 = 44 (7 pentes, 9 lignes de vue, 10 champs de vision ; 6 exports seuls
  sans « PDF », plus 3 en fin d'étape de terrain (9 neufs) ; points en Lambert-93 seuls).
  Refus TR33-TR36 : `flood` ; `flow_accum` ; `cut_fill`, `volume` ; `solar`, `insolation`.
  Seuil 38/44. Essai renommé `terrain_reel_at_least_7_of_8_of_the_answers_match`.
- Mots-clés des refus : `atlas`, `shortest_path`, `distance_matrix`, `flood`, `flow_accum` sont
  des morceaux d'identifiants planifiés ou réservés du catalogue ; `export_image`, `image`,
  `cut_fill`, `volume`, `solar`, `insolation` sont des mots-clés anglais choisis (aucun
  identifiant au catalogue). `itinerar` retiré (racine française, aucun identifiant visé).
- Écarts aux Design Notes (règle « une seule réponse défendable ») : TR33 parle de l'extension
  d'une crue de 2 m sur le MNT, sans cote (`raster.reclassify` défendable) ni rivière (absente de
  l'inventaire, une question défendable). TR10 et TR18 nomment l'observateur (TR18 : au sommet du
  mât), pour que la hauteur ne se lise pas en `target_height`. EU28 écrit « UTM 31N » seul.
  Distances des phrases neuves en mètres (EU21, TR19, TR23, TR24) : la phrase change, pas
  `common::same_calls` (plus petit changement).
- `comparable` local retiré des deux fichiers ; les tests `every_sentence_has_a_valid_expected_plan`,
  `every_expected_plan_passes_the_coordinate_guard`, `every_expected_plan_is_left_unchanged_by_the_completion`
  et `an_export_without_a_title_does_not_match` sont couverts par `every_expected_answer_is_valid`
  (`common::invalid`, avec un cas de complétion fautive TRW) et
  `the_score_compares_titles_numbers_and_kinds_as_documented`. `no_export_sentence_names_pdf`
  repère les exports par la commande attendue (11 sur 44).
- `SENTENCES` et `expected` intacts ; les en-têtes des deux fichiers sont réécrits et le
  commentaire de `export` corrigé.
- `common/mod.rs` inchangé. `recouvrement` vert contre le générateur actuel, sans collision ;
  `the_real_trial_sources_have_distinct_names` ajouté.
- L'entrée du lot 2 de `deferred-work.md` reste ouverte : elle se ferme au commit.
- À faire par le propriétaire avant le commit : relire les 72 phrases neuves.

## Plan Change Log

## Review Triage Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 0, medium 6, low 11, false 0, maybe-false 0. Corrigés 14 (patch, même implémenteur), rejetés 3.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Distances en km : réponse en m défendable (EU21, TR19, TR23, TR24) | medium | patch | phrases et attendus en mètres |
| TR33 suppose une rivière absente de l'inventaire | medium | patch | « Simule l'extension d'une crue de 2 m sur le MNT » |
| TR18 : hauteur lisible en observateur ou en cible | medium | patch | observateur nommé au sommet du mât |
| Mots-clés de refus présentés comme ids réservés ; `itinerar` sans portée | medium | patch | réservés et mots-clés anglais séparés, `itinerar` retiré |
| Un seul mot-clé essayé par refus | medium | patch | chaque mot-clé de chaque refus essayé |
| Noms de sources de `recouvrement` non gardés distincts | medium | patch | `the_real_trial_sources_have_distinct_names` |
| `terrain_reel` : compte d'erreurs absent, commentaire TR17 faux, complétion jamais mise en défaut | low | patch | `errors.len() == 7`, cas TRW |
| Titre blanc non testé dans `terrain_reel` | low | patch | ajouté |
| Plage 40-50 contre assertion figée à 44 | low | patch | seuil tiré de `proportional(total, 7, 8)` |
| EU28 « UTM 31N (WGS 84) » | low | patch | « UTM 31N » |
| Imports non uniformes ; commentaire de 126 caractères | low | patch | corrigés |
| Entrée `deferred-work` fermée avant le commit | low | patch | rouverte, fermée au commit |
| Notes et doc : comptes d'exports, note des en-têtes, colonne mal nommée | low | patch | corrigés |
| Code de test dupliqué entre barres | low | reject | chaque barre garde son essai ; une mise en commun irait au-delà du lot |
| Notation des 8 anciennes passée par `same_calls` (`10.0` = `10`) | low | reject | D2 garde données et attendus ; la notation commune est celle des lots 1 et 3 |
| Le seuil 38/44 non mesuré sur le modèle | low | reject | remesure prévue après le lot 3 (D6) |
| Relecture des phrases par le propriétaire | — | propriétaire | consigne du 2026-11-11 : doutes de tournure tranchés en relecture à plusieurs points de vue, pas en demandant ; aucun doute restant après la passe |
