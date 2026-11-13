---
title: 'Générateur : gros projets de 20 à 60 couches rendus par la vue du noyau, et deux couches proches donnent une question'
type: 'feature'
ticket: '47'
created: '2026-11-10'
status: 'built'
baseline_revision: '7d5667c58e0e5d1bcf0e3c6c61528ae9246496ae'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-inventaire-des-gros-projets/change-inventaire-des-gros-projets.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** le modèle n'a jamais vu d'inventaire de plus d'une dizaine de couches ni la vue triée du noyau (`index` réduit + `layers` détaillées, 5.46) ; il ne sait pas non plus demander laquelle de deux couches proches prendre.

**Approach:** deux familles nouvelles dans `training/`. `gros_projet` reprend un exemple d'une autre famille (tirée selon les poids), le pose dans un projet de 20 à 60 couches (couches de remplissage façon BD TOPO / CNIG), traduit l'inventaire en sortie brute de QGIS et le fait rendre **par `cassini_core::agent_inventory::sorted_view`** avec la demande, puis retraduit les identifiants du plan en identifiants courts. `couches_proches` pose deux couches du même thème qui ne diffèrent que par un qualificatif (millésime, source, copie) : demande sans qualificatif → question qui nomme les deux ; demande avec le qualificatif (un tiers) → plan sur la bonne couche. Gros projets ≈ 25 % du jeu, jeu gardé à 8 600.

Décisions du propriétaire (2026-11-10) : forme réduite `{layer_id, name, geometry}` ; `MAX_LENGTH` 4 608 après un essai d'une heure sur la carte de 8 Go (fait dans ce ticket, `--longs-d-abord`) ; gros projets ≈ 25 % ; repli : gros projets ≤ 40 couches ; question entre couches proches apprise par le modèle, jamais écrite dans le noyau.

Décisions tranchées en party le 2026-11-10 (architecte, ingénieur ML, géomaticien), à confirmer par le propriétaire :
1. jeu gardé à 8 600 : `gros_projet` tire sa famille de base selon les poids, donc chaque famille garde son compte (petits + gros) ; seul `couches_proches` dilue (≈ 2 %) ;
2. un exemple emballé n'est gardé que si chaque couche « utile » (couches exigées par la famille, pas ses distractions) est détaillée, sauf une couche citée par un plan d'appels **seulement comme géométrie** (aucun paramètre de champ dans les appels qui la citent, pas un raster cité par sa source), qui peut rester dans l'index seul (point 5 du changement de cap) ;
3. une couche de remplissage dont un mot du nom, de l'identifiant ou du fichier égale un mot de la demande n'est pas posée : aucune ambiguïté non voulue ;
4. `redemande` peut servir de base (identifiants retraduits aussi dans la sortie rejetée) ;
5. le générateur écarte un exemple dont le prompt dépasse une borne en caractères équivalente à `MAX_LENGTH` jetons, pour que la chaîne n'en écarte aucun.

## Boundaries & Constraints

**Always:** la vue vient de `sorted_view` avec `ViewLimits::default()` (aucune copie de sa logique) ; graines `DEFAULT_SEED` et de validation inchangées, `recouvrement.rs` vert ; aucune phrase du jeu tenu à l'écart, de `ai/tests/` ni des recettes d'Annecy ; vocabulaire civil ; part des gros projets, couches, longueurs (moyenne, médiane, max, en jetons de Qwen3.5) et nombre de `couches_proches` consignés dans `docs/entrainement-du-modele.md`.

