---
title: "Vue triée branchée à la composition, cache placé à la fin de l'index, redemande élargie"
type: 'feature'
ticket: '48'
created: '2026-11-10'
status: 'in-review'
baseline_revision: '12333c56b72b30fbd7275ea020f2c11736777163'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-inventaire-des-gros-projets/change-inventaire-des-gros-projets.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/story-vue-triee-de-l-inventaire-d-un-gros-projet-index-reduit-et-c-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la vue triée de 5.46 n'est branchée nulle part : un projet réel d'environ 60 couches rend encore `ai.prompt_too_long`. Mesurée sur l'inventaire réel (63 couches), elle détaille des couches sans rapport (« carte », « parcelle », « pdf » désignent par une valeur), « PLU » ne trouve pas « PLUi », et elle monte à 14 000 caractères quand l'entraînement ne voit aucun prompt au-delà de 12 900.

**Approach:** tous les chemins de composition (canal `plan.propose`, `cassini plan`, mesures de `ai/tests`) passent par la vue triée avec la demande et le dernier plan lu dans le journal ; le cache est coupé à chaque entrée de l'index ; une redemande élargie unique quand un plan prend un champ d'une couche montrée réduite ; règle de désignation corrigée de façon générique ; budget de la vue triée sous la borne de l'entraînement ; nouveau message de `ai.prompt_too_long` ; durées mesurées.

Décisions du propriétaire (2026-11-10) : durée < 30 s sous le seuil (inchangé), gros projets < 60 s sur CPU et < 30 s sur GPU ; seuil et plafonds de 46 revus ici sur les mesures ; l'exactitude des plans sur le gros projet n'est pas une barre de 48 (5.50), la durée et l'absence de `ai.prompt_too_long` le sont.

Décisions prises par l'orchestrateur sur délégation du propriétaire (2026-11-10, à confirmer par lui) :
1. **Budget de la vue triée 11 000 caractères** (au lieu de 14 000), `MAX_LENGTH` inchangé à 4 608 : 11 000 + consignes courtes (≈ 1 350) + demande tient sous `MAX_PROMPT_CHARS` = 12 900, donc le modèle ne voit jamais une vue plus longue que celles apprises ; chaque millier de caractères relu coûte ≈ 6 s sur CPU (barre de 60 s) ; porter `MAX_LENGTH` à 5 600 rouvrirait le risque de la carte pleine à l'entraînement. Sous le seuil, budget et vue inchangés (14 000).
2. **Désignation** : un mot de la demande compte une fois par couche, à sa meilleure place ; ordre de priorité des couches désignées : nombre de mots qui la désignent par son nom, puis nombre de mots qui la désignent tout court, puis par un nom de champ, puis l'ordre stable. Un **sigle** de la demande (mot en capitales d'au moins 3 signes dont 2 lettres) trouve aussi un mot qu'il commence, plus long d'au plus 2 signes (« PLU » → « PLUi »). Une couche qui ne tient pas au budget est **sautée**, la suivante est essayée.
3. Seuil 9 000, plafonds 30 champs / 3 000 caractères, index 7 000 / 3 500 : gardés sauf si les mesures de 48 disent autrement (chiffres consignés).

## Boundaries & Constraints

**Always:** sous le seuil, octet pour octet la vue d'aujourd'hui ; Rust sans dépendance nouvelle ; vue jamais journalisée, rejeu (AD-11) inchangé ; redemande = même demande, vue plus détaillée, plan rendu jugé par les mêmes schémas et le même garde-fou (AD-23), une seule fois ; `project.inventory` brut pour l'IA externe (AD-29) ; graines et poids du générateur inchangés, `forme_inventaire.rs` et `recouvrement.rs` verts ; données réelles hors dépôt.

