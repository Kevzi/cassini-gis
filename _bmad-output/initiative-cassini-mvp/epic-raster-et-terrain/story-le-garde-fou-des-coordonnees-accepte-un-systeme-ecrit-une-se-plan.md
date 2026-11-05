---
title: 'Le garde-fou des coordonnées accepte un système écrit une seule fois pour plusieurs points'
type: 'feature'
ticket: '10'
created: '2026-11-05'
status: 'done'
baseline_revision: '8d5443e3dca550591116faa900493b6ebca5f9e9'
route: 'oneshot'
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

**Problem:** `guard_coordinates` (`core/src/point.rs`) fait retirer à chaque `Point` son x, son y et son epsg du multiensemble des nombres de la demande ; une demande qui écrit le système une seule fois pour deux points (« voit-on 657 300 / 6 848 400 depuis 656 500 / 6 847 800, en Lambert-93 ? ») transforme donc le plan juste d'une ligne de vue en question du garde-fou (S21 de l'essai réel).

**Approach:** Un système écrit une fois dans la demande vaut pour tous les points du plan : l'epsg d'un point est accepté s'il retire une occurrence de la demande, ou s'il est égal à un epsg qu'un point précédent du même plan a déjà retiré de la demande. Le x et le y de chaque point restent retirés chacun de la demande ; le système n'est toujours jamais pris au projet. Décision du propriétaire, 2026-11-05.

**Décision (propriétaire, 2026-11-05, relecture) :** l'emprunt n'est permis que si chaque système écrit dans la demande a servi à au moins un point ; un système écrit resté sans point (deux systèmes écrits, le modèle met le premier aux deux points) rend la question, comme avant.

</frozen-after-approval>

## Implementation Notes

Voie courte : un changement local à `invents_a_point` (une liste des epsg déjà retirés de la demande par les points du plan), ses tests dans `core/src/point.rs`, la doc de la règle dans l'en-tête et le commentaire de `guard_coordinates`, puis l'attendu de S21 dans `ai/tests/plan_reel.rs` (`expected("S21")` devient le plan de ligne de vue, `the_right_line_of_sight_for_s21_is_guarded` attend le plan inchangé, `the_terrain_sentences_expect_plans_…` et l'en-tête du fichier suivent). `guard_rejected` partage la même fonction et suit sans changement. Le générateur (`training/src/families.rs`) n'est pas touché : il écrit le système à chaque point, ce qui reste juste ; l'apprentissage du système écrit une fois est dans `deferred-work.md` pour le prochain ré-entraînement groupé.

Après relecture : `request_systems` (codes nommés, zones UTM, nombres écrits après « EPSG ») et `named_codes`, extrait de `request_numbers` sans changement de comportement ; l'emprunt est refusé si un système écrit n'a servi à aucun point, quel que soit l'ordre des points. Tests ajoutés : deux systèmes écrits et un seul porté (dans les deux ordres, en nom et en code), `guard_rejected` à `None` sur le plan juste. Vérifié : `cassini-core` 400 tests, `cassini-training` 68, `plan_reel` et `terrain_reel` sous Windows, `fmt` et `clippy` verts.

## Review Triage Log

Relecture thorough (blind-hunter, edge-case-hunter, verification-gap, intent-alignment), passe 1.

- medium — deux systèmes écrits, le modèle met le premier aux deux points : l'emprunt laissait passer un point au mauvais système (edge 1, blind 1 en partie). HALT, décision du propriétaire : emprunt restreint ; corrigé et testé.
- low — dépendance à l'ordre des points (edge 2) : même cause, couverte par la même règle et le test dans les deux ordres.
- low, rejeté — un nombre nu égal à un code (« 2154 » comptés) pris pour système (edge 3) : comportement d'avant 4.10, cas rare, correctif plus lourd qu'une correction directe.
- false — plausibilité de x/y pour le système emprunté (blind 1) : la décision du propriétaire fait valoir le système écrit pour tous les points ; un point hors MNT est refusé par le noyau (`terrain.outside_dem`).
- false — emprunt entre étapes différentes (blind 2) : l'intention dit « du même plan ».
- low — doc qui surestimait la règle (blind 3) : corrigée (« tout point du plan qui porte ce même epsg »).
- defer — `deferred-work.md` : l'apprentissage du système écrit une fois par le générateur n'y était pas (blind 4, verification-gap) : entrée ajoutée ; les entrées anciennes ne sont pas réécrites.
- low — `guard_rejected` non testé dans le cas positif (blind 5) : test ajouté.
- low, rejeté — UTM écrit une fois, WGS84 pour deux points (blind 6) : même chemin de code que Lambert-93, déjà couvert.
- false — chiffres de S21 différents de l'exemple du plan (blind 7) : l'exemple vient du ticket, S21 est testée avec la vraie phrase ; aucune contradiction (intent-alignment).
- low, rejeté — test S21 redondant (blind 8) : c'est le test que nomme le `verify` du ticket.
- low, rejeté — `Want::GuardQuestion` retiré (blind 9) : plus aucune phrase ne l'attend ; à rajouter le jour où une phrase l'attend.
- false — pas d'essai sur le vrai modèle (blind 10, intent-alignment D) : le ticket le place en contrôle hors ticket, proposé au propriétaire.

Tests à couvrir (`verify` du ticket) : plan juste d'une ligne de vue au système écrit une fois → inchangé ; même demande avec un x ou un y absent → question ; demande sans système → question ; deux points écrits dans deux systèmes différents (2154 et 4326) → inchangé comme avant ; et un point dont l'epsg diffère du seul système écrit → question.

## Verification

**Commands:**
- `cargo test -p cassini-core point` puis `cargo test -p cassini-ai --test plan_reel` -- expected: tout vert, l'essai réel ignoré sans modèle
- `cargo fmt --all --check && cargo clippy -p cassini-core -p cassini-ai --all-targets -- -D warnings` -- expected: aucun avertissement (sous Windows si `cassini-ai` ne compile pas sous WSL)

**Manual checks (if no CLI):**
- Contrôle hors ticket (risque high), à proposer au propriétaire : `plan_reel` et `terrain_reel` repassés sur le modèle installé (`--features cuda`), sans nouvelle coordonnée arrêtée à tort ni coordonnée inventée laissée passer.

## Contrôle sur le vrai modèle (2026-11-05)

`cassini-composition-4b` installé, `--features cuda` : `plan_reel` 23/25, 0 fausse question, 0 du garde-fou ; S21 donne désormais le plan attendu. S24 et S25 échouent comme au 5.41 (renfort reporté, décision du propriétaire). `terrain_reel` 8/8. Aucune coordonnée arrêtée à tort, aucune coordonnée inventée laissée passer.
