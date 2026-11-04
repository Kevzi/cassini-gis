---
title: 'Ré-entraînement groupé : commandes de terrain et export à l''échelle sans PDF'
type: 'feature'
ticket: '41'
created: '2026-11-04'
status: 'in-progress'
baseline_revision: 'd5eed5b9e8d76c784fcf44a411f7fa1ee51f0483'
route: 'full'
route_source: 'auto'
risk: 'high'
review: ''
review_source: ''
lenses_ran: []
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le modèle installé (5.39) n'a vu ni les trois commandes de terrain (il a appris des refus `NO_SLOPE`, `NO_LINE_OF_SIGHT`, `NO_VIEWSHED`, retirés en 4.6) ni d'export à l'échelle sans le mot « PDF » (5.40). De plus, `plan_reel` attend encore un refus pour S20, S21, S22 et S24 : un modèle qui planifie le terrain y « échouerait » alors qu'il a raison.

**Approach:** Remettre d'abord à jour les attendus de terrain de `plan_reel` et ajouter un essai réel de terrain et d'export sans « PDF ». Ensuite ré-entraîner par la commande unique (même base Qwen3.5-4B, même méthode, GPU), fusionner en GGUF, installer sous la déclaration retenue par le panneau en gardant 5.39 sous son nom, mesurer sur GPU, puis faire la recette dans une seule session QGIS sur le MNT de Villeneuve-le-Roi.

## Boundaries & Constraints

**Always:** les phrases de `tests/epreuve-modele-local/sentences.json` sont lues telles quelles et ne sont jamais modifiées : seuls les attendus de `ai/tests/plan_reel.rs` changent. Les essais réels se lancent avec `--features cuda`. Le modèle de 5.39 reste installé sous `cassini-composition-4b-5-39`. Une phrase mal comprise devient un constat consigné, jamais une règle de prompt.

**Never:** lancer l'entraînement sans le feu vert du propriétaire (hitl : sa carte graphique est prise plusieurs heures). Ne pas toucher au générateur ni au garde-fou des coordonnées (`core/src/point.rs`, entrée au report séparée). Aucun vocabulaire défense.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| S20 champ de vision | point Lambert-93, 25 m de haut, 2 km | `terrain.viewshed` : `dem` = `donnees/mnt_5m.tif`, `observer`, `observer_height` 25 m, `radius` 2 km | Un refus est un échec |
| S22 pente | « en pourcentage » | `terrain.slope` : `dem`, `unit: percent` | Un refus est un échec |
| S21 ligne de vue | système écrit une seule fois pour deux points | juste si une question de source `Guard` (décision du propriétaire, 2026-11-04) ; hors du plafond des fausses questions | Un plan ou un refus est un échec |
| S23, S24 | terrain puis moyenne par commune | refus (décision du propriétaire, 2026-11-04 : statistiques sur un raster) | Un plan est un échec |
| Export sans PDF | « … la carte au 1:25 000 » | un seul `layout.export_pdf` à `scale` | Un refus est un échec |

</frozen-after-approval>

## Code Map

