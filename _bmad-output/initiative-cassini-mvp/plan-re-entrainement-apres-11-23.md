---
title: 'Ré-entraînement après 11.23 : reprojection avant analyse, conversion, refus manqués'
type: 'feature'
ticket: ''
created: '2026-11-09'
status: 'built'
baseline_revision: '87ab17c52e666ef3938a9553b1d5a538b9bf20fd'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/docs/entrainement-du-modele.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/plan-second-essai-11-23.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le modèle installé (`ajuste-11-23c`) rate encore des demandes réelles. En priorité, à la recette QGIS du 2026-11-09 sur Annecy, une zone tampon autour des gares (couche CSV en EPSG:4326, projet en EPSG:2154) part sans `data.reproject`. Il ne connaît pas `data.convert` (10.9). Aux essais GPU, il manque des refus (vue 3D d'une partie d'une couche ; export d'un sous-ensemble filtré en tableur ; PDF qui choisit ses couches ; statistique d'altitude par zone), perd une rampe nommée d'un dégradé, écrit mal la zone UTM d'arrivée d'un point, pose une question sur un code de zone du PLU non littéral, et saute ou ajoute une étape dans les enchaînements sélection-sélection-tampon-intersection.

**Approach:** corriger le générateur d'exemples par familles (jamais par phrase) d'après ces constats, puis lancer l'entraînement accéléré, les essais GPU contre les barres de 11.16 et de 11.23, et rendre les résultats au propriétaire, qui décide seul de l'installation.

## Boundaries & Constraints

**Always:** chaque correction est une famille ou un poids du générateur, dérivée d'un constat abstrait ; aucune phrase du jeu tenu à l'écart (`plan_reel`, `ecart_reel`, `style_reel`, `epic11_reel`, `echelle_utm_reel`, `tenu_a_l_ecart`) n'est recopiée ni citée dans le générateur, et `training/tests/recouvrement.rs` reste vert ; mêmes barres qu'en 11.23 (plan_reel ≥ 23/25, question_reel ≥ 11/12, ecart_reel ≥ 32/40 ou 32/39 si H36 est rescoré, echelle_utm_reel ≥ 7/8, terrain_reel ≥ 7/8, epic11 ≥ 1/2 par commande, F03 en question, style_reel ≥ 9/10), plus la phrase des gares d'Annecy rejouée dans QGIS ; l'entraînement tourne depuis un worktree, jamais en changeant de branche dans le dépôt principal ; le modèle 11.23c reste installé tant que le propriétaire n'a pas décidé.

**Décisions du propriétaire (2026-11-09) :** le cas des gares d'Annecy attend la reprojection vers 2154, celui du vrai projet (1a) ; les trois écarts d'Annecy (deux couches en 4326, source `.csv`, inventaire long) passent en premier, la cible « système du projet » du cas « tout en 4326 » en second (2a) ; un seul plan, un seul entraînement, malgré sa taille (3a).

**Never:** installer le nouveau modèle sans décision du propriétaire ; supprimer un modèle (déplacer seulement) ; changer le noyau, le catalogue ou l'extension ; baisser une barre.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Gares d'Annecy | projet en 2154, 8 couches en 2154, parcelles et gares (`.csv`) en 4326, tampon autour des gares | `data.reproject` des gares vers 2154 puis le tampon | — |
| Tout en 4326 | projet en système métrique (2154 ou UTM), toutes les couches vecteur en 4326 | `data.reproject` vers le système du projet puis l'analyse | — |
| Couche déjà métrique | gares en mètres, autre couche en 4326 | tampon seul (S07 tenu) | — |
| Conversion | « exporte les gares en shapefile » | une étape `data.convert` | — |
| Tableur filtré | sous-ensemble filtré à envoyer en CSV | refus qui nomme les formats de `data.convert` | — |
| 3D partielle | vue 3D d'une partie d'une couche | refus | — |
| Barres | essais GPU | toutes tenues, ou résultats rendus tels quels | pas d'installation |

</frozen-after-approval>

## Code Map

