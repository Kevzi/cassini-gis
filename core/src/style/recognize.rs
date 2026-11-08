//! Reconnaissance déterministe du thème d'une couche, d'après son nom, son fichier, ses champs et
//! les valeurs de ses champs texte (changement de cap du 2026-11-08, décision 3).
//!
//! L'entrée est la forme brute de `project.inventory` (`crate::commands::project_inventory`).
//! Les règles, dans l'ordre : la sorte de couche (raster ou vecteur, et sa géométrie), puis les
//! champs connus (`typezone` du PLU, `importance` d'un tronçon de route, `usage_1` du bâti), puis
//! les mots du nom et du fichier, puis les valeurs du champ texte `nature` de la BD TOPO. Les
//! valeurs n'existent que pour un champ texte : la reconnaissance par valeurs ne vaut que pour eux.
//! Même inventaire, même résultat.

use serde_json::Value;

use super::table::Theme;
use crate::agent_inventory::{field_type, geometry, is_text};

/// Sorte de couche, pour la reconnaissance et le rangement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Shape {
    Point,
    Line,
    Surface,
    Raster,
    /// Vecteur sans géométrie, autre sorte de QGIS, ou sorte inconnue.
    Other,
}

impl Shape {
    /// Rang d'affichage, du haut vers le bas : points, lignes, surfaces, rasters, autres.
    pub fn rank(self) -> u8 {
        match self {
            Shape::Point => 0,
            Shape::Line => 1,
            Shape::Surface => 2,
            Shape::Other => 3,
            Shape::Raster => 4,
        }
    }
}

/// Un champ d'une couche vecteur.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldFacts {
    /// Nom du champ, tel que la couche le porte.
    pub name: String,
    /// Type de QGIS (`String`, `Integer64`, `Real`…).
    pub qgis_type: String,
    /// Quelques valeurs distinctes, pour un champ texte seulement.
    pub values: Vec<String>,
}

/// Ce que le noyau sait d'une couche de l'inventaire.
#[derive(Debug, Clone, PartialEq)]
pub struct LayerFacts {
    pub layer_id: String,
    pub name: String,
    pub source: String,
    pub shape: Shape,
    pub fields: Vec<FieldFacts>,
}

impl LayerFacts {
    /// Lit une couche brute de l'inventaire ; `None` sans `layer_id`.
    pub fn from_raw(raw: &Value) -> Option<LayerFacts> {
        let text = |key: &str| raw.get(key).and_then(Value::as_str).unwrap_or_default();
        let layer_id = raw.get("layer_id")?.as_str()?.to_string();
        let wkb = raw.get("geometry").and_then(Value::as_str);
        let shape = match (
            raw.get("kind").and_then(Value::as_str),
            wkb.and_then(geometry),
        ) {
            (Some("raster"), _) => Shape::Raster,
            (Some("vector") | None, Some(g)) => match g {
                "Point" | "MultiPoint" => Shape::Point,
                "LineString" | "MultiLineString" => Shape::Line,
                _ => Shape::Surface,
            },
            _ => Shape::Other,
        };
        let fields = raw
            .get("fields")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|f| {
                let name = f.get("name")?.as_str()?.to_string();
                let qgis_type = f
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                let values = if is_text(&qgis_type) {
                    f.get("values")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                } else {
                    Vec::new()
                };
                Some(FieldFacts {
                    name,
                    qgis_type,
                    values,
                })
            })
            .collect();
        Some(LayerFacts {
            layer_id,
            name: text("name").to_string(),
            source: text("source").to_string(),
            shape,
            fields,
        })
    }

    /// Le premier champ dont le nom, sans casse, est l'un de `names` (dans l'ordre de `names`).
    pub fn field_named(&self, names: &[&str]) -> Option<&FieldFacts> {
        names.iter().find_map(|wanted| {
            self.fields
                .iter()
                .find(|f| f.name.eq_ignore_ascii_case(wanted))
        })
    }

    /// Comme `field_named`, pour un champ texte seulement.
    pub fn text_field_named(&self, names: &[&str]) -> Option<&FieldFacts> {
        names.iter().find_map(|wanted| {
            self.fields
                .iter()
                .find(|f| f.name.eq_ignore_ascii_case(wanted) && is_text(&f.qgis_type))
        })
    }

    /// Comme `field_named`, pour un champ entier ou réel seulement.
    pub fn numeric_field_named(&self, names: &[&str]) -> Option<&FieldFacts> {
        names.iter().find_map(|wanted| {
            self.fields.iter().find(|f| {
                f.name.eq_ignore_ascii_case(wanted) && field_type(&f.qgis_type) != "string"
            })
        })
    }
}

