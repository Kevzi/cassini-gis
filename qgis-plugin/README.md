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

Sonde de bout en bout sous le vrai QGIS et le vrai binaire (fixture synthétique
`fixtures/points-2154.csv` et `fixtures/reseau-grille-2154.geojson`, aucune donnée hors dépôt) : `cargo build --release` puis
`python-qgis-ltr.bat qgis-plugin/tests/probe_end_to_end.py` -> `RÉSULTAT : OK`. Elle
enchaîne `layer.add` (points et réseau), `analysis.buffer`, `analysis.select_by_attribute`,
`analysis.intersect`, `analysis.isochrone` (à pied, puis en voiture) et `layout.export_pdf`, ferme et rouvre le projet,
puis `replay()` demande au noyau `journal.replay` : toutes les étapes sont refaites,
et les sorties des analyses vont sous `replay/`. Le journal garde le même nombre
d'étapes après le rejeu. Le démarrage n'y ajoute rien : l'extension lit les versions
du noyau dans la réponse de `session.hello`. Le CSV et le réseau d'origine sont
inchangés (empreinte). Enfin, un résultat du journal est altéré et le rejeu s'arrête
en nommant l'étape. Il n'y a pas de
bouton de rejeu dans le panneau : il viendra avec l'agent (epic 5).

Garde statique (imports réseau et PyQt directs interdits, taille, versions de
`metadata.txt`) : `python -I qgis-plugin/tests/check_extension.py`.

Recette manuelle du panneau [9.4] (dans QGIS, avec un noyau installé) :

1. Ouvrez un projet : le logo et la saisie grisée sont visibles ; « Démarrage du
   noyau… » puis le bandeau disparaît et la saisie devient active [ticket 6].
2. Dépliez « Mode expert », choisissez `system.about`, cliquez sur Lancer : le
   résultat dit que le noyau répond, le journal porte une ligne « Terminée ».
3. L'arrêt d'une commande n'est pas testable à la main : le binaire réel ne
   contient que `system.about`, les commandes `test.*` n'existant qu'aux tests
   Rust. `request.cancel`, la survie du noyau et le retour de Lancer sont
   couverts par `qgis-plugin/tests/test_tracer.py`.
4. Réglages (engrenage) : basculez FR / EN, les textes changent aussitôt.
5. Sans noyau installé : le bandeau propose « Choisir le noyau ».
6. Ouvrez le même projet dans une seconde fenêtre QGIS. La réponse de
   `session.hello` dit que la session est en lecture seule (`read_only`). Le
   panneau de la seconde fenêtre affiche le message du noyau et ne se dit pas prêt.

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

