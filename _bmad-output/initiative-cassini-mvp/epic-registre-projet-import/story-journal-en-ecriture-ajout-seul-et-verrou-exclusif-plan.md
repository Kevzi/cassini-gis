---
title: 'Journal en écriture, ajout seul et verrou exclusif'
type: 'feature'
ticket: '5'
created: '2026-10-15'
status: 'built'
baseline_revision: 'a1804547ce1f7da3147b3b6aa14c0917ad002fc7'
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

**Problem:** le noyau exécute des commandes mais n'en garde aucune trace : rien à rejouer (2.6), aucun dossier de travail (AD-10), aucune garde contre deux sessions écrivant le même journal (AD-11).

**Approach:** une `Session` ouverte sur un fichier de projet crée le dossier de travail, prend un verrou exclusif, puis exécute chaque commande par le `Router` et ajoute une ligne JSON au journal. `cassini run --project` s'appuie sur elle.

## Boundaries & Constraints

**Always:**
- Dossier de travail : `<dossier du projet>/<nom du projet sans extension>.cassini/`, journal `journal.jsonl`, verrou `journal.lock` (fichier distinct, pour que le journal reste lisible par une session en lecture seule, y compris sous Windows).
- Une ligne par étape exécutée, succès ou échec : `step` (à partir de 1, suite du journal existant), `id`, `params` canonisés (clés triées), `inputs` (`path` relatif POSIX, `portable`, `size`, `modified`), `result` résumé ou `error` (code), `versions` {core, protocol, registry}. Aucun chemin absolu, messages compris (AD-9).
- Une commande refusée avant exécution (id inconnu, paramètres invalides) n'écrit rien.
- Session sans verrou : lecture seule, annoncée ; `call` répond `journal.read_only`, rien n'est exécuté ni écrit.
- Les originaux de l'utilisateur ne sont jamais modifiés ; rien n'est écrit hors du dossier de travail (AD-10).

**Never:**
- Pas de rejeu ni de sous-commande `replay` (2.6) ; pas de `serve` (2.7) ni de `session.hello` ; pas de repères de sortie `n` de l'étape `k` (2.6).
- Pas de commande de production : les commandes de test restent sous `#[cfg(test)]`.
- Pas de suppression de fichier.
- Pas de crate de verrou : décision du propriétaire, le verrou est `std::fs::File::try_lock` (Rust ≥ 1.89), aucune dépendance ajoutée.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|--------------|---------------------|------------------|
| Trois commandes | `--project p.qgz`, 3 appels | 3 lignes lisibles, sans chemin absolu | — |
| Seconde session | même projet, première ouverte | lecture seule annoncée ; écriture refusée | `journal.read_only`, journal intact |
| Fichier hors projet | `../x.tif` | ligne `portable: false`, chemin relatif | — |
| Chemin absolu en entrée | absolu dans le projet | journalisé en relatif | hors projet et sans relatif possible : `journal.path_unrepresentable` |
| Paramètres invalides | champ inconnu | aucune ligne | `registry.invalid_params` |
| Journal existant | 2 lignes déjà là | la suivante porte `step: 3` | — |

</frozen-after-approval>

## Code Map

