---
title: 'Plus court chemin sur réseau (network.shortest_path)'
type: 'feature'
ticket: '13'
created: '2026-11-20'
status: 'built'
baseline_revision: 'a65cf26948a4b318f70a0f6824d31dcd15f12c35'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** On ne sait pas demander l'itinéraire le plus court entre deux points sur le réseau de l'utilisateur ; seule l'isochrone lit le contrat de réseau.

**Approach:** Ajouter `network.shortest_path`, déléguée à `native:shortestpathpointtopoint` (chaîne fixe, AD-24), calquée sur `analysis.isochrone` : même contrat de réseau (`NetworkOptions` : sens, vitesse, tolérance), deux points `start` et `end` (GeoJSON avec leur système, AD-8, écrits dans le plan ou repris par `step:k`), sortie en couche d'une ligne, longueur et durée rendues avec leur unité.

## Boundaries & Constraints

**Always:** Paramètres `layer_id`, `start`, `end` (`crate::point::Point`), `mode` (`walk`|`car`), `network` (facultatif, `NetworkOptions`), `out` (fourni par le noyau, `.gpkg`), `work_epsg` (rempli par le noyau, comme `analysis.isochrone`) ; contrat de réseau lu par `NetworkOptions::resolve` (défauts 5 km/h à pied, 50 en voiture, 100 m, deux sens), jamais redéfini ici ; la commande déclare `network_param` et `work_crs_param` ; stratégie « plus rapide » (la durée en dépend), longueur lue sur la géométrie dans le système en mètres `work_epsg` ; résultat `layer_id`, `name`, `feature_count` (1), `crs`, `length` (en `m`), `duration` (en `s`) ; chaîne de version 1 ; annulable ; un point à plus de la tolérance du réseau est refusé avant lancement par `analysis.origin_off_network`, dont le message nomme le départ ou l'arrivée ; deux points sur des composantes non connectées rendent l'erreur nommée `analysis.no_path`, jamais une couche vide muette ; la description FR/EN cite la distance d'accrochage (« 100 m du réseau ») ; les noms de paramètres de `QgsProcessing` et leurs sorties (`OUTPUT`, `TRAVEL_COST`) sont constatés par sonde sous QGIS 3.44, jamais supposés.

**Never:** Modifier `network_options.rs`, `Point`, `Distance`, `analysis.isochrone` ; une expression de coût écrite par le modèle ; plusieurs chemins ou points intermédiaires ; famille d'exemples pour le modèle (réentraînement du ticket 17) ; `SummaryRow`.

## I/O & Edge-Case Matrix

| Scénario | Entrée / État | Résultat attendu | Erreur |
|---|---|---|---|
| Nominal | réseau synthétique en Lambert-93, deux points sur le réseau | couche d'une ligne, le plus court chemin attendu, `length` en m et `duration` en s | aucune |
| Réseau en degrés | WGS84 | calcul dans `work_epsg`, longueur en mètres | aucune |
| Sens unique | champ de sens, parcours interdit à l'aller | chemin qui contourne ou erreur nommée si impossible | `analysis.no_path` si aucun |
| Point hors réseau | départ ou arrivée plus loin que la tolérance | refus avant lancement, aucun fichier écrit, message nommant le point | `analysis.origin_off_network` |
| Pas de chemin | deux composantes non connectées | erreur nommée | `analysis.no_path` |
| Points repris | `end` = `step:1.point` d'une étape précédente | accepté par le contrôle de plan | aucune |
| Paramètre inconnu ou manquant | `start` absent, champ inconnu | refus à la validation | `registry.invalid_params` |

</frozen-after-approval>

## Code Map