Recette manuelle de `analysis.isochrone` [ticket 11] (réseau apporté par l'utilisateur ;
`fixtures/reseau-grille-2154.geojson` pour l'essai) :

1. Ouvrez la grille de rues (couche de lignes en Lambert-93). Choisissez `analysis.isochrone` :
   `layer_id` est une liste de couches, `mode` une liste (`walk`, `car`), `origin` et
   `duration` se saisissent en `{x, y, epsg}` et `{value, unit}` ; aucun champ `out`.
2. Origine `{652500, 6850500, 2154}`, `walk`, 3 min : une couche d'un polygone apparaît,
   losange dont l'emprise fait environ 500 m sur 500 m autour de l'origine. En `car`, le polygone est plus grand
   (toute la grille). `{180, "s"}` donne le même résultat que 3 min.
3. Une couche de points ou de polygones : `analysis.bad_params` (choisir une couche de
   lignes). Un réseau en degrés : `analysis.crs_not_metric`. Une origine à plus de 100 m du
   réseau : `analysis.origin_off_network`. Aucun fichier dans ces trois cas.
4. L'enveloppe convexe surestime la zone sur un réseau en étoile ou coupé par une rivière
   (limite assumée au MVP).
5. Sonde sous le vrai QGIS : `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat"
   qgis-plugin/tests/probe_isochrone.py` doit finir par `RÉSULTAT : OK`.
6. Mesure sur un vrai réseau routier hors dépôt (extrait OpenStreetMap de la commune de
   Villeneuve-le-Roi, 11 699 tronçons, Lambert-93, `../donnees-test-cassini/`), origine à
   une gare (13,9 m du réseau), 2026-10-17, QGIS 3.44.15. `walk` 10 min : un polygone,
   emprise d'environ 1,3 km sur 1,5 km, 1,4 km² (un disque de 833 m ferait 2,2 km²), pour
   environ 40 km de tronçons atteints. `car` 5 min : un polygone, emprise d'environ 6 km sur
   6 km, 26 km² pour environ 1 080 km de tronçons atteints. Trois origines (trois gares)
   donnent des résultats du même ordre. Calcul d'environ 2 s. Une tolérance topologique de 0, 1, 2, 5 ou 10 m change l'aire de
   l'enveloppe de moins de 2 % pour la gare principale (marche comme voiture) et de 3 % à 9 %
   pour la gare de triage (marche, 0 m contre 5 m ; zone de voies ferrées où des nœuds ne se
   rejoignent pas) : aucun écart ne rend le polygone trop petit ni troué, l'enveloppe étant
   convexe. `TOLERANCE: 0` est conservée ; un gain de quelques pour cent ne justifie pas
   de lier des tronçons qui ne se touchent pas. Un réseau aux nœuds franchement décalés
   reste à éprouver. L'origine doit être à moins de 100 m du
   réseau : les 8 gares de l'extrait en sont à 2 m à 17 m.

Recette manuelle de `layout.export_pdf` [ticket 12] (jeu `../donnees-test-cassini/`) :

1. Ouvrez `projet-test-villeneuve.qgz`. Choisissez `layout.export_pdf` : les champs sont
   `title` et, facultatif, `scale` (le fichier `out` est nommé par le noyau, le panneau ne
   l'affiche pas).
2. Lancez avec un titre : un PDF A4 paysage apparaît dans le dossier de travail ; ouvert
   dans un lecteur, il montre le titre, la carte des couches visibles, la légende, la barre
   d'échelle, la flèche du nord et la ligne « Sources » (noms de fichiers, jamais de chemin).
3. Décochez toutes les couches puis relancez : `layout.empty`, aucun fichier. Un titre blanc
   donne `layout.bad_params`. Le projet ne gagne aucune mise en page.
4. Échelle [ticket 5.38] : déplacez la carte de QGIS sur un quartier, puis lancez avec
   `scale` 25000. Le PDF est centré sur ce que montre la carte de QGIS, la barre d'échelle
   correspond au 1:25 000 et le journal dit « exportée en PDF au 1:25 000 ». `scale` 99 ou
   100000001 donne `registry.invalid_params`, sans fichier. Sans `scale`, le cadrage reste
   l'emprise des couches visibles. Imprimé à 100 %, 1 cm sur la carte vaut 250 m.
   Le journal garde le centre employé (`center`, dans le système du projet) : déplacez la
   carte de QGIS ailleurs puis rejouez, le PDF rejoué montre le même endroit et le rejeu
   reste `identical: true`. Le champ `center` n'est rempli que par le noyau, au rejeu : le
   formulaire ne le montre pas. Le projet doit avoir un système à code EPSG, sinon
   `layout.bad_params`.
5. Cadre vide : déplacez la carte de QGIS loin de toute donnée, puis lancez avec `scale`
   25000 : `layout.empty` (« déplacez la vue de QGIS sur les données »), aucun fichier.
6. Sonde sous le vrai QGIS : `"C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat"
   qgis-plugin/tests/probe_layout_export.py` doit finir par `RÉSULTAT : OK` (elle exporte
   aussi au 1:25 000 et relit l'échelle, repasse le centre en EPSG:4326 pour éprouver la
   reprojection réelle, et vérifie qu'un cadre vide est refusé).

Recette manuelle de la saisie en langage naturel et du plan [ticket 6] (QGIS 3.44, jeu
`../donnees-test-cassini/`, un modèle GGUF dans le dossier des modèles : sans choix
dans Réglages, le premier utilisable est pris) :

1. Ouvrez `projet-test-villeneuve.qgz` : la saisie est active. Écrivez la phrase de démo
   « Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à
   Villeneuve-le-Roi et sors-moi la carte en PDF. » ; Maj+Entrée saute une ligne, Entrée
   envoie. Pendant la génération, la barre d'attente et « Je prépare un plan… » s'affichent,
   la saisie et le mode expert sont grisés, aucun bouton Arrêter (la composition n'est pas
   annulable). La première demande peut prendre plusieurs minutes.
2. « Voici ce que je propose. » et les étapes numérotées (description du registre, « En
   attente ») ; « Détails techniques » montre les paramètres, sans chemin absolu.
3. Valider le plan : le marqueur ▶ suit l'étape en cours, le journal ajoute une ligne par
   étape, chacune finit en « Terminée » ; le résultat dit « Plan exécuté : n étapes
   terminées. » et la saisie est rendue.
4. Nouvelle demande, puis Refuser : « Plan refusé. Reformulez votre demande. », saisie rendue.
5. Une demande qui nomme une couche absente (« les pharmacies ») : la question du modèle
   s'affiche comme une réplique, sans Valider ; on répond par une nouvelle demande.
6. Arrêter : pendant une étape annulable (une analyse longue), Arrêter envoie
   `request.cancel` ; l'étape et sa ligne passent à « Annulée ». Pendant une étape non
   annulable, Arrêter est grisé avec « Arrêt impossible : cette étape ne peut pas être
   interrompue. »
7. Sans modèle dans le dossier : « La demande n'a pas abouti (ai.no_model) : … », saisie rendue.

Recette manuelle du choix du modèle [ticket 14] (QGIS 3.44, même jeu, deux modèles dans
le dossier des modèles, par exemple `Qwen3.5-9B-Q4_K_M.gguf` et `ajuste.modele.json`) :

1. Réglages (engrenage) : sous « Modèle », la liste montre chaque modèle par son nom de
   fichier et sa taille (« Qwen3.5-9B-Q4_K_M.gguf (5,6 Go) »), jamais un dossier ni un
   chemin. Sans choix mémorisé, le premier utilisable est sélectionné.
2. Choisissez le second, fermez les Réglages, envoyez une demande : le plan est composé par
   ce modèle (la première demande après un changement recharge le modèle, donc plus lente).
3. Fermez puis rouvrez QGIS : Réglages montre toujours le second modèle, et la demande part
   avec lui.
4. Retirez le modèle choisi du dossier, puis ouvrez Réglages : le choix est oublié, le premier
   modèle restant est sélectionné.
5. Une déclaration illisible ou qui nomme un GGUF absent est listée grisée, avec sa raison
   (« … — indisponible : fichier manquant : base.gguf ») ; on ne peut pas la choisir.
6. Dossier des modèles vide : liste grisée, « Aucun modèle utilisable dans le dossier des
   modèles. » ; `CASSINI_MODELS` vers un dossier absent : liste grisée, message du noyau et
   code `models.no_folder`, le dossier remplacé par « (dossier des modèles) ».
7. Réglages, English : le libellé devient « Model », les tailles « 5.6 GB ».

Recette manuelle du journal explicite [ticket 13] (QGIS 3.44, même jeu, même modèle) :

1. Phrase de démo, puis Valider le plan. Le résultat dit « Plan exécuté. Étapes
   terminées : n. », suivi d'une phrase par étape, par exemple « 2. Zone tampon de 500 m
   autour de Gares : nouvelle couche Tampon, 8 entités. » et, pour l'export, « Carte « … »
   exportée en PDF : fichier 4-layout.export_pdf.pdf, 1 page. » : des noms de couches,
   jamais un identifiant QGIS, un code ni un chemin absolu.
2. Cliquez sur chaque ligne du journal : la zone Résultat affiche la phrase de cette
   étape ; « Détails techniques » montre ses paramètres et son résultat bruts, chemins
   masqués (`…/nom`). Une référence à une étape précédente s'affiche par le nom de la
   couche qu'elle a rendue.
3. Cliquez sur une étape du plan : même phrase. Entrée sur une ligne choisie au clavier
   fait de même.
4. Plan en erreur (par exemple une couche retirée du projet entre la proposition et la
   validation) : un clic sur une étape finie donne sa phrase, un clic sur l'étape fautive
   donne « Étape n en erreur (code) : message », une étape jamais lancée donne son libellé
   et « En attente ».
5. Mode expert : lancez `analysis.buffer` ; le résultat donne la même phrase qu'au clic
   sur sa ligne du journal. Tout nouvel événement (une autre commande, une demande)
   efface le détail cliqué et remet le dernier résultat.
6. Réglages, English : la phrase affichée passe en anglais (« Buffer of 500 m around
   Gares: new layer Tampon, 8 features. »), le détail cliqué reste affiché.

Recette manuelle de l'IA externe [ticket 5.26] (QGIS 3.44, même jeu, `cassini mcp` branché
sur un client MCP, par exemple ChatGPT pour ordinateur) :

1. Réglages : « Autoriser une IA externe » est décoché à chaque ouverture de QGIS. Cochez-le :
   le bandeau « Une IA externe peut agir sur ce projet. » apparaît avec **Couper**.
2. Depuis le client MCP, demandez la description d'une couche : une ligne « IA externe ·
   Décrit une couche. » passe de « En cours… » à « Terminée », avec sa durée ; un clic dessus
   montre la commande, ses paramètres et son résultat, sans chemin absolu. Pendant l'appel, la
   saisie, Envoyer, Lancer, Valider le plan et Refuser sont grisés : « L'IA externe
   travaille. »
3. Lancez par l'IA externe une analyse longue : **Arrêter** dans le bandeau l'annule (ligne
   « Annulée ») ; sur une commande non annulable, Arrêter est grisé avec sa raison.
4. Cochez « Me demander avant chaque appel » : chaque appel affiche une question avec
   **Autoriser** / **Refuser**, sans fenêtre. Refuser : l'IA externe reçoit un refus, rien n'est
   exécuté. Sans réponse pendant deux minutes, la question disparaît (refus). La case est
   retenue après redémarrage de QGIS, l'interrupteur non.
5. Ouvrez le même utilisateur dans une seconde session QGIS et cochez l'interrupteur : il reste
   éteint et grisé, « Une autre session QGIS accepte déjà l'IA externe. » ; il redevient
   cliquable à la prochaine ouverture des Réglages.
6. **Couper** : le bandeau disparaît, le client MCP reçoit une erreur claire. Relancez le noyau
   pendant un appel : l'interrupteur est éteint, la ligne en cours passe à « Annulée ».

Contrôle manuel : charger l'extension, ouvrir un projet, voir les deux versions,
fermer QGIS et vérifier qu'aucun `cassini` ne reste dans le gestionnaire des
tâches.
