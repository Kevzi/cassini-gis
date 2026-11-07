---
title: 'Inventaire du projet gardé entre les phrases, recalculé seulement pour une couche qui change'
type: 'feature'
ticket: ''
created: '2026-11-08'
status: 'built'
baseline_revision: '6ad13abbf642b139f818dd479666769de5e7c083'
route: 'oneshot'
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

**Problem:** Sur Annecy, chaque phrase met 30 à 35 s, au-delà des 30 s exigées. La mesure du 2026-11-08 montre que le modèle n'y est pour rien. `duree_reel` sur GPU donne 5,6 s par phrase en moyenne avec l'inventaire d'Annecy (5 250 jetons, repris d'un instantané), contre 7,0 s avec celui de Villeneuve. C'est le geste `project.inventory` de l'extension qui prend 20,7 s, contre 0,0 s à Villeneuve, et il est refait à chaque phrase : `uniqueValues` champ par champ sur les 250 000 routes (12 s), les 600 000 bâtiments (5 s) et les 32 000 parcelles (3 s). Une lecture unique de tous les champs ne coûte pas moins.

**Approach:** Dans `qgis-plugin/cassini_qgis/inventory.py`, les faits d'une couche vectorielle (sorte, géométrie, nombre d'objets, champs et valeurs) sont gardés en mémoire d'une phrase à l'autre. La clé est l'identité de la couche : id, source, filtre, nombre d'objets, champs et types, date du fichier source. Ils ne sont recalculés que si la clé change, ou si la couche a des modifications non enregistrées : on ne garde alors rien. Le résultat du geste reste octet pour octet celui d'aujourd'hui. Tests sans QGIS : une seconde lecture ne relit pas les valeurs ; un changement de clé ou une couche en cours d'édition les relit.

</frozen-after-approval>

## Implementation Notes

Route courte : une clé et un cache dans l'extension, plus des tests. Mesures à refaire après : le geste seul, puis une sonde de 3 phrases sur Annecy.

Fait (`qgis-plugin/cassini_qgis/inventory.py`) :
- `QgisBackend.facts_key` donne la clé d'une couche. Elle vaut None si la couche est en cours d'édition ou si sa source n'est pas un fichier local (couche en mémoire, base de données).
- `source_file` lit le chemin d'une source : `x.gpkg|layername=…`, ou `file:///…?…` pour un CSV. Elle n'importe aucun module réseau (garde-fou du principe 5).
- `source_stamps` relève date et taille du fichier, de son `.dbf` et de son `-wal`.
- `_FACTS` garde une entrée par couche. Celles des couches sorties du projet sont retirées à chaque inventaire. Une clé qui lève une exception fait relire la couche, sans faire échouer l'inventaire.
- 15 tests ajoutés, dont la vraie clé testée sur une fausse couche avec des fichiers temporaires.

Mesures (2026-11-08, QGIS 3.44, projet d'Annecy) :
- Inventaire relu deux fois dans la même session : 24,2 s puis 0,0 s, résultat identique.
- Sonde de 3 phrases enchaînées, vrai modèle : 32,5 s, puis 4,3 s, puis 2,9 s (avant : 30 à 35 s chacune). Rejeu identique.
- La 1re phrase reste au-dessus de 30 s : la première lecture des 850 000 objets coûte environ 24 s. Report : calculer l'inventaire en tâche de fond à l'ouverture du projet.
- `duree_reel` sur GPU (Vulkan, 3070 Ti) : 5,6 s par phrase en moyenne avec l'inventaire d'Annecy, 7,0 s avec celui de Villeneuve. Le modèle n'est pas en cause.

Vérifié : 453 tests de l'extension verts.

## Review Triage Log

- medium, patch : la clé d'un Shapefile ne suivait pas le `.dbf`. Elle prend maintenant la date et la taille du `.dbf`.
- medium, patch : la clé d'un GeoPackage ne suivait pas le `-wal`. Il est ajouté à la clé.
- medium, patch : un CSV en `file:///…?…` ou une couche en mémoire n'avaient pas de date. Le chemin du CSV est maintenant lu, et une source qui n'est pas un fichier n'est jamais gardée.
- medium, patch : au-delà de 64 couches, l'éviction vidait le cache à chaque passage. Plus de plafond : une entrée par couche, et les couches sorties du projet sont retirées.
- medium, patch : une clé qui échouait faisait échouer tout l'inventaire. L'exception est maintenant rattrapée, et la couche relue.
- medium, patch : la vraie clé n'était pas testée. Elle l'est sur une fausse couche et des fichiers réels, et le résultat avec cache est comparé à celui sans cache.
- low, patch : les mesures manquaient au plan. Elles sont consignées ci-dessus.

## Verification

**Commands:**
- tests de l'extension -- expected: verts, dont les nouveaux

**Manual checks:**
- Sonde de 3 phrases sur Annecy : la 1re sous 30 s, les suivantes au temps du modèle seul.
