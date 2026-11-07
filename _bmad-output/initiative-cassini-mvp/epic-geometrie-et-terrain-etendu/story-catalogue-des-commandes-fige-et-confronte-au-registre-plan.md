---
title: 'Catalogue des commandes figé et confronté au registre'
type: 'feature'
ticket: '1'
created: '2026-11-05'
status: done
baseline_revision: 'c57edf3e23e2de9e2f4b5b1b2fc898835b17cc15'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 1
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-geometrie-et-terrain-avant-la-demo/change-geometrie-et-terrain-avant-la-demo.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** les identifiants et paramètres des commandes changent au fil des tickets, ce qui oblige à reprendre le générateur d'exemples et à ré-entraîner le modèle ; aucune liste unique ne fixe ce qui est fait, planifié et réservé.

**Approach:** écrire `catalogue-des-commandes.md`, lisible par un humain et par une recette : une fiche par commande (faite, planifiée ou réservée) avec un bloc JSON normalisé ; une recette du noyau compare chaque commande du registre à sa fiche et échoue au moindre écart (AD-6, amendement du 2026-11-05).

## Boundaries & Constraints

**Always:** une fiche par commande, identifiant `domaine.verbe` (Consistency Conventions) ; chaque fiche dit `status` (`faite`, `planifiee`, `reservee`), `runs_in` (`core`, `qgis`), `dictated_by_core`, les paramètres (nom, type, obligatoire, unité ou valeurs permises, sens en une ligne) et les champs du résultat ; les types partagés `Point`, `MgrsRef`, `Angle`, `RasterPath` sont définis une fois en tête ; les fiches planifiées reprennent les règles d'AD-19, AD-20, AD-23, AD-24, AD-30, AD-31 et les identifiants de l'epic 10 tels qu'écrits dans son `tickets.toml` (`analysis.count_points_in_polygon` remplace `analysis.zonal_statistics`) ; tout seuil est un paramètre (spec, Constraints) ; texte en français, noms en anglais.

**Never:** aucune commande hors des listes du changement de cap §4.1, de l'epic 10 et des commandes déjà au registre ; aucun terme ni paramètre propre à un usage particulier (classes de véhicules, millièmes) ; pas de modification d'une commande existante ni du registre dans ce ticket ; pas de nouvelle dépendance ; les commandes `test.*` (compilées pour les tests) ne figurent pas au catalogue.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Registre conforme | registre actuel, catalogue écrit | la recette passe | — |
| Paramètre changé | un paramètre d'une commande faite renommé ou retypé dans le code, catalogue inchangé | la recette échoue | message : id, paramètre, attendu, trouvé |
| Commande absente du catalogue | commande enregistrée sans fiche | la recette échoue | nomme l'id |
| Fiche faite sans commande | fiche `faite` absente du registre | la recette échoue | nomme l'id |
| Planifiée déjà enregistrée | fiche `planifiee` dont l'id est au registre | la recette échoue | demande de passer la fiche en `faite` |
| Fiche mal formée | bloc JSON illisible ou champ manquant | la recette échoue | nomme la fiche et le champ |

</frozen-after-approval>

## Code Map

- `core/src/registry.rs` -- `CommandSpec` (id, runs_in, dictated_by_core, params_schema, output_schema engendrés par `schemars`), `Registry::standard()` et `specs()` : source de vérité côté code. Ne pas modifier.
- `core/src/contract.rs` -- `check_contract` : modèle de contrôle du registre réel par un test ; `is_point` reconnaît le type `Point` dans un schéma (via `$ref`/`$defs`).
- `core/src/point.rs` -- type `Point` partagé (x, y, epsg) ; référence pour la définition du catalogue.
- `core/src/commands/*.rs` -- 15 commandes faites (`analysis.*`, `layer.*`, `layout.export_pdf`, `model.list`, `project.inventory`, `system.about`, `terrain.*`) ; `test_*.rs` exclues.
- `core/tests/` -- tests d'intégration du noyau ; la nouvelle recette s'y ajoute (`catalogue.rs`), lancée par `cargo test` et donc par la CI.
- `_bmad-output/initiative-cassini-mvp/epic-catalogue-etendu-qgis/tickets.toml` -- identifiants de l'epic 10 à recopier.
- `_bmad-output/initiative-cassini-mvp/change-geometrie-et-terrain-avant-la-demo/...md` §4.1 -- commandes de l'epic 11 et réservées.

