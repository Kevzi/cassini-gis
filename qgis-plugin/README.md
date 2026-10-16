# Extension Cassini pour QGIS (traceur)

Licence GPL, comme QGIS. L'extension ne contient aucun binaire : le noyau
`cassini` s'installe à part, avec le paquet Cassini. Rien n'est téléchargé.

## Installation manuelle depuis le dépôt

1. Dans QGIS (3.44 ou plus), ouvrez *Extensions > Installer/Gérer* puis repérez le
   dossier des extensions : *Préférences > Profils utilisateur > Ouvrir le dossier
   du profil actif*, sous-dossier `python/plugins`.
2. Copiez (ou liez) le dossier `qgis-plugin/cassini_qgis` dans ce dossier.
3. Redémarrez QGIS, puis activez *Cassini* dans *Extensions > Installer/Gérer*.
4. Ouvrez ou enregistrez un projet : le panneau Cassini affiche la version du
   noyau et celle du protocole.

## Où l'extension cherche le noyau (dans cet ordre)

1. le réglage QGIS `cassini/core_path` (rempli par le bouton « Choisir le noyau »
   du panneau) ;
2. la variable d'environnement `CASSINI_CORE` ;
3. l'emplacement d'installation : `%LOCALAPPDATA%\Cassini\bin\cassini.exe`
   (Windows) ou `~/.local/share/cassini/bin/cassini` (Linux) ;
4. le `PATH`.

## Tests

Sans QGIS : `python -I -m unittest discover -s qgis-plugin/tests`.

Sonde d'annulation sous le vrai QGIS (jeu hors dépôt `../donnees-test-cassini/`) :
`python-qgis-ltr.bat qgis-plugin/tests/probe_cancel.py` -> `RÉSULTAT : OK`.

Garde statique (imports réseau et PyQt directs interdits, taille, versions de
`metadata.txt`) : `python -I qgis-plugin/tests/check_extension.py`.

Recette manuelle du panneau [9.4] (dans QGIS, avec un noyau installé) :

1. Ouvrez un projet : le logo et la saisie grisée (« Disponible avec l'agent. »)
   sont visibles ; « Démarrage du noyau… » puis le bandeau disparaît.
2. Dépliez « Mode expert », choisissez `system.about`, cliquez sur Lancer : le
   résultat dit que le noyau répond, le journal porte une ligne « Terminée ».
3. L'arrêt d'une commande n'est pas testable à la main : le binaire réel ne
   contient que `system.about`, les commandes `test.*` n'existant qu'aux tests
   Rust. `request.cancel`, la survie du noyau et le retour de Lancer sont
   couverts par `qgis-plugin/tests/test_tracer.py`.
4. Réglages (engrenage) : basculez FR / EN, les textes changent aussitôt.
5. Sans noyau installé : le bandeau propose « Choisir le noyau ».

Recette manuelle de `layer.add` [9.5] (dans QGIS, avec un noyau qui annonce la
commande) :

1. Enregistrez un projet à côté de trois fichiers : un `.gpkg`, un `.shp` et un
   `.geojson` (le jeu `fixtures/parcelles-test.geojson` convient).
2. Panneau, mode expert, commande `layer.add`, champ `path` : le chemin du fichier,
   relatif au projet. Lancez : la couche apparaît dans le projet et le résultat
   donne son SCR et si QGIS la reprojette. Répétez pour chaque format.
3. Le fichier reste intact (taille et date inchangées). Un `.txt` donne
   `layer.unsupported_format`, un fichier abîmé `layer.invalid_source`.
4. Un Shapefile sans `.prj` donne `layer.unknown_crs` : relancez avec le champ `crs`
   (ex. `EPSG:2154`). Le SCR est donné à la couche en mémoire, le fichier reste
   intact. Un `crs` qui contredit celui du fichier donne `layer.crs_conflict`.

Recette manuelle du CSV et du GeoParquet [9.6] (jeu `../donnees-test-cassini/`) :

1. `gares-2154.csv` avec `crs` = `EPSG:2154` : 8 points apparaissent (colonnes `X`,
   `Y` par défaut). Sans `crs` : `layer.unknown_crs`, rien n'est ajouté.
2. `villes.csv` avec `crs` = `EPSG:4326` ; pour d'autres noms de colonnes, remplissez
   `x_field` et `y_field`.
3. `villes.parquet` : la couche apparaît avec son SCR. Si le pilote Parquet manque
   dans votre QGIS, `layer.invalid_source`.
4. Les fichiers restent intacts (taille et date inchangées).

Recette manuelle de `project.inventory` et `layer.describe` (jeu
`../donnees-test-cassini/`, noyau qui annonce les commandes) :

1. Projet de test à deux couches, enregistré : mode expert, `project.inventory`
   (aucun paramètre). Le résultat liste les couches dans l'ordre de la légende (haut
   en bas), le SCR du projet et l'étendue d'ensemble ; chaque source est un nom de
   fichier, sans dossier.
