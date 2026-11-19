---
title: 'Résultat validé par le noyau et gabarit unique de résumé'
type: 'feature'
ticket: '1'
created: '2026-11-19'
status: 'built'
baseline_revision: 'e599bcedbb9a2049cb88af13e6b02ff7a656d6f1'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le noyau relaie tel quel tout résultat renvoyé par l'extension, sans le confronter au schéma de sortie de sa commande ; aucun gabarit de résumé chiffré n'existe pour les entrées 10.3 et 10.11 ; au rejeu, un résultat court qui porte lui-même `truncated: true` est pris pour un résumé tronqué du journal (`same_as_kept`, `summary.rs`).

**Approach:** définir en Rust le gabarit `SummaryRow` (`rows`, `values` avec unité, `truncated`) ; faire valider chaque résultat reçu contre le schéma de sortie de sa commande, rejeté en `qgis.bad_result` avant toute journalisation (AD-27) ; passer les marqueurs de résumé du journal sous une clé réservée, les anciennes lignes restant lisibles. Sans QGIS.

## Boundaries & Constraints

**Always:** les marqueurs écrits par `summarize` vivent sous une clé propre au journal, jamais dans l'espace de noms des résultats ; les journaux déjà écrits (forme `{truncated: true, bytes[, fingerprint]}` à la racine de `result`) se rejouent comme avant ; la borne des lignes est `maxItems` du schéma, côté noyau ; `data.reproject` et `layer.remove`, déjà construites, restent valides ; `LATE_FIELDS`, `same_result`, la clé `replay`, `without_layer_id`, `SUMMARY_LIMIT` et `delegated_error` sont conservés ; aucun nom d'algorithme ni de champ QGIS dans le noyau (AD-22) ; décision du propriétaire, 2026-11-19 : la validation passe par la crate `jsonschema` (MIT, `default-features = false`, sans accès réseau), sur le JSON brut contre `output_schema`, sans ajouter `Deserialize` aux sorties des commandes ; décision du propriétaire, 2026-11-19 : `MIT-0` et `Zlib` entrent dans `allow` de `deny.toml` (licences de `borrow-or-share` et `foldhash`, tirées par `jsonschema`) ; décision du propriétaire, 2026-11-19 : `qgis-plugin/cassini_qgis/runlog.py` lit aussi la clé réservée `$summary`, avec un test, l'ancienne forme restant reconnue.

**Never:** `model.list` (reporté, `deferred-work.md`) ; migrer `analysis.zonal_statistics` vers le gabarit ; construire 10.3, 10.11 ou `analysis.distance_matrix` ; modifier l'extension QGIS au-delà de `runlog.py` (lecture de `$summary`) ; ré-entraîner le modèle ; toucher à `.claude/worktrees` ni à `_bmad-output/party-mode/`.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Résultat conforme | réponse de l'extension valide pour le schéma | journalisé et rendu comme avant | — |
| Résultat mal formé | champ requis absent ou de mauvais type | rien n'est journalisé | erreur `qgis.bad_result` |
| Lignes en excès | `rows` au-delà de `maxItems` | rejeté | `qgis.bad_result` |
| Résultat portant `truncated: true` | court, propre à la commande | comparé en clair au rejeu, non pris pour un résumé | — |
| Gros résultat | au-delà de `SUMMARY_LIMIT` | résumé sous la clé réservée, comparé par empreinte | — |
| Ancienne ligne de journal | `result` = `{truncated: true, bytes[, fingerprint]}` | lue comme un résumé | — |
| Commandes déjà construites | `data.reproject`, `layer.remove` | acceptées | — |

</frozen-after-approval>

## Code Map

