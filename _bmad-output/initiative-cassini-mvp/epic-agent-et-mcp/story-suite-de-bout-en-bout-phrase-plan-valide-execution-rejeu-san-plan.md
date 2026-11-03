---
title: "Suite de bout en bout : phrase, plan valide, exécution, rejeu sans l'IA"
type: 'feature'
ticket: '9'
created: '2026-11-01'
status: done
baseline_revision: '88251ab07994982c0ce9f891722c40075a8bf746'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [quick, blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Chaque morceau du parcours de la phrase de démo est testé à part (plan à trois étapes rejoué, relais qui journalise `external`, garde réseau Windows sur les imports du binaire), mais aucun test commité ne prouve le parcours entier ni que le rejeu se passe de l'IA. Le Done when de l'epic 5 (points 2, 3 et 4) n'a donc pas sa preuve.

**Approach:** Ajouter les tests qui manquent : le plan de démo à cinq étapes composé, validé, exécuté par le canal puis rejoué dans une session **sans compositeur** ; un appel du relais `cassini mcp` journalisé `external` puis rejoué ; une garde réseau d'exécution sur `cassini serve` et `cassini mcp` ; une variante ignorée avec le vrai modèle. Re-mesurer ensuite le taux (`plan_reel`, GPU) et la durée par phrase (`duree_reel`, processeur, comme 11), et consigner les chiffres.

## Boundaries & Constraints

**Always:** aucun code produit modifié, sauf un défaut révélé par un test (signalé avant d'être corrigé) ; tests hors ligne, sans QGIS, sur fixtures synthétiques ; la suite par défaut ne charge aucun modèle ; chaque nouvelle garde dit en français ce qu'elle a trouvé quand elle échoue ; AD-1 à AD-29 non re-décidés.

**Never:** aucune dépendance nouvelle (la garde lit `/proc` sous Linux et `netstat -ano` sous Windows) ; aucun seuil de durée vérifié par un test (la durée reste une mesure, comme en 11) ; pas de données Villeneuve-le-Roi dans le dépôt.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan de démo | compositeur fixe rendant les 5 étapes de `demo_plan`, extension factice | 5 ordres un à un, 5 lignes au journal, paramètres `step:N` résolus | — |
| Rejeu sans IA | seconde session du même projet, sans compositeur | `{"steps":5,"identical":true}`, journal identique octet pour octet | — |
| Rejeu divergent | l'extension rend un autre résultat à l'étape 3 (un autre `layer_id` seul ne compte pas : valeur de session) | erreur `replay.diverged` qui arrête le rejeu, `data.step` = 3, message nommant la commande (décision du propriétaire, 2026-11-01 : comportement du noyau gardé) | pas de panique, la session reste debout |
| Relais | `cassini mcp` relaie `layer.describe` vers `cassini serve` | dernière ligne `origin: "external"` ; `journal.replay` → `identical: true` | — |
| Garde réseau | `serve` et `mcp` pendant un appel relayé | aucune socket TCP/UDP tenue par l'un ou l'autre ; le canal local (socket Unix, tube nommé) est permis | échec qui liste les sockets trouvées |
| Vrai modèle | `#[ignore]`, `CASSINI_TEST_MODEL` | phrase de démo → plan conforme exécuté ; rejeu avec `CASSINI_MODELS` vide → `identical: true` | sans la variable, l'essai ne fait rien |

</frozen-after-approval>

## Code Map

- `core/src/serve.rs:1439-1530` -- tests du module : `FixedComposer`, `composer(plan)`, `Client::start_with(grace, composer)`, `propose`/`expect_step`/`execute`/`journal`. Modèle à suivre : `a_three_step_plan_runs_one_order_at_a_time_and_replays_identically` (:1975) et `journal_replay_runs_the_qgis_steps_through_the_channel` (:1872). Le `Client` ouvre un projet temporaire : lui ajouter un moyen d'ouvrir **le même projet** dans une seconde session (le verrou du journal est relâché à la fin de la première).
- `ai/tests/ecart_reel.rs:529` `demo_plan` -- les 5 étapes de la phrase de démo (`select_by_attribute`, `buffer`, deux `intersect`, `layout.export_pdf`) ; recopier la forme, pas importer (crate différente). Les commandes déléguées doivent figurer dans les `commands` du hello.
- `core/src/serve/plans.rs:149-177` -- l'inventaire est demandé à l'extension par `project.inventory` (hors journal) : la fausse extension doit y répondre dans l'essai au vrai modèle.
- `cli/tests/mcp.rs:280-420` `QgisSession` (start, request, switch, wait_order, answer, journal, close) -- à déplacer dans `cli/tests/support/qgis_session.rs` (mécanique), avec le dossier de modèles en paramètre de `start` et `pid()` exposé ; `mcp.rs` l'importe.
- `cli/tests/mcp.rs:450` `the_relay_serves_the_catalogue_of_the_session_and_runs_a_qgis_command` -- relais + `origin` ; ne rejoue pas. Base du nouveau test relais + rejeu + garde réseau.
- `cli/tests/support/mcp_client.rs` -- client MCP (`connect_tagged`, `call_tool`) ; il faut aussi le pid du processus `cassini mcp`.
- `core/src/external/unix.rs`, `windows.rs` -- canal local : socket Unix sous Linux, tube nommé sous Windows ; ni l'un ni l'autre n'est une socket réseau.
- `cli/tests/hors_ligne.rs` -- garde statique (imports PE, Windows seul) ; ne pas toucher, la nouvelle garde la complète à l'exécution.
- `ai/tests/plan_reel.rs` (en-tête :4, `cargo test -p cassini-ai --release --features cuda -- --ignored plan_reel`) et `ai/tests/duree_reel.rs` (mode opératoire `docs/mesure-duree-par-phrase.md:12-48`, masque `0x55`, sans cuda) -- les deux mesures à relancer, sans les modifier.
- `docs/mesure-duree-par-phrase.md` (dernier tableau vers :515 : A0 = 29,8 s, moyenne 15,1 s ; `plan_reel` 25/25 vers :564) -- où consigner la re-mesure.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/serve.rs` (tests) -- plan de démo à 5 étapes, exécuté par le canal, rejoué dans une seconde session sans compositeur ; cas divergent -- points 1 et 2 du Done when.
- [x] `cli/tests/support/qgis_session.rs`, `cli/tests/support/reseau.rs`, `cli/tests/mcp.rs` -- déplacer `QgisSession` ; `inet_sockets(pid) -> Vec<String>` (Linux : sockets de `/proc/<pid>/fd` cherchées dans `/proc/net/{tcp,tcp6,udp,udp6}` ; Windows : lignes de `netstat -ano` du pid ; ailleurs : liste vide et note) ; nouveau test : appel relayé de `layer.describe`, garde réseau sur les deux processus pendant l'appel, `origin: "external"`, puis `journal.replay` → `identical: true` -- point 4.
- [x] `cli/tests/bout_en_bout.rs` -- essai `#[ignore]` au vrai modèle sur le binaire : `plan.propose` de la phrase de démo (inventaire de démo de `tests/epreuve-modele-local/inventory.json`), `plan.execute` répondu par la fausse extension, garde réseau sur `serve` après la composition, puis rejeu dans un second `serve` dont `CASSINI_MODELS` pointe un dossier vide -- point 3, conditions réelles.
- [x] `docs/mesure-duree-par-phrase.md`, `docs/entrainement-du-modele.md` -- consigner, datés avec la révision : taux `plan_reel` (GPU), durées A0–E (processeur, `0x55`), résultat de l'essai au vrai modèle -- la mesure demandée.

**Acceptance Criteria:**
- Given le dépôt, when `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings` et `cargo fmt --check` tournent, then ils passent, sans modèle et sans réseau.
- Given un modèle et une carte graphique, when `plan_reel` et l'essai `bout_en_bout` tournent, then le taux et le résultat du rejeu sont écrits dans la doc avec la date et la révision.
- Given le processeur seul bridé à 4 cœurs, when `duree_reel` tourne sur les phrases A0–E, then les durées sont écrites dans la doc à côté de la mesure précédente.

## Implementation Notes

- Aucun code produit modifié ; aucun défaut révélé. Seuls les tests et leurs supports changent.
- `core/src/serve.rs` (tests) : `Client::start_in` et `Client::reopen` (seconde session du même
  projet, le dossier passe à la seconde) ; `demo_plan`, `demo_answer` (la fausse extension écrit
  la sortie nommée par le noyau : le rejeu exige une sortie QGIS non vide) ; tests
  `the_demo_plan_runs_then_replays_in_a_session_without_composer` et
  `a_demo_replay_that_differs_names_the_step`.
- Rejeu divergent : le produit ne rend pas `identical: false` mais l'erreur `replay.diverged`
  avec `data.step` (le premier écart arrête le rejeu, `ReplayReport.identical` est toujours vrai).
  Et un autre `layer_id` seul ne diverge pas (valeur de session, liée par le rejeu). Le test
  vérifie donc : à l'étape 3, un autre résultat (99 entités au lieu de 7) → `replay.diverged`,
  étape 3, message qui nomme `analysis.intersect`, session encore debout.
- `cli/tests/support/qgis_session.rs` : `QgisSession` déplacée, `start(external, models,
  commands)`, `reopen(models)`, `pid()`, `wait_reply`, `wait_for_within`, `journal_path`.
  `mcp_client.rs` : `pid()`. `cli/tests/support/reseau.rs` : `inet_sockets`, `assert_offline`.
  Garde vérifiée sur les deux branches : Windows par la suite (`netstat -ano`), Linux par un
  essai à part (socket Unix ignorée, UDP et TCP6 vus). Le test
  `the_network_guard_sees_a_socket_held_by_the_process` lie une socket UDP sur la boucle locale
  (aucun paquet envoyé) pour prouver que la garde n'est pas aveugle.
- `cli/tests/bout_en_bout.rs` : phrase A0 exacte, inventaire de l'épreuve traduit dans la forme
  de `project.inventory` ; le modèle est pris dans son dossier et choisi par son nom ; plan jugé
  conforme si les quatre analyses égalent celles de S17 et la 5e étape est un export PDF titré.
  Le notateur et la traduction sont testés sans modèle.

- Vérification (2026-11-01, Windows, cargo Windows, `CARGO_TARGET_DIR=C:\ct`) :
  `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo fmt --check` verts ; `plan_reel` (cuda) 25/25 ; `bout_en_bout` (gpu) plan conforme en
  16,5 s, rejeu sans modèle `identical: true` ; `duree_reel` `0x55` : A0 27,9 s, moyenne 14,1 s.
  Consigné dans `docs/mesure-duree-par-phrase.md` et `docs/entrainement-du-modele.md`.
- Vérification manuelle QGIS faite le 2026-11-03 par Claude : noyau installé (5.32, code produit inchangé) et extension identique au dépôt ; `probe_demo.py` sur une copie neuve du projet Villeneuve-le-Roi, modèle livré seul : plan conforme composé en ~17 s, exécuté en 8,3 s, PDF lisible. Rejeu sous QGIS dans une session sans modèle, deux fois avec le projet rouvert entre les deux : `identical: true`, journal inchangé. Sans rouvrir le projet, le second rejeu diverge (défaut antérieur, au report).

## Plan Change Log

## Review Triage Log

Passe 1 (2026-11-01, lentille quick) — high 0, medium 2, low 5, false 0, maybe-false 0 ; puis 1 low rejeté.

- medium, hors code — vérification manuelle QGIS (`probe_demo.py`) non faite : réel ; faite à la présentation, avant la fusion (règles « démo de bout en bout par Claude » et « installer avant la recette »).
- medium, signalé — `CLAUDE.md` §8 demande `thorough` pour une entrée rejeu/canal, le workflow a épinglé `quick` : réel ; le correctif touche la config d'agent → signalé au propriétaire, pas corrigé ici.
- low, patch — `bout_en_bout.rs:347` `agent.models` affiché, pas vérifié, alors que la doc l'affirme : assertion de liste vide.
- low, patch — `mcp.rs` garde « pas aveugle » : vérifie seulement une liste non vide ; assertion que la ligne trouvée porte le port lié.
- low, patch — `bout_en_bout.rs:227` `gaps` : « pas de plan : null » sur une question ou un refus ; le message montre la réponse entière.
- low, patch — docs 5.9 : la révision `88251ab` ne contient pas les tests ; reformulé en « commit du ticket 5.9, base `88251ab` ».
- low, rejeté — `a_demo_replay_that_differs_names_the_step` : `analysis.intersect` vaut aussi pour l'étape 4 ; `data.step == 3` fixe déjà l'étape.
- low, rejeté — `Client::reopen` repart avec `CANCEL_GRACE` : tous les appelants l'emploient, aucun défaut atteignable.

Passe 2 (2026-11-03, thorough à la demande du propriétaire : blind-hunter, edge-case-hunter, verification-gap, intent-alignment) — high 0, medium 1, low 13 corrigés, 11 rejetés, 1 report.

- medium, patch — `serve.rs` `replay_demo` : les paramètres rejoués ne sont pas vérifiés (des ids périmés ou une seule des deux références de l'étape 4 reliée resteraient verts) ; assertions `r_1`, `r_3`, `r_2`, couches sources.
- low, patch — rejeu divergent : journal non vérifié après l'erreur ; comparaison octet pour octet.
- low, patch — `reseau.rs` Windows : statut de `netstat` ignoré ; assertion de succès.
- low, patch — `reseau.rs` Linux : table `/proc/<pid>/net/*` illisible sautée ; échec explicite.
- low, patch — `reseau.rs` : garde par instantanés (aussi intent-alignment 2, verification-gap) ; dit dans la doc du module.
- low, patch — garde « pas aveugle » : port cherché dans toute la ligne, UDP v4 seul ; colonne locale, et une socket TCP aussi.
- low, patch — `QgisSession` sans `Drop` (et dossier perdu si `reopen` échoue) : processus `serve` orphelin, modèle chargé ; `Drop` qui tue l'enfant et efface le projet.
- low, patch — `bout_en_bout.rs` : `CASSINI_TEST_MODEL` relatif nu → `CASSINI_MODELS` vide ; chemin canonique. Ordre des commandes et `step:` résolus non vérifiés sur le binaire ; assertions ajoutées.
- low, patch — docs : pas de hash ; `bf24d56` nommé ; moyenne 5.20 rapportée au tableau complet.
- false — réglage `quick` contre `CLAUDE.md` §8 : cette passe thorough le règle.
- false — vérification QGIS non faite : faite le 2026-11-03 (voir Implementation Notes).
- false — `bout_en_bout` vert sans `CASSINI_TEST_MODEL` : voulu par la matrice (« sans la variable, l'essai ne fait rien »).
- false — « A0 sous 30 s » lu comme exigence i5 : l'epic fixe la machine de mesure au 5800X bridé à 4 cœurs.
- low, rejeté — tables `raw`/`icmp`/`udplite` non lues : aucun composant n'ouvre ce genre de socket, et ajouter des tables parfois absentes complique la garde.
- low, rejeté — erreurs de `read_link` sur un fd ignorées : le fd fermé entre-temps n'est plus une socket tenue ; garde inutilement plus lourde.
- low, rejeté — garde vide hors Linux et Windows : le produit et la CI ne visent que ces deux systèmes ; une note est imprimée.
- low, rejeté — ordre inattendu pendant `plan.propose` : l'assertion échoue bruyamment, comportement voulu (le `Drop` ajouté évite l'orphelin).
- low, rejeté — `v as u64` sature sur une distance négative : normalisation de comparaison seulement, sans effet.
- low, rejeté — `gaps` strict : voulu, même règle que le plan attendu S17 de `plan_reel`.
- low, rejeté — deux écritures de la commande `plan_reel` : équivalentes.
- low, rejeté (intent-alignment 1, 3, 4) — la chaîne phrase → plan sur le binaire n'existe qu'avec le vrai modèle (essai ignoré) ; « identical » se juge contre la fausse extension par construction ; le point 4 suit le périmètre de l'Intent.
- report — deux rejeux de suite dans la même session QGIS : le second diverge (dossier `replay/` non vidé, fichiers tenus par QGIS) ; défaut antérieur, `deferred-work.md`.

## Design Notes

Le rejeu « sans l'IA » se prouve par l'absence : la seconde session n'a **aucun** compositeur (`None`), donc elle ne peut pas charger de modèle ; côté binaire, `CASSINI_MODELS` vide fait le même travail. Ne pas ajouter de compteur de chargement au produit pour le prouver.

La garde réseau vise les sockets TCP/UDP du processus, pas toutes les sockets : AD-29 permet le canal local d'IA externe.

## Verification

**Commands:**
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` -- expected: verts (sous Windows : cargo Windows, `LIBCLANG_PATH` d'outils-cassini, `CARGO_TARGET_DIR=C:\ct`).
- `cargo test -p cassini-ai --release --features cuda -- --ignored plan_reel` -- expected: taux consigné (≥ 80 %).
- `cargo test -p cassini --release --features gpu --test bout_en_bout -- --ignored` -- expected: plan exécuté, rejeu `identical: true`.
- `duree_reel` selon `docs/mesure-duree-par-phrase.md:12-48`, masque `0x55` -- expected: tableau consigné.

**Manual checks:**
- Installer noyau et extension, puis jouer la phrase de démo dans QGIS (Claude lui-même, `probe_demo.py`) : plan validé, exécuté, journal rejoué → `identical: true`.
