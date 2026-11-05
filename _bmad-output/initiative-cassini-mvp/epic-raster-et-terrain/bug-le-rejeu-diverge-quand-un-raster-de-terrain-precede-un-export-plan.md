---
title: 'Le rejeu diverge quand un raster de terrain précède un export PDF'
type: 'bugfix'
ticket: '9'
created: '2026-11-05'
status: 'built'
baseline_revision: '4d4aadcef72196e1508e44b35c95698faafce167'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Un export PDF rejoué après un raster de terrain du noyau ne montre pas les mêmes couches qu'à l'origine : `replay.diverged` à l'étape 5 dans la même session (les rasters d'origine, ouverts par le panneau sans `layer_id` au journal, ne sont pas laissés hors de la carte), à l'étape 12 dans une session neuve (le rejeu ne rouvre pas les rasters qu'il recalcule).

**Approach:** Décision du propriétaire (2026-11-05, option C) : l'ouverture d'un raster du noyau devient une commande du registre déléguée à QGIS, `layer.open_raster`, que le noyau dicte lui-même juste après chaque étape qui rend un raster, et qu'il journalise avec son `layer_id`. Le rejeu la rejoue et la laisse hors de la carte par le mécanisme de 5.36 ; le panneau n'ouvre plus de raster de lui-même. Le complément 4.3 d'AD-9 est amendé en conséquence.

## Boundaries & Constraints

**Always:** le noyau décide, l'extension exécute (AD-5, AD-10) ; journal relatif (AD-9) ; aucun échec silencieux (AD-21) : un raster qu'on ne peut pas ouvrir est une étape en erreur, nommée. `layer.open_raster` n'est offerte ni au modèle (prompt et grammaire inchangés à l'octet : le modèle installé reste valide), ni au client MCP, ni au formulaire du panneau. Sans QGIS branché (CLI `run`, MCP autonome) ou si l'extension ne l'annonce pas, rien n'est dicté. Le cas sans raster de 5.36 reste vert.

**Never:** comparer l'export autrement pour faire passer le rejeu ; supprimer une couche du projet ; toucher au générateur d'entraînement ; ajouter une méthode au protocole.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Ouverture dictée | `terrain.slope` par `command.run`, plan ou IA externe | ligne `terrain.slope` puis ligne `layer.open_raster` (même `plan_step` dans un plan) ; le raster est dans le projet | — |
| Plan pente puis export | un plan : `terrain.slope`, `layer.export_pdf` | la carte du PDF montre la pente | — |
| Même session | chaîne raster puis export ; rejeu sans rouvrir | `identical: true` ; raster d'origine hors de la carte | — |
| Session neuve | même journal, projet rouvert sans les rasters | `identical: true` ; raster recalculé ouvert avant l'export | — |
| Deux rejeux de suite | même session | deux fois `identical: true` | — |
| Ligne de vue | `terrain.line_of_sight` (aucun fichier) | rien n'est dicté | — |
| Sans QGIS / non annoncée | CLI `run`, ou extension sans le geste | seule l'étape de terrain est journalisée | — |
| Raster illisible | fichier absent ou invalide | étape `layer.open_raster` en erreur, journalisée, montrée au panneau | erreur nommée |

</frozen-after-approval>

## Code Map

