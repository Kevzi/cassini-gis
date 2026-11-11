---
title: 'epic11_reel et style_reel portées à 40-50 phrases écrites à la main, refus compris, puis remesure du parent (livrable 3 de 5.53, lot 3)'
type: 'feature'
ticket: '53'
created: '2026-11-11'
status: 'built'
baseline_revision: '9ad0716d1a7d3297a345181b029addf47163bc4a'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-agent-et-mcp/plan-5-53-3-barres-elargies.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `epic11_reel` (28 plans + F03, plus 4 points hors barre) et `style_reel` (10) sont
trop courtes pour juger un oubli à une phrase près, et n'attendent aucun refus. Leurs totaux
changés rendront la barre de non-oubli `non comparable` tant que le parent installé n'est pas
remesuré.

**Approach:** porter chaque barre à 40-50 phrases écrites à la main, dont au moins 3 refus,
sources existantes intactes, protégées par `recouvrement` ; puis écrire la remesure du parent
installé, à jouer quand le GPU est libre.

Décisions du propriétaire (2026-11-11, lot 1), adaptées à ce lot :
- D1 : phrases neuves dans `ai/tests/epic11_reel/phrases.json` et `ai/tests/style_reel/phrases.json`,
  schéma de `par_commande/phrases.json` ; dans `epic11_reel`, une phrase qui attend un plan porte
  `command` (une des 14), une question ou un refus n'en porte pas.
- D2 : élargies en place : `SENTENCES`, `GRID_QUESTION`, `FOREIGN_POINTS` (`epic11_reel`) et
  `SENTENCES` (`style_reel`) restent à l'octet près, attendus compris ; un seul `SCORE` sur le
  total de la barre (F03 compris, `FOREIGN_POINTS` toujours hors barre).
- D3 : au moins 3 refus attendus par barre, chacun avec `missing` (ou `missing_params`).
- D4 : seuils proportionnels aux décisions passées, arrondis vers le bas :
  `style_reel` 9/10 du total ; `epic11_reel` 27/28 rapporté au nouveau total
  (`common::proportional(total, 27, 28)`), sans règle par commande, F03 restant une question
  attendue (décision du coordinateur, 2026-11-11).
- D5 : `ecart_reel` reste à 40.
- D6 : lot 3 de 3 ; après lui (lots 1 à 3 réunis), remesure du parent installé
  `ajuste-apres-5-49-b`, pour que la barre de non-oubli redevienne comparable.

## Boundaries & Constraints