- `core/src/serve.rs` -- `Channel::execute` (l. ~712-807) : `Ok(result)` repart brut ; point d'insertion de la validation. Patron de test : `an_unannounced_qgis_command_is_refused_and_not_journaled` (l. ~1411) ; réponses forgées via `exchange`/`run_line`.
- `core/src/registry.rs` -- `CommandSpec.output_schema` (l. 158), `Delegated` (l. 472), `RegistryError::Failed` (l. 405-431) pour `qgis.bad_result`.
- `core/src/journal/summary.rs` -- `summarize`, `same_as_kept` (le bug), `same_result`, `SUMMARY_LIMIT` ; imiter la clé réservée `replay` de `journal/mod.rs` (~l. 735).
- `core/src/journal/tests.rs` -- `a_big_result_is_summarized` (l. 237), `a_big_result_is_compared_by_fingerprint` et cas `legacy` (l. 1200-1214), lectures directes de `kept.get("truncated")` (l. 857, 4572).
- `core/src/commands/` -- créer le type `SummaryRow` ; `analysis_zonal_statistics.rs` reste inchangé.
- À relire avant d'agir : `Router::call` (autres points d'entrée), `core/tests/canal_externe.rs`, `core/tests/catalogue.rs`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/summary_row.rs` -- type `SummaryRow` (étiquette, `values` avec unité) et son schéma -- gabarit unique d'AD-27
- [ ] `core/src/serve.rs`, `core/src/registry.rs` -- valider le résultat contre `output_schema` avant journalisation ; `qgis.bad_result` -- AD-27
- [ ] `core/src/journal/summary.rs`, `core/src/journal/mod.rs` -- marqueurs sous clé réservée ; lecture des anciennes lignes -- bug latent
- [ ] `core/src/serve.rs`, `core/src/journal/tests.rs` -- une épreuve par ligne de la matrice ; adapter les lectures directes de `truncated`

**Acceptance Criteria:**
- Given une réponse mal formée de l'extension, when le noyau la reçoit, then `qgis.bad_result` est rendu et `journal.jsonl` n'existe pas.
- Given `cargo test --workspace`, when on le lance, then tout réussit, cas `legacy` compris.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Aucune boucle : pas d'intent_gap ni de bad_plan.

| Constat | Verdict | Route / preuve |
|---|---|---|
| Fixture `bout_en_bout.rs` sans `source_details` (test `#[ignore]`) | medium | patch : `source_details` est requis (`layout_export_pdf.rs:296`) ; le test serait rejeté en `qgis.bad_result` ; vérifier aussi `inventory()` et `ai/tests/*_reel.rs` |
| Rejet puis appel valide : numérotation non testée | medium | patch : test d'un rejet suivi d'un succès, et d'un rejet après un succès (une seule ligne ajoutée) |
| Ligne « commandes déjà construites » (`data.reproject`, `layer.remove`) sans test ; schémas jamais compilés en balayage ; id inconnu ; ligne imbriquée mal formée | medium | patch : tests de résultats réels de ces deux commandes, balayage de compilation des schémas de sortie, id inconnu, ligne mal formée |
| `kept_summary` accepte `$summary` avec des clés en plus ou un contenu non objet | low | patch : accepter seulement l'unique clé avec un objet (correction directe, ligne de journal éditée à la main) |
| `runlog._summarized` prend tout dict sous `$summary` pour un résumé | low | patch : exiger `truncated` vrai dans le contenu |
| `validate_output` voit le champ caché `orders` avant qu'il soit ôté | low | patch : valider une copie sans `ORDERS_FIELD` ; aucune sortie réelle n'est fermée aujourd'hui (seule `test_commands.rs`), la correction évite le piège futur |
| `jsonschema = "0.61"` non épinglé, alors que `libm` et `tiff` le sont pour le rejeu | low | patch : épingler `=` |
| Sorties réelles de l'extension jamais confrontées aux schémas du noyau (dérive Python/Rust) | medium | defer : la dérive préexiste (aucun contrôle depuis AD-24) ; le changement la rend visible ; constaté : l'extension rend bien `source_details`, `project_crs`, `extent` ; un balayage de tous les résultats est un ticket à part |
| `CLAUDE.md` §2 et §8 ne listent que MIT, Apache-2.0, BSD alors que `deny.toml` admet MIT-0 et Zlib | low | defer : le correctif édite un fichier d'instructions ; MIT-0 et Zlib acceptées par le propriétaire le 2026-11-19 |
| Validation absente de `Router::call` et d'autres points d'entrée | false | `impl Executor` : `InProcess` (Rust), `QgisStub` (refuse), `Channel` (seul à recevoir l'extension), `Announced` (enveloppe) |
| Clé `$summary` forgée dans un résultat ou des paramètres | low, rejeté | les paramètres sont fermés (`deny_unknown_fields`) ; seul un résultat d'extension défaillant peut la porter ; la correction ajoute une garde sans cas démontré |
| Résultat rejeté : l'extension a déjà agi, rien n'est journalisé | rejeté | c'est l'intention (« rejeté et non journalisé », AD-27) |
| Rejeu interrompu par `qgis.bad_result` | false | un échec franc au rejeu est le comportement voulu |
| Code `qgis.bad_result` envoyé par l'extension elle-même | low, rejeté | abus de protocole ; effet limité à une erreur non journalisée |
| Pas de marqueur de version du journal, ancien binaire | low, rejeté | choix de conception, hors périmètre |
| Lecture de l'ancien format différente en Rust et en Python | low, rejeté | le test Python d'origine (`fingerprint`) préexiste |
| `same_as_kept` : bytes absent des deux côtés, fausse égalité | false | `summarize` met toujours `bytes` côté courant |
| `SummaryTable` inutilisé en production, `maxItems` hors gabarit | false | clippy propre ; le plan donne la borne à chaque commande, et migrer `zonal_statistics` est exclu |
| `Implementation Notes`, `Plan Change Log` vides, `lenses_ran` | rejeté | le correctif éditerait le plan |

## Design Notes

Relecture `thorough` exigée (rejeu et canal, retro epic 9, A6). Deux points d'entrée possibles pour `Router::call` : la validation doit couvrir tous ceux qui reçoivent un résultat de l'extension.

## Verification

**Commands:**
- `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` -- expected: vert
- `cargo clippy --workspace --all-targets` -- expected: aucun avertissement
