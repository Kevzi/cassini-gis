---
title: 'Différence de couches (analysis.difference)'
type: 'feature'
ticket: '21'
created: '2026-11-20'
status: 'built'
baseline_revision: '1544729cb88f0a5f18d6b0dc9087311f4c23aad6'
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

**Problem:** Aucune commande ne retranche une couche d'une autre : « les parcelles hors EBC », « hors zones humides », « parcelles moins un tampon du bâti » ne se composent pas, alors que l'intersection existe.

**Approach:** Ajouter `analysis.difference`, déléguée à `native:difference` (chaîne fixe, AD-24), calquée sur `analysis.intersect` : deux couches vecteur différentes, sortie en couche nouvelle, originaux intacts.

## Boundaries & Constraints

**Always:** Paramètres `layer_id`, `overlay_layer_id`, `out` (fourni par le noyau, `.gpkg`) ; couches différentes et vecteur, vérifiées par l'extension avant tout lancement ; résultat `layer_id`, `name`, `crs`, `feature_count` ; chaîne de version 1 ; annulable ; un recouvrement total rend une couche vide (0 entité) ajoutée au projet et nommée dans le résumé ; ce que QGIS fait des champs de la couche de superposition, des types de géométrie mixtes et de `fid` est constaté par sonde sous QGIS 3.44, jamais supposé.

**Never:** `work_crs_param` ; `SummaryRow` ; paramètre `GRID_SIZE` exposé ; modification d'`analysis.intersect` ; famille d'exemples pour le modèle (réentraînement du ticket 17) ; tout nom d'algorithme dans le noyau (AD-22).

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Nominal | deux carrés qui se recouvrent en partie | couche d'un entité : le premier carré moins la partie commune | aucune |
| Recouvrement total | le masque couvre toute la couche | couche vide, ajoutée, nommée dans le résumé (« 0 entité ») | aucune |
| Aucun recouvrement | masque disjoint | couche identique à l'entrée | aucune |
| Même couche | `layer_id` = `overlay_layer_id` | refus avant lancement | `analysis.bad_params` |
| Couche absente ou raster | identifiant inconnu ou raster | refus avant lancement | `layer.not_found` / `layer.not_vector` |
| Paramètre inconnu ou manquant | `expression`, ou un seul id | refus à la validation | `registry.invalid_params` |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_intersect.rs` -- modèle exact (Delegated, `.runs_in(Qgis).cancellable(true).output_params(&["out"]).output_ext(".gpkg")`, `inventory::submit!`, trois tests) ; nouveau `analysis_difference.rs`, enregistré là où l'est `analysis_intersect` (`core/src/commands/mod.rs`)
- `core/src/journal/tests.rs` (~2032-2110) -- bras simulé et test de rejeu de l'intersection, à imiter
- `core/tests/conformite_resultats.rs`, `fixtures/resultats-extension.json` -- un exemple de résultat (le test échoue sans)
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md:369,1752-1770` -- `planifiée`/`planifiee` → `faite`/`faite` ; params déjà conformes ; `core/tests/catalogue.rs` compare
- `qgis-plugin/cassini_qgis/analysis.py` (`translate_intersect` 87, entrée ALGORITHMS 773, `CANCELLABLE_ALGORITHMS` ~1035, refus « deux couches différentes » ~1841) -- `translate_difference` (`INPUT`, `GRID_SIZE` laissé au défaut), ligne de table avec `overlay_key: "OVERLAY"`, `native:difference` dans l'ensemble annulable
- `qgis-plugin/cassini_qgis/summary.py` (`_intersect` 215, table ~1069), `texts.py` (~115, FR et EN) -- « Différence de Parcelles et Zone N : nouvelle couche X, 0 entité »
- `qgis-plugin/tests/` -- `test_analysis.py` (`IntersectTest` 457), `test_summary.py` (89, SAMPLES ~1005), `test_result_conformance.py` (140, 451), `table_annulable.json`, nouveau `probe_difference.py` (sur le modèle de `probe_dissolve.py`)
- `ai/tests/par_commande/phrases.json` (modèle INT1, ligne 70), `training/src/barres.rs:54` (compte de phrases ; échoue sinon) -- phrase « Les parcelles hors zone N du PLU » (sélection `typezone eq N`, puis `step:1.layer_id`) ; sans casser les phrases de `analysis.intersect`
- `README.md`, `qgis-plugin/README.md`
- À ne pas toucher : `analysis_intersect.rs`, `summary_row.rs`, `work_crs`, `families.rs`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_difference.rs`, `mod.rs` -- commande déclarée, deux exemples, trois tests (déléguée, deux couches requises, sortie `.gpkg` nommée par le noyau) -- contrat du registre
- [ ] `core/src/journal/tests.rs`, `fixtures/resultats-extension.json` -- bras simulé, rejeu, exemple de résultat -- journal rejouable
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` -- chaîne `native:difference`, annulable, résumé FR/EN
- [ ] `qgis-plugin/tests/probe_difference.py` -- sonde QGIS 3.44 : recouvrement partiel, total (couche vide ajoutée), disjoint, champs conservés, `fid`, géométries multiparties ; constats assertés et consignés dans Implementation Notes
- [ ] `qgis-plugin/tests/test_analysis.py`, `test_summary.py`, `test_result_conformance.py`, `table_annulable.json` -- tests de la passerelle (calqués sur `IntersectTest`), du résumé (dont 0 entité), de la conformité
- [ ] Catalogue, `phrases.json`, `barres.rs`, READMEs -- fiche `faite`, phrase « hors zone N », compte de `barres.rs`

