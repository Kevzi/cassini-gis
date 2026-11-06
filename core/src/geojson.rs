//! Écrivain GeoJSON unique du noyau (AD-30, ticket 11.8).
//!
//! Toute commande du noyau qui rend des entités (points hauts, chemin de moindre coût) les
//! écrit par ce module, et par lui seul : un GeoJSON au format de 2008, celui qu'écrit et lit
//! par défaut le pilote GeoJSON de GDAL, qui déclare le système du raster source par le membre
//! `"crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::<code>"}}`. L'écart à
//! la RFC 7946, qui impose WGS84, est voulu : le noyau ne reprojette pas (AD-19).
//!
//! La sortie est déterministe, pour que le rejeu compare le fichier octet pour octet : entités
//! et propriétés dans l'ordre donné par l'appelant, nombres écrits par `serde_json` (toujours de
//! la même façon), aucune date, une seule ligne suivie d'un saut de ligne. Un nombre non fini
//! est refusé avant toute écriture : rien n'est écrit. Le noyau ne lit jamais de GeoJSON.

use std::fmt;
use std::path::Path;

use serde::ser::{SerializeMap, SerializeSeq, SerializeStruct};
use serde::{Serialize, Serializer};

/// Extensions de fichier d'une sortie vecteur du noyau : la session dicte `layer.open_vector`
/// après une étape du noyau dont la sortie porte l'une d'elles.
pub const VECTOR_EXTENSIONS: [&str; 1] = [".geojson"];

/// Géométrie d'une entité, en coordonnées du système déclaré (est, nord pour un système
/// projeté). Seuls le point et la ligne sont écrits.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    Point(f64, f64),
    LineString(Vec<(f64, f64)>),
}

/// Valeur d'une propriété : entier ou nombre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Property {
    Integer(i64),
    Number(f64),
}

/// Une entité : sa géométrie et ses propriétés, dans l'ordre où elles seront écrites.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub geometry: Geometry,
    pub properties: Vec<(&'static str, Property)>,
}

/// Erreur typée de l'écrivain, à code stable `geojson.unwritable` (AD-21), message en français
/// qui ne nomme que le fichier, jamais son chemin absolu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeojsonError {
    pub code: &'static str,
    pub message: String,
}

impl GeojsonError {
    fn unwritable(message: impl Into<String>) -> Self {
        Self {
            code: "geojson.unwritable",
            message: message.into(),
        }
    }
}

impl fmt::Display for GeojsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for GeojsonError {}

/// Écrit `features` dans `path`, en une fois, dans le système `EPSG:<epsg>`.
pub fn write(path: &Path, epsg: u32, features: &[Feature]) -> Result<(), GeojsonError> {
    let bytes = to_bytes(epsg, features)?;
    std::fs::write(path, bytes).map_err(|_| {
        GeojsonError::unwritable(format!(
            "Impossible d'écrire le GeoJSON « {} » : dossier absent ou non accessible en écriture.",
            file_name(path)
        ))
    })
}

/// Octets exacts du fichier : une ligne compacte et un saut de ligne final.
fn to_bytes(epsg: u32, features: &[Feature]) -> Result<Vec<u8>, GeojsonError> {
    if epsg == 0 {
        return Err(GeojsonError::unwritable(
            "Code EPSG 0 non inscriptible en GeoJSON.",
        ));
    }
    for (i, feature) in features.iter().enumerate() {
        check_finite(i + 1, feature)?;
    }
    let mut bytes = serde_json::to_vec(&Collection { epsg, features })
        .map_err(|e| GeojsonError::unwritable(format!("Écriture du GeoJSON : {e}.")))?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn check_finite(rank: usize, feature: &Feature) -> Result<(), GeojsonError> {
    let coordinates_finite = match &feature.geometry {
        Geometry::Point(x, y) => x.is_finite() && y.is_finite(),
        Geometry::LineString(points) => points.iter().all(|(x, y)| x.is_finite() && y.is_finite()),
    };
    if !coordinates_finite {
        return Err(GeojsonError::unwritable(format!(
            "L'entité {rank} a une coordonnée non finie : rien n'est écrit."
        )));
    }
    for (name, value) in &feature.properties {
        if matches!(value, Property::Number(v) if !v.is_finite()) {
            return Err(GeojsonError::unwritable(format!(
                "La propriété « {name} » de l'entité {rank} n'est pas finie : rien n'est écrit."
            )));
        }
    }
    Ok(())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

// ---- Sérialisation à ordre fixé -----------------------------------------------------------

struct Collection<'a> {
    epsg: u32,
    features: &'a [Feature],
}

impl Serialize for Collection<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut c = s.serialize_struct("FeatureCollection", 3)?;
        c.serialize_field("type", "FeatureCollection")?;
        c.serialize_field("crs", &Crs(self.epsg))?;
        c.serialize_field("features", self.features)?;
        c.end()
    }
}

