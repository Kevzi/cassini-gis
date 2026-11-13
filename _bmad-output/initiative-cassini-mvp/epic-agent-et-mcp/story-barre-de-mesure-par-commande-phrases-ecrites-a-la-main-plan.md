---
title: 'Barre de mesure par commande, phrases écrites à la main (livrable 1 de 5.53)'
type: 'feature'
ticket: '53'
created: '2026-11-11'
status: 'built'
baseline_revision: 'f574fff4ce59c2589b52531b73cb013283bcd195'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** aucune barre ne mesure chaque commande offerte au modèle local : 10.28 à 10.31 n'en
avaient aucune, et `data.convert_raster`, `model.list`, `system.about` n'ont aujourd'hui aucune
phrase de mesure. Un modèle peut oublier une commande sans qu'aucune barre ne le voie.

**Approach:** un jeu de phrases de mesure écrites à la main, au moins trois par commande offerte,
chacune avec son plan, sa question ou son refus attendu ; un test sans modèle qui échoue dès
qu'une commande offerte n'a pas ses phrases ou qu'un attendu est invalide ; un essai réel (ignoré
sans modèle) qui rend un score par commande ; le jeu protégé par `recouvrement`.

Découpage décidé avec le propriétaire le 2026-11-11 (« 1 puis 2 ») : ce plan est le livrable 1.
La barre de non-oubli (livrable 2), les barres portées à 40-50 phrases (3) et les formulations de
terrain du propriétaire (4) sont dans `deferred-work.md`.

## Boundaries & Constraints

**Always:** « offerte » = `Registry::standard().specs()` sans `dictated_by_core` (33 commandes à
f574fff), lu par le test, jamais une liste recopiée ; vocabulaire civil (dépôt public, §2.9) ;
phrases neuves, jamais recopiées d'une barre existante ni du générateur ; chaque attendu « plan »
passe `check_plan` ; le propriétaire relit les phrases de terrain et de géométrie avant le commit.

**Never:** utiliser le GPU ou charger un modèle avant que `nuit-apres-5-49.etat` dise « TOUT FINI » ;
toucher `~/cassini-worktrees/reentrainement-5-49` ou `CASSINI-nuit` ; modifier une barre existante
ou le générateur ; fixer un seuil de réussite (c'est le livrable 2) ; pousser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Couverture complète | chaque commande offerte a ≥ 3 phrases, dont ≥ 2 attendent un plan qui l'appelle | test vert | — |
| Commande sans phrases | on retire les phrases de `terrain.peaks` | test rouge, nommant `terrain.peaks` | message listant les commandes manquantes |
| Nouvelle commande au registre | commande offerte ajoutée sans phrases | test rouge | même message |
| Attendu invalide | plan attendu qui échoue `check_plan` | test rouge, nommant l'id de la phrase | — |
| Phrase recopiée au jeu | une phrase du jeu engendrée par le générateur | `recouvrement` rouge | identifiant seul imprimé |

</frozen-after-approval>

## Code Map

- `core/src/registry.rs:548` -- `Registry::standard()`, `specs()`, champ `dictated_by_core` : source de « offerte » (même filtre que `offered`, `ai/src/plan.rs:110`, privé).
- `core/src/plan.rs:233` -- `Registry::check_plan` : valide un attendu.
- `ai/tests/epic11_reel.rs` -- modèle à imiter : phrases + `expected(id)`, `comparable` (l. 383, titre du PDF libre), boucle de l'essai réel (l. 398 : `common::view`, `restored`, `completed`), score par commande.
- `ai/tests/common/mod.rs` -- `test_model`, `shown_inventory`, `view`, `restored`, `completed` : à réutiliser tels quels.
- `tests/epreuve-modele-local/inventory.json` -- parcelles, PLU, gares, routes, bâtiments, communes, MNT 5 m ; base de l'inventaire du jeu (copié, l'original reste gelé), complété des rasters dont ont besoin `raster.*` et `terrain.cost_distance` (comme `epic11_reel`, l. 9-16).
- `training/tests/recouvrement.rs:23-33,131` -- sources protégées et `real_trials` : y brancher le nouveau JSON.
- Commandes offertes sans phrase aujourd'hui : `data.convert_raster`, `model.list`, `system.about` ; à une seule : `layer.add`, `analysis.zonal_statistics`.

## Tasks & Acceptance

**Execution:**
- [x] `ai/tests/par_commande/inventory.json` -- inventaire du jeu (épreuve + rasters de coût et de notes) -- un seul inventaire, connu, pour tous les attendus.
- [x] `ai/tests/par_commande/phrases.json` -- ≥ 3 phrases par commande offerte (≈ 100) : `{id, command, phrase, expected: {kind: calls|question|refusal, calls?}}` ; ≥ 2 sur 3 attendent un plan appelant `command`, la troisième peut être une question ou un refus légitime -- couvrir plan direct, tournure courte, enchaînement.
- [x] `ai/tests/par_commande_reel.rs` -- test sans modèle : couverture, `check_plan` de chaque attendu, `command` présent dans le plan attendu, ids uniques ; essai `#[ignore]` sous `CASSINI_TEST_MODEL` : score par commande et total, ligne `SCORE` par commande, sans seuil -- c'est ce que lira le livrable 2.
- [x] `training/tests/recouvrement.rs` -- source `PAR_COMMANDE` ajoutée à `real_trials` et à `the_real_trials_are_read_whole`.
- [x] `docs/entrainement-du-modele.md` -- section « Barre par commande » : où, comment la lancer, règle « une commande offerte, trois phrases ».

