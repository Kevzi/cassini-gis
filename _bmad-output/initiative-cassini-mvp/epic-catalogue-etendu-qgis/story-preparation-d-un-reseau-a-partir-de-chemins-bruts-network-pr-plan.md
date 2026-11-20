---
title: "Préparation d'un réseau à partir de chemins bruts (network.prepare)"
type: 'feature'
ticket: '14'
created: '2026-11-20'
status: 'built'
baseline_revision: 'd4bcb6140fe295a717f626a9c67cd67ecebe775a'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Une couche de chemins bruts (extrémités disjointes, croisements sans nœud, doublons) donne à `analysis.isochrone` et `network.shortest_path` un résultat incomplet ou `analysis.origin_off_network`.

**Approach:** Ajouter `network.prepare`, déléguée à une chaîne fixe d'algorithmes `native:` seuls, sans GRASS (AD-24, AD-28) : `native:snapgeometries` (accrochage des extrémités), `native:splitwithlines` de la couche sur elle-même (découpe aux croisements), puis suppression des géométries nulles et en double. Elle rend une couche de lignes que les deux commandes de réseau lisent sans autre traitement.

## Boundaries & Constraints

**Always:** Paramètres `layer_id` (lignes), `tolerance` (`Distance`, facultatif, défaut 1 m, lu par le noyau), `out` (fourni par le noyau, `.gpkg`), `work_epsg` (rempli par le noyau, décision du 2026-11-12 : une tolérance en mètres sur une couche en degrés se calcule dans `work_epsg`, comme `analysis.buffer`) ; la commande déclare `work_crs_param` ; chaîne de version 1 ; annulable, donc tous ses algorithmes dans `CANCELLABLE_ALGORITHMS` ; résultat `layer_id`, `name`, `crs`, `feature_count` ; couche d'entrée non linéaire refusée avant lancement ; aucun mode de modification sur place, l'original n'est jamais touché ; un échec ne laisse aucun fichier ; les noms de paramètres de `QgsProcessing`, l'algorithme de suppression des géométries nulles et celui des doublons sont constatés par sonde sous QGIS 3.44, jamais supposés.