## Tasks & Acceptance

**Execution:**
- [x] `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- écrire l'introduction (règle de gel, statuts, types partagés), puis une fiche par commande faite (paramètres relevés dans les schémas engendrés), planifiée (epic 11, epic 10) et réservée -- c'est le contrat figé.
- [x] `core/tests/catalogue.rs` -- lire les blocs JSON du catalogue, normaliser chaque `CommandSpec` du registre standard (noms de paramètres, obligatoires, type de premier niveau ou nom du type partagé, valeurs d'énumération, champs du résultat, runs_in, dictated_by_core) et comparer ; tester aussi les cas d'erreur de la matrice sur des catalogues fabriqués en mémoire -- la recette d'AD-6.
- [x] `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/spec-cassini-mvp.md` -- retirer « (créé par l'entrée 1 de l'epic 11) » du companion.

**Acceptance Criteria:**
- Given le catalogue écrit, when on lance `cargo test -p cassini-core --test catalogue`, then la recette passe sur le registre actuel.
- Given le catalogue, when le propriétaire lit la liste des identifiants, then il l'approuve ou demande des changements avant le commit.
- Given une fiche planifiée de l'epic 11, when on la lit, then on y trouve tout ce qu'il faut pour écrire la commande sans rien inventer (paramètres, unités, erreurs nommées des AD).

## Implementation Notes

- 2026-11-05 : le propriétaire a approuvé la liste des 52 identifiants et les choix par défaut signalés par l'implémenteur (critère d'acceptation 2). Écarts de l'implémenteur, acceptés : types nommés `Distance`, `Height`, `Duration`, `NetworkOptions` en plus des quatre types partagés ; champ `items` comparé ; champs `planned_params`/`planned_by` sur les fiches faites, vérifiés dans leur forme seulement.

- 2026-11-05 : le propriétaire garde les deux ajouts de la boucle 1 sur `layer.add` : champ de résultat `kind` (`vector`, `raster`, `table`) et erreur `layer.one_input`.

## Plan Change Log

- Boucle 1 (2026-11-05), relecture quick. Constats : normalisation de surface (types nommés non comparés à leur définition, enum des éléments de liste perdu, union étiquetée illisible, champs des objets non figés) ; pas de champ pour un changement de résultat décidé ; `layer.add` sans ses évolutions décidées ; pas d'erreur pour une référence MGRS mal formée ; comparaison d'enum dépendante de l'ordre. Amendé : Design Notes (normalisation en profondeur, `planned_result`, union étiquetée). État évité : une recette qui laisse dériver les types partagés et les paramètres composés, et un catalogue figé qu'on devrait éditer hors changement de cap à 10.15. KEEP : le catalogue approuvé par le propriétaire (52 identifiants, choix par défaut, formes des fiches) et la structure de `core/tests/catalogue.rs` (lecture, confrontation, 14 tests de la matrice) restent ; on les amende, on ne les réécrit pas. Écart au workflow, au su du propriétaire : le code n'est pas annulé avant la reprise, pour garder le catalogue approuvé.

## Review Triage Log

Boucle 1 — high 0, medium 6, low 2, false 1, maybe-false 0.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Types nommés réduits à leur nom, contenu jamais comparé | medium | bad_plan | `normalize_type` rend le nom du `$defs` sans le lire ; §1 promet l'échec sur un changement de valeurs permises |
| Enum des éléments d'un tableau perdu ; `coord.convert.to` sans enum | medium | bad_plan | l. 471-474 ne garde que le type ; la fiche n'a pas d'`enum` |
| Union étiquetée (`grid`) normalisée en `union`, refusée par `is_known_type` | medium | bad_plan | serde + schemars émettent `oneOf` ; la fiche `object` ne pourra jamais correspondre |
| Champs internes des paramètres objet non figés (`classes`, `criteria`, `grid`) | medium | bad_plan | comparés comme `object` |
| Pas de champ pour un résultat planifié (`skipped_layers` d'export_pdf, 10.15) | medium | bad_plan | `planned_params` couvre les seuls paramètres ; §1 interdit l'édition hors changement de cap |
| `layer.add` sans évolutions décidées (raster 10.20, table 10.23) ; raster par id contraire à AD-24 amendée | medium | bad_plan | fiche sans `planned_params` ; AD-24 amendée impose `RasterPath` |
| Pas d'erreur nommée pour une MgrsRef mal formée | low | bad_plan (traité avec la boucle) | seuls `geo.unsupported_crs`, `coord.one_input`, `coord.out_of_range` sont listés |
| Comparaison d'enum dépendante de l'ordre | low | bad_plan (traité avec la boucle) | l. 565 compare des `Vec` |
| Approbation du propriétaire non tracée | false | rejeté | consignée dans Implementation Notes le 2026-11-05, après la mise en scène du diff |

Boucle 1, seconde passe — high 0, medium 3, low 4, false 0, maybe-false 0 ; tout en patch.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| `Distance`/`Duration` dits ≥ 0 alors que le code exige > 0 | medium | patch | `deserialize_with = "positive"` (analysis_buffer.rs, analysis_isochrone.rs) |
| Contraintes sans erreur nommée (hillshade, reclassify min ≥ max, EPSG du carroyage, points confondus) | medium | patch | fiches de l'epic 11 sans code d'erreur pour ces cas |
| Règle de gel incomplète (`planned_result`, retrait de `analysis.zonal_statistics`, `required` qui change) | medium | patch | §1 ne cite que deux gestes |
| `SummaryRow` hors des types nommés de la recette | low | patch | absent de `NAMED_TYPES`, pas de bloc `json type` |
| `read_ty` accepte `items` en chaîne `object`/`array` et `array` sans `items` | low | patch | l. 506-543 |
| Exemple du §2 différent de la fiche `terrain.slope` | low | patch | `out` absent ; la partie qui touche les Design Notes du plan est rejetée (édition du plan) |
| Tableau §4 non confronté aux fiches | low | patch | seconde copie id/statut/runs_in non lue par la recette |

## Design Notes

Bloc normalisé par fiche, ce que la recette compare (les fiches `planifiee` et `reservee` ne sont pas comparées au code, seulement validées dans leur forme ; une réservée peut n'avoir que `id`, `status` et `runs_in`) :

```json
{"id": "terrain.slope", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"dem": {"type": "string", "required": true}, "unit": {"type": "string", "required": false, "enum": ["degrees", "percent"]}},
 "result": ["file", "min", "max"]}
