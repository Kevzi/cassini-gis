---
title: 'Contrat complet d''une commande, contrôlé par le registre lui-même'
type: 'feature'
ticket: '3'
created: '2026-10-15'
status: 'built'
baseline_revision: 'c8906bf7fd874b20feba3be8ffaa36e0c3688341'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le registre accepte aujourd'hui toute commande : id mal formé, description vide, `runs_in` imposé à `core`, aucune déclaration d'annulation, aucun test, id en double (écrasé en silence). Les epics 9, 5 et 4 ajoutent des commandes en parallèle : sans contrôle automatique, le contrat de AD-6 ne tient que par discipline.

**Approach:** chaque commande déclare tout son contrat (`runs_in`, annulable ou non, noms de ses tests) ; une fonction du registre, `Registry::check_contract()`, rend toutes les violations ; un test la lance sur le registre réel. Une règle de rédaction écrite, tirée du verdict de l'epic 8, fixe la forme des descriptions et des schémas.

## Boundaries & Constraints

**Always:**
- Le contrôle porte sur un `Registry` quelconque : les tests de refus construisent leurs propres registres, jamais le registre global.
- `check_contract` rend **toutes** les violations, chacune avec un code stable `contract.<regle>`, l'id fautif et un message français (AD-21).
- `runs_in` et « annulable » n'ont pas de valeur par défaut : non déclarés, ils sont une violation.
- Un test déclaré doit exister : le test du registre réel vérifie que chaque nom déclaré est une fonction `#[test]` d'un fichier de `core/src/commands/`.
- Aucune dépendance ajoutée.

**Never:**
- Pas d'exécutant abstrait ni de validation des paramètres à l'appel (2.4), pas de journal (2.5).
- Pas de commande de production nouvelle.
- Pas de contrôle automatique des règles de rédaction 3 à 5 du verdict (mesures nues, chemins, étapes en trop) : elles sont écrites, pas vérifiées.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|--------------|---------------------|------------------|
| Registre réel | `Registry::standard()` | aucune violation | — |
| Id mal formé | `Buffer`, `analysis`, `analysis.Buffer` | `contract.id` | violation rendue |
| Description absente | fr ou en vide | `contract.description` | violation rendue |
| Exécutant ou annulation non déclaré | champ à `None` | `contract.runs_in`, `contract.cancellable` | violation rendue |
| Dialecte absent | schéma sans `$schema` 2020-12 | `contract.schema` | violation rendue |
| Aucun test | liste vide | `contract.test` | violation rendue |
| Géométries non bornées | tableau de GeoJSON sans `maxItems` | `contract.geometry_unbounded` (AD-8) | violation rendue |
| Id en double | deux `register` du même id | `contract.duplicate_id` | violation rendue, la première entrée est gardée |

</frozen-after-approval>

## Code Map

- `core/src/registry.rs` -- `RunsIn` (variante `Core` seule : ajouter `Qgis`), `CommandSpec::new` (fixe `runs_in: Core`), `Registry::register` (écrase un id en double). Ajouter les champs optionnels `runs_in`/`cancellable`, `tests`, un compteur de doublons ; ne pas toucher `call` ni `standard`.
- `core/src/contract.rs` -- nouveau : `ContractViolation`, `Registry::check_contract`, règles d'id et de schéma.
- `core/src/lib.rs` -- `pub mod contract;`, seule édition.
- `core/src/commands/system_about.rs`, `test_echo.rs` -- déclarer `runs_in`, annulable, `tests` ; `test_echo` gagne un test nommé.
- `docs/regle-de-redaction-des-commandes.md` -- nouveau, d'après `verdict-epreuve-du-modele-local.md`, section « Ce que l'epic 2 en retient » (points 1 à 8). Lier depuis `docs/README.md`.
- `deferred-work.md` -- clore l'entrée « id en double » une fois traitée.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/registry.rs` -- `RunsIn::Qgis` ; `CommandSpec` : `runs_in` et `cancellable` en `Option`, `tests: Vec<String>`, builders `.runs_in()`, `.cancellable()`, `.tests()` ; `register` note un id en double sans l'écraser -- contrat sans défaut implicite
- [ ] `core/src/contract.rs` -- `check_contract` : id `domaine.verbe` snake_case ; deux descriptions non vides ; `runs_in` et annulation déclarés ; schémas avec `$schema` 2020-12 ; au moins un test ; schéma sans tableau de géométries non borné ; doublons -- parcours récursif du schéma JSON
- [ ] `core/src/commands/system_about.rs`, `test_echo.rs` -- déclarations complètes, noms de tests réels
- [ ] `core/src/contract.rs` (tests) -- un registre de test par règle, avec une commande volontairement incomplète : chaque cas échoue avec le bon code ; un registre sain passe ; le registre réel passe ; chaque test déclaré existe dans `core/src/commands/`
- [ ] `docs/regle-de-redaction-des-commandes.md` -- la règle écrite (exemples centrés sur les chaînes à plusieurs étapes, `values` des champs de catégorie, `{value, unit}`, pas de chemin de sortie, option facultative seulement si demandée, fr et en, `step:k` provisoire, unités) -- verdict de l'epic 8
- [ ] `deferred-work.md` -- clore « id en double » avec renvoi à cette entrée

**Acceptance Criteria:**
- Given sept commandes volontairement incomplètes, chacune dans son registre de test, when on lance `check_contract`, then chaque registre rend la violation attendue.
- Given `Registry::standard()`, when on lance `check_contract`, then aucune violation, `system.about` comprise.
- Given deux commandes de même id, when on les inscrit, then la première est gardée et `contract.duplicate_id` est rendu.

## Design Notes

Choix faits sans interrogation, à contester à l'approbation : (1) la preuve du test est un nom déclaré, vérifié par lecture des sources de `core/src/commands/` (la bibliothèque standard suffit, aucun outil de test à embarquer) ; (2) « annulable » est un booléen déclaré, la mécanique d'annulation vient avec 2.11 ; (3) le brief dit « six commandes » mais liste sept règles : toutes les sept sont testées ; (4) le refus est un contrôle (`check_contract` + test), pas un échec de `register`, pour ne pas changer la signature des inscriptions de 2.2.

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement
- `cargo run -q --release -p cassini -- run system.about` -- expected: `{"core_version":"0.1.0","protocol_version":1}`

## Review Triage Log

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Test déclaré vérifié par le seul nom, dans tout `commands/`, et par un test plutôt que par `check_contract` | low | rejeté | Le plan (Design Notes 1 et 4) fixe exactement ce choix ; le corriger ajoute de la mécanique. |
| Détection de géométries heuristique (faux positifs sur `GeometryKind`, `prefixItems` non traité) | low | rejeté | Limite connue d'une règle sans outil de schéma ; peu probable en usage courant, correctif complexe. |
| Les autres violations d'un doublon écarté sont perdues | low | rejeté | Le plan garde la première entrée et ne note que l'id. |
| `tests` sérialisé avec `CommandSpec` | low | patch | `#[serde(skip)]` ajouté sur `tests` ; tests, clippy et fmt repassent. |
| Numérotation du doc différente du verdict | low | rejeté | Doc seul ; le doc cite le verdict en tête. |
| Règles 9 et 10 sans renvoi aux règles 7 et 8 du verdict | false | rejeté | La tâche du plan demande ces deux points, et ils figurent au doc. |