/// Mots du nom et du fichier, ramenés à une seule forme : minuscules, sans accents, séparés et
/// entourés de `_`. « Cours d'eau » devient `_cours_d_eau_`.
fn words(layer: &LayerFacts) -> String {
    let mut out = String::from("_");
    for c in format!("{} {}", layer.name, source_words(&layer.source)).chars() {
        let folded = fold(c);
        if folded.is_empty() {
            if !out.ends_with('_') {
                out.push('_');
            }
        } else {
            out.push_str(folded);
        }
    }
    if !out.ends_with('_') {
        out.push('_');
    }
    out
}

/// Ce que la source dit de la couche : le radical du fichier, sans dossier ni extension, et le
/// `layername=` d'un GeoPackage s'il y en a un. Les dossiers ne comptent jamais : un fichier rangé
/// sous `.../mnt/` ou `.../PLU_2020/` n'en devient ni un MNT ni un zonage.
fn source_words(source: &str) -> String {
    let mut parts = source.split('|');
    let path = parts.next().unwrap_or_default();
    let file = path.rsplit(['/', '\\']).next().unwrap_or_default();
    let stem = match file.rsplit_once('.') {
        Some((stem, _)) if !stem.is_empty() => stem,
        _ => file,
    };
    let mut out = stem.to_string();
    for part in parts {
        if let Some(name) = part.trim().strip_prefix("layername=") {
            out.push(' ');
            out.push_str(name);
        }
    }
    out
}

/// Un caractère en minuscule ASCII sans accent ; vide pour un séparateur.
fn fold(c: char) -> &'static str {
    const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz0123456789";
    let lower = c.to_lowercase().next().unwrap_or(c);
    let folded = match lower {
        'à' | 'â' | 'ä' | 'á' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' => 'i',
        'ô' | 'ö' | 'ó' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ç' => 'c',
        'œ' => return "oe",
        other => other,
    };
    match LETTERS.find(folded) {
        Some(i) => &LETTERS[i..i + 1],
        None => "",
    }
}

/// Vrai si l'un des mots ou groupes de mots de `phrases` (écrits avec `_`) est dans `words`.
fn has(words: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|p| words.contains(&format!("_{p}_")))
}

