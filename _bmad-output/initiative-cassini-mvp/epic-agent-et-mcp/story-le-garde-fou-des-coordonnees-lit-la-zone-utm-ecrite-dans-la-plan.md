---
title: 'Le garde-fou des coordonnées lit la zone UTM écrite dans la demande'
type: 'feature'
ticket: '37'
created: '2026-11-03'
status: 'done'
baseline_revision: '1da5efe52dc823b49e4d0e4c1ba722ad0efabcab'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le garde-fou de 5.16 ne relie à un code EPSG que Lambert-93 (2154) et WGS84 (4326). Une demande claire comme « … X 412300 Y 1652800 en UTM 31N » reçoit une question au lieu de son plan : le 32631 du plan n'est pas lu dans la demande et passe pour inventé.

**Approach:** Étendre la lecture des systèmes nommés de `request_numbers` : une zone UTM du WGS84 écrite avec son hémisphère (« UTM 31N », « UTM 31 N », « UTM zone 31 Nord », « UTM31N ») ajoute 32600 + zone au multiensemble, l'hémisphère sud (« S », « Sud ») 32700 + zone, zones 1 à 60 seulement. Sans hémisphère, aucun code. Une famille du générateur civil apprend le point en UTM (31N ou 32N), sans ré-entraîner ici.

## Boundaries & Constraints

**Always:** Lecture pure dans `core/src/point.rs`, même sens d'erreur sûr : en cas de doute, aucun code, donc la question. Le nombre de la zone (31) reste lu comme nombre ordinaire, comme le 93 de « Lambert-93 ». Les essais existants du garde-fou et du générateur restent verts. Décision (propriétaire, 2026-11-03) : seul le WGS84 UTM (326xx, 327xx) est visé ; « S » vaut sud, pas la bande de latitude S du MGRS. Décision (propriétaire, 2026-11-03) : un autre datum écrit juste avant « UTM » (« ED50 », « ETRS89 », « NTF », avec ou sans séparateur, ex. « ED50 UTM 31N », « ETRS89/UTM 31N ») bloque la lecture : aucun code, donc la question. Décision (propriétaire, 2026-11-03, après relecture) : un de ces datums écrit juste après la zone (« UTM 31N (ED50) », « UTM 31N ED50 ») bloque aussi.

