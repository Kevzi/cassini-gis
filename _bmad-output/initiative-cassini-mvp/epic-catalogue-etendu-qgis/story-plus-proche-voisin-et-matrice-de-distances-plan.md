---
title: 'Plus proche voisin (analysis.nearest)'
type: 'feature'
ticket: '10'
created: '2026-11-18'
status: 'built'
baseline_revision: 'a0a3abf3c8e522728af66c68fe1001bcb156badc'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 1
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-distance-en-metres-sur-couche-en-degres/change-distance-en-metres-sur-couche-en-degres.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le catalogue annonce `analysis.nearest` (jointure au plus proche voisin) mais la commande n'existe pas. Sa sœur `analysis.distance_matrix` exige la sortie `table` (entrées 10.1 et 10.3, non construites).

**Approach:** ajouter `analysis.nearest` seule, sur le modèle d'`analysis.buffer` : déclaration dans le noyau, ligne de la table fermée de l'extension (`native:joinbynearest`), distance calculée dans le système `work_epsg` du noyau. `analysis.distance_matrix` est reportée (décision du propriétaire, 2026-11-18) jusqu'à 10.1 et 10.3.

## Boundaries & Constraints

**Always:** fiche du catalogue reprise telle quelle (params `layer_id`, `target_layer_id`, `max_distance` facultative, `out`; résultat `crs`, `distance_field`, `feature_count`, `layer_id`, `name`, `unit`, `unmatched_count`), seul son statut passe à `faite` ; `work_epsg` déclaré par `.work_crs_param` comme `analysis.buffer` ; distance du champ en mètres même sur couche en degrés ; aucun nom d'algorithme ni de champ QGIS dans le noyau (AD-22) ; `layer_id` ≠ `target_layer_id`, refusé par le noyau ; textes en français et en anglais ; un test par ligne de la matrice.