**Never:** pousser, fusionner dans `main`, installer un noyau ou un modèle, lancer un entraînement ; changer de branche dans le dépôt principal ; question écrite par le noyau ; règle propre à la BD TOPO ou à une phrase ; toucher `SHORT_SYSTEM`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Champ d'une couche réduite | plan `select_by_attribute` `field` sur une couche de l'index seul | une seule recomposition, cette couche détaillée ; plan de la 2e composition gardé | 2e plan invalide → erreur ordinaire, pas de 3e |
| Géométrie seule | `buffer` sur une couche de l'index seul | aucune recomposition | — |
| Champ par `step:k` | `select_by_attribute` sur `step:1.layer_id`, étape 1 sur une couche réduite | recomposition, couche de l'étape 1 détaillée | — |
| Sous le seuil | petit projet | aucune redemande possible, vue d'aujourd'hui | — |
| Cache d'une vue triée | inventaire à `index` | une coupe par entrée de l'index, ancrage sur la dernière couche de l'utilisateur de l'index, rien coupé dans `layers` | forme inattendue → lu d'un tenant |
| Dernier plan | journal dont le dernier plan a produit `3-…gpkg` et cité `plu_<uuid>` | ces couches détaillées après les désignées | journal absent → `LastPlan` vide |
| Sigle | « zonage du PLU », couche dont une valeur porte `PLUI` | désignée | — |
| Budget | couche trop longue pour le reste du budget | sautée, la suivante essayée | — |
| Trop de couches | index seul > budget (≈ 150 couches) | `ai.prompt_too_long`, message « le projet a trop de couches pour le modèle » | — |
| MCP | `project.inventory` par le canal externe ou `command.run` | inventaire brut, sans `index` | — |

</frozen-after-approval>

## Code Map

