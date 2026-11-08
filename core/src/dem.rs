//! Modèle numérique d'élévation (MNT) : l'unique porte d'entrée raster du noyau (AD-20).
//!
//! Aucun autre module du noyau n'ouvre un raster. Ce module lit et écrit des GeoTIFF à une
//! bande avec la crate `tiff` (pur Rust, MIT), et décode lui-même le géoréférencement :
//! emprise, taille de cellule, code EPSG et valeur sans-donnée. Aucune GDAL, GEOS ni PROJ
//! (AD-1), et aucune reprojection (AD-19) : un MNT en degrés est refusé, l'utilisateur le
//! reprojette dans QGIS.
//!
//! Ce module n'est pas une commande du registre : les commandes de terrain s'appuient dessus.

use std::fmt;
use std::fs::File;
use std::io::{BufReader, Cursor};
use std::path::Path;

use tiff::decoder::{Decoder, DecodingResult, Limits};
use tiff::encoder::colortype::Gray32Float;
use tiff::encoder::compression::DeflateLevel;
use tiff::encoder::{Compression, TiffEncoder};
use tiff::tags::Tag;
use tiff::TiffError;

// Clés GeoTIFF lues dans `GeoKeyDirectoryTag`.
const GT_MODEL_TYPE: u16 = 1024;
const GT_RASTER_TYPE: u16 = 1025;
const GEOGRAPHIC_TYPE: u16 = 2048;
const PROJECTED_CS_TYPE: u16 = 3072;
const PROJ_LINEAR_UNITS: u16 = 3076;

const MODEL_TYPE_PROJECTED: u16 = 1;
const MODEL_TYPE_GEOGRAPHIC: u16 = 2;
const RASTER_PIXEL_IS_AREA: u16 = 1;
const RASTER_PIXEL_IS_POINT: u16 = 2;
const USER_DEFINED: u16 = 32767;
const LINEAR_METER: u16 = 9001;

/// Plafond de la grille décodée, 1 Gio : un MNT de 1 m sur une commune passe (10 km × 10 km
/// en Float32 font 400 Mo), la limite de `tiff` (256 Mo) ne le laissait pas. Il porte sur la
/// grille seule : la mémoire de pointe est plus haute (grille décodée puis sa copie en f32).
pub const MAX_DECODED_BYTES: usize = 1 << 30;

/// Valeur sans-donnée de tout raster écrit par le noyau (pente, exposition, ombrage, champ de
/// vision, coût cumulé, reclassement, superposition).
pub const NODATA: f32 = -9999.0;

/// Message commun des refus pour taille, avant décodage ou par la limite de `tiff`.
const TOO_LARGE: &str = "dépasse le plafond de 1 Gio pour la grille ; découpez-le dans QGIS (Raster > Extraction > Découper un raster selon une emprise).";

const REPROJECT_HINT: &str =
    "reprojetez-le dans QGIS (Raster > Projections > Reprojeter) vers un système projeté en mètres, Lambert-93 par exemple";

/// Erreur typée du module, à code stable `dem.cause` (AD-21) et message en français.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemError {
    pub code: &'static str,
    pub message: String,
}

impl DemError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    fn unreadable(message: impl Into<String>) -> Self {
        Self::new("dem.unreadable", message)
    }

    fn unsupported(message: impl Into<String>) -> Self {
        Self::new("dem.unsupported", message)
    }

    fn not_projected(detail: &str) -> Self {
        Self::new(
            "dem.not_projected",
            format!(
                "Le MNT n'est pas dans un système projeté en mètres ({detail}) : {REPROJECT_HINT}."
            ),
        )
    }

    fn no_epsg(detail: &str) -> Self {
        Self::new(
            "dem.no_epsg",
            format!("Le MNT n'a pas de code EPSG ({detail}) : {REPROJECT_HINT}."),
        )
    }

    fn unwritable(message: impl Into<String>) -> Self {
        Self::new("dem.unwritable", message)
    }
}

impl fmt::Display for DemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

impl std::error::Error for DemError {}

/// Emprise d'un MNT, dans les unités de son système (mètres).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Extent {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

/// Grille d'altitudes nord en haut, sans rotation, dans un système projeté métrique.
#[derive(Debug, Clone, PartialEq)]
pub struct Dem {
    pub width: u32,
    pub height: u32,
    /// Coin haut-gauche de la cellule (0, 0).
    pub origin_x: f64,
    pub origin_y: f64,
    /// Tailles de cellule, positives : x croît vers l'est, y décroît vers le sud.
    pub cell_size_x: f64,
    pub cell_size_y: f64,
    pub epsg: u32,
    pub nodata: Option<f64>,
    /// Altitudes ligne par ligne, du nord au sud, d'ouest en est. Tout type d'échantillon est
    /// converti en f32 : les F64, U32 et I32 perdent de la précision au-delà de 2^24 ou sur
    /// les décimales fines, sans effet sur une altitude au centimètre.
    pub values: Vec<f32>,
}

/// En-tête d'un MNT, lu sans décoder la grille : le décodeur, prêt à lire l'image, la taille,
/// le géoréférencement, les clés GeoTIFF et le code EPSG contrôlé.
struct Header {
    decoder: Decoder<BufReader<File>>,
    width: u32,
    height: u32,
    geo: (f64, f64, f64, f64),
    keys: GeoKeys,
    epsg: u32,
}

/// Code EPSG d'un MNT, lu dans son en-tête seul (ticket 11.22) : mêmes contrôles que
/// [`Dem::open`] jusqu'au système, sans décoder la grille, pour qu'un gros MNT ne ralentisse
/// pas la proposition d'un plan.
pub fn epsg_of(path: &Path) -> Result<u32, DemError> {
    Ok(read_header(path)?.epsg)
}