- `core/src/registry.rs` -- `CommandSpec` gagne `path_params(&[..])` (entrées relatives au projet) et `output_params(&[..])` (valeurs fournies par le noyau, dans le dossier de travail) ; `PROTOCOL_VERSION` y vit déjà.
- `core/src/exec.rs` -- `Router::call` : réutilisé tel quel, sans modification.
- `core/src/journal.rs` -- nouveau : `Session`, `Mode`, `JournalLine`, `registry_version` (FNV-1a des ids et schémas triés, sans dépendance), verrou.
- `core/src/commands/test_commands.rs` -- nouveau, `#![cfg(test)]`, propriétaire unique réutilisé par 6, 8 et 10 : `test.read_file` (entrée relative), `test.write_output` (sortie dans le dossier de travail), `test.qgis_probe` (`runs_in: qgis`, servie par un exécutant factice).
- `core/src/lib.rs` -- `pub mod journal;`.
- `cli/src/main.rs` -- `--project <fichier>` pour `run` ; sans lui, comportement actuel inchangé. Ne pas toucher `ErrorReport` (`core/src/error.rs`) hors ajout des codes `journal.*`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/registry.rs` -- `path_params`, `output_params` sur `CommandSpec`, non sérialisés au catalogue -- le noyau sait quels paramètres sont des chemins
- [ ] `core/src/journal.rs` -- `Session::open(registry, project)`, `call`, résolution des chemins en relatif POSIX, verrou, ajout seul -- AD-9, AD-10, AD-11
- [ ] `core/src/commands/test_commands.rs` -- les trois commandes de test et l'exécutant factice
- [ ] `core/src/error.rs`, `core/src/lib.rs` -- codes `journal.read_only`, `journal.path_unrepresentable`, module
- [ ] `cli/src/main.rs` -- `--project` ; message de lecture seule sur `err`, code de sortie 1
- [ ] tests dans `journal.rs` et `main.rs` -- une ligne de la matrice chacune (deux sessions simultanées sur le même projet)

**Acceptance Criteria:**
- Given `--project` et trois commandes, when on lit le journal, then trois lignes JSON, aucune ne contient de chemin absolu.
- Given une session qui tient le verrou, when une seconde s'ouvre sur le même projet, then elle se dit en lecture seule et son `call` n'écrit rien.
- Given la fermeture d'une session, when une autre s'ouvre, then elle obtient l'écriture et continue la numérotation.

## Design Notes

Choix faits sans interrogation (le propriétaire a délégué l'epic) : `--project` ne porte que `run`, `replay` n'existant pas encore (2.6 réutilise l'analyse d'arguments) ; un `output_param` est rempli par le noyau avant validation avec `<étape>-<id>.out`, relatif au dossier de travail ; le résumé garde le résultat s'il tient en 200 octets, sinon `{"truncated":true,"bytes":n}`.

## Verification

**Commands:**
- `cargo fmt --all --check` -- expected: aucune différence
- `cargo test -p cassini-core -p cassini --locked` -- expected: tous les tests passent
- `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: aucun avertissement
- `cargo run -q --release -p cassini -- run system.about --project %TEMP%/p.qgz` (trois fois) -- expected: `p.cassini/journal.jsonl` de trois lignes

## Review Triage Log

Passe 1 (quick) : 0 high, 0 medium, 8 low, 0 false.

| Constat | Verdict | Route | Evidence |
|---|---|---|---|
| Un chemin absolu peut entrer au journal par `result` | low | rejeté | Aucune commande n'en renvoie ; la correction ajoute un filtre. Les commandes de l'epic 9 rendent des noms relatifs (AD-10). |
| Dossier `.cassini` et verrou créés même si rien n'est écrit | low | rejeté | La matrice parle des lignes du journal ; `journal.jsonl` reste absent. |
| Échec d'ajout après exécution : numéro d'étape réutilisé | low | rejeté | Disque plein ou erreur d'E/S seulement ; la correction ajoute de l'état. |
| Journal relu en entier à chaque appel | low | rejeté | Journaux courts ; sans effet visible. |
| `path_unrepresentable` testé sous Windows seulement | low | rejeté | Le cas n'existe que sur plusieurs disques. |
| Lecture seule non annoncée à l'ouverture par la CLI | low | rejeté | Un appel par exécution : le message `journal.read_only` l'annonce, code de sortie 1. |
| Empreinte du registre sans `runs_in` ni paramètres de chemin | low | rejeté | Conforme au plan (« ids et schémas »). |
| `rust-version` absent alors que `try_lock` exige 1.89 | low | patch (fait) | `rust-version = "1.89"` ajouté au workspace, repris par `core` et `cli` ; fmt, clippy et 79 tests passent. |