- `core/src/agent_inventory/sorted_view.rs` -- prototype déjà écrit dans le worktree (non commité, à garder et finir) : `layer_score`/`Score` (décision 2), `Request::matched`/`value_words`/`sigles`, `SIGLE_SLACK`, `SORTED_BUDGET_CHARS` = 11 000 et `ViewLimits::sorted_budget`, `continue` au lieu de `break` dans le remplissage. Reste : entrée `wanted` (ids QGIS complets, détaillées en tout premier), entrée en forme apprise, `user_layers` d'une vue triée = couches de l'utilisateur en tête de l'**index** (c'est l'ancrage du cache), tests. `the_details_stop_at_the_first_layer_that_does_not_fit` devient « sautée, suivante essayée ».
- `core/src/agent_inventory.rs` -- `model_inventory` (l. 426) commence par `stable_order(agent_inventory(raw))` : en sortir une variante en forme apprise, sortie de `model_inventory` inchangée. Réexports et doc du module (« pas encore branchée » à retirer).
- `core/src/journal/mod.rs` -- `intermediate_files` (l. 797) : modèle de lecture ; nouvelle `last_plan(registry, journal) -> LastPlan` : lignes du dernier plan (depuis la dernière ligne `plan_step == 1`, lignes à `plan_step` seulement), `results` = fichiers de sortie (`output_file`) des étapes sans erreur, `cited` = valeurs texte de premier niveau des `params` et `layer_id` des objets.
- `core/src/serve/plans.rs` `propose` (l. 135) -- remplacer `model_inventory` par la vue triée (demande, `last_plan`), garder l'inventaire brut lu pour la recomposition (pas de 2e `project.inventory`) ; détection sur le plan en ids courts : clé `field`, `fields` ou finissant par `_field`, couche = `layer_id` de l'appel, `step:k`/`step:k.layer_id` suivi jusqu'au `layer_id` de l'étape k ; couche dans `index` mais pas dans `layers` → recomposer une fois avec `wanted`. `short_ids`/`left_out` de la dernière vue.
- `core/src/serve.rs` `FixedComposer` (l. 1724) -- file `first` de plans scriptés, `seen` des inventaires : y écrire les tests de la redemande ; son assertion `user_layers <= layers` doit compter l'index quand il existe. Tests MCP/`command.run` de `project.inventory` vers l. 2790–2900.
- `ai/src/plan.rs` -- `layer_cuts` (l. 360) exige `{"layers":[` en tête : pour `{"index":[`, une coupe par entrée de l'index (même `last_key_cut`) ; `inventory_parts` inchangé sinon. Tests : `the_whole_generated_set_is_cut_at_every_layer` (l. 1495) sur tout le jeu (index → entrées d'index), parité au jeton (l. 1645, ignorée sans `CASSINI_TEST_MODEL`) : `points.len()` = entrées coupées + 2.
- `ai/src/lib.rs` l. 81 -- message de `PromptTooLong` ; test l. 813.
- `cli/src/main.rs` -- `ModelComposer::compose` (l. 270) reçoit déjà la vue ; `plan()` (l. 578, `cassini plan --inventory`, forme apprise) : passer par la vue triée en forme apprise avec la demande, ids retraduits avant affichage ; test l. 1458 à doubler pour une vue à `index`.
- `ai/tests/common/mod.rs` `shown_inventory` (l. 39) -- ajouter une vue par demande (forme apprise ; inventaire brut de QGIS reconnu à `project_crs` et traduit par `agent_inventory`), à utiliser dans `duree_reel.rs` (composition ancrée) et les autres essais réels ; sous le seuil, test : égale à `ids.shorten(inventaire)` pour chaque inventaire de `ai/tests` et `tests/epreuve-modele-local`.
- `training/src/families.rs` l. 6942, `training/tests/forme_inventaire.rs` l. 384 -- appellent `sorted_view` : signature à suivre, comportement régénéré.
- `docs/mesure-duree-par-phrase.md`, `docs/entrainement-du-modele.md`, `architecture-noyau-et-facade-qgis.md` AD-23 -- mesures, budget, rappel de la redemande.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/agent_inventory/sorted_view.rs`, `core/src/agent_inventory.rs` -- finir le prototype, `wanted`, forme apprise, `user_layers` d'index, tests de la matrice (sigle, saut, ordre des scores, `wanted`)
- [ ] `core/src/journal/mod.rs` -- `last_plan` et ses tests
- [ ] `core/src/serve/plans.rs`, `core/src/serve.rs` -- branchement, redemande, tests (une redemande, aucune pour la géométrie, `step:k`, aucune journalisation de plus, rejeu identique, MCP brut)
- [ ] `ai/src/plan.rs`, `ai/src/lib.rs` -- coupes d'index, tests rétablis, message
- [ ] `cli/src/main.rs`, `ai/tests/common/mod.rs`, `ai/tests/*_reel.rs` -- chemins de composition par la vue
- [ ] `training/` -- appels mis à jour ; jeu régénéré : `forme_inventaire.rs`, `recouvrement.rs`, tests de familles verts ; part des gros projets et longueurs reconsignées
- [ ] docs -- budget, mesures (laissées à l'orchestrateur : « à compléter »)

**Acceptance Criteria:**
- Given `cargo fmt --all --check`, `cargo clippy --workspace --exclude cassini-app --all-targets -- -D warnings`, `cargo test --workspace --exclude cassini-app`, then vert.
- Given le jeu par défaut (graine 1, 8 600), then gros projets entre 23 et 27 %, aucun prompt au-delà de `MAX_PROMPT_CHARS`, `the_whole_generated_set_is_cut_at_every_layer` vert sans filtre.

## Design Notes

- Coupe d'une entrée d'index `{"geometry":…,"layer_id":…,"name":…}` : avant `name` ; la partie après la dernière entrée (fin de l'index, `layers`, `project`) se relit quand les couches détaillées changent, l'index se reprend de l'instantané `SLOT_LAST`, ou `SLOT_ANCHOR` quand des résultats sortent de l'index.
- La recomposition est faite par `propose`, pas par le compositeur : elle vaut pour tout `Composer`.

## Verification

Worktree `/home/kevin/cassini-worktrees/vue-branchee` (branche `story/5-48-vue-branchee`), `{project-root}` y désigne ce dossier. Avant de compiler : `export LIBCLANG_PATH=/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/38f6e1c7-3a62-4e6d-a1e2-bc44d994e1ac/scratchpad/clang/x/libclang-18.1.1.data/platlib/clang/native BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`. Ne pas committer ; les mesures réelles (inventaire réel, Windows, QGIS) sont faites par l'orchestrateur.

**Commands:**
- `cargo test -p cassini-core -p cassini-ai -p cassini` -- vert
- `cargo test -p cassini-training --release` -- vert
- `cargo fmt --all --check && cargo clippy --workspace --exclude cassini-app --all-targets -- -D warnings` -- vert

## Implementation Notes

## Plan Change Log

## Review Triage Log