**Never:** `analysis.distance_matrix`, gabarit `SummaryRow`, sortie `table` ou chaîne fixe (10.1, 10.3) ; modifier le catalogue au-delà du statut ; ré-entraîner le modèle ; changer `analysis.buffer` ; toucher à `.claude/worktrees` ni à `_bmad-output/party-mode/`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Couche en mètres | points et cibles en Lambert-93 | chaque entité reçoit les attributs de sa cible la plus proche et la distance en m | — |
| Couche en degrés | `layer_id` en WGS84 | le noyau remplit `work_epsg`, la chaîne reprojette, distance en m | — |
| Limite | `max_distance` 500 m | entités sans cible à moins de 500 m gardées sans voisin ; `unmatched_count` les compte | — |
| Aucune limite | `max_distance` absente | `unmatched_count` = 0 | — |
| Même couche | `layer_id` = `target_layer_id` | refus | erreur du noyau, rien exécuté |
| Cible vide | couche cible sans entité | toutes les entités sans voisin | `unmatched_count` = `feature_count` |
| Annulation | arrêt demandé | chaîne interrompue, sortie retirée | comme `analysis.buffer` |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_buffer.rs` -- modèle à suivre : `Delegated`, `CommandSpec`, `work_crs_param`, `Distance`/`DistanceUnit` (l. 13-45), `inventory::submit!`. Rien d'autre à enregistrer (`mod.rs` inclut `build.rs`).
- `core/src/contract.rs` -- contrôle `.tests(...)` et `work_crs_param` ; `core/src/journal/mod.rs` `fill_work_crs` (l. 427-460) remplit seul `work_epsg`.
- `core/tests/catalogue.rs` -- confronte la commande à sa fiche ; `catalogue-des-commandes.md` (fiche l. 1943-1969, tableau l. 375).
- `qgis-plugin/cassini_qgis/analysis.py` -- `ALGORITHMS` (l. ~327-400), `translate_buffer` (l. 53), `_distance_in_meters` (l. 42), clé `work_crs`, `summary`.
- `qgis-plugin/cassini_qgis/texts.py`, `summary.py` (`_SENTENCES`, l. ~897) -- textes `sum_analysis_nearest…`.
- `qgis-plugin/tests/test_analysis.py` (`BufferTest`, fixtures synthétiques), `test_summary.py`, `test_delegated.py` (ids de l'extension = fiches `faite`).
- `training/src/families.rs`, `docs/entrainement-du-modele.md`, `ai/tests/par_commande/phrases.json`, `README.md` (l. 116-120) -- une famille de phrases, une ligne de doc.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_nearest.rs` -- commande déléguée, `max_distance: Option<Distance>`, `work_epsg`, refus de la même couche, exemple et noms de tests -- suit `analysis_buffer.rs`
- [ ] `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- statuts `analysis.nearest` en `faite` (fiche et tableau) -- seul changement permis
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- ligne `analysis.nearest` : `native:joinbynearest`, un voisin, limite en mètres, champ `distance`, sans entité écartée, comptage des non appariées
- [ ] `qgis-plugin/cassini_qgis/texts.py`, `summary.py` -- phrases de résumé fr et en
- [ ] `qgis-plugin/tests/test_analysis.py`, `test_summary.py`, `core/src/journal/tests.rs` -- une épreuve par ligne de la matrice ; `work_epsg` rempli pour nearest
- [ ] `training/src/families.rs`, `docs/entrainement-du-modele.md`, `ai/tests/par_commande/phrases.json`, `README.md` -- famille d'exemples, barre par commande, ligne de table
- [ ] `qgis-plugin/tests/probe_nearest.py` -- sonde QGIS réelle sur fixture synthétique (`native:joinbynearest`, couche en degrés)

**Acceptance Criteria:**
- Given une couche et une couche cible, when l'agent compose un plan avec `analysis.nearest`, then le plan est valide pour le noyau.
- Given `cargo test --workspace` et les tests Python, when on les lance, then ils réussissent, y compris `catalogue.rs` et `test_delegated.py`.

## Implementation Notes

## Plan Change Log

- 2026-11-18, boucle 1 (relecture quick). Déclencheurs : (a) le champ `distance` est codé en dur, or QGIS renomme le nouveau champ (`distance_2`) quand la couche d'entrée en porte déjà un, ou après un second `analysis.nearest` : `unmatched_count` et `distance_field` lisent alors l'ancien champ (constaté sous QGIS 3.44) ; (b) `native:joinbynearest` rend une ligne par voisin à égale distance : 1 entité, 3 cibles équidistantes, 3 lignes en sortie (constaté sous QGIS 3.44), ce qui casse « chaque entité reçoit son plus proche voisin » et `feature_count` = nombre d'entités de `layer_id`. Amendé : Design Notes (deux contraintes). État connu mauvais évité : nom de champ fixe ; sortie dédoublée. KEEP : déclaration du noyau `analysis_nearest.rs` et son journal de test (work_epsg), ligne de fiche du catalogue avec `work_epsg` (exigée par `catalogue.rs`), textes et résumés fr/en, famille `plus_proche_voisin` et phrases NEA1-3, poids `gros_projet` 152, barre 102, `FOCUS_EXCLUDED = ["redemande"]` (tirage GA06). Diff d'origine sauvegardé : `scratchpad/diff-10-10.patch`.

## Review Triage Log

| Constat | Verdict | Preuve |
|---|---|---|
| `distance` codé en dur, champ renommé en cas de conflit | medium → bad_plan | sonde QGIS : 2e jointure rend `distance_2`, couche avec `distance` rend `distance_2` ; le code lit l'ancien champ |
| Voisins à égale distance : plusieurs lignes par entité | medium → bad_plan | sonde QGIS : 1 entité, 3 cibles équidistantes, 3 lignes |
| Fiche du catalogue : `work_epsg` ajouté au-delà du statut | low, rejeté | exigé par `core/tests/catalogue.rs` ; le correctif serait d'éditer le plan |
| `FOCUS_EXCLUDED`/`gros_projet` hors liste de tâches ; doc « Vide depuis 5.54 » | low, rejeté | forcé par le déplacement du tirage ; le commentaire explique l'entrée 10.10 |
| `try_from` perd les descriptions du schéma | false | schemars 1.2.3 n'emprunte le type source que si `from` ET `into` sont posés (`schema_exprs.rs:60`) ; ici `try_from` seul |
| Sonde non consignée, `MAX_DISTANCE: None`, cas degrés avec cible d'un autre SCR | false | la sonde a été lancée (RÉSULTAT : OK) et couvre ces cas sous le vrai QGIS |
| Sonde : sortie vide et annulation | low, rejeté | couverts par les tests à backend factice ; correctif = complexité sans gain |

Boucle 2 (relecture quick) :

| Constat | Verdict | Preuve |
|---|---|---|
| Fiche du catalogue : `work_epsg` au-delà du statut | carried low, rejeté | même constat que la boucle 1 |
| Commentaires « vide depuis 5.54 » (`lib.rs`, `commande.rs`) devenus faux | low → patch | `FOCUS_EXCLUDED` contient `redemande` ; trois commentaires corrigés ; les lignes 830 et 838 de la doc d'entraînement décrivent l'état de 5.54 et restent vraies |
| Voisin retenu à égalité dépend de l'ordre de QGIS | false | le plan demande une ligne par entité, de façon répétable ; le rang 1 est stable sur mêmes données (sonde cas 6) |
| `_nearest_fields` : contrôle de la queue faible | false | un autre ordre de QGIS échoue en `analysis.failed`, bruyamment, sans chiffres faux |
| Sonde non lancée par `unittest`, cas non couverts | low, rejeté | couverts par le backend factice ; correctif = complexité sans gain |
| Cases du plan non cochées, Implementation Notes vide | rejeté | le correctif serait d'éditer le plan |

## Design Notes

Contraintes issues de la boucle 1 : (1) le nom du champ de distance doit être lu sur la couche produite, jamais supposé : QGIS le suffixe en cas de conflit ; `distance_field` et `unmatched_count` portent sur ce nom. (2) `native:joinbynearest` rend une ligne par voisin équidistant : la chaîne doit garantir une ligne par entité de `layer_id`, `feature_count` égal à leur nombre, sans choix arbitraire non déterministe.

`native:joinbynearest` mesure dans le système de la couche d'entrée : la chaîne ouvre par la reprojection vers `work_epsg` (comme le buffer), donc le champ `distance` est en mètres. À vérifier par la sonde : la couche cible est-elle transformée par l'algorithme ? Sinon, la reprojeter aussi.

## Verification

**Commands:**
- `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` -- expected: vert (première compilation longue : `C:\ct` a été vidé)
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert

**Manual checks (if no CLI):**
- Sonde `probe_nearest.py` sous QGIS : voisins et distances en mètres sur une couche en degrés.