struct Crs(u32);

impl Serialize for Crs {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut c = s.serialize_struct("Crs", 2)?;
        c.serialize_field("type", "name")?;
        c.serialize_field("properties", &CrsName(self.0))?;
        c.end()
    }
}

struct CrsName(u32);

impl Serialize for CrsName {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut c = s.serialize_struct("CrsName", 1)?;
        c.serialize_field("name", &format!("urn:ogc:def:crs:EPSG::{}", self.0))?;
        c.end()
    }
}

impl Serialize for Feature {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut c = s.serialize_struct("Feature", 3)?;
        c.serialize_field("type", "Feature")?;
        c.serialize_field("geometry", &self.geometry)?;
        c.serialize_field("properties", &Properties(&self.properties))?;
        c.end()
    }
}

impl Serialize for Geometry {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut c = s.serialize_struct("Geometry", 2)?;
        match self {
            Geometry::Point(x, y) => {
                c.serialize_field("type", "Point")?;
                c.serialize_field("coordinates", &[x, y])?;
            }
            Geometry::LineString(points) => {
                c.serialize_field("type", "LineString")?;
                c.serialize_field("coordinates", &Positions(points))?;
            }
        }
        c.end()
    }
}

struct Positions<'a>(&'a [(f64, f64)]);

impl Serialize for Positions<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut seq = s.serialize_seq(Some(self.0.len()))?;
        for (x, y) in self.0 {
            seq.serialize_element(&[x, y])?;
        }
        seq.end()
    }
}

struct Properties<'a>(&'a [(&'static str, Property)]);

impl Serialize for Properties<'_> {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (name, value) in self.0 {
            match value {
                Property::Integer(v) => map.serialize_entry(name, v)?,
                Property::Number(v) => map.serialize_entry(name, v)?,
            }
        }
        map.end()
    }
}