const HILLSHADE_WORDS: &[&str] = &[
    "hillshade",
    "ombrage",
    "ombre",
    "ombrages",
    "relief_ombre",
    "estompage",
    "shade",
    "shaded_relief",
];
/// Mots d'un raster dérivé d'un MNT (pente, exposition, visibilité, coût) : jamais un MNT, même
/// si son nom dit aussi `mnt` (`pente_mnt.tif`).
const DERIVED_WORDS: &[&str] = &[
    "pente",
    "pentes",
    "slope",
    "aspect",
    "exposition",
    "viewshed",
    "champ_de_vision",
    "visib",
    "visibilite",
    "visible",
    "intervisibilite",
    "cout",
    "couts",
    "cost",
];
const DEM_WORDS: &[&str] = &[
    "mnt",
    "mns",
    "dem",
    "dtm",
    "dsm",
    "rgealti",
    "bdalti",
    "srtm",
    "alti",
    "altitude",
    "altitudes",
    "elevation",
    "relief",
];
const PLU_WORDS: &[&str] = &["plu", "plui", "zone_urba", "zonage", "zonage_plu"];
const ROAD_WORDS: &[&str] = &[
    "route",
    "routes",
    "troncon_de_route",
    "voirie",
    "road",
    "roads",
];
const RAIL_WORDS: &[&str] = &[
    "voie_ferree",
    "voies_ferrees",
    "ferre",
    "ferree",
    "ferrees",
    "rail",
    "rails",
    "railway",
    "railways",
];
const STATION_WORDS: &[&str] = &["gare", "gares", "station", "stations"];
const WATERCOURSE_WORDS: &[&str] = &[
    "cours_d_eau",
    "troncon_hydrographique",
    "riviere",
    "rivieres",
    "ruisseau",
    "ruisseaux",
    "fleuve",
    "fleuves",
    "canal",
    "canaux",
    "river",
    "rivers",
    "stream",
    "streams",
    "waterway",
    "waterways",
];
const WATER_BODY_WORDS: &[&str] = &[
    "plan_d_eau",
    "plans_d_eau",
    "surface_hydrographique",
    "surface_en_eau",
    "lac",
    "lacs",
    "etang",
    "etangs",
    "water",
    "eau",
];
const HYDRO_WORDS: &[&str] = &["hydro", "hydrographie", "hydrographique", "hydrography"];
const VEGETATION_WORDS: &[&str] = &[
    "vegetation",
    "zone_de_vegetation",
    "foret",
    "forets",
    "bois",
    "boise",
    "boisement",
    "boisements",
    "forest",
    "forests",
    "wood",
    "woods",
];
const BUILDING_WORDS: &[&str] = &[
    "batiment",
    "batiments",
    "bati",
    "building",
    "buildings",
    "chateau_d_eau",
    "chateaux_d_eau",
];
const ADMIN_WORDS: &[&str] = &[
    "commune",
    "communes",
    "limite",
    "limites",
    "limite_administrative",
    "limites_administratives",
    "arrondissement",
    "arrondissements",
    "departement",
    "departements",
    "region",
    "regions",
    "epci",
    "canton",
    "cantons",
    "admin_express",
];
const PARCEL_WORDS: &[&str] = &[
    "parcelle",
    "parcelles",
    "cadastre",
    "cadastral",
    "cadastrale",
    "cadastrales",
    "parcellaire",
];
const CONTOUR_WORDS: &[&str] = &[
    "courbe",
    "courbes",
    "courbe_de_niveau",
    "courbes_de_niveau",
    "contour",
    "contours",
    "isohypse",
    "isohypses",
];

/// Débuts de valeurs du champ `nature` de la BD TOPO, par thème (comparés sans casse ni accent).
const NATURE_STARTS: &[(Shape, Theme, &[&str])] = &[
    (
        Shape::Line,
        Theme::Watercourses,
        &["cours_d_eau", "canal", "ruisseau", "riviere", "ecoulement"],
    ),
    (
        Shape::Line,
        Theme::Roads,
        &[
            "route",
            "type_autoroutier",
            "bretelle",
            "rond_point",
            "chemin",
            "sentier",
            "piste_cyclable",
        ],
    ),
    (
        Shape::Surface,
        Theme::Vegetation,
        &[
            "foret",
            "bois",
            "haie",
            "lande",
            "vigne",
            "verger",
            "peupleraie",
        ],
    ),
    (
        Shape::Surface,
        Theme::WaterBodies,
        &["lac", "etang", "plan_d_eau", "retenue", "bassin", "mare"],
    ),
];

