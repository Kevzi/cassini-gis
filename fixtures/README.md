# Jeux de données de test

Cassini ne distribue aucune donnée géographique (CLAUDE.md §2.4). Ce dossier ne
contient donc **que** des jeux de test légers, fabriqués pour la recette :

- assez petits pour être versionnés sans alourdir le dépôt ;
- soit synthétiques, soit issus d'une source sous licence ouverte explicitement
  citée dans ce fichier ;
- jamais une donnée réelle de production, et jamais un livrable d'utilisateur.

Le `.gitignore` du dépôt ignore les formats de données lourds partout ailleurs,
avec une exception pour ce dossier.

| Fichier | Nature | Origine | Utilisé par |
|---|---|---|---|
| `basemap-villeneuve-le-roi.pmtiles` | Archive PMTiles v3, tuiles PNG en niveaux de gris, 35 tuiles, 50,3 Ko | **Généré**, aucun téléchargement : `app/scripts/generate-basemap.mjs` | Le fond de carte de l'application, et la recette |
| `points-2154.csv` | 4 points en Lambert-93 (colonnes `X`, `Y`, `name`), sans SCR dans le fichier | **Synthétique**, écrit à la main (valeurs inventées) | La sonde de bout en bout : `layer.add` avec `crs`, puis `analysis.buffer`, qui exige un SCR métrique |
| `reseau-grille-2154.geojson` | Grille de rues en Lambert-93 : 220 tronçons de 100 m (11 rues dans chaque sens), carré de 1 km de côté d'origine (652000, 6850000), sans nom de rue réel | **Synthétique**, calculé (aucune donnée réelle) | La sonde `probe_isochrone.py` et la recette de `analysis.isochrone` : origine au centre (652500, 6850500), 3 min à pied donnent un polygone d'emprise 500 m sur 500 m |
| `parcelles-test.geojson` | Une parcelle carrée en WGS84, 1 entité | **Synthétique**, écrit à la main | Les tests du geste `layer.add` et sa recette |
| `points-2154.geojson` | 3 points en Lambert-93 autour du sommet de la colline (652250, 6850250), GeoJSON de 2008 qui déclare `EPSG:2154` par son membre `crs`, propriétés `rank`, `elevation`, `prominence` | **Synthétique**, écrit par l'écrivain unique du noyau (`core/src/geojson.rs`) et régénéré par `geojson::tests::two_writes_are_identical_and_equal_to_the_fixture` (même variable `CASSINI_REGENERER_FIXTURES=1`) | L'écrivain GeoJSON et l'ouverture dictée `layer.open_vector` (11.8), la sonde `probe_vector_output.py` |
| `mnt-plan-incline-2154.tif` | MNT GeoTIFF Float32 deflate, EPSG:2154, 40 × 30 cellules de 10 m, coin haut-gauche (652000, 6850300), sans-donnée -9999 : plan de pente 10 % descendant vers l'est, altitude `99,5 - colonne` au centre de chaque cellule | **Synthétique**, calculé et écrit par `Dem::write_geotiff` (`core/src/dem.rs`) | La lecture des MNT (entrée 4.1), `terrain.slope` (4.2), puis les autres commandes de terrain (4.3 à 4.5, 10.20) |
| `mnt-colline-2154.tif` | MNT GeoTIFF Float32 deflate, EPSG:2154, 50 × 50 cellules de 10 m, coin haut-gauche (652000, 6850500), sans-donnée -9999 : colline lisse `50 + 100 / (1 + d² / 150²)`, sommet au centre (652250, 6850250) | **Synthétique**, calculé et écrit par `Dem::write_geotiff` | Idem, dont `terrain.slope` (4.2), `terrain.line_of_sight` (4.4) et `terrain.viewshed` (4.5) |
| `mnt-colline-sans-systeme.asc` | ASCII Grid (ESRI), 12 × 10 cellules de 25 m, coin bas-gauche (945000, 6535000), **sans `.prj`** (aucun système), sans-donnée -99999 sur trois cellules : colline `450 + 120·exp(−d²/12)`, d en cellules depuis (6, 5) | **Synthétique**, écrit par un script Python de quelques lignes | `data.convert_raster` (epic 10, entrée 30) : déclaration du système, réécriture du sans-donnée en -9999, sonde `probe_convert_raster.py` |
| `style-cases.json` | Styles de couche (`StyleSpec`) admis et refusés, chacun avec le code attendu ; aucune donnée géographique | **Synthétique**, écrit à la main | Les deux côtés de la vérification des styles : `core/src/style/spec.rs` (`the_shared_style_cases_agree_with_the_extension`) et `qgis-plugin/tests/test_style_gestures.py` |

