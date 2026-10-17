---
epic: epic-facade-qgis
date: 2026-10-17
verdict: accepted-with-open-items
criteria: declared
headless: false
---

# Rétrospective de l'epic 9 — Façade QGIS

## Epic summary

Interactif, mais le propriétaire a répondu d'avance : analyse complète, aucun point à peser en priorité, pas de discussion d'équipe (phase 3 sautée). Un point a été regardé en particulier : le 9.11 a été construit **après** le refactor sweep 9.13 et la suite de bout en bout 9.14.

`pending_tickets` est vide. Seul le 9.1 est `done` ; les tickets 9.2 à 9.14 sont au statut `built` (finis par le build, pas encore déclarés `done`).

| Ticket | Titre | Statut | Baseline du plan | Plage (commits) |
|---|---|---|---|---|
| 9.1 | Traceur : l'extension charge le noyau, `system.about` | done | `3d36aeb` | `3d36aeb..194dab6` (2) |
| 9.2 | Boucle d'ordres non bloquante, annulation | built | `194dab6` | `..99ce30f` (1) |
| 9.3 | Garde hors ligne, recette du Python | built | `99ce30f` | `..7ccad45` (1) |
| 9.4 | Panneau | built | `7ccad45` | `..fb76e5b` (3) |
| 9.5 | Ouvrir GeoPackage, Shapefile, GeoJSON | built | `fb76e5b` | `..2a88c98` (4) |
| 9.6 | Ouvrir CSV et GeoParquet | built | `2a88c98` | `..cf04edd` (1) |
| 9.7 | Inventaire, description d'une couche | built | `cf04edd` | `..57e3351` (2) |
| 9.8 | Zone tampon | built | `57e3351` | `..e5dcb0c` (3) |
| 9.9 | Intersection, sélection par attribut | built | `e5dcb0c` | `..40517e9` (1) |
| 9.10 | Statistiques par zone | built | `40517e9` | `..97d51dc` (1) |
| 9.12 | Export PDF | built | `97d51dc` | `..793c237` (4) |
| 9.13 | Refactor sweep | built | `793c237` | `..5b3e8c4` (1) |
| 9.14 | Suite de bout en bout | built | `5b3e8c4` | `..7829f94` (1) |
| **9.11** | Isochrones | built | `7829f94` | `..a6a1d58` (1) |

- **Ordre des plages** : classé par la place des baselines dans l'historique, pas par numéro. Le 9.11 passe en dernier. Sa plage s'arrête à `a6a1d58`, dernier commit de l'epic ; le commit `7735770` (`.gitignore`) est exclu. La plage du 9.11 est donc enregistrée (baseline) mais bornée par moi à la main.
- **Plage totale** `3d36aeb..a6a1d58` : 26 commits, 0 fusion, 78 fichiers. Sur `core`, `cli` et `qgis-plugin` : 43 fichiers, +8080 −18. Aucun `Cargo.toml` ni `Cargo.lock` touché : aucune dépendance ajoutée.
- **Inventaire disponible** : fichier de l'epic (Done when présent), `tickets.toml`, 14 plans, `deferred-work.md`, architecture, `git_evidence.py` (une exécution par plage), rétrospective de l'epic 2, code courant. Aucun fichier d'histoire (tickets sans `story_file`).
- **Inventaire manquant** : **journaux de session absents** ; l'analyse des leçons de processus tirées des sessions n'a pas été faite (ce qui a été essayé et abandonné est inconnu). Le plan orphelin `plan-8-2-phrases-et-validateur.md` (epic 8) est hors périmètre.
- **Portée réduite, déclarée** : `bmad-review` n'a pas été invoqué (je tourne comme sous-agent, sans lancer de relecteurs en parallèle). Les lentilles adversariale, cas limites et trous de vérification ont été appliquées **en ligne**, sur une portée réduite : le diff du 9.11 (`a6a1d58`), le code de rejeu du 9.14 (`journal.rs`, `serve.rs`), les plans 9.11, 9.13, 9.14 et la jointure entre eux. Les 11 autres tickets n'ont pas été relus ligne à ligne ; ils sont couverts par les plans, le triage qu'ils portent et les tests exécutés.

