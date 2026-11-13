---
title: 'Tampon sur une couche en degrés : calcul dans un système en mètres choisi par le noyau'
type: 'feature'
ticket: '16'
created: '2026-11-12'
status: 'built'
baseline_revision: '2a6d064b86100c5cf7c2169e5ed726b78051fbeb'
route: 'full'
route_source: 'pinned'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/change-distance-en-metres-sur-couche-en-degres/change-distance-en-metres-sur-couche-en-degres.md'
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La phrase de démo d'Annecy échoue à l'étape `analysis.buffer` (`analysis.crs_not_metric`) : la couche « gares » est en EPSG:4326 et aucun modèle ne compose `data.reproject` avant le tampon.

**Approach:** Le noyau remplit, au moment de dicter l'ordre, un paramètre caché `work_epsg` (comme `out`), choisi par une fonction pure ; le geste de l'extension fait toujours la chaîne fixe `native:reprojectlayer` (temporaire) puis `native:buffer`. Aucune étape ajoutée au plan (AD-23) ; `work_epsg` journalisé et relu au rejeu (AD-11).

## Boundaries & Constraints

**Always:** règle de choix du §3.2 du changement de cap, dans cet ordre : couche `m` → son système ; couche `deg` → système du projet s'il est `m`, sinon `project.utm_epsg` (même `utm_epsg()` que l'inventaire du modèle) ; sinon `analysis.crs_not_metric` avant tout ordre, sans ligne au journal, message qui nomme la couche, son système et la raison ; unité inconnue (`other`, absente) → système de la couche. Le noyau ôte toute valeur de `work_epsg` donnée par l'appelant. Données d'origine intactes (sortie temporaire de QGIS). Commentaires en français.

**Never:** GDAL, GEOS ou PROJ dans le noyau ; le geste qui choisit un système ; une étape `data.reproject` ajoutée ; `work_epsg` montré au modèle, au formulaire ou au client MCP ; recalcul de `work_epsg` au rejeu d'une ligne qui le porte ; ré-entraînement ; toucher aux autres commandes à distance (10.32 et notes de l'epic 10) ou au MNT (10.33).

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Couche métrique | couche EPSG:2154 (`m`) | `work_epsg` 2154 ; reprojection identité ; même système, même nombre d'entités, même géométrie qu'avant | — |
| Couche en degrés, projet métrique | gares 4326, projet 2154 | `work_epsg` 2154 ; résultat `crs` EPSG:2154 ; écart au tampon fait après `data.reproject` manuel ≤ quelques mètres | — |
| Projet en degrés | couche et projet 4326, étendue Annecy | `work_epsg` 32632 | — |
| Aucun système en mètres | couche et projet 4326, étendue > 12° ou absente | aucun ordre, aucune ligne | `analysis.crs_not_metric`, message clair (passer le projet en Lambert-93 ou « reprojette la couche … ») |
| Couche `step:k` | entrée = sortie d'une étape précédente du plan | unité lue dans l'inventaire relu, comme une couche de l'utilisateur | — |
| Unité inconnue | `crs_unit` `other` ou absent | `work_epsg` = système de la couche | le geste refuse par `analysis.crs_not_metric` s'il n'est pas en mètres |
| Rejeu, ligne neuve | ligne avec `work_epsg`, projet passé en 4326 depuis | même `work_epsg`, même résultat typé, même session ou session neuve | — |
| Rejeu, ligne ancienne | ligne sans `work_epsg` | complétée par le système de la couche (inventaire relu) : succès sur couche métrique, même refus sur couche en degrés | divergence nommée sinon |
| Appel direct | `command.run`, MCP relais, `cassini run --project` | même remplissage dans `call_step` ; sans QGIS : `registry.unavailable` comme aujourd'hui | — |

**Décisions du propriétaire (2026-11-12, approbation)** : (1) le texte lu par le modèle ne change pas — ni la description FR/EN de `analysis.buffer`, ni la doc de `layer_id`, ni l'attendu du prompt ; seuls la documentation interne et le catalogue changent (le modèle a été entraîné sur le texte actuel) ; (2) la version de chaîne d'AD-11 reste non construite, l'écart va dans `deferred-work.md` ; (3) le plan reste entier.