## Les MNT de recette

Les deux MNT sont **régénérés à chaque passage des tests** de `cassini-core`
(`dem::tests::fixtures_regenerate_identically`) et comparés, octet pour octet, aux
fichiers versionnés : c'est la preuve que l'écriture est déterministe. Le calcul
n'emploie que des opérations exactement arrondies (addition, multiplication,
division), pour donner les mêmes octets sous Linux et sous Windows. Pour les
réécrire après un changement voulu :

```bash
CASSINI_REGENERER_FIXTURES=1 cargo test -p cassini-core dem::tests::fixtures_regenerate_identically
```

## Le fond de carte de recette

C'est un damier, pas une carte. Il sert à prouver trois choses : que l'archive
est conforme à la spécification PMTiles v3, que MapLibre sait l'afficher depuis
un fichier local, et qu'aucune requête réseau n'est nécessaire pour voir une
carte à l'écran.

- **Emprise** : `2.37, 48.71` → `2.46, 48.76`, soit la commune de
  Villeneuve-le-Roi, la même boîte que le repère de `app/src/basemap.style.json`.
- **Zooms** : 10 à 14. En dehors, l'archive est vide — c'est assumé. La carte
  s'ouvre au zoom 12 sur `2.415, 48.735`.
- **Hors de l'emprise** : aucune tuile. Un lecteur doit répondre « pas de
  tuile », pas planter.

### D'où l'application le lit

L'archive est **embarquée dans l'installeur** : la carte affiche un fond dès la
première ouverture, sans aucune manipulation. C'est un damier fabriqué par le
dépôt, sans la moindre donnée géographique tierce, donc rien n'est redistribué
(CLAUDE.md §2.4).

1. `app/src-tauri/tauri.conf.json`, clé `bundle.resources`, copie ce fichier dans
   l'installeur sous `fixtures/basemap-villeneuve-le-roi.pmtiles`.
2. `app/src-tauri/src/lib.rs` le cherche d'abord dans le répertoire de ressources
   de l'application installée, puis — en développement, où `cargo tauri dev` ne
   passe pas par l'installeur — dans le fichier de ce dossier.
3. Le chemin est validé par `BasemapSource::from_local_path`, puis lu par
   plages d'octets avec `BasemapSource::read_range`. **L'interface ne connaît
   jamais le chemin** : elle demande des octets par l'IPC Tauri, et la carte les
   reçoit par le protocole `pmtiles://cassini-basemap-fixture`. Aucune requête
   réseau, aucune origine HTTP ajoutée au CSP de la fenêtre.

`app/scripts/verify-protocol.mjs` vérifie que ces endroits s'accordent : la clé
du style est celle du code, le chemin du moteur est celui de l'installeur, et la
commande appelée par l'interface est celle qu'enregistre le moteur.

### Régénérer

```bash
cd app
npm run fixtures          # réécrit le .pmtiles
npm run verify:fixtures   # le relit avec la bibliothèque pmtiles et le valide
```

La génération est **déterministe** : deux exécutions produisent le même fichier,
octet pour octet, ce que la vérification contrôle. `npm run check` lance la
vérification, donc une archive cassée fait échouer la recette.

### Ce que la vérification contrôle

30 contrôles, dont : la magie et la version de l'en-tête, le type de tuile, les
compressions, les bornes de zoom et l'emprise annoncées, la relecture des
métadonnées, la concordance de notre calcul d'identifiant de tuile avec celui de
la bibliothèque sur les 35 tuiles, la valeur de référence donnée par la
spécification (`z12/3423/1763 → 19078479`), la validité PNG de chaque tuile avec
ses CRC, l'absence de tuile hors emprise et hors zooms, le déterminisme, la
concordance de l'emprise avec le repère lu dans `app/src/basemap.style.json`, le
centre et le zoom d'ouverture annoncés, et enfin **quatre contrôles qui mettent
le validateur en échec exprès** — octet corrompu, tuile tronquée, signature
absente — parce qu'un garde-fou qu'on n'a jamais vu refuser quelque chose ne
garde rien.

`npm run verify:protocol` ajoute 24 contrôles sur le chemin de chargement de la
carte : le TileJSON construit depuis l'en-tête, une tuile connue servie en PNG,
une tuile hors emprise ou hors zooms qui revient vide sans exception, et la
preuve qu'une clé non enregistrée part bien sur le réseau — c'est ce que la garde
de `check-offline.mjs` existe pour empêcher. Le refus d'une plage hors fichier,
lui, est contrôlé là où il est écrit : les tests de `cassini-core`.
