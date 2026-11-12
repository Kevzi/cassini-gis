---
title: 'PDF de résultat lisible : cadre sur le résultat, résultat mis en évidence, légende en clair'
type: 'feature'
ticket: '9'
created: '2026-11-12'
status: 'built'
baseline_revision: '96e4b99842fbb82439ce2524e0457a880bd9f9fa'
route: 'full'
route_source: 'pinned'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-facade-qgis/story-tampon-sur-une-couche-en-degres-calcul-dans-un-systeme-en-me-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Le PDF de la phrase de démo d'Annecy (2026-11-12) ne montre pas la réponse : emprise du département, résultat orange perdu dans les routes, légende pleine de noms techniques (« 4-analysis.intersect ») et de couches intermédiaires.

**Approach:** Au moment de dicter `layout.export_pdf` dans un plan, le noyau remplit un paramètre caché `focus` (comme `grid.epsg`/`squares` et `work_epsg`) d'après les lignes du plan en cours au journal : couches de résultat, nom en clair, style de mise en évidence, couches à cacher, couches de cadrage. Le geste l'applique à la seule mise en page ; journalisé, relu au rejeu (AD-11).

## Boundaries & Constraints

**Always:** choix dans le noyau seul (AD-5) : une « couche produite » est une sortie de Cassini posée par une étape réussie du plan en cours (sortie d'une commande, ou `layer.open_*` sur `out:k.n`, même règle que `intermediate_files`) ; intermédiaire = produite et consommée par une étape suivante du plan ; résultat = produite et non consommée, dans l'ordre du plan. Nom : un seul résultat → le `title` de l'export ; plusieurs → titre court FR de la commande au registre, « — », nom de sa première couche d'entrée (inventaire pour une couche de l'utilisateur, titre court de l'étape pour une sortie). Style : constante du noyau `StyleSpec::Single` contrastée (magenta plein, contour foncé), vecteurs seulement. Le noyau ôte tout `focus` de l'appelant. La couche du projet garde nom, style, visibilité et place dans l'arbre. Texte lu par le modèle inchangé : descriptions FR/EN, doc des paramètres, exemples, prompt (décision du propriétaire).

**Never:** décision, nommage ou couleur choisis par l'extension ; `focus` montré au modèle, au formulaire ou au client MCP ; recalcul de `focus` au rejeu ; étape ajoutée au plan (AD-23) ; changement du centre d'un export à l'échelle (`scale`) ; restyle ou masquage des couches de l'utilisateur dans le projet ; ré-entraînement.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan de démo | select (A) → buffer (B) → intersect (C) → intersect C×B (D) → export | `focus` : résultat D nommé par le titre, mis en évidence et au-dessus ; A, B, C cachés de la carte, de la légende et des sources ; cadre = D + marge | — |
| Plan sans couche produite | `layer.open_vector` d'un fichier de l'utilisateur, ou export seul, dans un plan | pas de `focus` : comportement actuel | — |
| Export hors plan | `command.run`, formulaire, MCP, `cassini run` | pas de `focus` : comportement actuel, octet pour octet | — |
| Résultat vide | `feature_count` 0 au journal pour tous les résultats | résultats nommés et cachés comme ci-dessus ; cadre = couches de l'utilisateur consommées par le plan ; `framed_on: "inputs"` ; le panneau dit « résultat vide, carte cadrée sur les couches d'entrée » | sans couche d'entrée : pas de cadre, cadrage actuel |
| Plusieurs résultats | deux branches non consommées | deux libellés distincts, les deux en évidence, cadre = leur union | `feature_count` illisible (résumé) : compté non vide |
| Couche disparue | résultat retiré du projet ou décoché avant l'export | le geste ignore l'id absent des couches visibles ; cadre sur ce qui reste, sinon cadrage actuel | — |
| Rejeu | ligne avec `focus`, session neuve ou même session | `focus` journalisé repassé, ids liés aux neufs (`substitute`) ; même PDF, même résultat typé | divergence nommée sinon |
| Rejeu, ligne ancienne | ligne sans `focus` | rejouée sans lui, comme avant | — |

**Décisions de la party d'approbation (2026-11-12 soir, par délégation du propriétaire)** :
1. Nom d'un résultat unique : le `title` de l'export coupé avant un complément final « à <commune> » (« à », « au », « aux », « en » suivi d'un nom propre en fin de titre), au plus 60 caractères (coupe au dernier mot entier) ; calculé par le noyau, journalisé dans `focus`.
2. Style du résultat : toujours par un clone en mémoire de la couche de résultat, style dicté par le noyau, posé dans la seule mise en page (carte et légende), jamais ajouté au projet ; pas de surcharge de style de la carte de mise en page (la légende ne la suit pas).
3. Intitulés de bande des rasters (« Band 1 (Gray) ») : retirés de la légende du PDF quand `focus` le dit (`focus.hide_band_labels: true`, rempli par le noyau pour tout export d'un plan) ; le nom de la couche et sa rampe restent ; la couche de l'utilisateur ne change pas dans le projet.
4. Le nouveau nom de paramètre `focus` passe par un changement de cap léger (`change-mise-en-evidence-du-resultat-dans-le-pdf/`), approuvé par la même party.

</frozen-after-approval>

## Code Map

- `core/src/commands/layout_export_pdf.rs:202-268,299-330` -- paramètres (`focus: Option<Focus>`, `serde(default)`), spec, tests ; descriptions INCHANGÉES.
- `core/src/registry.rs:60-190,245-330` -- `CommandSpec` : métadonnée `focus_param`, non sérialisée, sur le modèle de `mgrs_grid_param`/`work_crs_param`.
- `core/src/journal/mod.rs:383-450,524-590,883-960,1430-1500` -- `fill_mgrs_grid`, `fill_work_crs`, `call_step` (entonnoir), `intermediate_files` (règle produite/consommée à reprendre), boucle du rejeu.
- `core/src/journal/summary.rs` -- résumé du résultat (>200 octets).
- `core/src/style/spec.rs:31-48` -- `StyleSpec::Single`.
- `core/src/contract.rs:190-260` -- contrôle des paramètres déclarés.
- `ai/src/plan.rs:280-290,615-630`, `core/src/serve.rs:924-975`, `qgis-plugin/cassini_qgis/form.py:21` -- paramètres cachés (grammaire, refus, outil MCP, formulaire).
- `qgis-plugin/cassini_qgis/layout_export.py:280-310,470-590,652,879-960` -- `apply_legend_labels`, montage de la carte et de la légende, `_padded`, `_read` (couches visibles, `leave_out`, cadrage).
- `qgis-plugin/cassini_qgis/style_gestures.py` -- traduction d'un `StyleSpec::Single` en symbole QGIS (à réutiliser, jamais dupliquer).
- `qgis-plugin/cassini_qgis/summary.py:325-346`, `texts.py` -- phrase de l'export.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md:711-790` -- fiche `layout.export_pdf` ; `core/tests/catalogue.rs`.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/focus.rs` (nouveau) -- fonction pure `focus_of(lines_of_current_plan, registry, title, inventory?) -> Option<Focus>` : règle du bloc Always, libellés, `framed_on`, constante de style ; inventaire lu par le routeur seulement s'il y a plusieurs résultats -- testable sans QGIS.
- [x] `core/src/commands/layout_export_pdf.rs`, `core/src/registry.rs`, `core/src/contract.rs` -- type `Focus {results: [{layer_id, label, style?}], hidden: [id], frame: [id], framed_on}` ; `.focus_param("focus")` ; contrat : facultatif, au schéma.
- [x] `core/src/journal/mod.rs` -- `call_step` : ôter `focus` de l'appelant ; si `plan_step` est donné, lire les lignes du plan en cours au journal et remplir avant `resolve_call` (journalisé dans `params`) ; rejeu : `focus` de la ligne repassé après `substitute`, jamais recalculé.
- [x] `ai/src/plan.rs`, `core/src/serve.rs`, `qgis-plugin/cassini_qgis/form.py` -- cacher `focus` comme `work_epsg` ; refus d'un plan qui l'écrit.
- [x] Tests noyau -- `focus.rs` : chaque ligne de la matrice ; `journal/tests.rs` (QGIS simulé) : démo de 5 étapes → `focus` journalisé, valeur de l'appelant écrasée, hors plan sans `focus`, rejeu neuf identique avec ids liés, ligne ancienne ; MCP sans `focus` ; `catalogue.rs`.
- [x] `qgis-plugin/cassini_qgis/layout_export.py` -- lire et valider `focus` (`layout.bad_params` sinon) ; ôter `hidden` des couches, de la légende et des sources ; résultats en tête des couches de la carte et de la légende ; libellé posé sur le nœud de légende seul (`setUseLayerName(False)`) ; style posé dans la mise en page seule (surcharge de style de la carte de mise en page, ou clone en mémoire jamais ajouté au projet, choix relevé par la sonde) ; cadre = `_padded` de l'union de `frame` sans `scale` ; sans `focus` : appels au fond identiques à aujourd'hui.
- [x] `qgis-plugin/tests/test_layout_export.py` -- `FakeBackend` : chaque comportement ci-dessus, nom et style de la couche du projet inchangés après l'export, id absent ignoré, `focus` invalide refusé, régression sans `focus`.
- [x] `qgis-plugin/cassini_qgis/summary.py`, `texts.py` -- phrase de l'export : « résultat vide, carte cadrée sur les couches d'entrée » si `framed_on` vaut `inputs` (FR/EN, lecture des paramètres seule).
- [x] `catalogue-des-commandes.md` -- `focus` à la fiche, une fois le changement de cap approuvé ; `deferred-work.md` : légende des MNT de l'utilisateur.

**Acceptance Criteria:**
- Given la session QGIS d'Annecy, when la phrase de démo est exécutée, then le plan a les mêmes 5 étapes et le PDF répond à la question d'un coup d'œil.
- Given `cargo test --workspace` et la recette Python, then tout est vert, `catalogue.rs` compris.

## Implementation Notes

- Noyau : `core/src/focus.rs` (types `Focus`, `FocusResult`, `FramedOn` ; `focus_of`, `needs_inventory`, `result_label`, `short_title`, `result_style`). Consommateur = étape réussie qui écrit une sortie, hors ouverture dictée ; ses paramètres sont lus à toute profondeur (id de couche ou repère `out:k.n`), hors `focus` et `leave_out` : une mise en forme (`layer.set_style`) ou un export antérieur ne consomment rien. `framed_on` vaut `results`, `inputs` ou `visible_layers` (aucune couche d'entrée : cadrage d'avant). Couches d'entrée = paramètres `layer_id` et `…_layer_id`. Libellés en double suffixés « (2) ». Titre court = règle de `summary.short_title` du panneau, réécrite en Rust.
- `Session::fill_focus` dans `call_step`, après `fill_work_crs` : valeur de l'appelant ôtée (tout appel) ; rien hors plan ni à `plan_step` 1 ; plan en cours lu par `current_plan` (même découpe que `last_plan`) ; inventaire lu seulement pour plusieurs résultats, une lecture ratée est l'erreur de l'export (code gardé, message qui nomme `layout.export_pdf`), sans ligne. Rejeu : rien à faire, `focus` est dans les paramètres journalisés et `substitute` lie ses ids.
- Cas « plan sans couche produite » : pas de `focus` du tout (matrice), donc pas de `hide_band_labels` pour cet export, alors que la décision 3 dit « tout export d'un plan » : la matrice a été suivie, écart noté au `deferred-work.md` (légende des MNT).
- Modèle : `model_params` ôte `focus`, et l'élagage des `$defs` devient transitif (sinon `FocusResult`, `StyleSpec`… restaient au prompt). Vérifié : `system_text` + `plan_grammar` identiques octet pour octet à la base (85,6 Ko). Relais MCP : `without_core_filled` ôte `focus` et élague les définitions devenues inutiles (`prune_defs`) ; `Point` reste.
- Extension : `_focus_param` (forme stricte, style par `normalize_style`, `single` seul) ; `highlighted_clone` (clone de `QgsVectorLayer`, `apply_single` de `style_gestures`) ; `hide_band_label` (premier nœud `QgsSimpleLegendNode` suivi d'autres, sauf `LEGEND_FIRST_LABEL`). Choix relevé par la sonde : clone en mémoire (décision 2). Ajout constaté à la sonde : un nom de résultat long faisait replier toute la légende (`fit_legend`) ; avec `focus`, `setAutoWrapLinesAfter` fait passer le nom à la ligne dans la colonne.
- Vérification (2026-11-12) : `cargo fmt --check`, `cargo clippy --workspace --exclude cassini-app --all-targets -D warnings` sans avertissement ; `cargo test --workspace --exclude cassini-app` ; `python -m unittest discover -s qgis-plugin/tests` (746). Sonde nouvelle `qgis-plugin/tests/probe_layout_focus.py` sous QGIS 3.44.15 réel (projet mémoire + MNT de test) : 15/15, PNG regardé (aplat magenta au-dessus, légende nommée et repliée à la ligne, MNT sans « Band 1 (Gray) » avec sa rampe, intermédiaires absents, projet inchangé).
- Suite de relecture (2026-11-12, demandée par le coordinateur) :
  - Bornes du plan : la session note `plan_start`, la ligne où commence le plan en cours (son étape 1, même refusée sans ligne ; `None` hors plan). `current_plan` remonte depuis la fin du journal et exige exactement les étapes k-1 … 1 de suite, chaque étape suivie des seules lignes dictées par le noyau (`dictated_by_core`) à la même place, la première ligne au plus tôt à `plan_start` ; sinon pas de `focus`. Tests : plan commencé à la ligne 7, nouveau plan dont l'étape 1 ouvre un fichier de l'utilisateur, étape 1 refusée sans ligne.
  - `focus.rs` : rien si aucun résultat (tout consommé ou retiré) ; couches produites dédoublonnées par `layer_id` ; une couche retirée par `layer.remove` dans le plan n'est pas un résultat ; couches d'entrée tirées des seules étapes réussies qui écrivent une sortie ; `first_input` s'arrête à la première clé de couche (`layer_id`, chemins déclarés, `…_layer_id`), nommée ou non ; noms de plusieurs résultats bornés à 60 caractères (le titre de la commande cède la place au nom de l'entrée) ; apostrophe typographique et noms élidés (« d'Abeau ») admis dans un lieu final ; échec d'écriture de `focus` en `core.internal` ; inventaire lu seulement si un résultat a pour première entrée une couche de l'utilisateur. Tests unitaires réécrits sur de vrais numéros de ligne ; test `Session` pente → ouverture dictée → pente de la pente → export.
  - Élagage des `$defs` : une seule fonction, `cassini_core::registry::prune_defs`, employée par `serve.rs` et `ai/src/plan.rs`.
  - Extension : résultats vecteur en tête, résultats raster juste au-dessus du premier raster de l'utilisateur (sous ses vecteurs) ; le clone perd les libellés de légende de Cassini et le seuil d'échelle d'un `layer.set_style` antérieur ; `hide_band_label` part de `legendNodeOrder` ; ids en double refusés ; la ligne imprimée des sources nomme un résultat par son libellé (`sources` garde les fichiers) ; le cadrage de repli est celui d'avant 7.9, sur les couches visibles avant l'ôtage ; un clone en échec est dit dans `warnings`, nouveau champ facultatif du résultat (catalogue mis à jour). Panneau : « résultat vide » aussi pour `visible_layers`, « n résultats vides sur m » quand une partie l'est.
  - Sonde `probe_layout_focus.py` étendue (résultat raster, résultat déjà stylé) : 21/21 sous QGIS 3.44.15.
- Vérification réelle (2026-11-12 soir), noyau `--features gpu` construit (`-j 4`) dans `C:\ct-79` depuis une copie Windows de la branche (`C:\ct-src-79`), extension lue depuis cette copie, modèle `ajuste-continu-5-54-3` seul dans `CASSINI_MODELS` ; rien installé :
  - `probe_demo.py` sur une copie de `projet-test-annecy.qgz` : plan de 5 étapes inchangé, `focus` journalisé (résultat l'étape 4, étapes 1 à 3 cachées, `framed_on: results`) ; PDF rendu en PNG et regardé : cadré autour des gares d'Annecy et de Pringy, parcelles en aplat magenta au-dessus, légende en tête « Parcelles constructibles du PLU à moins de 500 m d'une gare », aucun nom « N-analysis.* », sources sans fichier intermédiaire. Copie supprimée.
  - `probe_recette.py` + `annecy.json` : 20/20 (barre 16), rejeu après réouverture identique (31 étapes) ; `projet-gros-annecy.qgz` + `phrases-gros.json` : 6/7 (barre 6, raté G06 comme avant 7.9), rejeu identique (7 étapes). Le rejeu en session diverge, indicatif et attendu (couches des phrases encore au projet). Copies de recette supprimées. Durée de l'export non relevée par la sonde.
  - QGIS 4.x non essayé.

## Plan Change Log

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Verdicts : high 1, medium 6, low 12, false 0, maybe-false 0. Corrigés 17 (patch, même implémenteur), reportés 4.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Limite du plan en cours fausse (lignes sans `plan_step` sautées, ouverture d'un fichier à l'étape 1, étape 1 refusée sans ligne) : un export peut prendre les couches d'un plan précédent | high | patch | `plan_start` gardé par la session ; `current_plan` exige les étapes k-1…1 contiguës ; trois tests |
| Tests de `focus.rs` aux numéros d'étape irréels ; aucun plan de terrain par `out:k.n` | medium | patch | numéros de lignes réels ; test `Session` pente → ouverture dictée → export |
| Résultat raster placé au-dessus de tout | medium | patch | vecteurs en tête, rasters au-dessus du raster le plus haut de l'utilisateur |
| Clone qui reprend les propriétés de légende et la visibilité par échelle de Cassini | medium | patch | effacées avant le style ; sonde (résultat déjà stylé) |
| Aucun résultat (tout consommé) → focus quand même | medium | patch | `None` |
| Couche retirée plus loin comptée comme résultat ; doublons par `layer_id` | medium | patch | exclue ; dédoublonnage ; doublon refusé par le geste |
| Ligne des sources avec le nom technique du résultat | low | patch | nommé par son libellé |
| `input_layers` et `first_input` incohérents avec `consumed()` | low | patch | étapes réussies à sortie seulement ; première clé de couche |
| Libellés multiples sans borne ; apostrophe typographique | low | patch | 60 caractères ; ’ acceptée |
| Code d'erreur `registry.invalid_params` sur un échec interne | low | patch | `core.internal` |
| Inventaire relu sans besoin | low | patch | seulement si un résultat vient d'une couche de l'utilisateur |
| `hide_band_label` sur un ordre de nœuds personnalisé | low | patch | `legendNodeOrder` |
| Cadrage de repli calculé après le filtre | low | patch | avant, comme avant 7.9 |
| Clone en échec silencieux | low | patch | avertissement `CLONE_FAILED` dans le résultat |
| Résultat vide non dit (`visible_layers`, résultats en partie vides) | low | patch | phrases FR/EN, test |
| Élagage des `$defs` écrit deux fois | low | patch | `registry::prune_defs` partagé |
| `focus` non vérifié absent du formulaire | low | patch | test |
| Dessin du focus vérifié seulement par la sonde manuelle | low | defer | la sonde (21/21) et la démo réelle tiennent lieu ; dépôt sans QGIS en CI |
| Libellés en français seulement | low | defer | `deferred-work.md` |
| Titres à majuscules coupés par `result_label` | low | defer | `deferred-work.md` |
| Sorties de plans précédents visibles avec leurs noms techniques | low | defer | `deferred-work.md` |

## Design Notes

**Party simulée (noms lisibles).** *UX :* reprendre la phrase en langage courant du panneau (`summary.py`). *Architecte :* non — elle vit dans l'extension, dépend de la langue de l'interface et n'est pas journalisée : nommer dans l'extension contredit AD-5 et le rejeu pourrait diverger. *Géomaticienne :* la légende d'une carte de réponse dit ce que montre l'aplat ; le titre, écrit par le modèle depuis la demande et déjà journalisé, est ce nom (« Parcelles constructibles à moins de 500 m d'une gare »). Plusieurs résultats : le titre ne suffit plus, d'où titre court de la commande + couche d'entrée, calculés dans le noyau. **Décision : titre pour un résultat, titre court + entrée sinon, tout dans le noyau.**

**Pourquoi un paramètre.** Rien de ce que le geste reçoit aujourd'hui ne dit quelle couche est le résultat ; le geste ne peut pas le déduire sans décider (AD-5). `leave_out` reste réservé au rejeu. Un paramètre rempli par le noyau suit le précédent de 9.16, mais le catalogue figé (§1) n'admet un nom de paramètre nouveau que par un changement de cap : voir Open Questions.

**Forme journalisée (exemple).**
```json
"focus": {"results": [{"layer_id": "D", "label": "Parcelles constructibles à moins de 500 m d'une gare",
  "style": {"kind": "single", "fill": "#c2187a", "stroke": "#3d0026", "width_mm": 0.5}}],
  "hidden": ["A", "B", "C"], "frame": ["D"], "framed_on": "results"}
```

## Open Questions

- **Changement de cap léger à approuver (propriétaire).** `layout.export_pdf` gagne un paramètre facultatif `focus`, rempli par le noyau et caché au modèle, comme `work_epsg` en 9.16 ; identifiant, autres paramètres, résultat et texte lu par le modèle inchangés ; aucune AD contredite (AD-5, AD-11, AD-23 tenues). À écrire par `bmad-correct-course` avant le code, catalogue en `planned_params` puis `params`.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement.
- `cargo test --workspace` -- expected: vert, `core/tests/catalogue.rs` compris.
- `python -m unittest discover -s qgis-plugin/tests` -- expected: vert.

**Manual checks (if no CLI):**
- Installer noyau et extension de la branche, modèle installé seul (mémoire « installer avant la recette »).
- `probe_demo.py` sur une copie de `projet-test-annecy.qgz`, `QT_QPA_PLATFORM=windows`, PDF rendu en PNG et regardé : emprise de l'ordre de la ville (gares d'Annecy visibles, plus le département) ; parcelles retenues en aplat magenta, au-dessus, distinctes des routes ; légende sans « N-analysis.* » ni tampon ni sélection, résultat nommé par le titre ; titre, échelle, nord, sources (sans fichier intermédiaire) présents ; couches du projet inchangées (nom, style, coche). Fermer, rouvrir, rejouer : identique.
- Recette `annecy.json` : 20/20, rejeu identique ; `projet-gros-annecy.qgz` + `phrases-gros.json` : barre tenue, durée de l'export relevée.