#[cfg(test)]
/// Les trois points de la fixture `points-2154.geojson`, autour du sommet de la colline.
pub(crate) fn fixture_points() -> Vec<Feature> {
    [
        (1, 652250.0, 6850250.0, 150.0, 100.0),
        (2, 652100.0, 6850400.0, 87.5, 12.25),
        (3, 652400.0, 6850100.0, 62.0, 4.5),
    ]
    .into_iter()
    .map(|(rank, x, y, elevation, prominence)| Feature {
        geometry: Geometry::Point(x, y),
        properties: vec![
            ("rank", Property::Integer(rank)),
            ("elevation", Property::Number(elevation)),
            ("prominence", Property::Number(prominence)),
        ],
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cassini-geojson-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn two_writes_are_identical_and_equal_to_the_fixture() {
        let dir = scratch("fixture");
        let versioned = fixtures_dir().join("points-2154.geojson");
        if std::env::var("CASSINI_REGENERER_FIXTURES").as_deref() == Ok("1") {
            write(&versioned, 2154, &fixture_points()).unwrap();
        }
        let (a, b) = (dir.join("a.geojson"), dir.join("b.geojson"));
        write(&a, 2154, &fixture_points()).unwrap();
        write(&b, 2154, &fixture_points()).unwrap();
        let bytes = std::fs::read(&a).unwrap();
        assert_eq!(bytes, std::fs::read(&b).unwrap());
        assert_eq!(
            bytes,
            std::fs::read(&versioned).unwrap(),
            "points-2154.geojson régénéré diffère du fichier versionné"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_written_form_is_the_compact_2008_form() {
        let one = &fixture_points()[..1];
        let text = String::from_utf8(to_bytes(2154, one).unwrap()).unwrap();
        assert_eq!(
            text,
            "{\"type\":\"FeatureCollection\",\"crs\":{\"type\":\"name\",\"properties\":{\"name\":\"urn:ogc:def:crs:EPSG::2154\"}},\"features\":[{\"type\":\"Feature\",\"geometry\":{\"type\":\"Point\",\"coordinates\":[652250.0,6850250.0]},\"properties\":{\"rank\":1,\"elevation\":150.0,\"prominence\":100.0}}]}\n"
        );
    }

    #[test]
    fn features_and_properties_keep_the_given_order() {
        let mut features = fixture_points();
        features.reverse();
        features[0].properties.reverse();
        let text = String::from_utf8(to_bytes(2154, &features).unwrap()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        let ranks: Vec<_> = value["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["properties"]["rank"].as_i64().unwrap())
            .collect();
        assert_eq!(ranks, vec![3, 2, 1]);
        // Ordre des noms : celui donné, pas l'ordre alphabétique.
        assert!(
            text.contains("\"properties\":{\"prominence\":4.5,\"elevation\":62.0,\"rank\":3}"),
            "{text}"
        );
        assert!(
            text.contains("\"properties\":{\"rank\":2,\"elevation\":87.5,\"prominence\":12.25}"),
            "{text}"
        );
    }

    #[test]
    fn the_crs_member_declares_the_epsg_code() {
        let text = String::from_utf8(to_bytes(32631, &fixture_points()).unwrap()).unwrap();
        let value: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            value["crs"],
            serde_json::json!({"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::32631"}})
        );
        assert!(!text.contains("bbox"));
        assert_eq!(
            to_bytes(0, &fixture_points()).unwrap_err().code,
            "geojson.unwritable"
        );
    }

    #[test]
    fn a_line_string_is_written() {
        let line = Feature {
            geometry: Geometry::LineString(vec![(652000.0, 6850000.0), (652010.5, 6850020.0)]),
            properties: vec![("cost", Property::Number(12.0))],
        };
        let text = String::from_utf8(to_bytes(2154, &[line]).unwrap()).unwrap();
        assert!(
            text.contains(
                "\"geometry\":{\"type\":\"LineString\",\"coordinates\":[[652000.0,6850000.0],[652010.5,6850020.0]]}"
            ),
            "{text}"
        );
        assert!(text.contains("\"properties\":{\"cost\":12.0}"), "{text}");
        // Une collection vide reste une collection.
        let empty = String::from_utf8(to_bytes(2154, &[]).unwrap()).unwrap();
        assert!(empty.ends_with(",\"features\":[]}\n"), "{empty}");
    }

    #[test]
    fn a_non_finite_number_writes_nothing() {
        let dir = scratch("non-fini");
        let mut cases = Vec::new();
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut point = fixture_points();
            point[1].geometry = Geometry::Point(bad, 6850000.0);
            cases.push(point);
            let mut line = fixture_points();
            line[2].geometry = Geometry::LineString(vec![(1.0, 2.0), (3.0, bad)]);
            cases.push(line);
            let mut property = fixture_points();
            property[0].properties[1].1 = Property::Number(bad);
            cases.push(property);
        }
        for (i, features) in cases.iter().enumerate() {
            let path = dir.join(format!("{i}.geojson"));
            let e = write(&path, 2154, features).unwrap_err();
            assert_eq!(e.code, "geojson.unwritable");
            assert!(e.message.contains("rien n'est écrit"), "{}", e.message);
            assert!(!path.exists(), "{i} écrit");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_folder_names_the_file_alone() {
        let dir = scratch("absent");
        let path = dir.join("absent").join("1-terrain.peaks.geojson");
        let e = write(&path, 2154, &fixture_points()).unwrap_err();
        assert_eq!(e.code, "geojson.unwritable");
        assert!(
            e.message.contains("« 1-terrain.peaks.geojson »"),
            "{}",
            e.message
        );
        assert!(
            !e.message.contains(&dir.display().to_string()),
            "{}",
            e.message
        );
        assert!(
            !e.message.contains('/') && !e.message.contains('\\'),
            "{}",
            e.message
        );
        assert!(!path.exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
