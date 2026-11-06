---
title: 'Un seul type de raster : RasterPath partout'
type: 'refactor'
ticket: '5'
created: '2026-11-06'
status: 'built'
baseline_revision: 'cc60a2a262bebdd792f228d5773b992a9ba90c27'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** les paramètres raster (`dem` de `terrain.slope`, `terrain.viewshed`, `terrain.line_of_sight`, `path` de `layer.open_raster`) sont des chaînes nues. Un plan « pente puis champ de vision sur la pente » ne peut citer la sortie que par `step:k.file`, que le noyau résout en chemin : le journal garde ce chemin du dossier de travail, et le rejeu relit l'ancien fichier au lieu de celui qu'il vient de recalculer (AD-24 amendé, AD-11).

**Approach:** poser le type partagé `RasterPath` (chaîne marquée `x-cassini: raster`), y migrer les quatre paramètres ; dans un plan, une référence `step:k` sur un `RasterPath` désigne le raster écrit par l'étape k et devient, avant l'appel, le repère `out:<étape du journal>.1`, que le journal garde tel quel et que le rejeu résout déjà vers son propre dossier.

## Boundaries & Constraints

**Always:** `RasterPath` défini une fois dans le noyau, marqué dans le schéma engendré comme `Point` et `MgrsRef` ; fiches des quatre commandes : `planned_params` reportés dans `params` (type `RasterPath`), `planned_by` ôté, dans le même commit ; à la vérification d'un plan, `step:k` sur un `RasterPath` n'est admis que si l'étape k écrit exactement un raster (sinon `plan.type_mismatch`, message en français qui nomme l'étape) ; le numéro du repère est celui de l'étape au journal, pas celui du plan (les ouvertures dictées s'intercalent) ; un chemin relatif au projet se comporte comme aujourd'hui ; tests existants du terrain et du rejeu verts.

**Never:** aucun changement de l'entraînement (entrées 12 et 16) ; pas de recette de la table fermée (entrée 17) ; pas de garde de provenance sur `RasterPath` ; aucune commande nouvelle ; aucune dépendance nouvelle ; `analysis.zonal_statistics` (`raster` planifié par l'epic 10) inchangé.

**Decisions (propriétaire, 2026-11-06):** `step:k.<champ>` sur un `RasterPath` est refusé (`plan.bad_reference`, « désignez le raster par step:k ») : c'est la forme qui journalisait un chemin. Le long prompt montré au modèle reste inchangé : un `RasterPath` y est rendu `"type": "string"` avec sa description. Le panneau garde un champ texte pour `dem`. Le plan est gardé entier (~2 600 tokens).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Chemin du projet | `terrain.slope {dem: "mnt.tif"}` | comme avant ; journal `dem: "mnt.tif"` + entrée `inputs` | — |
| Chaîne par référence | plan `slope {dem: mnt.tif}` puis `viewshed {dem: "step:1", …}`, ouverture dictée active | journal : 1 slope, 2 `layer.open_raster {path: out:1.1}`, 3 viewshed `dem: "out:1.1"` | — |
| Sans ouverture dictée | même plan, extension sans `layer.open_raster` | viewshed journalisé `dem: "out:1.1"` | — |
| Plan après d'autres appels | deux appels déjà journalisés, puis le plan ci-dessus | viewshed porte `out:3.1` | — |
| Rejeu | journal ci-dessus rejoué | viewshed lit la pente recalculée sous `replay/` ; résultats identiques | — |
| Étape sans raster | `dem: "step:1"` où l'étape 1 est `analysis.buffer` | plan refusé avant d'être montré | `plan.type_mismatch` |
| Forme champ | `dem: "step:1.file"` | plan refusé | `plan.bad_reference` |
| Repère hors plan | `command.run terrain.slope {dem: "out:9.1"}` sans étape 9 | — | `replay.bad_reference` (inchangé ; corrigé par le propriétaire le 2026-11-06, le plan portait par erreur `journal.bad_reference`) |

</frozen-after-approval>

## Code Map