**Acceptance Criteria:**
- Given le registre à f574fff, when `cargo test -p cassini-ai --test par_commande_reel` tourne sans modèle, then il est vert et compte 33 commandes couvertes.
- Given les phrases d'une commande retirées, when le test tourne, then il échoue en nommant la commande.
- Given le jeu par défaut engendré, when `recouvrement` tourne, then aucune phrase du jeu par commande n'y figure, à l'identique ni à un nombre ou une commune près.
- Given la recette locale complète, then verte.

## Implementation Notes

- Mis en œuvre directement (sous-agent refusé : l'extension context-mode réécrit l'appel, le mode
  auto ne peut pas le juger).
- 99 phrases, 33 commandes ; deux attendent une question (ISO3 : durée absente, RP3 : système
  cible absent), toutes les autres un plan.
- Vérifications de chaque attendu au-delà du plan : garde-fou des coordonnées, complétion sans
  effet, couches et rasters présents ; points du terrain écrits en Lambert-93, le système du MNT.
- Règle d'écriture : vers UTM ou MGRS, `coord.convert` ; vers Lambert-93 ou WGS84,
  `data.transform_point`.
- `recouvrement` a trouvé LR3, GS1, HS1 déjà engendrées : réécrites, générateur intact.
- Les phrases sont écrites par un script de travail hors dépôt ; seul le JSON est versionné.

## Design Notes

JSON plutôt que Rust : le propriétaire relit les phrases sans lire de code, et `recouvrement` lit
déjà des JSON (`from_json`). L'essai réel rend un score par commande, pas un verdict : sans
parent à comparer, un seuil serait arbitraire ; le livrable 2 compare deux de ces relevés.

## Verification

Environnement (WSL) : travailler dans `/home/kevin/cassini-worktrees/5-53-barre-par-commande` ; exporter `LIBCLANG_PATH` vers un `libclang.so` (paquet pip `libclang`) et `BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include` avant cargo.

**Commands:**
- `cargo fmt --all --check && cargo clippy -p cassini-core -p cassini-ai -p cassini -p cassini-training --all-targets -- -D warnings` -- sans avertissement
- `cargo test -p cassini-core -p cassini-ai -p cassini -p cassini-training --locked` -- vert, `par_commande_reel` et `recouvrement` compris
- `python3 -I -m unittest discover -s training/entrainement/tests` -- vert

**Manual checks:**
- Retirer temporairement les phrases d'une commande : le test nomme la commande.

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 0, medium 5, low 15, false 2, maybe-false 0. Corrigés 12 (patch), reportés 4, rejetés 7.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| `layer.add` : `path` jamais comparé à `unloaded_files` (3 lentilles) | medium | patch | `Known.unloaded`, erreur « fichier absent », mutation LA1 |
| `field` / `label_field` non comparés aux champs de la couche | medium | patch | erreur « champ absent », mutation SEL1 |
| `assert_eq!(check_plan)` dans la boucle de l'essai réel : panique sans lignes `SCORE` | medium | patch | plan invalide compté en échec, l'essai continue |
| Égalité JSON : `2.0` ≠ `2` | medium | patch | `normalize_numbers` des deux côtés (comme `ecart_reel`) |
| Question du garde-fou comptée juste pour ISO3/RP3 | medium | patch | seule `QuestionSource::Model` compte |
| Règle « jamais recopiée d'une barre existante » non vérifiée | low | patch | test `no_sentence_of_the_per_command_bar_copies_another_real_trial` ; il a trouvé LD1, réécrite |
| Compte de `the_real_trials_are_read_whole` calculé par l'analyseur contrôlé | low | patch | 99 en dur, `assert_eq!` dans `real_trials` |
| Test des attendus invalides : 2 branches, dépendant de l'ordre | low | patch | 6 branches, comparaison par appartenance |
| Pas de plancher sur le nombre de commandes offertes | low | patch | `offered.len() >= 33` |
| DR3 : pente sur un relevé sans système | low | patch | phrase « en Lambert-93 », `assign_epsg: 2154` attendu |
| Chemin temporaire de session dans le plan versionné | low | patch | consigne générique |
| Commande de lancement sans `--test` | low | patch | doc et en-tête du test |
| Aucune phrase n'attend un refus | low | defer | `deferred-work.md`, livrable 3 |
| Défauts et ordre des listes comparés strictement | low | defer | même rigueur que les autres barres ; livrable 2 |
| Filtre « offerte » recopié de `offered`, privé | low | defer | rendre public = surface d'API |
| Relevé non lisible par machine ; pas raté d'un enchaînement compté contre l'étiquette | low | defer | à fixer par le livrable 2 |
| `shown_view` et `shown_inventory` pourraient diverger au-dessus du seuil du tri | false | reject | 12 couches, sous le seuil : la vue est l'inventaire court, même table (comme `epic11_reel`) |
| Référence `step:` pendante non vérifiée | false | reject | `check_plan` contrôle les références d'étape (en-tête de `core/src/plan.rs`) |
| Identifiants communs entre sources | low | reject | vérifié par verification-gap : aucun doublon |
| `source_plan: none` dans `deferred-work.md` | low | reject | format prescrit pour un découpage d'intention (step-01) |
| Chiffres datés dans la doc | low | reject | datés explicitement « au 2026-11-11 » |
| Clés de référence fixes, pas tirées des schémas | low | reject | aucune commande actuelle hors liste ; complexité sans cas montré |
| Attendus question/refus non validés | low | reject | aucun n'a de champ parasite ; garde sans cas montré |
| SL3 « au-dessus de 20 % », DP2 « cap » | — | propriétaire | relu le 2026-11-11 : phrases de terrain approuvées, SL3 gardée ; DP2 confié à Claude : « cap » ambigu (souvent magnétique) remplacé par « azimut 75° » |
