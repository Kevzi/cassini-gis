---
title: 'Suite de bout en bout : ouvrir, analyser, exporter, rejouer'
type: 'feature'
ticket: '14'
created: '2026-10-16'
status: done
baseline_revision: '5b3e8c47748d4fc8c6dbf460c7c2cace76d1ae98'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/docs/canal-local.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** rien ne prouve la chaîne entière, et le rejeu d'un journal à étapes QGIS est impossible : `cassini replay` n'a que le bouchon QGIS (`registry.unavailable`), les `layer_id` changent à chaque session, et la comparaison d'octets des sorties QGIS (GeoPackage, PDF) diverge.

**Approach:** le noyau rejoue par le canal (`journal.replay`, même exécutant QGIS que l'exécution d'origine, AD-11) en liant les `layer_id` anciens aux neufs ; une sonde sous le vrai QGIS et le vrai binaire enchaîne `layer.add`, `analysis.buffer`, `layout.export_pdf`, ferme, rouvre le projet et rejoue.

## Boundaries & Constraints

**Always:** le rejeu passe par le même `Router` et les mêmes résolutions que l'exécution (`Resolver`) ; il n'écrit pas le journal, ses sorties vont sous `replay/` ; il s'arrête au premier écart en nommant la commande ; aucune logique métier dans le Python ; le noyau passe ses tests sans QGIS ; fixtures synthétiques seulement.

