# Changement de cap : compléments de géométrie, conversion d'un raster et mise en page (2026-11-10)

Statut : **approuvé dans son rangement par le propriétaire le 2026-11-10 ; détail approuvé en
party, à confirmer.** Mode batch (`bmad-correct-course`). Toutes les décisions de détail
marquées « tranchée en party le 2026-11-10, à confirmer par le propriétaire » sont réversibles
jusqu'à la construction de leur entrée.

## 1. Le problème

- **Déclencheur :** demande du propriétaire du 2026-11-10 : six besoins d'usagers civils que le
  catalogue figé ne couvre pas, à ranger après la démo.
- **Nature :** nouvelles exigences, sans retour sur ce qui est construit.
- **Pourquoi un changement de cap :** le catalogue est figé (`catalogue-des-commandes.md` §1,
  changement de cap du 2026-11-05) ; un paramètre ou un champ de résultat ne s'ajoute à une
  commande faite, et un identifiant au catalogue, que par un changement de cap.

| N° | Besoin | Usage civil | Ce qui manque aujourd'hui |
|---|---|---|---|
| 1 | **Gisement** : angle compté depuis le nord du quadrillage (convergence des méridiens), en degrés et en grades | géomètres, topographes, implantation et contrôle sur plan | `geo.distance_azimuth` et `geo.destination_point` ne comptent que depuis le nord vrai (AD-31). |
| 2 | **Azimut magnétique** par le modèle magnétique mondial (WMM2025) | randonnée, course d'orientation, lecture à la boussole | aucune déclinaison magnétique au noyau. |
| 3 | **Carroyage étiqueté en MGRS** à l'export PDF | secours et recherche de personnes (grille USNG, identique au MGRS) | `grid.kind = "projected"` trace les lignes UTM sans étiquettes MGRS ; `degrees` étiquette en décimal. |
| 4 | **Conversion d'un raster vers GeoTIFF** par QGIS | tout utilisateur qui reçoit un MNT en ASCII Grid (`.asc`), ERDAS (`.img`), JPEG 2000 (`.jp2`)… | `data.convert` ne traite que le vecteur ; les commandes de terrain ne lisent que le GeoTIFF (AD-20). |
| 5 | **En-tête et pied de page libres** sur l'export PDF | bureaux d'études : « Document de travail », nom du bureau, mention de confidentialité envers le client | aucun texte libre hors du titre. |
| 6 | **Sources et date sur la carte** : la ligne des sources dit la résolution et la date des données, comme sur les cartes de l'IGN | tout rendu à un client ou à une collectivité | le PDF ne liste que les noms des fichiers sources. |

Accord du propriétaire du 2026-11-10 : usage du WMM (NOAA et British Geological Survey ;
coefficients et rapport technique publiés par la NOAA, œuvre du gouvernement fédéral des
États-Unis, **domaine public**).

## 2. Analyse d'impact

