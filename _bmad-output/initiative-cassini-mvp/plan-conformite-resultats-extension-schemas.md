---
title: 'Conformité des résultats de l''extension aux schémas de sortie du noyau'
type: 'feature'
ticket: ''
created: '2026-11-19'
status: 'built'
baseline_revision: 'c1d6e9fbf139785a06710987b92651aaf420c032'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/fixtures/README.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** depuis 10.1, le noyau rejette en `qgis.bad_result` tout résultat de l'extension qui ne suit pas le schéma de sortie de sa commande ; or aucun test ne confronte ce que l'extension produit réellement à ces schémas (seules des fixtures écrites à la main le font, et l'une avait déjà dérivé). Une dérive Python/Rust ferait échouer une commande en production sans qu'aucun test ne casse.

**Approach:** un fichier d'exemples de résultat partagé dans `/fixtures`, un par commande déléguée réelle (plusieurs quand une option fait varier la forme), lu des deux côtés sur le précédent de `style-cases.json` : le noyau les fait valider par `Registry::validate_output`, l'extension prouve qu'elle produit cette forme. Les deux côtés exigent un exemple pour chaque commande déléguée.

## Boundaries & Constraints

**Always:** le schéma de sortie du noyau fait foi : un écart se corrige côté extension (producteur Python), avec un test de non-régression ; exemples synthétiques écrits à la main, noms inventés, aucune donnée réelle (règle de `fixtures/README.md`, une ligne ajoutée à son tableau) ; l'ensemble des ids du fichier égale celui des commandes déléguées réelles (hors préfixe `test.`), dans les deux langages ; un exemple refusé doit l'être pour la bonne raison (un cas négatif prouve que le test détecte).

**Never:** modifier les schémas de sortie, les fiches du catalogue ou les commandes du noyau ; assouplir un schéma pour faire passer un exemple (si le schéma lui-même paraît faux, s'arrêter et le signaler) ; lancer QGIS dans les tests ; `analysis.distance_matrix` et les entrées 10.3, 10.11 ; toucher à `.claude/worktrees` ni à `_bmad-output/party-mode/`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Exemple conforme | un exemple par commande déléguée réelle | accepté par `validate_output` | — |
| Commande sans exemple | une commande déléguée absente du fichier | le test échoue, nomme l'id | test rouge |
| Exemple d'une commande inconnue | id absent du registre | le test échoue, nomme l'id | test rouge |
| Variantes de forme | `layout.export_pdf` (sans échelle, avec échelle et centre, avec avertissements, tronqué), `layer.add` (reprojetée ou non), `layer.describe` (sans emprise), `project.inventory` (projet vide), `analysis.zonal_statistics` (tronqué, sans zone), `data.convert_raster` (sans EPSG) | chaque variante acceptée | — |
| Exemple mal formé | champ requis absent, champ en trop, type faux | refusé en `qgis.bad_result` | cas négatif du test |
| Forme réellement produite | le geste Python rejoué avec son faux backend | résultat égal à l'exemple, ou même forme (clés et types JSON) quand le faux backend ne peut pas le produire | écart corrigé côté extension |

</frozen-after-approval>

## Code Map

- `fixtures/resultats-extension.json` -- à créer : `[{id, case, result}]` ; modèle : `fixtures/style-cases.json` (« lu des deux côtés »).
- `fixtures/README.md` -- ajouter la ligne du fichier (synthétique, écrit à la main).
- `core/tests/conformite_resultats.rs` -- à créer : lit `../fixtures` (`CARGO_MANIFEST_DIR`), `Registry::standard().validate_output` ; ids attendus : `registry.specs()` avec `runs_in == Qgis`, hors `test.`. Précédent : `validate_output_accepts_the_already_built_delegated_commands` (`core/src/registry.rs` ~l. 767).
- `qgis-plugin/tests/test_result_conformance.py` -- à créer : lit le même fichier ; ids attendus via `delegated_ids`/`read_fiches` de `test_delegated.py` ; garde-fou anti-lecture vide.
- Producteurs à rejouer : `analysis.py` (`_load`, `_written`, `ALGORITHMS`), `layer_add.py`, `layer_remove.py`, `raster_output.py`, `vector_output.py`, `style_gestures.py`, `inventory.py` (`_read`, `LayerDescribeGesture`), `point_transform.py`, `layout_export.py` (l. ~1189-1199). Résultats réalistes à reprendre : `test_analysis.py` (l. 187, 695, 839, 968, 1114), `test_layout_export.py` l. 149, `test_inventory.py` l. 116.
- Écarts possibles, non confirmés : `SourceDetail` (`resolution`, `date`, `date_origin` nuls) de `layout.export_pdf` ; `crs` vide de `layer.describe` ; bloc lon/lat de l'emprise de `project.inventory` ; `epsg` de `data.convert_raster` et `data.transform_point`.

## Tasks & Acceptance

**Execution:**
- [ ] `fixtures/resultats-extension.json`, `fixtures/README.md` -- un exemple par commande déléguée réelle et ses variantes -- partagé des deux côtés
- [ ] `core/tests/conformite_resultats.rs` -- ids exigés, exemples acceptés, cas négatifs refusés -- côté noyau
- [ ] `qgis-plugin/tests/test_result_conformance.py` -- ids exigés, forme produite par chaque geste avec son faux backend -- côté extension
- [ ] producteurs Python concernés -- corriger tout écart révélé, avec test de non-régression -- le schéma du noyau fait foi

**Acceptance Criteria:**
- Given une nouvelle commande déléguée sans exemple, when on lance les tests Rust ou Python, then les deux échouent en nommant son id.
- Given `cargo test --workspace` et les tests Python, when on les lance, then ils réussissent.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick). Aucune boucle : pas d'intent_gap ni de bad_plan.

| Constat | Verdict | Route / preuve |
|---|---|---|
| Variantes `truncated`, `reprojected` comparées par forme (booléen), un producteur inversé passerait | medium | patch : le faux backend reproduit la valeur, comparer les booléens par valeur côté Python ; la présence de `scale`/`center`/`warnings` est déjà tenue par l'égalité des clés |
| Tests négatifs « commande sans exemple » et « id inconnu » tautologiques (n'exercent pas la comparaison des ids) | medium | patch : extraire la comparaison des ensembles d'ids dans une fonction appelée par le test réel et par les tests négatifs, dans les deux langages |
| Le test Rust des champs requis lit `schema[\"required\"]` : une racine en `$ref` ne serait pas contrôlée sans bruit | low | patch : échouer franchement si la racine n'est pas un objet en ligne |
| Champ en trop refusé seulement pour `Point` et `Distance` côté Rust | rejeté | les schémas de sortie sont ouverts par construction (aucun `deny_unknown_fields`), et le plan interdit de les modifier ; l'égalité des clés côté Python tient ce garde-fou |
| Négatifs sur les champs requis imbriqués | low, rejeté | la correction ajoute une récursion sur `$defs` sans cas démontré ; `source_details` est couvert |
| Liste comparée à n'importe quel modèle d'élément ; bornes `maxLength` non exercées | low, rejeté | le schéma Rust valide l'exemple ; la longueur du producteur sort du plan |
| Un seul exemple `data.convert` | false | `format` est une valeur, pas une variante de forme : `{file, format, feature_count}` pour tous les formats |
| `date_origin` : enum Rust, simple chaîne côté Python | low, rejeté | le Rust valide l'enum de l'exemple ; un producteur fautif sort du cas démontré |
| Tâche 4 sans trace visible, Implementation Notes vide | rejeté | le correctif éditerait le plan ; aucun écart n'a été révélé, donc aucun producteur changé |

## Design Notes

Un seul fichier JSON lu des deux côtés : si Rust et Python divergent, c'est le fichier qui les départage, et le schéma du noyau reste la référence.

## Verification

**Commands:**
- `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test -p cassini-core --test conformite_resultats'` -- expected: vert
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