## Findings

Les constats marqués **vérifié** ont été relus dans le code ou exécutés ici. Les autres sont signalés comme tels.

### A. Frontière 9.11 / 9.13 / 9.14 (le point demandé)

| # | Constat | Source | Disposition |
|---|---|---|---|
| F1 | **L'ordre de construction a contredit l'ordre déclaré, une seconde fois.** `tickets.toml` met 9.13 `after` 1 à 12 (donc 9.11) et 9.14 `after` 13 ; or le commit du 9.11 (`a6a1d58`) est postérieur à ceux de 9.13 (`5b3e8c4`) et 9.14 (`7829f94`). Le plan 9.13 dit balayer « les entrées 9.1 à 9.12 » et ne cite 9.11 que par sa décision (`793c237`). **Vérifié.** C'est la même faute que l'action R7 de la rétro de l'epic 2 (2.9 et 2.10 construits avant 2.11). Cause apparente : l'entrée 9.11 attendait une décision du propriétaire (réseau routier), prise le 2026-10-16 ; les deux entrées de fermeture ont été construites sans elle au lieu d'attendre. | `tickets.toml` ; `git log` (`793c237`, `5b3e8c4`, `7829f94`, `a6a1d58`) ; plan 9.13 intent | Process : récidive avérée, voir A1 |
| F2 | **Le 9.13 n'a pas pu balayer le 9.11.** Aucun constat reporté par le 9.11 : son triage n'a aucun `defer`, et `deferred-work.md` n'a pas bougé au commit `a6a1d58`. Les garde-fous du 9.13 (garde réseau, `redact`) ont toutefois été testés avec du code déjà présent ; le diff du 9.11 touche `analysis.py` et reste sous la garde (`check_extension.py` : « propre », **vérifié**). Aucun défaut direct, mais le 9.11 n'a eu ni balayage ni relecture d'ensemble. | `a6a1d58 --stat` ; `deferred-work.md` (`git log`) ; `check_extension.py` | Accepter, couvert par A1 |
| F3 | **Le rejeu par le canal fonctionne pour `analysis.isochrone` — vérifié par exécution, mais non couvert par un test commité.** Sonde jetable (scratchpad, copie adaptée de `probe_end_to_end.py`, sous QGIS 3.44.15 et le binaire release à jour) : `layer.add` du réseau `reseau-grille-2154.geojson`, deux `analysis.isochrone` (marche 3 min, voiture 180 s), arrêt du noyau, projet vidé et rouvert, nouveau noyau, `replay()` → `{'steps': 5, 'identical': True}` ; `replay/` contient `3-analysis.isochrone.gpkg` et `4-analysis.isochrone.gpkg` ; un résultat altéré dans le journal fait échouer le rejeu en nommant l'étape. Le mécanisme est générique : `Replay::run` substitue les `layer_id` anciens dans toutes les chaînes des paramètres (`journal.rs:643`), donc le `layer_id` du réseau, passé en paramètre, est relié ; le résultat de l'isochrone porte un `layer_id` masqué à la comparaison (`journal.rs:681-696`). **Mais** la sonde commitée `probe_end_to_end.py` n'enchaîne que `layer.add` (CSV), `analysis.buffer` et `layout.export_pdf` ; `probe_isochrone.py` pilote le geste directement, sans noyau ; les tests Rust du rejeu utilisent `FakeQgis`. Aucun test commité ne rejoue `analysis.isochrone`, ni `analysis.intersect`, `analysis.select_by_attribute`, `analysis.zonal_statistics`. | `journal.rs:643,681-696` ; `qgis-plugin/tests/probe_end_to_end.py` ; `probe_isochrone.py` ; sonde jetable (scratchpad) | Corriger (A2) |
| F4 | **Écart du plan 9.14 jamais étendu après coup** : le titre « ouvrir, analyser, exporter, rejouer » et le Done when 4 (CAP-3 : cinq analyses « couvertes par un test ») sont tenus par un test par commande, mais le chemin **rejeu** n'est prouvé que sur une seule analyse (le tampon). Le 9.11 aurait dû ajouter son étape à la suite ; il n'en avait pas la consigne, la suite n'existant pas à son plan. | plan 9.14 (Code Map, Acceptance) ; plan 9.11 (Tasks) | Corriger avec F3 |