**Acceptance Criteria:**
- Given deux carrés qui se recouvrent, when `analysis.difference` s'exécute, then la couche nouvelle contient la partie attendue du premier carré et les originaux sont intacts.
- Given un masque qui couvre tout, when elle s'exécute, then la couche vide est nommée dans le résumé.
- Given « les parcelles hors zone N » et l'inventaire de test, when l'agent compose, then le plan est valide.
- Given un ancien journal, when on le rejoue, then le résultat est inchangé ; la commande n'est annoncée que si sa chaîne existe (10.4).

## Implementation Notes

Sonde `probe_difference.py` sous QGIS 3.44.15 (VERDICT : OK), constats : la sortie ne garde que les champs de l'entrée (plus `fid` du GeoPackage, renumérotée 1..n), aucun champ de la couche retranchée ; recouvrement partiel : partie commune retranchée, entité non touchée intacte ; recouvrement total : couche vide (0 entité), sans erreur, ajoutée au projet ; masque disjoint : entrées recopiées ; multiparties conservées ; couche de lignes retranchée d'un polygone : aucun effet (géométries mixtes acceptées) ; `GRID_SIZE` laissé au défaut.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 1 low corrigé, 1 low déféré, 3 rejetés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| low | patch | README de l'extension, étape 4 de la recette de `analysis.difference` : un octet retour arrière (0x08) à la place de `\b` rend le chemin `15<BS>in` ; corrigé, relu dans le diff |
| low | defer | Deux autres chemins du même README portent le même octet, antérieurs à ce ticket (2 dans HEAD) |
| low | rejeté | Test de journal : le bras simulé rend toujours 0 entité ; même motif que le test de `analysis.dissolve`, et ajouter un cas non vide ajoute un bras sans rien prouver de plus sur le rejeu |
| false | rejeté | « Constats de la sonde non vérifiables » : `probe_difference.py` relancé par la revue, VERDICT : OK |
| false | rejeté | Phrases DIF2 et DIF3 en plus de DIF1 : exigées par `every_offered_command_has_its_sentences` (3 par commande) ; les identifiants de l'inventaire existent |

## Design Notes

`native:difference` : INPUT, OVERLAY, OUTPUT, GRID_SIZE (défaut). Les champs de sortie sont ceux de INPUT seuls (contrairement à `native:intersection`, aucun champ de OVERLAY) : à confirmer par la sonde. Le résumé ne voit que le nom et le nombre d'entités de sortie, ce qui suffit à nommer une couche vide.

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_difference.py` -- expected: VERDICT OK
