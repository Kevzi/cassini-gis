---
title: 'Fusion par champ (analysis.dissolve)'
type: 'feature'
ticket: '24'
created: '2026-11-19'
status: 'built'
baseline_revision: '161b0bade9123d0cacb3d0f395e9263ae5ba3cff'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 1
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Aucune commande ne regroupe les entités d'une couche par un champ (parcelles par commune, zones par type) ; `analysis.buffer` ne fusionne que ses propres zones.

**Approach:** Ajouter `analysis.dissolve`, déléguée à `native:dissolve` (chaîne fixe, AD-24) : un à quatre champs de regroupement, sortie en couche nouvelle, aucun champ agrégé.

## Boundaries & Constraints

**Always:** Champs nommés dans l'inventaire, vérifiés un à un par l'extension (`analysis.unknown_field`, avant tout lancement) ; le noyau refuse à la désérialisation une liste vide, de plus de quatre noms, un nom vide ou en double ; `SEPARATE_DISJOINT=false` (une entité par groupe) ; toute couche vecteur acceptée, sans restriction de géométrie ; résultat `layer_id`, `name`, `crs`, `feature_count` ; chaîne de version 1 ; ce que QGIS fait des valeurs nulles, des attributs conservés et de `fid` est constaté par sonde sous QGIS 3.44, jamais supposé.

**Never:** `native:aggregate` (décision du 2026-11-04) ; agrégation de champs ; `work_crs_param` ; `SummaryRow` ; famille d'exemples pour le modèle (réentraînement du ticket 17) ; collision avec le paramètre `dissolve` de `analysis.buffer`.

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Nominal | 4 parcelles de 2 communes, champ code INSEE | 2 entités, surface de chaque = somme de ses parcelles | aucune |
| Deux champs | commune et zone | une entité par couple | aucune |
| Champ inconnu | un nom absent de la couche | refus avant lancement, aucun fichier écrit | `analysis.unknown_field` |
| Liste invalide | vide, plus de 4, doublon, nom vide | refus à la validation | `registry.invalid_params` |
| Valeurs nulles | champ avec des nulls | comportement constaté par la sonde, consigné | aucune |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_join_table.rs` -- modèle (RawParams/TryFrom, `.runs_in(Qgis).cancellable(true).output_params(&["out"]).output_ext(".gpkg")`, `inventory::submit!`) ; nouveau `analysis_dissolve.rs`, `fields: Vec<String>`
- `core/src/journal/tests.rs` (~4784, ~4930) -- bras simulé et test de rejeu
- `core/tests/conformite_resultats.rs`, `fixtures/resultats-extension.json` -- un exemple de résultat (le test échoue sans)
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md:379,2063-2080` -- `planifiee` → `faite`, params `layer_id`, `fields`, `out` déjà conformes ; `core/tests/catalogue.rs` compare
- `qgis-plugin/cassini_qgis/analysis.py` (`translate_join_table` ~421, ALGORITHMS ~702, `CANCELLABLE_ALGORITHMS` ~867, `_check_field` 1579, attributs ~540-585) -- `translate_dissolve`, ligne de table, attribut `fields` qui boucle `_check_field`, documenté
- `qgis-plugin/cassini_qgis/summary.py` (`_join_table` 315, `_out_layer` 189, table ~1022), `texts.py` (~135, ~466) -- « Parcelles fusionnées par commune : 2 entités »
- `qgis-plugin/tests/` -- `test_analysis.py`, `test_summary.py` (SAMPLES ~987), `test_result_conformance.py` (~160/416), `table_annulable.json`, nouveau `probe_dissolve.py`
- `ai/tests/par_commande/phrases.json` (+inventory si besoin), `training/src/barres.rs:54` (compte 111 + phrases ajoutées ; échoue sinon) -- phrase « fusionne les parcelles par commune » (ajouter un champ `commune` à la couche de parcelles de `inventory.json` si cela ne casse aucune autre phrase ; sinon garder les phrases actuelles et le consigner), sans empiéter sur `analysis.buffer`
- `qgis-plugin/cassini_qgis/panel.py` (~1895-1912 `_build_form`), `form.py` (54-75 `build_fields`, 89-91 `is_field_field`) -- aujourd'hui un tableau de noms tombe dans un champ « JSON » brut ; il faut un sélecteur à choix multiples des champs de la couche choisie (règle « choisir, pas saisir »)
- `README.md:~121`, `qgis-plugin/README.md`
- À ne pas toucher : `summary_row.rs`, `analysis.buffer`, `work_crs`, `families.rs`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/analysis_dissolve.rs` -- commande, schéma, exemples, tests de validation de la liste -- contrat du registre
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` -- chaîne `native:dissolve`, `fields` vérifiés un à un, résumé FR/EN
- [ ] `qgis-plugin/cassini_qgis/panel.py`, `form.py` -- `fields` (tableau de noms de champs) rendu en sélecteur à choix multiples des champs de la couche `layer_id`, générique pour tout paramètre tableau de noms de champs, avec tests de logique du panneau ; le README n'invite plus à saisir du JSON
- [ ] `qgis-plugin/tests/probe_dissolve.py` -- sonde QGIS 3.44 : un puis deux champs, nulls, multiparties, attributs et `fid` conservés, surfaces ; les constats sur nulls, attributs conservés et `fid` sont des assertions (pas de simples affichages) et sont consignés dans Implementation Notes
- [ ] `qgis-plugin/tests/test_analysis.py` -- tests des refus de la passerelle : `fields` absent, non liste, vide
- [ ] Tests plugin, conformité, journal, catalogue, `barres.rs`, `phrases.json`, READMEs -- fiche `faite`

