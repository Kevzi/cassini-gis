---
title: "Garde-fou : une valeur ou un champ absent de l'inventaire donne une question, jamais un plan"
type: 'feature'
ticket: '57'
created: '2026-11-12'
status: 'ready-for-dev'
baseline_revision: 'b20058dba1420a7bc2cb1f2199c9b70921297c22'
route: 'full'
route_source: 'pinned'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: []
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
- [ ] `core/src/attribute_guard.rs`, `core/src/lib.rs` -- `guard_attributes(registry, plan, inventory) -> Plan` et `attribute_question` (tournures des Design Notes), tests de chaque ligne de la matrice (inventaire en JSON, registre standard) -- le cœur de la règle
- [ ] `core/src/registry.rs`, `core/src/commands/analysis_select_by_attribute.rs`, `core/src/contract.rs` -- déclaration `attribute_filter`, règle de contrat et test de refus -- générique, sans nom de commande dans le garde-fou
- [ ] `ai/src/plan.rs` -- appel après `guard_coordinates` ; tests sans modèle : valeur absente → question `guard`, valeur présente → plan, un seul appel au générateur -- tous les chemins
- [ ] `core/src/serve/` (tests de `propose`) -- question `guard` rendue sans `plan_id`, journal sans nouvelle ligne -- trace sans journal (AD-11)
- [ ] `training/src/families.rs` (tests) -- tout exemple engendré (graine et taille par défaut) passe `guard_attributes` inchangé avec son inventaire, et les exemples de `question_valeur_absente` à question gardent la même couche, le même champ et les mêmes valeurs que la question du noyau -- le modèle n'apprend rien que le noyau bloque
- [ ] `docs/canal-local.md`, `_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md` -- `guard` couvre aussi les valeurs et les champs ; sous AD-23, « Précision (2026-11-12, ticket 5.57) » de cinq lignes, acceptée par le propriétaire le 2026-11-12 -- la règle est écrite où on la cite

**Acceptance Criteria:**
- Given le modèle installé (`ajuste-continu-5-54-2`) sur GPU, when tournent `plan_reel`, `question_reel`, `ecart_reel`, then aucun score ne baisse, `question_reel` monte ou reste égale, et la ligne `SCORE` compte les questions du garde-fou.
- Given la recette d'Annecy (`probe_recette.py`, modèle installé), when elle tourne, then le score ne baisse pas et le rejeu reste identique.
- Given `duree_reel`, when il tourne, then la durée par phrase reste dans la barre de 5.19.

## Implementation Notes

## Plan Change Log

## Review Triage Log

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