- Faits vérifiés de la recette (`../donnees-test-cassini/annecy/LISEZMOI.txt`, `projet-recette-11-23c.cassini/journal.jsonl` entrée 0) : projet Annecy en **EPSG:2154**, 8 couches vecteur en 2154, **deux** en 4326 (parcelles GeoJSON, gares CSV) ; le modèle a planifié `analysis.buffer` sur les gares sans aucun `data.reproject` (échec `analysis.crs_not_metric`). Ce n'est pas une mauvaise cible : l'étape manque. L'inventaire rend la source vecteur par son nom de fichier (`qgis-plugin/cassini_qgis/inventory.py`), donc `gares-74-4326.csv`.
- `training/src/families.rs` `reprojection_avant_analyse` (l. 5386, poids 4) et `metric_analysis` (l. 5434) -- une seule couche en 4326, toujours celle de l'analyse ; `analyse_couche_metrique` (l. 5419, poids 4) pose à côté une couche en 4326 non citée et ne reprojette rien. Écarts avec Annecy à combler : (1) une seconde couche en 4326 non citée dans `reprojection_avant_analyse` (la couche analysée reste reprojetée) ; (2) source `.csv` pour une couche ponctuelle en 4326 — `extension()` (`training/src/inventories.rs` l. 1485) ne rend jamais `csv` ; (3) inventaires plus longs (Annecy : ~10 couches ; `Builder::finish` l. 1850 n'ajoute que 1 à 3 distractions). Relever le poids de `reprojection_avant_analyse` au-dessus de celui d'`analyse_couche_metrique`.
- Même famille, cas « toutes les couches en 4326 » (`_ => (4326, 2154)`) : le projet y est rendu en 4326 (`Builder::finish`, `epsg: vector_epsg`), ce qu'aucun projet réel ne fait ; poser un `project.epsg` métrique (2154 ou le fuseau UTM) et viser ce système. Champ à ajouter au `Builder` (`training/src/inventories.rs` l. 1515).
- `training/src/families.rs` `conversion_format` -- déjà là (10.9), rien à ajouter sauf poids.
- `training/src/families.rs` `refus` (~l. 3092, 14 branches à parts égales) -- 3D (branche par défaut, une phrase fixe) : y ajouter une couche ou un sous-ensemble nommé ; tableur (branches 5, 6) : ajouter sous-ensemble filtré, destinataire ou but ; modification d'attributs (branche 12, une tournure) : varier ; suppression (branche 4).
- `training/src/families.rs` `refus_hors_registre` (~l. 3177) -- PDF qui nomme ses couches (`NO_LAYER_CHOICE`, ~l. 3201, rare) : plus de poids, avec format, orientation et titre ; statistique d'altitude par zone : plus de poids.
- `training/src/families.rs` `style_degrade` -- rampe nommée plus souvent présente avec la méthode.
- `training/src/families.rs` `conversion_coordonnees` -- point UTM vers la zone voisine : forme `to: ["utm"]` + `utm_zone`.
- Familles de sélection par attribut sur le PLU -- catégorie de zone dite en langage courant (agricole, naturelle, urbaine, à urbaniser) vers `typezone` égal ou commençant par le code ; enchaînements à deux sélections puis tampon puis intersection, nombre d'étapes exact.
- `docs/entrainement-du-modele.md` -- tableau des familles et barres de cet essai.
- `training/entrainement/pipeline.py` -- chaîne accélérée (Unsloth, segments extensibles) ; script d'essais GPU `C:\Users\kevin\AppData\Local\Temp\essais-11-23.ps1` (pointer vers le nouveau dossier de modèle et un nouveau journal).
- `ai/tests/tenu_a_l_ecart/sentences.json` H36 -- attente corrigée en 59a0664 : rescorer.

## Tasks & Acceptance

