---
title: 'Rejeu sans poignée de main : system.about ne s''ajoute plus au journal'
type: 'bugfix'
ticket: '32'
created: '2026-10-31'
status: 'built'
baseline_revision: '2ebe68b1b8f47b3b38467138a81cc715498489a6'
route: 'full'
route_source: 'pinned'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 2
context:
  - '{project-root}/docs/canal-local.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** À l'ouverture d'un projet, l'extension envoie `session.hello` puis `command.run` de `system.about` pour lire les versions du noyau ; le noyau journalise cet appel comme une étape. Chaque rouverture, et donc chaque rejeu dans QGIS, allonge le journal d'une ligne (19 étapes devenues 20 à la recette de 5.31). Ce `system.about` servait aussi, sans le dire, à révéler au démarrage qu'une seconde session sur un projet déjà tenu est en lecture seule (`journal.read_only`).

**Approach:** Décisions du propriétaire (2026-10-31) : (1) l'extension prend `core_version` et `protocol_version` dans la réponse de `session.hello`, qui les porte déjà, et n'appelle plus `system.about` au démarrage ; (2) la réponse de `session.hello` dit elle-même si la session est en lecture seule (champ ajouté, compatible, `PROTOCOL_VERSION` reste 2), et l'extension bloque alors au démarrage avec le message de lecture seule du noyau, comme avant. Les nouveaux journaux ouverts par la nouvelle extension ne portent plus de ligne `system.about` d'accueil.

## Boundaries & Constraints

**Always:** l'événement `READY` garde sa forme (`core_version`, `protocol_version`, `commands`) ; une erreur de `session.hello` reste rendue comme aujourd'hui (`FAILED`, noyau arrêté) ; une session en lecture seule rend `FAILED` au démarrage avec le message du noyau, tel quel ; les journaux déjà écrits avec des lignes `system.about` rejouent à l'identique par la même version du noyau ; `system.about` reste une commande du registre, lancée à la main (mode expert, CLI, MCP) et alors journalisée.

**Never:** changer le noyau au-delà de l'ajout du champ de lecture seule à la réponse de `session.hello` et de ses tests ; changer `PROTOCOL_VERSION` ; réécrire ou filtrer un journal existant ; masquer `core_version` au rejeu des anciens journaux (reste une entrée différée).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Démarrage | réponse de `session.hello` avec versions, catalogue, session en écriture | `READY` avec ces versions et ce catalogue ; le client n'envoie aucun `command.run` | — |
| Démarrage, champs absents | réponse sans `core_version` | `READY` avec `core_version: None` | pas d'échec |
| Seconde session | le journal du projet est verrouillé par une autre session | la réponse de `session.hello` le dit ; `FAILED` avec le message de lecture seule du noyau, noyau arrêté | aucun `READY` |
| Refus de la poignée de main | erreur `session.protocol_mismatch` | message affiché tel quel, noyau arrêté | inchangé |
| Rejeu d'un journal neuf | projet rouvert, `journal.replay` | le nombre de lignes du journal ne bouge pas | — |
| Rejeu d'un ancien journal | journal portant une ligne `system.about` d'accueil, même version | `identical: true`, lignes inchangées octet pour octet | — |

</frozen-after-approval>

## Code Map

