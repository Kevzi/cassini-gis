---
title: 'Second essai de 11.23 : corrections du générateur'
type: 'feature'
ticket: ''
created: '2026-11-09'
status: 'built'
baseline_revision: '6d1e620'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-geometrie-et-terrain-etendu/story-re-entrainement-groupe-points-tels-qu-ecrits-et-commandes-de-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le premier entraînement de 11.23 (validation 297/300) manque deux barres : `plan_reel` 22/25 et F03 d'`epic11_reel` ; la lecture des échecs (Implementation Notes du plan de 11.23) montre un attendu périmé (S05) et quatre défauts d'apprentissage : S07 reprojette une couche déjà en mètres, F03 invente un fuseau UTM quand l'inventaire n'en a pas, S21 perd la hauteur de la cible dans « 2 m de hauteur de chaque côté », ST08 perd la méthode « à intervalles égaux ».

**Approach:** corriger l'attendu de S05 et le générateur sur ces quatre points, puis relancer une seconde et dernière fois l'entraînement (règle de 11.16), avec les mêmes essais et barres. Décision du propriétaire du 2026-11-09 (« 1 »).

## Boundaries & Constraints

**Always:** S05 « Mets les bâtiments en Lambert-93 » attend `data.reproject` de la couche des bâtiments (en 4326 dans l'inventaire de l'épreuve) vers 2154, dans `ai/tests/plan_reel.rs` seulement ; `tests/epreuve-modele-local/` lu tel quel ; la famille `reprojection_avant_analyse` (et toute famille d'analyse métrique) n'ajoute jamais `data.reproject` à une couche déjà dans un système en mètres, et le jeu contient des exemples explicites d'analyses métriques sur des couches en mètres dans un inventaire qui a aussi des couches en degrés ; le carroyage UTM sans fuseau écrit pose la question quand l'inventaire n'a pas `utm_epsg`, avec une part de questions au moins égale à la moitié des recopies ; « {h} de hauteur de chaque côté » (et tournures voisines) donne `observer_height` et `target_height` ; « à intervalles égaux », « par quantiles », « par seuils naturels » (et voisines) donnent la `method` de `map.graduated_style` ; chaque exemple passe `check_plan` et `guard_coordinates` ; `training/tests/recouvrement.rs` reste vert sans changer la graine ; mêmes base, méthode, `DEFAULT_COUNT`, `DEFAULT_SEED`.

**Never:** un troisième entraînement ; installer le modèle ; pousser ; toucher au noyau, au prompt ou à l'extension.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Tampon sur couche métrique | gares en 2154, bâtiments en 4326 dans l'inventaire ; « tampon de 300 m autour des gares » | `analysis.buffer` sur les gares, sans `data.reproject` | — |
| Tampon sur couche en degrés | gares en 4326 | `data.reproject` puis le tampon (inchangé) | — |
| Carroyage UTM, inventaire sans `utm_epsg` | « carroyage UTM au kilomètre » | question sur le système | — |
| Hauteurs | « 2 m de hauteur de chaque côté » | `observer_height` et `target_height` à 2 m | — |
| Méthode du dégradé | « à intervalles égaux » | `method: equal_interval` | — |

</frozen-after-approval>

## Code Map

- `ai/tests/plan_reel.rs` -- attendu de S05.
- `training/src/families.rs` -- `reprojection_avant_analyse` et familles d'analyse métrique (tampon, recherche à distance) ; `carroyage_utm_projet` (part des questions) ; `ligne_de_vue` (hauteurs « de chaque côté ») ; `style_degrade` (tournures des méthodes) ; tests de famille.
- `docs/entrainement-du-modele.md` -- section de 11.23.
- Journal des essais du premier entraînement : `~/cassini-entrainement/11-23-mesures/essais-gpu.log`.

## Tasks & Acceptance

**Execution:**
- [x] `ai/tests/plan_reel.rs` -- S05.
- [x] `training/src/families.rs` + tests -- les quatre corrections de la matrice.
- [x] docs ; `cargo test -p cassini-training --release`, `cargo test -p cassini-ai`, fmt, clippy (Windows).

**Acceptance Criteria:**
- Given le jeu par défaut (graine 1, 8 000), when on le compte, then aucun `data.reproject` ne vise une couche en mètres, et les questions du carroyage UTM sans `utm_epsg` font au moins la moitié des recopies.

## Implementation Notes

- 2026-11-09 (sans entraînement ni commit) :
  - S05 : attendu `data.reproject` des bâtiments vers 2154 dans `ai/tests/plan_reel.rs` ; en-tête mis à jour.
  - S07 : nouvelle famille `analyse_couche_metrique` (poids 4, 123 exemples à la graine 1), code commun `metric_analysis` avec `reprojection_avant_analyse` (inchangée) ; couche en degrés posée par `Builder::distract_in_degrees` (`inventories.rs`). Test du jeu par défaut : un `data.reproject` sur une couche hors 4326 n'existe que si la phrase écrit le système d'arrivée (seul `reprojection_couche`, demande explicite, en produit).
  - F03 : `carroyage_utm_projet` poids 3 → 4, recopie 3/8 et question sans `utm_epsg` 2/8 ; au jeu par défaut 57 recopies, 32 questions, aucun fuseau deviné (test).
  - S21 : `both_heights_clause` (6 tournures dont « de hauteur de chaque côté ») une fois sur quatre dans `ligne_de_vue`.
  - ST08 : `GRADUATED_METHODS` 6 → 14 tournures (formes entre virgules comprises), méthode écrite une fois sur deux ; test : chaque tournure tirée donne sa `method`.
  - Vérifié (Windows) : `cargo test -p cassini-training --release` (103 + 3 + 1 + 5 verts, recouvrement compris, graine inchangée), `cargo test -p cassini-ai` vert, `cargo fmt --check` et `cargo clippy -p cassini-ai -p cassini-training --all-targets -- -D warnings` propres.
  - Reste : relecture, commit, second entraînement (non lancés).

- Second essai lancé le 2026-11-09 à 10 h 22 (chaîne accélérée, Unsloth, commit `7b382e3` de `story/acceleration-entrainement`), fini à 13 h 28 (185 min ; la carte de 8 Go s'est remplie au pas ~180 et la vitesse est retombée de 10,5 à 25 s/pas). Sortie `~/cassini-entrainement/regroupe-11-23c/`, modèle copié dans `C:\Users\kevin\cassini-modeles\ajuste-11-23c\`, non installé.
- Validation (llama-server, jeu tiré du générateur corrigé) : 288/300.
- Essais GPU (`~/cassini-entrainement/11-23-mesures/essais-gpu-11-23c.log`) : `plan_reel` 23/25 (S15, S18), `question_reel` 12/12, `ecart_reel` 32/40 (plans 21/26, questions 8/8, refus 3/6), `echelle_utm_reel` 8/8, `terrain_reel` 8/8, `epic11_reel` 28/28 et F03 en question, points d'un autre système 3/4 hors barre (Y04), `style_reel` 9/10 (ST07). **Toutes les barres sont tenues** ; installation sur décision du propriétaire.

## Plan Change Log

## Review Triage Log

Boucle 0 (relecture quick) — low 4 ; corrigé 1, rejetés 3 (tests et chiffres, sans effet sur le jeu d'entraînement).

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Ligne `style_degrade` de la doc qui attribue mal « une fois sur deux » | low | patch | ligne corrigée |
| Test du jeu par défaut plus lâche que le critère (forme écrite n'importe où dans la phrase) | low | rejeté | exception nécessaire à `reprojection_couche`, notée ; le jeu n'en dépend pas |
| Tournure « observateur et cible à {n} m » non vérifiée pour l'égalité des hauteurs | low | rejeté | test seulement ; le générateur écrit les deux hauteurs |
| Chiffres non revérifiés par le relecteur | low | rejeté | relecture en lecture seule ; chiffres issus de l'exécution des tests |

## Verification

**Commands:** Rust sous Windows seulement : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test -p cassini-training --release'`.
- `cargo test -p cassini-training --release` et `cargo test -p cassini-ai` -- expected: verts
- `cargo fmt --check`, `cargo clippy -p cassini-ai -p cassini-training --all-targets -- -D warnings` -- expected: propres