**Always:** vocabulaire civil (§2.9) ; phrases neuves, jamais engendrées ni recopiées d'une autre
barre ; chaque attendu passe `common::invalid` (sur l'inventaire de la barre : augmenté pour
`epic11_reel`, celui de l'épreuve pour `style_reel`) ; aucune phrase ne nomme un fichier non
ouvert ; points des plans neufs dans le système de leur raster, ou sur une commande sans raster ;
le propriétaire relit les phrases de géométrie, de terrain et de style à la présentation, avant
la fusion (aucun doute de tournure ne lui est posé pendant la construction).

**Never:** modifier `tests/epreuve-modele-local/`, `training/src/`, `ai/tests/par_commande/`,
les barres des lots 1 et 2 ; envoyer une phrase de mesure au jeu d'entraînement ; charger un
modèle ou utiliser le GPU pendant la construction (la remesure est écrite, pas lancée) ;
toucher `~/cassini-worktrees/reentrainement-5-49` ; pousser.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Barre élargie | tests sans modèle | vert, total 40-50 chacune, ≥ 3 refus | — |
| Attendu invalide | plan neuf qui échoue un contrôle commun | rouge, nommant l'id | — |
| Refus devenu faux | mot de `missing` dans un id du registre | rouge, nommant l'id | — |
| Commande sans phrase neuve | une des 14 sans plan neuf dans le JSON | rouge, nommant la commande | — |
| Phrase engendrée ou recopiée | au jeu par défaut, ou dans deux barres | `recouvrement` rouge | identifiant seul |
| Sources gelées | constantes de phrases et `tests/epreuve-modele-local/` | inchangées | — |

</frozen-after-approval>

## Code Map

- `ai/tests/epic11_reel.rs` -- `SENTENCES`/`GRID_QUESTION`/`FOREIGN_POINTS` l. 44-183 et `expected`, `foreign_*` : intacts. Essai l. 398-526 : ajouter la boucle des neuves, scores par commande imprimés sans seuil, `SCORE` unique (première fraction = barre). Garder les lignes `COMMANDE …` : jamais `SCORE <commande>`, réservé à `par_commande_reel` par `non_oubli.py`. `inventory()` l. 353 : l'inventaire à passer à `common::invalid`.
- `ai/tests/style_reel.rs` -- `SENTENCES` l. 34-73 et `expected` : intacts. Seuil l. 164 (`right >= 9`) → `common::proportional(total, 9, 10)` ; `Want` sans refus aujourd'hui ; `every_graduated_field_is_a_numeric_field_of_its_layer` à étendre aux neuves.
- `ai/tests/plan_reel.rs:161-218,295-600`, `ai/tests/question_reel.rs:65-100` -- modèle à suivre : `all_phrases`, `answers`, seuils, tests sans modèle (total 40-50, ≥ 3 refus, `invalid`, `naming_unloaded_files`, notation sans modèle).
- `ai/tests/common/mod.rs:143-556` -- `read_phrases`, `invalid`, `same_calls`, `proportional`, `naming_unloaded_files` : réutiliser, ne rien changer sauf manque prouvé.
- `training/tests/recouvrement.rs:45-49,198-202,253-280` -- `HAND_WRITTEN` (+2 entrées, compte dans l'entrée), sommes de `the_real_trials_are_read_whole` ; `pairs(EPIC11_REEL|STYLE_REEL, …)` et leurs comptes 28/1/4/10 inchangés.
- `training/entrainement/non_oubli.py` -- ne pas modifier ; lit la première fraction de `SCORE` par section.
- `/mnt/c/Users/kevin/AppData/Local/Temp/essais-cuda.ps1` -- `-Model -Log -Tests`, `CARGO_TARGET_DIR=C:\ct-cuda`, se place dans `CASSINI-nuit` ; ne pas modifier. Modèle de lancement : `~/cassini-entrainement/mesures-5-56.sh`.
- Relevés actuels du parent : `~/cassini-entrainement/apres-5-49-mesures/essais-gpu-cuda-b.log`, `par-commande-b.log`, `recette-annecy-b.txt` ; `C:\Users\kevin\cassini-modeles\ajuste-apres-5-49-b\rapport.json`.

## Tasks & Acceptance

**Execution:**
- [x] `ai/tests/epic11_reel/phrases.json` -- 12 à 21 phrases neuves (total 41-50 avec les 29) : au moins un plan par commande (portant `command`), au moins 2 questions, au moins 3 refus -- D1, D3.
- [x] `ai/tests/epic11_reel.rs` -- lire les neuves ; comparaison `common::same_calls` pour elles, `comparable` exact pour les 28 ; seuils D4 ; en-tête ; tests sans modèle : `invalid`, total, refus, chaque plan neuf appelle sa `command`, chaque commande a un plan neuf, aucun fichier non ouvert, notation sans modèle, seuils en valeurs fixes.
- [x] `ai/tests/style_reel/phrases.json` -- 30 à 40 phrases neuves (total 40-50) : mises en forme topographiques, dégradés, questions, au moins 3 refus -- D1, D3.
- [x] `ai/tests/style_reel.rs` -- même schéma que `plan_reel` ; essai renommé (préfixe `style_reel` gardé) ; seuil `proportional(total, 9, 10)` ; tests sans modèle comme ci-dessus, champ numérique pour tout dégradé.
- [x] `training/tests/recouvrement.rs` -- deux entrées `HAND_WRITTEN`, en-tête -- garde contre le générateur et anti-recopie.
- [x] `docs/entrainement-du-modele.md` -- section du livrable 3 : totaux, compositions, seuils des deux barres, remesure.
- [x] `_bmad-output/initiative-cassini-mvp/deferred-work.md` -- entrée du lot 3 : résolution des barres ; la laisser ouverte jusqu'à la remesure jouée.
- [x] `~/cassini-entrainement/mesures-5-53-3.sh` -- écrire le script de remesure (Design Notes), hors dépôt, sans le lancer.

**Acceptance Criteria:**
- Given `cargo test -p cassini-ai --test epic11_reel --test style_reel` sans modèle, then vert.
- Given la recette locale complète, then verte ; `recouvrement` vert à la graine par défaut.
- Given le script de remesure, then `bash -n` passe et il s'arrête, sans rien changer, si le GPU est pris ou `CASSINI-nuit` n'est pas propre.

## Design Notes

**Décisions prises au plan** (doutes tranchés en lentilles party, sans le propriétaire) :
- Seuil d'`epic11_reel` (décision du coordinateur, 2026-11-11, dans D4) : 27/28 des plans
  rapporté au nouveau total, arrondi vers le bas, comme les autres barres ; plus de règle par
  commande (les lignes `COMMANDE …` restent imprimées, sans seuil) ; F03 reste une question
  attendue. Si la remesure du parent installé montre qu'il ne tient pas ce seuil, le seuil est
  revu sur les chiffres, par décision du propriétaire.
- Question rendue par le garde-fou des coordonnées : juste, comme dans `question_reel`.
- Évités, faute d'une seule réponse défendable : classes hors bornes (`class_count` > 12),
  dégradé hors table, dégradé sur un champ texte, relèvement inverse, visée « à la boussole »,
  gisement en Lambert-93 (décisions D2-D5 du livrable 4), point de passage d'un chemin, profil.
- Remesure dans `CASSINI-nuit` (l'arbre fixe d'`essais-cuda.ps1`, pratique de 5.56), et non dans
  un worktree neuf comme le disait le lot 1 : seulement s'il est propre et sans entraînement en
  cours, sa révision remise ensuite.
- Conflits attendus avec le lot 2 (`HAND_WRITTEN`, doc, `deferred-work.md`) : union des deux.

**Tournures civiles** (pistes, chaque phrase écrite à neuf) : géométrie — « référence de
carroyage MGRS », « reporte en Lambert-93 », « à quelle distance et dans quelle direction »,
« point rayonné à 800 m à l'azimut 210° », « intersection de deux visées » ; terrain — « exposition
des versants », « estompage du relief », « bassin visuel cumulé », « sommets remarquables »,
« classes de pente », « carte d'aptitude pondérée », « effort de déplacement cumulé », « tracé
le plus économique », « tirage A1 paysage » ; style — « charte graphique IGN », « symbolisation
conventionnelle », « carte choroplèthe », « seuils naturels », « effectifs égaux », « amplitudes
égales », « du jaune au rouge », « divergent rouge-bleu ». Refus — terrain : accumulation des
écoulements (`flow`, `hydro`), lissage du MNT (`smooth`, `focal`), rugosité (`roughness`),
volume de déblais (`volume`) ; style : catégories par valeur (`categor`), étiquettes (`label`),
transparence (`opacity`), symboles proportionnels (`proportional`), carte de chaleur (`heatmap`).
Questions : point sans système, destination sans azimut, chemin sans arrivée ; couche ou champ
absents.

**Remesure du parent** (lots 1 à 3 réunis sur une révision `REV`, GPU libre) :

```bash
# ~/cassini-entrainement/mesures-5-53-3.sh REV
set -eu; REV=$1; E=~/cassini-entrainement; MES=$E/5-53-3-mesures; mkdir -p $MES
NUIT="/mnt/c/Users/kevin/Downloads/Projet code/CASSINI-nuit"; TEMPW=/mnt/c/Users/kevin/AppData/Local/Temp
MOD='C:\Users\kevin\cassini-modeles\ajuste-apres-5-49-b\cassini-composition-4b.modele.json'
[ -z "$(nvidia-smi --query-compute-apps=pid --format=csv,noheader)" ] || { echo "GPU pris"; exit 1; }
[ -z "$(git -C "$NUIT" status --porcelain)" ] || { echo "CASSINI-nuit pas propre"; exit 1; }
PREV=$(git -C "$NUIT" rev-parse HEAD); trap 'git -C "$NUIT" checkout -q --detach "$PREV"' EXIT
git -C "$NUIT" checkout -q --detach "$REV"
L=essais-gpu-parent-5-53-3.log; rm -f $TEMPW/$L
powershell.exe -NoProfile -ExecutionPolicy Bypass -File 'C:\Users\kevin\AppData\Local\Temp\essais-cuda.ps1' \
  -Model "$MOD" -Log "C:\\Users\\kevin\\AppData\\Local\\Temp\\$L" \
  -Tests "plan_reel,question_reel,ecart_reel,echelle_utm_reel,terrain_reel,epic11_reel,style_reel,par_commande_reel" || true
cp $TEMPW/$L $MES/   # une barre ratée n'arrête pas le relevé ; le trap remet CASSINI-nuit
```

Contrôle, depuis le dépôt : `python3 -I training/entrainement/non_oubli.py --parent
$E/apres-5-49-mesures/essais-gpu-cuda-b.log $E/apres-5-49-mesures/par-commande-b.log --candidat
$MES/essais-gpu-parent-5-53-3.log --json $MES/non-oubli-controle.json` : attendu code 3 (même
modèle des deux côtés), `ecart_reel` et `par_commande:*` `tenue`, les six barres élargies
`non comparable`. Ensuite, le parent d'un candidat est `$MES/essais-gpu-parent-5-53-3.log`,
`recette-annecy-b.txt` et `rapport.json` d'`ajuste-apres-5-49-b`, et tout candidat se mesure à `REV`.

## Verification

Environnement : `LIBCLANG_PATH` vers un `libclang.so`, `BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`.

**Commands:**
- `cargo fmt --all --check && cargo clippy -p cassini-core -p cassini-ai -p cassini -p cassini-training --all-targets -- -D warnings` -- sans avertissement
- `cargo test -p cassini-core -p cassini-ai -p cassini -p cassini-training --locked` -- vert
- `python3 -I -m unittest discover -s training/entrainement/tests` -- vert
- `git diff 9ad0716 -- tests/epreuve-modele-local/ training/src/ ai/tests/par_commande/ ai/tests/plan_reel/ ai/tests/question_reel/` -- vide
- `for f in epic11_reel style_reel; do diff <(git show 9ad0716:ai/tests/$f.rs | sed -n '/^const \(SENTENCES\|GRID_QUESTION\|FOREIGN_POINTS\)/,/^\]\?)\?\];/p') <(sed -n '/^const \(SENTENCES\|GRID_QUESTION\|FOREIGN_POINTS\)/,/^\]\?)\?\];/p' ai/tests/$f.rs); done` -- vide
- `bash -n ~/cassini-entrainement/mesures-5-53-3.sh` -- passe

**Manual checks:**
- Un attendu faussé, un refus dont la commande existe, une commande privée de plan neuf : le test nomme la phrase ou la commande.
- `git diff 9ad0716 -- ai/tests/epic11_reel.rs ai/tests/style_reel.rs` : aucune ligne retirée dans `expected`, `foreign_as_written`, `foreign_expected`.

## Implementation Notes

- `epic11_reel` : 28 + F03 + 20 = 49 (EP01-EP14 un plan par commande, `command` porté ; EP15-EP17
  questions : point sans système, destination sans azimut, chemin sans arrivée ; EP18-EP20
  refus : `flow`/`hydro`/`accumul`, `smooth`/`focal`/`filter`, `rough`/`rugos`). Seuil 47/49.
  Essai renommé `epic11_reel_at_least_27_of_28_answers_match_on_the_whole_bar` ; F03 compte dans
  le total ; règle par commande retirée, lignes `COMMANDE <c> <j>/<n>` gardées (n = 2 + plans
  neufs). `bar()` met toute la barre (SENTENCES, F03, neuves) dans la forme `phrases.json`
  pour `common::invalid` sur l'inventaire augmenté. Points des plans neufs : Lambert-93 sur les
  rasters, WGS84/UTM 31N sur les commandes sans raster ; « cap » évité (ambigu au générateur).
- `style_reel` : 10 + 32 = 42 (SR01-SR16 dégradés, SR17-SR25 topographiques, SR26-SR28
  questions, SR29-SR32 refus `categor`/`unique_value`, `label`, `opacity`/`transparen`,
  `proportional`/`symbol_size`). Seuil 37/42. Essai renommé
  `style_reel_at_least_9_of_10_answers_match_on_the_whole_bar`. `Want` et `expected` intacts ;
  `all_phrases` les convertit (`exact`).
- `recouvrement.rs` : entrées `epic11_reel:phrases` (20) et `style_reel:phrases` (32), nommées à
  part des sources `epic11_reel`/`style_reel` pour que l'anti-recopie les compare aussi aux
  constantes de leur barre. Vert à la graine par défaut du premier coup (aucune phrase
  réécrite).
- `common/mod.rs` inchangé (aucun manque).
- Remesure : `~/cassini-entrainement/mesures-5-53-3.sh` écrit, non joué ; `bash -n` passe ;
  arrêt vérifié sans rien changer sur révision inconnue, GPU pris (nvidia-smi simulé) et
  nvidia-smi illisible ; le cas « CASSINI-nuit pas propre » n'est pas simulé (on ne salit pas
  l'arbre), même forme de garde. Durci par rapport aux Design Notes : `nvidia-smi` ou `git status`
  en échec arrête aussi (un échec dans `$(…)` sous `[ -z … ]` passait pour « libre »), et `REV`
  est vérifiée avant tout checkout.
- Retours du coordinateur (2026-11-11), corrigés : l'essai boucle sur `bar()` (total et plans par
  commande tirés de la barre, `exact` pour `SENTENCES`), F03 garde son exigence dure en plus du
  total, garde-fou compté sur toute la barre, avertissement non bloquant pour une commande à 0,
  essai renommé `epic11_reel_at_least_27_of_28_rounded_down_and_the_grid_question_asked` ; EP15
  réécrite (coordonnées sans forme de système, système dit inconnu), `filter` → `mean_filter`
  (EP19) ; SR25 réécrite, mots `labels`/`label_placement` (SR30) et `proportional_symbol` (SR32),
  SR09, SR12, SR16 remplacées (bâtiments, parcelles, routes) ; tests : ST06 en flottant refusé,
  aucune phrase neuve de style ne porte `command` ni `exact` ; doc : tous les mots `missing` ;
  piste « au cap 210° » retirée ; script de remesure durci.
- À faire par le propriétaire avant la fusion : relire les phrases de géométrie, de terrain et
  de style (EP01-EP20, SR01-SR32).

## Plan Change Log

## Review Triage Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment).
Verdicts : high 0, medium 7, low 10, false 0, maybe-false 0. Corrigés 14 (patch, même implémenteur), rejetés 3.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| F03 ne fait plus échouer l'essai à lui seul | medium | patch | décision du coordinateur : exigence dure de F03 gardée en plus du total |
| Total et score de l'essai calculés à part de `bar()` | medium | patch | une boucle sur `bar()`, `total = bar().len()` |
| EP15 : coordonnées de forme Lambert-93, plan défendable | medium | patch | réécrite, système explicitement inconnu |
| `filter` (EP19), `label`, `proportional` trop généraux | medium | patch | `mean_filter`, `labels`/`label_placement`, `proportional_symbol` |
| Comparaison exacte des 10 phrases de `style_reel` non prouvée | medium | patch | ST06 `6.0` refusé ; aucune neuve ne porte `command`/`exact` |
| Script de remesure : garde GPU aveugle côté Windows, relevé ancien ou sans SCORE accepté, trap sous cargo vivant, course | medium | patch | gardes WSL + Windows rejouées avant chaque lancement, relevé vérifié, trap qui attend |
| Dégradés peu variés (doublon SR01/SR16) | medium | patch | SR09, SR12, SR16 remplacés ; gares et PLU sans champ numérique |
| Dénominateur par commande en dur ; `guarded` partiel ; aucune alerte commande à 0 | low | patch | comptes tirés de la barre, garde-fou sur toute la barre, avertissement non bloquant |
| Nom de l'essai trompeur | low | patch | `…_27_of_28_rounded_down_and_the_grid_question_asked` |
| SR25 ambiguë | low | patch | réécrite |
| Tableau des mots `missing` incomplet ; piste « au cap » | low | patch | doc complète ; « à l'azimut » |
| Vérification automatique des barres gelées incomplète | low | reject | `recouvrement` et les tests de chaque barre gardent leur contenu ; les diffs de constantes sont vérifiés à chaque lot |
| Script de remesure hors dépôt | low | reject | scripts de mesure hors dépôt par choix du projet (chemins du poste) |
| Seuils non mesurés sur le modèle | low | reject | remesure du parent juste après la fusion des lots (D6) |
