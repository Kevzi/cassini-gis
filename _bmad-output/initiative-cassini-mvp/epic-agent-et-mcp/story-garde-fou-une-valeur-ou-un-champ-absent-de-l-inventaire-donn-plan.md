---
title: "Garde-fou : une valeur ou un champ absent de l'inventaire donne une question, jamais un plan"
type: 'feature'
ticket: '57'
created: '2026-11-12'
status: 'built'
baseline_revision: 'b20058dba1420a7bc2cb1f2199c9b70921297c22'
route: 'full'
route_source: 'pinned'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le modèle compose parfois une sélection par attribut sur une valeur que l'inventaire montre absente de la liste complète d'un champ (clé `values`, au plus `MAX_VALUES`), ou sur un champ que la couche n'a pas. L'utilisateur reçoit une carte vide ou fausse, ou une erreur d'exécution `analysis.unknown_field`, sans savoir pourquoi.

**Approach:** Une fonction pure du noyau, appliquée au plan validé juste après le garde-fou des coordonnées, contrôle chaque sélection par attribut contre la vue d'inventaire envoyée au modèle. Une valeur absente d'une liste complète (selon l'opérateur) ou un champ absent change le plan en une question à provenance `guard` qui nomme la couche, le champ et, pour une valeur, les valeurs connues. Invariant générique du registre (AD-23), sans phrase ni pansement.

## Boundaries & Constraints

**Always:** Fonction pure dans `core`, sans IA ni QGIS (AD-1). Un seul point d'application, `compose_plan_anchored`, que suivent le canal, `cassini plan` et les mesures `ai/tests`. Le contrôle porte sur l'inventaire montré au modèle, identifiants courts compris. Un champ sans `values` n'est jamais contrôlé sur sa valeur. Une étape dont la couche est `step:k…` n'est pas contrôlée. Comparaisons exactes, casse comprise : `eq` et `ne` (valeur dans la liste), `begins_with` (une valeur commence par elle), `contains` (une valeur la contient). `lt`, `lte`, `gt`, `gte` jamais contrôlés. Plusieurs fautes : la question porte sur la première, dans l'ordre du plan. La question finit par « ? ».

**Never:** Toucher le prompt, `SHORT_SYSTEM`, `judge`, `retry_turns`, la grammaire, la forme de l'inventaire vue par le modèle, ni le jeu tenu à l'écart. Écrire la question ou sa provenance au journal (AD-11). Appliquer le garde-fou au serveur MCP. Remonter les champs d'une sortie d'étape (connaissance propre à chaque commande). Ré-entraîner ici. Mettre de la logique dans l'extension (AD-5).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Valeur absente | `typezone` `values` [N, U], plan `eq` « AU » | question `guard` : couche, champ, « AU », valeurs N, U | — |
| Valeur présente | même champ, `eq` « U » | plan inchangé | — |
| Champ sans `values` | `nom` sans `values`, `eq` « Gare X » | plan inchangé | — |
| Étape intermédiaire | `layer_id: "step:2.layer_id"` (ou `step:2`), valeur quelconque | plan inchangé | — |
| Champ absent | couche détaillée, liste de champs complète, `field: "zone_plu"` absent | question `guard` (couche, champ) au lieu de l'erreur d'exécution | — |
| Zonage AU | `values` [AUc, AUs, N, U], `begins_with` « AU » | plan inchangé ; `begins_with` « AUx » → question | — |
| Gros projet, champ coupé | vue triée (clé `index`), couche détaillée sans le champ, ou couche seulement dans `index` | plan inchangé (5.48 recompose si besoin) | — |
| Plusieurs sélections | 1re juste, 2e valeur absente | question sur la 2e seule | — |
| Question, refus, question des coordonnées | `Plan::Question` / `Refusal` | inchangé | — |

**Décisions du propriétaire (2026-11-12, approbation)** : (1) le journal ne note pas la question, le contrat (AD-11, AD-23 « Trace ») prime, la phrase du ticket est corrigée ; (2) la précision sous AD-23 est acceptée ; (3) le plan reste entier malgré sa taille.

</frozen-after-approval>

## Code Map

- `ai/src/plan.rs` l.722 -- `Ok(plan) => return Ok(guard_coordinates(...))` dans `compose_plan_anchored` (l.684) : y chaîner `guard_attributes(registry, …, inventory)`. Sortie rejetée (l.724-733, `guard_rejected`) inchangée. Tests modèles l.859-904 ; `guard_coordinates` (`core/src/point.rs` l.730) est le modèle de forme, à ne pas modifier. Doc de `compose_plan` l.646 à compléter.
- `core/src/plan.rs` l.30-77 -- `Plan::Question { question, source }`, `QuestionSource::Guard` réutilisé tel quel ; `parse_reference` pour reconnaître `step:`.
- `core/src/registry.rs` l.37, l.192 -- `CommandSpec` et ses déclarations (`output_params`) : ajouter `attribute_filter` (noms des paramètres couche, champ, opérateur, valeur). `catalog_entry` (`ai/src/plan.rs` l.270) ne lit que `id`, description et `params` : le prompt ne change pas.
- `core/src/commands/analysis_select_by_attribute.rs` l.~70 -- `.attribute_filter("layer_id", "field", "operator", "value")`, seul déclarant aujourd'hui.
- `core/src/contract.rs` `check_spec` l.86 -- règle `contract.attribute_filter` : les quatre noms existent au schéma des paramètres.
- `core/src/agent_inventory.rs` l.73 `MAX_VALUES`, l.743 `field` (liste triée, dédoublonnée, complète ou absente), l.531 `results_without_values` -- source de la règle « `values` présente = liste complète ».
- `core/src/agent_inventory/sorted_view.rs` l.358 `capped` -- coupe les champs d'une couche détaillée sans le marquer, mais garde chaque champ entier (valeurs comprises) : d'où la règle « vue triée = pas de contrôle de champ absent ». Ne pas toucher.
- `core/src/serve/plans.rs` l.237-244 -- question rendue `{"question","source"}` sans `plan_id`, sans journal : rien à changer, y ajouter un test.
- `training/src/families.rs` l.1497 `zone_operator`, l.1510 `in_zone_category`, l.3620 `missing_field_question`, l.3971 `missing_value_question`, l.10966 (exemples passés au garde-fou des coordonnées) -- tournures et règle à aligner ; générateur non modifié.
- `docs/canal-local.md` l.268-282 -- `source: guard` décrit pour les coordonnées seules.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/attribute_guard.rs`, `core/src/lib.rs` -- `guard_attributes(registry, plan, inventory) -> Plan` et `attribute_question` (tournures des Design Notes), tests de chaque ligne de la matrice (inventaire en JSON, registre standard) -- le cœur de la règle
- [x] `core/src/registry.rs`, `core/src/commands/analysis_select_by_attribute.rs`, `core/src/contract.rs` -- déclaration `attribute_filter`, règle de contrat et test de refus -- générique, sans nom de commande dans le garde-fou
- [x] `ai/src/plan.rs` -- appel après `guard_coordinates` ; tests sans modèle : valeur absente → question `guard`, valeur présente → plan, un seul appel au générateur -- tous les chemins
- [x] `core/src/serve/` (tests de `propose`) -- question `guard` rendue sans `plan_id`, journal sans nouvelle ligne -- trace sans journal (AD-11)
- [x] `training/src/families.rs` (tests) -- tout exemple engendré (graine et taille par défaut) passe `guard_attributes` inchangé avec son inventaire, et les exemples de `question_valeur_absente` à question gardent la même couche, le même champ et les mêmes valeurs que la question du noyau -- le modèle n'apprend rien que le noyau bloque
- [x] `docs/canal-local.md`, `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md` -- `guard` couvre aussi les valeurs et les champs ; sous AD-23, « Précision (2026-11-12, ticket 5.57) » de cinq lignes, acceptée par le propriétaire le 2026-11-12 -- la règle est écrite où on la cite

**Acceptance Criteria:**
- Given le modèle installé (`ajuste-continu-5-54-2`) sur GPU, when tournent `plan_reel`, `question_reel`, `ecart_reel`, then aucun score ne baisse, `question_reel` monte ou reste égale, et la ligne `SCORE` compte les questions du garde-fou.
- Given la recette d'Annecy (`probe_recette.py`, modèle installé), when elle tourne, then le score ne baisse pas et le rejeu reste identique.
- Given `duree_reel`, when il tourne, then la durée par phrase reste dans la barre de 5.19.

## Implementation Notes

- `core/src/attribute_guard.rs` : `guard_attributes`, `value_question` (forme 0 du générateur, valeurs dans l'ordre de l'inventaire) et `field_question` (la question d'un champ absent cite aussi les champs de la couche, au plus dix, « champs : a, b, c »). Une couche sans clé `fields` (raster) n'est pas contrôlée, comme une couche introuvable. Une couche `step:` est reconnue par `parse_reference` (toute valeur commençant par `step:`).
- `CommandSpec::attribute_filter` (`registry.rs`, type `AttributeFilter`, `#[serde(skip)]`) ; seul déclarant `analysis.select_by_attribute` ; règle `contract.attribute_filter` (les quatre noms aux propriétés du schéma des paramètres), test de refus `an_attribute_filter_names_existing_params`.
- `ai/src/plan.rs` : `guard_attributes` chaîné après `guard_coordinates` sur le plan validé seulement ; `guard_rejected` inchangé.
- Test du canal : `serve::tests::an_attribute_guard_question_has_no_plan_id_and_writes_no_journal_line` (le compositeur de test applique le garde-fou sur la vue reçue, champ `guard_attributes` de `FixedComposer`), dans `core/src/serve.rs` où vivent les tests de `propose`.
- **Écart aux Design Notes (après relecture)** : `begins_with` et `contains` comparent **sans la casse** (le fournisseur peut appliquer LIKE sans la casse) et ne sont pas contrôlés si la valeur porte `%` ou `_` ; un nom de champ se cherche aussi **sans la casse** (QGIS le fait), et une question n'est posée que si aucun champ ne correspond même ainsi. `eq` et `ne` restent exacts. Ne sont pas contrôlées non plus une valeur vide et une liste `values` de plus de `MAX_VALUES` valeurs (inventaire écrit à la main).
- Contrat : `contract.attribute_filter` exige aussi un opérateur en énumération des seuls opérateurs connus du garde-fou (`KNOWN_OPERATORS`) et une valeur qui accepte une chaîne.
- Barres : `ai/tests/common/mod.rs` `invalid` passe aussi chaque plan attendu par `guard_attributes` ; cas cassé `begins_with` ajouté à `an_invalid_expected_answer_is_named` (`par_commande_reel.rs`).
- Mesures : les lignes `SCORE` de `plan_reel`, `question_reel` et `ecart_reel` comptent déjà les questions du garde-fou (`QuestionSource::Guard`) : rien à changer.
- Extension : `qgis-plugin/tests/test_values_limit.py` épingle `VALUES_LIMIT == MAX_VALUES + 1`, qui rend complète toute liste montrée.
- Générateur : `every_generated_example_passes_the_attribute_guard_unchanged` (jeu complet, graine 1, 8 600 exemples) et `a_value_absent_question_matches_the_core_question` (le texte du générateur et celui du noyau portent la même couche, le même champ, la même valeur et la même liste ordonnée ; plan reconstruit avec l'opérateur du plan contraste, `begins_with` pour le zonage à sous-zones) passent sans changer le générateur.

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Verdicts : high 0, medium 4, low 9, false 2, maybe-false 0. Corrigés 12 (patch, même implémenteur), rejetés 4, reporté 1.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| `invalid` des barres n'appelle pas `guard_attributes` (begins_with/contains attendus non contrôlés) | medium | patch | appel ajouté, cas cassé `SEL1b` |
| Jokers `%`/`_` de LIKE dans begins_with/contains → fausse question | medium | patch | non contrôlé, test |
| begins_with/contains et nom de champ comparés avec la casse alors que QGIS peut ne pas la voir | medium | patch | sans la casse (eq/ne exacts), écart aux Design Notes consigné |
| Liste `values` > MAX_VALUES (inventaire écrit à la main) tenue pour complète | low | patch | non contrôlée, test |
| Valeur vide (le noyau retire "" des valeurs) | low | patch | non contrôlée, test |
| Question « champ absent » sans les champs de la couche | low | patch | champs cités (≤ 10) |
| Contrat `attribute_filter` superficiel (opérateurs inconnus ignorés) | low | patch | enum ⊂ `KNOWN_OPERATORS`, valeur chaîne, test de refus |
| Chemin du compositeur : seul le cas valeur absente testé | medium | patch | champ absent, vue triée, priorité des coordonnées, redemande puis valeur absente |
| Test de concordance générateur/noyau en partie tautologique, toujours `eq` | low | patch | texte du générateur, opérateur du plan contraste (zonage begins_with) |
| Rien n'épingle VALUES_LIMIT = MAX_VALUES + 1 (complétude des listes) | low | patch | `test_values_limit.py` |
| Doc : « carte vide » faux pour `ne` ; phrase maladroite de `compose_plan` ; SCORE non dit | low | patch | corrigés, note |
| Valeurs non textuelles ignorées → fausse question | false | reject | `agent_inventory::field` ne garde que des chaînes : un champ numérique n'a jamais de clé `values` |
| Liste de l'extension tronquée prise pour complète | false | reject | l'extension lit 11 valeurs, le noyau ne montre que ≤ 10 |
| `ne` sur valeur absente → question | low | reject | décision du plan approuvé : le filtre resterait sans effet, la demande est mal comprise |
| begins_with « A » qui accepte AUc (catégorie de zonage) | low | reject | la règle de zonage bloquerait à tort des préfixes ordinaires (« Saint » sur « Saint-Denis ») ; reste à l'entraînement |
| Test du canal par le compositeur de test, pas le vrai | low | reject | le vrai chaînage passe par `compose_plan_anchored`, testé dans `ai` |
| Interaction avec la redemande élargie de 5.48 non testée | low | defer | couches réduites non contrôlées ; à observer en recette |

## Design Notes

Décisions (party simulée : architecte, développeuse, UX).

- **Place.** Dans `compose_plan_anchored`, après `guard_coordinates`, pas dans `judge` ni dans `check_plan` : `check_plan` ne voit pas l'inventaire, et une faute dans `judge` déclencherait une redemande, donc une réparation. Le plan validé n'est jamais redemandé (AD-23, « Où »). Une couche introuvable dans la vue, ou présente seulement dans `index`, n'est pas contrôlée. La redemande élargie de 5.48 (`serve/plans.rs`) reste en aval : elle recompose, et le second plan repasse le garde-fou.
- **Inventaire.** La vue reçue par `compose_plan_anchored`, identifiants courts compris : aucune signature ne change. Hors vue triée, la liste des champs d'une couche est complète. Avec la clé `index`, `capped` a pu couper des champs sans marque : on ne contrôle alors pas les champs absents, seulement les valeurs (un champ montré l'est avec toutes ses valeurs). Une fausse question bloquerait un plan juste ; `analysis.unknown_field` reste le filet.
- **Opérateurs.** Seule la présence compte, pas le sens : `begins_with` « AU » passe dès qu'une valeur commence par « AU ». Toute valeur de `in_zone_category` vérifie `starts_with` : les plans du générateur passent. Comparaison exacte : QGIS compare la casse, et « u » au lieu de « U » donne la même carte vide.
- **Question.** Pas de tirage : la forme 0 du générateur, valeurs dans l'ordre de l'inventaire, séparées par « , ». Pour `begins_with` et `contains`, on écrit « qui commence par » et « qui contient ».
  - `Le champ typezone de la couche « Zonage PLU » n'a pas de valeur « AU » (valeurs : N, U) : laquelle faut-il prendre ?`
  - `Le champ typezone de la couche « Zonage PLU » n'a aucune valeur qui commence par « AUx » (valeurs : AUc, AUs, N, U) : laquelle faut-il prendre ?`
  - `La couche « Zonage PLU » n'a aucun champ « zone_plu » : quel champ faut-il utiliser ?`
- **Provenance.** `QuestionSource::Guard` réutilisé : le panneau remet déjà la phrase dans la saisie. Pas de nouvelle variante, et pas de changement dans l'extension (AD-5).
- **Journal.** Le verify du ticket dit « le journal note la question », ce qui contredit AD-11 et AD-23 (« Trace » : le journal ne tient que des étapes exécutées). On garde l'AD : la trace est `source: guard` dans la réponse du canal et à `cassini plan`, et un test vérifie qu'aucune ligne n'est écrite. Confirmé par le propriétaire le 2026-11-12.

## Verification

Environnement : `core` et `training` sous WSL ; `ai` et `cli` sous PowerShell (`LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`). Aucune mesure GPU pendant un entraînement : vérifier `ps` avant.

**Commands:**
- `cargo test -p cassini-core`, `cargo test -p cassini-training`, `cargo fmt --check`, `cargo clippy -p cassini-core -p cassini-training --all-targets -- -D warnings` (WSL) -- expected: succès
- PowerShell : `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check` -- expected: succès
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: succès (rien de changé)
- PowerShell, `CASSINI_TEST_MODEL` = déclaration du modèle installé : `cargo test -p cassini-ai --release --features cuda -- --ignored --nocapture plan_reel` (puis `question_reel`, `ecart_reel`) -- expected: scores ≥ ceux du modèle installé, `question_reel` ≥
- `cargo test --release -p cassini-ai --test duree_reel` (sans cuda, `docs/mesure-duree-par-phrase.md`) -- expected: barre de 5.19 tenue

**Manual checks (if no CLI):**
- Installer le noyau et l'extension, puis lancer la recette d'Annecy : `probe_recette.py <projet Annecy.qgz> qgis-plugin/tests/recette/annecy.json` sous le Python de QGIS 3.44 (`CASSINI_CORE`, `CASSINI_MODELS`, `QT_QPA_PLATFORM=windows`). Attendu : score ≥ 4/5 et rejeu identique. Ensuite, une démo dans QGIS : une phrase sur une valeur absente doit rendre la question et remettre la phrase dans la saisie.
