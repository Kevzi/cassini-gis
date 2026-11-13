---
title: 'Retirer une couche du projet (layer.remove)'
type: 'feature'
ticket: '27'
created: '2026-11-10'
status: 'built'
baseline_revision: 'e668ea6b067065f2efdf7d9a6318fbd248c48675'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-commande-layer-remove/change-commande-layer-remove.md'
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** aucune commande ne retire une couche du projet ; le générateur apprend au modèle à refuser (« Aucune commande du catalogue ne retire de couche du projet »), alors que le propriétaire veut ranger un projet après une analyse (changement de cap `change-commande-layer-remove`, approuvé le 2026-11-10).

**Approach:** ajouter `layer.remove`, déléguée à QGIS, exactement comme la fiche du changement de cap : paramètre `layer_id` seul, résultat `layer_id`, `runs_in: qgis`, non dictée, genre de sortie `none` (amendement du 2026-11-08 d'AD-24). Le geste retire la couche par `QgsProject.removeMapLayer`, jamais le fichier ; une couche absente rend le même résultat sans erreur. Le générateur apprend la commande, H35 et S04 attendent un plan.

## Boundaries & Constraints

**Always:** fiche, ligne du §4 et statut `faite` dans le même commit ; rejeu par le chemin générique (AD-11), jamais sauté ; tests de l'extension par fond doublure ; aucune phrase du jeu tenu à l'écart ni des essais réels recopiée dans le générateur ; `training/tests/recouvrement.rs` vert sans changer la graine.

**Never:** effacer, déplacer ou modifier un fichier ; nouvelle dépendance ; nouveau code d'erreur ; toucher au modèle installé ou lancer un entraînement ; plus de paramètres que la fiche.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Couche présente | `routes_1` au projet | quitte le projet ; fichier (octets, date) inchangé ; `{"layer_id": "routes_1"}` | — |
| Couche absente | id inconnu du projet | même résultat | aucune erreur |
| Rejeu même session | journal `[layer.remove routes_1]`, couche déjà retirée | rejeu identique | — |
| Rejeu session neuve | projet rouvert avec `routes_1` | couche retirée, rejeu identique | — |
| Retrait d'un résultat | `analysis.buffer` puis `layer.remove` de sa couche | couche du tampon retirée ; au rejeu l'id neuf est lié, identique | — |
| Étape suivante qui cite la couche retirée | retrait puis tampon de `routes_1` | l'étape échoue en la nommant, par le code de son geste (`layer.not_found` pour une analyse, comme décidé le 2026-11-09 ; `qgis.layer_missing` pour un geste de style) ; même erreur au rejeu, identique | erreur du geste |
| Paramètres invalides | `{}`, champ inconnu | refus avant QGIS | `registry.invalid_params` |
| Plan | « retire la couche « Routes » du projet » | plan valide d'une étape `layer.remove` | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/layer_arrange.rs`, `data_reproject.rs` -- modèle d'une commande déléguée (`impl Delegated`, `.runs_in(RunsIn::Qgis)`, `.output_none()`, exemples, `.tests`, `inventory::submit!`) ; nouveau `layer_remove.rs` (modules déclarés par `build.rs`). Non dictée : pas de `.dictated_by_core()`. Non annulable.
- `core/src/journal/mod.rs` l. ~541-548, ~1366-1404 -- `output_none` : le `layer_id` du résultat n'est ni journalisé comme couche posée ni lié ; rien à changer, à prouver par tests.
- `core/src/journal/tests.rs` l. ~3458 (`ConvertQgis`) -- forme d'un QGIS de test ; en faire un avec état (`RefCell<HashSet>` des couches du projet) qui sait `analysis.buffer` (id neuf à chaque appel) et `layer.remove` idempotent, `layer.not_found` sur une couche absente pour le tampon. Tests de la matrice : même session, session neuve (nouveau routeur, couche présente), retrait d'un résultat, étape suivante.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md` -- §4 après `data.convert` (l. ~372), fiche §7 après celle de `data.convert` (l. ~1605-1645, même forme, note d'implémentation) ; `core/tests/catalogue.rs` les confronte au registre.
- `qgis-plugin/cassini_qgis/open_output.py` (`OpenOutputGesture`), `style_gestures.py` (`_layer_or_fail`, `QgisBackend`) -- socle et forme d'un geste à fond injectable ; nouveau `layer_remove.py` : fond `QgisBackend` (`mapLayer`, `name()`, `removeMapLayer`), seul à importer `qgis.core`. Garde le nom de la couche retirée (affichage seul) pour `panel._layer_name` (l. ~149), qui sinon montrerait l'id.
- `qgis-plugin/cassini_qgis/delegated.py` -- ligne `layer.remove` de la table fermée ; `tests/test_delegated.py` la confronte au catalogue.
- `qgis-plugin/cassini_qgis/summary.py` (`_SENTENCES` l. ~806, modèle `_data_convert` avec `names.ahead`), `texts.py` FR/EN, `tests/test_summary.py`.
- `training/src/families.rs` -- table `FAMILIES` (l. ~186, ~270) : famille `retrait_couche` ; `refus` l. ~3529-3530 : les deux branches deviennent un refus d'effacer le fichier du disque (principe 6), le nombre de branches ne change pas ; garde-fou `what_is_refused_out_of_registry_is_still_missing_from_the_registry` (l. ~7915) : prédicat `removes_no_layer` sur le modèle d'`exports_no_layer` (l. ~6830), aucune famille ne le rend ; test de la famille sur le modèle de `a_requested_conversion_is_one_step_to_the_written_format`. Attention : S04 de `plan_reel` dit « Retire la couche des routes du projet. » — ne jamais produire « la couche des <pluriel> du projet » ; ne pas employer « ne sert plus ».
- `ai/tests/tenu_a_l_ecart/sentences.json` (H35) -- attendu `calls` : `layer.remove` sur `routes_bdt_25af7c60_9b14_4d83_a2e7_6c0d58f1b935` ; `ai/tests/ecart_reel.rs` l. ~345 : comptes (40, 28, 8, 4), commentaire daté.
- `ai/tests/plan_reel.rs` -- S04 attend `layer.remove` sur `ROUTES` ; doc du fichier (l. ~8) et commentaire des refus (l. ~138) à jour.
- `docs/entrainement-du-modele.md` -- ligne de famille, ligne `refus` (l. ~106), paragraphe des garde-fous (l. ~197-203). `README.md` : liste des commandes s'il y en a une.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/commands/layer_remove.rs` + catalogue §4 et §7 -- commande, fiche `faite`, tests du module (déléguée, non dictée, `output_none`, paramètres).
- [x] `core/src/journal/tests.rs` -- matrice : rejeu même session et session neuve, retrait d'un résultat, étape suivante.
- [x] `qgis-plugin/cassini_qgis/layer_remove.py`, `delegated.py`, `summary.py`, `texts.py`, `panel.py` + `tests/test_layer_remove.py`, `tests/test_summary.py` -- geste, résumé FR/EN, nom gardé.
- [x] `training/src/families.rs`, `docs/entrainement-du-modele.md` -- famille, refus réécrit, garde-fou, test.
- [x] `ai/tests/tenu_a_l_ecart/sentences.json`, `ai/tests/ecart_reel.rs`, `ai/tests/plan_reel.rs` -- H35 et S04.

**Acceptance Criteria:**
- Given le dépôt, when `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings` et les tests de l'extension, then tout passe, `recouvrement` compris.
- Given QGIS 3.44 et une copie d'un projet de test, when on retire une couche par une sonde directe, then elle quitte le projet, son fichier est intact, un second retrait ne fait pas d'erreur, et le rejeu est identique en même session et en session neuve.

## Implementation Notes

- Garde d'un ordre sans `layer_id` dans le geste : code existant `qgis.gesture_failed` (aucun code neuf) ; le noyau refuse ce cas avant QGIS (`registry.invalid_params`).
- Refus réécrit : `NO_FILE_DELETION` (« Aucune commande du catalogue n'efface de fichier du disque : les fichiers de l'utilisateur ne sont jamais modifiés. »), branches 2 et 3 de `refus`, 17 branches comme avant.
- Famille `retrait_couche`, poids 3, couches vecteur seulement ; `recouvrement` vert à la graine par défaut.
- Test de plus : la limite connue (rejeu même session d'une analyse puis retrait de sa couche) diverge à l'étape 1, `replay.diverged`, et rejoue à l'identique en session neuve.
- README : ligne `layer.remove` ajoutée au tableau des commandes exécutées par QGIS (ce tableau n'avait déjà ni `data.reproject` ni `data.convert`, laissés tels quels).

## Plan Change Log

## Review Triage Log

Boucle 0, relecture thorough (blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Verdicts : high 0, medium 4, low 11, false 4, maybe-false 0. Corrigés 6, reportés 4 (dont 2 décisions du propriétaire), rejetés 8.

| Constat | Verdict | Route | Preuve / action |
|---|---|---|---|
| `layer_id` vide admis par le noyau, refusé par le geste (`qgis.gesture_failed`) | low | patch | une chaîne vide est une couche absente : même résultat, sans erreur |
| « son fichier intact » faux pour une couche sans fichier (mémoire, base, service web) | low | patch | phrase reformulée : « aucun fichier effacé » |
| Repli du nom retiré du panneau jamais exécuté par un test | medium | patch | aide pure `display_name` dans `layer_remove.py`, testée avec `summary.sentence` |
| Limite du rejeu en même session absente de la fiche | low | patch | phrase ajoutée à la fiche du catalogue |
| Paragraphe 10.27 inséré au milieu de celui de 10.9 (`docs/entrainement-du-modele.md`) | low | patch | déplacé en paragraphe propre |
| Mise en page PEP 8 (lignes vides, import long) | low | patch | corrigée |
| Retrait d'une couche en mémoire : son contenu est perdu sans recours | medium | defer (décision du propriétaire) | `removeMapLayer` détruit une couche mémoire ; la refuser demande un code d'erreur que la fiche exclut |
| Retrait d'une couche en cours d'édition : modifications non enregistrées perdues | medium | defer (décision du propriétaire) | même raison ; Cassini n'ouvre ses couches qu'en lecture seule, le cas vient d'une édition à la main |
| `step:k.layer_id` d'une étape `layer.remove` admis par `check_plan`, échec à l'exécution seulement | low | defer | règle de validation des sorties `none`, qui touche aussi `map.graduated_style` : décision d'architecture, pas de ce ticket |
| Famille `retrait_couche` : ni raster, ni retrait de plusieurs couches, ni verbe nu (« supprime les gares ») | medium | defer | choix du générateur à revoir avec le propriétaire avant le prochain entraînement groupé |
| Codes `qgis.gesture_failed` non listés dans la fiche | false | rejet | codes génériques de tout geste, aucune fiche ne les liste |
| « Supprime les <pluriel> de la carte » contre le refus de suppression d'entités | low | rejet | les refus d'entités disent « de la couche » et portent un sous-ensemble ; le recouvrement des sorties est contrôlé par les tests de famille |
| `removes_no_layer` trop large ou trop étroit | false | rejet | aucune raison de refus existante ne le déclenche (tests sur 2000 tirages) ; garde-fou de non-retour, pas un classifieur |
| `_REMOVED_NAMES` jamais vidé, nom gardé même si le retrait échoue | low | rejet | ids QGIS uniques par couche ; une couche présente prend toujours son nom vivant ; affichage seul |
| Nom périmé après changement de projet | false | rejet | même id = même couche (id à UUID) ; le nom vivant passe avant le nom gardé |
| Aucun contrôle sous le vrai QGIS | false | rejet | sonde réelle faite après la construction : 22/22 contrôles (couche retirée, fichiers intacts, second retrait sans erreur, rejeu identique en même session et en session neuve) |
| H35 et S04 en échec avec le modèle installé jusqu'au ré-entraînement | low | rejet | attendu, écrit dans la doc d'entraînement ; même cas que S09 en 10.9 |
| Branches du refus devenues un refus neuf (effacer un fichier) au lieu de plans ; `qgis.layer_missing` du changement de cap ; « version de chaîne 1 » | low | rejet (signalé au propriétaire) | plans dans la famille `retrait_couche` ; code d'analyse `layer.not_found` décidé le 2026-11-09 ; la version de chaîne n'existe pas encore (entrée 10.3, qui migre cette ligne) |

## Design Notes

Le résultat ne porte que `layer_id` (fiche) : aucun `name` n'y est ajouté. Le nom affiché après le retrait vient d'une mémoire d'affichage de l'extension, remplie par le geste avant `removeMapLayer`.

Limite connue, à dire au propriétaire sans la corriger ici : un rejeu dans la même session d'un plan qui analyse une couche puis la retire diverge à l'analyse (la couche n'est plus au projet), en la nommant ; c'est le comportement honnête d'AD-11.

## Verification

**Commands:**
- `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` (de même `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`) -- expected: vert. Rust sous Windows seulement.
- `python3 -I -m unittest discover -s qgis-plugin/tests` et `python3 -I qgis-plugin/tests/check_extension.py` -- expected: vert.

**Manual checks (if no CLI):**
- Sonde QGIS réelle, faite par l'orchestrateur après la construction (pas par l'implémenteur).

## Recette QGIS réelle

2026-11-10, QGIS 3.44.15, noyau `--features gpu` installé (ancien gardé sous `cassini.exe.avant-10-27`), extension copiée dans le profil. Sonde directe, sans modèle, sur `annecy/projet-recette-10-27.qgz` (copie du projet d'Annecy), couche `plu_zones` (Shapefile) : retrait, second retrait sans erreur, tampon de la couche retirée (`layer.not_found`), rejeu en même session puis projet relu et session neuve : 22/22 contrôles, rejeu `identical: true` deux fois, fichiers du Shapefile inchangés (empreintes et date). Rejouée après les corrections de relecture : 22/22. Archivée dans `donnees-test-cassini/_archives/sondes/recette-10-27-layer-remove/`.

## Décisions du propriétaire (2026-11-10)

Le propriétaire a tranché le 2026-11-10 (« on suit toutes les recommandations ») les points
signalés par la relecture :

- **Couche en mémoire ou en édition modifiée : refusée.** Changement de cap
  `change-refus-retrait-couche-non-enregistree` (approuvé) : le geste refuse ces deux cas par
  le nouveau code `layer.unsaved`, la couche reste au projet, intacte. Cette décision lève, pour
  ce seul code, la règle « Never : nouveau code d'erreur » du plan ci-dessus. Plan :
  `plan-10-27-refus-retrait-couche-non-enregistree.md`.
- **Acceptés tels quels :**
  - la limite du rejeu en même session (analyse d'une couche puis son retrait : le rejeu dans
    la même session diverge à l'analyse, en la nommant ; identique en session neuve) ;
  - le code `layer.not_found` d'une analyse qui cite une couche retirée, au lieu du
    `qgis.layer_missing` écrit dans le changement de cap (code d'analyse décidé le 2026-11-09 ;
    `qgis.layer_missing` reste celui des gestes de style) ;
  - le refus neuf « effacer le fichier d'une couche » (`NO_FILE_DELETION`) qui remplace, dans
    la famille `refus` du générateur, les deux branches devenues des plans ;
  - la « version de chaîne 1 » du changement de cap, reportée à l'entrée 10.3, qui crée les
    versions de chaîne et migrera cette ligne de la table fermée.