### B. Défauts et risques de l'isochrone (lentilles sur `a6a1d58`)

| # | Constat | Source | Disposition |
|---|---|---|---|
| F5 | **Tolérance topologique à zéro, validée sur une grille synthétique seulement.** `translate_isochrone` fixe `TOLERANCE: 0` ; un vrai réseau (BD TOPO, OSM) a des nœuds qui ne coïncident pas au mètre, et le service area de QGIS s'arrêterait aux coupures sans erreur : le polygone serait trop petit et rendu comme un succès. Non vérifié sur un réseau réel (hypothèse) ; la seule fixture est `fixtures/reseau-grille-2154.geojson`. | `analysis.py:124-150` ; plan 9.11, Design Notes ; `fixtures/README.md` | Reporter, vérifier sur le jeu hors dépôt (A3) |
| F6 | **Distance d'accroche de 100 m non dite au modèle.** `_ORIGIN_SNAP_METERS = 100.0` décide du refus `analysis.origin_off_network`, mais la description de la commande (lue par l'IA) ne la mentionne pas, ni le plan. **Vérifié** (lecture). | `analysis.py:110` ; `analysis_isochrone.rs` (description) | Reporter, ajouter une phrase à la description |
| F7 | **Rejeu aveugle à la forme du polygone.** L'isochrone rend `feature_count: 1`, quel que soit le mode ; au rejeu, marche et voiture, ou un polygone tout autre, sont « identiques », et le GeoPackage n'est vérifié que par « existe et non vide ». C'est l'écart assumé de l'architecture (6 lignes ajoutées par `7829f94`), pas une faute du 9.11, mais l'isochrone est le verbe où cela compte le plus (le résultat typé ne dit rien du contenu). | `7829f94` (architecture, « Écart assumé ») ; `analysis_isochrone.rs` (résultat) | Accepter pour le MVP ; envisager une empreinte de géométrie ou une emprise au résultat |
| F8 | Schéma `Duration.value` affiché `range(min = 0.0)` inclusif alors que la durée est strictement positive (le noyau refuse 0 à la désérialisation) ; le catalogue le montrerait au modèle. Écarté au triage du 9.11 (« même motif que `analysis.buffer` »). | `analysis_isochrone.rs` ; plan 9.11, triage 3 | Accepter, noté |
| F9 | L'enveloppe convexe surestime l'isochrone sur un réseau en étoile ou coupé par une rivière ; l'origine se saisit en coordonnées, faute de sélecteur sur la carte. Limites assumées et écrites (description, Design Notes). | plan 9.11 ; description de la commande | Accepter |

### C. Vues agrégées

