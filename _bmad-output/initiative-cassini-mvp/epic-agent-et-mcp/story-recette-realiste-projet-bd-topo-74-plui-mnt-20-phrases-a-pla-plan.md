---
title: "Recette réaliste : projet BD TOPO 74 + PLUi + MNT, 20 phrases à plan attendu (barre 16/20)"
type: 'feature'
ticket: '50'
created: '2026-11-13'
status: 'built'
baseline_revision: '6ad9f381fa0a26016e47aaa0173e42d760753074'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** La recette d'Annecy (5.45) joue sur un projet de 11 couches, sous le seuil du tri de l'inventaire. Rien ne prouve que le modèle tient sur le projet réel d'environ 60 couches (54 tables BD TOPO, 7 couches du PLUi, 2 MNT), où la vue est triée (5.46 à 5.49) : bonne couche trouvée, question entre couches proches, aucun `ai.prompt_too_long`. La démo filmée attend cette preuve.

**Approach:** Réutiliser la sonde `probe_recette.py` telle quelle (projet et phrases en arguments) avec un second fichier de 20 phrases écrites à la main, `qgis-plugin/tests/recette/gros-annecy.json`, chacune avec son plan attendu. Même règle qu'en 5.45 (plan conforme ET exécuté, ou question pour une phrase ambiguë), barre 16/20. La sonde imprime en plus les durées utiles à la décision et le nombre de `ai.prompt_too_long`.

## Boundaries & Constraints

**Always:** les 20 phrases sont écrites à la main, relues par le propriétaire avant la première mesure (hitl) ; couches désignées par `@nom` (noms réels du projet : `batiment`, `commune`, `erp`, `200066793_ZONE_URBA_20260326`…) ; au moins deux phrases à couches proches dont la question est attendue ; au moins deux phrases dont la couche ne se trouve que par une valeur ou un synonyme (ex. « les lacs » → `plan_d_eau`, valeur `Lac`) ; la sonde travaille sur une copie du projet, jamais l'original ; `CASSINI_MODELS` obligatoire ; les 20 phrases rejoignent le garde de recouvrement et ne sont jamais copiées dans le générateur ; barre ≥ 16/20 (décision du propriétaire, 2026-11-10) ; durées cibles < 60 s sur CPU et < 30 s sur GPU (décision du propriétaire, 2026-11-10).

**Never:** pas de données d'Annecy au dépôt (le projet reste dans `../donnees-test-cassini/annecy/`) ; pas de nouvelle dépendance ; ne pas toucher au noyau, au tri de l'inventaire ni au générateur ; ne rien installer, ne lancer aucun entraînement ; la mesure ne décide d'aucune installation ; ne pas réécrire `probe_recette.py`, seulement l'enrichir.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Plan conforme exécuté | plan = attendu, `plan_done` | juste | — |
| Couches proches | deux couches se disputent un mot | juste si `plan_question` | un plan rendu = raté |
| Couche trouvée par une valeur | « les lacs » | plan sur `@plan_d_eau` | autre couche = raté « plan non conforme » |
| Refus de composition | `ai.prompt_too_long` | raté « refus : ai.prompt_too_long », compté à part | la phrase suivante est jouée |
| Fin de session | 20 phrases jouées | `SCORE : N/20 (barre 16)`, ratés, durées, `prompt_too_long : n`, rejeu | code 1 si barre non tenue ou rejeu non identique |

</frozen-after-approval>

## Code Map

