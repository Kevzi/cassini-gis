---
title: 'Ré-entraînement groupé : point en UTM et échelle de l''export PDF'
type: 'feature'
ticket: '39'
created: '2026-11-03'
status: 'built'
baseline_revision: '21eaf272805b8d48b27feac54d4797cd379a39e7'
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

**Problem:** Le modèle ajusté installé (5.17) n'a jamais vu de point en UTM ni d'export à une échelle : « Exporte la carte au 1:25 000 » est refusé, et un point « UTM 31N » ne sort pas en `epsg` 326xx. Les familles du générateur existent (`isochrone_utm` en 5.37, `export_pdf_scale` en 5.38), sans ré-entraînement.

**Approach:** Ré-entraîner par la commande unique de 5.10 (même base, même méthode, jeu régénéré qui contient ces familles), fusionner, installer le nouveau modèle sous la déclaration retenue par le panneau, mesurer sans recul par rapport à 5.17, plus un court essai réel à l'échelle et en UTM, puis une démo réelle dans QGIS.

## Boundaries & Constraints

**Always:** Commande unique `training/entrainement/pipeline.py`, sortie hors du dépôt (`~/cassini-entrainement/5-39`), aucun réglage changé. Mesures sur GPU (`--features cuda`). Décision (propriétaire, 2026-11-03) : barres de non-recul — `plan_reel` ≥ 24/25, `ecart_reel` ≥ 32/40, `question_reel` ≥ 11/12. Le nouvel essai rend le plan attendu sur au moins 7 phrases sur 8. L'ancien modèle reste installé sous `cassini-composition-4b-5-17` (GGUF et déclaration). Hitl : l'entraînement (≈ 5 h 30 sur la carte du propriétaire) ne part que sur son feu vert.

**Never:** Modifier le générateur, le prompt, la grammaire, le jeu tenu à l'écart ou les phrases de `tests/epreuve-modele-local/`. Retoucher une barre pour qu'elle passe. Installer le nouveau modèle si une barre est manquée sans décision du propriétaire.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Échelle seule | « Exporte la carte au 1:25 000 » | `layout.export_pdf`, `scale: 25000`, titre non vide | — |
| Formes d'échelle | « 1/25000 », « au 25 000e », « au 1:50k » | `scale` égal au nombre écrit | — |
| Échelle au bout d'une recherche | « Garde les zones U du PLU et sors la carte en PDF au 1:10 000 » | sélection puis export à `scale: 10000` | — |
| Point en UTM | isochrone « depuis le point 452 300 / 5 411 800 (UTM 31N), 10 min à pied » | `analysis.isochrone`, `origin.epsg` 32631, x et y écrits | — |
| Export sans échelle | phrases existantes de `plan_reel` | pas de `scale` ajouté | barre `plan_reel` |
| Barre manquée | un score sous sa barre | modèle non installé, chiffres consignés | décision du propriétaire |

</frozen-after-approval>

## Code Map