/// Ouvre un GeoTIFF et contrôle son en-tête : bandes, format, taille, géoréférencement, système.
fn read_header(path: &Path) -> Result<Header, DemError> {
    let file = File::open(path).map_err(|e| {
        DemError::unreadable(format!(
            "Impossible d'ouvrir le MNT « {} » : {e}.",
            path.display()
        ))
    })?;
    let mut limits = Limits::default();
    limits.decoding_buffer_size = MAX_DECODED_BYTES;
    limits.intermediate_buffer_size = MAX_DECODED_BYTES;
    let mut decoder = Decoder::new(BufReader::new(file))
        .map_err(|e| {
            DemError::unreadable(format!(
                "« {} » n'est pas un TIFF lisible : {e}.",
                path.display()
            ))
        })?
        .with_limits(limits);

    let (width, height) = decoder.dimensions().map_err(map_read_error)?;
    let bits = decoder
        .find_tag_unsigned_vec::<u16>(Tag::BitsPerSample)
        .map_err(map_read_error)?
        .and_then(|b| b.into_iter().max())
        .unwrap_or(1);
    check_size(width, height, bits)?;
    let samples = decoder
        .find_tag_unsigned::<u16>(Tag::SamplesPerPixel)
        .map_err(map_read_error)?
        .unwrap_or(1);
    if samples != 1 {
        return Err(DemError::unsupported(format!(
            "Le MNT a {samples} bandes ; seuls les MNT à une bande sont lus."
        )));
    }
    let sample_format = decoder
        .find_tag_unsigned_vec::<u16>(Tag::SampleFormat)
        .map_err(map_read_error)?
        .unwrap_or_else(|| vec![1]);
    if sample_format.iter().any(|f| !matches!(f, 1..=3)) {
        return Err(DemError::unsupported(format!(
            "Format d'échantillon {sample_format:?} non lu : entiers ou flottants seulement."
        )));
    }

    let geo = read_georeferencing(&mut decoder)?;
    let keys = read_geokeys(&mut decoder)?;
    let epsg = check_crs(&keys)?;
    Ok(Header {
        decoder,
        width,
        height,
        geo,
        keys,
        epsg,
    })
}

impl Dem {
    /// Lit un GeoTIFF à une bande en système projeté métrique.
    pub fn open(path: &Path) -> Result<Dem, DemError> {
        let Header {
            mut decoder,
            width,
            height,
            geo,
            keys,
            epsg,
        } = read_header(path)?;
        let (origin_x, origin_y, cell_size_x, cell_size_y) = geo;
        let (origin_x, origin_y) = if keys.get(GT_RASTER_TYPE) == Some(RASTER_PIXEL_IS_POINT) {
            // Le point de rattachement est le centre de la cellule : on revient à son coin.
            (origin_x - cell_size_x / 2.0, origin_y + cell_size_y / 2.0)
        } else {
            (origin_x, origin_y)
        };

        let nodata = match decoder.find_tag(Tag::GdalNodata).map_err(map_read_error)? {
            None => None,
            Some(v) => {
                let text = v.into_string().map_err(map_read_error)?;
                let text = text.trim_matches(|c: char| c == '\0' || c.is_whitespace());
                Some(text.parse::<f64>().map_err(|_| {
                    DemError::unsupported(format!("Valeur sans-donnée « {text} » illisible."))
                })?)
            }
        };

        let values = to_f32(decoder.read_image().map_err(map_read_error)?)?;
        let expected = width as usize * height as usize;
        if values.len() != expected {
            return Err(DemError::unsupported(format!(
                "Le MNT rend {} valeurs pour {width} × {height} cellules.",
                values.len()
            )));
        }

        Ok(Dem {
            width,
            height,
            origin_x,
            origin_y,
            cell_size_x,
            cell_size_y,
            epsg,
            nodata,
            values,
        })
    }

    /// Altitude de la cellule, ou `None` hors de la grille, en sans-donnée ou si elle n'est pas un nombre.
    pub fn value(&self, col: u32, row: u32) -> Option<f32> {
        if col >= self.width || row >= self.height {
            return None;
        }
        let v = *self
            .values
            .get(row as usize * self.width as usize + col as usize)?;
        // Comparaison en f32 : c'est le type des valeurs gardées, une sans-donnée non
        // représentable exactement (-9999,99) y est arrondie comme les cellules.
        if v.is_nan() || self.nodata.is_some_and(|nd| v == nd as f32) {
            return None;
        }
        Some(v)
    }

    /// Emprise de la grille.
    pub fn extent(&self) -> Extent {
        Extent {
            min_x: self.origin_x,
            max_x: self.origin_x + self.cell_size_x * f64::from(self.width),
            max_y: self.origin_y,
            min_y: self.origin_y - self.cell_size_y * f64::from(self.height),
        }
    }

    /// Premier écart entre la grille de `other` et celle-ci, décrit en français, ou `None` si
    /// les deux grilles sont identiques au sens d'AD-20 : mêmes code EPSG et dimensions, taille
    /// de cellule et origine à moins d'un millionième de cellule (cellule de `self`). Ordre du
    /// contrôle : système, dimensions, taille de cellule, origine. Aucune grille n'est
    /// rééchantillonnée : la commande qui combine des rasters refuse l'écart (AD-19, AD-20).
    pub fn grid_mismatch(&self, other: &Dem) -> Option<String> {
        if self.epsg != other.epsg {
            return Some(format!(
                "système EPSG:{} au lieu d'EPSG:{}",
                other.epsg, self.epsg
            ));
        }
        if (self.width, self.height) != (other.width, other.height) {
            return Some(format!(
                "dimensions {} × {} cellules au lieu de {} × {}",
                other.width, other.height, self.width, self.height
            ));
        }
        let tolerance = |cell: f64| cell.abs() * 1e-6;
        if (other.cell_size_x - self.cell_size_x).abs() > tolerance(self.cell_size_x)
            || (other.cell_size_y - self.cell_size_y).abs() > tolerance(self.cell_size_y)
        {
            return Some(format!(
                "taille de cellule {} × {} m au lieu de {} × {} m",
                french(other.cell_size_x),
                french(other.cell_size_y),
                french(self.cell_size_x),
                french(self.cell_size_y)
            ));
        }
        for (axis, mine, theirs, cell) in [
            ("x", self.origin_x, other.origin_x, self.cell_size_x),
            ("y", self.origin_y, other.origin_y, self.cell_size_y),
        ] {
            let shift = theirs - mine;
            if shift.abs() > tolerance(cell) {
                let cells = shift / cell;
                let plural = if cells.abs() >= 2.0 { "s" } else { "" };
                return Some(format!(
                    "origine {axis} décalée de {} m ({} cellule{plural}) : {} au lieu de {}",
                    french(shift),
                    french(cells),
                    french(theirs),
                    french(mine)
                ));
            }
        }
        None
    }

    /// Écrit la grille en GeoTIFF Float32 compressé deflate. Déterministe : aucune date ni
    /// aucun logiciel n'est écrit, les octets ne dépendent que de la grille.
    pub fn write_geotiff(&self, path: &Path) -> Result<(), DemError> {
        let bytes = self.to_geotiff_bytes()?;
        std::fs::write(path, bytes).map_err(|e| {
            DemError::unwritable(format!(
                "Impossible d'écrire le MNT « {} » : {e}.",
                path.display()
            ))
        })
    }