- `qgis-plugin/tests/probe_recette.py` -- sonde 5.45, déjà paramétrée (projet, phrases) ; affiche la durée par phrase ; à enrichir : récapitulatif des durées (première, médiane, maximale) et compte des refus `ai.prompt_too_long`.
- `qgis-plugin/tests/recette_verdict.py` -- module pur (`load`, `judge`, `score_line`, `bar`) ; y ajouter les fonctions pures du récapitulatif, testées sans QGIS.
- `qgis-plugin/tests/test_recette_verdict.py` -- tests actuels ; modèle pour les nouveaux.
- `qgis-plugin/tests/recette/annecy.json` -- modèle de forme `{id, text, expect: {"plan": [...]} | {"question": true}, note}`.
- `../donnees-test-cassini/annecy/essais-5-48/phrases-gros.json` et `inventaire-gros.json` -- 7 phrases de mesure déjà écrites et noms de couches, champs et valeurs réels du projet à 63 couches ; sources, pas des phrases à reprendre telles quelles.
- `training/src/barres.rs:25,201` -- `RECETTE_ANNECY` et `from_json` : ajouter la constante du nouveau fichier et son compte (20).
- `training/tests/recouvrement.rs:157` -- `the_real_trials_are_read_whole` : le total passe de +20 à +40 pour les deux recettes.
- `docs/entrainement-du-modele.md:638` -- section « Recette d'Annecy à 20 phrases » : modèle de la section à ajouter.
- Ne pas toucher : `core/` (inventaire trié), `ai/`, `training/` hors garde.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/tests/recette/gros-annecy.json` -- 20 phrases `{id, text, expect, note}` (ids `GA01`…`GA20`) sur le projet de 63 couches : données et projet, sélection par attribut sur BD TOPO et PLUi, tampon, terrain sur le MNT 5 m, coordonnées, mise en forme, PDF, couches proches (routes ; voies ferrées) et couches trouvées par une valeur (lacs ; gares)
- [ ] `qgis-plugin/tests/recette_verdict.py` -- fonctions pures : récapitulatif des durées et compte des refus `ai.prompt_too_long` -- consignation exigée par le ticket
- [ ] `qgis-plugin/tests/test_recette_verdict.py` -- tests du récapitulatif et du compte ; contrôle que `gros-annecy.json` porte 20 phrases, ids uniques, au moins deux questions, au moins deux phrases à couche trouvée par valeur (note), noms `@` valides contre `inventaire-gros.json` lorsqu'il est présent (sinon saut)
- [ ] `qgis-plugin/tests/probe_recette.py` -- imprimer le récapitulatif et `prompt_too_long : n` après le score ; sans changer le verdict ni le code de sortie, sauf `prompt_too_long > 0` → code 1
- [ ] `training/src/barres.rs`, `training/tests/recouvrement.rs` -- lire `gros-annecy.json` dans les essais réels (`recette:gros-annecy`), total mis à jour
- [ ] **Halt hitl** -- le propriétaire relit les 20 phrases et plans attendus ; corrections ; aucune mesure avant son accord
- [ ] Mesure CPU puis GPU (modèle installé, noyau installé, QGIS 3.44.15, projet `projet-gros-annecy.qgz`) -- score, ratés, durées, première composition (inventaire QGIS compris), rejeu ; sorties brutes hors dépôt
- [ ] `docs/entrainement-du-modele.md` -- section « Recette du gros projet (5.50) » : commande, règle, phrases relues par le propriétaire, scores et durées CPU/GPU

**Acceptance Criteria:**
- Given les tests purs, when `python -I -m unittest discover -s qgis-plugin/tests`, then ils passent et `gros-annecy.json` porte 20 phrases dont au moins deux questions attendues.
- Given le générateur aux graines par défaut, when `cargo test -p cassini-training --test recouvrement`, then le garde est vert et lit les 40 phrases des deux recettes.
- Given le modèle installé et le projet de 63 couches, when la sonde joue les 20 phrases dans une session, then elle imprime `SCORE : N/20` avec N ≥ 16, aucun `ai.prompt_too_long`, les durées par phrase (première composition à part) et un rejeu après réouverture identique.
- Given un score sous 16/20 ou un `ai.prompt_too_long`, when la mesure est consignée, then le résultat est écrit tel quel dans la doc et le ticket reste ouvert : aucune barre n'est abaissée.

## Implementation Notes

- Implémenté par un sous-agent selon le plan ; les cases des tâches ne sont pas cochées, la mesure ayant été faite ensuite par la session principale. Critère « N ≥ 16 » non atteint (voir plus bas). Fichiers : `qgis-plugin/tests/recette/gros-annecy.json` (GA01–GA20), `recette_verdict.py` (`too_long_count`, `durations_line`), `probe_recette.py` (récapitulatif, code 1 si `prompt_too_long > 0`), `test_recette_verdict.py`, `training/src/barres.rs`, `training/tests/recouvrement.rs` (+20), `docs/entrainement-du-modele.md`.
- Phrases relues par le propriétaire, mesure demandée « telle quelle » (2026-11-13). Valeurs `Départementale` et `Gare voyageurs et fret` non vérifiées sur les données (inventaire de test limité à 11 valeurs par champ).
- **Mesure : 14/20 sur CPU et sur GPU, barre de 16 non tenue**, mêmes six ratés (GA03, GA06, GA08, GA09, GA13, GA19). `prompt_too_long` 0, rejeu identique. Première composition 90,3 s (CPU) et 39,9 s (GPU), médianes 28,3 s et 3,9 s. GA13 : plan conforme, échec d'exécution `terrain.viewshed_too_large`, défaut de la phrase.
- Mesures lancées avec le noyau `C:\ct-5-57g` ; CPU = même dossier copié sans `ggml-vulkan.dll`. Premier lancement : `python-qgis-ltr.bat` appelé sans `call` dans un `.bat` ; la suite du script n'a pas tourné, GPU relancé à part.

## Plan Change Log

## Review Triage Log

Passe 1 (2026-11-13), lentille `quick` par sous-agent. Verdicts : high 0, medium 4, low 3, false 0, maybe-false 0.

| Constat | Verdict | Route | Preuve |
|---|---|---|---|
| Score 14/20 sous la barre, première composition au-delà des cibles | medium | defer | résultat de mesure réel, consigné tel quel (AC 4) ; la suite relève d'une décision, pas d'un patch |
| Bloc `unittest.main()` avant les classes `Summary` et `GrosAnnecy` | medium | patch | lancé seul, le fichier n'exécutait pas les nouveaux tests ; bloc remonté en fin de fichier |
| Doc : « durées hors première phrase sous 60 s sur CPU » | medium | patch | faux : la sortie brute CPU donne 64,7 s ; doc corrigée |
| Valeurs `Départementale` et `Gare voyageurs et fret` jamais vérifiées dans les données | medium | defer | cause possible de GA06 et GA09 ; à vérifier dans le GeoPackage BD TOPO |
| Doc : « à relire avant toute mesure » périmé | low | patch | remplacé par « relues avant la mesure » |
| Plan : « sans écart au plan » contradictoire | low | patch | note corrigée |
| `durations` compte aussi refus et questions ; pas de test de la chaîne de `on_failed` | low | rejeté | sans effet ici, correction plus lourde qu'un fix direct |

## Design Notes

Le plan comparé est celui que le noyau montre (`plan_proposed`), plan complété compris, comme en 5.45. Les phrases qui changent le projet (retrait de couche, reprojection) viennent en fin de session, pour ne pas changer l'inventaire des phrases suivantes. Une couche détaillée au tri (5.46) se reconnaît à son nom, à son identifiant, à un champ ou à une valeur : « les lacs » ne trouve `plan_d_eau` que par la valeur `Lac` du champ `nature`, d'où son intérêt dans la recette.

```json
{"id": "GA00", "text": "…", "expect": {"plan": [{"command": "analysis.select_by_attribute",
  "params": {"layer_id": "@plan_d_eau", "field": "nature", "operator": "eq", "value": "Lac"}}]}}
```

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: OK
- `python -I qgis-plugin/tests/check_extension.py` -- expected: code 0
- `cargo test -p cassini-training --test recouvrement` -- expected: vert
- sonde sous `python-qgis-ltr.bat`, `QT_QPA_PLATFORM=windows`, sur `projet-gros-annecy.qgz` et `gros-annecy.json`, CPU puis GPU -- expected: score ≥ 16/20, rejeu identique, `prompt_too_long : 0`
