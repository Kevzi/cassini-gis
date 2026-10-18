# Revue « faits vérifiés » : delta E1 du contrat d'architecture (2026-10-18)

Périmètre : AD-24, AD-25, AD-26, la précision d'AD-6, l'écart d'AD-15, les conventions
« Familles de commandes » et « Réglages ». Lentille : chaque décision engagée a-t-elle été
confrontée au web ou au dépôt plutôt qu'affirmée de mémoire ? Le contrat n'a pas été modifié.

**Verdict.** Le delta tient sur le fond : toutes les primitives nommées existent en natif dans
QGIS, et la vérification d'AD-25 est déjà employée par le code. Mais trois affirmations sont
imprécises ou incomplètes face aux faits : la liste blanche d'AD-24 ne nomme pas les algorithmes
réseau réels (dont un géocodeur Nominatim **dans le cadre de traitement**), l'énumération d'AD-6
oublie `command.run`, et la convention « Réglages » ne décrit pas toutes les variables
réellement lues. Le format BAN d'AD-26 n'est que partiellement confirmé.

Légende : **confirmé** · **à corriger** · **non vérifiable** (dans cette session).

---

## 1. AD-25 : vérifier qu'un algorithme existe dans le registre de traitement

| Fait | Statut | Source |
| --- | --- | --- |
| `QgsApplication.processingRegistry().algorithmById(id)` rend `None` si l'algorithme est absent | **confirmé** | Doc PyQGIS `QgsProcessingRegistry` (master, donc série 4.x) : « If no matching algorithm is found, None is returned » — https://qgis.org/pyqgis/master/core/QgsProcessingRegistry.html |
| Fonctionne sur QGIS 3.44 | **confirmé par le dépôt** | `qgis-plugin/cassini_qgis/analysis.py:274` appelle déjà `algorithmById` et lève `GestureError` si `None` ; la sonde `qgis-plugin/tests/probe_zonal_statistics.py` tourne sous QGIS 3.44.15 |
| Les algorithmes GRASS ne sont au registre que si le fournisseur est actif | **confirmé (source ancienne)** | Bug #18488 (2018), corrigé : les fournisseurs décochés ne chargent plus leurs algorithmes — https://trac.qgis.org/issues/18488 ; le registre émet `providerAdded` / `providerRemoved` (doc PyQGIS ci-dessus) |
| Identifiant des algorithmes GRASS | **à préciser** | Le fournisseur a été renommé `grass7` → `grass` (commit « Rename grass7 provider to grass provider », avec redirection transparente des anciens `grass7:`). La version exacte de QGIS n'a pas été confirmée. La table doit nommer `grass:…`, et le test d'annonce doit tourner sur 3.44 **et** 4.x |

Remarque de conception (non bloquante) : le fournisseur GRASS est une extension Python du
cœur de QGIS ; l'ordre de chargement des extensions n'est pas garanti. Faire la vérification à
`session.hello` (après `initGui`, au lancement du noyau) l'évite, comme l'écrit AD-25. Un
fournisseur désactivé **en cours de session** fera échouer l'étape à l'exécution :
`run_algorithm` le traite déjà (`analysis.py:274-276`), conformément à AD-21. On pourrait l'écrire
en une phrase dans AD-25.

Le code actuel n'applique pas encore AD-25 : `register_all` (`analysis.py:573`) enregistre
toutes les lignes sans rien vérifier. C'est attendu (décision nouvelle), et cela fait un critère
pour le premier ticket de l'epic 10.

## 2. AD-24 : liste blanche et primitives de CAP-14

### 2a. Algorithmes natifs qui accèdent au réseau — **à corriger** (les nommer)

AD-24 dit « aucune ligne ne nomme un algorithme qui accède au réseau, ni le géocodeur intégré de
QGIS », sans les nommer. Ceux que la documentation officielle (QGIS latest) montre :

| Algorithme | Réseau | Source |
| --- | --- | --- |
| `native:filedownloader` (Download file via HTTP(S)) | oui | https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/filetools.html |
| `native:httprequest` (HTTP(S) POST/GET request) | oui | même page |
| `native:openurl` | indirect (ouvre le navigateur) | même page |
| `native:downloadvectortiles` | oui | https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/vectortiles.html |
| **`native:batchnominatimgeocoder`** (Batch Nominatim geocoder) | oui, service Nominatim de la fondation OpenStreetMap | https://docs.qgis.org/latest/en/docs/user_manual/processing_algs/qgis/vectorgeneral.html |