- `ai/tests/question_reel.rs` -- patron du nouvel essai : `common::test_model`, `common::shown_inventory`, `compose_plan_with`, `common::restored`, score phrase par phrase, `#[ignore]`.
- `ai/tests/plan_reel.rs` l.20-45 -- ids de l'inventaire de l'épreuve (`PLU`, `ROUTES` réseau de lignes) et helpers `select` ; comparaison au plan attendu (le titre d'export n'est pas comparé, seulement non vide).
- `core/src/commands/analysis_isochrone.rs` l.16-70 -- `layer_id`, `origin: Point`, `mode` (`walk`/`car`), `duration {value, unit}`.
- `training/entrainement/pipeline.py`, `docs/entrainement-du-modele.md` l.257-320 -- la commande et ce qui sort ; sections « Tâche B de 5.17 » l.904-1050 : modèle du compte rendu (entraînement, parité, essais réels, durée, démo, bilan).
- `training/entrainement/parite_decoupeurs.py` -- parité des découpeurs, à relancer.
- Installation : `%LOCALAPPDATA%\Cassini\bin\models\` (liens durs vers `C:\Users\kevin\cassini-modeles\`), déclaration `cassini-composition-4b.modele.json` retenue par `QGIS3.ini` (`cassini/model`).
- Démo : sonde jetable sur le patron de `%TEMP%\sonde-5-38\sonde.py` (copie du projet réel, phrases enchaînées, rejeu).

## Tasks & Acceptance

**Execution:**
- [x] `ai/tests/echelle_utm_reel.rs` -- nouvel essai réel : 8 phrases (5 à l'échelle, dont une au bout d'une recherche, toutes les formes de la matrice ; 3 isochrones en UTM 31N ou 32N), seuil 7/8 -- mesure ce que 37 et 38 ont appris
- [x] (hitl, feu vert du propriétaire) entraînement par la commande unique vers `~/cassini-entrainement/5-39` -- le modèle
- [x] parité des découpeurs ; installation (ancien gardé en `-5-17`) ; `plan_reel`, `question_reel`, `ecart_reel`, `echelle_utm_reel` sur GPU -- les barres
- [x] démo dans QGIS (phrase de démo, « Exporte la carte au 1:25 000 », isochrone UTM) puis rejeu -- le produit
- [x] `docs/entrainement-du-modele.md` -- section « Ré-entraînement de 5.39 » sur le patron de 5.17, et lignes de famille déjà là -- la trace

**Acceptance Criteria:**
- Given le modèle entraîné et installé, when tournent les essais réels sur GPU, then les quatre barres sont tenues et chaque score est consigné.
- Given QGIS et le projet réel, when les trois phrases de la démo s'enchaînent puis le journal est rejoué, then les plans sont justes, les PDF lisibles et le rejeu rend `identical: true`.

## Implementation Notes

- Feu vert GPU du propriétaire le 2026-11-03 (réponse directe). Branche rebasée sur main (4.1) avant de commencer ; `baseline_revision` gardée (`21eaf27`) : le diff du ticket se lit depuis `5b83b97`.
- Essai `ai/tests/echelle_utm_reel.rs` (E01-E05, U01-U03) et ses phrases ajoutées à `training/tests/recouvrement.rs` (54 phrases), commités avant l'entraînement (`06d4608`) pour que `rapport.json` cite un arbre propre.
- Entraînement : 316 min, validation 296/300 ; parité 300/300 ; modèle `b6fc49d0…`, copie dans `C:\Users\kevin\cassini-modeles\ajuste-5-39\`.
- Mesures GPU : `plan_reel` 25/25, `question_reel` 12/12, `ecart_reel` 34/40, `echelle_utm_reel` 7/8 (E01 « Exporte la carte au 1:25 000. » refusé). Barres tenues → installé, 5.17 gardé en `-5-17`.
- Démo QGIS (`%TEMP%\sonde-5-39\`, `projet-sonde-5-39.qgz`) : phrase de démo et isochrone UTM 31N justes, PDF écrit, rejeu `identical: true` ; « Exporte la carte au 1:25 000. » refusée → second critère d'acceptation manqué sur cette phrase. Cause : aucune tournure de `export_pdf_scale` sans « PDF ». Décision du propriétaire (2026-11-04) : clore 5.39, modèle gardé, entrée au report pour le générateur.

## Plan Change Log

## Review Triage Log

Passe 1 (2026-11-04, lentilles thorough) : medium 1, low 4, false 3, rejets 9. Vérification : `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`, `cargo test --workspace` verts (PowerShell).

| # | Constat (lentille) | Verdict | Route | Preuve / action |
|---|---------|---------|-------|-----------------|
| 1 | La démo n'a jamais sorti un PDF à l'échelle avec le nouveau modèle : E01 refusée, la phrase de démo n'a pas d'échelle (blind) | medium | patch | Seconde sonde `projet-sonde-5-39b.qgz` : « Sors-moi la carte en PDF au 1/25000. » → `scale: 25000`, PDF 1,9 Mo, centre journalisé, rejeu `identical: true` ; consigné dans la doc |
| 2 | « voit les quatre modèles » sans dire lesquels ; « Une redemande (0 sauvée) » obscur (blind) | low | patch | Doc précisée |
| 3 | Entrée du report qui cite des numéros de ligne (blind) | low | patch | Nom de la famille à la place |
| 4 | Barre 7/8 qui laisse passer E01, la phrase du problème ; matrice « Échelle seule » manquée ; critère 2 manqué sous tâches cochées (blind, edge, intent) | low | reject | Décision du propriétaire du 2026-11-04 (clore, ticket à part), écrite dans les Implementation Notes, la doc et `deferred-work.md` ; la corriger ici modifierait le générateur, interdit par l'intention |
| 5 | Comparaison stricte des nombres : `452300.0` contre `452300` compterait faux (blind, edge) | low | reject | Le modèle écrit des entiers (E02-E05, U01-U03 justes) ; le correctif ajoute une normalisation pour un cas non observé ; écart possible seulement vers le bas (sens sûr) |
| 6 | Paramètre facultatif ajouté par le modèle compté faux (edge) | false | reject | Même règle stricte que `plan_reel` ; aucun cas observé |
| 7 | Deux formes de la commande de mesure (plan contre doc) (blind) | low | reject | La doc donne la commande lancée ; la ligne du plan est un filtre équivalent |
| 8 | Nombre d'exemples `isochrone_utm` / `export_pdf_scale` non montré (blind) | false | reject | Les tests du générateur de 5.37 et 5.38 exigent ces familles dans le jeu par défaut ; l'essai réel prouve l'apprentissage |
| 9 | H23 recule sans analyse (blind, intent) | low | reject | Barre totale tenue (34 ≥ 32, 33 en 5.17) ; recul nommé dans la doc ; H23 était fausse en 5.15 |
| 10 | Pas d'UTM sud, un seul trajet en voiture (blind) | low | reject | Le générateur n'apprend que 31N/32N ; le sud est couvert par le garde-fou (5.37) |
| 11 | Plans attendus vérifiés par `validate_params` sans `check_plan` (blind) | low | reject | Un plan attendu faux ferait échouer l'essai réel, visible au premier passage ; 7/8 rendus justes |
| 12 | Plan `in-review` sans entrée au Plan Change Log ; triage vide (blind, edge) | false | reject | Aucun changement de plan (pas de boucle) ; triage écrit ici ; correctif = éditer ce plan |
| 13 | Style : `eprintln!` multiligne, `7`/`8` en dur, ligne longue, `baseline_revision` (blind) | low | reject | Cosmétique ; `cargo fmt` et clippy verts ; base expliquée dans les notes |

## Verification

**Commands:**
- `cargo test -p cassini-ai --release --features cuda -- --ignored plan_reel question_reel ecart_reel echelle_utm_reel` avec `CASSINI_TEST_MODEL` = la déclaration installée (PowerShell, `LIBCLANG_PATH`, `CARGO_TARGET_DIR=C:\ct`) -- expected: barres tenues
- `cargo test --workspace`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings` -- expected: vert

**Manual checks (if no CLI):**
- Sonde QGIS enchaînée et rejeu ; puis recette du propriétaire dans le panneau.
