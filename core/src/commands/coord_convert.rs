//! Commande `coord.convert` : convertit **un** point entre WGS84 (EPSG:4326), UTM sur WGS84
//! (EPSG:32601 à 32660, 32701 à 32760) et MGRS, dans le noyau (AD-19 amendé, AD-31, CAP-15).
//!
//! Les calculs sont ceux de `geodesy.rs` (Karney, NGA). Jamais une couche, jamais un autre
//! système : un point en Lambert-93 passe d'abord par `data.transform_point`, que l'erreur
//! `geo.unsupported_crs` nomme. Une conversion n'est jamais insérée en silence avant un autre
//! calcul : elle est une étape visible du plan.

use schemars::JsonSchema;
use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use super::geo_point::{
    check_grid_latitude, out_of_range, read_point, utm_point, utm_zone_param, wgs84_point,
};
use crate::geodesy::{self, Mgrs};
use crate::point::{MgrsRef, Point};
use crate::registry::{Command, CommandFailure, CommandSpec, Registration, RunsIn};

// Une forme demandée : WGS84 (degrés décimaux), UTM sur WGS84 (mètres) ou MGRS. Sans
// commentaire de documentation : il deviendrait une description par valeur dans le schéma du
// long prompt, qui doit tenir dans le contexte du modèle avec l'inventaire du projet.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Form {
    Wgs84,
    Utm,
    Mgrs,
}

/// Paramètres de `coord.convert`. Les commentaires de documentation des champs deviennent les
/// descriptions du schéma, lues par le modèle dans le long prompt : ils restent courts.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CoordConvertParams {
    /// Point écrit dans la demande (EPSG 4326, 326NN ou 327NN), ou absent.
    #[serde(default)]
    #[schemars(with = "Point")]
    pub point: Option<Point>,
    /// Référence MGRS recopiée de la demande, ou absente.
    #[serde(default)]
    #[schemars(with = "MgrsRef")]
    pub mgrs: Option<MgrsRef>,
    /// Formes voulues, sans doublon.
    #[serde(deserialize_with = "forms")]
    #[schemars(length(min = 1, max = 3))]
    pub to: Vec<Form>,
    /// Zone UTM imposée, seulement si la demande la nomme.
    #[serde(default, deserialize_with = "utm_zone_param")]
    #[schemars(range(min = 1, max = 60))]
    pub utm_zone: Option<u8>,
    /// Chiffres MGRS par axe, seulement si la demande les fixe.
    #[serde(default, deserialize_with = "precision")]
    #[schemars(range(min = 1, max = 5))]
    pub mgrs_precision: Option<u8>,
}

fn forms<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Form>, D::Error> {
    let forms = Vec::<Form>::deserialize(d)?;
    if forms.is_empty() || forms.len() > 3 {
        return Err(D::Error::custom(
            "`to` demande de 1 à 3 formes parmi wgs84, utm et mgrs",
        ));
    }
    if forms.iter().enumerate().any(|(i, f)| forms[..i].contains(f)) {
        return Err(D::Error::custom("`to` ne répète pas une forme"));
    }
    Ok(forms)
}

fn precision<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u8>, D::Error> {
    let precision = Option::<u8>::deserialize(d)?;
    if precision.is_some_and(|p| !(1..=5).contains(&p)) {
        return Err(D::Error::custom("`mgrs_precision` va de 1 à 5"));
    }
    Ok(precision)
}

/// Résultat de `coord.convert` : un champ par forme demandée, chacun utilisable comme
/// référence d'étape (AD-23).
#[derive(Debug, Serialize, JsonSchema)]
pub struct CoordConvertOutput {
    /// Présent si `wgs84` est demandé : x longitude, y latitude, en degrés décimaux.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Point")]
    pub wgs84: Option<Point>,
    /// Présent si `utm` est demandé : x, y en mètres, `epsg` de la zone.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "Point")]
    pub utm: Option<Point>,
    /// Présent si `mgrs` est demandé, forme canonique.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "MgrsRef")]
    pub mgrs: Option<MgrsRef>,
}

/// Lit la référence MGRS d'entrée : coin sud-ouest de sa case, en WGS84.
fn read_mgrs(reference: &MgrsRef) -> Result<(f64, f64), CommandFailure> {
    let invalid = |e: geodesy::MgrsError| {
        CommandFailure::new(
            "coord.invalid_mgrs",
            format!(
                "La référence MGRS « {} » est mal formée ({}) : {}.",
                reference.0.trim(),
                e.part.label(),
                e.detail
            ),
        )
    };
    let mgrs = reference.parse().map_err(invalid)?;
    let corner = mgrs.south_west().map_err(invalid)?;
    Ok(geodesy::from_utm(&corner))
}