- `ai/tests/plan_reel.rs` l.47-85 -- `expected(id)` : `None` = refus attendu. Le commentaire l.78 liste S20-S24 en refus. Comparaison exacte des `calls`. Les questions comptent dans `question_cap` (1 au prompt court). Inventaire de l'épreuve : MNT `mnt_5m_b40c2e97_…`, `source` = `donnees/mnt_5m.tif`, Lambert-93.
- `training/src/families.rs` `ligne_de_vue`, `champ_de_vision`, `pente` -- la forme apprise : `dem` = `source` du raster, `observer`/`target`, hauteurs et rayon seulement s'ils sont écrits (`{value, unit}`).
- `ai/tests/echelle_utm_reel.rs` -- patron du nouvel essai : tableau `SENTENCES`, `expected(id)`, `comparable` (titre non comparé), `every_sentence_has_a_valid_expected_plan`.
- `training/tests/recouvrement.rs` -- ajouter le nouvel essai (constante `include_str!` + `pairs`), comme `ECHELLE_UTM_REEL`.
- `training/entrainement/pipeline.py`, `parite_decoupeurs.py`, `docs/entrainement-du-modele.md` § « Ré-entraînement de 5.39 » -- commande, parité, patron du compte rendu.
- Installation : `%LOCALAPPDATA%\Cassini\bin\models\` (liens durs vers `C:\Users\kevin\cassini-modeles\`), déclaration `cassini-composition-4b.modele.json`.
- Recette : sonde jetable sur le patron de `%TEMP%\sonde-5-39\`, MNT de `../donnees-test-cassini/dem/`.

## Tasks & Acceptance

**Execution:**
- [x] `ai/tests/plan_reel.rs` -- attendus S20, S22 (plans), S21 (question du garde-fou, hors plafond), commentaire d'en-tête et l.78 -- les attendus ne périment plus
- [x] `ai/tests/terrain_reel.rs` (nouveau) -- 8 phrases sur l'inventaire de l'épreuve : 2 pentes, 2 lignes de vue (système écrit à chaque point), 2 champs de vision, 2 exports à l'échelle sans « PDF » ; seuil 7/8 ; test sans modèle qui valide chaque attendu par `check_plan` -- mesure ce que 4.6 et 5.40 ont appris
- [x] `training/tests/recouvrement.rs` -- inclure `terrain_reel` -- aucune de ses phrases dans le jeu engendré
- [ ] (hitl, feu vert du propriétaire) entraînement par la commande unique vers `~/cassini-entrainement/5-41`, fusion GGUF, parité des découpeurs -- le modèle
- [ ] installation (5.39 gardé en `-5-39`) ; `plan_reel`, `question_reel`, `ecart_reel`, `echelle_utm_reel`, `terrain_reel` sur GPU -- les barres
- [ ] recette QGIS : une dizaine de phrases enchaînées (pente, ligne de vue, champ de vision, export « au 1:25 000 » sans PDF, phrase de démo), puis rejeu -- le produit
- [ ] `docs/entrainement-du-modele.md` -- section « Ré-entraînement de 5.41 » sur le patron de 5.39 -- la trace

**Acceptance Criteria:**
- Given le modèle entraîné et installé, when tournent les essais sur GPU, then aucun recul par rapport à 5.39 : `plan_reel` ≥ 24/25 avec les nouveaux attendus, `question_reel` ≥ 11/12, `ecart_reel` ≥ 32/40, `echelle_utm_reel` ≥ 7/8, et `terrain_reel` ≥ 7/8 ; chaque score est consigné.
- Given QGIS, le projet réel et le MNT de Villeneuve-le-Roi, when une dizaine de phrases s'enchaînent dans une seule session puis le journal est rejoué, then les plans sont justes et le rejeu rend `identical: true`.

## Implementation Notes

- 2026-11-04 : `plan_reel` attend désormais `Want::{Plan, Refusal, GuardQuestion}`. S21 est juste si la question rendue a la source `Guard` ; elle n'entre pas dans `questions` ni `guarded` (le plafond). Tests sans modèle ajoutés : chaque plan attendu passe `check_plan` et le garde-fou ; le plan juste de S21 est bien gardé (un seul « Lambert-93 » pour deux points), ce qui confirme l'attendu.
- `terrain_reel` : T01-T02 pentes, L01-L02 lignes de vue, V01-V02 champs de vision, X01-X02 exports sans « PDF » (5 000 et 10 000). Chaque attendu passe `check_plan` et le garde-fou des coordonnées ; aucune phrase n'est engendrée (recouvrement vert).
- Arrêt avant l'entraînement : il attend le feu vert du propriétaire (hitl). Restent : entraînement, installation, mesures GPU, recette QGIS, section de `docs/entrainement-du-modele.md` (elle consigne les scores, donc après les mesures).

## Plan Change Log

## Review Triage Log

## Verification

**Commands:**
- `cargo test -p cassini-ai --test plan_reel --test terrain_reel` et `cargo test -p cassini-training` -- expected: verts sans modèle
- `CASSINI_TEST_MODEL=<déclaration> cargo test -p cassini-ai --release --features cuda -- --ignored <essai>` -- expected: les barres ci-dessus
- `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` -- expected: propres