    fn to_geotiff_bytes(&self) -> Result<Vec<u8>, DemError> {
        if self.values.len() != self.width as usize * self.height as usize {
            return Err(DemError::unwritable(
                "La grille n'a pas autant de valeurs que de cellules.",
            ));
        }
        let epsg = u16::try_from(self.epsg)
            .ok()
            .filter(|c| *c != 0 && *c != USER_DEFINED)
            .ok_or_else(|| {
                DemError::unwritable(format!(
                    "Code EPSG {} non inscriptible en GeoTIFF.",
                    self.epsg
                ))
            })?;
        let werr = |e: TiffError| DemError::unwritable(format!("Écriture du GeoTIFF : {e}."));

        let mut buffer = Cursor::new(Vec::new());
        {
            let mut tiff = TiffEncoder::new(&mut buffer)
                .map_err(werr)?
                .with_compression(Compression::Deflate(DeflateLevel::Balanced));
            let mut image = tiff
                .new_image::<Gray32Float>(self.width, self.height)
                .map_err(werr)?;
            let dir = image.encoder();
            dir.write_tag(
                Tag::ModelPixelScaleTag,
                &[self.cell_size_x, self.cell_size_y, 0.0][..],
            )
            .map_err(werr)?;
            dir.write_tag(
                Tag::ModelTiepointTag,
                &[0.0, 0.0, 0.0, self.origin_x, self.origin_y, 0.0][..],
            )
            .map_err(werr)?;
            let keys: [u16; 20] = [
                1,
                1,
                0,
                4,
                GT_MODEL_TYPE,
                0,
                1,
                MODEL_TYPE_PROJECTED,
                GT_RASTER_TYPE,
                0,
                1,
                RASTER_PIXEL_IS_AREA,
                PROJECTED_CS_TYPE,
                0,
                1,
                epsg,
                PROJ_LINEAR_UNITS,
                0,
                1,
                LINEAR_METER,
            ];
            dir.write_tag(Tag::GeoKeyDirectoryTag, &keys[..])
                .map_err(werr)?;
            if let Some(nd) = self.nodata {
                dir.write_tag(Tag::GdalNodata, format!("{nd}").as_str())
                    .map_err(werr)?;
            }
            image.write_data(&self.values).map_err(werr)?;
        }
        Ok(buffer.into_inner())
    }
}

/// Coin haut-gauche et tailles de cellule (positives), depuis le point de rattachement et
/// l'échelle, ou depuis une matrice de transformation sans rotation.
fn read_georeferencing<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> Result<(f64, f64, f64, f64), DemError> {
    let scale = find_f64_vec(decoder, Tag::ModelPixelScaleTag)?;
    let tiepoints = find_f64_vec(decoder, Tag::ModelTiepointTag)?;
    let transform = find_f64_vec(decoder, Tag::ModelTransformationTag)?;

    let (ox, oy, sx, sy) = match (scale, tiepoints, transform) {
        (Some(scale), Some(tp), _) => {
            if tp.len() != 6 {
                return Err(DemError::unsupported(
                    "Le MNT est calé par plusieurs points de rattachement ; seule une grille régulière est lue.",
                ));
            }
            if scale.len() < 2 {
                return Err(DemError::unreadable("Échelle de pixel du MNT incomplète."));
            }
            let (sx, sy) = (scale[0], scale[1]);
            (tp[3] - tp[0] * sx, tp[4] + tp[1] * sy, sx, sy)
        }
        (_, _, Some(m)) => {
            if m.len() != 16 {
                return Err(DemError::unreadable(
                    "Matrice de transformation du MNT incomplète.",
                ));
            }
            if m[1] != 0.0 || m[4] != 0.0 {
                return Err(DemError::unsupported(
                    "Le MNT est tourné ; seule une grille nord en haut est lue.",
                ));
            }
            (m[3], m[7], m[0], -m[5])
        }
        _ => {
            return Err(DemError::unreadable(
                "Le TIFF n'a pas de géoréférencement (ni point de rattachement ni transformation).",
            ))
        }
    };
    if !(sx.is_finite() && sy.is_finite() && sx > 0.0 && sy > 0.0) {
        return Err(DemError::unsupported(
            "Tailles de cellule nulles, négatives ou retournées : seule une grille nord en haut est lue.",
        ));
    }
    if !(ox.is_finite() && oy.is_finite()) {
        return Err(DemError::unreadable("Origine du MNT illisible."));
    }
    Ok((ox, oy, sx, sy))
}

fn find_f64_vec<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
    tag: Tag,
) -> Result<Option<Vec<f64>>, DemError> {
    match decoder.find_tag(tag).map_err(map_read_error)? {
        None => Ok(None),
        Some(v) => v.into_f64_vec().map(Some).map_err(map_read_error),
    }
}

/// Clés GeoTIFF dont la valeur est portée directement dans le répertoire.
struct GeoKeys(Vec<(u16, u16)>);

impl GeoKeys {
    fn get(&self, key: u16) -> Option<u16> {
        self.0.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
    }
}

fn read_geokeys<R: std::io::Read + std::io::Seek>(
    decoder: &mut Decoder<R>,
) -> Result<GeoKeys, DemError> {
    let Some(value) = decoder
        .find_tag(Tag::GeoKeyDirectoryTag)
        .map_err(map_read_error)?
    else {
        return Err(DemError::no_epsg("aucune clé GeoTIFF"));
    };
    let dir = value.into_u16_vec().map_err(map_read_error)?;
    if dir.len() < 4 {
        return Err(DemError::unreadable("Répertoire des clés GeoTIFF tronqué."));
    }
    let count = dir[3] as usize;
    let mut keys = Vec::with_capacity(count);
    for entry in dir[4..].chunks_exact(4).take(count) {
        // Lieu 0 : la valeur est dans l'entrée ; sinon elle est dans une autre balise
        // (doubles, texte), dont aucune clé lue ici n'a besoin.
        if entry[1] == 0 {
            keys.push((entry[0], entry[3]));
        }
    }
    Ok(GeoKeys(keys))
}

