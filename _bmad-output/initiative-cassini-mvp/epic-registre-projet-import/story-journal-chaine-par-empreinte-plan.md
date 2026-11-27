---
title: 'Journal chaîné par empreinte'
type: 'feature'
ticket: '13'
created: '2026-11-27'
status: 'built'
baseline_revision: '46104c5144bc1c2726dba91b46ec7edde69f253c'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, edge-case]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `journal.jsonl` est en ajout seul, mais une ligne modifiée, retirée ou insérée à la main ne se voit pas, et le rejeu rejoue ce qu'il lit. Un bureau d'études ne peut pas dire « ce journal est celui que Cassini a écrit ».

**Approach:** chaque ligne écrite porte `prev` (SHA-256 hexadécimal des octets exacts de la ligne précédente, sans son saut de ligne, `null` pour la première) et `chain: 1`. À l'ouverture de la session, lecture seule comprise, le noyau relit le journal et rend l'état dans `session.hello` (`journal_chain`) ; le panneau signale une rupture. Le rejeu ne lit pas ces clés.

## Boundaries & Constraints

**Always:** on hache les octets écrits, jamais un JSON recanonisé ; les lignes anciennes ne sont jamais réécrites ; un journal sans `prev` ni `chain` est `unchained` et se rejoue à l'identique ; une chaîne rompue est signalée avant le rejeu, sans le refuser ; la vérification n'écrit rien (lecture seule comprise) ; le protocole reste à la version 2 (ajout compatible) ; `sha2` est déjà acceptée (2026-11-10), aucune dépendance nouvelle.

**Never:** toucher `Replay::run`, `check_inputs`, `same_result`, `plan_step`, `origin` ; ajouter une commande ou changer le catalogue ; l'empreinte de tête au pied du PDF (décision séparée, non prise) ; réécrire ou « réparer » une ligne ; une signature ou un secret.

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Journal neuf | 3 appels | ligne 1 `prev:null`, lignes 2-3 `prev` = SHA-256 des octets de la précédente, toutes `chain:1` ; `intact` | aucune |
| Vide / absent | pas de `journal.jsonl` | `intact`, sans ligne | aucune |
| Ligne modifiée | octets de la ligne k changés | `broken` à la ligne k+1 (sa `prev` ne correspond plus) | signalé, rejeu non refusé |
| Ligne retirée | ligne j retirée | `broken` à la ligne qui suit la coupure | idem |
| Ligne insérée | ligne sans `prev` ou à `prev` faux après la chaîne | `broken` à la ligne insérée | idem |
| Ancien journal | aucune ligne ne porte `chain` | `unchained` jusqu'à la dernière ligne ; rejeu identique | aucune |
| Ancien prolongé | n lignes anciennes, puis écritures | `unchained` jusqu'à la ligne n, première ligne neuve chaînée aux octets de la n-ième ; `intact` côté neuf | aucune |
| Dernière ligne tronquée | fin sans `\n`, JSON illisible | `broken` à cette ligne, motif `unreadable` ; la ligne suivante s'y chaîne par ses octets lus, sans réécriture | signalé |
| Lecture seule | verrou pris par une autre session | état rendu, rien écrit | aucune |
| Chaîne rompue au rejeu | `Replay::load` sur journal rompu | avertissement avant la première étape, rejeu lancé | aucune |

</frozen-after-approval>

## Code Map