/// Le thème d'une couche, ou `None` si rien ne la désigne.
pub fn recognize(layer: &LayerFacts) -> Option<Theme> {
    let w = words(layer);
    match layer.shape {
        Shape::Raster => {
            if has(&w, HILLSHADE_WORDS) {
                Some(Theme::Hillshade)
            } else if has(&w, DERIVED_WORDS) || has_prefix(&w, "visib") {
                None
            } else if has(&w, DEM_WORDS) {
                Some(Theme::Dem)
            } else {
                None
            }
        }
        Shape::Other => None,
        Shape::Point => has(&w, STATION_WORDS).then_some(Theme::Stations),
        Shape::Line => {
            // Un nom de cours d'eau ou de courbe l'emporte sur le champ `importance`.
            let named = [
                (WATERCOURSE_WORDS, Theme::Watercourses),
                (CONTOUR_WORDS, Theme::Contours),
            ];
            if let Some(theme) = by_words(&w, &named) {
                return Some(theme);
            }
            if layer.field_named(&["importance"]).is_some() && !has(&w, RAIL_WORDS) {
                return Some(Theme::Roads);
            }
            let by_name = [
                (RAIL_WORDS, Theme::Railways),
                (ROAD_WORDS, Theme::Roads),
                (WATERCOURSE_WORDS, Theme::Watercourses),
                (HYDRO_WORDS, Theme::Watercourses),
                (ADMIN_WORDS, Theme::AdminLimits),
            ];
            by_words(&w, &by_name).or_else(|| by_nature(layer))
        }
        Shape::Surface => {
            if layer.field_named(&["typezone"]).is_some() {
                return Some(Theme::PluZones);
            }
            if layer.field_named(&["usage_1", "usage1"]).is_some() {
                return Some(Theme::Buildings);
            }
            // Un zonage nommé sans champ `typezone` n'a rien à classer : il n'est pas reconnu.
            if has(&w, PLU_WORDS) {
                return None;
            }
            // Le bâti avant l'eau : un `chateau_d_eau` est un bâtiment.
            let by_name = [
                (BUILDING_WORDS, Theme::Buildings),
                (WATER_BODY_WORDS, Theme::WaterBodies),
                (HYDRO_WORDS, Theme::WaterBodies),
                (VEGETATION_WORDS, Theme::Vegetation),
                (PARCEL_WORDS, Theme::Parcels),
                (ADMIN_WORDS, Theme::AdminLimits),
            ];
            by_words(&w, &by_name).or_else(|| by_nature(layer))
        }
    }
}

/// Vrai si un mot entier de `words` commence par `prefix` (`visib` : `visibilite`, `visible`…).
fn has_prefix(words: &str, prefix: &str) -> bool {
    words.split('_').any(|token| token.starts_with(prefix))
}

fn by_words(w: &str, rules: &[(&[&str], Theme)]) -> Option<Theme> {
    rules
        .iter()
        .find(|(phrases, _)| has(w, phrases))
        .map(|(_, theme)| *theme)
}