- `core/src/point.rs` -- modèle : `MgrsRef` (newtype, `#[schemars(extend("x-cassini" = "mgrs"))]`), `MARK_KEY`, `is_marked`/`is_mgrs`. Créer `core/src/raster_path.rs` (`RasterPath(pub String)`, `RASTER_MARK = "raster"`, `is_raster_path(schema, root)` réutilisant `resolve`/`is_marked`, à rendre `pub(crate)`) et le déclarer dans `lib.rs`.
- `core/src/commands/terrain_slope.rs` l. 41, `terrain_viewshed.rs`, `terrain_line_of_sight.rs` l. 121, `layer_open_raster.rs` l. 23 -- `String` → `RasterPath` ; garder les doc-commentaires des champs (description vue par le modèle) ; `.path_params(..)` inchangés ; adapter les lectures (`&params.dem.0`).
- `core/src/plan.rs` `check_plan` l. 173-250 -- avant la comparaison de types : si `param_schema` est un `RasterPath`, `step:k` admis ssi la commande de l'étape k a `output_params.len() == 1` et `output_ext` dans les extensions raster (`RASTER_EXTENSIONS`, `journal/mod.rs`), `step:k.<champ>` refusé. Ajouter une fonction pure `raster_markers(spec, params, journal_steps: &[usize]) -> Value` qui remplace ces `step:k` par `out:{journal_steps[k-1]}.1`, appliquée **avant** `resolve_references`.
- `core/src/serve/plans.rs` `execute` l. 291-334 -- relever `session.next_step()` (`journal/mod.rs` l. 216) avant chaque `call_in_plan`, appeler `raster_markers` puis `resolve_references`.
- `core/src/journal/mod.rs` `resolve_param` l. 643-670, `resolve_reference`, `reference_file` -- rendent déjà `out:k.n` (relatif au journal, absolu sous `replay/` au rejeu) : ne pas modifier.
- `ai/src/plan.rs` `catalog_entry` l. 237-278 -- remplacer tout `{"$ref": "#/$defs/RasterPath", ..}` par `{"type": "string", ..}` et laisser l'élagage existant ôter la définition ; test : le `dem` montré au modèle est identique à l'avant (type et description).
- `qgis-plugin/cassini_qgis/form.py` `_kind` l. 50 -- suivre un `$ref` local vers un `$defs` de type chaîne simple → `TEXT` (racine du schéma à passer) ; test dans `qgis-plugin/tests/test_form.py`.
- `core/tests/catalogue.rs` -- `RasterPath` déjà dans `NAMED_TYPES` ; rien à changer, la recette confronte.
- `_bmad-output/.../spec-cassini-mvp/catalogue-des-commandes.md` l. 50-57, 420-428, 625-710 -- fiches `layer.open_raster`, `terrain.slope`, `terrain.viewshed`, `terrain.line_of_sight` ; exemple du §2 ; phrase « Posé par l'entrée 5 » de `RasterPath` → posé.
- `training/` -- ne pas modifier.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/raster_path.rs`, `core/src/lib.rs` -- type, marque, détecteur, tests de schéma (marque présente, désérialisation d'une chaîne).
- [x] `core/src/commands/terrain_*.rs`, `layer_open_raster.rs` -- migration des quatre paramètres.
- [x] `core/src/plan.rs` -- vérification et `raster_markers`, tests unitaires de la matrice (refus sans raster, forme champ, numéros du journal).
- [x] `core/src/serve/plans.rs` -- branchement dans `execute` ; test de session : plan pente → champ de vision, lignes du journal attendues, puis rejeu identique.
- [x] `ai/src/plan.rs` -- prompt inchangé, avec son test.
- [x] `qgis-plugin/cassini_qgis/form.py`, `qgis-plugin/tests/test_form.py` -- champ texte conservé.
- [x] `catalogue-des-commandes.md` -- fiches reportées.

**Acceptance Criteria:**
- Given le noyau sans QGIS, when `cargo test --workspace`, then tout passe, dont `catalogue` avec les quatre fiches en `RasterPath`.
- Given un plan pente puis champ de vision par `step:1`, when on l'exécute puis rejoue le journal, then aucune ligne ne porte de chemin du dossier de travail en paramètre raster et le rejeu rend les mêmes résultats typés.
- Given le registre, when on construit le long prompt avant et après, then le texte du catalogue montré au modèle est identique.

## Implementation Notes

- Implémenté directement (le sous-agent d'implémentation n'a pas pu être lancé : un hook modifie l'appel, le mode auto ne le vérifie pas).
- `core/src/raster_path.rs` porte aussi `RASTER_EXTENSIONS` (repris de `journal/mod.rs`) et `writes_one_raster` ; `point::is_marked` passé en `pub(crate)`.
- Le rejeu compare les octets de chaque sortie au fichier d'origine : le test de session ne peut donc pas effacer la pente d'origine pour prouver la lecture sous `replay/` ; cette résolution est déjà couverte par les tests du journal (`out:1.1` → dossier du rejeu). Le test prouve le repère journalisé et le rejeu identique.
- `core/tests/catalogue.rs` : l'attendu du test `a_retyped_parameter_fails_with_expected_and_found` devient « RasterPath ».
- Exemple du §2 du catalogue : mis à jour (fiche `terrain.slope` livrée, sans `planned_*`).
- Écart à la matrice : la dernière ligne nomme `journal.bad_reference`, le code existant (inchangé) est `replay.bad_reference` ; réponse du propriétaire : garder le code existant, matrice corrigée.
- Après relecture : quatre patchs (voir le journal de tri), vérification relancée verte.
- Vérifié sous Windows (`CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH` vers `outils-cassini/libclang`) : `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test --workspace` verts ; `python -I -m unittest discover -s qgis-plugin/tests` : 409 tests verts.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture thorough, 4 lentilles) — high 0, medium 1, low 6, false 9, maybe-false 0 ; 4 patchs, 2 reports, le reste rejeté.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| `is_raster_path` ne suit pas `anyOf` : un futur `Option<RasterPath>` (`raster` d'`analysis.zonal_statistics`, epic 10) recevrait la sortie entière ou journaliserait un chemin | medium | patch | `raster_path.rs` passe par `point::admits` (alternatives suivies) ; test `mask: Option<RasterPath>` |
| Imports `raster_path` avant `point` dans deux commandes | low | patch | ordre corrigé (`cargo fmt` ne réordonnait pas) |
| `inline_raster_paths` ne parcourt pas `$defs` | low | patch | parcours de tout le schéma ; aucune définition actuelle ne cite `RasterPath` |
| Lignes `dem` du catalogue (viewshed, line_of_sight) et usage direct de `out:k.n` non documentés ; description de `RasterPath` vue au MCP sans « dans un plan » | low | patch | catalogue précisé ; description « ou `step:k` dans un plan » |
| Le modèle n'apprend pas `step:k` sur un raster ; `step:k.file` désormais refusé | low | defer | aucun exemple ni règle n'écrit `step:k.file` (lentille verification-gap) ; l'entraînement est hors du ticket (Never), entrées 12 et 16 |
| Une pente consommée par `step:k` reste visible dans l'inventaire du modèle (`intermediate_files` ne suit que les `layer_id`) | low | defer | comportement antérieur inchangé (le chemin résolu n'était pas un `layer_id` non plus) |
| `step:k` hors plan (`command.run`, MCP) donne une erreur de fichier, non typée en référence | false | rejeté | `Dem::open` rend `dem.unreadable`, erreur nommée (AD-21) ; description précisée |
| `out:k.1` vers un `.gpkg` passé en `dem` | low | rejeté | appel direct seulement, `dem.unsupported`/`dem.unreadable` nommée ; garde = branche nouvelle |
| `writes_one_raster` exige `.tif` : un raster délégué futur | low | rejeté | aucun délégué n'écrit de raster aujourd'hui ; l'entrée qui en livre un déclarera `output_ext` |
| `raster_markers` suppose la vérification préalable | false | rejeté | `execute` appelle `verify` (donc `check_plan`) avant toute étape |
| Messages minuscule / majuscule | false | rejeté | même usage que les messages voisins de `check_plan` |
| `inputs: []` sur un repère | false | rejeté | comportement établi au ticket 4.9 pour `out:k.n` ; la ligne amont porte l'empreinte |
| Référence en avant sur un `RasterPath` non testée | false | rejeté | le test `k >= step` précède la nouvelle branche (`plan.rs` l. 224) |
| `RasterPath` en tableau | false | rejeté | les références ne valent qu'au premier niveau (convention de `plan.rs`) |
| `_follow` ne suit pas `allOf` | false | rejeté | schemars rend `$ref` + `description` côte à côte (test `ai` le confirme) |
| Le formulaire suit aussi `MgrsRef` → champ texte | false | rejeté | amélioration : la saisie JSON entre guillemets disparaît |
| Le test de rejeu ne prouve pas quel fichier est lu ; test du prompt non textuel ; test placé hors de `test_form.py` | false | rejeté | résolution sous `replay/` couverte par les tests du journal ; serde_json sans `preserve_order` (ordre trié) ; correction = éditer le plan |

## Design Notes

Pourquoi le numéro du journal : `out:k.n` est lu par `reference_file` sur les ids des lignes du journal ; or une étape de plan qui écrit un raster est suivie d'une ligne `layer.open_raster` dictée, et un plan peut suivre d'autres appels. L'étape 2 du plan n'est donc pas la ligne 2 : la table `journal_steps` (ligne du journal de chaque étape du plan) fait la traduction, une seule fois, dans le noyau.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement (sous Windows : `CARGO_TARGET_DIR=C:\ct`, `LIBCLANG_PATH` posé)
- `cargo test --workspace` -- expected: tout vert
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