| # | Vue | Constat | Source | Disposition |
|---|---|---|---|---|
| F10 | Taille | **`core/src/journal.rs` : 1471 lignes** (1121 à la rétro de l'epic 2), +350 sur l'epic dont +264 au seul 9.14 ; **`core/src/serve.rs` : 1416 lignes** (+124 au 9.14, le reste vient de 2.7 et 2.11). La rétro de l'epic 2 demandait de découper `journal.rs` « avant d'y toucher » (R4) : on y a touché sans le découper. Côté Python : `qgis-plugin/cassini_qgis/panel.py` 667 lignes (+687 sur l'epic), `analysis.py` 579, `client.py` 423 (tous neufs ou presque). Candidats à regarder, pas des verdicts : la part de tests dans les fichiers Rust reste à mesurer. | `git_evidence.py` (plage totale) ; `wc -l` | Reporter, A4 |
| F11 | Architecture | Aucune dépendance ajoutée ; noyau sans QGIS (`test_noyau_sans_qgis` passe) ; la garde réseau de l'extension est étendue (9.13). Rien de plus cassé que ce que l'epic 2 avait déjà. Pas de cycle nouveau relevé sur la portée lue. | `git diff` des `Cargo.*` ; `cargo test` ; `check_extension.py` | Accepter |
| F12 | Duplication | Les tables `translate_*` d'`analysis.py` suivent un seul motif (exécuteur générique + `ALGORITHMS`), le 9.11 y ajoute une entrée et l'étape `then` sans copier l'exécuteur : bon point. Les six copies du « projet de test » côté Rust (constat F7 de l'epic 2) n'ont pas été revérifiées. | `analysis.py:191-250` | Accepter |
| F13 | Réconciliation à la spec | **AD-9 n'est toujours pas amendé** : la note de l'epic dit « AD-9 reste à amender » pour les chemins absolus de `order.execute` et de `session.hello`, et le commit `7829f94` n'ajoute à l'architecture que l'écart des `layer_id`. De plus, AD-9 dit que l'identifiant de couche « n'est jamais une clé de résolution dans le journal » ; le rejeu 9.14 le lie désormais (écart assumé, écrit) sans que la règle AD-9 soit réécrite. | `epic-facade-qgis.md` (Notes) ; `architecture-noyau-et-facade-qgis.md` AD-9 ; `git show 7829f94` | Réconcilier (A5) |

### D. Process

| # | Constat | Source | Disposition |
|---|---|---|---|
| F14 | **Aucun ticket de l'epic n'a eu mieux que la relecture `quick`** : les 14 plans portent `review: 'quick'`, `review_source: 'pinned'` (ou équivalent) et `lenses_ran: [quick]`, y compris pour les entrées à risque `high` (9.1, 9.2, 9.8, 9.9, 9.11, 9.14). La boucle de relecture a donc toujours été à une lentille. **Vérifié.** | les 14 plans, front matter | Process, A6 |
| F15 | **Les constats F1 à F3 de la rétro de l'epic 2 (rejeu fragile) sont toujours là alors que le 9.14 bâtit le rejeu par le canal dessus** : une ligne illisible donne toujours `InvalidData` sans nommer l'étape (`journal.rs:509,522`) ; un résultat de plus de 200 octets reste un résumé `{"truncated":true,"bytes":n}` (`journal.rs:693,717`) ; `size`/`modified` des entrées sont écrits (`input_entry`, `:699-708`) et jamais relus. **Vérifié** (lecture). | `journal.rs` ; rétro epic 2 (F1, F2, F3) | Corriger (A7) |

## Behavior verification

L'epic change le comportement à l'exécution ; il a été exercé ici, sur le dépôt à `HEAD` (`7735770`), arbre propre avant et après.

- `cargo test --locked` : **140 tests passent** dans le crate `cassini-core` plus les autres crates (15, 3 et 2 dans les autres binaires), 0 échec.
- `cargo clippy --all-targets -- -D warnings` : aucun avertissement. `cargo fmt --check` : propre.
- `python -I -m unittest discover -s qgis-plugin/tests` : **184 tests, OK**. `python -I qgis-plugin/tests/check_extension.py` : « Garde de l'extension : propre. »
- `cargo build --release` puis, sous le vrai QGIS 3.44.15 (`python-qgis-ltr.bat`) : `probe_end_to_end.py` → **RÉSULTAT : OK** (exécution de `layer.add`, `analysis.buffer`, `layout.export_pdf`, fermeture, réouverture, rejeu `identical: true`, rejeu altéré refusé, CSV d'origine inchangé par empreinte) ; `probe_isochrone.py` → **OK** ; `probe_layout_export.py` → **OK** (PDF d'une page, 6 couches, sources `mnt-villeneuve-le-roi.tif` et `villeneuve-le-roi-2154.gpkg`).
- **Rejeu d'`analysis.isochrone` par le canal** : voir F3 ; **OK** sur la sonde jetable (5 étapes, `identical: true`, écart détecté). Cette sonde n'est pas commitée.
- **Non exercé** : QGIS 4.x (le plancher du manifeste dit 3.44 à 4.99, seul 3.44.15 est installé) ; un réseau routier réel (F5) ; le panneau en interactif (listes déroulantes, bouton Parcourir : recette manuelle, hors de ce que je lance) ; GeoParquet sur un fichier réel ; un poste sans réseau (la garde statique est contrôlée, pas le trafic).

## Previous-retro follow-through

Rétrospective précédente : `epic-registre-projet-import/epic-registre-projet-import-retrospective.md` (epic 2). Ses actions R1 à R8, vérifiées contre le code et les commits :

| Action (propriétaire) | Atterri ? |
|---|---|
| R1 : construire 2.11, y inclure F4 et F17 (Dev) | **Oui** pour 2.11 : `df8cc21` (`feat(2.11)`), l'epic 9 en dépend (9.1 `after` 2.11). F17 (`mode` dans `session.hello`) : *aucune preuve trouvée*, non revérifié. |
| R2 : rendre le rejeu robuste (ligne illisible nommée, gros résultats par empreinte, entrées relues) (Dev) | **Non** : `journal.rs:509,522`, `:693,717`, `:699-708` inchangés dans leur principe (F15). |
| R3 : contrôler `path_params` / `output_params` dans `contract.rs` (Dev) | *Aucune preuve trouvée* : `contract.rs` n'a aucun commit depuis `22fa9a8` (2.4), antérieur aux dernières entrées de l'epic 2. |
| R4 : découper `journal.rs` avant d'y toucher, mutualiser le projet de test (Dev) | **Non** : `journal.rs` passe de 1121 à 1471 lignes (F10). |
| R5 : réconcilier la spec (R4, forme de référence entre étapes, règle 9) (Propriétaire) | *Aucune preuve trouvée* ; l'écart des `layer_id` a été écrit à part par le 9.14 (F13). |
| R6 : trancher la confiance en un journal reçu et la borne d'une ligne sur `serve` (Propriétaire) | *Aucune preuve trouvée* (pas de commit sur `serve.rs` nommant ce point). |
| R7 : tenir l'ordre `after` des tickets (Propriétaire) | **Non** : récidive avec 9.11 après 9.13 et 9.14 (F1). |
| R8 : reporter F12, F13, F14, F18, F19, F23 dans `deferred-work.md` (Dev) | **Non** : `deferred-work.md` ne mentionne aucun de ces points (recherche de `core_version`, lien symbolique, borne de ligne, U+FFFD : aucun résultat). |

## Action items

Tous proposés, **non appliqués** ; le propriétaire décide de l'exécution. « Remédiation » et « réconciliation de spec » sont des propositions à appliquer à la main ou par `bmad-build`.

| # | Action | Constats | Propriétaire | Nature |
|---|---|---|---|---|
| A1 | Ne plus construire un ticket de fermeture (refactor sweep, suite de bout en bout) tant qu'une entrée qu'il attend n'est pas construite. Pour le 9.11 : relancer un balayage court sur son diff (garde, redact, constats du 9.11) et le noter ; et, à l'avenir, faire échouer la mise à `built` d'un ticket dont un `after` est inachevé, ou le dire au lancement. | F1, F2 | Propriétaire (avec `bmad-ticket`) | Leçon de processus |
| A2 | Ajouter `analysis.isochrone` (et, au moins, une des trois autres analyses) à la sonde `probe_end_to_end.py`, pour que le rejeu des analyses soit prouvé par un test commité ; étendre la suite de bout en bout, pas la sonde jetable. | F3, F4 | Dev (`bmad-build`, entrée de correction courte) | Remédiation |
| A3 | Vérifier `analysis.isochrone` sur un réseau routier réel (par exemple un extrait de BD TOPO placé dans `../donnees-test-cassini/`, hors dépôt) : le polygone est-il complet ? Si non, passer une tolérance non nulle (à décider avec le propriétaire) et décrire la distance d'accroche de 100 m à l'IA. | F5, F6 | Propriétaire (données) puis Dev | Remédiation, à vérifier d'abord |
| A4 | Découper `core/src/journal.rs` (chemins, `canonical`, `registry_version` à part) et regarder `serve.rs` ; à faire avant l'epic 5, qui touche le rejeu. Mesurer la part de tests dans les fichiers avant de trancher. | F10, rétro 2 (R4) | Dev | Passe de nettoyage |
| A5 | Amender le contrat d'architecture : AD-9 (chemins absolus de `order.execute` et `session.hello`, rôle du `layer_id` au rejeu) ; l'interprétation incertaine n'est pas écrite d'office. | F13 | Propriétaire, avec `bmad-architecture` | Réconciliation de spec |
| A6 | Décider si la relecture `quick` suffit aux entrées `high`. Au moins pour le rejeu et le canal (9.2, 9.14), faire tourner une relecture à plusieurs lentilles au prochain epic. | F14 | Propriétaire | Leçon de processus |
| A7 | Appliquer les actions R2 et R3 de la rétro de l'epic 2 avant l'epic 5 : nommer l'étape d'une ligne illisible, comparer les gros résultats par empreinte, relire `size`/`modified`, contrôler `path_params`/`output_params`. | F15, rétro 2 | Dev | Remédiation |
| A8 | Reporter dans `deferred-work.md` : F7 (rejeu aveugle à la géométrie), F8 (schéma de durée inclusif), et les points R8 non reportés de la rétro de l'epic 2. | F7, F8, rétro 2 (R8) | Dev | Suivi |
| A9 | Passer 9.2 à 9.14 de `built` à `done` (clôture) après décision du propriétaire sur ces actions ; rien n'est écrit par cette rétro. | Epic summary | Propriétaire (`bmad-ticket`) | Clôture |

## Acceptance verdict

**Accepté avec points ouverts**, critères **déclarés** (Done when de l'epic). Aucun ticket inachevé (`pending_tickets` vide), donc la règle du verdict « rejeté » ne joue pas.

Done when, un par un, sur ce qui a été exécuté :

1. CAP-13, extension installable, panneau, noyau en processus séparé, commande de bout en bout : **tenu** (9.1 `done` par recette manuelle ; `probe_end_to_end.py` OK).
2. Noyau sans QGIS : **tenu** (`cargo test` passe sans lier QGIS ; `test_noyau_sans_qgis` ; aucune dépendance ajoutée).
3. CAP-9 et CAP-10, cinq formats et inventaire, original inchangé par empreinte : **tenu** (la sonde vérifie l'empreinte du CSV ; 9.5, 9.6 et 9.7 portent leurs tests). GeoParquet sur un fichier réel non exercé ici.
4. CAP-3, cinq analyses par le cadre de traitement, chacune une commande couverte par un test : **tenu** (tampon, intersection, sélection, statistiques, isochrone, tous déclarés et testés). Nuance : le rejeu n'est prouvé que pour le tampon par un test commité (F3, F4).
5. CAP-5, PDF avec titre, légende, échelle, flèche du nord, sources : **tenu** (`probe_layout_export.py` OK, sources listées au résultat).
6. Garde hors ligne couvrant le Python : **tenu** (`check_extension.py` propre, étendue par 9.13).

Points ouverts, aucun bloquant : F5 (isochrone sur réseau réel non éprouvée), F13 (AD-9), F10 et F15 (dette du journal, héritée de l'epic 2, non traitée), F1 (récidive d'ordre), F14 (relecture à une lentille). Une décision humaine l'emporte sur ce verdict machine.

## Open questions

1. Un réseau routier réel donne-t-il un isochrone complet avec `TOLERANCE: 0` (F5) ? Seul le propriétaire a les données ; ce que la réponse changerait : l'isochrone reste « tenue » ou passe en correction avant l'epic 6 (démonstration).
2. La relecture à une lentille est-elle un choix assumé pour tout l'epic (F14), ou un réglage par défaut non remis en cause ?
3. Faut-il un rejeu qui compare mieux que le nombre d'entités pour les sorties QGIS (F7), ou le résultat typé suffit-il pour le MVP ? Politique, pas technique.
4. Sessions de build : aucun journal de session disponible, donc aucune leçon sur ce qui a été essayé et abandonné (en particulier pourquoi 9.13 et 9.14 ont été construits avant 9.11).
5. Le plancher `qgisMaximumVersion=4.99` n'a été exercé que sous 3.44.15 ; est-ce attendu que 4.x reste à vérifier avant publication ?