Correctif proposé : dans AD-24, nommer ces identifiants comme exemples interdits, et ajouter un
test de recette qui échoue si une ligne de la table commence par l'un d'eux. Le « géocodeur
intégré » n'est donc pas seulement `QgsNominatimGeocoder` (voir §3) : c'est aussi un algorithme
du cadre de traitement, celui qu'une table de primitives pourrait inclure par mégarde.

**Trou à signaler (AD-14 plutôt qu'AD-24)** : un algorithme local peut quand même toucher le
réseau à travers une couche distante du projet (WMS/WFS/XYZ). C'est le cas de l'export de mise
en page et d'atlas (`native:printlayouttopdf`, `native:atlaslayouttopdf`), qui dessine toutes les
couches visibles. La liste blanche ne protège pas de ça ; seul le test réseau d'AD-14 le peut.
À noter dans AD-24 ou dans les risques de l'epic 10.

### 2b. Les primitives de CAP-14 existent-elles en natif ? — **confirmé**

| Primitive | Identifiant vérifié | Source (docs.qgis.org, latest) |
| --- | --- | --- |
| Comptage de points par polygone, pondéré | `native:countpointsinpolygon`, paramètre facultatif `WEIGHT` (la somme des poids remplace le compte ; champ non numérique → 0) | vectoranalysis.html |
| Plus proche voisin | `native:joinbynearest` | vectorgeneral.html |
| Matrice de distances | `qgis:distancematrix` (préfixe `qgis:`, pas `native:`) | vectoranalysis.html |
| Plus court chemin | `native:shortestpathpointtopoint` (+ `…layertopoint`, `…pointtolayer`) | networkanalysis.html |
| Statistiques de raster par polygone | `native:zonalstatisticsfb` (« Append to layer » remplace l'ancienne variante en place) ; aussi `native:rasterlayerzonalstats` | rasteranalysis.html |
| Reprojection | `native:reprojectlayer` | vectorgeneral.html |
| Atlas | `native:atlaslayouttopdf` (un fichier) et `native:atlaslayouttomultiplepdf` | cartography.html |

Le dépôt utilise déjà `native:countpointsinpolygon` derrière `analysis.zonal_statistics`
(`analysis.py:164,221`) : le renommage d'AD-6 et des conventions correspond à ce que fait
réellement la commande. **Confirmé.**

### 2c. Préparation de réseau sans GRASS `v.clean` — **confirmé (natif suffisant)**

| Geste | Natif | Source |
| --- | --- | --- |
| Découpe aux croisements | `native:splitwithlines`, avec la même couche en entrée et en découpe : le code détecte `sameLayer` et saute l'entité elle-même | https://raw.githubusercontent.com/qgis/QGIS/master/src/analysis/processing/qgsalgorithmsplitwithlines.cpp |
| Accrochage | `native:snapgeometries`, accrochage dans la même couche documenté | vectorgeometry.html |
| Nettoyage | `native:deleteduplicategeometries`, `native:checkvalidity`, `native:explodelines` | vectorgeneral.html, vectorgeometry.html |
| Tolérance de connexion | paramètre `TOLERANCE` des algorithmes de réseau (« Two lines with nodes closer than the specified tolerance are considered connected ») ; `POINT_TOLERANCE` depuis 3.38, donc présent sous 3.44 | networkanalysis.html |

GRASS `v.clean` n'est donc pas requis. Une chaîne fixe `snapgeometries → splitwithlines →
deleteduplicategeometries` rentre dans la forme d'AD-24. **Non vérifié** : la doc ne dit pas
explicitement que le graphe de QGIS ne relie deux lignes qu'à un sommet commun. C'est le
comportement attendu, mais il est affirmé de mémoire. À confirmer par une sonde sous QGIS avant
de figer la chaîne.

## 3. AD-26 : format de la Base Adresse Nationale et géocodeur de QGIS

| Fait | Statut | Source |
| --- | --- | --- |
| Fichiers CSV par département, à `adresse.data.gouv.fr/data/ban/adresses/latest/csv` | **confirmé** | https://forge.inrae.fr/flood-appraisal/floodam/floodam-data/-/issues/22 ; jeu BAN sur data.gouv.fr — https://www.data.gouv.fr/datasets/base-adresse-nationale/ |
| Colonnes de coordonnées : `x`, `y` en **projection légale** ; `lon`, `lat` en WGS-84 | **confirmé** | Dictionnaire BAL sur data.gouv.fr (« x : Coordonnées cartographique en projection légale … lon : Longitude en WGS-84 ») — https://www.data.gouv.fr/datasets/base-adresse-locale-bal-2 ; https://docs.ideal-postcodes.co.uk/docs/data/ban.md (x/y en Lambert-93) |
| Séparateur `;` et encodage UTF-8 | **non vérifiable** | Aucune source consultée ne l'écrit, et le téléchargement direct d'un en-tête a échoué (réseau du bac à sable). À lire sur un vrai fichier avant le ticket |
| Le fichier contient aussi les autres colonnes d'appariement (`numero`, `rep`, `nom_voie`, `code_postal`, `code_insee`, `nom_commune`) | **confirmé en partie** | ideal-postcodes (champs d'un enregistrement BAN) ; l'ordre des colonnes n'est pas vérifié |

**À corriger : la cohérence de « CSV à coordonnées qui déclare son code EPSG ».** Le contrat
est cohérent si le noyau recopie **soit** `lon`/`lat` avec EPSG:4326, **soit** `x`/`y` avec un code
EPSG. Mais la « projection légale » de `x`/`y` **n'est pas une seule projection** : Lambert-93
(EPSG:2154) en métropole, des UTM locales outre-mer (Antilles, Guyane, Réunion, Mayotte). Les
codes outre-mer, cités de mémoire, ne sont pas vérifiés ici. Comme le noyau ne reprojette pas
(AD-19), il doit soit (a) écrire toujours `lon`/`lat` avec EPSG:4326, la seule colonne au système
unique, et laisser QGIS reprojeter, soit (b) déduire le code EPSG du département, ce qui fait
une table de plus dans le noyau. Correctif proposé : écrire l'option (a) dans AD-26 (« le CSV
écrit porte `lon`/`lat` en EPSG:4326 »), qui ferme la question sans table.

Autre point, mineur : AD-26 dit « tous deux en CSV ». Le fichier BAN est distribué compressé
(`.csv.gz`). Soit l'utilisateur le décompresse, soit le noyau lit le gzip, et il faut alors une
crate de plus, à valider avec sa licence (AD-16). Il faut le décider dans le ticket.

**Géocodeur intégré de QGIS** : **confirmé**. `QgsNominatimGeocoder`, ajouté en 3.18, « uses
the Nominatim geocoding API » ; la doc rappelle la politique d'usage du serveur de la fondation
OpenStreetMap — https://qgis.org/pyqgis/master/core/QgsNominatimGeocoder.html. Il est aussi
exposé dans le cadre de traitement par `native:batchnominatimgeocoder` (voir §2a). AD-24 gagnerait
à nommer les deux.

Contexte : selon une présentation du CNIG de juin 2026, l'ancienne URL de l'API publique de
géocodage a été arrêtée fin janvier 2026, au profit du service de la Géoplateforme IGN
(https://cnig.gouv.fr/IMG/pdf/cnig_gt_assurances_19062026.pdf). C'est sans effet sur AD-26, qui ne
touche pas le réseau. L'argument vaut en revanche pour la documentation « où trouver ses
données » : n'y citer que les fichiers de téléchargement.

## 4. AD-15 : l'écart du ticket 5.14, et la convention « Réglages »

| Fait | Statut | Source |
| --- | --- | --- |
| L'écart concorde avec le ticket 14 (`agent.models` hors registre et non journalisé, sans dossier ni chemin ; `plan.propose` accepte `model`, absent = premier ; `models.unknown` ; `model.list` reste au registre) | **confirmé** | `epic-agent-et-mcp/tickets.toml`, entrée 14 : description et note « Decision: 2026-10-18 — écart assumé à AD-15 … » |
| `docs/canal-local.md` décrit `agent.models` | **pas encore** (attendu) | `docs/canal-local.md` ne contient ni `agent.models`, ni le paramètre `model`, ni `models.unknown` (seule la ligne 199 parle de `models.*`). Aucun fichier de code ne contient `agent.models`. Le ticket 14 n'est pas commencé (pas de fichier de plan) et prévoit cette mise à jour de la doc. L'écart décrit donc un état à venir. C'est cohérent, mais il faut le savoir |
| `CASSINI_MODELS` est la variable lue pour le dossier de modèles | **confirmé** | `core/src/commands/model_list.rs:15` (`MODELS_ENV`), `:141` ; `cli/src/main.rs:165` |
| `CASSINI_MODELS` est la **seule** variable lue par le noyau | **à corriger** | `ai/src/lib.rs:373` lit `CASSINI_GPU_LAYERS` (seulement avec la feature `cuda`, pour les mesures) ; `CASSINI_TEST_MODEL`, `CASSINI_TEST_THREADS` et `CASSINI_TEST_INVENTORY` ne servent qu'aux tests |
| Côté extension, les réglages sont « sous le préfixe `cassini/` » | **à compléter** | Exact pour `cassini/core_path`, `cassini/lang`, `cassini/expert_open` (`locator.py:8`, `panel.py:115-116`). Mais l'extension lit aussi la variable d'environnement `CASSINI_CORE` (`locator.py:9`), dans l'ordre d'AD-17 |

Correctif proposé pour la ligne « Réglages » : « côté extension, les réglages QGIS sous le
préfixe `cassini/`, plus `CASSINI_CORE` dans l'ordre de recherche d'AD-17 ; côté noyau,
`CASSINI_MODELS` lue au démarrage, sans fichier de configuration (les variables
`CASSINI_GPU_LAYERS` et `CASSINI_TEST_*` ne servent qu'aux mesures et aux tests) ».

Point voisin, à vérifier par la lentille adverse : `model.list` reste au registre et rend
`folder`, un **chemin absolu** (`model_list.rs:33`). Par `cassini run`, il serait journalisé ; par
MCP, il serait offert à un client tiers. Cela heurte « jamais d'absolu » (AD-9) et contredit en
partie la raison même de l'écart (la note du ticket écarte `command.run model.list` parce qu'il
« rend le dossier absolu »). À trancher : ne pas offrir `model.list` à MCP ni au modèle, ou rendre
`folder` relatif ou masqué.

## 5. Précision d'AD-6 : l'énumération des méthodes — **à corriger**

Méthodes réellement présentes dans `cli/src`, `core/src`, `qgis-plugin/cassini_qgis` et
`docs/canal-local.md` : `session.hello`, `order.execute`, `request.cancel`, `journal.replay`,
`plan.propose`, `plan.execute`, `plan.reject`, la notification `plan.step`, et **`command.run`**
(15 occurrences ; AD-15 elle-même la cite). La liste d'AD-6 (« `session.*`, `plan.*`,
`order.execute`, `journal.replay`, `request.cancel`, `agent.models` ») **oublie `command.run`**.

Correctif : ajouter `command.run` en précisant son statut. C'est une méthode de transport qui
**porte** une commande du registre, journalisée, et pas une action en soi. Sans cette précision,
la phrase « toute action sur le projet ou sur les données est une commande du registre ; aucune ne
s'ajoute comme méthode du canal » paraît contredite par `command.run` lui-même.

## 6. Renommage `analysis.zonal_statistics` → `analysis.count_points_in_polygon` : étendue

Comptage `grep -c zonal_statistics`, hors `target/` et `.git/` :

**Code et tests (à changer, sinon la recette casse)**
- `core/src/commands/analysis_zonal_statistics.rs` — 19 (id, quatre noms de tests ; fichier à renommer, plus sa déclaration de module)
- `qgis-plugin/cassini_qgis/analysis.py` — 6 (clé de table, `translate_`/`summarize_zonal_statistics`)
- `qgis-plugin/tests/test_analysis.py` — 3
- `qgis-plugin/tests/probe_zonal_statistics.py` — 4 (sonde manuelle, à renommer)
- `qgis-plugin/README.md` — 3 (recette manuelle du ticket 10)

**Jeu de l'épreuve du modèle (à décider)**
- `tests/epreuve-modele-local/catalog.json` — 1 ; `sentences.json` — 3 (S15, S23, S24) ; `reading_independent.json` — 1
- `resultats-8-4.json` (3), `resultats-8-5.json` (15), `resultats-8-6-cpu.json` (4) : résultats historiques, à laisser tels quels

À noter : S15, S23 et S24 (« altitude moyenne par commune », « moyenne de pente par commune »)
attendent en réalité des **statistiques de raster** par polygone. Ces trois phrases doivent donc
pointer vers `raster.zonal_statistics`, et non vers `analysis.count_points_in_polygon`. Le
renommage n'est pas mécanique dans ce jeu.

**Documents BMAD (historique, à laisser ou à annoter)** : `epic-facade-qgis/` (plans des tickets
de statistiques par zone, des isochrones et de l'export PDF, `tickets.toml`, rétrospective),
`deferred-work.md` (2), le verdict et le plan de l'épreuve, le plan de clarification de l'epic
agent, les `.memlog.md`.

**Journaux existants** : `../donnees-test-cassini/projet-test-villeneuve.cassini/journal.jsonl`
ne contient pas l'id (0 occurrence). Le rejeu d'un journal plus ancien qui le contiendrait
échouerait en nommant la commande (AD-11), au lieu de la sauter. Il faut l'accepter dans le
ticket, ou garder un alias de lecture.

Le prompt de `ai/src` ne contient pas l'id (il vient du registre) : rien à changer à la main de
ce côté.