**Décision du propriétaire (2026-11-12 après-midi, après la recette RA06)** : on ne reprojette que les couches en degrés. Une couche en mètres, ou d'unité inconnue, ne reçoit pas `work_epsg` et passe par `native:buffer` seul, comme avant 9.16 ; la reprojection ne tourne que si `work_epsg` est rempli. Les lignes ci-dessus qui disent « toujours », « reprojection identité » ou « ligne ancienne complétée » sont remplacées par cette décision (voir Plan Change Log et l'amendement sous AD-24).

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_buffer.rs` -- paramètre `work_epsg: Option<u32>` (`serde(default)`), descriptions FR/EN et doc de `layer_id` INCHANGÉES (lues par le modèle, décision du propriétaire), déclaration du paramètre rempli.
- `core/src/registry.rs:60-140,245-300` -- `CommandSpec` : nouvelle métadonnée `work_crs_param: Option<(param, layer_param)>`, jamais sérialisée, sur le modèle de `mgrs_grid_param`.
- `core/src/journal/mod.rs:383-408,482-640` -- `fill_mgrs_grid` (précédent : lecture de `project.inventory` par le routeur, hors journal) ; `call_step`, entonnoir unique de toute exécution.
- `core/src/journal/mod.rs:1385-1460` -- boucle du rejeu : params de la ligne rejoués tels quels ; complétion d'une ligne ancienne.
- `core/src/agent_inventory.rs:579-626,638,649` -- `utm_epsg()`, `MAX_UTM_SPAN_DEGREES`, `epsg()` ; `layer()` en liste blanche (`crs_unit` n'atteint pas le modèle).
- `core/src/commands/project_inventory.rs` -- `InventoryLayer.crs_unit`, `project_crs_unit`.
- `core/src/contract.rs:~909` -- contrôle des paramètres déclarés (étendre à `work_crs_param`).
- `ai/src/plan.rs:280-290,615-630` -- paramètres cachés du modèle et refus d'un plan qui les écrit.
- `cli/src/mcp.rs:112-135` -- `catalogue_from_session` : schéma des outils du relais.
- `qgis-plugin/cassini_qgis/analysis.py:305-311,384-420,640-800` -- ligne `analysis.buffer`, `crs_is_metric`, `_begin`, `_on_finished`/`_chain`.
- `qgis-plugin/cassini_qgis/inventory.py:375-406` -- `ProjectInventoryGesture._read`.
- `qgis-plugin/cassini_qgis/form.py:20` -- `CORE_FILLED`.
- `qgis-plugin/cassini_qgis/summary.py:188-198`, `texts.py:105,412`, `panel.py:160-174` -- compte rendu du tampon.
- `qgis-plugin/tests/probe_demo.py:31-34` -- phrase de Villeneuve à remplacer.
- `_bmad-output/initiative-cassini-mvp/spec-cassini-mvp/catalogue-des-commandes.md:432-450,577-610` -- fiches à reporter ; `core/tests/catalogue.rs` les confronte.

## Tasks & Acceptance

**Execution:**
- [x] `core/src/work_crs.rs` (nouveau, déclaré dans `lib.rs`) -- `work_epsg(inventory, layer_id) -> Result<Option<u32>, RegistryError>` pure : couche absente → `None` (le geste répond `layer.not_found`) ; sinon règle du §3.2 ; `utm_epsg` sur `extent` de l'inventaire -- choix testable sans QGIS.
- [x] `core/src/registry.rs`, `core/src/commands/analysis_buffer.rs`, `core/src/contract.rs` -- métadonnée `.work_crs_param("work_epsg", "layer_id")` ; contrat : les deux noms existent au schéma, `work_epsg` entier facultatif -- un seul mécanisme, réutilisable par 10.32.
- [x] `core/src/journal/mod.rs` -- `call_step` : ôter la valeur de l'appelant ; après `validate_params`, lire `project.inventory` par le routeur (une `Unavailable` reste `registry.unavailable`), remplir avant `resolve_call` pour que `logged` porte `work_epsg` ; rejeu : si la ligne n'a pas `work_epsg`, relire l'inventaire et poser l'EPSG de la couche liée -- AD-11.
- [x] `core/src/commands/project_inventory.rs` -- `crs_unit` (`m`/`deg`/`other`, facultatif) par couche, `project_crs_unit` facultatif.
- [x] `ai/src/plan.rs`, `cli/src/mcp.rs`, `qgis-plugin/cassini_qgis/form.py` -- cacher `work_epsg` comme `out` : grammaire et prompt, refus d'un plan qui l'écrit, outil MCP du relais, formulaire ; l'attendu du prompt ne change pas (décision du propriétaire).
- [x] Tests noyau -- `work_crs.rs` : cinq cas + `step:k` + étendue trop large + EPSG non lisible ; `journal/tests.rs` (exécutant QGIS simulé qui répond à `project.inventory`) : `work_epsg` journalisé, valeur de l'appelant écrasée, refus sans ligne, rejeu neuf identique après changement du système du projet, ligne ancienne métrique → succès et en degrés → même erreur, sans QGIS `registry.unavailable` ; `agent_inventory` inchangé avec `crs_unit` ; MCP : l'outil n'expose pas `work_epsg`.
- [x] `qgis-plugin/cassini_qgis/analysis.py` -- clé de ligne `work_crs: "work_epsg"` : `work_epsg` absent → `analysis.bad_params` (« le noyau n'a pas donné de système de calcul ») ; inconnu → `data.unknown_crs` ; non métrique (`epsg_is_metric`, nouveau au fond) → `analysis.crs_not_metric` nommant la couche et le système ; puis `native:reprojectlayer` (`TARGET_CRS` EPSG:`work_epsg`, sortie temporaire) toujours exécuté, puis `native:buffer` sur la couche temporaire vers `out` ; une couche vide passe ; l'annulation à chaque stade nettoie -- AD-24.
- [x] `qgis-plugin/cassini_qgis/inventory.py` -- `crs_unit` par couche et `project_crs_unit` d'après `crs().mapUnits()` (SCR invalide → `other`).
- [x] `qgis-plugin/tests/test_analysis.py`, `test_inventory.py` -- `FakeBackend` : ordre de la chaîne, `TARGET_CRS`, temporaire puis `out`, trois refus, couche vide, annulation au stade 0 ; unités de l'inventaire.
- [x] `qgis-plugin/cassini_qgis/summary.py`, `texts.py`, `panel.py` -- quand le résultat a un `crs` différent de celui de la couche d'entrée lu en direct dans QGIS (comme `_live_layer_name`), ajouter « calculée en EPSG:… » (FR/EN) ; inconnu → rien.
- [x] `qgis-plugin/tests/probe_buffer_degrees.py` (nouveau, vrai QGIS, couche mémoire synthétique) -- tampon 500 m par le geste sur points 4326 vs `data.reproject` + tampon : écart de surface et `hausdorffDistance` imprimés, seuil 5 m ; couche 2154 : même SCR, même compte, géométrie égale.
- [x] `qgis-plugin/tests/probe_demo.py` -- phrase d'Annecy de CLAUDE.md §1 (et docstring).
- [x] `catalogue-des-commandes.md` -- `planned_params`→`params` (tampon), `planned_result`→`result` (inventaire), `planned_by` ôtés ; `deferred-work.md` : `resolution` de l'entrée de la sonde 11.18.

**Acceptance Criteria:**
- Given la session QGIS d'Annecy, when la phrase de démo est exécutée, then le plan a les mêmes étapes qu'avant, le tampon sort en EPSG:2154 et le PDF est produit.
- Given un journal de cette démo, when on ferme et rouvre le projet puis rejoue, then le rejeu est identique.
- Given `cargo test --workspace` et la recette Python, then tout est vert, `core/tests/catalogue.rs` compris.

## Implementation Notes

- Noyau : `core/src/work_crs.rs` (`work_epsg`, règle du §3.2 amendée : rien pour toute unité autre que `deg` ; Mercator mondiaux écartés du système du projet). `Session::fill_work_crs` dans `call_step`, après `fill_mgrs_grid` : valeur de l'appelant ôtée, `validate_params`, lecture de `project.inventory` par le routeur (une `Unavailable` est renommée vers l'id de la commande, pour garder « analysis.buffer » dans le message), remplissage avant `resolve_call`. Couche qui n'est pas en degrés, ou absente de l'inventaire : rien n'est rempli (tampon seul, ou `layer.not_found`). Une lecture d'inventaire en échec garde son code et nomme la commande. Rejeu : une ligne qui porte `work_epsg` le relit tel quel ; une ligne qui ne le porte pas se rejoue sans lui (amendement RA06).
- MCP : le schéma de l'outil du relais est ôté de `work_epsg` côté session (`serve.rs`, `without_core_filled`), là où la métadonnée `work_crs_param` est connue : le relais ne lit jamais son propre registre et la métadonnée n'est pas sérialisée. `cli/src/mcp.rs` n'a donc pas changé ; test dans `core/tests/canal_externe.rs`.
- Extension : clé `work_crs` de la table ; avec `work_epsg`, la garde porte sur lui (`epsg_is_metric`), sinon la garde `metric` d'avant s'applique (amendement RA06). La reprojection ouvre la chaîne en stade `work_crs`, puis l'algorithme de la ligne est relancé sur la couche temporaire (traduction faite une première fois sur l'entrée pour refuser un paramètre invalide avant tout lancement). Ordre des refus : sortie, couche absente, non vecteur, puis `work_epsg` inconnu / non métrique (ou, sans lui, couche non métrique).
- Faux QGIS des tests existants : la lecture de l'inventaire ne compte plus dans les ids de `SessionQgis` (un id attendu `couche_g1_3` devient `couche_g1_2`), et l'inventaire court porte `crs` ; `RemoveQgis` et les tests du canal externe répondent à cette lecture.
- Compilation de `cassini-ai` sous WSL : `LIBCLANG_PATH` vers un libclang de pip (dossier temporaire) et `BINDGEN_EXTRA_CLANG_ARGS=-I/usr/lib/gcc/x86_64-linux-gnu/13/include` ; `cassini-app` (Tauri) exclu sous WSL faute de GTK.

- Vérification réelle (2026-11-12), noyau `--features gpu` construit dans `C:\ct-916` depuis une copie Windows de la branche, extension lue depuis la branche, modèle installé seul dans `CASSINI_MODELS` (lien dur) ; rien n'a été installé sur le poste :
  - `probe_buffer_degrees.py` : OK (Hausdorff 0,000 m, écart de surface 0 ; couche métrique identique à `native:buffer` seul ; refus sur 4326).
  - `probe_demo.py` sur une copie `projet-test-annecy-9-16.qgz` : plan de 5 étapes, inchangé ; tampon en EPSG:2154 (`work_epsg` 2154 au journal) ; PDF rendu en PNG et regardé.
  - `probe_recette.py` sur `projet-test-annecy.qgz` + `annecy.json` : 19/20 (barre 16 tenue), rejeu après réouverture identique (31 étapes). Raté : RA06 (« bande de 10 m de part et d'autre des cours d'eau ») en `analysis.failed`.
- **Régression RA06 (tranchée le 2026-11-12, voir Plan Change Log).** `cours-d-eau-74.gpkg` est une couche `MultiLineString` dont 6 entités sur 1 105 sont des `GeometryCollection` (lignes et points, restes du découpage sur le 74). `native:buffer` seul les acceptait ; `native:reprojectlayer` vers un temporaire les refuse (« Could not add feature with geometry type GeometryCollection to layer of type MultiLineString ») et l'algorithme échoue, identité comprise. La précision d'AD-24 du 2026-11-12 impose une reprojection « toujours exécutée, identité comprise » : sauter la reprojection quand le système est déjà le bon serait un choix entre deux suites dans le geste, donc une décision d'architecture, non prise ici.

## Plan Change Log

- 2026-11-12, amendement RA06 (décision du propriétaire, option a ; amendement daté sous AD-24, puce « Couche en mètres » sous AD-19, §7 du changement de cap) : on ne reprojette que les couches en degrés. Cause : RA06 (tampon de 10 m sur `cours_d_eau`, en mètres) échouait en `analysis.failed`, `native:reprojectlayer` refusant 6 `GeometryCollection` que `native:buffer` seul acceptait. Changements :
  - `work_crs::work_epsg` : couche `crs_unit` = `m` → `Ok(None)`, rien n'est rempli ni journalisé (une couche en mètres sans code EPSG passe donc de nouveau, la régression notée au `deferred-work.md` disparaît) ; couche en degrés et unité inconnue : règle inchangée.
  - Rejeu : la complétion des lignes anciennes par le système de la couche est retirée (et `work_crs::layer_epsg` avec elle) ; une ligne sans `work_epsg` se rejoue sans lui (tampon seul), une ligne qui le porte le relit tel quel.
  - Extension : la ligne `analysis.buffer` retrouve sa garde `metric` ; `work_epsg` présent → garde sur lui, puis `native:reprojectlayer` et `native:buffer` ; absent → `native:buffer` seul, exactement comme avant 9.16. Le choix est dicté par le paramètre.
  - Vérification après l'amendement, même noyau reconstruit dans `C:\ct-916` : `probe_buffer_degrees.py` OK ; `probe_demo.py` : 5 étapes, tampon en EPSG:2154 (`work_epsg` 2154 au journal), PDF regardé ; `probe_recette.py` + `annecy.json` : 20/20 (RA06 repasse, 1 105 entités), rejeu après réouverture identique (31 étapes) ; `projet-gros-annecy.qgz` + `phrases-gros.json` : 6/7 (barre 6 tenue ; raté G06, `map.graduated_style` sur une autre couche de communes, sans lien avec le tampon), rejeu après réouverture identique (7 étapes), tampon du gros projet exécuté en 0,1 s lecture de l'inventaire comprise.
  - Catalogue (fiche `analysis.buffer`), `deferred-work.md` (entrée « collections de géométries sur une couche en degrés »), tests du noyau, du canal externe, de la CLI et de l'extension.

- 2026-11-12, suite de relecture : toute unité autre que `deg` (mètres, `other`, absente) → rien n'est rempli, le tampon se fait seul sous la garde du geste (une couche en mètres sans code EPSG passe comme avant 9.16) ; un projet en Mercator mondial (EPSG 3857, 3395, 900913, 102100, 102113) n'est jamais retenu comme système de calcul, le fuseau UTM de l'étendue le remplace ; le refus dit la raison d'après `project_crs_unit` ; une lecture d'inventaire en échec rend une erreur qui nomme `analysis.buffer` (code gardé), sans ordre ni ligne ; le compte rendu « calculée en EPSG:… » ne se lit plus qu'au `work_epsg` des paramètres journalisés (plus de lecture du système en direct dans QGIS) ; `out` n'est pris par le geste qu'après tous les refus.

## Review Triage Log

Passe 1 (thorough : blind-hunter, edge-case-hunter, verification-gap, intent-alignment). Verdicts : high 0, medium 4, low 10, false 0, maybe-false 0. Corrigés 11 (patch, même implémenteur), reportés 4, rejetés 1.

| Constat | Verdict | Sort | Preuve / action |
|---|---|---|---|
| Unité inconnue/absente/`other` → `work_epsg` = système de la couche, reprojection d'identité (RA06 revient avec une extension ancienne) ; IGNF:LAMB93 refusé | medium | patch | toute unité ≠ `deg` → rien rempli ; tests |
| Projet en Web Mercator retenu comme « en mètres » : tampon de 500 m ≈ 350 m au sol à 46° | medium | patch | Mercator mondiaux exclus (`GLOBAL_MERCATOR`) → UTM ; test |
| Lecture d'inventaire en échec (hors Unavailable) : l'erreur ne nomme pas le tampon ; non testé | medium | patch | erreur nommant `analysis.buffer`, code gardé ; test `broken_inventory` |
| Tests Python du tampon passés par un chemin que le noyau n'envoie plus (`work_epsg` par défaut) | medium | patch | `params()` sans `work_epsg`, chemin en degrés à part, annulation/vide/illisible couverts |
| Message de refus « projet en degrés » faux pour un projet métrique sans code | low | patch | raison tirée de `project_crs_unit` |
| « calculée en EPSG:… » lu sur le système vivant (faux pour `step:k`) ; `detail` non testé | low | patch | lu sur `work_epsg` journalisé ; test `detail` |
| `_out` posé avant la traduction : un refus pouvait effacer un fichier non écrit | low | patch | posé après ; test |
| Sonde : seuil de 5 m infalsifiable, aucun cas GeometryCollection, nombre magique | low | patch | distance ellipsoïdale indépendante (0,45 m), cas GeoPackage mixte, contrôles nommés |
| `wait_buffer_order` figé sur « gares » 4326 ; test du contrat dépendant de l'ordre | low | patch | arguments ; ensemble |
| Commentaires et docs périmés (« toujours exécutée », docstring `_unit`, entrée reportée, garde d'AD-24) | low | patch | corrigés ; AD-24 par une phrase dans le bloc amendé |
| Bloc figé décrit l'ancienne règle | low | patch | décision du propriétaire ajoutée au bloc figé (2026-11-12) |
| Annulation pendant la lecture cachée de l'inventaire → `request.not_cancellable` | low | defer | fenêtre ≈ 0,1 s ; `deferred-work.md` |
| Clients MCP lisent encore « doit être en mètres » | low | defer | texte du modèle gardé par décision ; note au catalogue |
| Couche en mètres en Web Mercator tamponnée en 3857 | low | defer | préexistant ; `deferred-work.md` |
| Fuseau UTM tiré de l'étendue du projet, pas de la couche | low | defer | `deferred-work.md` |
| Aucun test automatique avec une vraie GeometryCollection | low | reject | le QGIS simulé ne la représente pas ; prouvé par la sonde et RA06 en vrai QGIS |

## Design Notes

**Inconnu tranché (party simulée).** *Architecte :* toute exécution — `plan.execute`, `command.run` du formulaire, MCP relais (`calls.rs` → `call_from`), `cassini run --project` — passe par `Session::call_step` ; le noyau y lit lui-même `project.inventory` par le routeur, hors journal, comme `fill_mgrs_grid` (10.31) et `inventory_param` (11.20). Aucune AD n'est contredite. *Développeuse :* pas de cache d'inventaire de session — périmé pour une couche `step:k` créée pendant le plan ; une lecture fraîche couvre `step:k` sans lire le `crs` de l'étape, puisque `plan.execute` a déjà résolu la référence en `layer_id`. Sans QGIS (CLI, MCP autonome), le tampon est déjà indisponible : on garde `registry.unavailable`. Les faits par couche sont gardés par l'extension (`facts_key`) : la relecture coûte peu, à mesurer sur le gros projet. *Géomaticien :* l'unité vient de QGIS (`mapUnits`), le noyau ne déduit rien d'un code ; un système sans code `EPSG:n` ne peut pas devenir `work_epsg` (entier) : refus nommé `analysis.crs_not_metric`, régression acceptée pour ce cas rare, notée au `deferred-work.md`. **Décision : lecture fraîche dans `call_step`. Pas d'Open Question.**

**`work_epsg` au schéma, comme `out`.** `schemars(skip)` le cacherait partout mais casserait la fiche approuvée (`params`) contre `catalogue.rs` ; on le garde au schéma et on le cache aux mêmes endroits que `out`, plus l'outil du relais MCP.

**Version de chaîne.** Ni la table ni `session.hello` n'en portent aujourd'hui (resserrement d'AD-11 du 2026-10-18 jamais construit). On ne la construit pas ici, ce qui changerait le protocole : la fidélité du rejeu tient à `work_epsg` journalisé. Écart noté au `deferred-work.md` et à signaler au propriétaire.

## Verification

**Commands:**
- `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` -- expected: aucun avertissement.
- `cargo test --workspace` -- expected: vert, `core/tests/catalogue.rs` compris.
- `python -m unittest discover -s qgis-plugin/tests` -- expected: vert.

**Manual checks (if no CLI):**
- Installer le noyau et l'extension de la branche (mémoire « installer avant la recette »), modèle installé seul dans `CASSINI_MODELS`.
- `probe_buffer_degrees.py` sous `python-qgis-ltr.bat` : écart ≤ 5 m, couche métrique inchangée.
- `probe_demo.py` sur `donnees-test-cassini/annecy/projet-test-annecy.qgz` (gares en 4326), `QT_QPA_PLATFORM=windows` : plan sans étape ajoutée, tampon en EPSG:2154, PDF rendu en PNG et regardé à l'œil ; puis fermer, rouvrir, rejouer depuis le journal : identique.
- `outils-cassini\scripts\recette-5-54.ps1` sur `projet-test-annecy.qgz` avec `qgis-plugin/tests/recette/annecy.json` : 20/20 et rejeu identique ; puis sur `projet-gros-annecy.qgz` avec `phrases-gros.json` : barre tenue, durée de l'inventaire relevée. Le script part de `CASSINI-nuit` : le lancer sur une copie Windows de cette branche (ou un double du script avec le dossier du dépôt en paramètre), choix noté ici.