- `core/src/journal/mod.rs` -- `Session::call_step` (~794-1009) : poser `prev` et `chain` juste avant `self.append` ; `Session::append` (1015) relit déjà le fichier, y prendre la dernière ligne (octets jusqu'au dernier `\n`, ligne tronquée comprise) ; `content_lines` (1097) définit la ligne ; `chain_warnings` (1725) et `Replay::load` (1653, `warnings`) : modèle de l'avertissement avant rejeu. Ajouter `verify_chain(project) -> ChainState` (`Intact | Unchained{upto} | Broken{line, reason}`), lecture seule, `sha2` comme `core/src/commands/model_list.rs:71`.
- `core/src/serve.rs` -- `hello` (883-1017, résultat 1006) : clé `journal_chain` (`state`, `line`, `reason`) ; tests d'après `hello_line` (1187) et `a_second_session_is_read_only_at_hello` (1418).
- `core/src/journal/tests.rs` -- helpers `temp_project`, `hand_journal` (314), `lines`, `replay_of` ; modèle `a_project_copied_with_its_journal_folder_replays_identically` (761).
- `docs/canal-local.md` (23, 29, puce après `history` l.60), `qgis-plugin/README.md`, `core/Cargo.toml` (commentaire de `sha2`).
- `qgis-plugin/cassini_qgis/client.py` (525-547, payload READY), `plugin.py` (268), `panel.py` (175-183 table des bandeaux, `set_ready` 965), `texts.py` (FR 65, EN 432) : bandeau `journal_broken`, absent sinon. Tests : `test_thread.py`, `test_tracer.py`.
- À ne pas toucher : `Replay::run`, `check_inputs`, `same_result`, `core/tests/catalogue.rs`, les schémas de commandes.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/journal/mod.rs` -- écriture `prev`/`chain`, `verify_chain`, avertissement avant rejeu -- chaînage et signalement
- [ ] `core/src/journal/tests.rs` -- une ligne de la matrice par test, plus rejeu identique d'un journal ancien et d'un journal prolongé -- preuve noyau sans QGIS
- [ ] `core/src/serve.rs` + `docs/canal-local.md` -- `journal_chain` dans `session.hello` -- contrat
- [ ] `qgis-plugin/cassini_qgis/{client,plugin,panel,texts}.py`, `qgis-plugin/tests/*`, `qgis-plugin/README.md` -- bandeau FR/EN en cas de rupture -- affichage en clair
- [ ] `core/Cargo.toml`, epic 2 (entrée 2.13) -- commentaire et trace -- documentation

**Acceptance Criteria:**
- Given un journal écrit par la session, when un octet d'une ligne est changé à la main, then `session.hello` rend `broken` à la ligne attendue et le panneau l'écrit en clair.
- Given un journal rompu, when on le rejoue, then l'avertissement précède la première étape et le résultat rejoué est inchangé.
- Given une session en lecture seule, when elle ouvre le journal, then l'état est rendu et le fichier est identique octet pour octet.
- Given un projet rouvert après fermeture, when on rejoue, then le rejeu est identique (recette QGIS).

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : quick + cas limites) : 4 constats corrigés (patch), 3 différés, 8 écartés.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch | CRLF : un journal converti LF vers CRLF est « rompu » à la ligne 2 (`written_lines` garde le `` dans les octets hachés) ; commentaire « mêmes lignes que content_lines » inexact |
| medium | patch | Panneau : l'avis de rupture n'est jamais affiché en lecture seule (`banner == "read_only"`), alors que le critère 1 le demande |
| low | patch | `Intact{unchained_upto>0}` rendu sans `line` : un ancien journal prolongé perd « non chaîné jusqu'à k » |
| low | patch | Limites non documentées : retrait des dernières lignes, retrait de `prev`/`chain` partout (journal lu non chaîné) |
| low | defer | Lecture concurrente par une seconde fenêtre pendant un `write_all` : fausse rupture `unreadable` jusqu'au prochain lancement |
| low | defer | Rupture non acquittable : aucune commande ne la scelle à nouveau (le plan interdit de réécrire) |
| low | defer | `chain`/`prev` non contrôlés en valeur (`chain:0`), `insert` écrase un champ de premier niveau |
| false | rejeté | Fuite de `prev`/`chain` dans `history_of` : la fonction recopie une liste fermée de clés, aucune fuite |
| false | rejeté | `test_thread.py` et fichier de l'epic absents : l'epic est modifié (note 2.13), `test_thread.py` n'était qu'une piste du Code Map |
| low | rejeté | Ligne illisible d'un ancien journal rendue `broken` : le plan fixe ce motif, `Replay::load` refuse déjà ce fichier |
| low | rejeté | `Replay::load` refuse la ligne tronquée avant l'avertissement, UTF-8 invalide : comportement antérieur inchangé |
| low | rejeté | Coût O(n) par `append` : relecture déjà présente avant le changement |
| low | rejeté | Atomicité lecture puis écriture dans `append` : le verrou exclusif d'AD-11 garantit un seul écrivain |
| low | rejeté | Lignes blanches finales, test de lecture seule du noyau jugé faible : couvert par le test de `serve.rs` |

## Design Notes

Décisions du plan (réponse à la question ouverte du ticket, ligne tronquée) : une ligne illisible est `broken` avec le motif `unreadable`, elle le reste tant que le fichier est inchangé ; la suivante se chaîne à ses octets lus, sans réécriture. La ligne k signalée est celle dont `prev` ne correspond pas à la précédente. La modification de la toute dernière ligne n'est pas détectable sans successeur : c'est la limite assumée du change (§2). Le panneau n'affiche que la rupture, pas `intact` ni `unchained` (un ancien journal le resterait pour toujours) ; les trois états restent dans `session.hello`.

```json
{"journal_chain": {"state": "broken", "line": 4, "reason": "prev_mismatch"}}
```

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini --locked` (CARGO_TARGET_DIR=C:/ct) -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
