# Catalogue des commandes de Cassini

> Contrat figé le 2026-11-05 (epic 11, entrée 1). Il donne, pour chaque commande faite,
> planifiée ou réservée, son identifiant, ses paramètres, son résultat et l'endroit où elle
> s'exécute. Il fait foi sur la liste des commandes (AD-6, amendement du 2026-11-05 ;
> Consistency Conventions du contrat d'architecture).
>
> Sources : `../change-geometrie-et-terrain-avant-la-demo/change-geometrie-et-terrain-avant-la-demo.md`
> (§4.1), `../epic-catalogue-etendu-qgis/tickets.toml` (epic 10), le registre du noyau
> (`core/src/commands/`) et le contrat d'architecture (AD-8, AD-19, AD-20, AD-23, AD-24,
> AD-27, AD-28, AD-30, AD-31).

## 1. Règle de gel

- Un identifiant, un nom de paramètre, son type, son caractère obligatoire, ses valeurs
  permises ou un champ du résultat **ne changent que par un changement de cap**. Le
  générateur d'exemples et l'entraînement du modèle s'alignent sur ce catalogue une fois.
- Une entrée qui implémente une commande planifiée l'écrit **exactement** comme sa fiche
  la décrit, puis passe la fiche en `faite` dans le même commit.
- Une évolution déjà décidée d'une commande faite est écrite dans sa fiche sous
  `planned_params` (paramètres ajoutés, retypés ou dont `required` change) et
  `planned_result` (champs de résultat ajoutés), avec `planned_by`, l'entrée qui la
  livre : l'entrée reporte `planned_params` dans `params` et `planned_result` dans
  `result` quand elle la livre.
- Une fiche faite dont le remplacement est décidé porte `replaced_by`, l'identifiant de
  la fiche planifiée qui la remplace (la recette vérifie que cette fiche existe) ;
  l'entrée qui livre le remplacement retire la fiche faite et passe la fiche planifiée en
  `faite`, dans le même commit.
- Une recette du noyau, `core/tests/catalogue.rs` (lancée par `cargo test`, donc par la
  CI), confronte ce document au registre et échoue au moindre écart.
- Aucun paramètre propre à un usage particulier (classe de véhicule, seuil métier, choix
  d'un emplacement) n'entre au catalogue : le noyau fournit les briques. Tout seuil
  (distance, durée, pente, hauteur, nombre) est un paramètre, avec sa valeur par défaut
  dans le schéma Rust quand il en a une (AD-24).

## 2. Statuts, exécution, forme des fiches

| Champ | Valeurs | Sens |
|---|---|---|
| `status` | `faite` | Au registre ; la recette compare la fiche au code. |
| | `planifiee` | Contrat figé, pas encore au registre ; la recette vérifie sa forme et qu'elle n'est pas au registre. |
| | `reservee` | Identifiant retenu, paramètres écrits plus tard par un changement de cap. |
| `runs_in` | `core` | Calculée en Rust dans le noyau, testée sans QGIS (AD-1). |
| | `qgis` | Déléguée à QGIS : une ligne de la table fermée de l'extension (AD-24). |
| `dictated_by_core` | `true` / `false` | Vrai pour une commande que seul le noyau appelle, après une autre étape ; elle n'est offerte ni au modèle, ni au client MCP, ni au formulaire du panneau (amendement 4.9 d'AD-9, AD-30). |

Chaque fiche est un titre de niveau 3 qui nomme l'identifiant entre accents graves, un
tableau lisible, puis un bloc ` ```json fiche ` que lit la recette :

```json
{"id": "terrain.slope", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"dem": {"type": "RasterPath", "required": true},
            "unit": {"type": "string", "required": true, "enum": ["degrees", "percent"]},
            "out": {"type": "string", "required": false}},
 "result": ["file", "max", "min", "nodata_cells", "unit"]}
```

- `params` : chaque paramètre a `type`, `required`, et au besoin `enum` (valeurs
  permises, comparées comme un ensemble : l'ordre ne compte pas), `items` (les éléments
  d'un tableau : un nom de type, ou un objet de type avec son `enum` ou ses `fields`),
  `fields` (les champs d'un objet en ligne, même forme que `params`, obligatoires pour
  tout `object`), ou `tag` et `variants` pour une union étiquetée.
- `type` est le type JSON de premier niveau (`string`, `number`, `integer`, `boolean`,
  `object`, `array`), le **nom d'un type nommé** de la section 3, ou `union`. Le schéma
  engendré se normalise **en profondeur** : un `$ref` vers un type nommé garde son nom, et
  la définition de ce type (`$defs`) est comparée une fois à son bloc ` ```json type ` ; un
  `$ref` vers un autre type est suivi ; une alternative avec `null` (un `Option`) rend le
  paramètre facultatif ; une alternative de constantes donne `enum` ; un objet garde ses
  champs ; un tableau garde le type et l'`enum` de ses éléments ; une alternative d'objets
  qui portent tous un même champ constant (enum Rust `#[serde(tag = "kind")]`) devient
  `{"type": "union", "tag": "kind", "variants": {"<valeur>": {champs}}}`, l'étiquette
  ôtée des champs de chaque variante.
- `required` vaut `true` quand le paramètre est obligatoire et non nul.
- `result` : la liste des noms des champs du résultat.
- Une fiche faite peut porter `planned_params` (paramètres ajoutés, retypés ou dont
  `required` change) et `planned_result` (champs de résultat ajoutés), avec `planned_by`
  qui nomme les entrées qui les livrent ; leur forme est vérifiée, jamais confrontée au
  code. Elle peut aussi porter `replaced_by`, l'identifiant de la fiche planifiée qui la
  remplace, qui doit exister au catalogue.
- Un objet en ligne déclare ses `fields` ; une table de noms libres (les `values` de
  `SummaryRow`) s'écrit `"fields": {}`. Un tableau déclare ses `items` ; des éléments
  objets, tableaux ou unions s'écrivent en objet de type, jamais par la seule chaîne
  `object`, `array` ou `union`.
- Une fiche `reservee` peut n'avoir que `id`, `status` et `runs_in`.
- Les commandes `test.*`, compilées pour les tests seulement, ne figurent pas au catalogue.

**Conventions communes.**

- `out` (chaîne, facultative) : fichier de sortie, **fourni par le noyau** dans le dossier
  de travail (AD-10) ; ni le modèle ni l'utilisateur ne l'écrivent.
- Une commande déléguée qui produit une couche vecteur rend `layer_id`, `name`, `crs`
  (code explicite, `EPSG:2154`) et `feature_count`.
- Une commande qui écrit un raster rend `file` (GeoTIFF Float32, sans-donnée -9999) ; le
  noyau dicte ensuite `layer.open_raster` (AD-9, AD-24). Une commande du noyau qui rend
  des entités écrit un GeoJSON et le noyau dicte `layer.open_vector` (AD-30). Un seul écrivain
  du noyau écrit ces fichiers ; il refuse un nombre non fini ou un dossier absent par
  `geojson.unwritable`, sans rien écrire.
- Toute erreur suit l'enveloppe d'AD-21 (`domaine.cause`, message en français). Les
  erreurs du module raster unique (`dem.unreadable`, `dem.unsupported`, `dem.no_epsg`,
  `dem.not_projected`, `dem.unwritable`) valent pour toute commande qui lit un raster
  dans le noyau, MNT, coût ou critère (AD-20).

## 3. Types nommés

### Types partagés

**`Point`** — un point en coordonnées, avec son système (AD-19, AD-23). Défini dans
`core/src/point.rs`, marqué `x-cassini: point` dans le schéma.

| Champ | Type | Sens |
|---|---|---|
| `x` | nombre | Abscisse ; **en WGS84, la longitude**. |
| `y` | nombre | Ordonnée ; **en WGS84, la latitude**. |
| `epsg` | entier | Code EPSG du système (2154 Lambert-93, 4326 WGS84, 326NN/327NN UTM). |

```json type
{"name": "Point", "type": "object",
 "fields": {"x": {"type": "number", "required": true},
            "y": {"type": "number", "required": true},
            "epsg": {"type": "integer", "required": true}}}
```

**`MgrsRef`** — une référence MGRS (AD-23, CAP-15). Chaîne validée par la grammaire de la
NGA : zone de 1 à 60, bande de C à X (sans I ni O), carré de 100 km (deux lettres), puis
de 1 à 5 chiffres par axe, autant pour chaque axe. Forme canonique rendue : majuscules,
sans espace, zone sur deux chiffres (`31UDQ4825111750`) ; à la lecture, espaces et casse
sont ignorés. Les zones polaires (UPS, bandes A, B, Y, Z) sont hors du type. Marquée dans
le schéma engendré comme `Point`, et soumise à la même garde de provenance : elle figure
telle quelle dans la demande, ou vient d'une référence `step:` vers un champ déclaré
`MgrsRef`. Vers MGRS, la conversion **tronque** (convention de la NGA). Posée par
l'entrée 2 de l'epic 11 (`core/src/point.rs`, marque `x-cassini: mgrs`) ; la chaîne passe
telle quelle au schéma, sa grammaire est lue par la commande qui la reçoit.

Une référence mal formée est refusée par l'erreur `coord.invalid_mgrs` (AD-21), qui
nomme la partie fautive (zone, bande, carré de 100 km ou chiffres).

```json type
{"name": "MgrsRef", "type": "string"}
```

**`Angle`** — un angle avec son unité (AD-31).

| Champ | Type | Valeurs | Sens |
|---|---|---|---|
| `value` | nombre | — | Valeur de l'angle. |
| `unit` | chaîne | `deg` (défaut), `gon` | Degrés ou grades. |

Un azimut se compte depuis le **nord vrai**, dans le sens horaire, dans [0, 360) degrés
ou [0, 400) grades ; un azimut rendu est ramené à cet intervalle. Posé par l'entrée 3 de
l'epic 11 (`core/src/angle.rs`) ; une valeur non finie est refusée.

```json type
{"name": "Angle", "type": "object",
 "fields": {"value": {"type": "number", "required": true},
            "unit": {"type": "string", "required": false, "enum": ["deg", "gon"]}}}
```