- `core/src/commands/analysis_isochrone.rs` -- modèle exact (`Point`, `TravelMode`, `network_param("network","mode")`, `work_crs_param("work_epsg","layer_id")`, huit tests dont la description qui cite 100 m) ; nouveau `network_shortest_path.rs` (déclarations seules), enregistrement automatique par `core/build.rs` comme les autres ; `Duration` n'a que `Deserialize` : définir des types de sortie `Serialize` (`Distance`-like en `m`, `Duration`-like en `s`) dans le nouveau fichier, sans toucher aux existants
- `core/src/commands/network_options.rs`, `core/src/point.rs` (`is_point`, `guard_coordinates`, tests `terrain.line_of_sight` à deux points, `step:1.point`) -- à réutiliser sans modifier ; vérifier que `start`/`end` passent `guard_coordinates`
- `core/src/journal/tests.rs`, `core/tests/catalogue.rs` (~1208-1226), `core/tests/conformite_resultats.rs`, `fixtures/resultats-extension.json` -- bras simulé avec `work_epsg`, rejeu, exemple de résultat, fiche
- catalogue des commandes (`catalogue-des-commandes.md`, ligne de table ~384, section ~2199) -- `planifiée` → `faite` ; fixer `length`/`duration` (m, s), `feature_count` 1, `analysis.no_path` ; `core/tests/catalogue.rs` compare
- `qgis-plugin/cassini_qgis/analysis.py` -- ligne `analysis.isochrone` (`chain_version: 2`, `_REPROJECT_TO_WORK`, `origin`, `network`, `_network_of`, `_network_speed`, `_network_tolerance_meters`, `backend.transform_point`, `_check_origin` ~1889, `ORIGIN_OFF_NETWORK`) ; nouvelle ligne `network.shortest_path` (chaîne 1) ; généraliser l'accrochage à une liste de points `start` et `end` sans casser l'isochrone (clé d'entrée à deux points) ; `native:shortestpathpointtopoint` ajouté à `CANCELLABLE_ALGORITHMS` ; garde `analysis.no_path` ; `summary`/`summary_results` (modèle `summarize_join_table`) pour `length` (géométrie en mètres) et `duration` (`TRAVEL_COST` ou champ `cost`, selon la sonde)
- `qgis-plugin/cassini_qgis/summary.py` (`_isochrone` 275, table 1104), `texts.py` (130, 471) -- « Plus court chemin de Gare à Mairie à pied : 1 250 m, 15 min » FR/EN
- `qgis-plugin/tests/` -- `test_analysis.py` (classe isochrone ~1082), `test_chain.py` (versions de chaîne), `test_result_conformance.py` (246, 464), `test_summary.py`, `table_annulable.json`, nouveau `probe_shortest_path.py` (modèle `probe_isochrone.py` et `probe_isochrone_degrees.py`)
- `ai/tests/par_commande/phrases.json`, `training/src/barres.rs` (compte à recalculer), `ai/tests/plan_reel.rs:478`, `ai/tests/echelle_utm_reel/phrases.json:~682` (mot-clé de refus `shortest_path`), `docs/entrainement-du-modele.md:~2630`, `docs/couverture-phrases-reelles.md` -- phrases « Quel est le plus court chemin à pied de la gare à la mairie ? » ; retirer ou adapter le refus devenu faux, sans casser les autres phrases
- `README.md`, `qgis-plugin/README.md`, `docs/regle-de-redaction-des-commandes.md` si elle liste les commandes
- À ne pas toucher : `network_options.rs`, `point.rs`, `analysis_isochrone.rs`, `_REPROJECT_TO_WORK`, `training/src/families.rs`.

## Tasks & Acceptance

**Execution:**
- [ ] `core/src/commands/network_shortest_path.rs` -- commande, schéma, exemples (dont un point repris par `step:k`), tests (déléguée, deux points requis, contrat de réseau, description citant l'accrochage, sortie `.gpkg`) -- contrat du registre
- [ ] `core/src/journal/tests.rs`, `core/tests/`, `fixtures/resultats-extension.json` -- bras simulé, rejeu, exemple de résultat, fiche
- [ ] `qgis-plugin/cassini_qgis/analysis.py`, `summary.py`, `texts.py` -- chaîne, accrochage des deux points, `analysis.no_path`, longueur et durée, résumé FR/EN
- [ ] `qgis-plugin/tests/probe_shortest_path.py` -- sonde QGIS 3.44 : noms de paramètres et sorties, chemin attendu sur réseau synthétique, longueur en m et durée en s, sens unique, point hors réseau, composantes non connectées, couche en degrés ; constats assertés et consignés
- [ ] `qgis-plugin/tests/` unitaires, conformité, `table_annulable.json` -- accrochage départ/arrivée, `no_path`, isochrone inchangée
- [ ] Catalogue, phrases, `barres.rs`, refus à retirer, READMEs -- fiche `faite`, phrase « plus court chemin à pied »

**Acceptance Criteria:**
- Given un réseau synthétique et deux points sur le réseau, when `network.shortest_path` s'exécute, then la couche contient le chemin attendu et le résultat porte `length` en `m` et `duration` en `s`.
- Given un point à plus de la tolérance du réseau, when la commande est lancée, then elle est refusée avant lancement par une erreur nommée qui désigne ce point.
- Given « le plus court chemin à pied de la gare à la mairie » et l'inventaire de test, when l'agent compose, then le plan est valide.
- Given un ancien journal, when on le rejoue, then le résultat est inchangé ; la commande n'est annoncée que si sa chaîne existe (10.4) ; `analysis.isochrone` reste inchangée.

## Implementation Notes

Constats de `probe_shortest_path.py` sous QGIS 3.44.15 (VERDICT OK, 16 cas) : à pied 600 m et 432 s ; en voiture 43,2 s ; sens unique : détour de 300 m ; sens unique sans issue, composantes non connectées et points identiques rendent `analysis.no_path` ; départ ou arrivée hors réseau refusés avant lancement en nommant le point ; couche en degrés avec `work_epsg` : longueur en mètres ; un échec ne laisse aucun fichier. `TRAVEL_COST` est en heures (multiplié par 3600). Ajouts hors plan : clé facultative `on_failure` d'une étape de chaîne (erreur nommée quand l'algorithme échoue) ; `training/src/families.rs` : branche de refus `NO_SHORTEST_PATH` retirée (comme `NO_CONTOUR` en 10.26) et mot-clé `network` de `NO_ROUTE` retiré, sans nouvelle famille ; PL22 remplacée par un refus encore vrai (`watershed`) ; EU34 devient un plan « en voiture » avec les coordonnées écrites ; CHE3 (« le plus court chemin à pied de la gare à la mairie », sans coordonnées) attend une question : le contrôle de points refuse un point que la phrase n'écrit pas (AD-8), donc le critère « plan valide » n'est tenu que par CHE1 et CHE2 qui donnent leurs coordonnées.

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 1 low corrigé, 4 rejetés ; un écart de critère signalé au propriétaire.

| Verdict | Route | Constat et preuve |
|---|---|---|
| low | patch | README de l'extension : octet retour arrière (0x08) à la place de `\b` dans la recette de `network.shortest_path` ; corrigé, relu |
| low | rejeté | `families.rs` modifié malgré « à ne pas toucher » : la branche `NO_SHORTEST_PATH` enseignait une chose devenue fausse et deux tests rougissaient ; même geste qu'en 10.26 (`NO_CONTOUR`), aucune famille ajoutée ; le plan se trompait |
| low | rejeté | CHE3 attend une question et non un plan : un point non écrit dans la phrase est refusé par le contrôle de points (AD-8), donc la question est la réponse juste ; consigné ci-dessus |
| low | rejeté | `on_failure` nomme `analysis.no_path` pour tout échec de l'algorithme : le message énumère les causes, et la couche, le réseau et l'accrochage des deux points sont validés avant lancement ; les autres échecs sont peu probables et la passerelle ne reçoit pas le texte de QGIS |
| false | rejeté | Fichier laissé si la lecture du résumé échoue : `_fail` appelle `_clean` tant que la couche n'est pas ajoutée au projet |

## Design Notes

Décisions prises faute de réponse dans le ticket : un seul chemin (une entité), longueur en `m`, durée en `s`, `analysis.origin_off_network` gardé pour les deux points (le catalogue le nomme déjà), `analysis.no_path` ajouté pour qu'un réseau non connecté ne rende pas une couche vide muette. La durée vient de la stratégie « plus rapide » ; la longueur se lit sur la géométrie dans `work_epsg`, jamais sur un coût.

## Verification

**Commands:**
- `cargo fmt --all --check` et `cargo clippy --workspace --all-targets --locked -- -D warnings` -- expected: sans avertissement
- `cargo test -p cassini-core -p cassini -p cassini-ai -p cassini-training --locked` -- expected: vert
- `python -I qgis-plugin/tests/check_extension.py` puis `python -I -m unittest discover -s qgis-plugin/tests` -- expected: vert
- `python-qgis-ltr.bat qgis-plugin/tests/probe_shortest_path.py` -- expected: VERDICT OK