- `core/src/serve.rs:819-881` -- `hello` : rend `protocol_version`, `core_version`, `commands`. Ajouter `"read_only": bool` (toujours présent) d'après `Session::mode()` (`core/src/journal/mod.rs:186`, `Mode::ReadOnly`), et, seulement quand il vaut `true`, `"message"` : le texte de `JournalError::ReadOnly` (`journal/mod.rs:91-93`, via `ErrorReport::from(JournalError::ReadOnly)` ou son `Display`) — le noyau reste la source du texte. Rien d'autre ne change dans `hello`.
- `core/src/serve.rs:1018` -- test `hello_then_about_journals_one_line` : modèle (`exchange`, `hello_line`, `run_line`, `project_in`, `temp_dir`). Pour la lecture seule : tenir le verrou avec une `Session::open` du même projet gardée vivante pendant un `exchange` (voir les tests de verrou existants, `grep -n "lock\|read_only" core/src/journal/tests.rs cli/src/main.rs`).
- `core/src/serve.rs:954` -- `journal_replay` : passe par `Router`, n'écrit pas au journal. Ne pas modifier.
- `qgis-plugin/cassini_qgis/client.py:470-495` -- `_handle`, branches `hello` et `about` : la branche `hello` doit émettre `READY`, ou `_fail(message)` si `read_only` est vrai ; supprimer la branche `about`. Docstring l.3-5 et commentaire de `READY` l.27 (le payload porte aussi `commands`) à corriger.
- `qgis-plugin/cassini_qgis/plugin.py:188-199` -- `_on_event` : sur `FAILED`, jette aujourd'hui le `payload` et affiche le bandeau fixe `refused` (« versions incompatibles »). Passer le message reçu au panneau : `self.panel.set_banner("refused", payload)` (et `_banner(kind, message=None)` l.148 qui le relaie). `DIED` inchangé.
- `qgis-plugin/cassini_qgis/panel.py:121-127,519-528,724-737` -- `BANNERS`, `set_banner`, rendu du bandeau. `set_banner(kind, message=None)` garde `self.banner_message` (remis à `None` à chaque appel et par `set_ready`) ; le rendu et `_announce` affichent `message` tel quel quand c'est une chaîne non vide, sinon `t(key)` comme aujourd'hui. Bouton `choose` du bandeau `refused` inchangé. Le double de test du panneau (`test_tracer.py:525-530`, liste `banners`) accepte le nouvel argument.
- `qgis-plugin/tests/test_tracer.py:605-611` -- `test_protocol_refusal_is_not_a_relaunchable_death` : modèle des tests de plugin pour `FAILED`.
- `qgis-plugin/tests/test_tracer.py:115-137` -- test de démarrage à réécrire ; les autres suites (`test_plan_client.py`, `test_external.py`) répondent au `system.about` d'accueil : retirer ces réponses, la première requête après le hello prend l'`id` 2.
- `qgis-plugin/tests/probe_end_to_end.py:21-22,196-200,208,223-227` -- journal attendu `STEP_IDS`, rejeu en `len(STEPS)` étapes, garde `layer_id` sur `rows[:-1]`.
- `docs/canal-local.md:99-100` et la section `session.hello` du même fichier ; `qgis-plugin/README.md:38`.
- `_bmad-output/initiative-cassini-mvp/deferred-work.md:142-143` -- entrée `core_version`.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/serve.rs` -- ajouter `read_only` (et `message` quand vrai) à la réponse de `hello` -- la seconde session le dit au démarrage.
- [x] `core/src/serve.rs` (tests) -- (a) `session.hello` seul : le fichier journal n'existe pas (assertion explicite sur l'absence, pas un compte qui vaut 0 sur erreur), `read_only == false`, `core_version` et `protocol_version` présents ; puis hello + une commande écrit une ligne ; (b) un journal `system.about` + une commande, rejoué deux fois dans deux sessions par `journal.replay` : `{"steps": 2, "identical": true}` chaque fois et le contenu du journal identique octet pour octet à celui d'avant les rejeux ; (c) verrou tenu par une autre session : `read_only == true` et `message` égal au texte de `journal.read_only`.
- [x] `qgis-plugin/cassini_qgis/client.py` -- branche `hello` : `read_only` vrai → `_fail(message)` ; sinon `READY` (versions du résultat, `None` si absentes, catalogue) ; sans `command.run`. Supprimer la branche `about` ; docstring et commentaire `READY` à jour.
- [x] `qgis-plugin/cassini_qgis/plugin.py`, `qgis-plugin/cassini_qgis/panel.py` -- sur `FAILED`, le bandeau `refused` affiche le message reçu tel quel (lecture seule comme refus de version), repli sur le texte `refused` quand le message est absent ou vide -- critère 3 : la seconde fenêtre dit pourquoi elle n'est pas prête.
- [x] `qgis-plugin/tests/test_tracer.py` (et suites touchées) -- démarrage : un seul message écrit par le client, `READY` avec les versions du hello ; versions absentes → `None` ; `read_only: true` → `FAILED` avec le message tel quel, noyau arrêté, aucun `READY`. Plugin et panneau : `FAILED` avec le texte de lecture seule → bandeau `refused` portant ce texte ; `FAILED` avec un message vide → texte `refused` habituel ; `set_ready` efface le message.
- [x] `qgis-plugin/tests/probe_end_to_end.py` -- attentes sans `system.about` ; après le rejeu nominal, relire le journal et vérifier qu'il compte toujours `len(STEPS)` lignes.
- [x] `docs/canal-local.md`, `qgis-plugin/README.md` -- décrire : le démarrage ne lance aucune commande journalisée (le panneau peut demander `agent.models`, non journalisé) ; le champ `read_only` de la réponse de `session.hello`, et que le panneau en affiche le message ; sans prétendre que le journal compte « tout ce que le noyau exécute » (`model.list`, ordres d'inventaire ne sont pas journalisés). Dans le README, des phrases courtes, sans incise entre tirets.
- [x] `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- annoter l'entrée `core_version` : les journaux ouverts par l'extension 5.32 n'ont plus de `system.about` d'accueil ; restent exposés les anciens journaux (dès l'étape 1, à la première montée de version), un `system.about` lancé à la main, et une extension antérieure à 5.32 couplée à un noyau neuf.

**Acceptance Criteria:**
- Given un projet ouvert dans QGIS avec un journal de N étapes, when on rejoue deux fois son journal, then le journal compte toujours N étapes et chaque rejeu rend `identical: true`.
- Given le panneau ouvert, when on lance `system.about` en mode expert, then le résultat s'affiche et le journal porte cette ligne, comme avant.
- Given un projet déjà ouvert dans une première fenêtre QGIS, when on l'ouvre dans une seconde, then le panneau de la seconde affiche le message de lecture seule et ne se dit pas prêt.

## Implementation Notes

## Plan Change Log

- Boucle 1 (2026-10-31). Constat : relectures quick et edge-case — sans le `system.about` d'accueil, une seconde session en lecture seule se disait prête (`journal.read_only` n'apparaissait qu'à la première commande), contre AD « le dit explicitement ». Décision du propriétaire : la réponse de `session.hello` porte `read_only`. Amendé : Intent, Boundaries, matrice (bloc figé, par le propriétaire), Code Map, tâches. Évité : un panneau « prêt » sur une session qui ne peut rien exécuter. Intégrés aussi : assertion d'absence du journal au lieu de `unwrap_or(0)`, comparaison octet pour octet après rejeu, second contrôle du journal dans la sonde, doc sans « un seul message » ni « rien d'autre », commentaire `READY`, note `deferred-work` nuancée. KEEP : branche `hello` qui émet `READY` avec `None` pour les versions absentes ; ids réalignés (`reply(2, …)` pour la proposition dans `test_plan_client.py`) ; garde `layer_id` sur `rows[:-1]` dans la sonde ; tests Rust en deux `exchange` sur le même `project` (le verrou est relâché entre eux).
- Boucle 2 (2026-10-31). Constat : relectures verification-gap et edge-case — `plugin.py:194-199` jette le `payload` de `FAILED` et affiche le bandeau fixe « versions incompatibles » ; la seconde fenêtre dit donc à l'utilisateur d'installer des versions assorties, contre le critère 3 et la matrice (« message affiché tel quel »). Le Code Map s'arrêtait au client. Amendé : Code Map (`plugin.py`, `panel.py`, test de plugin), tâche plugin/panneau, tests du plugin, doc ; ajoutés aussi l'assertion de `protocol_version` dans le test Rust (a) et le README en phrases courtes. Évité : un message trompeur sur la cause du blocage, tests verts. KEEP (diff de la boucle 1, bon) : `hello` du noyau avec `read_only` toujours présent et `message` = `JournalError::ReadOnly.to_string()` seulement si vrai ; les trois tests Rust `hello_alone_writes_no_journal_and_says_the_session_writes`, `replaying_twice_leaves_the_journal_byte_for_byte`, `a_second_session_says_read_only_at_hello` ; branche `hello` du client avec `isinstance(result, dict)`, `read_only is True` → `_fail`, repli de texte quand `message` manque ; tests Python `test_hello_without_versions_is_ready_with_none`, `test_read_only_session_fails_at_hello_with_the_core_message`, `test_writing_session_is_ready` ; réponses `reply(2, …)` du `system.about` d'accueil retirées des suites ; sonde (`STEP_IDS`, `len(STEPS)`, relecture du journal) ; section `session.hello` de `docs/canal-local.md` et puce du rejeu réécrite ; note `deferred-work` listant les trois cas restés exposés.

## Review Triage Log

Passe 1 (quick, edge-case, adversarial) — 1 high (intent_gap), 6 low, 7 rejetés.
- high, intent_gap — seconde session en lecture seule dite prête (quick, edge-case) : vérifié, `Session::open` → `Mode::ReadOnly` (`journal/mod.rs:157-160`), `hello` ne le dit pas. Tranché par le propriétaire, plan amendé.
- low — doc « l'extension n'envoie que `session.hello` » : faux, `plugin.py:191-193` envoie `agent.models` (non journalisé). Intégré au plan.
- low — doc « tout ce que le noyau a exécuté, et rien d'autre » contredit par `model.list` et les ordres d'inventaire non journalisés. Intégré.
- low — `journal_line_count` avec `unwrap_or(0)` : l'assertion « rien écrit » passe sur une erreur de chemin. Intégré.
- low — test de rejeu ne compare que le nombre de lignes. Intégré (octet pour octet).
- low — commentaire `READY` périmé (`client.py:27`). Intégré.
- low — note `deferred-work` trop affirmative (`system.about` à la main, extension ancienne). Intégré.
- rejeté (low, coût) — la sonde ne rejoue pas deux fois : un second rejeu QGIS complet alourdit la sonde ; le double rejeu est prouvé sans QGIS et la sonde relit désormais le journal ; la recette du propriétaire rejoue deux fois.
- rejeté (false) — `protocol_version` absent accepté : le noyau refuse déjà toute version différente à `session.hello` (`serve.rs:833-848`).
- rejeté (low) — l'écriture du journal n'est plus éprouvée au démarrage : disque plein ou ACL sur le seul fichier, rare ; la première commande échoue avec le message d'E/S.
- rejeté (low) — les tests Rust passaient déjà avant : ils prouvent les propriétés du noyau demandées par le ticket ; le changement de l'extension est couvert par les tests Python.
- rejeté (low) — `replay()` sur un projet neuf rend `replay.no_journal` : aucun appelant du panneau ne le fait.
- rejeté (low) — pas de test « noyau mort après READY » : chemin `_finished` inchangé.
- rejeté (low) — décalage des numéros d'étape dans les nouveaux projets : la sonde les tire du journal ; comportement voulu.
- défer (préexistant) — les anciens journaux divergeront dès l'étape 1 à la première montée de version (`core_version`) : entrée existante, annotée par la tâche `deferred-work`.

Passe 2 (blind-hunter, edge-case-hunter, verification-gap, intent-alignment) — 1 medium (bad_plan), 2 low intégrés, 10 rejetés.
- medium, bad_plan — le panneau de la seconde fenêtre affiche « versions incompatibles » au lieu du message de lecture seule (verification-gap, edge-case ×2) : vérifié, `plugin.py:194-199` ignore `payload` et `texts.py:50-53` code en dur le refus de version ; critère 3 et ligne « Refus de la poignée de main » de la matrice non tenus. Boucle 2.
- low — `hello_alone…` ne vérifie pas `protocol_version` alors que l'extension ne le lit plus qu'au hello (blind) : vérifié. Intégré au plan (boucle 2).
- low — phrase du README illisible, incise entre tirets (blind) : vérifié. Intégré au plan (boucle 2).
- rejeté (low) — extension neuve sur noyau antérieur à 5.32 : `read_only` absent, la seconde session se dit prête et la lecture seule ne sort qu'à la première commande, avec le message du noyau (blind, edge-case, intent) : vérifié ; cas rare (deux fenêtres sur le même projet et paquets désassortis), sans perte de données ; le refuser casserait le couplage compatible voulu par l'Intent.
- rejeté (low) — texte de repli écrit par l'extension quand `message` manque (blind, intent) : jamais atteint avec le vrai noyau, qui l'envoie toujours.
- rejeté (low) — pas de test du repli (blind) : même raison.
- rejeté (false) — `read_only` non booléen accepté comme écriture (blind, edge-case) : le noyau sérialise un `bool` (`serve.rs`, `json!(read_only)`), aucune autre valeur n'est émise.
- rejeté (false) — commentaire du test de rejeu (« puis une commande ») faux (blind) : le second `system.about` est bien une commande du registre.
- rejeté (low) — pas de test Rust de rejeu d'un journal neuf (blind) : `hello_alone…` prouve que le hello n'écrit rien, le rejeu passe par le même `journal_replay` ; la sonde couvre le journal neuf sous QGIS.
- rejeté (low) — le test de lecture seule ne vérifie pas l'absence d'écriture (blind) : une session `ReadOnly` refuse toute écriture par construction (`journal/mod.rs:209`), testé dans `journal/tests.rs`.
- rejeté (low, préexistant) — `hello` publie le catalogue externe même en lecture seule (blind, edge-case) : comportement d'avant 5.32, le noyau est arrêté aussitôt par l'extension ; toute commande y répond `journal.read_only`.
- rejeté (low) — Implementation Notes vide, `[…]` dans les blocs JSON (blind) : exemples illustratifs, sans effet.
- rejeté (low, coût) — la sonde ne teste pas la seconde session (blind, intent) : contrôle manuel du propriétaire prévu ; les tests de plugin de la boucle 2 couvrent l'affichage.

Passe 3 (blind-hunter, edge-case-hunter, verification-gap, intent-alignment, après boucle 2) — 4 patch, 1 défer, 12 rejetés ou reportés de la passe 2.
- low, patch — le vrai `render()` du bandeau n'est jamais exécuté par les tests (`PanelBannerTest` le remplace par `lambda: None`) : remettre `setText(t(key))` laisserait tout vert (verification-gap, blind). Test ajouté.
- low, patch — `PanelBannerTest` laisse `cassini_panel_stub*` dans `sys.modules` (blind) : vérifié, `restore()` ne nettoie que `qgis*`.
- low, patch — la sonde ne compare que le nombre de lignes du journal après rejeu (blind) : vérifié ; comparer le contenu relu avant et après.
- low, patch — `docs/canal-local.md` ne montre pas la réponse en lecture seule (blind) : vérifié, seul `"read_only":false` est en exemple.
- défer (préexistant) — bandeau de lecture seule avec « Choisir le noyau » et sans « Relancer » (blind, edge-case, intent) : vérifié `panel.py:126` ; avant 5.32 le même cas tombait déjà sur `refused`/`choose`. Entrée `deferred-work`.
- rejeté (low) — en anglais, le refus de version et la lecture seule montrent le texte français du noyau (blind) : vérifié ; même règle que les erreurs de commande, déjà affichées telles quelles depuis le noyau ; le message du noyau dit en plus quel côté mettre à jour (`serve.rs:833-848`).
- rejeté (low) — pas de test Rust que `read_only` repasse à `false` après libération du verrou (blind) : la libération est déjà couverte dans `journal/tests.rs` ; `hello` ne fait que lire `Session::mode()`.
- rejeté (low, préexistant) — nettoyage des dossiers temporaires seulement en cas de succès dans les tests Rust (blind) : même motif que tous les tests de `serve.rs`.
- rejeté (false) — `set_banner` garde un message pour un autre bandeau que `refused` (edge-case) : aucun appelant ne le fait, `plugin.py` ne passe un message qu'avec `refused`.
- rejeté (low) — Code Map à numéros de ligne, Implementation Notes vide (blind) : sans effet sur le livrable.
- carried, rejeté (low) — extension neuve sur noyau antérieur à 5.32 (blind, edge-case, intent) : voir passe 2.
- carried, rejeté (low) — texte de repli en dur dans `client.py` (blind, edge-case, intent) : voir passe 2.
- carried, rejeté (false) — `read_only` non booléen (edge-case) : voir passe 2.
- carried, rejeté (low, préexistant) — catalogue externe publié en lecture seule (edge-case) : voir passe 2.
- rejeté (false) — le panneau va au-delà de l'Intent (intent) : la matrice figée exige le message « tel quel » pour le refus de la poignée de main, et le critère 3 pour la lecture seule.

## Design Notes

Le noyau ne sait pas distinguer un `system.about` d'accueil d'un `system.about` voulu : la correction du journal est du côté de l'appelant. Le message de lecture seule reste écrit par le noyau (le canal ne porte que des résultats typés, l'extension affiche tel quel).

```json
{"protocol_version":2,"core_version":"0.1.0","read_only":true,
 "message":"Le journal de ce projet est tenu par une autre session : …","commands":[…]}
```

## Verification

**Commands:**
- `cargo test -p cassini-core serve` -- expected: verts, nouveaux compris.
- `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` -- expected: verts (sous WSL la compilation complète échoue sur GTK/libclang : cargo Windows avec `LIBCLANG_PATH` et `CARGO_TARGET_DIR=C:\ct`).
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: OK.
- `python3 -I qgis-plugin/tests/check_extension.py` -- expected: propre.

**Manual checks:**
- Recette QGIS (propriétaire) : installer noyau et extension ; ouvrir un projet, rejouer deux fois son journal : le nombre d'étapes ne bouge pas ; ouvrir le même projet dans une seconde fenêtre : message de lecture seule.