**`RasterPath`** — désignation d'un raster, du noyau ou délégué (amendement du
2026-11-05 d'AD-24). Chaîne : un chemin relatif au projet (AD-9), ou une référence
`step:k` vers la sortie raster d'une étape, journalisée comme le repère `out:k.n` d'AD-11,
jamais comme un chemin résolu. Marquée dans le schéma engendré (`x-cassini: raster`). Un
raster ne se désigne jamais par son identifiant de couche. La référence `step:k` n'est admise
que si l'étape `k` écrit exactement un raster (`plan.type_mismatch`), et jamais sous la forme
d'un champ de sa sortie, `step:k.file` (`plan.bad_reference`). Posé par l'entrée 5 de
l'epic 11 (`core/src/raster_path.rs`), qui y a migré `dem` de `terrain.slope`,
`terrain.viewshed`, `terrain.line_of_sight` et `path` de `layer.open_raster`. Le long prompt
du modèle local le voit comme une chaîne. Hors d'un plan, `step:k` n'a pas de sens ; un appel
direct (`command.run`, MCP) peut citer la sortie d'une ligne du journal par son repère
`out:<k>.<n>`, résolu par le journal (`replay.bad_reference` s'il ne désigne aucune sortie).

```json type
{"name": "RasterPath", "type": "string"}
```

### Types de mesure (déjà au code)

Aucune mesure nue ne traverse le canal (Consistency Conventions, « Mesures »).

| Type | Champs | Valeurs de `unit` | Remarque |
|---|---|---|---|
| **`Distance`** | `value` (nombre > 0), `unit` | `m`, `km` | Distance ou rayon. Dans un résultat du noyau, `unit` vaut toujours `m`. |
| **`Height`** | `value` (nombre ≥ 0), `unit` | `m`, `km` | Hauteur au-dessus du sol, ou dénivelé. |
| **`Duration`** | `value` (nombre > 0), `unit` | `s`, `min` | Durée. |

```json type
{"name": "Distance", "type": "object",
 "fields": {"value": {"type": "number", "required": true},
            "unit": {"type": "string", "required": true, "enum": ["m", "km"]}}}
```

```json type
{"name": "Height", "type": "object",
 "fields": {"value": {"type": "number", "required": true},
            "unit": {"type": "string", "required": true, "enum": ["m", "km"]}}}
```

```json type
{"name": "Duration", "type": "object",
 "fields": {"value": {"type": "number", "required": true},
            "unit": {"type": "string", "required": true, "enum": ["s", "min"]}}}
```

### Contrat de réseau (AD-28)

**`NetworkOptions`** — défini une fois en Rust, lu par `analysis.isochrone` (après
l'entrée 10.6) et par `network.*`. Posé par l'entrée 6 de l'epic 10.

| Champ | Type | Oblig. | Valeurs / unité | Sens |
|---|---|---|---|---|
| `speed` | objet `{value, unit}` | non | `unit` : `km/h` ; `value` > 0 | Vitesse uniforme. Absente : la vitesse du `mode` de la commande (5 km/h à pied, 50 km/h en voiture). |
| `speed_field` | chaîne | non | champ numérique, en km/h | Vitesse lue par tronçon ; prioritaire sur `speed`. |
| `direction` | objet `{field, forward, backward}` | non | trois chaînes | Sens de circulation : valeur de `field` qui veut dire « sens de saisie seul » (`forward`) et « sens inverse seul » (`backward`) ; toute autre valeur, double sens. Absent : double sens partout. |
| `tolerance` | `Distance` | non | défaut 100 m | Accrochage d'un point de départ ou d'arrivée au réseau, et des extrémités entre elles. |

```json type
{"name": "NetworkOptions", "type": "object",
 "fields": {"speed": {"type": "object", "required": false,
                      "fields": {"value": {"type": "number", "required": true},
                                 "unit": {"type": "string", "required": true, "enum": ["km/h"]}}},
            "speed_field": {"type": "string", "required": false},
            "direction": {"type": "object", "required": false,
                          "fields": {"field": {"type": "string", "required": true},
                                     "forward": {"type": "string", "required": true},
                                     "backward": {"type": "string", "required": true}}},
            "tolerance": {"type": "Distance", "required": false}}}
```

### Gabarit de résumé (AD-27)

**`SummaryRow`** — un élément de `rows` : `label` (chaîne, d'un champ désigné ou du rang,
jamais de l'identifiant d'entité) et `values`, qui associe un nom à `{value, unit}`. Un
résultat qui porte `rows` porte aussi `truncated` (booléen explicite) ; la borne est une
constante du noyau. Posé par l'entrée 1 de l'epic 10.

```json type
{"name": "SummaryRow", "type": "object",
 "fields": {"label": {"type": "string", "required": true},
            "values": {"type": "object", "required": true, "fields": {}}}}
```

### Description de style (changement de cap du 2026-11-08)

**`StyleSpec`** — le style d'une couche, neutre (AD-22) : des couleurs, des épaisseurs,
des classes par valeur de champ, une rampe d'altitudes, jamais un nom de moteur de rendu ni
une expression de QGIS. Le noyau le compose depuis sa table de sémiologie (IGN, CNIG) ;
l'extension le traduit en rendu de QGIS. Union étiquetée par `kind` :

| `kind` | Champs | Sens |
|---|---|---|
| `single` | `stroke`, `fill`, `width_mm`, `opacity`, `max_scale` | Un seul symbole. `fill` absent : trait seul (ligne, contour) ; `stroke` absent : surface sans contour ; au moins l'un des deux. |
| `categories` | `field`, `classes`, `other`, `opacity`, `max_scale` | Un symbole par valeur de `field` ; `classes` = `{values, label, stroke, fill, width_mm, max_scale}` ; `other` = `{label, stroke, fill, width_mm, max_scale}`, le symbole des valeurs non listées (absent : non dessinées). |
| `graduated` | `field`, `method`, `class_count`, `ramp`, `stroke`, `width_mm`, `max_scale` | Dégradé selon un champ numérique d'un vecteur : QGIS calcule `class_count` classes par `method` (`equal_interval`, `quantile`, `jenks`) sur les valeurs réelles, et colore de `ramp.from` à `ramp.to`. Le noyau ne calcule aucun seuil. |
| `elevation_ramp` | `stops`, `opacity` | Teintes d'altitude d'un MNT : `stops` = `{value, color}`, valeurs croissantes, en mètres. |
| `hillshade` | `opacity` | Un raster d'ombrage **déjà présent** (sortie de `terrain.hillshade` ou fichier de relief ombré) rendu en gris et posé en fondu (mode produit) sur les couches du dessous. Un raster que QGIS dessine déjà en relief ombré (un MNT au rendu d'ombrage) garde ce rendu : seuls l'opacité et le fondu changent (ajout du 2026-11-08). Le style ne calcule aucun ombrage. |

Une couleur est une chaîne `#rrggbb` ; `width_mm` est en millimètres de carte ; `opacity`
va de 0 à 1 (1 par défaut) ; elle est portée par `single`, `categories` (opacité de toute
la couche, par exemple les zones d'un PLU qui laissent voir le fond : ajout du 2026-11-08),
`elevation_ramp` et `hillshade`, et `graduated` n'en a **pas**. `max_scale`, facultatif
sur les trois formes vecteur, est un dénominateur d'échelle entier : la couche n'est dessinée
qu'aux échelles plus grandes que 1:`max_scale` (en zoomant davantage) ; absent, elle l'est à
toute échelle ; une valeur sous 1 est refusée par `style.invalid` (ajout du 2026-11-08, voir
le changement de cap, décision 8). Une classe de `categories`, et `other`, peuvent porter leur
propre `max_scale`, de même sens et de mêmes bornes : la classe n'est dessinée qu'aux échelles
plus grandes que 1:`max_scale`, et celui de la couche s'applique en plus. Une couleur mal formée, une épaisseur négative, une opacité
hors de [0, 1], des `stops` non croissants, ou un `single` sans `stroke` ni `fill` sont
refusés par `style.invalid` (AD-21). Les `single` et `categories` vont aux vecteurs, les
`elevation_ramp` et `hillshade` aux rasters ; l'inverse est refusé par
`style.geometry_mismatch`. `class_count` va de 2 à 12, sinon `style.invalid` ; un `field`
non numérique est refusé par `style.field_missing` (l'extension le vérifie) ; `graduated` va
aux vecteurs (sinon `style.geometry_mismatch`). Variante `graduated` ajoutée sur décision du
propriétaire du 2026-11-08 (entrée 20). Ce bloc fait foi sur le changement de cap du 2026-11-08, qui en
donne l'esquisse.

```json type
{"name": "StyleSpec", "type": "union", "tag": "kind",
 "variants": {
   "single": {"stroke": {"type": "string", "required": false},
              "fill": {"type": "string", "required": false},
              "width_mm": {"type": "number", "required": false},
              "opacity": {"type": "number", "required": false},
              "max_scale": {"type": "integer", "required": false}},
   "categories": {"field": {"type": "string", "required": true},
                  "classes": {"type": "array", "required": true,
                              "items": {"type": "object",
                                        "fields": {"values": {"type": "array", "required": true, "items": "string"},
                                                   "label": {"type": "string", "required": true},
                                                   "stroke": {"type": "string", "required": true},
                                                   "fill": {"type": "string", "required": false},
                                                   "width_mm": {"type": "number", "required": false},
                                                   "max_scale": {"type": "integer", "required": false}}}},
                  "other": {"type": "object", "required": false,
                            "fields": {"label": {"type": "string", "required": true},
                                       "stroke": {"type": "string", "required": true},
                                       "fill": {"type": "string", "required": false},
                                       "width_mm": {"type": "number", "required": false},
                                       "max_scale": {"type": "integer", "required": false}}},
                  "opacity": {"type": "number", "required": false},
                  "max_scale": {"type": "integer", "required": false}},
   "graduated": {"field": {"type": "string", "required": true},
                 "method": {"type": "string", "required": true, "enum": ["equal_interval", "quantile", "jenks"]},
                 "class_count": {"type": "integer", "required": true},
                 "ramp": {"type": "object", "required": true,
                          "fields": {"from": {"type": "string", "required": true},
                                     "to": {"type": "string", "required": true}}},
                 "stroke": {"type": "string", "required": false},
                 "width_mm": {"type": "number", "required": false},
                 "max_scale": {"type": "integer", "required": false}},
   "elevation_ramp": {"stops": {"type": "array", "required": true,
                                "items": {"type": "object",
                                          "fields": {"value": {"type": "number", "required": true},
                                                     "color": {"type": "string", "required": true}}}},
                      "opacity": {"type": "number", "required": false}},
   "hillshade": {"opacity": {"type": "number", "required": false}}}}
```

## 4. Liste des identifiants

| Identifiant | Statut | Exécution | Livrée par |
|---|---|---|---|
| `system.about` | faite | core | — |
| `model.list` | faite | core | — |
| `project.inventory` | faite | qgis | — |
| `layer.add` | faite | qgis | — |
| `layer.describe` | faite | qgis | — |
| `layer.open_raster` | faite (dictée) | qgis | — |
| `analysis.buffer` | faite | qgis | — |
| `analysis.intersect` | faite | qgis | — |
| `analysis.select_by_attribute` | faite | qgis | — |
| `analysis.zonal_statistics` | faite, renommée par 10.2 | qgis | — |
| `analysis.isochrone` | faite | qgis | — |
| `layout.export_pdf` | faite | qgis | — |
| `terrain.slope` | faite | core | — |
| `terrain.viewshed` | faite | core | — |
| `terrain.line_of_sight` | faite | core | — |
| `coord.convert` | faite | core | epic 11, entrée 2 |
| `geo.distance_azimuth` | faite | core | epic 11, entrée 3 |
| `geo.destination_point` | faite | core | epic 11, entrée 3 |
| `geo.bearing_intersection` | faite | core | epic 11, entrée 3 |
| `data.transform_point` | faite | qgis | epic 11, entrée 4 |
| `terrain.aspect` | faite | core | epic 11, entrée 6 |
| `terrain.hillshade` | faite | core | epic 11, entrée 6 |
| `terrain.viewshed_cumulative` | faite | core | epic 11, entrée 7 |
| `layer.open_vector` | faite (dictée) | qgis | epic 11, entrée 8 |
| `terrain.peaks` | faite | core | epic 11, entrée 15 |
| `raster.reclassify` | faite | core | epic 11, entrée 9 |
| `raster.weighted_overlay` | faite | core | epic 11, entrée 9 |
| `terrain.cost_distance` | faite | core | epic 11, entrée 10 |
| `terrain.least_cost_path` | faite | core | epic 11, entrée 10 |
| `analysis.count_points_in_polygon` | planifiée | qgis | epic 10, entrée 2 |
| `data.reproject` | faite | qgis | epic 10, entrée 7 |
| `analysis.difference` | planifiée | qgis | epic 10, entrée 21 |
| `analysis.area` | planifiée | qgis | epic 10, entrée 22 |
| `data.convert` | planifiée | qgis | epic 10, entrée 9 |
| `analysis.nearest` | planifiée | qgis | epic 10, entrée 10 |
| `analysis.distance_matrix` | planifiée | qgis | epic 10, entrée 10 |
| `analysis.statistics_by_category` | planifiée | qgis | epic 10, entrée 11 |
| `analysis.sum_line_lengths` | planifiée | qgis | epic 10, entrée 11 |
| `analysis.join_table` | planifiée | qgis | epic 10, entrée 23 |
| `analysis.dissolve` | planifiée | qgis | epic 10, entrée 24 |
| `raster.zonal_statistics` | planifiée | qgis | epic 10, entrée 12 |
| `raster.polygonize` | planifiée | qgis | epic 10 (entrée ajoutée après la 12) |
| `raster.align` | planifiée | qgis | epic 10 (entrée ajoutée après la 12) |
| `raster.contour` | planifiée | qgis | epic 10, entrée 26 |
| `network.shortest_path` | planifiée | qgis | epic 10, entrée 13 |
| `network.prepare` | planifiée | qgis | epic 10, entrée 14 |
| `layout.export_atlas` | planifiée | qgis | epic 10, entrée 15 |
| `geocode.ban` | planifiée | core | epic 10, entrée 16 |
| `map.topographic_style` | faite | core | epic 11, entrée 20 (geste : 21) |
| `map.graduated_style` | faite | core | epic 11, entrée 20 (geste : 21) |
| `layer.set_style` | faite (dictée) | qgis | epic 11, entrée 20 (geste : 21) |
| `layer.arrange` | faite (dictée) | qgis | epic 11, entrée 20 (geste : 21) |
| `terrain.flow_accumulation` | réservée | core | après la démo |
| `terrain.watershed` | réservée | core | après la démo |
| `terrain.flood_extent` | réservée | core | après la démo |
| `terrain.radio_coverage` | réservée | core | après la démo |

---

## 5. Commandes faites

### `system.about`

Version du noyau et du protocole. Aucun paramètre.

| Résultat | Type | Sens |
|---|---|---|
| `core_version` | chaîne | Version du noyau. |
| `protocol_version` | entier | Version du protocole (2). |

```json fiche
{"id": "system.about", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {},
 "result": ["core_version", "protocol_version"]}
```

### `model.list`

Modèles d'IA locale (GGUF) disponibles sur le poste. Aucun paramètre.

| Résultat | Type | Sens |
|---|---|---|
| `folder` | chaîne | Dossier de modèles retenu. |
| `models` | liste de `{file, size_bytes, declaration?}` | Modèles triés par nom. |

```json fiche
{"id": "model.list", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {},
 "result": ["folder", "models"]}
```

### `project.inventory`

Inventaire du projet ouvert : couches dans l'ordre de la légende, système du projet,
étendue. Ne modifie rien. Aucun paramètre.

| Résultat | Type | Sens |
|---|---|---|
| `extent` | emprise ou `null` | Étendue de l'ensemble, dans le système du projet. |
| `layers` | liste de couches | `layer_id`, `name`, `crs`, `kind`, `source`, `geometry`, `feature_count`, `fields`. |
| `project_crs` | chaîne | Système du projet, code explicite. |
| `project_name` | chaîne ou `null` | Nom du fichier de projet sans extension. |

```json fiche
{"id": "project.inventory", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {},
 "result": ["extent", "layers", "project_crs", "project_name"]}
```

### `layer.add`

Ajoute au projet une couche d'un fichier GeoPackage, Shapefile, GeoJSON, GeoParquet ou CSV
à coordonnées.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `path` | chaîne | oui | `.gpkg`, `.shp`, `.geojson`, `.parquet`, `.csv` | Fichier à ouvrir, relatif au projet. |
| `crs` | chaîne | non | code explicite (`EPSG:2154`) | Système à donner quand le fichier n'en dit rien ; obligatoire pour un CSV. |
| `x_field` | chaîne | non | défaut `X` | CSV : colonne des abscisses. |
| `y_field` | chaîne | non | défaut `Y` | CSV : colonne des ordonnées. |
| `raster` | `RasterPath` | non | GeoTIFF | **Epic 10, entrée 20** : raster à ouvrir (AD-24 amendée : un raster se désigne par `RasterPath`, jamais par un identifiant de couche). |
| `table` | chaîne | non | `.csv`, relatif au projet | **Epic 10, entrée 23** : table sans géométrie, ouverte comme couche vecteur sans géométrie (ni `crs`, ni `x_field`, ni `y_field`). |

Après les entrées 10.20 et 10.23, `path` devient facultatif : exactement un de `path`,
`raster` et `table` est donné, sinon l'erreur `layer.one_input`.

| Résultat | Type | Sens |
|---|---|---|
| `layer_id` | chaîne | Identifiant de la couche, valable pour la session. |
| `name` | chaîne | Nom affiché. |
| `project_crs` | chaîne | Système du projet. |
| `reprojected` | booléen | Vrai si QGIS reprojette à l'affichage. |
| `source_crs` | chaîne | Système de la couche (vide pour une table sans géométrie). |
| `kind` | chaîne | **Epics 10.20 et 10.23** : `vector`, `raster` ou `table`. |

```json fiche
{"id": "layer.add", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"path": {"type": "string", "required": true},
            "crs": {"type": "string", "required": false},
            "x_field": {"type": "string", "required": false},
            "y_field": {"type": "string", "required": false}},
 "result": ["layer_id", "name", "project_crs", "reprojected", "source_crs"],
 "planned_by": "epic 10, entrée 20 (raster) et entrée 23 (table)",
 "planned_params": {"path": {"type": "string", "required": false},
                    "raster": {"type": "RasterPath", "required": false},
                    "table": {"type": "string", "required": false}},
 "planned_result": ["kind"]}
```

### `layer.describe`

Décrit une couche : champs, étendue, système, nombre d'objets, type de géométrie. Ne
modifie rien.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche à décrire. |

| Résultat | Type | Sens |
|---|---|---|
| `crs` | chaîne | Système de la couche. |
| `extent` | emprise ou `null` | Étendue ; `null` pour une couche vide. |
| `feature_count` | entier | Nombre d'objets. |
| `fields` | liste de `{name, type}` | Champs de la table attributaire. |
| `geometry` | chaîne | `Point`, `Line`, `Polygon` ou `NoGeometry`. |
| `name` | chaîne | Nom affiché. |

```json fiche
{"id": "layer.describe", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true}},
 "result": ["crs", "extent", "feature_count", "fields", "geometry", "name"]}
```

### `layer.open_raster`

Ouvre dans le projet un raster écrit par une étape précédente. **Dictée par le noyau**
après chaque sortie raster (amendement 4.9 d'AD-9) ; refusée par `command.run`
(`registry.dictated`).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `path` | `RasterPath` | oui | `out:<k>.<n>` | Sortie d'une étape précédente. |

| Résultat | Type | Sens |
|---|---|---|
| `layer_id` | chaîne | Identifiant de la couche, valable pour la session. |
| `name` | chaîne | Radical du fichier (`1-terrain.slope`). |

```json fiche
{"id": "layer.open_raster", "status": "faite", "runs_in": "qgis", "dictated_by_core": true,
 "params": {"path": {"type": "RasterPath", "required": true}},
 "result": ["layer_id", "name"]}
```

### `layer.open_vector`

Ouvre dans le projet un GeoJSON écrit par une étape du noyau. **Dictée par le noyau** juste
après l'étape, avec le même `origin` et le même `plan_step`, jamais offerte au modèle ni au
client MCP, refusée par `command.run` (AD-30). Même traitement au rejeu que
`layer.open_raster`.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `path` | chaîne | oui | `out:<k>.<n>` | GeoJSON d'une étape précédente. |

| Résultat | Type | Sens |
|---|---|---|
| `layer_id` | chaîne | Identifiant de la couche, valable pour la session. |
| `name` | chaîne | Radical du fichier. |
| `crs` | chaîne | Système lu par QGIS, code explicite (`EPSG:2154`) : celui que déclare le fichier. |

Erreur : `layer.invalid_source` (fichier illisible ou système non reconnu).

```json fiche
{"id": "layer.open_vector", "status": "faite", "runs_in": "qgis", "dictated_by_core": true,
 "params": {"path": {"type": "string", "required": true}},
 "result": ["crs", "layer_id", "name"]}
```

### `analysis.buffer`

Zone tampon autour des entités d'une couche, ajoutée comme nouvelle couche. La couche doit
être dans un système en mètres (`analysis.crs_not_metric` sinon).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche vecteur à entourer. |
| `distance` | `Distance` | oui | `m`, `km` | Distance du tampon. |
| `dissolve` | booléen | non | défaut faux | Fusionner les tampons en une entité, seulement si la phrase le demande. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count` (couche créée).

```json fiche
{"id": "analysis.buffer", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "distance": {"type": "Distance", "required": true},
            "dissolve": {"type": "boolean", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

### `analysis.intersect`

Parties de la première couche que la seconde recouvre, en nouvelle couche.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche à découper. |
| `overlay_layer_id` | chaîne | oui | — | Couche masque, différente de `layer_id`. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`.

```json fiche
{"id": "analysis.intersect", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "overlay_layer_id": {"type": "string", "required": true},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

### `analysis.select_by_attribute`

Garde les entités dont un champ vérifie une comparaison, en nouvelle couche (AD-22 : un
champ, un opérateur, une valeur, jamais une expression).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche à filtrer. |
| `field` | chaîne | oui | — | Champ, tel que `layer.describe` le donne. |
| `operator` | chaîne | oui | `eq`, `ne`, `lt`, `lte`, `gt`, `gte`, `contains`, `begins_with` | Comparaison. |
| `value` | chaîne | oui | — | Valeur, toujours en texte ; QGIS la convertit. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`.

```json fiche
{"id": "analysis.select_by_attribute", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "field": {"type": "string", "required": true},
            "operator": {"type": "string", "required": true,
                         "enum": ["eq", "ne", "lt", "lte", "gt", "gte", "contains", "begins_with"]},
            "value": {"type": "string", "required": true},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

### `analysis.zonal_statistics`

Compte, pour chaque polygone d'une couche de zones, les points d'une seconde couche. **Remplacée
par `analysis.count_points_in_polygon`** (epic 10, entrée 2), qui retire cette fiche et passe
la sienne en `faite` dans le même commit.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche de zones (polygones). |
| `points_layer_id` | chaîne | oui | — | Couche de points, différente de `layer_id`. |
| `label_field` | chaîne | non | — | Champ qui nomme chaque zone ; sinon son numéro. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`, `zones` (liste de `{label, count}`,
100 au plus), `truncated`.

```json fiche
{"id": "analysis.zonal_statistics", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "replaced_by": "analysis.count_points_in_polygon",
 "params": {"layer_id": {"type": "string", "required": true},
            "points_layer_id": {"type": "string", "required": true},
            "label_field": {"type": "string", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name", "truncated", "zones"]}
```

### `analysis.isochrone`

Zone atteignable en un temps donné depuis un point, sur un réseau de lignes apporté par
l'utilisateur (`analysis.origin_off_network` si le point est trop loin du réseau).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche de lignes du réseau. |
| `origin` | `Point` | oui | — | Point de départ. |
| `duration` | `Duration` | oui | `s`, `min` | Durée maximale du trajet. |
| `mode` | chaîne | oui | `walk` (5 km/h), `car` (50 km/h) | Mode de déplacement. |
| `out` | chaîne | non | — | Fourni par le noyau. |
| `network` | `NetworkOptions` | non | — | **Epic 10, entrée 6** : vitesse, sens et tolérance (AD-28) ; absent, les valeurs du `mode`. |

Résultat : `layer_id`, `name`, `crs`, `feature_count` (un polygone).

```json fiche
{"id": "analysis.isochrone", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "origin": {"type": "Point", "required": true},
            "duration": {"type": "Duration", "required": true},
            "mode": {"type": "string", "required": true, "enum": ["walk", "car"]},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"],
 "planned_by": "epic 10, entrée 6",
 "planned_params": {"network": {"type": "NetworkOptions", "required": false}}}
```

### `layout.export_pdf`

Export PDF de la carte : couches visibles, titre, légende, barre d'échelle, flèche du nord,
sources. Une page A4 paysage par défaut ; format, orientation et carroyage au choix
(epic 11, entrée 11).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `title` | chaîne | oui | non vide | Titre imprimé en haut de page. |
| `scale` | entier | non | 100 à 100 000 000 | Dénominateur d'échelle, seulement si la demande en donne une. |
| `center` | `Point` | non | — | Fourni par le noyau au rejeu seulement (ticket 5.38). |
| `leave_out` | liste de chaînes | non | — | Fourni par le noyau au rejeu seulement (ticket 5.36). |
| `out` | chaîne | non | — | Fourni par le noyau. |
| `page_size` | chaîne | non | `a4` (défaut), `a3`, `a2`, `a1`, `a0` | Format de page. |
| `orientation` | chaîne | non | `landscape` (défaut), `portrait` | Orientation de la page. |
| `grid` | union étiquetée par `kind` | non | voir ci-dessous | Carroyage ; absent, aucun. |

`grid` est l'une de deux formes : `{"kind": "degrees", "interval": Angle}` (graticule en
WGS84, étiquettes en degrés décimaux) ou `{"kind": "projected", "epsg": entier, "interval":
Distance}` (quadrillage dans le système nommé ; un carroyage UTM au kilomètre trace les
lignes MGRS, sans leurs étiquettes, la grille de QGIS n'en ayant pas). Un `epsg` de grille
inconnu de QGIS rend l'erreur `data.unknown_crs`. Pas de nombre d'exemplaires :
l'impression reste hors du produit.

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | PDF écrit. |
| `page_count` | entier | Nombre de pages (1). |
| `layer_count` | entier | Couches visibles dessinées. |
| `sources` | liste de chaînes, 50 au plus | Fichiers sources des couches visibles. |
| `truncated` | booléen | Vrai si `sources` est tronquée. |
| `scale` | entier ou `null` | Échelle tenue, si demandée. |
| `center` | `Point` ou `null` | Centre employé, si `scale` demandée ; journalisé pour le rejeu. |
| `skipped_layers` | liste de chaînes | **Epic 10, entrée 15** : couches distantes (WMS, WMTS, XYZ, tuiles vectorielles distantes) écartées de la carte et nommées. |

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
 "planned_by": "epic 10, entrée 15 (skipped_layers)",
 "planned_result": ["skipped_layers"]}
```

### `terrain.slope`

Pente de chaque cellule d'un MNT, méthode de Horn (3 × 3), en degrés ou en pourcentage.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `dem` | `RasterPath` | oui | GeoTIFF une bande, système projeté en mètres | MNT : chemin relatif au projet, ou `step:k`. |
| `unit` | chaîne | oui | `degrees`, `percent` | Unité de la pente. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoTIFF Float32 de la pente, sans-donnée -9999. |
| `min`, `max` | nombre ou `null` | Pente extrême sur les cellules valides. |
| `nodata_cells` | entier | Cellules sans donnée (bord compris). |
| `unit` | chaîne | Unité écrite. |

```json fiche
{"id": "terrain.slope", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"dem": {"type": "RasterPath", "required": true},
            "unit": {"type": "string", "required": true, "enum": ["degrees", "percent"]},
            "out": {"type": "string", "required": false}},
 "result": ["file", "max", "min", "nodata_cells", "unit"]}
```

### `terrain.viewshed`

Champ de vision depuis un point : cellules du MNT visibles. Observateur dans le système du
MNT (`terrain.crs_mismatch`), dans son emprise (`terrain.outside_dem`) et sur une cellule
valide (`terrain.no_elevation`) ; rayon d'au plus 500 cellules (`terrain.viewshed_too_large`).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `dem` | `RasterPath` | oui | GeoTIFF une bande, projeté en mètres | MNT : chemin relatif au projet, ou `step:k`. |
| `observer` | `Point` | oui | système du MNT | Position de l'observateur. |
| `observer_height` | `Height` | non | défaut 1,6 m | Hauteur de l'œil au-dessus du sol. |
| `target_height` | `Height` | non | défaut 0 m | Hauteur des cibles au-dessus du sol. |
| `radius` | `Distance` | non | défaut 5 km, ramené à la borne ; au plus 500 cellules | Rayon maximal. |
| `curvature` | booléen | non | défaut vrai | Courbure terrestre. |
| `refraction` | nombre | non | de 0 à moins de 1, défaut 0,13 | Coefficient de réfraction, si `curvature`. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoTIFF Float32 : 1 visible, 0 masqué, -9999 hors rayon ou trou. |
| `visible_cells` | entier | Cellules visibles (celle de l'observateur comprise). |
| `hidden_cells` | entier | Cellules du rayon masquées. |
| `nodata_cells` | entier | Cellules sans donnée. |

Dans un plan, un observateur d'un autre système que le MNT est converti par une étape `data.transform_point` que le noyau ajoute avant de montrer le plan (amendement d'AD-19, 2026-11-08) ; `terrain.crs_mismatch` reste l'erreur d'un appel direct.

```json fiche
{"id": "terrain.viewshed", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"dem": {"type": "RasterPath", "required": true},
            "observer": {"type": "Point", "required": true},
            "observer_height": {"type": "Height", "required": false},
            "target_height": {"type": "Height", "required": false},
            "radius": {"type": "Distance", "required": false},
            "curvature": {"type": "boolean", "required": false},
            "refraction": {"type": "number", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["file", "hidden_cells", "nodata_cells", "visible_cells"]}
```

### `terrain.line_of_sight`

Ligne de vue entre deux points sur un MNT. Mêmes erreurs que `terrain.viewshed`.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `dem` | `RasterPath` | oui | GeoTIFF une bande, projeté en mètres | MNT : chemin relatif au projet, ou `step:k`. |
| `observer` | `Point` | oui | système du MNT | Observateur. |
| `target` | `Point` | oui | système du MNT | Cible. |
| `observer_height` | `Height` | non | défaut 1,6 m | Hauteur de l'œil. |
| `target_height` | `Height` | non | défaut 0 m | Hauteur de la cible. |
| `curvature` | booléen | non | défaut vrai | Courbure terrestre. |
| `refraction` | nombre | non | de 0 à moins de 1, défaut 0,13 | Réfraction, si `curvature`. |

| Résultat | Type | Sens |
|---|---|---|
| `visible` | booléen | Vrai si aucun relief ne masque la cible. |
| `distance` | nombre | Distance horizontale, en mètres. |
| `obstruction` | `Point` ou `null` | Premier obstacle, dans le système du MNT. |
| `obstruction_distance` | nombre ou `null` | Distance au premier obstacle, en mètres. |
| `profile` | liste de `{distance, elevation}`, 256 au plus | Profil du sol. |
| `profile_step` | nombre | Écart entre deux échantillons, en mètres. |
| `nodata_samples` | entier | Échantillons sans altitude. |
| `unit` | chaîne | Toujours `m`. |

Dans un plan, un observateur ou une cible d'un autre système que le MNT est converti par une étape `data.transform_point` que le noyau ajoute avant de montrer le plan (amendement d'AD-19, 2026-11-08) ; `terrain.crs_mismatch` reste l'erreur d'un appel direct.

```json fiche
{"id": "terrain.line_of_sight", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"dem": {"type": "RasterPath", "required": true},
            "observer": {"type": "Point", "required": true},
            "target": {"type": "Point", "required": true},
            "observer_height": {"type": "Height", "required": false},
            "target_height": {"type": "Height", "required": false},
            "curvature": {"type": "boolean", "required": false},
            "refraction": {"type": "number", "required": false}},
 "result": ["distance", "nodata_samples", "obstruction", "obstruction_distance", "profile", "profile_step", "unit", "visible"]}
```

### `coord.convert`

Convertit **un** point entre WGS84 (EPSG:4326), UTM sur WGS84 (EPSG:32601 à 32660,
32701 à 32760) et MGRS, par les formules publiées de la NGA et de Karney (AD-19, amendement
du 2026-11-05). Jamais une couche, jamais un autre système : un point en Lambert-93 passe
d'abord par `data.transform_point`. Une conversion n'est jamais insérée en silence avant un
calcul : elle est une étape visible du plan. Plusieurs points font plusieurs étapes.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `point` | `Point` | non | EPSG 4326, 326NN ou 327NN | Point à convertir ; exactement un de `point` et `mgrs`. |
| `mgrs` | `MgrsRef` | non | — | Référence MGRS à convertir ; exactement un de `point` et `mgrs`. |
| `to` | liste de chaînes | oui | `wgs84`, `utm`, `mgrs` ; de 1 à 3, sans doublon | Formes demandées. |
| `utm_zone` | entier | non | 1 à 60 | Zone UTM imposée à la sortie `utm` ; absente, la zone normale du point, exceptions 32V et 31X à 37X comprises ; l'hémisphère suit la latitude. |
| `mgrs_precision` | entier | non | 1 à 5, défaut 5 | Chiffres par axe de la sortie `mgrs` (5 : 1 m ; 1 : 10 km). Troncature, pas arrondi. |

| Résultat | Type | Sens |
|---|---|---|
| `wgs84` | `Point` ou `null` | Présent si `wgs84` est demandé : x longitude, y latitude, en degrés décimaux. |
| `utm` | `Point` ou `null` | Présent si `utm` est demandé : x, y en mètres, `epsg` de la zone. |
| `mgrs` | `MgrsRef` ou `null` | Présent si `mgrs` est demandé, forme canonique. |

Une référence MGRS lue donne le **coin sud-ouest** de sa case (convention de la NGA) ; la
case doit toucher la bande annoncée, sinon `coord.invalid_mgrs` (« bande »). Le WGS84 rendu
est arrondi au milliardième de degré, l'UTM au millimètre. Chaque champ est utilisable comme
référence d'étape (AD-23) : `wgs84` et `utm` dans un paramètre `Point`, `mgrs` dans un
paramètre `MgrsRef`. Livrée par l'entrée 2 de l'epic 11 (`core/src/commands/coord_convert.rs`,
calculs dans `core/src/geodesy.rs`). Erreurs :
`geo.unsupported_crs` (système hors liste, nomme l'étape `data.transform_point`),
`coord.invalid_mgrs` (référence MGRS mal formée, partie fautive nommée),
`coord.one_input` (ni `point` ni `mgrs`, ou les deux), `coord.out_of_range` (latitude
hors de [-80, 84] pour UTM ou MGRS, ou zone imposée à plus d'une zone du point).

```json fiche
{"id": "coord.convert", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"point": {"type": "Point", "required": false},
            "mgrs": {"type": "MgrsRef", "required": false},
            "to": {"type": "array", "required": true,
                   "items": {"type": "string", "enum": ["wgs84", "utm", "mgrs"]}},
            "utm_zone": {"type": "integer", "required": false},
            "mgrs_precision": {"type": "integer", "required": false}},
 "result": ["wgs84", "utm", "mgrs"]}
```

### `geo.distance_azimuth`

Distance géodésique et azimuts entre deux points, sur l'ellipsoïde WGS84 (Karney), AD-31.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `from` | `Point` | oui | EPSG 4326, 326NN ou 327NN | Point de départ. |
| `to` | `Point` | oui | EPSG 4326, 326NN ou 327NN | Point d'arrivée ; peut être dans un autre système de la liste que `from`. |
| `unit` | chaîne | non | `deg` (défaut), `gon` | Unité des angles rendus. |

| Résultat | Type | Sens |
|---|---|---|
| `distance` | `Distance` | Distance géodésique, toujours en `m`. |
| `azimuth` | `Angle` | Azimut au départ, de `from` vers `to`, nord vrai, sens horaire. |
| `back_azimuth` | `Angle` | Azimut à l'arrivée, de `to` vers `from`. |

Distance rendue au millimètre, azimuts au milliardième d'unité, dans [0, 360) degrés ou
[0, 400) grades ; `back_azimuth` est l'azimut de marche à l'arrivée augmenté d'un demi-tour.
Livrée par l'entrée 3 de l'epic 11 (`core/src/commands/geo_distance_azimuth.rs`, calculs dans
`core/src/geodesic.rs`). Erreurs : `geo.unsupported_crs` (nomme `coord.convert` pour une
référence MGRS, `data.transform_point` pour un autre système) ; `geo.coincident_points` (les
deux points confondus, à moins d'un demi-millimètre : l'azimut n'est pas défini) ;
`coord.out_of_range` (point d'entrée invalide : WGS84 hors de [-90, 90] ou [-180, 180],
point UTM hors de sa zone, coordonnées non finies).

```json fiche
{"id": "geo.distance_azimuth", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"from": {"type": "Point", "required": true},
            "to": {"type": "Point", "required": true},
            "unit": {"type": "string", "required": false, "enum": ["deg", "gon"]}},
 "result": ["distance", "azimuth", "back_azimuth"]}
```

### `geo.destination_point`

Point atteint depuis un départ, à un azimut et une distance géodésiques (problème direct
de Karney), AD-31.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `from` | `Point` | oui | EPSG 4326, 326NN ou 327NN | Point de départ. |
| `azimuth` | `Angle` | oui | nord vrai, sens horaire | Azimut de départ. |
| `distance` | `Distance` | oui | `m`, `km` | Distance géodésique à parcourir. |

| Résultat | Type | Sens |
|---|---|---|
| `point` | `Point` | Point d'arrivée, dans la zone UTM de `from` (l'hémisphère suit la latitude), ou en WGS84 si `from` l'est. |

Un départ UTM donne une arrivée dans la même zone, l'hémisphère suivant la latitude (comme
`coord.convert`) ; WGS84 au milliardième de degré, UTM au millimètre. Un azimut hors de
[0, 360) degrés vaut son équivalent dans un tour. Livrée par l'entrée 3 de l'epic 11
(`core/src/commands/geo_destination_point.rs`). Erreurs : `geo.unsupported_crs` ;
`coord.out_of_range` si le point d'arrivée sort de la zone UTM de `from` de plus d'une zone,
ou de la grille UTM (80° S à 84° N), ou si `from` est invalide (WGS84 hors de [-90, 90] ou
[-180, 180], point UTM hors de sa zone, coordonnées non finies).

```json fiche
{"id": "geo.destination_point", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"from": {"type": "Point", "required": true},
            "azimuth": {"type": "Angle", "required": true},
            "distance": {"type": "Distance", "required": true}},
 "result": ["point"]}
```

### `geo.bearing_intersection`

Recoupement : intersection de deux visées géodésiques, chacune donnée par un point et un
azimut, AD-31. Seule compte l'intersection en avant des deux points, la plus proche.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `first` | `Point` | oui | EPSG 4326, 326NN ou 327NN | Première station. |
| `first_azimuth` | `Angle` | oui | nord vrai, sens horaire | Visée depuis `first`. |
| `second` | `Point` | oui | EPSG 4326, 326NN ou 327NN | Seconde station. |
| `second_azimuth` | `Angle` | oui | nord vrai, sens horaire | Visée depuis `second`. |

| Résultat | Type | Sens |
|---|---|---|
| `point` | `Point` | Intersection, dans la zone UTM de `first` (l'hémisphère suit la latitude), ou en WGS84 si `first` l'est. |
| `distance_from_first` | `Distance` | Distance géodésique de `first` à l'intersection, en `m`. |
| `distance_from_second` | `Distance` | Distance géodésique de `second` à l'intersection, en `m`. |

Deux visées sont parallèles quand elles se coupent sous un angle inférieur à 1°, ou portent
la même géodésique (décision du propriétaire, 2026-11-05). « En avant » s'entend à moins d'un
demi-tour de la Terre : la visée doit rester la géodésique la plus courte de la station au
point. Point rendu comme pour `geo.destination_point`, distances au millimètre. Livrée par
l'entrée 3 de l'epic 11 (`core/src/commands/geo_bearing_intersection.rs`). Erreurs :
`geo.unsupported_crs` ; `geo.no_intersection` (visées parallèles, le message donne l'angle, ou
qui ne se coupent pas en avant des deux stations) ; `coord.out_of_range` (stations en UTM et
point à plus d'une zone de celle de `first`, ou hors de la grille UTM ; ou station invalide :
WGS84 hors de [-90, 90] ou [-180, 180], point UTM hors de sa zone, coordonnées non finies).

```json fiche
{"id": "geo.bearing_intersection", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"first": {"type": "Point", "required": true},
            "first_azimuth": {"type": "Angle", "required": true},
            "second": {"type": "Point", "required": true},
            "second_azimuth": {"type": "Angle", "required": true}},
 "result": ["point", "distance_from_first", "distance_from_second"]}
```

### `data.transform_point`

Reprojection d'un point vers un système quelconque, **par QGIS** (AD-19, AD-31) : relie un
point WGS84 ou UTM au système d'un MNT, ou un point Lambert-93 à la géodésie du noyau.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `point` | `Point` | oui | tout code EPSG connu de QGIS | Point à reprojeter. |
| `target_epsg` | entier | oui | code EPSG | Système d'arrivée. |

| Résultat | Type | Sens |
|---|---|---|
| `point` | `Point` | Point reprojeté, `epsg` égal à `target_epsg`. |

Même code source et cible : le point est rendu tel quel. La reprojection est celle de QGIS
(`QgsCoordinateTransform`, contexte de transformation du projet), jamais du noyau ; aucune
couche n'est créée, aucun fichier écrit. Livrée par l'entrée 4 de l'epic 11
(`core/src/commands/data_transform_point.rs`, geste dédié `qgis-plugin/cassini_qgis/point_transform.py`).
Erreurs : `data.unknown_crs` (code EPSG inconnu de QGIS, source ou cible ; le message nomme le
code fautif) ; `data.transform_failed` (point que QGIS ne sait pas reprojeter : hors du
domaine du système, ou résultat non fini) ; `registry.invalid_params` (paramètre absent,
code négatif ou non entier, champ en trop).

```json fiche
{"id": "data.transform_point", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"point": {"type": "Point", "required": true},
            "target_epsg": {"type": "integer", "required": true}},
 "result": ["point"]}
```

### `terrain.aspect`

Exposition de chaque cellule d'un MNT, en degrés depuis le **nord du quadrillage** du MNT,
sens horaire, dans [0, 360) ; voisinage 3 × 3 de Horn, comme `terrain.slope` ; une cellule
plate (gradient nul) a l'exposition NoData (AD-31).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `dem` | `RasterPath` | oui | GeoTIFF une bande, projeté en mètres | MNT : chemin relatif au projet, ou `step:k`. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoTIFF Float32 de l'exposition en degrés, sans-donnée -9999. |
| `flat_cells` | entier | Cellules plates, mises en NoData. |
| `nodata_cells` | entier | Cellules sans donnée en tout (bord, trous et plates compris). |

Livrée par l'entrée 6 de l'epic 11 (`core/src/commands/terrain_aspect.rs`, fenêtre de Horn
commune dans `core/src/horn.rs`). Erreurs : celles du module raster unique (`dem.*`).

```json fiche
{"id": "terrain.aspect", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"dem": {"type": "RasterPath", "required": true},
            "out": {"type": "string", "required": false}},
 "result": ["file", "flat_cells", "nodata_cells"]}
```

### `terrain.hillshade`

Ombrage d'un MNT par un soleil placé à un azimut et une hauteur, voisinage de Horn. Azimut
rapporté au nord du quadrillage du MNT (AD-31).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `dem` | `RasterPath` | oui | GeoTIFF une bande, projeté en mètres | MNT : chemin relatif au projet, ou `step:k`. |
| `sun_azimuth` | `Angle` | non | défaut 315° | Direction d'où vient la lumière. |
| `sun_altitude` | `Angle` | non | de 0 à 90°, défaut 45° | Hauteur du soleil au-dessus de l'horizon. |
| `z_factor` | nombre | non | > 0, défaut 1 | Exagération verticale. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoTIFF Float32, de 0 (ombre) à 255 (pleine lumière), sans-donnée -9999. |
| `nodata_cells` | entier | Cellules sans donnée (bord et trous). |

Erreurs : `terrain.invalid_parameter`, qui nomme le paramètre (`sun_altitude` hors de 0 à
90°, `z_factor` nul ou négatif) ; erreurs du module raster unique (`dem.*`). Livrée par
l'entrée 6 de l'epic 11 (`core/src/commands/terrain_hillshade.rs`).

```json fiche
{"id": "terrain.hillshade", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"dem": {"type": "RasterPath", "required": true},
            "sun_azimuth": {"type": "Angle", "required": false},
            "sun_altitude": {"type": "Angle", "required": false},
            "z_factor": {"type": "number", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["file", "nodata_cells"]}
```

### `terrain.viewshed_cumulative`

Nombre d'observateurs qui voient chaque cellule, par le calcul de `terrain.viewshed`
répété pour chacun (mêmes hauteurs, rayon, courbure et réfraction pour tous). Liste
d'observateurs bornée par le schéma (AD-8).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `dem` | `RasterPath` | oui | GeoTIFF une bande, projeté en mètres | MNT : chemin relatif au projet, ou `step:k`. |
| `observers` | liste de `Point` | oui | de 1 à 32, système du MNT | Observateurs. |
| `observer_height` | `Height` | non | défaut 1,6 m | Hauteur de l'œil, pour chacun. |
| `target_height` | `Height` | non | défaut 0 m | Hauteur des cibles. |
| `radius` | `Distance` | non | défaut 5 km, ramené à la borne ; au plus 500 cellules | Rayon autour de chaque observateur. |
| `curvature` | booléen | non | défaut vrai | Courbure terrestre. |
| `refraction` | nombre | non | de 0 à moins de 1, défaut 0,13 | Réfraction, si `curvature`. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoTIFF Float32 : nombre d'observateurs qui voient la cellule (0 à n), -9999 hors de tous les rayons ou trou. |
| `observer_count` | entier | Nombre d'observateurs calculés. |
| `max_count` | entier | Compte le plus élevé atteint. |
| `visible_cells` | entier | Cellules vues par au moins un observateur. |
| `nodata_cells` | entier | Cellules sans donnée. |

Erreurs (comme `terrain.viewshed`, l'observateur fautif nommé par son rang,
« observateur n° 2 ») : `terrain.crs_mismatch`, `terrain.outside_dem`,
`terrain.no_elevation`, `terrain.viewshed_too_large`. Une liste vide ou de plus de 32
observateurs est refusée (`registry.invalid_params`, qui nomme `observers`). Tous les
observateurs sont contrôlés avant que rien ne soit écrit. Calcul non annulable : environ
5 s au pire par observateur au rayon maximal, soit environ 2 min 30 s pour 32 (décision du
propriétaire, 2026-11-06 : gardé tel quel, et dit). Livrée par l'entrée 7 de l'epic 11 (`core/src/commands/terrain_viewshed_cumulative.rs`).

Dans un plan, chaque observateur d'un autre système que le MNT est converti par une étape `data.transform_point` que le noyau ajoute avant de montrer le plan, la liste recevant `step:k.point` à sa place (amendement d'AD-19, 2026-11-08) ; `terrain.crs_mismatch` reste l'erreur d'un appel direct.

```json fiche
{"id": "terrain.viewshed_cumulative", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"dem": {"type": "RasterPath", "required": true},
            "observers": {"type": "array", "required": true, "items": "Point"},
            "observer_height": {"type": "Height", "required": false},
            "target_height": {"type": "Height", "required": false},
            "radius": {"type": "Distance", "required": false},
            "curvature": {"type": "boolean", "required": false},
            "refraction": {"type": "number", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["file", "max_count", "nodata_cells", "observer_count", "visible_cells"]}
```

### `terrain.peaks`

Sommets d'un MNT : cellules plus hautes que leurs 8 voisines, de proéminence et d'écart
minimaux, du plus haut au plus bas. Proéminence : dénivelé entre le sommet et le col le
plus haut qui le relie à un terrain plus haut ; pour le point culminant du MNT, dénivelé
jusqu'à la cellule valide la plus basse. Deux sommets plus proches que `min_distance` : seul
le plus haut est gardé. Écrit un GeoJSON de points (AD-30), propriétés `rank` (entier, 1
pour le plus haut), `elevation` et `prominence` (nombres, en mètres).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `dem` | `RasterPath` | oui | GeoTIFF une bande, projeté en mètres | MNT : chemin relatif au projet, ou `step:k`. |
| `min_prominence` | `Height` | non | défaut 10 m | Proéminence minimale. |
| `min_distance` | `Distance` | non | défaut 100 m | Écart horizontal minimal entre deux sommets gardés. |
| `max_count` | entier | non | 1 à 100, défaut 20 | Nombre maximal de sommets écrits. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoJSON écrit (format de 2008, `crs` du MNT). |
| `feature_count` | entier | Sommets écrits. |
| `peaks` | liste de `{point: Point, elevation, prominence}`, 100 au plus | Sommets, du plus haut au plus bas ; la liste entière se cite par `step:k.peaks`, un sommet seul ne se cite pas. |
| `truncated` | booléen | Vrai si des sommets au-delà de `max_count` ont été écartés. |
| `unit` | chaîne | Toujours `m`. |

Égalité : une cellule n'est sommet que si aucune voisine n'est plus haute et qu'aucune
voisine de même altitude ne la précède dans l'ordre de lecture (ligne, puis colonne) ; un
plateau donne un sommet, sa première cellule, sauf un plateau non convexe, qui peut donner un
second candidat de proéminence nulle, gardé seulement si `min_prominence` vaut 0. Bord : une cellule du bord du MNT, ou voisine
d'un trou, n'est jamais un sommet, mais compte comme terrain pour la proéminence. Ordre total
pour le tri et les égalités : altitude décroissante, puis ordre de lecture. Point au centre de
la cellule. Un résultat vide est un succès, qui écrit un GeoJSON sans entité. Erreurs : celles
de `dem.rs` telles quelles, `geojson.unwritable` à l'écriture ; `max_count` hors de 1 à 100,
une hauteur négative ou une distance nulle sont refusés (`registry.invalid_params`). Calcul
non annulable : au pire environ 15 octets par cellule en plus du MNT, soit environ 3,7 Gio
de plus et de l'ordre d'une à deux minutes au plafond de 1 Gio. Livrée par l'entrée 15 de l'epic 11 (`core/src/commands/terrain_peaks.rs`).

```json fiche
{"id": "terrain.peaks", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"dem": {"type": "RasterPath", "required": true},
            "min_prominence": {"type": "Height", "required": false},
            "min_distance": {"type": "Distance", "required": false},
            "max_count": {"type": "integer", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["feature_count", "file", "peaks", "truncated", "unit"]}
```

### `raster.reclassify`

Reclasse un raster mono-bande par une table de classes, **dans le noyau** (AD-20 ; déplacée
de l'entrée 25 de l'epic 10, même identifiant). Une classe couvre `[min, max[` ; `min` ou
`max` absent ouvre la borne. La première classe qui convient l'emporte ; une cellule hors
de toute classe devient NoData ; une cellule NoData reste NoData. Une classe dont `min`
est supérieur ou égal à `max` est refusée par l'erreur `raster.invalid_class`, qui nomme
son rang ; une classe de valeur -9999 (le sans-donnée écrit) l'est aussi. Livrée par l'entrée 9 de l'epic 11 (`core/src/commands/raster_reclassify.rs`).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `raster` | `RasterPath` | oui | une bande | Raster à reclasser. |
| `classes` | liste de `{min?, max?, value}` | oui | 1 à 32 classes ; nombres | Table de classes, dans l'unité du raster. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoTIFF Float32 reclassé, sans-donnée -9999. |
| `counts` | liste de `{value, cells}` | Cellules par valeur de sortie, dans l'ordre des classes. |
| `nodata_cells` | entier | Cellules sans donnée (entrée NoData ou hors classes). |

```json fiche
{"id": "raster.reclassify", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"raster": {"type": "RasterPath", "required": true},
            "classes": {"type": "array", "required": true,
                        "items": {"type": "object",
                                  "fields": {"min": {"type": "number", "required": false},
                                             "max": {"type": "number", "required": false},
                                             "value": {"type": "number", "required": true}}}},
            "out": {"type": "string", "required": false}},
 "result": ["counts", "file", "nodata_cells"]}
```

### `raster.weighted_overlay`

Superposition pondérée de rasters de même grille, **dans le noyau** (AD-20) : score =
somme des valeurs × poids, divisée par la somme des poids. Une cellule NoData dans un
critère donne NoData ; une cellule non nulle d'un masque d'exclusion donne NoData. Grilles
différentes : `raster.grid_mismatch`, qui nomme l'écart ; le noyau ne rééchantillonne
jamais (aligner d'abord par `raster.align`). Livrée par l'entrée 9 de l'epic 11 (`core/src/commands/raster_weighted_overlay.rs`).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `criteria` | liste de `{raster: RasterPath, weight}` | oui | 2 à 8 critères ; `weight` > 0 | Rasters de critères et leurs poids. |
| `exclusions` | liste de `RasterPath` | non | 0 à 8 | Masques : toute cellule non nulle et valide est exclue. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoTIFF Float32 du score, sans-donnée -9999. |
| `min`, `max` | nombre ou `null` | Score extrême sur les cellules valides. |
| `nodata_cells` | entier | Cellules sans donnée (critère NoData ou exclusion). |

```json fiche
{"id": "raster.weighted_overlay", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"criteria": {"type": "array", "required": true,
                         "items": {"type": "object",
                                   "fields": {"raster": {"type": "RasterPath", "required": true},
                                              "weight": {"type": "number", "required": true}}}},
            "exclusions": {"type": "array", "required": false, "items": "RasterPath"},
            "out": {"type": "string", "required": false}},
 "result": ["file", "max", "min", "nodata_cells"]}
```

### `terrain.cost_distance`

Coût cumulé depuis une ou plusieurs sources sur un raster de coût, par le module de coût
unique (AD-20) : coût par mètre parcouru ; cellule NoData, nulle ou négative
infranchissable ; 8 voisins, coût moyen des deux cellules × distance (taille de cellule,
× √2 en diagonale) ; un pas en diagonale demande que les deux cellules qu'il frôle soient
franchissables. Le coût cumulé est dans l'unité du raster × mètres.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `cost` | `RasterPath` | oui | une bande, projeté en mètres | Raster de coût. |
| `sources` | liste de `Point` | oui | de 1 à 32, système du raster | Sources (coût cumulé nul). |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoTIFF Float32 du coût cumulé, -9999 sur l'infranchissable et l'inatteignable. |
| `max` | nombre ou `null` | Coût cumulé le plus élevé atteint. |
| `reachable_cells` | entier | Cellules atteintes. |
| `unreachable_cells` | entier | Cellules franchissables mais inatteignables. |

Erreurs : `terrain.crs_mismatch`, `terrain.outside_dem` (source hors de l'emprise, nommée
par son rang), `terrain.impassable_source` (source sur une cellule infranchissable), la
source fautive nommée par son rang (« source n° 2 ») ; toutes les sources sont contrôlées
avant que rien ne soit écrit. Une liste vide ou de plus de 32 sources est refusée
(`registry.invalid_params`, qui nomme `sources`) ; une source répétée ne change rien. Calcul
non annulable : au pire 144 octets par cellule en plus du raster (cumul, prédécesseur, grille
écrite et tas), soit environ 36 Gio au plafond de 1 Gio. Livrée par l'entrée 10
de l'epic 11 (`core/src/cost.rs`, `core/src/commands/terrain_cost_distance.rs`).

Dans un plan, chaque source d'un autre système que le raster de coût est convertie par une étape `data.transform_point` que le noyau ajoute avant de montrer le plan, la liste recevant `step:k.point` à sa place (amendement d'AD-19, 2026-11-08) ; `terrain.crs_mismatch` reste l'erreur d'un appel direct.

```json fiche
{"id": "terrain.cost_distance", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"cost": {"type": "RasterPath", "required": true},
            "sources": {"type": "array", "required": true, "items": "Point"},
            "out": {"type": "string", "required": false}},
 "result": ["file", "max", "reachable_cells", "unreachable_cells"]}
```

### `terrain.least_cost_path`

Chemin de moindre coût du départ à l'arrivée, dans cet ordre, sur le module de coût unique
(mêmes règles que `terrain.cost_distance`, dont le pas en diagonale permis seulement si les
deux cellules qu'il frôle sont franchissables). Écrit un GeoJSON d'une ligne (AD-30),
propriétés `total_cost` (nombre, unité du raster × mètres) et `length` (nombre, mètres).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `cost` | `RasterPath` | oui | une bande, projeté en mètres | Raster de coût. |
| `start` | `Point` | oui | système du raster | Départ. |
| `end` | `Point` | oui | système du raster | Arrivée. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoJSON écrit (une ligne, format de 2008, `crs` du raster). |
| `feature_count` | entier | Toujours 1. |
| `total_cost` | nombre | Coût total, unité du raster × mètres. |
| `length` | `Distance` | Longueur de la ligne, en `m`. |

Erreurs : `terrain.crs_mismatch`, `terrain.outside_dem`, `terrain.impassable_source`
(départ ou arrivée infranchissable), `terrain.no_path` (arrivée inatteignable) ; l'extrémité
fautive est nommée (« départ », « arrivée ») et rien n'est écrit. Sommets aux centres des
cellules, les sommets intermédiaires alignés retirés ; `total_cost` vaut le coût cumulé à la
cellule d'arrivée. Départ et arrivée dans une même cellule : deux sommets confondus, coût et
longueur nuls. Livrée par l'entrée 10 de l'epic 11
(`core/src/commands/terrain_least_cost_path.rs`).

Dans un plan, un départ ou une arrivée d'un autre système que le raster de coût est converti par une étape `data.transform_point` que le noyau ajoute avant de montrer le plan (amendement d'AD-19, 2026-11-08) ; `terrain.crs_mismatch` reste l'erreur d'un appel direct.

```json fiche
{"id": "terrain.least_cost_path", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"cost": {"type": "RasterPath", "required": true},
            "start": {"type": "Point", "required": true},
            "end": {"type": "Point", "required": true},
            "out": {"type": "string", "required": false}},
 "result": ["feature_count", "file", "length", "total_cost"]}
```

---

## 6. Commandes planifiées — epic 11, géométrie et terrain étendu

Les commandes des entrées 1 à 15 sont faites (§5). Le changement de cap du 2026-11-08
(`../change-styles-topographiques-officiels/change-styles-topographiques-officiels.md`)
ajoute la mise en forme topographique, entrées 19 à 21. Les fiches ci-dessous sont faites
dans le noyau par l'entrée 20 (`core/src/style/`, `core/src/commands/`) ; les gestes QGIS de
`layer.set_style` et `layer.arrange` viennent avec l'entrée 21.

### `map.topographic_style`

Met les couches du projet aux couleurs de la sémiologie publique de l'IGN (carte au
1:25 000, BD TOPO) et du standard CNIG pour les zones d'un PLU, puis les range par thème
(entrées 20 et 21). Sur demande seulement ; aucun fichier d'origine n'est modifié.

Le noyau lit l'inventaire du projet auprès de QGIS, comme pour composer un plan : une
lecture, qui n'est pas une étape du journal. Il reconnaît le thème de chaque couche
d'après son nom, son fichier et ses champs (par exemple `nature` et `importance` de la
BD TOPO, `typezone` du PLU), et compose son `StyleSpec` à partir de sa table. Une classe
par attribut est posée quand la donnée le permet : routes par importance, zones du PLU par
type, bâti par usage, MNT en teintes d'altitude.

Il dicte ensuite, juste après la ligne de `map.topographic_style` dans le journal, un
`layer.set_style` par couche reconnue, puis un seul `layer.arrange`, qui range **toutes**
les couches de l'inventaire en groupes par thème et dans l'ordre d'affichage (amendement
d'AD-30), de haut en bas :

- « Autres couches » : les vecteurs non reconnus, souvent les résultats de l'utilisateur
  (tampons, parcelles sélectionnées), en haut de tout ;
- les groupes des thèmes, points, puis lignes, puis surfaces ;
- « Autres rasters » : les rasters non reconnus (pente, champ de vision…) ;
- « Relief » en dessous de tout, l'ombrage au-dessus des teintes d'altitude.

Une couche non reconnue garde son style ; un MNT non reconnu ne recouvre donc jamais la
carte. Avec `layers`, seules ces couches sont mises en forme, mais le rangement porte
toujours sur tout le projet.

Le thème se lit dans le nom de la couche, dans le **radical** de son fichier (et le
`layername=` d'un GeoPackage), jamais dans ses dossiers. Un raster dérivé (pente,
exposition, visibilité, coût) n'est jamais un MNT. Les valeurs de `typezone` rejoignent
leur zone par préfixe (`1AU`, `AUc` : AUc ; `2AU`, `AUs` : AUs ; `Ah`, `Ap` : A ; `Nh`,
`Nd` : N).

**Sans QGIS** (CLI `run`, MCP autonome), ou si l'extension n'annonce pas
`project.inventory`, `layer.set_style` et `layer.arrange`, la commande n'est pas offerte
(AD-25) ; appelée quand même, elle est refusée par `style.unavailable`, sans rien écrire.
En relais MCP, elle est offerte dès que le catalogue de la session la marque disponible ;
seul le mode MCP autonome l'écarte.

**Au rejeu**, chaque ligne est rejouée seule (AD-11) : la ligne de `map.topographic_style`
relit l'inventaire, sans les couches d'une exécution précédente, et compare son résultat,
mais ne dicte rien ; les lignes dictées qui la suivent sont rejouées avec leurs paramètres
journalisés, `StyleSpec` compris. Les mêmes styles et le même rangement reviennent donc. Un
rejeu dont l'exécutant n'annonce pas les gestes d'une ligne de mise en forme (la commande,
ou la ligne dictée elle-même) s'arrête à cette ligne sur `style.unavailable`.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layers` | liste de chaînes | non | identifiants de couche | Couches à mettre en forme ; absent : toutes les couches du projet. |

| Résultat | Type | Sens |
|---|---|---|
| `styled` | liste | `{layer_id, theme}` : couches mises en forme, avec le thème reconnu. |
| `unchanged` | liste | `{layer_id, reason}` : couches laissées telles quelles, avec la raison. |
| `groups` | liste | `{name, layers}` : les groupes posés, de haut en bas. |

Erreurs : `qgis.layer_missing`, une couche de `layers` absente du projet ; `style.invalid`,
une liste `layers` vide ; `style.unavailable`, sans QGIS, sans les gestes, ou un inventaire
sans liste de couches ou dont une couche n'a pas d'identifiant.

```json fiche
{"id": "map.topographic_style", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"layers": {"type": "array", "required": false, "items": "string"}},
 "result": ["groups", "styled", "unchanged"]}
```

### `map.graduated_style`

Colore une couche vecteur en dégradé selon un champ numérique, par exemple « colore les
communes selon la population » (décision du propriétaire du 2026-11-08, entrée 20 ; geste :
21). Offerte au modèle. Le noyau ne voit pas les valeurs : il compose un `StyleSpec`
`graduated`, avec les couleurs `from` et `to` du dégradé nommé prises dans sa table
(ColorBrewer, licence Apache-2.0 ; `red_blue` est divergent, du rouge au bleu), et QGIS
calcule les seuils sur les vraies valeurs. Le noyau dicte ensuite un seul `layer.set_style`,
juste après la ligne de la commande, comme pour `map.topographic_style` ; au rejeu, la ligne
mère ne dicte rien et la ligne dictée est rejouée. Aucun inventaire n'est lu : l'extension
vérifie que le champ existe et qu'il est numérique (`style.field_missing`).

**Sans QGIS** (CLI `run`, MCP autonome), ou si l'extension n'annonce pas `layer.set_style`,
la commande n'est pas offerte ; appelée quand même, elle est refusée par `style.unavailable`,
sans rien écrire. En relais MCP, elle est offerte dès que le catalogue de la session la marque
disponible ; seul le mode MCP autonome l'écarte. Un `layer_id` vide est refusé par
`style.invalid`.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | identifiant de couche | Couche vecteur à colorer. |
| `field` | chaîne | oui | champ numérique | Champ du dégradé. |
| `method` | chaîne | non | `equal_interval`, `quantile`, `jenks` ; défaut `jenks` | Calcul des seuils, par QGIS. |
| `class_count` | entier | non | 2 à 12 ; défaut 5 | Nombre de classes (`style.invalid` hors des bornes). |
| `ramp` | chaîne | non | `yellow_red`, `blues`, `greens`, `reds`, `purples`, `greys`, `red_blue` ; défaut `yellow_red` | Dégradé nommé. |

| Résultat | Type | Sens |
|---|---|---|
| `layer_id` | chaîne | La couche colorée (une couche existante : genre de sortie `none`). |
| `field` | chaîne | Le champ du dégradé. |
| `class_count` | entier | Le nombre de classes demandé. |

Erreurs : `style.invalid`, `style.unavailable` ; celles de `layer.set_style`.

```json fiche
{"id": "map.graduated_style", "status": "faite", "runs_in": "core", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "field": {"type": "string", "required": true},
            "method": {"type": "string", "required": false, "enum": ["equal_interval", "quantile", "jenks"]},
            "class_count": {"type": "integer", "required": false},
            "ramp": {"type": "string", "required": false, "enum": ["yellow_red", "blues", "greens", "reds", "purples", "greys", "red_blue"]}},
 "result": ["class_count", "field", "layer_id"]}
```

### `layer.set_style`

Applique un `StyleSpec` à une couche du projet (entrée 21). Dictée par le noyau après
`map.topographic_style` ou `map.graduated_style`, jamais offerte au modèle, au client MCP ni au formulaire. Le style
est celui de la couche dans le projet : le fichier de données n'est pas touché.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | identifiant de couche | Couche à mettre en forme. |
| `style` | `StyleSpec` | oui | — | Le style à poser. |

Résultat : `layer_id`. Erreurs : `qgis.layer_missing`, `style.invalid`,
`style.field_missing` (le `field` d'une classe absent de la couche), `style.geometry_mismatch`
(une rampe ou un ombrage sur un vecteur, un symbole sur un raster).

```json fiche
{"id": "layer.set_style", "status": "faite", "runs_in": "qgis", "dictated_by_core": true,
 "params": {"layer_id": {"type": "string", "required": true},
            "style": {"type": "StyleSpec", "required": true}},
 "result": ["layer_id"]}
```

### `layer.arrange`

Range les couches du projet dans des groupes et dans un ordre d'affichage (entrée 21,
décision du propriétaire du 2026-11-08 : « groupes + ordre »). Dictée par le noyau après les
`layer.set_style` de `map.topographic_style`. Les groupes sont donnés de haut en bas, et les
couches de chaque groupe de haut en bas ; le noyau choisit les noms des groupes (AD-5).
Dans une exécution normale, le noyau liste chaque couche qu'il connaît, une fois. Une
couche du projet absente de la liste n'est **pas** une erreur : elle reste au-dessus des
groupes, intacte. C'est le cas au rejeu, où les couches posées par une exécution précédente
sont encore au projet.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `groups` | liste | oui | `{name, layers}` | Groupes à poser, de haut en bas ; `layers`, identifiants de couche de haut en bas. |

Résultat : `group_count`, `layer_count`. Erreurs : `qgis.layer_missing` ;
`arrange.invalid` (un groupe vide, une couche citée deux fois).

```json fiche
{"id": "layer.arrange", "status": "faite", "runs_in": "qgis", "dictated_by_core": true,
 "params": {"groups": {"type": "array", "required": true,
                       "items": {"type": "object",
                                 "fields": {"name": {"type": "string", "required": true},
                                            "layers": {"type": "array", "required": true, "items": "string"}}}}},
 "result": ["group_count", "layer_count"]}
```

---

## 7. Commandes planifiées — epic 10, catalogue étendu (après la démo)

Toutes déléguées à QGIS (une ligne de la table fermée, AD-24), sauf `geocode.ban` (AD-26).
Un résumé chiffré suit le gabarit d'AD-27 (`rows`, `truncated`), validé par le noyau.

### `analysis.count_points_in_polygon`

Renommage de `analysis.zonal_statistics` (epic 10, entrée 2), résultat au gabarit d'AD-27,
avec somme pondérée facultative. Chaque ligne de `rows` porte `count` (unité `points`) et,
si `weight_field` est donné, `weighted_sum` (unité vide).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche de zones (polygones). |
| `points_layer_id` | chaîne | oui | — | Couche de points, différente de `layer_id`. |
| `label_field` | chaîne | non | — | Champ qui nomme chaque zone ; sinon son rang. |
| `weight_field` | chaîne | non | champ numérique des points | Pondération : somme de ce champ par zone. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`, `rows`, `truncated`.

```json fiche
{"id": "analysis.count_points_in_polygon", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "points_layer_id": {"type": "string", "required": true},
            "label_field": {"type": "string", "required": false},
            "weight_field": {"type": "string", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name", "rows", "truncated"]}
```

### `data.reproject`

Reprojette une couche entière vers un système, en nouvelle couche (epic 10, entrée 7).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche à reprojeter. |
| `target_epsg` | entier | oui | code EPSG | Système d'arrivée. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`.

La reprojection est celle de QGIS (`native:reprojectlayer`, contexte de transformation du
projet), jamais du noyau (AD-19) ; la couche d'origine n'est pas modifiée, la nouvelle couche
est écrite en `.gpkg` dans le dossier de travail. Même code source et cible : la couche est
copiée. Livrée par l'entrée 7 de l'epic 10 (`core/src/commands/data_reproject.rs`, ligne de
la table `ALGORITHMS` de `qgis-plugin/cassini_qgis/analysis.py`). Erreurs : `data.unknown_crs`
(code cible inconnu de QGIS, nommé dans le message, rien lancé) ; `layer.not_found`,
`layer.not_vector` ; `analysis.failed` (échec de l'algorithme dans QGIS, ou couche produite
illisible) ; `qgis.cancelled` ; `registry.invalid_params` (paramètre absent, code nul,
négatif ou non entier, champ en trop).

```json fiche
{"id": "data.reproject", "status": "faite", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "target_epsg": {"type": "integer", "required": true},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

### `analysis.difference`

Entités d'une couche privées de ce qu'une couche de superposition recouvre (epic 10,
entrée 21). Un recouvrement total rend une couche vide, nommée dans le résultat.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche à découper. |
| `overlay_layer_id` | chaîne | oui | — | Couche à retrancher, différente de `layer_id`. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`.

```json fiche
{"id": "analysis.difference", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "overlay_layer_id": {"type": "string", "required": true},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

### `analysis.area`

Ajoute un champ de surface plane (dans le système projeté de la couche, formule fixe de la
ligne de table) sur une couche nouvelle (epic 10, entrée 22). Couche en degrés :
`analysis.crs_not_metric`.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche de polygones. |
| `unit` | chaîne | oui | `m2`, `ha` | Unité du champ de surface. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `layer_id`, `name`, `crs`, `feature_count` | — | Couche créée. |
| `field` | chaîne | Nom du champ de surface ajouté. |
| `unit` | chaîne | Unité du champ (`m2` ou `ha`), que reprend la sélection qui suit. |

```json fiche
{"id": "analysis.area", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "unit": {"type": "string", "required": true, "enum": ["m2", "ha"]},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "field", "layer_id", "name", "unit"]}
```

### `data.convert`

Écrit une couche dans un autre format ; le noyau déduit l'extension du fichier du format
(AD-10, AD-24) ; l'original est intact (epic 10, entrée 9).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche à convertir. |
| `format` | chaîne | oui | `gpkg`, `shp`, `geojson` | Format de sortie. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | Fichier écrit. |
| `format` | chaîne | Format écrit. |
| `feature_count` | entier | Entités écrites. |

```json fiche
{"id": "data.convert", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "format": {"type": "string", "required": true, "enum": ["gpkg", "shp", "geojson"]},
            "out": {"type": "string", "required": false}},
 "result": ["feature_count", "file", "format"]}
```

### `analysis.nearest`

Jointure au plus proche voisin : chaque entité reçoit les attributs de l'entité la plus
proche d'une seconde couche, et la distance (epic 10, entrée 10).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche qui reçoit le voisin. |
| `target_layer_id` | chaîne | oui | — | Couche des voisins, différente de `layer_id`. |
| `max_distance` | `Distance` | non | `m`, `km` | Distance au-delà de laquelle aucun voisin n'est joint ; absente, sans limite. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `layer_id`, `name`, `crs`, `feature_count` | — | Couche créée. |
| `distance_field` | chaîne | Nom du champ de distance. |
| `unit` | chaîne | Unité du champ de distance (`m`). |
| `unmatched_count` | entier | Entités sans voisin dans `max_distance`. |

```json fiche
{"id": "analysis.nearest", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "target_layer_id": {"type": "string", "required": true},
            "max_distance": {"type": "Distance", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "distance_field", "feature_count", "layer_id", "name", "unit", "unmatched_count"]}
```

### `analysis.distance_matrix`

Matrice des distances entre les entités de deux couches de points ; livre la sortie
`table` du geste générique (epic 10, entrée 10). Chaque ligne de `rows` : une entité de
départ, `values` donnant la distance vers chaque cible (unité `m`).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Points de départ. |
| `target_layer_id` | chaîne | oui | — | Points d'arrivée. |
| `label_field` | chaîne | non | — | Champ qui nomme les départs ; sinon le rang. |
| `target_label_field` | chaîne | non | — | Champ qui nomme les arrivées ; sinon le rang. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | Table écrite. |
| `rows` | liste de `SummaryRow` | Distances, bornées par le noyau. |
| `truncated` | booléen | Vrai si la table dépasse `rows`. |

```json fiche
{"id": "analysis.distance_matrix", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "target_layer_id": {"type": "string", "required": true},
            "label_field": {"type": "string", "required": false},
            "target_label_field": {"type": "string", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["file", "rows", "truncated"]}
```

### `analysis.statistics_by_category`

Compte et somme par valeur d'un champ de catégorie (epic 10, entrée 11). Chaque ligne de
`rows` : une catégorie, `values` portant `count` et, si `sum_field` est donné, `sum`.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche à résumer. |
| `category_field` | chaîne | oui | — | Champ de catégorie. |
| `sum_field` | chaîne | non | champ numérique | Champ à sommer par catégorie. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | Table écrite. |
| `rows` | liste de `SummaryRow` | Une ligne par catégorie, bornée. |
| `truncated` | booléen | Vrai si les catégories dépassent `rows`. |

```json fiche
{"id": "analysis.statistics_by_category", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "category_field": {"type": "string", "required": true},
            "sum_field": {"type": "string", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["file", "rows", "truncated"]}
```

### `analysis.sum_line_lengths`

Longueur des lignes contenues dans chaque polygone (epic 10, entrée 11). Couche en degrés :
`analysis.crs_not_metric`.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche de polygones. |
| `lines_layer_id` | chaîne | oui | — | Couche de lignes. |
| `label_field` | chaîne | non | — | Champ qui nomme chaque polygone ; sinon son rang. |
| `unit` | chaîne | non | `m` (défaut), `km` | Unité des longueurs. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`, `rows` (valeur `length` avec
unité), `truncated`.

```json fiche
{"id": "analysis.sum_line_lengths", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "lines_layer_id": {"type": "string", "required": true},
            "label_field": {"type": "string", "required": false},
            "unit": {"type": "string", "required": false, "enum": ["m", "km"]},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name", "rows", "truncated"]}
```

### `analysis.join_table`

Rattache à une couche les colonnes d'une table CSV sans géométrie, ouverte comme couche
vecteur sans géométrie, par un champ commun (epic 10, entrée 23).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche qui reçoit les colonnes. |
| `field` | chaîne | oui | — | Champ commun, côté couche. |
| `table_layer_id` | chaîne | oui | — | Table sans géométrie. |
| `table_field` | chaîne | oui | — | Champ commun, côté table. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `layer_id`, `name`, `crs`, `feature_count` | — | Couche créée. |
| `unmatched_count` | entier | Entités sans correspondance dans la table. |

```json fiche
{"id": "analysis.join_table", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "field": {"type": "string", "required": true},
            "table_layer_id": {"type": "string", "required": true},
            "table_field": {"type": "string", "required": true},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name", "unmatched_count"]}
```

### `analysis.dissolve`

Regroupe les entités par un ou plusieurs champs, sans agréger d'autre champ (epic 10,
entrée 24).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche à fusionner. |
| `fields` | liste de chaînes | oui | 1 à 4 champs | Champs de regroupement. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`.

```json fiche
{"id": "analysis.dissolve", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "fields": {"type": "array", "required": true, "items": "string"},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

### `raster.zonal_statistics`

Moyenne, minimum et maximum d'un raster par polygone, MNT compris (epic 10, entrée 12) ;
livre le genre `raster` du geste générique. Le raster est un `RasterPath` (amendement
d'AD-24) ; aucun calcul de `terrain.*` n'est refait. Chaque ligne de `rows` porte `mean`,
`min` et `max`, avec l'unité du raster (`m` pour un MNT).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `raster` | `RasterPath` | oui | une bande | Raster à résumer. |
| `layer_id` | chaîne | oui | — | Couche de polygones. |
| `label_field` | chaîne | non | — | Champ qui nomme chaque polygone ; sinon son rang. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`, `rows`, `truncated`.

```json fiche
{"id": "raster.zonal_statistics", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"raster": {"type": "RasterPath", "required": true},
            "layer_id": {"type": "string", "required": true},
            "label_field": {"type": "string", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name", "rows", "truncated"]}
```

### `raster.polygonize`

Polygones des zones de même valeur d'un raster (algorithme visé : `gdal:polygonize`, à
vérifier au ticket), champ `value` sur chaque polygone.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `raster` | `RasterPath` | oui | une bande | Raster à vectoriser (par exemple reclassé). |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`.

```json fiche
{"id": "raster.polygonize", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"raster": {"type": "RasterPath", "required": true},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

### `raster.align`

Rééchantillonne un raster sur la grille d'un raster de référence (système, taille de
cellule, origine, dimensions), pour qu'il se combine dans le noyau (AD-20). Algorithme visé
`native:alignrasters`, repli `native:alignsingleraster`, à trancher au ticket. Le noyau
dicte ensuite `layer.open_raster`.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `raster` | `RasterPath` | oui | une bande | Raster à aligner. |
| `reference` | `RasterPath` | oui | une bande | Raster dont la grille est reprise. |
| `resampling` | chaîne | non | `nearest` (défaut), `bilinear` | Méthode de rééchantillonnage ; `nearest` pour des classes. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | GeoTIFF aligné. |

```json fiche
{"id": "raster.align", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"raster": {"type": "RasterPath", "required": true},
            "reference": {"type": "RasterPath", "required": true},
            "resampling": {"type": "string", "required": false, "enum": ["nearest", "bilinear"]},
            "out": {"type": "string", "required": false}},
 "result": ["file"]}
```

### `raster.contour`

Courbes de niveau d'un MNT à l'équidistance demandée (`gdal:contour`, epic 10, entrée 26),
champ `elevation` (mètres) sur chaque ligne. Non annoncée si l'algorithme manque (AD-25).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `raster` | `RasterPath` | oui | MNT une bande | MNT. |
| `interval` | `Distance` | oui | `m`, `km` | Équidistance. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`.

```json fiche
{"id": "raster.contour", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"raster": {"type": "RasterPath", "required": true},
            "interval": {"type": "Distance", "required": true},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

### `network.shortest_path`

Plus court chemin entre deux points sur un réseau de lignes, sur le contrat de réseau
(AD-28 ; epic 10, entrée 13). Un point trop loin du réseau : `analysis.origin_off_network`.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche de lignes du réseau. |
| `start` | `Point` | oui | — | Départ. |
| `end` | `Point` | oui | — | Arrivée. |
| `mode` | chaîne | oui | `walk` (5 km/h), `car` (50 km/h) | Mode, comme `analysis.isochrone`. |
| `network` | `NetworkOptions` | non | — | Vitesse, sens, tolérance ; absent, les valeurs du mode. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `layer_id`, `name`, `crs`, `feature_count` | — | Couche du chemin. |
| `length` | `Distance` | Longueur du chemin. |
| `duration` | `Duration` | Durée du trajet. |

```json fiche
{"id": "network.shortest_path", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "start": {"type": "Point", "required": true},
            "end": {"type": "Point", "required": true},
            "mode": {"type": "string", "required": true, "enum": ["walk", "car"]},
            "network": {"type": "NetworkOptions", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "duration", "feature_count", "layer_id", "length", "name"]}
```

### `network.prepare`

Prépare une couche de chemins bruts en réseau lisible par `analysis.isochrone` et
`network.shortest_path` : accrochage des extrémités, découpe aux croisements, suppression
des géométries nulles et en double, par des algorithmes `native:` seuls (epic 10,
entrée 14).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `layer_id` | chaîne | oui | — | Couche de lignes brutes. |
| `tolerance` | `Distance` | non | défaut 1 m | Distance d'accrochage des extrémités. |
| `out` | chaîne | non | — | Fourni par le noyau. |

Résultat : `layer_id`, `name`, `crs`, `feature_count`.

```json fiche
{"id": "network.prepare", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"layer_id": {"type": "string", "required": true},
            "tolerance": {"type": "Distance", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "feature_count", "layer_id", "name"]}
```

### `layout.export_atlas`

Atlas PDF : une page par entité d'une couche de couverture, avec les éléments de
`layout.export_pdf` ; livre le genre `layout` (epic 10, entrée 15). Seules les couches
locales sont dessinées ; les couches distantes sont écartées et nommées (même règle pour
`layout.export_pdf`). Aucune expression de filtre ou de tri (AD-23).

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `title` | chaîne | oui | non vide | Titre de chaque page. |
| `coverage_layer_id` | chaîne | oui | — | Couche de couverture, une page par entité. |
| `label_field` | chaîne | non | — | Champ ajouté au titre de chaque page. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `file` | chaîne | PDF écrit. |
| `page_count` | entier | Nombre de pages. |
| `layer_count` | entier | Couches dessinées. |
| `skipped_layers` | liste de chaînes | Couches distantes écartées. |
| `sources` | liste de chaînes, 50 au plus | Fichiers sources. |
| `truncated` | booléen | Vrai si `sources` est tronquée. |

```json fiche
{"id": "layout.export_atlas", "status": "planifiee", "runs_in": "qgis", "dictated_by_core": false,
 "params": {"title": {"type": "string", "required": true},
            "coverage_layer_id": {"type": "string", "required": true},
            "label_field": {"type": "string", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["file", "layer_count", "page_count", "skipped_layers", "sources", "truncated"]}
```

### `geocode.ban`

Géocodage d'un CSV d'adresses sur un CSV de la Base Adresse Nationale, **dans le noyau**
(AD-26 ; epic 10, entrée 16). Écrit un CSV déterministe avec `lon` et `lat` en EPSG:4326,
qu'ouvre ensuite une étape `layer.add` explicite. Non annulable.

| Paramètre | Type | Oblig. | Unité / valeurs | Sens |
|---|---|---|---|---|
| `addresses` | chaîne | oui | CSV relatif au projet | Fichier d'adresses. |
| `ban` | chaîne | oui | CSV relatif au projet | Fichier de la BAN. |
| `address_field` | chaîne | oui | — | Colonne de l'adresse (numéro et voie). |
| `postcode_field` | chaîne | non | — | Colonne du code postal. |
| `city_field` | chaîne | non | — | Colonne de la commune. |
| `out` | chaîne | non | — | Fourni par le noyau. |

| Résultat | Type | Sens |
|---|---|---|
| `path` | chaîne | CSV écrit, relatif au projet. |
| `crs` | chaîne | Toujours `EPSG:4326`. |
| `x_field` | chaîne | `lon`. |
| `y_field` | chaîne | `lat`. |
| `matched_count` | entier | Adresses trouvées. |
| `unmatched_count` | entier | Adresses introuvables. |

```json fiche
{"id": "geocode.ban", "status": "planifiee", "runs_in": "core", "dictated_by_core": false,
 "params": {"addresses": {"type": "string", "required": true},
            "ban": {"type": "string", "required": true},
            "address_field": {"type": "string", "required": true},
            "postcode_field": {"type": "string", "required": false},
            "city_field": {"type": "string", "required": false},
            "out": {"type": "string", "required": false}},
 "result": ["crs", "matched_count", "path", "unmatched_count", "x_field", "y_field"]}
```

---

## 8. Commandes réservées (après la démo)

Identifiants retenus ; paramètres et résultat écrits plus tard, par un changement de cap.

### `terrain.flow_accumulation`

Accumulation d'écoulement sur un MNT.

```json fiche
{"id": "terrain.flow_accumulation", "status": "reservee", "runs_in": "core"}
```

### `terrain.watershed`

Bassin versant d'un exutoire.

```json fiche
{"id": "terrain.watershed", "status": "reservee", "runs_in": "core"}
```

### `terrain.flood_extent`

Emprise inondée pour une hauteur d'eau.

```json fiche
{"id": "terrain.flood_extent", "status": "reservee", "runs_in": "core"}
```

### `terrain.radio_coverage`

Couverture radio d'un émetteur sur le relief.

```json fiche
{"id": "terrain.radio_coverage", "status": "reservee", "runs_in": "core"}
```