| Endroit | Effet |
|---|---|
| Principes (CLAUDE.md §2) | §2.1 salle blanche : gisement et WMM écrits d'après les publications (formules de Karney et de Krüger pour la convergence ; rapport technique du WMM2025 pour le champ), jamais d'après un code. §2.2 : aucune dépendance ajoutée ; les coefficients du WMM sont une donnée du domaine public, pas une bibliothèque. §2.5 : aucun réseau ; les coefficients sont embarqués dans le binaire du noyau. §2.6 : la conversion du raster écrit un fichier neuf, l'original intact. §2.9 : six besoins justifiés par un usage civil (tableau du §1). |
| Catalogue | `geo.distance_azimuth`, `geo.destination_point` et `layout.export_pdf` gagnent des `planned_params` et `planned_result` (§1 du catalogue, évolution d'une commande faite) ; une fiche neuve `data.convert_raster` en `planifiee` et sa ligne au §4. |
| Architecture | **AD-31** (« un azimut se compte depuis le nord vrai ») doit être amendée pour admettre un nord choisi explicitement : texte proposé au §4.5, **à porter au contrat seulement après confirmation du propriétaire**, avant la construction de l'entrée 28. **AD-19** : la convergence des méridiens n'est pas une conversion de coordonnées (elle se calcule depuis la longitude et la latitude d'un point déjà accepté par AD-31) ; aucun amendement. **AD-20** : la conversion du raster est déléguée à QGIS, le noyau lit ensuite le GeoTIFF comme aujourd'hui. **AD-17** (amendement du 2026-11-10) : la mise à jour du WMM passe par « Mettre à jour Cassini », comme toute nouvelle version du noyau ; aucun amendement. AD-24 : deux lignes de table fermée (conversion du raster ; l'export garde sa ligne). |
| Rejeu (AD-11) | Un `date` absent de la demande est fixé par le noyau à l'exécution, rendu dans le résultat et repassé au rejeu (même mécanisme que `center` de `layout.export_pdf`) : le rejeu d'un azimut magnétique est identique des années plus tard. Les journaux anciens n'ont ni `north` ni `date` : `north` absent vaut `true`, ils se rejouent à l'identique. `source_details` est un champ nouveau : un journal ancien ne l'a pas et le rejeu ne le compare pas pour une ligne qui ne le porte pas (même règle que les champs ajoutés par 11.11). |
| Journal | Rien de nouveau dans la forme ; les paramètres fournis par le noyau sont journalisés comme les autres. |
| Extension | Ligne de table de `data.convert_raster` ; pour l'export : en-tête, pied de page, étiquettes du carroyage MGRS (format de grille fixe, voir §4.3), ligne des sources détaillée. Aucune logique MGRS dans l'extension : les désignations de zone et de carré de 100 km sont calculées par le noyau et reçues comme du texte. |
| Panneau, MCP | Paramètres offerts au modèle, au client MCP et au formulaire, sauf ceux « fournis par le noyau ». Le résultat dit toujours quel nord a servi. |
| Générateur d'entraînement | Familles nouvelles, une par entrée (§4.6), apprises au ré-entraînement groupé suivant (entrée 17 de l'epic 10). |
| Jeu tenu à l'écart | Aucune phrase rescorée : aucune ne porte sur ces besoins. Des phrases nouvelles s'ajoutent à l'entrée 17. |
| Epics | Epic 10 (`epic-catalogue-etendu-qgis`) seul : quatre entrées (28 à 31), après la dernière primitive (16), avant la ré-épreuve (17) et le nettoyage (18). Tout l'epic 10 attend l'epic de démo (6) et la publication (7) : ces entrées ne passent devant ni 5.48 à 5.50 ni l'epic de démo. |
| PRD, spec, UX | Spec inchangée : CAP-14 (catalogue étendu) et CAP-15 (géométrie) couvrent ces ajouts. UX : aucun écran nouveau. |

## 3. Voie recommandée

**Ajustement direct** : quatre entrées dans l'epic 10, sans retour en arrière ni réduction de
périmètre. Effort : deux entrées du noyau (gisement ; azimut magnétique, la plus lourde), une
primitive déléguée (conversion du raster), une entrée de mise en page (extension et noyau).
Risques : gisement `medium`, azimut magnétique `high` (calcul nouveau, rejeu d'une date,
donnée embarquée à tenir à jour), conversion du raster `low`, mise en page `medium`. Aucun
effet sur le calendrier de la démo.

## 4. Changements proposés

### 4.1 Décisions de la party du 2026-11-10

Table ronde tenue par `bmad-party-mode`, en l'absence du propriétaire. Présents : 🏛️
l'architecte, gardien des AD ; 📐 la géomètre-topographe ; 🗺️ le cartographe de bureau
d'études.

> 📐 **Géomètre :** Un gisement, c'est l'angle depuis le nord du quadrillage. Si vous me rendez
> l'azimut de la géodésique corrigé de la convergence, je veux qu'on l'écrive noir sur blanc :
> ce n'est pas exactement l'angle que je tire de ΔE et ΔN sur mon plan.
>
> 🏛️ **Architecte :** Une seule convention, AD-31 : tout est géodésique. On ne fait pas une
> géométrie plane pour un champ et une géodésique pour l'autre. On corrige l'azimut de la
> géodésique de la convergence au point où il est mesuré, et on documente l'écart de corde.
>
> 📐 **Géomètre :** D'accord, à condition que la fiche le dise. Et le Lambert-93 ? Mes clients
> travaillent en 2154, pas en UTM.
>
> 🏛️ **Architecte :** AD-19 exclut le Lambert-93 du noyau, en toutes lettres. La convergence en
> Lambert-93 ne demande pourtant que la longitude : ce n'est pas une conversion. Mais je ne
> re-décide pas AD-19 en passant. UTM d'abord ; le Lambert-93 est une question au propriétaire.
>
> 🗺️ **Cartographe :** Pour le carroyage MGRS, l'extension ne doit rien savoir du MGRS ? Alors
> on étiquette les lignes par leurs deux chiffres principaux, comme sur les cartes USNG, et le
> cadre « zone 31T, carré GL » est un texte que le noyau calcule.
>
> 🏛️ **Architecte :** Oui. Deux chiffres tirés d'une coordonnée, c'est de la mise en forme. Les
> lettres du carré de 100 km, c'est du MGRS : noyau.
>
> 🗺️ **Cartographe :** Et la date des données : la date du fichier n'est pas la date de
> l'édition de la BD TOPO. Si on l'imprime, on dit d'où elle vient.

Décisions, chacune **tranchée en party le 2026-11-10, à confirmer par le propriétaire** :

- **D1 (besoin 1)** — Un paramètre `north` commun aux deux commandes : `true` (défaut, nord
  vrai, comportement actuel), `grid` (nord du quadrillage UTM), `magnetic` (besoin 2). Le
  quadrillage est la zone UTM de `from` si `from` est en UTM, sinon sa zone normale, ou la zone
  imposée par `utm_zone` (même règle et mêmes exceptions que `coord.convert`). Les unités
  restent celles d'`Angle` (`deg`, `gon`) : les grades sont déjà là.
- **D2 (besoin 1)** — Le gisement rendu est l'azimut de la géodésique au point de mesure,
  diminué de la convergence des méridiens en ce point (convention : convergence positive à
  l'est du méridien central dans l'hémisphère nord) ; `back_azimuth` est corrigé au point
  d'arrivée. La fiche dit qu'il diffère du gisement de la corde (ΔE, ΔN) de la correction de
  corde, petite à courte distance, chiffrée par le ticket à partir de la publication.
- **D3 (besoin 1)** — Le gisement en **Lambert-93** n'entre pas dans cette entrée : AD-19 exclut
  le Lambert-93 du noyau. **Question ouverte au propriétaire** : amender AD-19 pour admettre la
  seule convergence du Lambert-93 (fonction de la longitude, sans conversion de coordonnées),
  ou laisser le Lambert-93 aux angles calculés par QGIS.
- **D4 (besoin 2)** — Déclinaison au point de mesure, à l'altitude 0 m sur l'ellipsoïde (l'écart
  dû à l'altitude est négligeable pour la boussole, et le noyau ne connaît pas l'altitude d'un
  point sans MNT). Azimut magnétique = azimut vrai − déclinaison (déclinaison positive à l'est).
- **D5 (besoin 2)** — Paramètre `date` (AAAA-MM-JJ) ; absent, le noyau prend la date du jour
  (UTC) à l'exécution, la rend dans le résultat et la repasse au rejeu. Une date hors de la
  période de validité du WMM2025 (du 2025-01-01 au 2029-12-31) **n'est pas refusée** : le calcul
  est fait et le résultat porte un avertissement (`warning`). Même avertissement près des pôles
  magnétiques, dans les zones que le rapport du WMM déclare peu fiables (composante horizontale
  faible).
- **D6 (besoin 2)** — Coefficients du WMM2025 **compilés dans le binaire du noyau** (fichier de
  coefficients publié, gardé au dépôt avec sa source, sa date et son empreinte SHA-256) ; aucun
  fichier lu à l'exécution, donc rien à chercher ni à télécharger. Le prochain modèle
  (WMM2030, attendu fin 2029) arrive par une nouvelle version du noyau, installée par
  « Mettre à jour Cassini » ou par la porte hors ligne (AD-17, amendement du 2026-11-10) ;
  jamais par le réseau du noyau. Une révision hors cycle publiée par la NOAA suivrait le même
  chemin.
- **D7 (besoin 3)** — Troisième forme de `grid` : `{"kind": "mgrs", "interval": Distance}`.
  Lignes du quadrillage UTM ; chaque ligne étiquetée par ses **deux chiffres principaux** (le
  kilomètre dans le carré de 100 km), par un format de grille fixe de QGIS, sans calcul MGRS
  dans l'extension. La zone UTM (`epsg`) et les désignations de zone et de carré de 100 km
  couverts (`squares`, ex. `31T GL`) sont **fournies par le noyau** d'après l'emprise du projet
  en degrés (`extent.lonlat`, déjà rendue par `project.inventory`) et imprimées dans un cadre de
  légende. `interval` : 1 km, 10 km ou 100 km.
- **D8 (besoin 5)** — `header` et `footer` : une ligne de texte libre chacun, 120 caractères au
  plus, sans saut de ligne, imprimés tels quels en haut et en bas de page. Le modèle les
  recopie de la demande ; il ne les invente jamais (garde-fou ordinaire : une demande sans
  texte n'en a pas).
- **D9 (besoin 6)** — Nouveau champ de résultat `source_details`, à côté de `sources` qui ne
  change pas (le rejeu des journaux anciens reste identique) : pour chaque source, sa
  résolution (taille de cellule d'un raster, avec unité ; rien pour un vecteur) et sa date,
  avec l'**origine de la date** : `metadata` si les métadonnées de la couche dans QGIS en
  portent une, sinon `file` (dernière modification du fichier, déjà journalisée par AD-11).
  La ligne imprimée le dit : « fichier du 2025-03-14 » ou « données de 2024 ». Rien n'est
  inventé quand QGIS ne sait pas.
- **D10 (besoin 4)** — Identifiant `data.convert_raster` (famille `data.*`, « reprojection et
  conversion » ; `data.convert` reste vecteur, un raster ne se désignant jamais par son
  identifiant de couche, `RasterPath`). Sortie GeoTIFF Float32, sans-donnée -9999 (convention
  du catalogue pour un raster écrit), première bande. `assign_epsg` facultatif, seulement pour
  une source **sans système** (cas fréquent de l'ASCII Grid sans `.prj`) ; une source qui a un
  système et un `assign_epsg` différent est refusée : jamais de reprojection (AD-19). Le noyau
  dicte ensuite `layer.open_raster`.
- **D11 (ordre)** — Ordre de construction : gisement (28), azimut magnétique (29, qui réutilise
  le paramètre `north`), conversion du raster (30), mise en page (31). Les quatre après
  l'entrée 16, la ré-épreuve (17) et le nettoyage (18) les attendent.

### 4.2 Catalogue — `geo.distance_azimuth` et `geo.destination_point`

Ancien (extrait de la fiche `geo.distance_azimuth`) :

```json fiche
{"id": "geo.distance_azimuth", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"from": {"type": "Point", "required": true},
            "to": {"type": "Point", "required": true},
            "unit": {"type": "string", "required": false, "enum": ["deg", "gon"]}},
 "result": ["distance", "azimuth", "back_azimuth"]}
```

Nouveau (les `params` et le `result` faits ne changent pas ; les ajouts sont planifiés) :

```json fiche
{"id": "geo.distance_azimuth", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"from": {"type": "Point", "required": true},
            "to": {"type": "Point", "required": true},
            "unit": {"type": "string", "required": false, "enum": ["deg", "gon"]}},
 "result": ["distance", "azimuth", "back_azimuth"],
 "planned_by": "epic 10, entrée 28 (north true et grid, utm_zone, convergence) ; entrée 29 (north magnetic, date, declination, warning)",
 "planned_params": {"north": {"type": "string", "required": false, "enum": ["true", "grid", "magnetic"]},
                    "utm_zone": {"type": "integer", "required": false},
                    "date": {"type": "string", "required": false}},
 "planned_result": ["north", "convergence", "declination", "date", "warning"]}
```

Même ajout à `geo.destination_point` (`planned_params` identiques ; `azimuth` en entrée est
alors compté depuis le nord choisi ; `planned_result` : `north`, `convergence`,
`declination`, `date`, `warning`).

Texte ajouté aux deux fiches :

> **Nord de référence (changement de cap du 2026-11-10, epic 10, entrées 28 et 29).** `north`
> vaut `true` (défaut : nord vrai), `grid` (gisement : nord du quadrillage UTM de `from`, ou de
> la zone `utm_zone` ; convergence des méridiens d'après les formules de Krüger et de Karney)
> ou `magnetic` (azimut magnétique : déclinaison du modèle magnétique mondial WMM2025, à
> l'altitude 0 m, à la date `date`, AAAA-MM-JJ ; absente, la date du jour, fixée par le noyau
> et repassée au rejeu). Chaque azimut est corrigé au point où il est mesuré. Le gisement est
> celui de la géodésique, pas celui de la corde sur le plan. Résultats : `north` (le nord
> employé), `convergence` (`Angle` ou `null`, au point de départ, pour `grid`), `declination`
> (`Angle` ou `null`, au point de départ, pour `magnetic`), `date` (ou `null`), `warning`
> (texte en français ou `null` : date hors de la validité du WMM2025, du 2025-01-01 au
> 2029-12-31, ou point dans une zone que le rapport du WMM déclare peu fiable ; le calcul est
> rendu quand même). Erreurs nouvelles : `geo.invalid_date` (date mal formée) ;
> `geo.date_unused` (`date` sans `north: "magnetic"`) ; `utm_zone` sans `north: "grid"` est
> refusé de même (`geo.zone_unused`) ; `coord.out_of_range` hors de la grille UTM pour `grid`.

**Livraison en deux temps.** L'entrée 28 reporte dans `params` `north` avec l'énumération
`["true", "grid"]` et `utm_zone`, et `north`, `convergence` dans `result` ; elle laisse en
`planned_params` `north` avec l'énumération complète et `date`, en `planned_result`
`declination`, `date`, `warning`, avec `planned_by` « entrée 29 ». L'entrée 29 reporte le
reste et retire les champs planifiés.

### 4.3 Catalogue — `layout.export_pdf`

Ancien : fiche faite actuelle, `planned_by` « epic 10, entrée 15 (skipped_layers) »,
`planned_result` `["skipped_layers"]`.

Nouveau :

```json fiche
{"id": "layout.export_pdf", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"title": {"type": "string", "required": true},
            "scale": {"type": "integer", "required": false},
            "center": {"type": "Point", "required": false},
            "leave_out": {"type": "array", "required": false, "items": "string"},
            "out": {"type": "string", "required": false},
            "page_size": {"type": "string", "required": false, "enum": ["a4", "a3", "a2", "a1", "a0"]},
            "orientation": {"type": "string", "required": false, "enum": ["landscape", "portrait"]},
            "grid": {"type": "union", "required": false, "tag": "kind",
                     "variants": {"degrees": {"interval": {"type": "Angle", "required": true}},
                                  "projected": {"epsg": {"type": "integer", "required": true},
                                                "interval": {"type": "Distance", "required": true}}}}},
 "result": ["center", "file", "layer_count", "page_count", "scale", "sources", "truncated"],
 "planned_by": "epic 10, entrée 15 (skipped_layers) ; entrée 31 (header, footer, grid mgrs, source_details)",
 "planned_params": {"header": {"type": "string", "required": false},
                    "footer": {"type": "string", "required": false},
                    "grid": {"type": "union", "required": false, "tag": "kind",
                             "variants": {"degrees": {"interval": {"type": "Angle", "required": true}},
                                          "projected": {"epsg": {"type": "integer", "required": true},
                                                        "interval": {"type": "Distance", "required": true}},
                                          "mgrs": {"interval": {"type": "Distance", "required": true},
                                                   "epsg": {"type": "integer", "required": false},
                                                   "squares": {"type": "array", "required": false, "items": "string"}}}}},
 "planned_result": ["skipped_layers", "source_details"]}
```

Texte ajouté à la fiche :

> **Epic 10, entrée 31 (changement de cap du 2026-11-10).** `header` et `footer` : une ligne
> libre chacun (120 caractères au plus, sans saut de ligne), imprimée en haut et en bas de
> page. `grid` gagne la forme `{"kind": "mgrs", "interval": Distance}` (1 km, 10 km ou
> 100 km) : quadrillage UTM étiqueté par les deux chiffres principaux de chaque ligne, et cadre
> de légende des désignations de zone et de carré de 100 km ; `epsg` et `squares` sont
> **fournis par le noyau** d'après `extent.lonlat` de l'inventaire, jamais écrits par le modèle
> ni par l'utilisateur, et journalisés pour le rejeu. Sans emprise lisible, `layout.no_extent` (aucun carroyage MGRS sans emprise).
> `source_details` (liste de 50 au plus, tronquée comme `sources`) : pour chaque source
> `source` (nom du fichier), `resolution` (`Distance` ou `null`), `date` (AAAA-MM-JJ ou `null`)
> et `date_origin` (`metadata`, `file` ou `null`) ; la ligne des sources imprimée les reprend
> (« mnt_5m.tif, 5 m, fichier du 2025-03-14 »).

### 4.4 Catalogue — `data.convert_raster` (nouvelle fiche)

§4, nouvelle ligne après `data.convert` :

```
| `data.convert_raster` | planifiée | qgis | epic 10, entrée 30 |
```

§7, nouvelle fiche après `data.convert` :

> ### `data.convert_raster`
>
> Écrit un raster lisible par QGIS (ASCII Grid `.asc`, ERDAS `.img`, JPEG 2000 `.jp2`, GeoTIFF,
> VRT…) en GeoTIFF Float32, sans-donnée -9999, première bande, pour que les commandes du noyau
> le lisent (AD-20) ; l'original est intact (changement de cap du 2026-11-10, epic 10,
> entrée 30). Algorithme visé `gdal:translate`, repli `gdal:warpreproject` sans changement de
> système si le sans-donnée doit être réécrit ; à vérifier dans la documentation de QGIS 3.44
> au ticket. Le noyau dicte ensuite `layer.open_raster`. Jamais de reprojection (AD-19).
>
> | Paramètre | Type | Oblig. | Unité / valeurs | Sens |
> |---|---|---|---|---|
> | `raster` | `RasterPath` | oui | — | Raster à convertir. |
> | `assign_epsg` | entier | non | — | Système à déclarer, seulement pour une source qui n'en a pas. |
> | `out` | chaîne | non | — | Fourni par le noyau. |
>
> | Résultat | Type | Sens |
> |---|---|---|
> | `file` | chaîne | GeoTIFF écrit. |
> | `epsg` | entier ou `null` | Système du GeoTIFF écrit, s'il en a un. |
>
> Erreurs : `layer.not_found` (fichier absent) ; `analysis.bad_params` (`assign_epsg` pour une
> source qui a déjà un autre système) ; `data.unknown_crs` ; `analysis.failed` ;
> `qgis.cancelled`. Un GeoTIFF sans système reste lisible par QGIS mais refusé ensuite par les
> commandes du noyau (`dem.no_epsg`) : le résumé le dit.
>
> ```json fiche
> {"id": "data.convert_raster", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
>  "params": {"raster": {"type": "RasterPath", "required": true},
>             "assign_epsg": {"type": "integer", "required": false},
>             "out": {"type": "string", "required": false}},
>  "result": ["epsg", "file"]}
> ```

### 4.5 Architecture — amendement proposé d'AD-31 (non appliqué)

À porter au contrat **après confirmation du propriétaire**, avant la construction de
l'entrée 28 ; aucune autre AD ne change.

> - **Amendement (2026-11-10, changement de cap
>   `change-complements-geometrie-et-mise-en-page`) — nord choisi.** Un azimut se compte
>   depuis le nord vrai, sauf quand la commande reçoit `north` : `grid` (nord du quadrillage
>   UTM, par la convergence des méridiens calculée d'après la publication, ce qui n'est pas une
>   conversion de coordonnées au sens d'AD-19) ou `magnetic` (déclinaison du modèle magnétique
>   mondial embarqué, à une date journalisée). Le nord employé est toujours rendu dans le
>   résultat ; une commande ne change jamais de nord sans ce paramètre. Les coefficients du
>   modèle magnétique sont une donnée compilée dans le noyau, mise à jour par une nouvelle
>   version du noyau (AD-17), jamais lue par le réseau.

### 4.6 Epic 10 — `epic-catalogue-etendu-qgis/tickets.toml`

Quatre entrées, dans l'ordre de construction, après l'entrée 16 :

| Id | Titre | Risque | Famille du générateur |
|---|---|---|---|
| 28 | Gisement : azimut compté depuis le nord du quadrillage | medium | `gisement` |
| 29 | Azimut magnétique par le modèle magnétique mondial (WMM2025) | high | `azimut_magnetique` |
| 30 | Conversion d'un raster en GeoTIFF (`data.convert_raster`) | low | `conversion_raster` |
| 31 | Mise en page : carroyage MGRS étiqueté, en-tête et pied de page, sources détaillées | medium | `carroyage_mgrs`, `entete_pied_de_page` (le besoin 6 est automatique : pas de famille) |

L'entrée 17 (ré-épreuve) et l'entrée 18 (nettoyage) gagnent 28 à 31 dans leur `after`.

### 4.7 Décision dans l'epic 10 — `epic-catalogue-etendu-qgis.md`

> - Decision: 2026-11-10 — changement de cap `change-complements-geometrie-et-mise-en-page`
>   (rangement approuvé par le propriétaire, détail tranché en party, à confirmer) : gisement
>   (28), azimut magnétique par le WMM2025 du domaine public (29), `data.convert_raster` (30),
>   mise en page — carroyage MGRS étiqueté, en-tête et pied de page, sources détaillées (31) ;
>   après l'entrée 16, avant 17 et 18 ; amendement d'AD-31 à confirmer avant 28.

## 5. Passage à la réalisation

- **Portée : modérée** (quatre entrées et un amendement d'AD-31 à confirmer).
- **Appliqué avec `bmad-ticket` le 2026-11-10 :** entrées 28 à 31 de l'epic 10, ligne Decision,
  fiches du catalogue (`planned_*` et fiche `planifiee`) ; recette `core/tests/catalogue.rs`
  verte.
- **Attend le propriétaire :** confirmation des décisions D1 à D11 ; amendement d'AD-31 (§4.5) ;
  question D3 (gisement en Lambert-93, AD-19).
- **Au moment de construire l'entrée 29 :** télécharger à la main, depuis la page officielle du
  WMM de la NOAA (NCEI), le fichier de coefficients du WMM2025 et le rapport technique ;
  vérifier qu'ils sont bien ceux de la NOAA, noter la date, l'adresse et l'empreinte SHA-256
  dans le dépôt, et confirmer la mention de domaine public sur la page. Les valeurs de contrôle
  du rapport technique servent de tests.
- **Réussite :** chaque entrée verte sur ses tests et sa recette QGIS ; le modèle compose ces
  demandes après le ré-entraînement groupé de l'entrée 17.