**Execution:**
- [x] Générateur : corrections ci-dessus, tests du générateur (dont `recouvrement.rs`) verts, `cargo test --workspace` vert ; commit sur `story/re-entrainement-apres-11-23`.
- [x] Worktree de ce commit, entraînement accéléré (base Qwen3.5-4B, même commande qu'en 11.23c), GGUF dans un nouveau dossier `ajuste-apres-11-23`.
- [x] Essais GPU contre toutes les barres ; phrase des gares rejouée dans QGIS sur Annecy (copie archivée) avec le nouveau modèle chargé à part, sans l'installer.
- [x] Rendu au propriétaire : scores par barre, phrases encore ratées (par constat), durée ; la décision d'installer lui revient.

**Acceptance Criteria:**
- Given le nouveau modèle et le projet Annecy, when on demande une zone tampon autour des gares, then le plan commence par `data.reproject` des gares vers EPSG:2154, suivi du tampon sur `step:1.layer_id`.
- Given les essais GPU, when ils finissent, then chaque barre est lue et rapportée, tenue ou non.

## Implementation Notes

- 2026-11-09, générateur : `inventories.rs` (CSV pour une couche ponctuelle une fois sur huit, `as_csv`, `long_inventory`, `project_system`, `distract_in_degrees` en CSV une fois sur trois) ; `families.rs` : `reprojection_avant_analyse` poids 6 (seconde couche en 4326 non citée 2/3, couche analysée en CSV 1/2, inventaire long 1/2, cas « tout en 4326 » vers le système du projet), `analyse_couche_metrique` (CSV, inventaires longs), `conversion_format` 5, `refus` 8 (3D partielle, tableur filtré avec but ou destinataire, modification et suppression variées), `refus_hors_registre` 9 (PDF qui choisit ses couches 1/6 avec titre parfois, altitude par zone 1/3), `style_degrade` (dégradé avec la méthode 1/2, ordre libre), `conversion_coordonnees` (fuseau voisin 1/2, quatre écritures du fuseau), `zone_category` (zones du PLU en mots), `draft_multi` (deux sélections ×2). Recouvrement : tête « Exporte la carte » → « Exporte-moi la carte » (rendait F01). Tests : cinq nouveaux ou étendus (cas d'Annecy entier, projet en mètres, refus 3D et tableur, zones en mots, dégradé). `cargo test --workspace`, fmt, clippy verts (Windows).

- 2026-11-09, entraînement : commit `84adb16`, worktree `~/cassini-worktrees/apres-11-23`, chaîne accélérée (Unsloth, segments extensibles), lancé à 20 h 10, fini à 22 h 06 (116 min, 12 s/pas tenus, pic 5,70 Gio, carte jamais pleine), 0 exemple trop long. Sortie `~/cassini-entrainement/ajuste-apres-11-23/`, modèle copié dans `C:\Users\kevin\cassini-modeles\ajuste-apres-11-23\` (SHA-256 79a80220a6eb6213…), **non installé**. Validation (GGUF) : 292/300 (288 pour 11.23c).
- Essais GPU (`~/cassini-entrainement/11-23-mesures/essais-gpu-apres-11-23.log`) : `plan_reel` 24/25 (S24) ; `question_reel` 11/12 (Q11) ; **`ecart_reel` 31/40, barre ratée d'une phrase** (plans 18/27, questions 8/8, refus 5/5 ; H36 rescoré, juste ; ratés H08 et H21 question au lieu d'un plan, H10 `dissolve`, H13 `label_field`, H17 H18 H19 une étape en moins, H20 `overlay_layer_id`, H25 `value`) ; `echelle_utm_reel` 8/8 ; `terrain_reel` 8/8 ; `epic11_reel` 27/28 (M02, chemin de moindre coût 1/2, barre tenue), F03 en question, points d'un autre système 3/4 hors barre (Y03) ; `style_reel` 9/10 (ST07). Toutes les barres sont tenues sauf `ecart_reel`. Face à 11.23c : refus 3/6 → 5/5 (H39, H40 corrigés) ; nouveaux ratés de plan H10, H13, H21, H25 ; H17, H18, H19 toujours une étape en moins.
- Recette QGIS, Annecy (copie `projet-recette-apres-11-23.qgz`, nouveau modèle par `CASSINI_MODELS`, vérifié par `model.list`) : « Trouve les parcelles à moins de 500 m d'une gare » → `data.reproject` des gares vers 2154, tampon sur `step:1.layer_id`, intersection (critère d'acceptation tenu) ; « tampon de 300 m autour des gares » → reprojection puis tampon ; « Exporte les gares en shapefile » → `data.convert` ; 3/3 exécutés, rejeu identique. Journal : `~/cassini-entrainement/11-23-mesures/recette-annecy-apres-11-23.txt`.
- Décision d'installer au propriétaire : `ajuste-11-23c` reste installé.

## Verification

**Commands:**
- Rust sous Windows : `powershell.exe -NoProfile -Command '$env:LIBCLANG_PATH="C:\Users\kevin\Downloads\Projet code\outils-cassini\libclang\clang\native"; $env:CARGO_TARGET_DIR="C:\ct"; Set-Location "C:\Users\kevin\Downloads\Projet code\CASSINI"; cargo test --workspace'` -- expected: vert
- Essais GPU : script `essais-11-23.ps1` adapté, `--features cuda` -- expected: barres lues

## Review Triage Log

| # | Constat | Verdict | Suite | Preuve |
|---|---|---|---|---|
| 1 | `draft_multi` ne produit jamais `select(cible)`, `select(réf)`, `buffer(step:2)`, `intersect(step:1, step:3)` | medium | patch | `filtered_reference(` n'est appelé qu'en l. 1076 et 1411, toujours avec une zone ou une cible non filtrée ; les bras doublés sont des plans à cinq étapes |
| 2 | Catégorie de zone dite en mots seulement en condition d'une autre cible, jamais en sélection de la couche de zonage elle-même | medium | patch | `zone_category` n'est atteint que par `area()` (l. 1203) |
| 3 | `zone_category` garde `libelle` une fois sur deux, plan sur le code alors que la valeur littérale est présente | medium | patch | `b.add(..., &["zone"], &[], ...)` : `forbid` vide, `Builder::add` l. 1666 tire chaque champ non requis à 1/2 |
| 4 | `subset_phrase` tire champ et valeur du thème, pas de la couche | false | rejeté | la demande (3D, tableur, suppression, modification) est refusée pour la capacité, quel que soit le champ : un champ absent ne change pas l'attendu |
| 5 | Barre `ecart_reel` ratée (31/40) | false | rejeté | résultat rendu tel quel comme le veut la matrice ; décision du propriétaire (2026-11-09) : ne pas installer |
| 6 | Cible d'intersection en 4326 citée, à côté de couches métriques, jamais tirée | low | rejeté | le cas réel est tenu en QGIS ; l'ajouter demanderait une nouvelle branche de famille, pas une correction directe |
| 7 | Test `conversion_coordonnees` passé d'une égalité à une implication | low | patch | correction directe de l'assertion |
| 8 | Tableau des familles de `docs/entrainement-du-modele.md` non mis à jour | low | patch | lignes `analyse_couche_metrique`, `conversion_coordonnees` inchangées |
| 9 | Intention figée : « projet en UTM » contre 2154 partout ailleurs | false | rejeté | correctif = modifier le plan ; signalé au propriétaire, seul à pouvoir changer le bloc figé |
| 10 | « quatre écritures du fuseau » alors que `convert_words` en a six | low | patch | regroupé avec 8 (même correction de la doc) |
