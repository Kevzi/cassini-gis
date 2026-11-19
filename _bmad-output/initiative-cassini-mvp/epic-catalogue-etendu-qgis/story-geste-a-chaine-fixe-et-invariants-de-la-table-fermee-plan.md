---
title: 'Geste à chaîne fixe et invariants de la table fermée'
type: 'feature'
ticket: '3'
created: '2026-11-19'
status: 'built'
baseline_revision: '616cef8344072a3899eeb8f9a2374ac764be0825'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'thorough'
review_source: 'pinned'
lenses_ran: [blind-hunter, edge-case-hunter, verification-gap, intent-alignment]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problème :** une ligne de la table `ALGORITHMS` est un algorithme unique plus des cas particuliers (`then`, `work_crs`) codés dans le geste ; elle n'a ni version de chaîne, ni annulabilité déclarée, et rien n'interdit qu'une ligne cite un algorithme réseau (AD-24, AD-14).

**Approche :** toute ligne devient une chaîne fixe d'étapes (garde, algorithme, temporaires non journalisés), portant `chain_version` ; l'annulabilité se déduit de la chaîne et est confrontée au registre ; la recette refuse la liste noire réseau.

## Boundaries & Constraints

**Always :** seule la dernière étape écrit `out` dicté par le noyau (AD-10) ; les étapes intermédiaires sont des temporaires jamais nommés ni journalisés ; aucune étape ne choisit entre deux suites, chaque valeur vient des paramètres du noyau ; `work_epsg` rempli ⇒ étape `native:reprojectlayer` d'abord, absent ⇒ algorithme seul ; une commande est annulable seulement si chaque algorithme de sa chaîne l'est ; `chain_version` initiale = 1 pour toutes les lignes (les ajouts `then` et `work_crs` sont antérieurs) ; le protocole `start/cancel/done`, les codes d'erreur et messages actuels, `register_all(registry, table, backend_factory)`, les ids annoncés à `session.hello` et le comportement de `data.reproject` ne changent pas ; aucun import `qgis` au chargement du module.