**Never:** pas de bouton de rejeu dans le panneau (agent, epic 5) ; pas de `qgis_process` ni de rejeu sans QGIS (reporté, contrat d'architecture) ; pas de sortie QGIS comparée octet par octet ; pas de rejeu annulable ; pas de nouvelle dépendance ; aucune donnée téléchargée.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Rejeu nominal | journal de 3 étapes, projet rouvert, extension branchée | `{steps: 3, identical: true}` ; fichiers sous `replay/` | — |
| `layer_id` d'une étape | étape *k* a rendu l'id A, rejeu rend B | les paramètres suivants égaux à A deviennent B ; `layer_id` ignoré à la comparaison | — |
| Sortie QGIS | gpkg ou PDF d'octets différents | acceptée si le fichier existe et n'est pas vide | sinon `journal.diverged` |
| Extension absente | aucune commande QGIS annoncée | — | `registry.unavailable` nommant la commande |
| Résultat différent | `feature_count` ou `layer_count` change | — | `journal.diverged`, étape nommée |
| Autre requête pendant le rejeu | `command.run` reçu | — | `session.busy` |

</frozen-after-approval>

## Code Map

- `core/src/journal.rs` -- `Session::call` écrit la ligne : ajouter le champ optionnel `layer_id` (id rendu par l'étape). `Replay::run` : table ancien→neuf, substitution dans les chaînes des paramètres, masque de `layer_id` des deux côtés, octets comparés seulement hors `RunsIn::Qgis`. Tests : `FakeQgis`, `InventoryQgis`, `replay_of`.
- `core/src/serve.rs` -- `dispatch` : méthode `journal.replay` ; routeur `Router::new(registry).with_qgis(Channel{link})`, sans `run_id` (non annulable) ; `answer_during_order` rend déjà `session.busy`. Tests sur le gabarit `exchange`.
- `cli/src/main.rs` -- `replay` garde le bouchon : ne pas toucher.
- `qgis-plugin/cassini_qgis/client.py` -- `replay()` envoie `journal.replay`, événements `REPLAY_DONE` / `REPLAY_FAILED` ; réutiliser `_request`, le suivi `_run`.
- `qgis-plugin/tests/test_tracer.py` -- gabarit `FakeProcess`/`reply` pour les tests du client.
- `qgis-plugin/tests/probe_layout_export.py`, `probe_cancel.py` -- gabarit de sonde sous le vrai QGIS. Le noyau : `CASSINI_CORE`, sinon `target/release/cassini.exe`, sinon `target/debug/`.
- `fixtures/points-2154.csv` (nouveau), `fixtures/README.md` -- 4 points synthétiques en Lambert-93 (le tampon exige un SCR métrique).
- `docs/canal-local.md`, `qgis-plugin/README.md` -- document de `journal.replay` et la recette.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/journal.rs` -- champ `layer_id`, liaison, masque, octets hors QGIS, tests -- rejeu possible
- [ ] `core/src/serve.rs` -- `journal.replay`, tests (nominal, extension absente, occupé) -- rejeu par le canal
- [ ] `qgis-plugin/cassini_qgis/client.py`, `tests/test_tracer.py` -- `replay()` et événements, tests avec doublure -- côté extension
- [ ] `fixtures/points-2154.csv`, `fixtures/README.md` -- fixture -- données de la sonde
- [ ] `qgis-plugin/tests/probe_end_to_end.py` -- projet temporaire ; `layer.add` (csv, `crs`), `analysis.buffer`, `layout.export_pdf` ; fermeture ; rouverture ; `replay()` ; compare résultats, PDF `%PDF` -- preuve
- [ ] `docs/canal-local.md`, `qgis-plugin/README.md` -- méthode et recette -- documentation
- [ ] `deferred-work.md` -- clore l'entrée « rejeu de `analysis.buffer` » -- suivi

**Acceptance Criteria:**
- Given le vrai binaire, QGIS 3.44 et la fixture, when la sonde tourne, then elle imprime `RÉSULTAT : OK`, le fichier d'origine est inchangé (empreinte) et le rejeu rend `identical: true` avec 3 étapes.
- Given un journal dont une étape QGIS donne un autre résultat, when il est rejoué, then le rejeu s'arrête en nommant l'étape.
- Given aucun QGIS installé, when `cargo test` tourne, then tout passe.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| `remove_dir_all` ignoré : un fichier périmé non vide passerait la règle « existe et non vide » | medium (non vérifié) | defer | Exige un échec de suppression (Windows, fichier tenu par QGIS) ET une étape QGIS qui n'écrit rien ; la sonde vide le projet avant le rejeu. |
| Anciens journaux à chemins absolus divergent au rejeu | low | rejeté | Aucun journal à étape QGIS n'était rejouable avant ce changement (bouchon). |
| Masque `layer_id` inopérant si le résumé est tronqué | low | rejeté | Ids QGIS de longueur constante ; correctif = branche en plus, déjà signalé par l'implémenteur. |
| Liaison seulement sur `layer_id` de premier niveau, égalité de chaîne entière | low | rejeté | Seul contrat actuel des commandes (`layer.add`, `analysis.*`). |
| `ReplayTest` après le bloc `__main__` | low | patch (corrigé) | Bloc déplacé en fin de fichier ; 46 puis 171 tests OK. |
| 5 étapes au lieu de 3 (le `system.about` de chaque démarrage est journalisé) | low | rejeté | Le correctif éditerait le plan ; `identical: true` tient, la sonde le documente. |
| Étape QGIS en erreur avec liaison d'id non testée | low | rejeté | Chemin d'erreur existant inchangé ; improbable. |


## Design Notes

AD-11 voulait des repères `out:k.n` ; `layer.add` n'a pas de sortie fichier, donc le `layer_id` rendu par l'étape *k* est lié à celui du rejeu (écart assumé, à écrire dans le contrat). Octets : GeoPackage et PDF portent dates et identifiants, la preuve de fidélité des sorties QGIS est le résultat typé comparé.

## Verification

**Commands:**
- `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propre
- `python -I -m unittest discover -s qgis-plugin/tests` et `python -I qgis-plugin/tests/check_extension.py` -- expected: OK
- `cargo build --release` puis `python-qgis-ltr.bat qgis-plugin/tests/probe_end_to_end.py` -- expected: `RÉSULTAT : OK`