**Acceptance Criteria:**
- Given quatre parcelles sans recouvrement de deux communes, when `analysis.dissolve` s'exécute par code INSEE, then la couche nouvelle a deux entités dont la surface égale la somme des parcelles.
- Given « fusionne les parcelles par commune » et un inventaire adapté, when l'agent compose, then le plan est valide.
- Given un ancien journal, when on le rejoue, then le résultat est inchangé ; la commande n'est annoncée que si sa chaîne existe (10.4).

## Implementation Notes

Constats de `probe_dissolve.py` sous QGIS 3.44 (tous assertés, VERDICT OK) : les entités dont le champ est nul forment un groupe à part (une entité) ; les champs non groupés gardent la valeur de la première entité du groupe ; des parcelles disjointes d'un même groupe donnent une seule entité multipartie ; la colonne `fid` du GeoPackage de sortie est renumérotée (1, 2, 3), celle de l'entrée n'est pas reportée et c'est le seul champ ajouté. Sélecteur : `form.FIELD_LIST` (tableau de textes nommé `fields` ou `*_fields`) rendu par `FieldList` (cases à cocher des champs de la couche `layer_id`). Inventaire de test : champ `commune` ajouté aux parcelles ; phrase DIS1 « Fusionne les parcelles par commune. ».

## Plan Change Log

- 2026-11-19, boucle 1 : constat « `fields` sans sélecteur » (medium), plus la sonde sans assertions, la phrase absente et les refus non testés (low). Amendé : Code Map et tâches (sélecteur à choix multiples dans `panel.py`/`form.py`, assertions de sonde, test des refus, phrase par commune). État connu mauvais évité : un paramètre de champs saisi en JSON à la main. KEEP : toute la première passe, qui était juste et verte, sauvegardée comme référence dans `C:/Users/kevin/AppData/Local/Temp/claude/C--Users-kevin-Downloads-Projet-code-CASSINI/9df2dbb2-6291-4f3c-9fa2-1f15a56c9af6/scratchpad/keep-10-24-first-pass.patch` (`git apply` possible depuis la base) : `analysis_dissolve.rs` avec RawParams/TryFrom, `translate_dissolve` (`SEPARATE_DISJOINT=False`), attribut `fields_list` qui boucle `_check_field`, résumé FR/EN, fiche `faite`, `barres.rs`, sonde (VERDICT OK : nulls en un groupe, multiparties, surfaces, première valeur gardée, `fid` renumérotée). Code remis à la base avant la redérivation.

## Review Triage Log

Passe 1 (quick) : 1 medium bad_plan, 3 low repris dans la redérivation, 1 rejeté.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | bad_plan | `fields` (tableau) n'a aucun sélecteur : `form.py:is_field_field` ne reconnaît qu'un texte nommé `field`/`*_field`, le panneau donne un champ « JSON » brut ; viole « choisir, pas saisir » et le plan ne le demandait pas |
| low | bad_plan | La sonde affiche seulement nulls, attributs conservés et `fid` sans les asserter, et Implementation Notes est vide ; le plan voulait un constat consigné |
| low | bad_plan | Aucune phrase d'exemple « fusionne les parcelles par commune » : l'inventaire n'a pas de champ commune |
| low | bad_plan | Les refus de la passerelle (`fields` absent, non liste, vide) ne sont pas testés |
| low | rejeté | Bornes 1 à 4 sur le schéma : même motif que `analysis_join_table` (schemars lit la structure publique), et le noyau refuse par `TryFrom` ; ajouter un test n'apporte rien de plus |

Passe 2 (quick, après la boucle 1) : aucun constat retenu, 5 rejetés ; les 5 lignes de la passe 1 sont corrigées dans le code.

| Verdict | Route | Constat et preuve |
|---|---|---|
| low | rejeté | `_build_form` : un tableau de champs sans `layer_id` tombe sur un champ texte ; aucune commande n'en a, `analysis.dissolve` a `layer_id` ; la réparer ajoute une branche pour un cas inexistant |
| low | rejeté | `_is_field_list` reconnaît par le nom `fields`/`*_fields` ; convention du plan, un seul paramètre concerné, à revoir à la prochaine commande qui en porte un |
| low | rejeté | Le widget `FieldList` n'a pas de test Qt ; le plan demandait des tests de logique, la recette manuelle du README couvre le widget |
| false | rejeté | La passerelle ne vérifie pas doublons, plus de 4 noms, nom vide : le noyau les refuse par `TryFrom` avant tout geste, répartition voulue (AD-22) |
| low | rejeté | `_Unreadable` du résumé non testé : chemin d'erreur défensif, que le noyau rend inatteignable après validation |

## Design Notes

`native:dissolve` : INPUT, FIELD (liste), SEPARATE_DISJOINT=false, OUTPUT par défaut. Le résumé ne donne que le nombre de groupes en sortie : le résumé ne voit pas le nombre d'entités d'entrée.

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_dissolve.py` -- expected: VERDICT OK