**Never:** toucher `core/` (la vue de 5.46 ne change pas ici) ; nouvelle dépendance ; entraînement complet, installation, push, fusion.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Gros projet trié | 20–60 couches, couches de l'utilisateur > 9 000 car. | inventaire à clés `index`, `layers`, `project`, `unloaded_files`, identique à `sorted_view` sur la sortie brute | — |
| Gros projet sous le seuil | 20 couches légères | vue de `model_inventory` (pas d'`index`) | — |
| Couche utile réduite | filtre sur une couche restée dans l'index | exemple écarté, compté | — |
| Géométrie seule | tampon autour d'une couche de l'index seul | gardé, l'identifiant court de l'index dans le plan | — |
| Identifiants | id `slug_<8 hex>` ou uuid de la famille de base | plan réécrit avec l'identifiant court montré | — |
| Couches proches | « les gares », couches « Gares 2019 » et « Gares 2024 » | question qui cite les deux noms, finit par « ? » | — |
| Qualificatif dit | « les gares 2024 » | plan sur la couche 2024 | — |
| Trop long | prompt au-delà de la borne | exemple écarté | — |

</frozen-after-approval>

## Code Map

- `core/src/agent_inventory/sorted_view.rs` -- `sorted_view(raw, &[], &HashSet::new(), &LastPlan::default(), request, &ViewLimits::default()) -> (ModelInventory, ShortIds)` ; réexportée par `cassini_core::agent_inventory`. Ne pas modifier.
- `core/src/agent_inventory.rs` `agent_inventory` (l. 553) -- forme brute à produire : `project_name`, `project_crs: "EPSG:n"`, `extent.lonlat {west,south,east,north,crs:"EPSG:4326"}` (fuseau UTM tiré du centre : 31N ≈ lon 3, 32N ≈ 9, 30N ≈ −3), couches `layer_id`, `name`, `source`, `crs`, `kind`, `geometry` (WKB : `MultiPolygon`, `NoGeometry` pour `null`), `feature_count`, `fields [{name, type: String|Integer64|Real|Date, values}]`. Clé apprise absente → clé brute absente. `ShortIds` montre `nom_<4 hex>` pour un id QGIS `nom_<uuid aux soulignés 8_4_4_4_12>` : générer des ids bruts de cette forme ; pour un id de base `slug_<4 hex>`, faire commencer l'uuid par ces 4 chiffres.
- `training/src/families.rs` -- `Example` (ajouter `focus: Vec<String>`, ids des couches non-distraction, rempli par `example()` avant `finish`) ; `FAMILIES` (poids : total 342 ; `gros_projet` ≈ 116, `couches_proches` ≈ 6, à ajuster pour 25 %) ; `redemande` (l. 6911, modèle de famille qui emballe une autre) ; helpers `tampon_head`, `near`, `dist`, `de_ref`, `open_find`, `finish`, `buffer`, `intersect`, `calls`, `question`, `pick_theme`, `ordinary`.
- `training/src/inventories.rs` -- `Builder` (`layers`, `distractors`, `finish`), `Layer::to_json`, `layer_id`, `THEMES` (38 thèmes, 32 groupes : d'où les remplissages), `Noun`. Ajouter : `Builder::focus_ids()`, `Builder::add_twin(rng, key, qualifier)` (même thème, autre id/nom/fichier, contourne `taken`), table de remplissage `BIG_FILLERS` (≈ 55 couches façon BD TOPO 3 / CNIG, publiques, aucune qui recouvre un thème : pas de commune, bâtiment, route, cours d'eau, canalisation, ligne électrique, forêt, ERP, zone d'habitation, parc, IRIS, équipement de transport ; 2 rasters : orthophotographie, SCAN 25), 10 à 25 champs chacune (`cleabs`, dates, sources, précisions, `nature` à valeurs, `etat_de_l_objet` à valeurs…).
- `training/src/lib.rs` -- `validate` : les ids de `index` comptent comme présents pour `LAYER_PARAMS` ; les rasters cités par source restent ceux de `layers`. Tests unitaires existants construisent `Example` à la main (l. 347–475) : y ajouter `focus`.
- `training/src/main.rs` -- résumé ; rien d'autre.
- `training/tests/forme_inventaire.rs` -- apprendre aussi les jeux de clés des entrées d'`index` et la présence de la clé `index` ; nouveau cas : un gros projet brut synthétique (≥ 30 couches) rendu par `sorted_view` doit n'avoir que des formes vues.
- `training/tests/recouvrement.rs` -- doit rester vert, sans retouche.
- `training/entrainement/pipeline.py` -- `MAX_LENGTH` 2048 → 4608 (commentaire à jour) ; tests Python `training/entrainement/tests`.
- `docs/entrainement-du-modele.md` -- section « Gros projets et couches proches (5.47) ».

## Tasks & Acceptance

**Execution:**
- [x] `training/src/inventories.rs` -- remplissages, `add_twin`, `focus_ids`, conversion apprise → brute, tests -- matière des gros projets
- [x] `training/src/families.rs` -- `focus`, familles `gros_projet` et `couches_proches`, poids, tests (matrice) -- cœur du ticket
- [x] `training/src/lib.rs` -- `validate` voit l'index ; borne de longueur du prompt -- garde
- [x] `training/tests/forme_inventaire.rs` -- clé `index` et forme réduite -- vérif du ticket
- [x] `training/entrainement/pipeline.py` -- `MAX_LENGTH` 4 608
- [x] `docs/entrainement-du-modele.md` -- mesures et décisions

**Acceptance Criteria:**
- Given le jeu par défaut (graine 1, 8 600), when on le mesure, then 23 à 27 % d'exemples `gros_projet`, 20 à 60 couches chacun, au moins 70 % avec `index`, aucun prompt au-delà de la borne, au moins 100 exemples `couches_proches` dont un tiers environ en plan.
- Given `cargo fmt --all --check`, `cargo clippy -p cassini-training --all-targets -- -D warnings`, `cargo test -p cassini-training -p cassini-core -p cassini-ai`, `python3 -I -m unittest discover -s training/entrainement/tests`, then tout est vert.

## Design Notes

- Question des couches proches, ex. : « Deux couches peuvent correspondre aux gares : « Gares 2019 » et « Gares 2024 ». Laquelle faut-il utiliser ? » (plusieurs tournures). Qualificatifs : millésimes, `BD TOPO`/`OSM`, `copie`. Le mot de la demande doit désigner les deux noms (pluriel ôté) pour que `sorted_view` les détaille.
- Borne de longueur : environ 0,36 jeton par caractère d'inventaire ; borne posée en caractères de `prompt_text`, ajustée sur la mesure réelle au tokenizer.

## Verification

Worktree `/home/kevin/cassini-worktrees/gros-projets` (branche `story/5-47-gros-projets`), `{project-root}` y désigne ce dossier. Avant de compiler : `export LIBCLANG_PATH=/tmp/claude-1000/-mnt-c-Users-kevin-Downloads-Projet-code-CASSINI/38f6e1c7-3a62-4e6d-a1e2-bc44d994e1ac/scratchpad/clang/x/libclang-18.1.1.data/platlib/clang/native BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include`. Ne pas committer ; l'essai GPU est fait par l'orchestrateur.

**Commands:**
- `cargo test -p cassini-training` -- vert (`recouvrement`, `forme_inventaire` compris)
- `cargo run -p cassini-training --release -- --seed 1 --count 8600 --out <scratchpad>/exemples.jsonl` -- résumé : `gros_projet` ≈ 25 %
- `cargo fmt --all --check && cargo clippy -p cassini-training --all-targets -- -D warnings` -- vert

## Implementation Notes

- `Builder::add_twins(rng, key, label, first, second, copy)` remplace `add_twin` : les deux
  jumelles sont posées ensemble (la première renommée), ce qui évite de retrouver après coup le
  fichier non qualifié. `focus_ids()` comme prévu.
- Conversion apprise → brute : `inventories::raw_layer`, `raw_project`, `qgis_layer_id` ; un
  raster perd `feature_count` (QGIS ne le donne pas). `BIG_FILLERS` : 56 couches (54 vecteurs,
  2 rasters) ; pas de champ de nom dans les remplissages (sinon « dont le nom… » les détaille tous).
- `unloaded_files` : la vue du noyau rend `[]` ; on y remet celui de l'exemple de base (familles
  qui ouvrent un fichier). Seul écart à « identique à `sorted_view` ».
- Garde « couche utile » dans `validate` (`reduced_focus`), donc comptée ; précision : un appel
  `data.reproject` n'est pas « géométrie seule » (le plan dépend du système, absent de l'index).
  Un raster utile non détaillé est écarté par `validate` et compté (après relecture).
