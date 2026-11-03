---
title: 'Garde-fou : une coordonnée du plan vient de la demande, jamais du modèle'
type: 'feature'
ticket: '16'
created: '2026-10-23'
status: 'built'
baseline_revision: '921a7c772a5fbf9157cb1319d07f712fc75d0a5f'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: ['blind-hunter', 'edge-case-hunter', 'verification-gap', 'intent-alignment']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le modèle de 5.15 rend pour la phrase E une isochrone dont l'origine est inventée (EPSG:4326, 2,458 / 48,658). Le plan est accepté et montré comme juste, alors que le résultat est faux sans aucun signal.

**Approach:** Appliquer AD-23 étendu aux coordonnées (texte du contrat d'architecture, 2026-10-23). On crée un type `Point` partagé et marqué au schéma, dont le contrôle de contrat impose l'usage. Une fonction pure du noyau, appliquée à la fin de `compose_plan_with`, transforme en question à provenance `guard` tout plan dont un `Point` ne vient ni de la demande, ni d'une étape. Quand le garde-fou parle, le panneau remet la phrase d'origine dans la saisie.

## Boundaries & Constraints

**Always:** Fonction pure dans `core`, sans IA ni QGIS. Appliquée une seule fois, après `judge` et la redemande, sur tout plan `calls`. Tous les chemins y passent (canal, `cassini plan`, essais réels). Le système absent de la demande n'est jamais pris au projet. La provenance reste hors de la grammaire. Le sens d'erreur sûr : en cas de doute, une question.

**Décision (propriétaire, 2026-10-23) — nombres de la demande :** `2.458` et `2,458` (virgule suivie d'un chiffre = décimale) ; groupes de trois chiffres séparés par une espace, une espace insécable ou une espace fine (`652 000`, `6 850 000`) lus comme un seul nombre ; signe moins accepté. Toute autre forme (`652.000`, `652'000`, degrés-minutes) n'est pas reconnue et mène à la question. Plan gardé entier malgré sa taille (un seul livrable). Systèmes nommés (décision du propriétaire après implémentation, 2026-10-23) : liste fermée, « Lambert-93 » / « Lambert 93 » valent 2154, « WGS84 » / « WGS 84 » valent 4326 ; tout autre nom ne compte pas. Recette du propriétaire (2026-10-23) : dans QGIS, la phrase E a rendu `agent.plan_invalid` (origine inventée sans `epsg`, rejetée deux fois avant le garde-fou) ; décision : le garde-fou s'applique aussi à une sortie rejetée, et rend la question sans redemande quand elle porte un point qui ne vient pas de la demande.

**Never:** Toucher le prompt, `SHORT_SYSTEM`, `judge`, `retry_turns`, la grammaire, `training/` ou le jeu tenu à l'écart. Écrire la provenance au journal (AD-11). Appliquer le garde-fou au serveur MCP. Faire un correctif par phrase. Changer le comportement d'`analysis.isochrone` envers QGIS.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Phrase E | aucune coordonnée dans la demande, origine rendue par le modèle | question `guard` qui demande x, y et système | — |
| Point donné | « … depuis x 652 000, y 6 850 000 en EPSG:2154 » | plan inchangé | — |
| Système manquant | x et y dans la demande, pas d'EPSG | question `guard` | — |
| Nombre partagé | « 10 min depuis 10, 500 (EPSG:2154) », plan avec durée 10, x 10, y 10 | question : y n'a pas de nombre libre | — |
| Point d'une étape | `origin: "step:k.f"`, avec `f` déclaré `Point` à la sortie de k | plan inchangé | — |
| Référence non `Point` | `origin: "step:1.layer_id"` | question `guard` | — |
| Question ou refus du modèle | `Plan::Question` / `Refusal` | inchangé, provenance `model` | — |
| Contrat | champ numérique `x` ou `y` hors `Point` | `check_contract` le refuse | — |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_isochrone.rs` l.54-63 -- `Origin {x, y, epsg}`, seule coordonnée du registre : la remplacer par le `Point` partagé (même désérialisation, `deny_unknown_fields`).
- `core/src/lib.rs` -- déclarer le nouveau module `point` (type, marque, garde-fou, lecture des nombres).
- `core/src/plan.rs` -- `Plan::Question { question }` l.31 : ajouter `source` (`model` par défaut, `skip_deserializing`, omis à la sérialisation quand `model`, pour que les complétions d'entraînement restent identiques à l'octet). `parse_reference` l.55 est réutilisable (rendre `pub(crate)`). Les références n'existent qu'au premier niveau.
- `core/src/contract.rs` -- `check_spec` l.86 ; reprendre la marche de `is_geometry` l.256 (`$ref`, `anyOf`, `items`, `properties`) pour le parcours `Point` et pour la règle `contract.bare_coordinate`.
- `ai/src/plan.rs` -- `compose_plan_with` l.424 : appliquer le garde-fou à `Ok(plan)` (l.457) avec `request`. `judge` l.346 et `retry_turns` l.322 restent en lecture seule.
- `core/src/serve/plans.rs` l.160 -- rendre `{"question", "source"}`, toujours explicite.
- `cli/src/main.rs` -- `print_composed` l.561 sérialise le `Plan` (la provenance n'est montrée que pour `guard`). Adapter les motifs `Plan::Question { question }` (`..`) ici, dans `ai/tests/*_reel.rs` et dans `training/src/lib.rs` l.62.
- `ai/tests/{plan_reel,question_reel,ecart_reel}.rs` -- ligne `SCORE` : ajouter « dont N du garde-fou ».
- `qgis-plugin/cassini_qgis/client.py` l.421 -- passer `source` dans `PLAN_QUESTION`. `agentview.py` l.75 : `question(text, source)` qui retient s'il faut rendre la phrase. `panel.py` l.514 : remettre `_last_request` comme le fait `PLAN_FAILED` l.521.
- `docs/canal-local.md` l.161 -- documenter `source`.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/point.rs`, `core/src/lib.rs` -- `Point` (x, y, epsg) marqué au schéma par une extension `x-cassini`, la lecture des nombres de la demande, et `guard_coordinates(registry, plan, request) -> Plan`, avec les tests de la matrice -- le cœur de la règle
- [x] `core/src/commands/analysis_isochrone.rs` -- `origin: Point`, exemples et tests inchangés -- un seul type
- [x] `core/src/contract.rs` -- violation `contract.bare_coordinate`, plus un test de refus et le registre standard sans violation -- la règle tient pour les commandes futures
- [x] `core/src/plan.rs` -- `source` sur `Question`, avec un test d'aller-retour JSON où le modèle ne peut pas écrire `guard` -- la provenance
- [x] `ai/src/plan.rs` -- appel du garde-fou en fin de composition, et test sans modèle : coordonnées absentes, la question `guard` ; coordonnées présentes, le plan inchangé -- tous les chemins
- [x] `core/src/serve/plans.rs`, `cli/src/main.rs`, `training/src/lib.rs`, `ai/tests/*_reel.rs` -- provenance transmise et comptée -- les mesures distinguent
- [x] `qgis-plugin/cassini_qgis/{client,agentview,panel}.py`, `qgis-plugin/tests/test_plan_client.py`, `test_agent_view.py` -- phrase rendue à la saisie sur `guard` seulement
- [x] `docs/canal-local.md` -- champ `source`

**Acceptance Criteria:**
- Given l'inventaire réel de Villeneuve-le-Roi et le modèle de 5.15, when on envoie la phrase E par `cassini plan`, then la sortie est une question `guard` et aucun plan.
- Given le modèle de 5.15 sur GPU, when tournent `plan_reel`, `question_reel` et `ecart_reel`, then aucun score ne baisse par rapport à 5.15.
- Given QGIS et le panneau, when on envoie la phrase de démo exacte puis la phrase E, then la première s'exécute comme avant, et la seconde affiche la question avec la phrase remise dans la saisie.
- Given la recette complète, when elle tourne, then elle est verte.

## Implementation Notes

- `core/src/point.rs` : `Point` (marque `x-cassini: point`), `request_numbers`, `guard_coordinates`, `GUARD_QUESTION`. Le système est lu comme nombre (`EPSG:2154`) **ou par son nom dans une liste fermée** — Lambert-93 / Lambert 93 → 2154, WGS84 / WGS 84 → 4326 — comme le recommandait la revue adverse (C4). Sans cette liste, S16 (« (Lambert-93) ») et deux phrases du jeu tenu à l'écart devenaient des questions, et les scores baissaient. À confirmer par le propriétaire.
- Le signe moins n'est pris que devant un nombre détaché d'un mot (« Lambert-93 » n'est pas -93). Un objet portant `x` ou `y` hors schéma est lu comme un point (sens sûr) ; une commande inconnue rend la question.
- Patrons `Plan::Question { question, .. }` : `judge` (ai/src/plan.rs) et `training/src/lib.rs` adaptés pour compiler, sans changement de comportement.
- Le long prompt (GGUF sans déclaration) voit le catalogue engendré : `$defs/Origin` y devient `$defs/Point` avec `x-cassini`. Le prompt court du modèle ajusté ne change pas.
- Vérification hors dépôt : les 204 exemples du générateur (graine 1, 8 000) qui portent une origine, S16 et les 2 phrases à point du jeu tenu à l'écart (ids seuls, texte non imprimé) passent le garde-fou inchangés.
- GPU, déclaration de 5.15 : `plan_reel` 25/25, 0 question ; `question_reel` 12/12 ; `ecart_reel` 32/40 (plans 19/26, questions 8/8, refus 5/6), 0 question du garde-fou — identiques à 5.15. `cassini plan` (cuda, inventaire de Villeneuve) : phrase E → question `guard` ; phrase de démo → plan inchangé ; phrase avec `x 652 000, y 6 850 000 en EPSG:2154` → isochrone.
- Reste à faire : démo dans QGIS et recette du propriétaire (critère 3), non jouées ici.

Démo de bout en bout (2026-10-23, Claude) : noyau CPU installé (`cassini.exe.avant-5-16` gardé), extension copiée, lien du modèle du dossier des modèles basculé sur `ajuste-5-15`. Sonde sous le Python de QGIS 3.44.15, projet réel de Villeneuve-le-Roi, noyau GPU et modèle de 5.15 :
- phrase de démo exacte : plan de 5 étapes exécuté, PDF lisible (composition 21 s) ;
- phrase E : question `guard`, aucun plan ; la phrase revient à la saisie (`request_to_restore`) ;
- « … depuis x 655 700, y 6 849 023 en Lambert-93 ? » : isochrone depuis ce point, exécutée. Un point hors réseau (652 000 / 6 850 000) passe le garde-fou puis est refusé par `analysis.origin_off_network`, comme attendu.
Reste : recette du propriétaire dans QGIS.

Correctif de recette : `guard_rejected` (core/src/point.rs) appelé dans `compose_plan_with` sur une sortie rejetée lisible ; une commande inconnue ou un point donné laissent la redemande. Tests : cas de la recette dans `point::` et dans `ai` (un seul appel au modèle).

## Plan Change Log

## Review Triage Log

Passe 1 (2026-10-23, lentilles blind-hunter, edge-case-hunter, verification-gap, intent-alignment) : high 0, medium 2, low 6, false 3, rejetés 8.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | « Lambert 93 652 000 » fusionné en 93652000 (blind, edge) | medium | patch | `request_numbers` prend « 93 » comme tête de groupe ; ne plus grouper un nombre collé à un nom de système, test ajouté |
| 2 | Long prompt : la marque `x-cassini` entre au catalogue (blind) | medium | patch | Never « toucher le prompt » ; ôter la marque dans `catalog_entry`, seul reste le renommage `$defs/Origin` → `Point`, inhérent au type unique |
| 3 | Espace fine U+2009 non lue (edge) | low | patch | décision du propriétaire « espace fine » ; ajouter U+2009 |
| 4 | Objet à `x` hors `Point` lu comme point même quand son schéma est connu (blind) | low | patch | `walk` : ne traiter `x`/`y` comme point que sans schéma connu |
| 5 | Référence vers un `Option<Point>` refusée (blind, edge) | low | patch | `reference_is_a_point` : `admits_point` au lieu de `is_point` |
| 6 | `GUARD_QUESTION` finit par « . », pas « ? » (verification-gap) | low | patch | `training/src/lib.rs` l.55 exige « ? » ; 5.17 apprendra cette question |
| 7 | Remise de la phrase dans `panel.py` non testée (verification-gap, blind) | low | patch | décision déplacée dans `agentview`, testée sans Qt ; QGIS confirmera |
| 8 | Ordre de la recette de l'intention : phrase E réelle et QGIS en prose (intent) | low | reject | les mesures réelles sont manuelles par construction (GPU) ; la démo QGIS suit à l'étape 5 |
| 9 | x/y inversés acceptés (blind) | false | reject | la règle porte sur la provenance (AD-23) ; placer lon/lat est le travail du modèle |
| 10 | Noms `lon`/`lat`, `easting` hors règle (blind, edge) | false | reject | AD-23 nomme `x` et `y` ; aucune commande n'en déclare d'autres |
| 11 | Marque `x-cassini` non vérifiée sur la forme du schéma (blind) | low | reject | développeur seul, peu probable, ajout de garde |
| 12 | `question_reel` compte une question `guard` comme réussie (blind) | low | reject | compteur « dont N du garde-fou » imprimé à côté ; 0 sur les trois essais |
| 13 | Question unique, sans raison ni traduction (blind) | low | reject | elle dit quoi ajouter ; les questions du modèle sont aussi en français seul |
| 14 | Appariement glouton des nombres (blind) | false | reject | ordre décidé par les Design Notes |
| 15 | Profondeurs 8 et 16, `$ref` dupliqué (blind) | low | reject | sans effet observable |
| 16 | Code postal repris en x/y, contrôle d'étendue (edge) | low | reject | il faut aussi le système dans la demande ; contrôle d'étendue hors règle |
| 17 | Fiche incohérente (« À confirmer », journaux vides) (blind) | low | reject | correctif = éditer ce plan ; journal rempli ici |

Passe 2 (2026-11-03, mêmes lentilles, diff 921a7c7..4ff027d, correctif de recette compris ; code vérifié sur main) : high 0, medium 1, low 1 corrigés ; le reste rejeté ou reporté.

| # | Constat | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 18 | Un même point littéral repris par deux étapes (isochrones à pied et en voiture) devient une question (edge) | medium | patch | `invents_a_point` prenait x, y, epsg deux fois au multiensemble ; points identiques dédoublonnés, test `the_same_given_point_reused_by_two_steps…` |
| 19 | Docs du placement du garde-fou périmées : en-tête de `point.rs`, doc de `compose_plan`, `canal-local.md` muets sur la sortie rejetée (blind, edge) | low | patch | les trois textes disent maintenant que `guard_rejected` rend la question sans redemande |
| 20 | `$defs/Origin` devient `$defs/Point` au long prompt (blind, intent) | low | reject | carried : ligne 2, renommage inhérent au type unique ; le prompt court ne change pas |
| 21 | Exemples d'entraînement non passés au garde-fou (blind) | false | reject | `training/src/families.rs` applique `guard_coordinates` aux exemples engendrés (ticket suivant) |
| 22 | « 652000,6850000 » lu comme une décimale (blind) | low | reject | décision du propriétaire : virgule suivie d'un chiffre = décimale ; sens sûr (question) |
| 23 | Tout nombre de la demande peut tenir lieu d'epsg, x/y inversés (blind) | low | reject | carried : lignes 9 et 16, la règle porte sur la provenance |
| 24 | `bare_coordinate` limité à `x`/`y` (blind, edge) | false | reject | carried : ligne 10 |
| 25 | Question en français seul (blind) | low | reject | carried : ligne 13 |
| 26 | Deux parcours de schéma, profondeurs 8 et 16 (blind, edge) | low | reject | carried : ligne 15 |
| 27 | Référence `step:` imbriquée ou vers `Vec<Point>` toujours refusée (blind, edge) | low | reject | aucune commande du registre ne l'emploie ; sens sûr |
| 28 | Aucun test du vrai compositeur par le canal ou la CLI (blind, intent) | low | reject | le câblage est testé dans `ai` (`compose_plan_with`) ; démo et recette QGIS faites |
| 29 | `source` écrit toujours au canal, seulement pour `guard` à la CLI (blind, intent) | false | reject | voulu (Code Map) : complétions d'entraînement identiques à l'octet |
| 30 | Questions `guard` comptées comme réussies aux essais réels (blind) | low | reject | carried : ligne 12 |
| 31 | Séparateurs de nom de système limités (« Lambert  93 », « WGS:84 ») ; « Lambert 652 000 » coupé (blind, edge) | low | reject | nom non reconnu, donc question de toute façon ; sens sûr |
| 32 | Une chaîne numérique hors `Point` consomme un nombre (edge) | low | reject | aucun paramètre texte du registre ne prend de nombre en pratique |
| 33 | Référence mal formée ou en avant dans une sortie rejetée : question au lieu de redemande (edge) | low | reject | le point ne vient alors ni de la demande ni d'une étape ; sens sûr voulu |
| 34 | `source` perdu après un aller-retour JSON (edge) | false | reject | `serve/plans.rs` l.202 écrit `source` depuis la valeur en mémoire, sans relecture |
| 35 | Doc de `point.rs` « appliquée une fois » (edge, claim) | low | patch | même entrée que 19 |
| 36 | Branche `PLAN_QUESTION` de `panel.py` sans test (verification-gap) | low | reject | carried : ligne 7 ; recette du propriétaire dans QGIS faite et bonne |
| 37 | Le rejet court-circuite la redemande, contre « une seule fois après la redemande » (intent) | false | reject | décision du propriétaire du 2026-10-23, écrite dans l'intention |

Recette du propriétaire dans QGIS (phrase E, puis une phrase avec des coordonnées données), après le correctif de recette : faite et bonne, confirmée le 2026-11-03.

## Design Notes

Consommation des nombres. On lit les nombres de la demande dans un multiensemble. Chaque nombre littéral du plan hors `Point` (durée, distance) retire d'abord une occurrence égale s'il en trouve une. Ensuite, pour chaque `Point`, x, y et epsg retirent chacun une occurrence distincte. S'il en manque une, le garde-fou rend la question.

Question (forme de 5.5), en français : « Il manque à la demande un point en coordonnées : quels sont x, y et le système (code EPSG, par exemple 2154 pour le Lambert-93) ? » (finit par « ? », comme toute question du générateur).

## Verification

Environnement : `ai` et `cli` sous PowerShell (`LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`, voir 5.10 t4b) ; `core` et `training` sous WSL.

**Commands:**
- `cargo test -p cassini-core`, `cargo test -p cassini-training`, `cargo fmt --check`, `cargo clippy -p cassini-core --all-targets -- -D warnings` (WSL) -- expected: succès
- PowerShell : `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo deny check` -- expected: succès
- `python3 -I -m unittest discover -s qgis-plugin/tests` -- expected: succès
- PowerShell, `--features cuda` : `plan_reel`, `question_reel`, `ecart_reel` avec la déclaration de 5.15 ; `C:\ct\release\cassini.exe plan` sur la phrase E -- expected: scores ≥ 5.15, phrase E en question `guard`

**Manual checks (if no CLI):**
- Démo dans QGIS (Claude envoie les phrases au vrai modèle), puis la recette du propriétaire : phrase de démo exacte, phrase E, puis une phrase avec des coordonnées données.