/// Thème déduit des valeurs du champ texte `nature`, si elles désignent toutes un même thème
/// de la géométrie de la couche.
fn by_nature(layer: &LayerFacts) -> Option<Theme> {
    let field = layer.text_field_named(&["nature"])?;
    let mut found: Option<Theme> = None;
    for value in &field.values {
        let folded = words(&LayerFacts {
            name: value.clone(),
            source: String::new(),
            ..layer.clone()
        });
        let theme = NATURE_STARTS
            .iter()
            .filter(|(shape, _, _)| *shape == layer.shape)
            .find(|(_, _, starts)| starts.iter().any(|s| folded.starts_with(&format!("_{s}_"))))
            .map(|(_, theme, _)| *theme);
        match (theme, found) {
            (None, _) => {}
            (Some(t), None) => found = Some(t),
            (Some(t), Some(f)) if t == f => {}
            (Some(_), Some(_)) => return None,
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn layer(raw: Value) -> LayerFacts {
        LayerFacts::from_raw(&raw).unwrap()
    }

    fn vector(name: &str, source: &str, geometry: &str, fields: Value) -> Option<Theme> {
        recognize(&layer(
            json!({"layer_id": "x", "name": name, "source": source,
            "kind": "vector", "geometry": geometry, "fields": fields}),
        ))
    }

    fn raster(name: &str, source: &str) -> Option<Theme> {
        recognize(&layer(
            json!({"layer_id": "x", "name": name, "source": source,
            "kind": "raster", "geometry": null, "fields": []}),
        ))
    }

    #[test]
    fn words_fold_case_accents_and_separators() {
        let l = layer(json!({"layer_id": "x", "name": "Cours d’eau", "source": "Hydro.GPKG"}));
        assert_eq!(words(&l), "_cours_d_eau_hydro_");
        let l = layer(json!({"layer_id": "x", "name": "Voies ferrées", "source": ""}));
        assert_eq!(words(&l), "_voies_ferrees_");
    }

    #[test]
    fn villeneuve_layers_are_recognized() {
        let typezone =
            json!([{"name": "typezone", "type": "String", "values": ["A", "AU", "N", "U"]}]);
        assert_eq!(
            vector("zonage", "plu.gpkg", "MultiPolygon", typezone),
            Some(Theme::PluZones)
        );
        let importance = json!([{"name": "importance", "type": "String", "values": ["1", "6"]}]);
        assert_eq!(
            vector(
                "troncon_de_route",
                "bdtopo.gpkg",
                "MultiLineStringZ",
                importance
            ),
            Some(Theme::Roads)
        );
        assert_eq!(
            vector("Gares", "gares.gpkg", "Point", json!([])),
            Some(Theme::Stations)
        );
        assert_eq!(
            vector(
                "parcelles",
                "cadastre-94077-parcelles.json",
                "MultiPolygon",
                json!([])
            ),
            Some(Theme::Parcels)
        );
        assert_eq!(
            raster("RGEALTI_FXX_0650_6855_MNT_LAMB93", "mnt.tif"),
            Some(Theme::Dem)
        );
    }

    #[test]
    fn annecy_layers_are_recognized() {
        assert_eq!(
            vector(
                "cours_d_eau",
                "cours_d_eau.gpkg",
                "MultiLineString",
                json!([])
            ),
            Some(Theme::Watercourses)
        );
        assert_eq!(
            vector("plan_d_eau", "hydro.gpkg", "MultiPolygon", json!([])),
            Some(Theme::WaterBodies)
        );
        let usage = json!([{"name": "usage_1", "type": "String", "values": ["Résidentiel"]}]);
        assert_eq!(
            vector("batiment", "batiment.shp", "MultiPolygonZ", usage),
            Some(Theme::Buildings)
        );
        assert_eq!(
            vector("COMMUNE", "COMMUNE.shp", "MultiPolygon", json!([])),
            Some(Theme::AdminLimits)
        );
        assert_eq!(raster("mnt_annecy_2154", "mnt.tif"), Some(Theme::Dem));
        assert_eq!(raster("srtm_38_03", "srtm_38_03.tif"), Some(Theme::Dem));
        assert_eq!(
            raster("3-terrain.hillshade", "3-terrain.hillshade.tif"),
            Some(Theme::Hillshade)
        );
    }

    #[test]
    fn an_unknown_layer_is_not_recognized() {
        assert_eq!(
            vector("export_2024", "export_2024.gpkg", "MultiPolygon", json!([])),
            None
        );
        assert_eq!(raster("1-terrain.slope", "1-terrain.slope.tif"), None);
        // Un zonage sans `typezone` n'a rien à classer.
        assert_eq!(
            vector("zonage", "plu.gpkg", "MultiPolygon", json!([])),
            None
        );
        // Une géométrie qui ne va pas au thème : des gares en lignes ne sont pas des gares.
        assert_eq!(vector("gares", "gares.gpkg", "LineString", json!([])), None);
        // Une table sans géométrie.
        assert_eq!(
            recognize(&layer(
                json!({"layer_id": "t", "name": "routes", "kind": "vector",
                "geometry": null})
            )),
            None
        );
    }

    #[test]
    fn text_values_recognize_only_text_fields() {
        let nature = json!([{"name": "nature", "type": "String",
            "values": ["Forêt fermée de feuillus", "Haie"]}]);
        assert_eq!(
            vector("couche_a", "a.gpkg", "MultiPolygon", nature),
            Some(Theme::Vegetation)
        );
        // Valeurs d'un champ entier : ignorées (l'extension n'en rend pas).
        let numeric = json!([{"name": "nature", "type": "Integer", "values": ["Forêt fermée"]}]);
        assert_eq!(vector("couche_a", "a.gpkg", "MultiPolygon", numeric), None);
        // Valeurs de thèmes différents : rien n'est décidé.
        let mixed =
            json!([{"name": "nature", "type": "String", "values": ["Lac", "Forêt fermée"]}]);
        assert_eq!(vector("couche_a", "a.gpkg", "MultiPolygon", mixed), None);
    }

    #[test]
    fn only_the_file_stem_and_the_layer_name_count_never_the_folders() {
        // Un fichier rangé sous `mnt/` n'est pas un MNT ; un vecteur sous `PLU_2020/` n'est pas
        // un zonage.
        assert_eq!(
            raster("carte", "C:/Users/kevin/donnees/mnt/carte_scannee.tif"),
            None
        );
        assert_eq!(
            vector(
                "couche",
                "/home/kevin/donnees/PLU_2020/gares/export.shp",
                "MultiPolygon",
                json!([])
            ),
            None
        );
        // Le radical et le `layername=` d'un GeoPackage, eux, comptent.
        assert_eq!(
            raster("carte", "D:\\donnees\\relief\\RGEALTI_MNT_LAMB93.asc"),
            Some(Theme::Dem)
        );
        assert_eq!(
            vector(
                "couche",
                "/home/kevin/donnees/mnt/bdtopo.gpkg|layername=cours_d_eau",
                "MultiLineString",
                json!([])
            ),
            Some(Theme::Watercourses)
        );
    }

    #[test]
    fn a_raster_derived_from_a_dem_is_never_a_dem() {
        for (name, source) in [
            ("pente_mnt", "pente_mnt.tif"),
            ("mnt_slope", "mnt_slope.tif"),
            ("exposition du MNT", "expo.tif"),
            ("visibilite_mnt", "v.tif"),
            ("cout_mnt", "c.tif"),
        ] {
            assert_eq!(raster(name, source), None, "{name}");
        }
        assert_eq!(raster("mnt", "pente_mnt.tif"), None);
        assert_eq!(raster("mnt", "mnt.tif"), Some(Theme::Dem));
    }

    #[test]
    fn line_names_win_over_importance_and_buildings_win_over_water() {
        let importance = json!([{"name": "importance", "type": "String"}]);
        assert_eq!(
            vector("cours_d_eau", "c.gpkg", "LineString", importance.clone()),
            Some(Theme::Watercourses)
        );
        assert_eq!(
            vector(
                "courbes de niveau",
                "c.gpkg",
                "LineString",
                importance.clone()
            ),
            Some(Theme::Contours)
        );
        assert_eq!(
            vector("x", "r.gpkg", "LineString", importance),
            Some(Theme::Roads)
        );
        assert_eq!(
            vector("Châteaux d'eau", "c.gpkg", "MultiPolygon", json!([])),
            Some(Theme::Buildings)
        );
        // Mots entiers seulement : `eaux_usees` ne contient pas le mot `eau`.
        assert_eq!(
            vector("reseau_eaux_usees", "r.gpkg", "MultiPolygon", json!([])),
            None
        );
    }

    #[test]
    fn the_same_layer_always_gives_the_same_theme() {
        let raw = json!({"layer_id": "x", "name": "Routes", "source": "r.gpkg",
            "kind": "vector", "geometry": "LineString"});
        let first = recognize(&layer(raw.clone()));
        for _ in 0..10 {
            assert_eq!(recognize(&layer(raw.clone())), first);
        }
        assert_eq!(first, Some(Theme::Roads));
    }
}