/// `coord.convert` : commande du noyau.
pub struct CoordConvert;

impl Command for CoordConvert {
    type Params = CoordConvertParams;
    type Output = CoordConvertOutput;

    fn spec() -> CommandSpec {
        CommandSpec::new::<CoordConvertParams, CoordConvertOutput>(
            "coord.convert",
            "Convertit un seul point entre WGS84 (EPSG:4326, x longitude, y latitude), UTM (EPSG:326NN, 327NN) et MGRS : exactement un de `point` (coordonnées de la demande) et `mgrs` (référence MGRS de la demande) ; `to` liste les formes voulues. Un point dans un autre système passe d'abord par data.transform_point.",
            "Converts a single point between WGS84 (EPSG:4326, x longitude, y latitude), UTM on WGS84 (EPSG:326NN north, 327NN south) and MGRS. Give exactly one of `point` (coordinates written in the request) and `mgrs` (MGRS reference copied from the request); `to` lists the wanted forms. To MGRS, the reference is truncated; a read MGRS reference gives the south-west corner of its square. Never a layer nor another system: a Lambert-93 point goes through data.transform_point first.",
        )
        .runs_in(RunsIn::Core)
        .cancellable(false)
        .example(serde_json::json!({"point": {"x": 2.3, "y": 48.86, "epsg": 4326}, "to": ["utm", "mgrs"]}))
        .example(serde_json::json!({"mgrs": "31UDQ4825111750", "to": ["wgs84"]}))
        .tests(&[
            "geographiclib_references_are_found",
            "projection_references_are_found_to_the_millimetre",
            "the_special_zones_are_those_of_the_exception",
            "geotrans_references_are_found",
            "an_imposed_zone_is_kept_when_it_is_a_neighbour",
            "the_mgrs_precision_truncates",
            "a_mgrs_reference_reads_as_the_south_west_corner",
            "exactly_one_input_is_required",
            "a_malformed_mgrs_names_the_faulty_part",
            "another_system_names_data_transform_point",
            "a_polar_latitude_is_out_of_range",
            "a_conversion_round_trips_and_repeats_identically",
            "coord_convert_runs_in_the_core_and_the_contract_accepts_it",
        ])
    }

    fn run(&self, params: CoordConvertParams) -> Result<CoordConvertOutput, CommandFailure> {
        let (lat, lon) = match (&params.point, &params.mgrs) {
            (Some(point), None) => read_point(point, "coord.convert")?,
            (None, Some(reference)) => read_mgrs(reference)?,
            _ => {
                return Err(CommandFailure::new(
                    "coord.one_input",
                    "Donnez exactement un point à convertir : soit `point` (x, y et système), soit `mgrs` (une référence MGRS), pas les deux.",
                ))
            }
        };
        let wants = |form: Form| params.to.contains(&form);
        let mut output = CoordConvertOutput {
            wgs84: None,
            utm: None,
            mgrs: None,
        };
        if wants(Form::Wgs84) {
            output.wgs84 = Some(wgs84_point(lat, lon));
        }
        if wants(Form::Utm) {
            check_grid_latitude(lat)?;
            let normal = geodesy::utm_zone(lat, lon);
            let zone = params.utm_zone.unwrap_or(normal);
            if geodesy::zone_distance(zone, normal) > 1 {
                return Err(out_of_range(format!(
                    "La zone UTM {zone} est à plus d'une zone de celle du point ({normal}) : une zone imposée ne peut être que la sienne ou une voisine."
                )));
            }
            output.utm = Some(utm_point(lat, lon, zone));
        }
        if wants(Form::Mgrs) {
            check_grid_latitude(lat)?;
            let precision = params.mgrs_precision.unwrap_or(5);
            let mgrs = Mgrs::from_lat_lon(lat, lon, precision).ok_or_else(|| {
                out_of_range(format!(
                    "Le point ({lon}, {lat}) n'a pas de référence MGRS dans sa zone."
                ))
            })?;
            output.mgrs = Some(MgrsRef::canonical(&mgrs));
        }
        Ok(output)
    }
}

