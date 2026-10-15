---
title: 'Rejeu du journal par cassini replay'
type: 'feature'
ticket: '6'
created: '2026-10-15'
status: 'built'
baseline_revision: '5cb85552abbba1ed660a373a8ee164b5e04a3baa'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le journal s'écrit (2.5) mais rien ne le relit : R5 (rejeu à l'identique) n'est pas tenu, et les sorties d'une étape ne peuvent pas servir d'entrée à la suivante.

**Approach:** `cassini replay <projet>` relit `journal.jsonl` et réexécute chaque étape par le même chemin de résolution et le même `Router` que l'exécution d'origine, puis compare résultat et erreur à ce que le journal a gardé. Les sorties se désignent par un repère local à l'étape, `out:<k>.<n>`.

## Boundaries & Constraints

**Always:**
- Repère `out:<k>.<n>` : sortie `n` (à partir de 1, dans l'ordre de `output_params`) de l'étape `k`. Valeur d'un paramètre de `path_params`. Journalisé tel quel ; l'exécutant reçoit le chemin absolu du fichier de sortie. Même résolution à l'exécution et au rejeu (AD-11).
- Le rejeu n'écrit pas le journal. Ses sorties vont dans `<projet>.cassini/replay/`, jamais sur les sorties d'origine (AD-10). Il fonctionne aussi quand une autre session tient le verrou.
- Avant la première étape : une commande inconnue du registre fait échouer en la nommant, sans rien exécuter ; chaque entrée `portable: false` est signalée sur `err`, et le rejeu continue.
- Une commande que l'exécutant ne sait pas faire échoue en la nommant (`registry.unavailable`) ; jamais d'étape sautée.
- Identique = même `result` résumé (ou même code d'`error`), et mêmes octets pour chaque fichier de sortie. Premier écart : arrêt, `replay.diverged` à l'étape fautive.
- Aucun chemin absolu dans les messages (AD-9).

**Never:**
- Pas de `serve` (2.7), pas de nouvelle dépendance, pas de suppression de fichier.
- Pas de contrôle de version du registre ni de modification des entrées d'origine.
- Pas de commande de production : commandes de test sous `#[cfg(test)]`.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|--------------|---------------------|------------------|
| Rejeu fidèle | journal `write_output` puis `read_file out:1.1` | résultats et sorties identiques, rapport `{steps, identical}` | — |
| Commande inconnue | une ligne dont l'id n'est pas au registre | rien n'est exécuté | `registry.unknown_command`, id nommé, étape indiquée |
| Commande QGIS sans exécutant | `test.qgis_probe` | échec nommant la commande | `registry.unavailable` |
| Non portable | entrée `../x.tif` | signalée avant l'étape 1 ; rejeu poursuivi | — |
| Entrée modifiée | fichier d'entrée changé | écart détecté | `replay.diverged` |
| Repère invalide | `out:9.1` ou étape sans sortie | refus | `replay.bad_reference` |
| Pas de journal | dossier absent ou vide | refus | `replay.no_journal` |
| Session concurrente | verrou tenu ailleurs | rejeu permis | — |

</frozen-after-approval>

## Code Map

- `core/src/journal.rs` -- `Session::call` résout déjà `path_params`/`output_params` (l. 183-242) ; extraire cette résolution dans une fonction partagée, y ajouter le repère `out:<k>.<n>`. Réutiliser `canonical`, `summarize`, `normalize`, `relative_to`. Ne pas changer le format des lignes existantes.
- `core/src/exec.rs` -- `Router::call` réutilisé tel quel.
- `core/src/error.rs` -- `From<&JournalError>` ; ajouter les codes `replay.*` sans changer `ErrorReport`.
- `core/src/commands/test_commands.rs` -- `test.write_output`, `test.read_file`, `test.qgis_probe` suffisent ; ne rien ajouter.
- `cli/src/main.rs` -- `parse_run`/`run` ; ajouter `replay <projet>`, même sortie JSON et mêmes codes de sortie que `run`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/journal.rs` -- résolution partagée (chemins, sorties, repère `out:`) utilisée par `Session::call` et le rejeu -- un seul chemin, AD-11
- [ ] `core/src/journal.rs` -- `Replay::load(registry, project)` (lecture, contrôles préalables, `warnings()`) et `Replay::run(&Router)` -> `ReplayReport{steps, identical}` ; sorties sous `replay/`
- [ ] `core/src/error.rs` -- codes `replay.no_journal`, `replay.bad_reference`, `replay.diverged`
- [ ] `cli/src/main.rs` -- sous-commande `replay`, avertissements sur `err` avant l'exécution, usage mis à jour
- [ ] tests dans `journal.rs` et `main.rs` -- une ligne de la matrice chacune, dont le repère en exécution d'origine

**Acceptance Criteria:**
- Given un projet dont le journal compte une écriture de sortie puis sa lecture par repère, when `cassini replay`, then rapport identique, sorties sous `replay/`, journal d'origine inchangé.
- Given un journal portant une commande inconnue, when on rejoue, then échec nommant la commande, aucune étape exécutée.
- Given une session qui tient le verrou, when on rejoue le même projet, then le rejeu réussit sans écrire le journal.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 0 medium, 8 low, 0 false.

| Constat | Verdict | Route | Evidence |
|---|---|---|---|
| Ligne de journal tronquée par un arrêt brutal : le rejeu échoue en `journal.io` sans nommer l'étape | low | rejeté | Réel (`Replay::load`, l. 480-483) mais il faut un arrêt entre deux octets d'un `write_all` suivi d'un `sync_data` ; la correction demande une politique (sauter ou refuser) que le plan n'a pas. |
| `replay/` jamais vidé : un fichier périmé peut masquer une sortie non réécrite ; deux absents sont égaux | low | rejeté | Réel (l. 573-575) mais la commande qui n'écrit plus change aussi son `result` ; supprimer un fichier est interdit par le plan. |
| `size` et `modified` des entrées non comparés au rejeu | low | rejeté | Le plan définit « identique » par le résultat et les octets de sortie ; conforme. |
| `replay.bad_reference` levé pendant `run`, après des étapes déjà exécutées | low | rejeté | Les sorties vont sous `replay/`, rien d'origine n'est touché ; sans effet visible. |
| Toute chaîne `out:…` d'un `path_params` est prise pour un repère ; référence vers une étape en échec acceptée | low | rejeté | Un chemin relatif nommé `out:…` est improbable (et illégal sous Windows) ; l'exécutant reçoit un chemin absent, ce que l'écart de résultat révèle. |
| `journal_ids` relu à chaque appel | low | rejeté | Journaux courts ; un id illisible donne `replay.bad_reference`, pas une dérive silencieuse. |
| Étape numérotée par position (`i + 1`) et non par le champ `step` | low | rejeté | Le journal est en ajout seul et `step` est `lignes + 1` : les deux coïncident toujours. |
| Pas de test CLI du rejeu fidèle ni de l'avertissement non portable sur `err` | low | rejeté | Couverts dans `core` ; les commandes de test n'existent pas dans le binaire. |

## Design Notes

Choix faits sans interrogation (epic délégué) : `replay` prend le projet en positionnel ; un écart arrête le rejeu car les étapes suivantes dépendent des sorties ; non portable avertit sans bloquer, comme l'exige AD-9 (« le signale avant de commencer »). Exemple de ligne : `{"step":2,"id":"test.read_file","params":{"path":"out:1.1"},...}`.

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement
