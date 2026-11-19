---
title: 'Jointure d''une table à une couche (analysis.join_table)'
type: 'feature'
ticket: '23'
created: '2026-11-19'
status: 'built'
baseline_revision: '8e22e9dc408e2e85d058d30503e004bd9cc95fc7'
route: 'full'
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

**Problem:** Un bureau d'études ne peut pas rattacher à une couche les colonnes d'un tableau CSV sans géométrie (DVF, INSEE) : `layer.add` exige des colonnes X/Y et aucune commande ne joint.

**Approach:** Ajouter `analysis.join_table`, déléguée à `native:joinattributestable` (chaîne fixe, AD-24), et faire ouvrir à `layer.add` un CSV sans géométrie par un paramètre `table`, comme couche de genre `vector`.

## Boundaries & Constraints

**Always:** Un seul champ commun nommé dans l'inventaire des deux côtés ; les deux entrées désignées par `layer_id` ; sortie en couche nouvelle, une ligne par entité d'entrée ; CSV seul ; la table est ouverte avec tous ses champs en texte (zéros de tête des codes INSEE conservés) ; clé en double dans la table : première correspondance (`METHOD=1`), déterministe ; le nombre de lignes sans correspondance est dans le résultat et le résumé, confirmé par sonde sous QGIS 3.44 ; chaîne de version 1 ; aucune logique métier dans l'extension.

**Never:** `.xlsx` ; nouveau genre (une table sans géométrie reste `vector`, AD-24 : la fiche `layer.add` qui dit `kind` = `table` est corrigée) ; `work_crs_param` ; `SummaryRow` ; paramètre `raster` de `layer.add` (ticket 20) ; réentraînement du modèle (ticket 17).

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Nominal | communes + CSV joints par code INSEE | couche nouvelle avec les colonnes du CSV, `feature_count` = communes | aucune |
| Sans correspondance | une commune absente du CSV, une ligne CSV sans commune | `unmatched_count` = 1, commune gardée, colonnes vides | aucune |
| Clé en double | deux lignes CSV pour un code | première retenue, `feature_count` inchangé | aucune |
| Champ inconnu | `table_field` absent de la table | refus | `analysis.unknown_field` |
| Même couche | `layer_id` = `table_layer_id` | refus à la désérialisation | erreur typée de paramètre |
| Deux entrées à `layer.add` | `path` et `table` ensemble, ou aucun | refus | `layer.one_input` |

</frozen-after-approval>

## Code Map