inventory::submit! { Registration(|r| r.register(CoordConvert)) }

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use super::*;
    use crate::registry::{Registry, RegistryError};

    fn convert(params: Value) -> Result<Value, RegistryError> {
        Registry::standard().call("coord.convert", params)
    }

    fn wgs84(lon: f64, lat: f64) -> Value {
        json!({"x": lon, "y": lat, "epsg": 4326})
    }

    #[track_caller]
    fn assert_near(got: &Value, x: f64, y: f64, epsg: u32, tolerance: f64) {
        assert_eq!(got["epsg"], epsg, "{got}");
        let (gx, gy) = (got["x"].as_f64().unwrap(), got["y"].as_f64().unwrap());
        assert!(
            (gx - x).abs() <= tolerance && (gy - y).abs() <= tolerance,
            "attendu ({x}, {y}), trouvé ({gx}, {gy})"
        );
    }

    fn failure(params: Value) -> (String, String) {
        let err = convert(params).unwrap_err();
        (err.code().to_string(), err.to_string())
    }

    // Jeu de référence 1 — GeographicLib, page de manuel de GeoConvert (C. F. F. Karney,
    // https://geographiclib.sourceforge.io/C++/doc/GeoConvert.1.html, GeographicLib sous
    // licence MIT/X11) : exemples des sections EXAMPLES, MGRS, ZONE et des désignateurs
    // d'hémisphère, au centimètre ou au mètre près selon les chiffres publiés.
    #[test]
    fn geographiclib_references_are_found() {
        // « 38n 444140.54 3684706.36 » pour 33,3° N 44,4° E.
        let out = convert(json!({"point": wgs84(44.4, 33.3), "to": ["utm"]})).unwrap();
        // Valeurs publiées au centimètre : un demi-centimètre d'écart, plus le millimètre rendu.
        assert_near(&out["utm"], 444140.54, 3684706.36, 32638, 0.006);
        // « E44d24 N33d20 | GeoConvert -m -p -3 => 38SMB4488 ».
        let out = convert(json!({"point": wgs84(44.4, 33.0 + 20.0 / 60.0), "to": ["mgrs"],
            "mgrs_precision": 2}))
        .unwrap();
        assert_eq!(out["mgrs"], "38SMB4488");
        // « 32CMS4324728161 (standard UTM zone = 32) » pour 79,9° S 6,1° E, et « 32CMS4328 »
        // à deux chiffres.
        let point = wgs84(6.1, -79.9);
        let out = convert(json!({"point": point, "to": ["mgrs"]})).unwrap();
        assert_eq!(out["mgrs"], "32CMS4324728161");
        let out = convert(json!({"point": point, "to": ["mgrs"], "mgrs_precision": 2})).unwrap();
        assert_eq!(out["mgrs"], "32CMS4328");
        // « 31CEM6066227959 (neighboring UTM zone = 31) » : la zone voisine imposée.
        let out = convert(json!({"point": point, "to": ["utm"], "utm_zone": 31})).unwrap();
        assert_eq!(out["utm"]["epsg"], 32731);
        assert_eq!(out["utm"]["x"].as_f64().unwrap().floor(), 560662.0);
        assert_eq!(out["utm"]["y"].as_f64().unwrap().floor(), 1127959.0);
        // « echo -1 3 | GeoConvert -u => 31s 500000 9889470 » : l'hémisphère suit la latitude.
        let out = convert(json!({"point": wgs84(3.0, -1.0), "to": ["utm"]})).unwrap();
        assert_near(&out["utm"], 500000.0, 9889470.0, 32731, 0.5);
        // « 38SMB4488 … the south-west corner of the square is returned (38n 444000 3688000) ».
        let out = convert(json!({"mgrs": "38SMB4488", "to": ["utm"]})).unwrap();
        assert_near(&out["utm"], 444000.0, 3688000.0, 32638, 0.001);
    }

    // Jeu de référence 2 — PROJ 9.8.1 (`cs2cs`, livré avec QGIS 3.44 ; PROJ sous licence MIT),
    // implémentation indépendante du Mercator transverse (Poder et Engsager), valeurs figées au
    // dixième de millimètre le 2026-11-05 : vérification croisée, PROJ n'est jamais lancé par
    // les tests.
    const PROJ_REFERENCES: [(f64, f64, u32, f64, f64); 9] = [
        // (longitude, latitude, EPSG UTM, abscisse, ordonnée)
        (2.294481, 48.858370, 32631, 448250.5768, 5411951.5880),
        (151.215297, -33.856784, 32756, 334900.2613, 6252290.5224),
        (-74.044502, 40.689247, 32618, 580735.6450, 4504700.3807),
        (44.4, 33.3, 32638, 444140.5449, 3684706.3556),
        (6.1, -79.9, 32732, 443247.8718, 1128161.3729),
        (5.32, 60.39, 32632, 297230.2202, 6700510.1753),
        (15.65, 78.22, 32633, 514813.5273, 8683004.1533),
        (8.0, 78.0, 32631, 615914.5249, 8663320.2014),
        (40.0, 78.0, 32637, 523208.7381, 8658567.6999),
    ];

    #[test]
    fn projection_references_are_found_to_the_millimetre() {
        for (lon, lat, epsg, x, y) in PROJ_REFERENCES {
            let out = convert(json!({"point": wgs84(lon, lat), "to": ["utm"]})).unwrap();
            assert_near(&out["utm"], x, y, epsg, 0.001);
            // Et l'inverse : l'UTM rendu redonne le point au milliardième de degré près.
            let back = convert(json!({"point": out["utm"], "to": ["wgs84"]})).unwrap();
            assert_near(&back["wgs84"], lon, lat, 4326, 2e-8);
        }
        // Une zone imposée voisine, à l'est et à l'ouest (même source).
        let point = wgs84(2.458, 48.658);
        let out = convert(json!({"point": point, "to": ["utm"], "utm_zone": 30})).unwrap();
        assert_near(&out["utm"], 901860.4901, 5403829.2769, 32630, 0.001);
        let out = convert(json!({"point": point, "to": ["utm"], "utm_zone": 32})).unwrap();
        assert_near(&out["utm"], 18368.2161, 5410124.2978, 32632, 0.001);
    }

    // Jeu de référence 3 — NGA, NGA.SIG.0012_2.0.0_UTMUPS (2014), règles des zones 32V et 31X
    // à 37X ; références MGRS lettrées par le schéma AA de la même spécification, sur les
    // coordonnées du jeu 2.
    #[test]
    fn the_special_zones_are_those_of_the_exception() {
        for (lon, lat, utm, mgrs) in [
            // Bergen : zone 32 et non 31.
            (5.32, 60.39, 32632, "32VKN9723000510"),
            // Svalbard : 31X, 33X, 35X et 37X, les zones paires n'existent pas.
            (8.0, 78.0, 32631, "31XFG1591463320"),
            (15.65, 78.22, 32633, "33XWG1481383004"),
            (20.0, 78.0, 32633, "33XXG1591463320"),
            (30.0, 78.0, 32635, "35XNG6960060152"),
            (40.0, 78.0, 32637, "37XEG2320858567"),
        ] {
            let out = convert(json!({"point": wgs84(lon, lat), "to": ["utm", "mgrs"]})).unwrap();
            assert_eq!(out["utm"]["epsg"], utm, "{lon} {lat}");
            assert_eq!(out["mgrs"], mgrs, "{lon} {lat}");
        }
        // Hors de l'exception, la zone normale : 2,9° E à 60,4° N reste en 31.
        let out = convert(json!({"point": wgs84(2.9, 60.4), "to": ["utm"]})).unwrap();
        assert_eq!(out["utm"]["epsg"], 32631);
    }

    // Jeu de référence 4 — GEOTRANS de la NGA (domaine public), lu par le paquet Python `mgrs`
    // 1.7.0 (licence MIT), valeurs figées le 2026-11-05 : implémentation indépendante du MGRS.
    // Points tirés au hasard dans le monde, en 31V/32V, au Svalbard et au bord des zones et des
    // bandes, loin de plus de 5 mm de tout mètre entier en PROJ (là, GEOTRANS arrondit avant de
    // tronquer et peut rendre le mètre suivant : la troncature exacte est celle du noyau).
    // Contrôle croisé du 2026-11-05 sur 8 500 points : aucun écart hors de ces cas sub-millimétriques.
    #[test]
    fn geotrans_references_are_found() {
        for (lon, lat, expected) in [
            (-144.3036777, -67.607879, "06DXK1459498358"),
            (-52.6953046, -73.1221623, "22CDD4506885114"),
            (-161.217936, 73.892651, "04XDH3133001339"),
            (-85.7807689, 50.1482275, "16UEA8710855823"),
            (73.4480354, -53.3247044, "43FCA9663390484"),
            (63.0407992, -69.4369913, "41DNC0159996909"),
            (6.3356227, 12.3986362, "32PKU1029672081"),
            (126.0975948, 54.5647725, "52UCF1235950235"),
            (5.0217254, 62.7384361, "32VKQ9678462722"),
            (9.3635763, 59.1796754, "32VNL2077960115"),
            (3.0364814, 59.4287091, "32VJM6194602958"),
            (0.9095442, 56.735837, "31VCC7212289932"),
            (9.7750897, 63.8723028, "32VNR3808383015"),
            (0.9599598, 58.1317739, "31VCE7986945197"),
            (10.8319159, 56.8863918, "32VPJ1161606234"),
            (3.7072022, 58.3609033, "32VJK9049981075"),
            (24.83953, 81.7635812, "35XML6545479082"),
            (19.316775, 74.638288, "33XXC2754487885"),
            (1.6756448, 81.3265589, "31XDL7770529910"),
            (1.8151759, 83.6194539, "31XDN8530085762"),
            (38.8645734, 77.9950632, "37XDG9685557822"),
            (22.1195978, 82.9101096, "35XMN3282809265"),
            (10.0105563, 83.0147173, "33XVN3233821028"),
            (4.0811639, 73.5886702, "31XEB3409366458"),
            (9.0000769, -53.0890285, "32FNG0000517825"),
            (8.9999067, -17.1311797, "32KMG9999005933"),
            (162.0000777, -48.0000352, "58FBM7623079340"),
            (-75.6376249, 48.4385331, "18UVU5284065239"),
            (6.0000017, 63.5352986, "32VLR5087248729"),
            (-3.8100907, 24.8519844, "30RVN1815448801"),
        ] {
            let out = convert(json!({"point": wgs84(lon, lat), "to": ["mgrs"]})).unwrap();
            assert_eq!(out["mgrs"], expected, "{lon} {lat}");
            // Et la référence de GEOTRANS se relit sans refus.
            assert!(convert(json!({"mgrs": expected, "to": ["wgs84"]})).is_ok(), "{expected}");
        }
    }

    #[test]
    fn an_imposed_zone_is_kept_when_it_is_a_neighbour() {
        let point = wgs84(2.458, 48.658);
        for (zone, epsg) in [(31, 32631), (30, 32630), (32, 32632)] {
            let out = convert(json!({"point": point, "to": ["utm"], "utm_zone": zone})).unwrap();
            assert_eq!(out["utm"]["epsg"], epsg);
        }
        for zone in [33, 29, 1] {
            let (code, message) =
                failure(json!({"point": point, "to": ["utm"], "utm_zone": zone}));
            assert_eq!(code, "coord.out_of_range", "{zone}");
            assert!(message.contains("voisine"), "{message}");
        }
        // De 60 à 1 : voisines.
        let out = convert(json!({"point": wgs84(179.5, 10.0), "to": ["utm"], "utm_zone": 1}))
            .unwrap();
        assert_eq!(out["utm"]["epsg"], 32601);
        for zone in [0, 61] {
            let (code, _) = failure(json!({"point": point, "to": ["utm"], "utm_zone": zone}));
            assert_eq!(code, "registry.invalid_params");
        }
    }

    #[test]
    fn the_mgrs_precision_truncates() {
        // 448250,58 E, 5411951,59 N (jeu 2) : les mètres sont tronqués, pas arrondis.
        let point = wgs84(2.294481, 48.858370);
        for (precision, expected) in [
            (None, "31UDQ4825011951"),
            (Some(4), "31UDQ48251195"),
            (Some(2), "31UDQ4811"),
            (Some(1), "31UDQ41"),
        ] {
            let mut params = json!({"point": point, "to": ["mgrs"]});
            if let Some(p) = precision {
                params["mgrs_precision"] = json!(p);
            }
            assert_eq!(convert(params).unwrap()["mgrs"], expected);
        }
        for precision in [0, 6] {
            let (code, _) =
                failure(json!({"point": point, "to": ["mgrs"], "mgrs_precision": precision}));
            assert_eq!(code, "registry.invalid_params");
        }
        for to in [json!([]), json!(["mgrs", "mgrs"]), json!(["lambert93"])] {
            let (code, _) = failure(json!({"point": point, "to": to}));
            assert_eq!(code, "registry.invalid_params", "{to}");
        }
    }

    #[test]
    fn a_mgrs_reference_reads_as_the_south_west_corner() {
        // Coin sud-ouest 448251 E, 5411750 N de la zone 31 ; WGS84 de PROJ 9.8.1 (jeu 2).
        for reference in ["31u dq 48251 11750", "31UDQ4825111750", " 31U DQ 4825111750 "] {
            let out = convert(json!({"mgrs": reference, "to": ["wgs84", "utm", "mgrs"]})).unwrap();
            assert_near(&out["wgs84"], 2.2945122525, 48.8565567193, 4326, 2e-9);
            assert_near(&out["utm"], 448251.0, 5411750.0, 32631, 0.001);
            assert_eq!(out["mgrs"], "31UDQ4825111750");
        }
        // Une zone sur un chiffre, et une case de 10 km : son coin.
        let out = convert(json!({"mgrs": "38SMB4488", "to": ["wgs84"]})).unwrap();
        assert_near(&out["wgs84"], 44.3982863863, 33.3296994741, 4326, 2e-9);
        let out = convert(json!({"mgrs": "31UDQ41", "to": ["utm"]})).unwrap();
        assert_near(&out["utm"], 440000.0, 5410000.0, 32631, 0.001);
        // Un point UTM se lit aussi, au sud comme au nord.
        let out = convert(json!({"point": {"x": 500000.0, "y": 9889469.8412, "epsg": 32731},
            "to": ["wgs84"]}))
        .unwrap();
        assert_near(&out["wgs84"], 3.0, -1.0, 4326, 2e-9);
        // Des nombres qui ne sont pas un point de la zone : trop loin à l'est, ordonnée du sud
        // sous un code du nord (89° N, puis au-delà de 84° N), ordonnée du nord sous un code du sud.
        for (x, y, epsg) in [
            (5_000_000.0, 5_411_750.0, 32631),
            (448_251.0, 9_889_470.0, 32631),
            (500_000.0, 9_600_000.0, 32631),
            (500_000.0, 10_100_000.0, 32731),
        ] {
            let (code, message) =
                failure(json!({"point": {"x": x, "y": y, "epsg": epsg}, "to": ["wgs84"]}));
            assert_eq!(code, "coord.out_of_range", "{x} {y} {epsg}");
            assert!(message.contains(&format!("hors de la zone UTM EPSG:{epsg}")), "{message}");
        }
    }

    #[test]
    fn exactly_one_input_is_required() {
        let (code, _) = failure(json!({"to": ["wgs84"]}));
        assert_eq!(code, "coord.one_input");
        let (code, message) = failure(json!({"point": wgs84(2.3, 48.8),
            "mgrs": "31UDQ4825111750", "to": ["wgs84"]}));
        assert_eq!(code, "coord.one_input");
        assert!(message.contains("pas les deux"), "{message}");
    }

    #[test]
    fn a_malformed_mgrs_names_the_faulty_part() {
        for (reference, part) in [
            ("UDQ4825111750", "zone"),
            ("61UDQ4825111750", "zone"),
            ("31IDQ4825111750", "bande"),
            ("31ODQ4825111750", "bande"),
            ("31ZDQ4825111750", "bande"),
            ("31TDQ4825111750", "bande"),
            ("31USQ4825111750", "carré de 100 km"),
            ("31UDW4825111750", "carré de 100 km"),
            ("31UD", "carré de 100 km"),
            ("31UDQ482511175", "chiffres"),
            ("31UDQ", "chiffres"),
            ("31UDQ48251117501", "chiffres"),
            ("31UDQ482511175000", "chiffres"),
            ("31UDQ48251-11750", "chiffres"),
            // Zones de grille qui n'existent pas au Svalbard.
            ("32XNG1591463320", "zone"),
            ("34XEG1591463320", "zone"),
            ("36XSG1591463320", "zone"),
            // Un carré de 31V tout entier à l'est de 3° E (il appartient à 32V).
            ("31VFG0000000000", "carré de 100 km"),
        ] {
            let (code, message) = failure(json!({"mgrs": reference, "to": ["wgs84"]}));
            assert_eq!(code, "coord.invalid_mgrs", "{reference}");
            assert!(message.contains(&format!("({part})")), "{reference} : {message}");
        }
    }

    #[test]
    fn another_system_names_data_transform_point() {
        for epsg in [2154, 3857, 32600, 32661, 32700, 32761] {
            let (code, message) = failure(json!({"point": {"x": 652000, "y": 6850000,
                "epsg": epsg}, "to": ["wgs84"]}));
            assert_eq!(code, "geo.unsupported_crs", "{epsg}");
            assert!(message.contains("data.transform_point"), "{message}");
            assert!(message.contains(&format!("EPSG:{epsg}")), "{message}");
        }
    }

    #[test]
    fn a_polar_latitude_is_out_of_range() {
        for (lat, form) in [(85.0, "mgrs"), (85.0, "utm"), (-80.5, "mgrs"), (-80.5, "utm")] {
            let (code, _) = failure(json!({"point": wgs84(10.0, lat), "to": [form]}));
            assert_eq!(code, "coord.out_of_range", "{lat} {form}");
        }
        // Le WGS84 seul n'a pas de borne de grille ; au-delà de 90°, ce n'est plus un point.
        assert!(convert(json!({"point": wgs84(10.0, 85.0), "to": ["wgs84"]})).is_ok());
        let (code, _) = failure(json!({"point": wgs84(10.0, 91.0), "to": ["wgs84"]}));
        assert_eq!(code, "coord.out_of_range");
        // 84° N et 80° S sont dans la grille ; 10° E à 84° N est au Svalbard, zone 33X.
        assert_eq!(
            convert(json!({"point": wgs84(10.0, 84.0), "to": ["mgrs"], "mgrs_precision": 1}))
                .unwrap()["mgrs"]
                .as_str()
                .unwrap()[..3],
            *"33X"
        );
        assert!(convert(json!({"point": wgs84(10.0, -80.0), "to": ["mgrs"]})).is_ok());
    }

    #[test]
    fn a_conversion_round_trips_and_repeats_identically() {
        for (lon, lat) in [
            (2.4, 48.7),
            (-58.4, -34.6),
            (139.7, 35.7),
            (-0.0001, 0.0001),
            (8.99, 71.99),
            (2.9, 47.99999),
        ] {
            let first =
                convert(json!({"point": wgs84(lon, lat), "to": ["mgrs", "utm"]})).unwrap();
            let again =
                convert(json!({"point": wgs84(lon, lat), "to": ["utm", "mgrs"]})).unwrap();
            assert_eq!(first, again);
            // La case relue redonne la même référence (cohérence de la bande comprise).
            let corner = convert(json!({"mgrs": first["mgrs"], "to": ["mgrs"]})).unwrap();
            assert_eq!(corner["mgrs"], first["mgrs"], "{lon} {lat}");
            // Le résultat écrit puis relu en JSON est le même : condition du rejeu.
            let all = convert(json!({"mgrs": first["mgrs"], "to": ["wgs84", "utm"]})).unwrap();
            let text = all.to_string();
            assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), all, "{text}");
            assert!(text.len() < 120, "{text}");
        }
    }

    #[test]
    fn coord_convert_runs_in_the_core_and_the_contract_accepts_it() {
        let registry = Registry::standard();
        let spec = registry.spec("coord.convert").unwrap();
        assert_eq!(spec.runs_in, Some(RunsIn::Core));
        assert!(!spec.dictated_by_core);
        let violations = registry.check_contract();
        assert!(
            violations.iter().all(|v| v.id != "coord.convert"),
            "{violations:?}"
        );
        // `point` et les champs `wgs84` et `utm` sont des `Point`, `mgrs` des `MgrsRef` : un champ
        // du résultat se reprend par référence d'étape dans le paramètre du même type.
        let params = &spec.params_schema;
        let output = &spec.output_schema;
        assert!(crate::point::is_point(&params["properties"]["point"], params));
        assert!(crate::point::is_mgrs(&params["properties"]["mgrs"], params));
        assert!(crate::point::is_point(&output["properties"]["wgs84"], output));
        assert!(crate::point::is_mgrs(&output["properties"]["mgrs"], output));
        let plan: crate::plan::Plan = serde_json::from_value(json!({"kind": "calls", "calls": [
            {"command": "coord.convert", "params": {"mgrs": "31UDQ4825111750",
                "to": ["wgs84", "mgrs"]}},
            {"command": "coord.convert", "params": {"point": "step:1.wgs84", "to": ["utm"]}},
            {"command": "coord.convert", "params": {"mgrs": "step:1.mgrs", "to": ["wgs84"]}}
        ]}))
        .unwrap();
        let checked = registry.check_plan(&plan);
        assert!(checked.is_ok(), "{checked:?}");
    }
}