**Never:** Algorithme GRASS ou `qgis:` hors `native:` ; modifier `network_options.rs`, `Point`, `analysis.isochrone`, `network.shortest_path` ; mode « sur place » ; téléchargement d'un réseau ; famille d'exemples pour le modèle (réentraînement du ticket 17) ; `SummaryRow`.

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Nominal | chemins bruts synthétiques, extrémités disjointes à moins de la tolérance, un croisement sans nœud, un doublon | couche de lignes accrochée, découpée aux croisements, sans doublon ; isochrone et plus court chemin y réussissent | aucune |
| Avant préparation | même couche brute | isochrone ou plus court chemin incomplet ou `analysis.origin_off_network` | (témoin) |
| Couche en degrés | WGS84 | accrochage dans `work_epsg`, résultat dans le système de la couche | aucune |
| Tolérance trop petite | écart supérieur à la tolérance | extrémités non reliées, couche rendue telle quelle | aucune |
| Couche non linéaire | polygones ou points | refus avant lancement, aucun fichier écrit | `registry.invalid_params` ou erreur nommée de géométrie |
| Paramètre inconnu ou manquant | `layer_id` absent, champ inconnu | refus à la validation | `registry.invalid_params` |
| Chaînage | `layer_id` = `step:k.layer_id`, sortie passée à `analysis.isochrone` | accepté par le contrôle de plan | aucune |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_dissolve.rs` -- modèle de déclaration (RawParams `deny_unknown_fields`, `Delegated`, `.runs_in(RunsIn::Qgis)`, `.cancellable`, `.output_params(&["out"])`, `.output_ext(".gpkg")`, exemple, tests) ; nouveau `network_prepare.rs`, enregistrement automatique par `core/build.rs`
- `core/src/commands/network_shortest_path.rs` (~40, 95-125) -- `work_epsg: Option<u32>` et `.work_crs_param("work_epsg", "layer_id")` à copier ; `analysis_buffer.rs:14-38` -- `Distance`/`DistanceUnit` (m, km), accesseur en mètres ; défaut 1 m écrit dans la nouvelle commande, pas dans `network_options.rs` (défaut 100 m, à ne pas toucher)
- `core/src/journal/tests.rs`, `core/tests/catalogue.rs`, `core/tests/conformite_resultats.rs`, `fixtures/resultats-extension.json` -- bras simulé avec `work_epsg`, rejeu, exemple de résultat, fiche
- Catalogue des commandes (`_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md`, ligne 385 et section 2237-2258) -- `planifiee` devient `faite`, ajouter `work_epsg` rempli par le noyau si les autres fiches le listent
- `qgis-plugin/cassini_qgis/analysis.py` -- table `ALGORITHMS` (~786, commentaire des clés 783-868), modèles : `analysis.area` (~1067, `_REPROJECT_TO_WORK` puis algorithme, `"geometry"`), `network.shortest_path` (~1110, `"geometry": "Line"`, `on_failure`), `translate_difference` (~101, étape à deux entrées) ; nouvelle ligne `network.prepare` (chaîne 1, entrée `Line`) ; étape `native:splitwithlines` avec la couche pour `LINES` ; ajouter à `CANCELLABLE_ALGORITHMS` (~1207) les algorithmes manquants ; `validate_table` interdit une garde sur la première étape
- `qgis-plugin/cassini_qgis/summary.py`, `texts.py` -- résumé FR/EN « Réseau préparé : N tronçons »
- `qgis-plugin/tests/` -- `test_analysis.py`, `test_chain.py`, `test_result_conformance.py`, `test_summary.py`, `table_annulable.json` (dérivé de `derived_cancellable`), nouveau `probe_prepare.py` (modèle `probe_shortest_path.py` : `expect`, `memory_network`) ; lancement par `python-qgis-ltr.bat`
- `ai/tests/par_commande/phrases.json`, `training/src/barres.rs` (compte), `docs/couverture-phrases-reelles.md:38` (« aucune phrase ne demande la préparation » devient faux), `README.md`, `qgis-plugin/README.md` -- phrase « prépare le réseau de chemins avant l'isochrone »
- Aucun refus `NO_...` de préparation dans `training/src/families.rs` ; à ne pas toucher : `network_options.rs`, `point.rs`, `analysis_isochrone.rs`, `network_shortest_path.rs`, `_REPROJECT_TO_WORK`

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/network_prepare.rs` -- commande, schéma, exemples (dont `step:k.layer_id`), tests (déléguée, `layer_id` requis, défaut 1 m, `work_epsg`, sortie `.gpkg`, annulable)
- [ ] `core/src/journal/tests.rs`, `core/tests/`, `fixtures/resultats-extension.json` -- bras simulé, rejeu, exemple de résultat, fiche
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` -- chaîne de quatre ou cinq étapes, garde de géométrie, tolérance en mètres, résumé FR/EN
- [ ] `qgis-plugin/tests/probe_prepare.py` -- sonde QGIS 3.44 : noms de paramètres, algorithmes de nettoyage, avant/après sur couche brute avec isochrone et plus court chemin, couche en degrés, échec sans fichier ; constats assertés et consignés
- [ ] `qgis-plugin/tests/` unitaires, conformité, `table_annulable.json` -- chaîne, annulation, isochrone et plus court chemin inchangés
- [ ] Catalogue, phrases, `barres.rs`, `docs/couverture-phrases-reelles.md`, READMEs -- fiche `faite`, phrase de plan valide

**Acceptance Criteria:**
- Given une couche de chemins brute synthétique aux extrémités disjointes, when `network.prepare` s'exécute puis une isochrone et un plus court chemin, then ils rendent le résultat attendu, alors qu'ils étaient incomplets ou refusés avant préparation.
- Given une demande de préparation suivie d'une isochrone, when l'agent compose, then le plan est valide.
- Given un ancien journal, when on le rejoue, then le résultat est inchangé ; la commande n'est annoncée que si sa chaîne existe (10.4).

## Implementation Notes

Ajout hors plan : `default_param` sur `CommandSpec` (`core/src/registry.rs`, appliqué dans `core/src/journal/mod.rs`) pour que le noyau écrive la tolérance de 1 m au journal. Phrases PRE1 à PRE3 (trois exigées par `par_commande_reel.rs`). Sonde VERDICT OK sous QGIS 3.44 : brut = `no_path` et aire nulle, préparé = 5 tronçons, 200 m / 144 s, isochrone 10 000 m².

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 1 low corrigé, 4 rejetés ; deux points signalés au propriétaire.

| Verdict | Route | Constat et preuve |
|---|---|---|
| low | patch | Catalogue : la ligne `work_epsg` était dans la table d'`analysis.intersect` ; déplacée dans celle de `network.prepare`, que la fiche JSON listait déjà |
| medium | rejeté | `default_param` (registre et journal) absent du Code Map : le plan exige un défaut de 1 m lu par le noyau et aucun mécanisme n'existait ; l'écart est consigné, non un défaut ; relecture `thorough` demandée par CLAUDE.md §8 non rendue par ce build (seule la lentille `quick` existe), signalée au propriétaire |
| low | rejeté | Pas de test d'un ancien journal : une commande nouvelle n'a pas d'ancien journal, et la branche `default_param` est sautée pour toute commande qui n'en déclare pas |
| low | rejeté | Sonde : témoins larges (`no_path` ou aire nulle) conformes au critère « incomplet ou refusé » ; l'écart non relié est tenu par l'étape `no_path` |
| false | rejeté | Dédoublonnage après le retour en degrés : une même transformation appliquée à des coordonnées identiques rend des coordonnées identiques, l'égalité topologique tient |

## Design Notes

Décision prise faute de réponse dans le ticket : `work_epsg` ajouté aux paramètres (note du 2026-11-12, sans changement de cap supplémentaire). Le choix de l'algorithme de suppression des géométries nulles (`native:removenullgeometries`) et des doublons (`native:deleteduplicategeometries`) est laissé à la sonde.

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_prepare.py` -- expected: VERDICT OK