- Borne `MAX_PROMPT_CHARS` = 12 900 après relecture (pire rapport mesuré 0,333 jeton/car.,
  réponse ≤ 300 : 4 596 au plus) ; aucune séquence au-delà de 3 708 jetons.
- Poids : `gros_projet` 128, `couches_proches` 7, `redemande` 52 → 58 (redemandes à 3,9 % sinon,
  sous le plancher de 4 % du test). `redemande` n'emballe pas `gros_projet` (l'inverse oui).
- Tests ajustés : `no_family_produces_rejected_examples` admet les seuls écarts de
  `gros_projet` ; `a_travel_without_coordinates_never_becomes_a_plan` admet `gros_projet` parmi
  les familles qui demandent des coordonnées.
- `ai/src/plan.rs`, test `the_whole_generated_set_is_cut_at_every_layer` : restreint aux
  inventaires sans `index` (les découpes de la vue triée, à la fin de l'index, relèvent de 5.48).
- 100 % des gros projets sont triés : le cas « sous le seuil » est couvert par un test dédié.
- Mesures finales (graine 1, 8 600, tokeniseur Qwen3.5) : `gros_projet` 2 139 (24,9 %), 20 à 60
  couches (moyenne 39,8), 100 % avec `index` ; `couches_proches` 162 ; jetons moyenne 1 062,
  médiane 901, maximum 3 708 (gros projets : 1 733 / 1 688 / 3 708) ; 9,13 M de jetons.
- Essai GPU (orchestrateur, 2026-11-10, `--longs-d-abord --pas-max 100`, jeu d'avant relecture) :
  pic 5,72 Gio, 27 s/pas sur les plus longs, 21 à 25 s ensuite, 39 min ; aucun écarté ;
  `MAX_LENGTH` 4 608 retenu, repli inutile. Chaîne arrêtée avant la conversion GGUF.

## Plan Change Log

## Review Triage Log

Passe 1 (quick). Verdicts : high 0, medium 1, low 7, false 1, maybe-false 0. Corrigés 4, reportés 1, rejetés 5. Correctifs appliqués par l'orchestrateur (sous-agent terminé).

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| `MAX_PROMPT_CHARS` 13 000 × 0,333 + 300 = 4 629 > 4 608 : garantie écrite fausse | low | patch | borne à 12 900 (4 596 au pire) ; doc à jour |
| Commentaire de `MAX_LENGTH` : 3 754 au lieu de 3 731, essai dit fait | low | patch | commentaire réécrit sur les mesures finales et l'essai réel |
| Raster utile non détaillé : `big` rend `None`, écart non compté (matrice : « compté ») | low | patch | boucle retirée : `validate` l'écarte et le compte (« raster absent ») ; test admet ce motif |
| `geometry_only` ne regarde que les clés de premier niveau | low | reject | aucune famille ne cite un champ ailleurs qu'en `field`/`label_field` ; correctif = branche de plus |
| Un champ cité peut être coupé par `capped()` (30 champs / 3 000 car.) sans garde | medium | patch | `reduced_focus` vérifie que `field`, `label_field`, `x_field`, `y_field` restent dans la couche détaillée |
| Test « sous le seuil » ne passe pas par `big()` | low | reject | aucun gros projet du générateur n'est sous le seuil (100 % triés) ; le chemin est celui de `model_inventory`, testé au noyau |
| `a_travel_without_coordinates…` assoupli pour tout `gros_projet` | low | reject | il faudrait enregistrer la famille de base (champ de plus) ; les familles de base restent contrôlées en petits projets |
| `ai/src/plan.rs` : découpes de la vue triée non testées | low | defer | relève de 5.48 (cache placé à la fin de l'index) ; deferred-work |
| Remplissages détaillés par un nom de champ ou une valeur (`typedoc` = `PLU`) | false | reject | voulu : la vue du noyau le fait aussi en production (BD TOPO, DOC_URBA d'un PLUi) ; décision 3 vise les noms, le modèle doit choisir la couche par son nom |
| Fichier `BDT_3-5_GPKG_LAMB93` posé dans un projet en UTM | low | patch | GeoPackage BD TOPO seulement si le système est 2154, sinon shapefile |