- `core/src/commands/layer_add.rs` -- patron d'une commande déléguée (`runs_in(Qgis)`, `path_params(["path"])`, sortie `layer_id`, `name`). Nouvelle `layer_open_raster.rs` sur ce patron ; le registre se construit par `build.rs`.
- `core/src/registry.rs` l.30-140 `CommandSpec` -- aucun champ ne cache une commande : en ajouter un (ex. `dictated_by_core`), lu par `ai/src/plan.rs` (`plan_grammar` l.115, prompt l.291, `catalog_entry` l.232), `cli/src/mcp.rs` (l.83 autonome, relais l.~109) et le catalogue envoyé au panneau (`panel.py` l.994-1014 grise/liste).
- `core/src/journal/mod.rs` l.246 `call_logged` -- point de passage des trois flux (`serve.rs` `command_run` l.936, `serve/plans.rs` l.326 `call_in_plan`, `serve/calls.rs` l.112 `call_from(.., "external")`). Enchaîner ici (ou dans une enveloppe de `Session`) : si le résultat d'une commande `runs_in: core` porte une sortie raster (`output_ext` `.tif`) et que l'exécutant QGIS annonce `layer.open_raster`, appeler `layer.open_raster {path: "out:<k>.1"}` avec le même `origin` et le même `plan_step`. Une ligne sans `plan_step` vide l'état des intermédiaires (`intermediate_files` l.432-446) : ne jamais l'omettre dans un plan.
- `core/src/journal/mod.rs` l.546-580 `resolve_param`/`resolve_reference` -- `out:k.n` journalisé tel quel, résolu vers `out_dir` (dossier de rejeu au rejeu, même nom `<k>-terrain.slope.tif`). `Replay::run` l.776-880 rejoue la ligne, lie le `layer_id`, `leave_out` couvre les ids d'origine et `replayed_layers` : rien à changer en principe.
- `core/src/serve.rs` l.922 `check_available` -- ne vaut que pour `command.run` : la disponibilité de `layer.open_raster` se teste avant de dicter (`state.announced`).
- `qgis-plugin/cassini_qgis/raster_output.py` -- garder `QgisBackend.add_raster`/`is_open`/`same_file` pour le geste ; retirer `to_open`/`outputs_of`/`report` s'ils ne servent plus.
- `qgis-plugin/cassini_qgis/panel.py` l.646, 688, 723, 739 -- retirer `_open_core_rasters` et ses trois appels.
- `qgis-plugin/cassini_qgis/plugin.py` l.178-184 -- enregistrer le geste ; `gestures.py` (`start(params, done)`, `cancel()`) ; patron `layer_add.py`. Résultat `{layer_id, name}` ; nom = radical du fichier ; déjà ouvert → même couche, pas de doublon.
- Contrat : `architecture-noyau-et-facade-qgis.md` AD-9, « Complément (2026-11-04, ticket 4.3) » -- ajouter un amendement daté.
- Tests à imiter : `core/src/journal/tests.rs` l.1212, 993, 271, 727, 1457, 1482 ; `core/src/serve.rs` l.1080, 1982, 2076, 2814 ; `qgis-plugin/tests/test_raster_output.py`, `test_panel_logic.py` l.38-62, `probe_raster_output.py`.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/layer_open_raster.rs` (nouveau), `core/src/registry.rs` -- commande déléguée et champ qui la cache ; tests de schéma
- [x] `ai/src/plan.rs`, `cli/src/mcp.rs`, catalogue du panneau -- exclure les commandes dictées ; test : prompt et grammaire identiques à l'octet à ceux d'avant
- [x] `core/src/journal/mod.rs` (ou `Session`) -- enchaînement après une sortie raster, `origin` et `plan_step` repris, seulement si annoncée ; tests : trois flux, ligne de vue, sans QGIS, échec d'ouverture
- [x] `core/src/journal/tests.rs`, `core/src/serve.rs` -- rejeu raster puis export : même session, session neuve, deux rejeux ; 5.36 gardé vert
- [x] `qgis-plugin/cassini_qgis/` -- geste `layer.open_raster`, enregistrement, retrait de l'ouverture par le panneau ; tests Python mis à jour
- [x] `architecture-noyau-et-facade-qgis.md` -- amendement d'AD-9 (complément 4.3) daté du 2026-11-05, ticket 4.9
- [x] installation (noyau `--features gpu`, extension) puis sonde QGIS réelle sur la chaîne de la reproduction du ticket

**Acceptance Criteria:**
- Given le projet réel et la chaîne de la reproduction, when le journal est rejoué dans la même session puis dans une session neuve, then les deux rejeux rendent `identical: true`, et chaque export rejoué montre les mêmes couches que l'export d'origine.
- Given les tests du noyau et de l'extension, when ils tournent, then un test qui échoue sans le correctif couvre la même session, un autre la session neuve, un troisième garde vert le cas de 5.36.

## Implementation Notes

- Champ `CommandSpec::dictated_by_core` (sérialisé seulement quand il est vrai : le catalogue des autres commandes est inchangé) ; `layer.open_raster` le porte, `cancellable: false`, `path_params: ["path"]`.
- L'enchaînement vit dans `Session::call_logged` (point de passage des trois flux) : `call_step` exécute et journalise, puis `rasters_written` rend un `out:<k>.<n>` par sortie d'une commande `runs_in: core` d'extension `.tif`/`.tiff`, si la session a été ouverte `with_raster_opening(true)` (posé à `session.hello` quand l'extension annonce la commande). Même `origin`, même `plan_step`. Une ouverture en échec rend son erreur à l'appelant (la pente reste journalisée, réussie). Le résultat rendu est celui de la commande demandée : les références `step:k` d'un plan ne changent pas.
- `is_available` rend faux pour une commande dictée (catalogue des relais, vérification d'un plan) ; `check_available` la refuse sous `registry.dictated` (`command.run`, appels des relais).
- `ai/src/plan.rs` : `offered()` filtre la grammaire et le catalogue du prompt ; test d'égalité à l'octet contre un registre sans la commande (`Registry::remove`, ajouté pour ce test).
- `cli/src/mcp.rs` : filtre aussi `dictated_by_core` dans les deux catalogues (le relais l'excluait déjà par `available`).
- `intermediate_files` : une couche ouverte par `layer.open_raster` (et elle seule) sur `out:<k>.<n>` est reconnue par le fichier de cette sortie (helper `reference_file`, partagé avec `Resolver::resolve_reference`) ; la ligne dictée garde le `plan_step` de la pente, comme demandé.
- Extension : `raster_output.py` devient le geste `LayerOpenRasterGesture`, qui ajoute toujours une couche neuve nommée du radical du fichier (relecture : rendre la couche déjà ouverte faisait diverger un second rejeu qui réécrit le même fichier de `replay/`, et suivait un renommage) ; `to_open`, `outputs_of`, `open_rasters`, `report` et le texte `raster_unreadable` sont retirés ; `form.offered` cache la commande dictée de la liste du panneau.

## Plan Change Log

## Review Triage Log

Relecture `thorough` du 2026-11-05 (quatre lentilles). Verdicts : 1 high, 1 medium (patch), 4 low (patch), 2 reportés, 11 rejetés.

| # | Constat | Verdict | Suite |
|---|---|---|---|
| 1 | Second rejeu de la même session sans verrou de fichier (Linux) : `replay/` vidé et réutilisé, le geste retrouve la couche du rejeu précédent par son chemin, déjà dans `leave_out` ; l'export rejoué perd le raster | high | patch : le geste ajoute toujours une couche neuve, nommée du radical |
| 2 | Couche déjà ouverte et renommée : le geste rendait son nom, pas le radical ; rejeu divergent | medium | patch (même correctif que 1) |
| 3 | `intermediate_files` lit les repères `out:k.n` de toute commande | low | patch : restreint à `layer.open_raster`, cas négatif testé |
| 4 | Aucun test ne vérifie que l'extension enregistre le geste ; test du panneau aux assertions vides | low | patch : `test_launch_registers_the_raster_opening_gesture` |
| 5 | Commentaire de `serve/plans.rs`, `docs/canal-local.md` (ordre dicté, `registry.dictated`), `qgis-plugin/README.md` périmés | low | patch |
| 6 | `Registry::remove` public pour un seul test | low | patch : `#[doc(hidden)]` et commentaire |
| 7 | Extension d'après 4.9 avec noyau d'avant (ou l'inverse) : rasters jamais ouverts, sans message | medium | defer (`deferred-work.md`, avant publication) |
| 8 | Journal écrit avant 4.9 : diverge toujours en session neuve | low | defer (`deferred-work.md`) |
| 9 | Dossiers de rejeu verrouillés sous Windows, `replay.output_locked` après 21 rejeux | false | préexistant : les couches vectorielles rejouées verrouillaient déjà ces dossiers (5.35, 5.36) |
| 10 | Ouverture ratée rendue comme l'erreur de l'étape de terrain | false | voulu : AD-21 et ligne « Raster illisible » de la matrice ; les deux lignes sont journalisées |
| 11 | `cassini replay` sans QGIS bute sur `layer.open_raster` | false | AD-11 : le rejeu échoue en nommant la commande que l'exécutant ne sait pas faire |
| 12 | Plusieurs sorties raster, échec de la première | low, rejeté | aucune commande n'a plusieurs sorties raster |
| 13 | Un plan ne peut pas chaîner le `layer_id` du raster ouvert | low, rejeté | hors du ticket ; aucune commande ne le demande |
| 14 | `cancel()` du geste contre `cancellable: false` | low, rejeté | le noyau n'annule jamais une commande non annulable |
| 15 | Vérification : `pytest` absent | rejeté | correction = éditer le plan ; `unittest discover` consigné |
| 16 | Aucun test d'un plan qui contiendrait `layer.open_raster` | low, rejeté | la grammaire ne l'offre pas ; `command.run` la refuse (`registry.dictated`, testé) |
| 17 | `terrain.viewshed` sans test du noyau ; erreur d'ouverture montrée au panneau sans test Python | low, rejeté | même chemin que la pente (`output_ext` `.tif`) ; viewshed vérifié dans la sonde réelle |
| 18 | Ancien journal et preuve sur doublures (alignement) | — | sonde réelle refaite après les correctifs (ci-dessous) |
| 19 | Comparaison de la sonde : preuve seulement déclarative | — | sorties `sortie-b.txt`, `sortie-rejeu-b.txt` |

## Verification

**Commands:**
- `cargo test -p cassini-core` et `cargo test -p cassini` (WSL) ; `cassini-ai` sous Windows (`powershell.exe`, `LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`, voir `docs/modele-local-sous-windows.md`) -- expected: vert
- `cd qgis-plugin && python3 -m pytest -q` -- expected: vert
- `cargo clippy --all-targets -- -D warnings` (Windows), `cargo fmt --check` -- expected: propres

**Manual checks (if no CLI):**
- Sonde QGIS réelle (Claude, `%TEMP%\sonde-4-9\`, copie de `projet-sonde-4-4.qgz`) : chaîne de la reproduction, rejeu dans la même session puis dans une session neuve, deux fois `identical: true`.

**Results (2026-11-05):**
- `cargo test -p cassini-core` (WSL) : 355 + 32 verts ; `cargo test -p cassini-ai -p cassini` (Windows, `C:\ct`) : verts ; `cargo clippy --all-targets -- -D warnings` (Windows) et `cargo fmt --check` : propres.
- Extension : `python3 -m unittest discover -s tests` : 379 verts (pytest n'est pas installé sous WSL) ; `check_extension.py` : propre.
- Installé : noyau `--features gpu` dans `%LOCALAPPDATA%\Cassini\bin\` (ancien gardé en `cassini.exe.avant-4-9`, renommé car tenu par des processus MCP en cours ; DLL inchangées), extension copiée au profil QGIS.
- Sonde réelle `%TEMP%\sonde-4-9\` (copie de `projet-sonde-4-4.qgz`, modèle `cassini-composition-4b`) : les dix phrases de la reproduction donnent 18 lignes (4 `layer.open_raster` dictées, `plan_step` 1, avec `layer_id`) ; exports à 12, 16 et 17 couches, rasters cités comme à l'origine. Deux rejeux dans la même session : `{"steps": 18, "identical": true}` deux fois ; session neuve (`sonde-rejeu.py`, projet rouvert sans les rasters) : `{"steps": 18, "identical": true}`. Sorties : `sortie.txt`, `sortie-rejeu.txt`.
- Après les correctifs de relecture (2026-11-05) : noyau `--features gpu` reconstruit et installé, extension recopiée ; sonde refaite (`sortie-b.txt`, `sortie-rejeu-b.txt`) : 18 lignes, deux rejeux de la même session puis un en session neuve, trois fois `{"steps": 18, "identical": true}`.