**Never:** Toucher `guard_coordinates`, `invents_a_point`, `judge`, le prompt, la grammaire ou le jeu tenu à l'écart. Reconnaître un autre système nommé (Lambert II, ED50 UTM, ETRS89-LCC…). Ré-entraîner le modèle (passe groupée après 37 et 38). Écrire une famille d'hémisphère sud au générateur.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Zone nord | « … X 412300 Y 1652800 en UTM 31N », plan à `epsg: 32631` | plan inchangé | — |
| Formes admises | « UTM 31 N », « UTM zone 31 Nord », « UTM31N », « utm 31n » | 32631 lu | — |
| Zone sud | « UTM 31S », « UTM 31 Sud » | 32731 lu | — |
| Hors bornes | « UTM 61N », « UTM 0N » | aucun code 326xx/327xx | plan à 32661 → question `guard` |
| Sans hémisphère | « UTM 31 », « UTM zone 31 » | aucun code | plan à 32631 → question `guard` |
| Lettre collée à un mot | « UTM 31 Nantes », « UTM 31 Sudouest » | aucun code (l'hémisphère doit finir le mot) | question `guard` |
| Mauvaise zone | demande « UTM 31N », plan à 32632 | question `guard` | — |
| Autre datum | « ED50 UTM 31N », « ETRS89 / UTM 31N », « NTF UTM 31N » | aucun code | plan à 32631 → question `guard` |
| Datum après la zone | « UTM 31N (ED50) », « UTM 31N ED50 » | aucun code | plan à 32631 → question `guard` |

</frozen-after-approval>

## Code Map

- `core/src/point.rs` l.186-263 -- `NAMED_SYSTEMS` (liste fermée nom + numéro → code fixe) et sa boucle de lecture dans `request_numbers` l.239-261. L'UTM a un numéro variable : ajouter une lecture dédiée (fonction `utm_codes(&[char]) -> Vec<f64>`) appelée après la boucle, sans changer `NAMED_SYSTEMS`. Réutiliser `is_name_separator` l.164 pour l'espace ou le tiret entre les morceaux. Mettre à jour la doc de `request_numbers` l.190-194 et l'en-tête du module.
- `core/src/point.rs` tests l.735-760 (`request_numbers` …) et l.630-675 (`a_point_given_in_the_request_leaves_the_plan_unchanged`) -- modèles des nouveaux tests ; helper `isochrone(origin, duration)` et `is_guard` déjà là.
- `training/src/families.rs` l.43-185 (`FAMILIES`), l.1602-1655 (`Origin`, `origin_point`), l.1699-1735 (`isochrone`) -- ajouter `utm_origin(rng)` et une famille `isochrone_utm` qui reprend le corps de `isochrone` avec ce point (extraire le corps en `isochrone_from(rng, name, origin)` pour ne pas le dupliquer). Le test l.2900-2925 montre comment passer les exemples au garde-fou.
- `training/src/lib.rs` l.486 -- `a_travel_without_coordinates_never_becomes_a_plan` reconnaît une demande à coordonnées par `lambert`, `epsg`, `wgs` : ajouter `utm`, sinon la nouvelle famille y échoue.
- `docs/entrainement-du-modele.md` l.~92 -- tableau des familles : une ligne `isochrone_utm`.
- `docs/canal-local.md` l.210 -- vérifier si les systèmes nommés y sont listés ; sinon rien.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/point.rs` -- lecture des zones UTM avec hémisphère dans `request_numbers`, docs ; tests de la matrice (lecture seule et garde-fou sur un plan isochrone) -- le cœur de la règle
- [x] `training/src/families.rs` -- `utm_origin` (zone 31N ou 32N, coordonnées plausibles en France, formes « UTM 31N », « UTM 31 N », « UTM zone 31 Nord », « UTM31N »), famille `isochrone_utm` (poids 3), et un test : chaque exemple est une isochrone à 32631 ou 32632 que `guard_coordinates` laisse inchangée -- le modèle apprendra la forme au prochain ré-entraînement
- [x] `training/src/lib.rs` -- `utm` reconnu comme coordonnées dans le test l.486 -- la règle « trajet sans coordonnées → question » reste juste
- [x] `docs/entrainement-du-modele.md` -- ligne de la famille -- la doc du générateur suit

**Acceptance Criteria:**
- Given le noyau sans QGIS, when tournent les tests de `cassini-core`, then « … X 412300 Y 1652800 en UTM 31N » et un plan à `epsg` 32631 passent le garde-fou, « UTM 31S » donne 32731, « UTM 61N » et « UTM 31 » ne donnent aucun code, et tous les essais existants du garde-fou restent verts.
- Given le générateur civil (graine et taille par défaut), when tournent les tests de `cassini-training`, then la famille `isochrone_utm` a des exemples gardés, tous passent `check_plan` et le garde-fou inchangés.

## Implementation Notes

- `core/src/point.rs` : `utm_codes` (appelée à la fin de `request_numbers`) et `follows_other_datum` ; le numéro de zone reste un nombre ordinaire. `training/src/families.rs` : `utm_origin`, `isochrone_from` (l'ancienne `isochrone` garde l'ordre de ses tirages), famille `isochrone_utm` (poids 3) ; l'ajout déplace le tirage du jeu par défaut. `docs/canal-local.md` cite aussi l'UTM.
- Tests : `cassini-core` 283, `cassini-training` 45 (WSL) ; `cargo test --workspace` vert sous PowerShell (276 tests du noyau sous Windows, contre 283 sous WSL : écart non examiné).
- Correctifs de relecture : `word_before` / `word_after` sautent tout caractère non alphanumérique ; `follows_other_datum` et `precedes_other_datum` lisent « ed50 » comme « ED 50 » (de même ETRS89, NTF), avant « UTM » ou après l'hémisphère ; une apostrophe après la lettre n'est pas un hémisphère. Effet voulu : un datum dans la phrase voisine (« … ED50. UTM 31N ») bloque aussi, sens sûr.
- Démo réelle non jouée : le modèle actuel n'a pas appris l'UTM ; à faire après le ré-entraînement groupé (37 + 38).

## Plan Change Log

## Review Triage Log

Passe 1 (2026-11-03, lentille quick) : high 0, medium 3, low 1, false 1.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | Datum avant « UTM » non vu derrière un tiret demi-cadratin, une parenthèse, une virgule, deux-points, une espace fine, ni écrit « ED 50 » / « ETRS 89 » | medium | patch | `follows_other_datum` ne saute que `is_name_separator` et `/` ; la décision dit « avec ou sans séparateur » : sauter tout caractère non alphanumérique et lire « ed 50 », « etrs 89 » |
| 2 | Hémisphère lu dans un mot élidé (« UTM 31 n'importe », « UTM 31 s'il ») | medium | patch | `ends_word` accepte l'apostrophe ; sens dangereux (code deviné) ; exclure `'` et `’` |
| 3 | Datum écrit après la zone (« UTM 31N (ED50) ») lu comme WGS84 | medium | patch | question posée au propriétaire, décision écrite dans l'intention : bloquer aussi |
| 4 | `docs/canal-local.md` : une ligne trop longue après le repli | low | patch | correction directe ; paragraphe replié à 95 caractères |

Correctifs 1 à 4 appliqués ; tests de lecture et du garde-fou ajoutés ; `cargo fmt --check`, `clippy -D warnings`, tests `cassini-core` et `cassini-training` verts.
| 5 | `utm_origin.coords` toujours entre parenthèses, au contraire d'`origin_point` | false | reject | `coords` n'est pas lu par `isochrone_from` ; aucun comportement ne change |

## Design Notes

Lecture (sur la demande en minuscules) : `utm` en début de mot ; séparateur facultatif ; `zone` + séparateur facultatifs ; 1 ou 2 chiffres (pas un troisième) ; séparateur facultatif ; `n`, `nord`, `s` ou `sud`, suivi d'une fin de texte ou d'un caractère non alphanumérique. Zone 1..=60 → `32600 + z` (nord) ou `32700 + z` (sud). Avant `utm`, en sautant les espaces, tirets et `/` : si le mot précédent est `ed50`, `etrs89` ou `ntf`, aucun code (« ed50utm » est déjà exclu : `utm` n'y commence pas un mot).

Coordonnées du générateur : 31N, E 400 000–700 000, N 5 300 000–5 500 000 ; 32N, E 280 000–450 000, mêmes N. Moitié avec milliers séparés (`thousands`), comme `origin_point`.

## Verification

Environnement : `core` et `training` sous WSL ; `ai` et `cli` non touchés.

**Commands:**
- `cargo test -p cassini-core`, `cargo test -p cassini-training` -- expected: succès
- `cargo fmt --check`, `cargo clippy -p cassini-core -p cassini-training --all-targets -- -D warnings` -- expected: aucun avertissement
- PowerShell : `cargo test --workspace` -- expected: succès (les essais du garde-fou dans `ai` restent verts)

**Manual checks (if no CLI):**
- Démo réelle (mémoire « démo de bout en bout par Claude ») : `cassini plan` sur la phrase UTM avec le modèle installé. Le modèle actuel n'a pas appris l'UTM : s'il rend un plan à 32631, il passe le garde-fou ; s'il rend autre chose, c'est attendu jusqu'au ré-entraînement groupé — à consigner, pas à corriger ici.