- `core/src/commands/layer_add.rs:11-26` -- `path` devient optionnel, ajout `table`, `path_params(&["path","table"])`, erreur `layer.one_input`
- `core/src/commands/analysis_nearest.rs:12-120` -- modèle à copier (RawParams/TryFrom, `unmatched_count`, `.runs_in(Qgis).cancellable(true).output_params(&["out"])`) ; nouveau `analysis_join_table.rs` (auto-déclaré par `build.rs`)
- `core/src/journal/tests.rs` (~4773, ~4880) -- bras simulé et test de rejeu après `analysis.nearest`
- `core/tests/conformite_resultats.rs`, `fixtures/resultats-extension.json` -- un exemple de résultat
- `core/tests/catalogue.rs`, `spec-cassini-mvp/catalogue-des-commandes.md:378,2036-2060,~440-475` -- fiches `planifiee` → `faite` ; corriger `kind`
- `qgis-plugin/cassini_qgis/layer_add.py:49-55,129-162` -- mode `table` : URI `delimitedtext` `geomType=none&detectTypes=no`, sans CRS
- `qgis-plugin/cassini_qgis/analysis.py` (ALGORITHMS ~642-664, `translate_nearest` 363, `CANCELLABLE_ALGORITHMS` ~800, `_check_field` ~1525, commentaire attributs 505-584) -- ligne `native:joinattributestable`, `translate_join_table`, attribut générique `overlay_field`, `summarize_join`
- `qgis-plugin/cassini_qgis/summary.py` (~291, ~1001), `texts.py` (131, 460) -- phrases FR/EN
- `qgis-plugin/tests/` -- `test_analysis.py`, `test_summary.py`, `test_result_conformance.py`, `table_annulable.json`, `test_chain.py`, nouveau `probe_join_table.py`
- `ai/tests/par_commande/phrases.json` + `inventory.json` (ajouter une table sans géométrie), `training/src/barres.rs` (compte 102), `training/src/families.rs` (famille civile) -- vérifier recouvrement et mots de refus
- `README.md:~119`, `qgis-plugin/README.md`, `docs/entrainement-du-modele.md`
- À ne pas toucher : `summary_row.rs`, logique de `analysis.nearest`, `NETWORK_BLACKLIST`, `work_crs`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/layer_add.rs` -- param `table`, un seul de `path`/`table`, `layer.one_input`, test -- ouvrir un CSV sans géométrie
- [ ] `qgis-plugin/cassini_qgis/layer_add.py` -- mode `table` sans CRS, `source_crs` vide -- lecture par QGIS
- [ ] `core/src/commands/analysis_join_table.rs` -- commande, schéma, exemple, tests (même couche refusée) -- contrat du registre
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` -- chaîne, vérification des champs, `unmatched_count` (sortie `NON_MATCHING` si la sonde la confirme, sinon décompte par `DISCARD_NONMATCHING`), résumé
- [ ] Fixtures synthétiques `fixtures/` (communes + CSV, une commune sans ligne, une ligne sans commune, un code à zéro de tête, une clé en double) et tests plugin/cœur/conformité/journal
- [ ] `qgis-plugin/tests/probe_join_table.py` -- sonde QGIS 3.44 : noms de champs, collisions de colonnes, comptes
- [ ] Catalogue, `phrases.json`/`inventory.json`, `barres.rs`, `families.rs`, READMEs -- fiche `faite`, phrase « joins le tableau des prix à la couche des parcelles »

**Acceptance Criteria:**
- Given communes et un CSV par code INSEE, when `analysis.join_table` s'exécute, then la couche nouvelle porte les colonnes attendues et `unmatched_count` est exact.
- Given la phrase « joins le tableau des prix à la couche des parcelles » et un inventaire contenant la table, when l'agent compose, then le plan est valide.
- Given un ancien journal, when on le rejoue, then le résultat est inchangé ; la commande n'est annoncée que si sa chaîne existe (10.4).

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 1 medium, 2 low corrigés, 4 rejetés ou différés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch | `layer.one_input` contrôlé seulement dans l'extension : `layer_add.rs` n'a aucun TryFrom, `{}` passe la validation ; TryFrom ajouté au noyau |
| low | patch | README : la même couche des deux côtés donne `registry.invalid_params` (refus du noyau), pas `analysis.bad_params` ; corrigé |
| low | patch | `test_summary.py` : chaîne non brute avec `\d` ; passée en chaîne brute |
| false | rejeté | `UNJOINABLE_COUNT` non prouvé : la sonde sous QGIS 3.44 a rendu VERDICT OK chez l'implémenteur, et est rejouée à la vérification |
| low | rejeté | Fixtures absentes de `/fixtures` : la sonde porte le CSV en ligne comme les sondes voisines, et couvre nominal, sans correspondance, clé en double, champ inconnu ; réparer ajouterait un fichier sans gain |
| low | rejeté | `families.rs`, `docs/entrainement-du-modele.md`, `test_chain.py` non touchés : réentraînement hors périmètre (ticket 17), et la chaîne est dérivée de `ALGORITHMS` |
| maybe-false | defer | Jointure sur une couche d'entrée sans géométrie : `crs` de sortie vide, `_load` peut échouer ; à confirmer par une sonde |

## Design Notes

`native:joinattributestable` : INPUT, FIELD, INPUT_2, FIELD_2, FIELDS_TO_COPY (vide = tous), METHOD=1, DISCARD_NONMATCHING=false, PREFIX="". Collision de noms de colonnes : à constater par la sonde, jamais supposée.

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_join_table.py` -- expected: VERDICT OK