**Never :** pas de nouvelle commande (donc pas de `summary.py`, `texts.py`, `families.rs`) ; pas de genres `raster`, `layout` ni sortie `table` nouveaux (livrés par la première primitive qui en a besoin) ; pas de changement des valeurs `cancellable` Rust ; l'annonce de `chain_version` à `session.hello` et au journal est l'entrée 4, ici on la stocke seulement ; les gestes écrits à la main autres que `layer.remove` (`layer.add`, `layer.set_style`, `layer.arrange`, `data.transform_point`, etc.) ne migrent pas ici.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Erreur |
|---|---|---|---|
| Chaîne de deux algorithmes | ligne de test à 2 étapes | seule la sortie nommée par le noyau est écrite, la 1re est temporaire | aucune |
| `work_epsg` absent / rempli | `analysis.buffer` | algorithme seul / `reprojectlayer` puis algorithme | `data.unknown_crs`, `analysis.crs_not_metric` comme avant |
| Garde sur intermédiaire vide | `analysis.isochrone` | erreur nommée `ORIGIN_OFF_NETWORK` | inchangée |
| Annulation en cours de chaîne | annulation à l'étape 1 | temporaires et sortie retirés, `qgis.cancelled` | aucune |
| Annulabilité divergente | ligne dont un algorithme n'est pas annulable, registre `cancellable: true` | le test échoue | message nomme la commande |
| Liste noire | ligne de test citant `native:filedownloader` | la recette échoue | message nomme la ligne et l'algorithme |
| `layer.remove` | entrée `project`, sortie `none` | ligne de geste fixe avec `chain_version`, non annulable | — |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/analysis.py:337-476` -- `ALGORITHMS` et sa légende : migrer chaque ligne (`algorithm`/`then`/`work_crs` → `chain`, `chain_version`) ; `ProcessingGesture` (712, `_begin` 759, `_on_finished` 930, `_clean` 1025) : piloter l'exécution par la liste d'étapes au lieu des états `_stage` 0/`_WORK_CRS_STAGE`/1 ; `_TEMPORARY` (195), `_remove` (205) à réutiliser.
- `qgis-plugin/cassini_qgis/layer_remove.py` -- ligne de geste fixe (entrée `project`, sortie `none`, `chain_version`) à déclarer sans toucher au comportement idempotent.
- `qgis-plugin/cassini_qgis/delegated.py` -- `build_registry` : à ne pas casser (ids annoncés).
- `qgis-plugin/tests/test_delegated.py` -- `ClosedTableTest` : la table reste égale aux commandes `runs_in: qgis` du catalogue gelé.
- `qgis-plugin/tests/test_analysis.py` -- double fond de test, modèle des tests de chaîne ; ne pas modifier les tests existants.
- `core/tests/catalogue.rs` -- modèle de la confrontation côté Rust ; `core/src/registry.rs:59` champ `cancellable`.
- `qgis-plugin/tests/probe_reproject.py`, `probe_cancel.py` -- sondes sous vrai QGIS, modèle de `probe_chain.py`.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- forme `chain` (étapes : algorithme, traduction, garde, condition « paramètre rempli »), `chain_version`, `CANCELLABLE_ALGORITHMS`, `NETWORK_BLACKLIST` (`native:filedownloader`, `httprequest`, `downloadvectortiles`, `openurl`, `batchnominatimgeocoder`), fonction `validate_table` ; exécuteur par étapes -- AD-24
- [ ] `qgis-plugin/cassini_qgis/analysis.py` -- migrer les 9 lignes, dont `data.reproject`, en gardant `translate`, `summary` et gardes -- migration sans changement de comportement
- [ ] `qgis-plugin/cassini_qgis/layer_remove.py` -- déclarer sa ligne de geste fixe -- cohérence de la table
- [ ] `qgis-plugin/tests/test_chain.py` -- chaîne de 2 étapes n'écrit que `out` ; annulation à chaque étape ; `work_epsg` conditionnel ; liste noire (ligne `native:filedownloader` refusée par `validate_table` ET par la recette) -- vérification des invariants
- [ ] `qgis-plugin/tests/table_annulable.json` + test Python l'égalant à la table dérivée ; `core/tests/catalogue.rs` -- confronte ce fichier à `cancellable` du registre pour les commandes `runs_in: qgis` -- la confrontation table/registre
- [ ] `qgis-plugin/tests/check_extension.py` -- appeler `validate_table` dans la recette
- [ ] `qgis-plugin/tests/probe_chain.py` + README du plugin -- sonde 2 étapes sous vrai QGIS, à lancer sur la série 3 et sur la série 4 si installée

**Acceptance Criteria:**
- Given une ligne de test à deux algorithmes, when le geste s'exécute, then seul le chemin `out` du noyau existe et aucun temporaire n'est journalisé.
- Given toutes les lignes migrées, when `test_analysis.py` et `test_delegated.py` tournent, then ils passent sans modification.
- Given une ligne dont un algorithme n'est pas annulable, when le registre la déclare annulable, then le test Rust échoue en nommant la commande.
- Given une ligne qui nomme `native:filedownloader`, when la recette tourne, then elle échoue.

## Implementation Notes

## Plan Change Log

## Review Triage Log

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | patch | `validate_table` plante (KeyError, AttributeError, StopIteration) sur une ligne sans chaîne, à garde non dict, ou dont toutes les étapes sont conditionnelles, au lieu de la nommer ; une garde sur la 1re étape n'est jamais lue ; chaîne vide déduite annulable (`all([])`). Lu dans `analysis.py:618-658`. |
| low | patch | Aucun test n'exerce la branche « garde mal formée » de `validate_table` (verification-gap, vérifié dans `test_chain.py`). |
| low | patch | Deux sources pour la reprojection : `work_crs` (contrôles) et `when` de l'étape ; `validate_table` ne les rapproche pas. |
| low | patch | Liste noire comparée sans tenir compte de la casse. |
| low | patch | `check_extension.check_table` plante sur une erreur d'import de l'extension au lieu de la signaler. |
| false | — | `work_epsg` abandonné par `start` : `_check_work_crs` renvoie `value` inchangé (`analysis.py:1045`), l'étape lit la même valeur. |
| false | — | Code de l'ancienne forme de ligne « sans usage » : `test_analysis.py` construit encore des lignes à l'ancienne forme, la lecture est exercée. |
| low | rejeté | Confrontation Rust à sens unique : le sens inverse est couvert par `ClosedTableTest` (table = commandes `runs_in: qgis` du catalogue) ; l'ajout demanderait de distinguer les gestes écrits à la main. |
| low | rejeté | Traduction de la 1re étape faite deux fois, et étapes suivantes validées en cours de chaîne : comportement antérieur inchangé (pré-existant). |
| low | rejeté | Liste noire fermée (et non liste blanche), `len == 5` figé, mot coupé `ht""tprequest` : voulu par AD-24 (liste nommée) et la garde de source du projet. |
| low | rejeté | `chain_version` jamais confrontée à un changement réel ni envoyée au journal : annonce = entrée 4, hors de l'intention. |
| low | rejeté | Sonde annulation à 200 ms, message « reprojection n'a rien rendu » non testé, `table.len() >= 10`, clé `gesture` : cosmétique ou peu probable, correction complexe. |
| — | écarté | Intent Alignment : lecture R1 (recette locale) implémentée, conforme à « la recette refuse » ; pas de contrôle à l'exécution demandé par l'intention. |

## Design Notes

Décisions prises au mieux du projet (questions ouvertes de l'entrée) : `chain_version` est un entier dans la ligne ; la fiche du catalogue n'a pas de champ d'annulabilité, la confrontation passe donc par un fichier d'or `table_annulable.json` lu des deux côtés (Python l'égale à la table, Rust l'égale au registre), même transitivité que le catalogue ; « les deux séries » = tests à fond double en CI (sans QGIS) plus sonde réelle documentée par série, car la CI n'a pas QGIS.

```python
"analysis.isochrone": {"chain_version": 1, "chain": [
  {"algorithm": "native:serviceareafrompoint", "output": "OUTPUT_LINES"},
  {"algorithm": "native:convexhull", "guard": {"empty": ORIGIN_OFF_NETWORK}}]}
```

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: OK
- `python -I qgis-plugin/tests/check_extension.py` -- expected: OK
- `cargo test -p cassini-core --test catalogue` -- expected: OK
- `cargo fmt --check && cargo clippy` -- expected: sans avertissement

**Manual checks (if no CLI):**
- `probe_chain.py` sous QGIS 3.44 LTR (et série 4 si disponible) : un seul fichier de sortie.