/// Exige un système projeté, doté d'un code EPSG, en mètres.
fn check_crs(keys: &GeoKeys) -> Result<u32, DemError> {
    let model = keys.get(GT_MODEL_TYPE);
    let projected = keys.get(PROJECTED_CS_TYPE);
    match model {
        Some(MODEL_TYPE_GEOGRAPHIC) => {
            let detail = match keys.get(GEOGRAPHIC_TYPE) {
                Some(code) if code != USER_DEFINED => {
                    format!("système géographique EPSG:{code}, en degrés")
                }
                _ => "système géographique, en degrés".to_string(),
            };
            return Err(DemError::not_projected(&detail));
        }
        Some(MODEL_TYPE_PROJECTED) => {}
        None if projected.is_some() => {}
        None if keys.get(GEOGRAPHIC_TYPE).is_some() => {
            return Err(DemError::not_projected("système géographique, en degrés"));
        }
        None => return Err(DemError::no_epsg("type de modèle absent")),
        Some(other) => {
            return Err(DemError::unsupported(format!(
                "Type de modèle GeoTIFF {other} non lu : seul un système projeté l'est."
            )))
        }
    }
    let epsg = match projected {
        None | Some(0) => return Err(DemError::no_epsg("système projeté sans code")),
        Some(USER_DEFINED) => return Err(DemError::no_epsg("système défini par l'utilisateur")),
        Some(code) => code,
    };
    match keys.get(PROJ_LINEAR_UNITS) {
        Some(LINEAR_METER) => {}
        Some(unit) => {
            return Err(DemError::not_projected(&format!(
                "unité linéaire {unit}, pas le mètre"
            )))
        }
        // GDAL omet souvent la clé d'unité : le mètre ne se vérifie alors que par la liste.
        None if is_known_metric(epsg) => {}
        None => {
            return Err(DemError::not_projected(&format!(
                "EPSG:{epsg} sans unité déclarée, le mètre ne peut pas être vérifié"
            )))
        }
    }
    // Clé au mètre ou non, seule la liste fermée passe (décision du propriétaire, 2026-11-05) :
    // les « mètres » de Web Mercator (EPSG:3857) sont dilatés d'environ 1,5 fois en France.
    if !is_known_metric(epsg) {
        return Err(DemError::not_projected(&format!(
            "EPSG:{epsg} n'est pas un système en vrais mètres connu de Cassini, ses distances seraient faussées"
        )));
    }
    Ok(u32::from(epsg))
}

/// Liste fermée des systèmes projetés acceptés, connus pour être en vrais mètres :
/// Lambert-93, coniques conformes 9 zones, Lambert zones (NTF), UTM WGS84 nord et sud,
/// UTM ETRS89. Un système métrique légitime manquant s'y ajoute par un ticket.
fn is_known_metric(epsg: u16) -> bool {
    matches!(
        epsg,
        2154 | 3942..=3950 | 27561..=27564 | 27571..=27574 | 32601..=32660 | 32701..=32760
            | 25828..=25838
    )
}

/// Refuse un MNT dont la grille décodée, ou sa copie en f32, dépasserait `MAX_DECODED_BYTES`.
fn check_size(width: u32, height: u32, bits_per_sample: u16) -> Result<(), DemError> {
    let bytes_per_cell = (u64::from(bits_per_sample).div_ceil(8)).max(4);
    let needed = u64::from(width) * u64::from(height) * bytes_per_cell;
    if needed > MAX_DECODED_BYTES as u64 {
        return Err(DemError::unsupported(format!(
            "MNT trop grand : {width} × {height} cellules, il {TOO_LARGE}"
        )));
    }
    Ok(())
}