2. Projet vide : `layers` est vide et `extent` est `null`, sans erreur.
3. `layer.describe` avec le `layer_id` d'une couche (celui de `project.inventory` ou
   de `layer.add`) : champs et types, étendue, SCR, nombre d'objets et géométrie,
   cohérents avec la table d'attributs. Un id inconnu donne `layer.not_found`, une
   couche raster `layer.not_vector`.
4. Aucune couche ni aucun fichier n'est modifié. Portabilité : copiez le projet avec
   son dossier `<projet>.cassini/` ailleurs ; le journal s'y rejoue à l'identique
   (couvert par `cargo test`, test du journal).

Recette manuelle de `analysis.buffer` [ticket 8] (jeu `../donnees-test-cassini/`) :

1. Ouvrez `gares-2154.csv` avec `layer.add` (`crs` = `EPSG:2154`), puis choisissez
   `analysis.buffer` : la couche se choisit dans la liste, aucun champ `out` n'apparaît.
2. `distance` = `{"value": 500, "unit": "m"}` : une couche de 8 entités apparaît autour
   des gares ; le fichier `.gpkg` est dans `<projet>.cassini/`. `{"value": 0.5, "unit":
   "km"}` donne le même résultat ; `dissolve` à vrai donne 1 entité.
3. Une couche en EPSG:4326 donne `analysis.crs_not_metric` (reprojetez-la d'abord) ; un
   raster donne `layer.not_vector`.
4. Lancez puis arrêtez : `qgis.cancelled`, aucun fichier ne reste dans `<projet>.cassini/`.

Recette manuelle de `analysis.intersect` et `analysis.select_by_attribute` [ticket 9]
(jeu `../donnees-test-cassini/`) :

1. `gares-2154.csv` ouvert (`layer.add`, `crs` = `EPSG:2154`) et sa zone tampon de 500 m
   (`analysis.buffer`). Choisissez `analysis.intersect` : `layer_id` et
   `overlay_layer_id` sont deux listes de couches. Gares en première, zone tampon en
   seconde : une couche de 8 entités apparaît. Les deux mêmes couches donnent
   `analysis.bad_params`.
2. `analysis.select_by_attribute` : `layer_id` est une liste de couches, `field` la liste
   des champs de la couche choisie (elle change avec la couche), `operator` une liste
   (`eq`, `ne`, `lt`, `lte`, `gt`, `gte`, `contains`, `begins_with`). `name` +
   `begins_with` + `A` rend une couche des gares dont le nom commence par A ; une valeur
   sans correspondance rend une couche vide (0 entité), sans erreur.
3. Sonde sous le vrai QGIS : `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat"
   qgis-plugin/tests/probe_intersect_select.py` doit finir par `RÉSULTAT : OK`.

Recette manuelle de `analysis.zonal_statistics` [ticket 10] (jeu `../donnees-test-cassini/`) :

1. Une couche de polygones (communes ou secteurs) et `gares-2154.csv` ouverts. Choisissez
   `analysis.zonal_statistics` : `layer_id` (zones) et `points_layer_id` (points) sont deux
   listes de couches ; `label_field` est une liste des champs de la couche de zones, avec
   une entrée vide (zone alors nommée par son numéro d'entité).
2. Lancez : une couche des zones avec un champ `NUMPOINTS` apparaît, et `zones` donne les
   décomptes (0 pour une zone sans gare). Au-delà de 100 zones, `zones` en garde 100 et
   `truncated` est vrai, la couche les garde toutes.
3. Les deux mêmes couches, ou des points donnés comme zones, donnent `analysis.bad_params`.
4. Sonde sous le vrai QGIS : `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat"
   qgis-plugin/tests/probe_zonal_statistics.py` doit finir par `RÉSULTAT : OK`.

Contrôle manuel : charger l'extension, ouvrir un projet, voir les deux versions,
fermer QGIS et vérifier qu'aucun `cassini` ne reste dans le gestionnaire des
tâches.