```

Normaliser un schéma : suivre un `$ref` vers `$defs` et garder le nom du type (`Point`) ; `anyOf` avec `null` = facultatif ; sinon le `type` JSON de premier niveau.

**Amendement de la boucle 1.** La normalisation va en profondeur : chaque type nommé (`Point`, `MgrsRef`, `Angle`, `RasterPath`, `Distance`, `Height`, `Duration`, `NetworkOptions`, et tout objet nommé d'un paramètre) a en §3 un bloc ` ```json type ` (champs, obligatoires, types, enum) comparé à son `$defs` quand il existe au code ; un tableau garde le type et l'enum de ses éléments ; un objet en ligne garde ses champs (même forme que `params`) ; une union étiquetée (`oneOf` d'objets portant un même champ `const`) se note `{"type": "union", "tag": "kind", "variants": {"<valeur>": {champs}}}` ; les enum se comparent comme des ensembles. Une fiche faite peut porter `planned_result` (champs ajoutés, `planned_by`), vérifié dans sa forme. Contenu à ajouter au catalogue sans toucher aux identifiants approuvés : enum de `coord.convert.to` (`wgs84`, `utm`, `mgrs`) ; champs de `classes`, `criteria`, `grid` (union par `kind`) ; `planned_result` de `layout.export_pdf` (`skipped_layers`, 10.15) ; `planned_params` de `layer.add` (rasters 10.20 par `RasterPath`, conformément à AD-24 amendée, et tables sans géométrie 10.23) ; erreur `coord.invalid_mgrs` pour une `MgrsRef` mal formée.

## Verification

**Commands:**
- `cargo test -p cassini-core --test catalogue` -- expected: tous les tests passent
- `cargo fmt --check && cargo clippy --all-targets -- -D warnings` -- expected: aucun avertissement