fn to_f32(result: DecodingResult) -> Result<Vec<f32>, DemError> {
    Ok(match result {
        DecodingResult::U8(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::I8(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::U16(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::I16(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::U32(v) => v.into_iter().map(|x| x as f32).collect(),
        DecodingResult::I32(v) => v.into_iter().map(|x| x as f32).collect(),
        DecodingResult::F32(v) => v,
        DecodingResult::F64(v) => v.into_iter().map(|x| x as f32).collect(),
        DecodingResult::F16(v) => v.into_iter().map(f32::from).collect(),
        DecodingResult::U64(_) | DecodingResult::I64(_) => {
            return Err(DemError::unsupported(
                "Échantillons entiers sur 64 bits non lus.",
            ))
        }
    })
}

fn map_read_error(e: TiffError) -> DemError {
    match e {
        TiffError::UnsupportedError(e) => {
            DemError::unsupported(format!("Forme de TIFF non lue : {e}."))
        }
        TiffError::LimitsExceeded => {
            DemError::unsupported(format!("MNT trop grand : il {TOO_LARGE}"))
        }
        e => DemError::unreadable(format!("TIFF illisible : {e}.")),
    }
}

/// Nombre écrit à la française pour un message : arrondi au millionième, virgule décimale.
fn french(v: f64) -> String {
    let rounded = (v * 1e6).round() / 1e6;
    // `-0` s'écrit `0`.
    let rounded = if rounded == 0.0 { 0.0 } else { rounded };
    format!("{rounded}").replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tiff::encoder::colortype::{
        ColorType, Gray16, Gray32Float, Gray64Float, GrayI16, GrayI32, RGB8,
    };
    use tiff::encoder::{DirectoryEncoder, TiffKindStandard, TiffValue};
    use tiff::TiffResult;

    type Dir<'a, 'b> = DirectoryEncoder<'a, &'b mut Cursor<Vec<u8>>, TiffKindStandard>;

    /// Dossier temporaire propre à un test.
    /// Effacé à la fin du test, y compris s'il échoue.
    fn scratch(name: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("cassini-dem-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    struct Scratch(PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// Fabrique un TIFF quelconque, avec les balises que pose `tags`.
    fn raw_tiff<C: ColorType>(
        path: &Path,
        width: u32,
        height: u32,
        data: &[C::Inner],
        tags: impl FnOnce(&mut Dir<'_, '_>) -> TiffResult<()>,
    ) where
        [C::Inner]: TiffValue,
    {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut tiff = TiffEncoder::new(&mut buffer).unwrap();
            let mut image = tiff.new_image::<C>(width, height).unwrap();
            tags(image.encoder()).unwrap();
            image.write_data(data).unwrap();
        }
        std::fs::write(path, buffer.into_inner()).unwrap();
    }

    /// Calage nord en haut, cellules de 10 m, coin haut-gauche (652000, 6850300).
    fn georef(dir: &mut Dir<'_, '_>) -> TiffResult<()> {
        dir.write_tag(Tag::ModelPixelScaleTag, &[10.0, 10.0, 0.0][..])?;
        dir.write_tag(
            Tag::ModelTiepointTag,
            &[0.0, 0.0, 0.0, 652000.0, 6850300.0, 0.0][..],
        )
    }

    fn geokeys(dir: &mut Dir<'_, '_>, keys: &[(u16, u16)]) -> TiffResult<()> {
        let mut v = vec![1, 1, 0, keys.len() as u16];
        for (k, val) in keys {
            v.extend([*k, 0, 1, *val]);
        }
        dir.write_tag(Tag::GeoKeyDirectoryTag, &v[..])
    }

    const L93: &[(u16, u16)] = &[(GT_MODEL_TYPE, 1), (PROJECTED_CS_TYPE, 2154)];

    fn projected(dir: &mut Dir<'_, '_>) -> TiffResult<()> {
        georef(dir)?;
        geokeys(dir, L93)
    }

    fn open_err(path: &Path) -> DemError {
        Dem::open(path).expect_err("le MNT devait être refusé")
    }

    // --- Recette versionnée -------------------------------------------------------------

    fn fixtures_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../fixtures")
    }

    /// Plan de pente 10 % descendant vers l'est : 40 × 30 cellules de 10 m, 1 m de moins par colonne.
    fn plan_incline() -> Dem {
        let (width, height) = (40u32, 30u32);
        let values = (0..height)
            .flat_map(|_| (0..width).map(|col| 99.5 - col as f32))
            .collect();
        Dem {
            width,
            height,
            origin_x: 652000.0,
            origin_y: 6850300.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    /// Colline lisse de 150 m au sommet central sur un fond à 50 m : 50 × 50 cellules de 10 m.
    /// Seules des opérations IEEE exactement arrondies (+, ×, ÷) : mêmes valeurs partout.
    fn colline() -> Dem {
        let (width, height) = (50u32, 50u32);
        let mut values = Vec::with_capacity((width * height) as usize);
        for row in 0..height {
            for col in 0..width {
                let dx = (f64::from(col) + 0.5) * 10.0 - 250.0;
                let dy = (f64::from(row) + 0.5) * 10.0 - 250.0;
                let d2 = dx * dx + dy * dy;
                values.push((50.0 + 100.0 / (1.0 + d2 / 22500.0)) as f32);
            }
        }
        Dem {
            width,
            height,
            origin_x: 652000.0,
            origin_y: 6850500.0,
            cell_size_x: 10.0,
            cell_size_y: 10.0,
            epsg: 2154,
            nodata: Some(-9999.0),
            values,
        }
    }

    /// Régénère chaque fixture et la compare, octet pour octet, au fichier versionné.
    /// `CASSINI_REGENERER_FIXTURES=1` réécrit les fichiers versionnés.
    #[test]
    fn fixtures_regenerate_identically() {
        let dir = scratch("fixtures");
        for (name, dem) in [
            ("mnt-plan-incline-2154.tif", plan_incline()),
            ("mnt-colline-2154.tif", colline()),
        ] {
            let versioned = fixtures_dir().join(name);
            if std::env::var("CASSINI_REGENERER_FIXTURES").as_deref() == Ok("1") {
                dem.write_geotiff(&versioned).unwrap();
            }
            let fresh = dir.join(name);
            dem.write_geotiff(&fresh).unwrap();
            assert_eq!(
                std::fs::read(&fresh).unwrap(),
                std::fs::read(&versioned).unwrap(),
                "{name} régénéré diffère du fichier versionné"
            );
            assert_eq!(Dem::open(&versioned).unwrap(), dem, "{name} relu");
        }
    }

    #[test]
    fn the_tilted_plane_fixture_gives_the_known_altitudes() {
        let dem = Dem::open(&fixtures_dir().join("mnt-plan-incline-2154.tif")).unwrap();
        assert_eq!((dem.width, dem.height, dem.epsg), (40, 30, 2154));
        assert_eq!((dem.cell_size_x, dem.cell_size_y), (10.0, 10.0));
        assert_eq!(dem.nodata, Some(-9999.0));
        assert_eq!(
            dem.extent(),
            Extent {
                min_x: 652000.0,
                min_y: 6850000.0,
                max_x: 652400.0,
                max_y: 6850300.0
            }
        );
        for row in 0..30 {
            for col in 0..40 {
                assert_eq!(dem.value(col, row), Some(99.5 - col as f32));
            }
        }
        assert_eq!(dem.value(40, 0), None);
        assert_eq!(dem.value(0, 30), None);
    }

    #[test]
    fn the_hill_fixture_peaks_in_the_centre() {
        let dem = Dem::open(&fixtures_dir().join("mnt-colline-2154.tif")).unwrap();
        let max = dem.values.iter().cloned().fold(f32::MIN, f32::max);
        assert!(max > 140.0 && max <= 150.0, "sommet {max}");
        assert_eq!(dem.value(24, 24), Some(max));
        assert!(dem.value(0, 0).unwrap() < 70.0);
    }

    // --- Lecture des types d'échantillon ------------------------------------------------

    fn assert_reads_1234(path: &Path) {
        let dem = Dem::open(path).unwrap();
        assert_eq!((dem.width, dem.height, dem.epsg), (2, 2, 2154));
        assert_eq!(dem.values, vec![1.0, 2.0, 3.0, 4.0]);
        assert_eq!((dem.origin_x, dem.origin_y), (652000.0, 6850300.0));
        assert_eq!(dem.extent().min_y, 6850280.0);
    }

    #[test]
    fn every_supported_sample_type_is_read_as_f32() {
        let dir = scratch("types");
        let p = dir.join("i16.tif");
        raw_tiff::<GrayI16>(&p, 2, 2, &[1, 2, 3, 4], projected);
        assert_reads_1234(&p);
        let p = dir.join("u16.tif");
        raw_tiff::<Gray16>(&p, 2, 2, &[1, 2, 3, 4], projected);
        assert_reads_1234(&p);
        let p = dir.join("i32.tif");
        raw_tiff::<GrayI32>(&p, 2, 2, &[1, 2, 3, 4], projected);
        assert_reads_1234(&p);
        let p = dir.join("f32.tif");
        raw_tiff::<Gray32Float>(&p, 2, 2, &[1.0, 2.0, 3.0, 4.0], projected);
        assert_reads_1234(&p);
        let p = dir.join("f64.tif");
        raw_tiff::<Gray64Float>(&p, 2, 2, &[1.0, 2.0, 3.0, 4.0], projected);
        assert_reads_1234(&p);
    }

    #[test]
    fn nodata_is_read_and_masked() {
        let dir = scratch("nodata");
        let p = dir.join("nd.tif");
        raw_tiff::<GrayI16>(&p, 2, 2, &[-32768, 12, 146, 50], |d| {
            projected(d)?;
            d.write_tag(Tag::GdalNodata, "-32768")
        });
        let dem = Dem::open(&p).unwrap();
        assert_eq!(dem.nodata, Some(-32768.0));
        assert_eq!(dem.value(0, 0), None);
        assert_eq!(dem.value(1, 0), Some(12.0));
    }

    #[test]
    fn pixel_is_point_moves_the_origin_to_the_cell_corner() {
        let dir = scratch("point");
        let p = dir.join("point.tif");
        raw_tiff::<Gray32Float>(&p, 2, 2, &[1.0; 4], |d| {
            georef(d)?;
            geokeys(
                d,
                &[
                    (GT_MODEL_TYPE, 1),
                    (GT_RASTER_TYPE, RASTER_PIXEL_IS_POINT),
                    (PROJECTED_CS_TYPE, 2154),
                ],
            )
        });
        let dem = Dem::open(&p).unwrap();
        assert_eq!((dem.origin_x, dem.origin_y), (651995.0, 6850305.0));
    }

    #[test]
    fn a_north_up_transformation_matrix_is_read() {
        let dir = scratch("matrix");
        let p = dir.join("m.tif");
        raw_tiff::<Gray32Float>(&p, 2, 2, &[1.0, 2.0, 3.0, 4.0], |d| {
            #[rustfmt::skip]
            let m = [10.0, 0.0, 0.0, 652000.0, 0.0, -10.0, 0.0, 6850300.0,
                     0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
            d.write_tag(Tag::ModelTransformationTag, &m[..])?;
            geokeys(d, L93)
        });
        assert_reads_1234(&p);
    }

    // --- Refus -----------------------------------------------------------------------------

    #[test]
    fn a_dem_in_degrees_is_refused_with_the_qgis_hint() {
        let dir = scratch("degres");
        let p = dir.join("4326.tif");
        raw_tiff::<GrayI16>(&p, 2, 2, &[1, 2, 3, 4], |d| {
            georef(d)?;
            geokeys(d, &[(GT_MODEL_TYPE, 2), (GEOGRAPHIC_TYPE, 4326)])
        });
        let e = open_err(&p);
        assert_eq!(e.code, "dem.not_projected");
        assert!(e.message.contains("EPSG:4326"), "{}", e.message);
        assert!(
            e.message.contains("reprojetez-le dans QGIS"),
            "{}",
            e.message
        );
        assert!(e.message.contains("Raster > Projections > Reprojeter"));
    }

    #[test]
    fn a_dem_without_epsg_is_refused() {
        let dir = scratch("sans-epsg");
        let p = dir.join("user.tif");
        raw_tiff::<GrayI16>(&p, 2, 2, &[1, 2, 3, 4], |d| {
            georef(d)?;
            geokeys(d, &[(GT_MODEL_TYPE, 1), (PROJECTED_CS_TYPE, USER_DEFINED)])
        });
        assert_eq!(open_err(&p).code, "dem.no_epsg");

        let p = dir.join("sans-cle.tif");
        raw_tiff::<GrayI16>(&p, 2, 2, &[1, 2, 3, 4], georef);
        assert_eq!(open_err(&p).code, "dem.no_epsg");

        let p = dir.join("projete-sans-code.tif");
        raw_tiff::<GrayI16>(&p, 2, 2, &[1, 2, 3, 4], |d| {
            georef(d)?;
            geokeys(d, &[(GT_MODEL_TYPE, 1)])
        });
        assert_eq!(open_err(&p).code, "dem.no_epsg");
    }

    #[test]
    fn a_non_metric_unit_is_refused() {
        let dir = scratch("pieds");
        let p = dir.join("pieds.tif");
        raw_tiff::<GrayI16>(&p, 2, 2, &[1, 2, 3, 4], |d| {
            georef(d)?;
            geokeys(
                d,
                &[
                    (GT_MODEL_TYPE, 1),
                    (PROJECTED_CS_TYPE, 2263),
                    (PROJ_LINEAR_UNITS, 9002),
                ],
            )
        });
        let e = open_err(&p);
        assert_eq!(e.code, "dem.not_projected");
        assert!(e.message.contains("QGIS"));
    }

    #[test]
    fn unsupported_shapes_are_refused() {
        let dir = scratch("formes");
        let p = dir.join("rvb.tif");
        raw_tiff::<RGB8>(&p, 2, 2, &[0; 12], projected);
        assert_eq!(open_err(&p).code, "dem.unsupported");

        let p = dir.join("rotation.tif");
        raw_tiff::<Gray32Float>(&p, 2, 2, &[1.0; 4], |d| {
            #[rustfmt::skip]
            let m = [8.66, 5.0, 0.0, 652000.0, 5.0, -8.66, 0.0, 6850300.0,
                     0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
            d.write_tag(Tag::ModelTransformationTag, &m[..])?;
            geokeys(d, L93)
        });
        assert_eq!(open_err(&p).code, "dem.unsupported");

        let p = dir.join("echantillon.tif");
        raw_tiff::<Gray16>(&p, 2, 2, &[1, 2, 3, 4], projected);
        // L'encodeur pose lui-même SampleFormat = 1 ; on le change en 5 (entier complexe,
        // qu'aucun MNT ne porte) dans l'entrée du répertoire : balise 339, type SHORT, 1 valeur.
        let mut bytes = std::fs::read(&p).unwrap();
        let entry = [0x53, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00];
        let at = bytes
            .windows(entry.len())
            .position(|w| w == entry)
            .expect("entrée SampleFormat");
        bytes[at + 8] = 5;
        std::fs::write(&p, bytes).unwrap();
        assert_eq!(open_err(&p).code, "dem.unsupported");
    }

    #[test]
    fn the_header_alone_gives_the_same_system_and_the_same_refusals() {
        // Ticket 11.22 : `epsg_of` fait les contrôles de `Dem::open` jusqu'au système.
        for name in ["mnt-plan-incline-2154.tif", "mnt-colline-2154.tif"] {
            let path = fixtures_dir().join(name);
            assert_eq!(epsg_of(&path), Ok(Dem::open(&path).unwrap().epsg), "{name}");
        }
        let dir = scratch("en-tete");
        let degrees = dir.join("4326.tif");
        raw_tiff::<GrayI16>(&degrees, 2, 2, &[1, 2, 3, 4], |d| {
            georef(d)?;
            geokeys(d, &[(GT_MODEL_TYPE, 2), (GEOGRAPHIC_TYPE, 4326)])
        });
        let unknown = dir.join("user.tif");
        raw_tiff::<GrayI16>(&unknown, 2, 2, &[1, 2, 3, 4], |d| {
            georef(d)?;
            geokeys(d, &[(GT_MODEL_TYPE, 1), (PROJECTED_CS_TYPE, USER_DEFINED)])
        });
        let text = dir.join("texte.tif");
        std::fs::write(&text, "ceci n'est pas un TIFF").unwrap();
        for path in [degrees, unknown, text, dir.join("absent.tif")] {
            assert_eq!(
                epsg_of(&path).unwrap_err(),
                open_err(&path),
                "{}",
                path.display()
            );
        }
    }

    #[test]
    fn the_system_is_read_without_decoding_the_image() {
        // En-tête valide, image hors du fichier : `Dem::open` échoue au décodage, `epsg_of` non.
        let dir = scratch("sans-image");
        let p = dir.join("tronque.tif");
        raw_tiff::<Gray32Float>(&p, 2, 2, &[1.0; 4], projected);
        let mut bytes = std::fs::read(&p).unwrap();
        assert_eq!(&bytes[..2], b"II", "TIFF petit-boutiste attendu");
        let ifd = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
        let count = u16::from_le_bytes(bytes[ifd..ifd + 2].try_into().unwrap()) as usize;
        let entry = (0..count)
            .map(|i| ifd + 2 + 12 * i)
            .find(|at| u16::from_le_bytes(bytes[*at..*at + 2].try_into().unwrap()) == 273)
            .expect("entrée StripOffsets");
        // Une seule bande : la valeur est l'adresse elle-même, envoyée loin après la fin.
        bytes[entry + 8..entry + 12].copy_from_slice(&0x00F0_0000u32.to_le_bytes());
        std::fs::write(&p, bytes).unwrap();
        assert!(Dem::open(&p).is_err());
        assert_eq!(epsg_of(&p), Ok(2154));
    }

    #[test]
    fn unreadable_files_are_refused() {
        let dir = scratch("illisible");
        assert_eq!(open_err(&dir.join("absent.tif")).code, "dem.unreadable");

        let p = dir.join("texte.tif");
        std::fs::write(&p, "ceci n'est pas un TIFF").unwrap();
        assert_eq!(open_err(&p).code, "dem.unreadable");

        let p = dir.join("sans-calage.tif");
        raw_tiff::<Gray32Float>(&p, 2, 2, &[1.0; 4], |_| Ok(()));
        assert_eq!(open_err(&p).code, "dem.unreadable");
    }

    // --- Écriture -------------------------------------------------------------------------

    #[test]
    fn writing_twice_gives_identical_bytes_read_back_unchanged() {
        let dir = scratch("ecriture");
        let dem = colline();
        let (a, b) = (dir.join("a.tif"), dir.join("b.tif"));
        dem.write_geotiff(&a).unwrap();
        dem.write_geotiff(&b).unwrap();
        assert_eq!(std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
        assert_eq!(Dem::open(&a).unwrap(), dem);

        let without_nodata = Dem {
            nodata: None,
            ..plan_incline()
        };
        without_nodata.write_geotiff(&a).unwrap();
        assert_eq!(Dem::open(&a).unwrap(), without_nodata);
    }

    #[test]
    fn an_inconsistent_grid_is_not_written() {
        let dir = scratch("incoherent");
        let mut dem = plan_incline();
        dem.values.pop();
        let e = dem.write_geotiff(&dir.join("x.tif")).unwrap_err();
        assert_eq!(e.code, "dem.unwritable");
        let dem = Dem {
            epsg: 32767,
            ..plan_incline()
        };
        assert_eq!(
            dem.write_geotiff(&dir.join("y.tif")).unwrap_err().code,
            "dem.unwritable"
        );
    }

    // --- Branches complémentaires --------------------------------------------------------

    fn with_keys(dir: &Path, name: &str, keys: &'static [(u16, u16)]) -> PathBuf {
        let p = dir.join(name);
        raw_tiff::<GrayI16>(&p, 2, 2, &[1, 2, 3, 4], |d| {
            georef(d)?;
            geokeys(d, keys)
        });
        p
    }

    #[test]
    fn without_unit_key_only_known_metric_codes_are_accepted() {
        let dir = scratch("unite-absente");
        let e = open_err(&with_keys(
            &dir,
            "2263.tif",
            &[(GT_MODEL_TYPE, 1), (PROJECTED_CS_TYPE, 2263)],
        ));
        assert_eq!(e.code, "dem.not_projected");
        assert!(
            e.message.contains("ne peut pas être vérifié"),
            "{}",
            e.message
        );
        assert!(
            e.message.contains("reprojetez-le dans QGIS"),
            "{}",
            e.message
        );

        let p = with_keys(
            &dir,
            "2154.tif",
            &[(GT_MODEL_TYPE, 1), (PROJECTED_CS_TYPE, 2154)],
        );
        assert_eq!(Dem::open(&p).unwrap().epsg, 2154);
        let p = with_keys(
            &dir,
            "32631.tif",
            &[(GT_MODEL_TYPE, 1), (PROJECTED_CS_TYPE, 32631)],
        );
        assert_eq!(Dem::open(&p).unwrap().epsg, 32631);
    }

    #[test]
    fn pseudo_metre_systems_are_refused_even_with_a_metre_unit() {
        let dir = scratch("pseudo-metres");
        for (name, keys) in [
            (
                "3857.tif",
                &[
                    (GT_MODEL_TYPE, 1),
                    (PROJECTED_CS_TYPE, 3857),
                    (PROJ_LINEAR_UNITS, LINEAR_METER),
                ],
            ),
            (
                "3395.tif",
                &[
                    (GT_MODEL_TYPE, 1),
                    (PROJECTED_CS_TYPE, 3395),
                    (PROJ_LINEAR_UNITS, LINEAR_METER),
                ],
            ),
        ] {
            let e = open_err(&with_keys(&dir, name, keys));
            assert_eq!(e.code, "dem.not_projected", "{name}");
            let code = name.trim_end_matches(".tif");
            assert!(e.message.contains(&format!("EPSG:{code}")), "{}", e.message);
            assert!(
                e.message.contains("reprojetez-le dans QGIS"),
                "{}",
                e.message
            );
        }
        // Les systèmes de la liste restent acceptés avec la clé d'unité au mètre.
        for code in [2154, 3944, 27572, 32631, 32730, 25831] {
            let p = dir.join(format!("{code}.tif"));
            raw_tiff::<GrayI16>(&p, 2, 2, &[1, 2, 3, 4], |d| {
                georef(d)?;
                geokeys(
                    d,
                    &[
                        (GT_MODEL_TYPE, 1),
                        (PROJECTED_CS_TYPE, code),
                        (PROJ_LINEAR_UNITS, LINEAR_METER),
                    ],
                )
            });
            assert_eq!(Dem::open(&p).unwrap().epsg, u32::from(code));
        }
    }

    #[test]
    fn a_missing_model_type_follows_the_other_keys() {
        let dir = scratch("sans-modele");
        let p = with_keys(&dir, "projete.tif", &[(PROJECTED_CS_TYPE, 2154)]);
        assert_eq!(Dem::open(&p).unwrap().epsg, 2154);
        let p = with_keys(&dir, "geo.tif", &[(GEOGRAPHIC_TYPE, 4326)]);
        assert_eq!(open_err(&p).code, "dem.not_projected");
    }

    #[test]
    fn several_tie_points_are_refused() {
        let dir = scratch("points");
        let p = dir.join("points.tif");
        raw_tiff::<Gray32Float>(&p, 2, 2, &[1.0; 4], |d| {
            d.write_tag(Tag::ModelPixelScaleTag, &[10.0, 10.0, 0.0][..])?;
            #[rustfmt::skip]
            let tp = [0.0, 0.0, 0.0, 652000.0, 6850300.0, 0.0,
                      1.0, 1.0, 0.0, 652010.0, 6850290.0, 0.0];
            d.write_tag(Tag::ModelTiepointTag, &tp[..])?;
            geokeys(d, L93)
        });
        assert_eq!(open_err(&p).code, "dem.unsupported");
    }

    #[test]
    fn a_non_numeric_nodata_is_refused() {
        let dir = scratch("nodata-texte");
        let p = dir.join("nd.tif");
        raw_tiff::<GrayI16>(&p, 2, 2, &[1, 2, 3, 4], |d| {
            projected(d)?;
            d.write_tag(Tag::GdalNodata, "rien")
        });
        assert_eq!(open_err(&p).code, "dem.unsupported");
    }

    #[test]
    fn a_tie_point_away_from_the_corner_gives_the_corner_origin() {
        let dir = scratch("rattachement");
        let p = dir.join("tp.tif");
        raw_tiff::<Gray32Float>(&p, 4, 4, &[1.0; 16], |d| {
            d.write_tag(Tag::ModelPixelScaleTag, &[10.0, 20.0, 0.0][..])?;
            d.write_tag(
                Tag::ModelTiepointTag,
                &[2.0, 3.0, 0.0, 652000.0, 6850300.0, 0.0][..],
            )?;
            geokeys(d, L93)
        });
        let dem = Dem::open(&p).unwrap();
        assert_eq!(
            (dem.origin_x, dem.origin_y),
            (652000.0 - 20.0, 6850300.0 + 60.0)
        );
    }

    #[test]
    fn a_non_integer_nodata_and_nan_are_masked() {
        let dem = Dem {
            nodata: Some(-9999.99),
            values: vec![-9999.99_f64 as f32, f32::NAN, 5.0],
            width: 3,
            height: 1,
            ..plan_incline()
        };
        assert_eq!(dem.value(0, 0), None);
        assert_eq!(dem.value(1, 0), None);
        assert_eq!(dem.value(2, 0), Some(5.0));
    }

    #[test]
    fn a_dem_beyond_one_gibibyte_is_refused_as_too_large() {
        // Un MNT de 1 m sur une commune (10 000 × 10 000 en Float32, 400 Mo) passe.
        assert!(check_size(10_000, 10_000, 32).is_ok());
        let e = check_size(20_000, 20_000, 32).unwrap_err();
        assert_eq!(e.code, "dem.unsupported");
        assert!(e.message.starts_with("MNT trop grand"), "{}", e.message);
        assert!(e.message.contains("1 Gio pour la grille"), "{}", e.message);
        assert!(e.message.contains("découpez-le dans QGIS"), "{}", e.message);
        let limit = map_read_error(TiffError::LimitsExceeded);
        assert_eq!(limit.code, "dem.unsupported");
        assert!(
            limit.message.starts_with("MNT trop grand"),
            "{}",
            limit.message
        );
        assert!(limit.message.ends_with(TOO_LARGE), "{}", limit.message);

        // De bout en bout : un TIFF de 2 × 2 dont les dimensions annoncées deviennent
        // 20 000 × 20 000 (1,6 Go en Float32), refusé avant tout décodage.
        let dir = scratch("trop-grand");
        let p = dir.join("grand.tif");
        raw_tiff::<Gray32Float>(&p, 2, 2, &[1.0; 4], projected);
        let mut bytes = std::fs::read(&p).unwrap();
        for tag in [0x00u8, 0x01] {
            // ImageWidth (256) puis ImageLength (257), type LONG, une valeur, 2.
            let entry = [
                tag, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00,
            ];
            let at = bytes
                .windows(entry.len())
                .position(|w| w == entry)
                .expect("entrée de dimension");
            bytes[at + 8..at + 12].copy_from_slice(&20_000u32.to_le_bytes());
        }
        std::fs::write(&p, bytes).unwrap();
        let e = open_err(&p);
        assert_eq!(e.code, "dem.unsupported", "{}", e.message);
        assert!(e.message.starts_with("MNT trop grand"), "{}", e.message);
    }

    #[test]
    fn an_inconsistent_hand_built_grid_returns_none() {
        let dem = Dem {
            values: vec![1.0],
            ..plan_incline()
        };
        assert_eq!(dem.value(5, 5), None);
    }

    #[test]
    fn identical_grids_have_no_mismatch() {
        let a = plan_incline();
        let b = Dem {
            values: vec![0.0; a.values.len()],
            nodata: None,
            ..a.clone()
        };
        assert_eq!(a.grid_mismatch(&b), None);
        assert_eq!(b.grid_mismatch(&a), None);
    }

    #[test]
    fn a_shift_below_a_millionth_of_a_cell_is_accepted() {
        let a = plan_incline();
        let tiny = a.cell_size_x * 5e-7;
        let b = Dem {
            origin_x: a.origin_x + tiny,
            origin_y: a.origin_y - tiny,
            cell_size_x: a.cell_size_x + tiny,
            ..a.clone()
        };
        assert_eq!(a.grid_mismatch(&b), None);
    }

    #[test]
    fn a_shifted_origin_is_named_with_its_size_and_values() {
        let a = plan_incline();
        let half = a.cell_size_x / 2.0;
        let b = Dem {
            origin_x: a.origin_x + half,
            ..a.clone()
        };
        let message = a.grid_mismatch(&b).unwrap();
        let expected = format!(
            "origine x décalée de {} m (0,5 cellule) : {} au lieu de {}",
            french(half),
            french(b.origin_x),
            french(a.origin_x)
        );
        assert_eq!(message, expected);
        let c = Dem {
            origin_y: a.origin_y - 2.0 * a.cell_size_y,
            ..a.clone()
        };
        let message = a.grid_mismatch(&c).unwrap();
        assert!(message.starts_with("origine y décalée de -"), "{message}");
        assert!(message.contains("(-2 cellules)"), "{message}");
    }

    #[test]
    fn other_differences_are_named_in_a_fixed_order() {
        let a = plan_incline();
        let dims = Dem {
            width: a.width + 1,
            values: vec![0.0; (a.width as usize + 1) * a.height as usize],
            ..a.clone()
        };
        assert_eq!(
            a.grid_mismatch(&dims).unwrap(),
            format!(
                "dimensions {} × {} cellules au lieu de {} × {}",
                a.width + 1,
                a.height,
                a.width,
                a.height
            )
        );
        let cell = Dem {
            cell_size_y: a.cell_size_y * 2.0,
            ..a.clone()
        };
        let message = a.grid_mismatch(&cell).unwrap();
        assert!(message.starts_with("taille de cellule "), "{message}");
        // Système et dimensions différents : le système l'emporte.
        let epsg = Dem {
            epsg: 32631,
            ..dims.clone()
        };
        assert_eq!(
            a.grid_mismatch(&epsg).unwrap(),
            "système EPSG:32631 au lieu d'EPSG:2154"
        );
    }

    #[test]
    fn numbers_in_messages_are_written_the_french_way() {
        assert_eq!(french(0.5), "0,5");
        assert_eq!(french(652000.0), "652000");
        assert_eq!(french(-5.0), "-5");
        assert_eq!(french(0.1 + 0.2), "0,3");
        assert_eq!(french(-1e-9), "0");
    }
}
