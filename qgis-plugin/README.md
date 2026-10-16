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

Contrôle manuel : charger l'extension, ouvrir un projet, voir les deux versions,
fermer QGIS et vérifier qu'aucun `cassini` ne reste dans le gestionnaire des
tâches.
