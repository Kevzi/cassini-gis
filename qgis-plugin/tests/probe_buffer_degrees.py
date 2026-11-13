"""Sonde manuelle sous le vrai QGIS : tampon sur une couche en degrés (entrée 9.16).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_buffer_degrees.py

Aucun noyau ni aucune donnée hors dépôt : deux couches mémoire synthétiques, des points autour
d'Annecy en EPSG:4326. Le geste reçoit `work_epsg` comme le noyau le lui donne.

1. Tampon de 500 m par le geste sur les points en EPSG:4326, `work_epsg` = 2154, comparé au
   tampon fait après un `data.reproject` manuel vers EPSG:2154 : même système, même nombre
   d'entités ; écart de surface et distance de Hausdorff imprimés, au plus 5 m par entité. Contrôle
   indépendant de la reprojection : la distance ellipsoïdale (`QgsDistanceArea`, WGS84) du point
   à chaque sommet du contour du tampon vaut 500 m, à 3 m près.
2. Couche déjà en EPSG:2154 (la reprojetée du point 1), sans `work_epsg`, comme le noyau la
   dicte (amendement RA06) : même système, même nombre d'entités et même géométrie que
   `native:buffer` lancé directement.
3. `work_epsg` = 4326 : `analysis.crs_not_metric`, rien lancé, aucun fichier.
4. La couche d'origine reste en EPSG:4326, avec ses entités.
5. Couche en mètres (GeoPackage écrit par OGR) dont une entité est une `GeometryCollection` de
   lignes et de points dans une couche de lignes (le cas de RA06), sans `work_epsg`, comme le
   noyau la dicte : le tampon passe, une entité par entité d'origine.
"""

import os
import shutil
import sys
import tempfile
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import (  # noqa: E402
    QgsApplication,
    QgsCoordinateReferenceSystem,
    QgsCoordinateTransform,
    QgsDistanceArea,
    QgsFeature,
    QgsGeometry,
    QgsPointXY,
    QgsProject,
    QgsVectorLayer,
    QgsWkbTypes,
)
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402

# Gares autour d'Annecy (longitude, latitude), synthétiques.
POINTS = [("annecy", 6.1222, 45.9013), ("pringy", 6.1235, 45.9465), ("rumilly", 5.9440, 45.8676)]
DISTANCE = {"value": 500, "unit": "m"}
MAX_GAP_M = 5.0
MAX_RADIUS_GAP_M = 3.0
# Les contrôles attendus, tous : le verdict exige chacun d'eux, nommé, et vrai.
EXPECTED = ["native:buffer seul s'exécute", "geste : pas d'erreur", "geste : tampon en EPSG:2154",
            "après data.reproject : pas d'erreur", "même nombre d'entités"]
for _name, _, _ in POINTS:
    EXPECTED += ["%s : Hausdorff ≤ %.0f m" % (_name, MAX_GAP_M),
                 "%s : écart de surface sous celui de 5 m de rayon" % _name,
                 "%s : contour à 500 m du point (ellipsoïde), à %.0f m près" % (_name, MAX_RADIUS_GAP_M)]
EXPECTED += ["couche métrique : même système que native:buffer seul", "couche métrique : même nombre d'entités",
             "couche métrique : même géométrie", "work_epsg 4326 : analysis.crs_not_metric",
             "work_epsg 4326 : aucun fichier", "la couche d'origine reste en EPSG:4326",
             "la couche d'origine garde ses entités", "collection de géométries : le tampon passe",
             "collection de géométries : une entité par entité d'origine",
             "collection de géométries : tampon en EPSG:2154"]

work = tempfile.mkdtemp(prefix="cassini-tampon-degres-")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))

gares = QgsVectorLayer("Point?crs=EPSG:4326&field=name:string", "gares", "memory")
features = []
for name, lon, lat in POINTS:
    f = QgsFeature(gares.fields())
    f.setAttribute("name", name)
    f.setGeometry(QgsGeometry.fromPointXY(QgsPointXY(lon, lat)))
    features.append(f)
gares.dataProvider().addFeatures(features)
gares.updateExtents()
project.addMapLayer(gares)

checks = []
outcomes = {}


def check(label, condition, detail=""):
    checks.append((label, bool(condition)))
    print(("ok     " if condition else "ECHEC  ") + label + (" : %s" % (detail,) if detail and not condition else ""))


def finish():
    QTimer.singleShot(0, QCoreApplication.quit)


def protect(fn):
    def wrapper(*args, **kwargs):
        try:
            fn(*args, **kwargs)
        except Exception:
            traceback.print_exc()
            check("aucune exception dans la sonde", False)
            finish()

    return wrapper


def gesture(command, params, label, then):
    """Lance le geste de `command` ; garde (résultat, erreur, sortie) sous `label`, puis `then`."""

    def done(result=None, error=None):
        outcomes[label] = (result, error, params.get("out"))
        QTimer.singleShot(0, protect(then))

    g = analysis.ProcessingGesture(analysis.ALGORITHMS[command])
    outcomes["_gesture_" + label] = g  # gardé en vie pendant la tâche
    g.start(params, done)


def out(name):
    return os.path.join(work, name)


def by_name(layer):
    return {f["name"]: f.geometry() for f in layer.getFeatures()}


# -- 1. Tampon par le geste, sur la couche en degrés --------------------------------------
def step_buffer_degrees():
    gesture(
        "analysis.buffer",
        {"layer_id": gares.id(), "distance": DISTANCE, "work_epsg": 2154, "out": out("1-tampon-degres.gpkg")},
        "geste",
        step_reproject,
    )


def step_reproject():
    gesture(
        "data.reproject",
        {"layer_id": gares.id(), "target_epsg": 2154, "out": out("2-gares-2154.gpkg")},
        "reprojection",
        step_buffer_metric,
    )


def step_buffer_metric():
    reprojected = outcomes["reprojection"][0]
    if not reprojected:
        check("data.reproject manuel", False, outcomes["reprojection"][1])
        finish()
        return
    gesture(
        "analysis.buffer",
        {"layer_id": reprojected["layer_id"], "distance": DISTANCE, "out": out("3-tampon-metrique.gpkg")},
        "metrique",
        step_native,
    )


def step_native():
    """`native:buffer` lancé directement sur la couche en mètres, sans reprojection."""
    from qgis.core import QgsProcessingContext, QgsProcessingFeedback, QgsProcessingUtils

    reprojected = project.mapLayer(outcomes["reprojection"][0]["layer_id"])
    params = analysis.translate_buffer({"distance": DISTANCE}, reprojected)
    params["OUTPUT"] = "TEMPORARY_OUTPUT"
    context = QgsProcessingContext()
    context.setProject(project)
    algorithm = QgsApplication.processingRegistry().algorithmById("native:buffer")
    results, ok = algorithm.run(params, context, QgsProcessingFeedback())
    check("native:buffer seul s'exécute", ok)
    outcomes["native_context"] = context  # la couche temporaire vit avec son contexte
    outcomes["native"] = QgsProcessingUtils.mapLayerFromString(results["OUTPUT"], context) if ok else None
    gesture(
        "analysis.buffer",
        {"layer_id": gares.id(), "distance": DISTANCE, "work_epsg": 4326, "out": out("4-refus.gpkg")},
        "refus",
        step_collection,
    )


def collection_layer():
    """GeoPackage en EPSG:2154, couche de lignes, dont une entité est une `GeometryCollection`
    (ligne et point), écrit par OGR comme un découpage peut le laisser ; ouvert par QGIS."""
    from osgeo import ogr, osr

    path = out("lignes-collection.gpkg")
    driver = ogr.GetDriverByName("GPKG")
    source = driver.CreateDataSource(path)
    srs = osr.SpatialReference()
    srs.ImportFromEPSG(2154)
    lines = source.CreateLayer("cours_d_eau", srs, ogr.wkbMultiLineString)
    for wkt in ("MULTILINESTRING((940000 6540000,940500 6540200))",
                "GEOMETRYCOLLECTION(LINESTRING(941000 6541000,941400 6541300),POINT(941600 6541500))"):
        feature = ogr.Feature(lines.GetLayerDefn())
        feature.SetGeometry(ogr.CreateGeometryFromWkt(wkt))
        lines.CreateFeature(feature)
    source = None  # écrit et fermé
    layer = QgsVectorLayer(path + "|layername=cours_d_eau", "cours_d_eau", "ogr")
    project.addMapLayer(layer)
    kinds = sorted(QgsWkbTypes.displayString(f.geometry().wkbType()) for f in layer.getFeatures())
    print("       couche de lignes, entités :", kinds)
    return layer


def step_collection():
    layer = collection_layer()
    outcomes["collection_count"] = layer.featureCount()
    gesture(
        "analysis.buffer",
        {"layer_id": layer.id(), "distance": {"value": 10, "unit": "m"}, "out": out("5-collection.gpkg")},
        "collection",
        verdict,
    )


def radius_gaps(point_lonlat, buffer_2154):
    """Écarts au rayon de 500 m : distance ellipsoïdale du point à chaque sommet du contour."""
    to_wgs84 = QgsCoordinateTransform(QgsCoordinateReferenceSystem("EPSG:2154"),
                                      QgsCoordinateReferenceSystem("EPSG:4326"), project)
    measure = QgsDistanceArea()
    measure.setSourceCrs(QgsCoordinateReferenceSystem("EPSG:4326"), project.transformContext())
    measure.setEllipsoid("EPSG:7030")  # WGS84
    centre = QgsPointXY(*point_lonlat)
    return [abs(measure.measureLine(centre, to_wgs84.transform(QgsPointXY(v.x(), v.y()))) - 500.0)
            for v in buffer_2154.vertices()]


def verdict():
    result, error, _ = outcomes["geste"]
    check("geste : pas d'erreur", error is None, error and (error.code, error.message))
    check("geste : tampon en EPSG:2154", result and result["crs"] == "EPSG:2154", result)
    manual, error, _ = outcomes["metrique"]
    check("après data.reproject : pas d'erreur", error is None, error and error.code)
    if result and manual:
        check("même nombre d'entités", result["feature_count"] == manual["feature_count"] == len(POINTS),
              (result, manual))
        a = by_name(project.mapLayer(result["layer_id"]))
        b = by_name(project.mapLayer(manual["layer_id"]))
        for name, _, _ in POINTS:
            ga, gb = a.get(name), b.get(name)
            if ga is None or gb is None:
                check("entité %s présente des deux côtés" % name, False)
                continue
            area_gap = abs(ga.area() - gb.area())
            hausdorff = ga.hausdorffDistance(gb)
            print("       %s : surface %.1f m² contre %.1f m², écart %.3f m², Hausdorff %.3f m"
                  % (name, ga.area(), gb.area(), area_gap, hausdorff))
            check("%s : Hausdorff ≤ %.0f m" % (name, MAX_GAP_M), hausdorff <= MAX_GAP_M, hausdorff)
            # 5 m de plus sur le rayon d'un tampon de 500 m : 2πr·5 ≈ 15 700 m².
            check("%s : écart de surface sous celui de 5 m de rayon" % name,
                  area_gap <= 2 * 3.1416 * 500 * MAX_GAP_M, area_gap)
            lonlat = next((lon, lat) for n, lon, lat in POINTS if n == name)
            gaps = radius_gaps(lonlat, ga)
            print("       %s : écart au rayon de 500 m sur l'ellipsoïde, au plus %.2f m" % (name, max(gaps)))
            check("%s : contour à 500 m du point (ellipsoïde), à %.0f m près" % (name, MAX_RADIUS_GAP_M),
                  gaps and max(gaps) <= MAX_RADIUS_GAP_M, max(gaps) if gaps else None)
    native = outcomes.get("native")
    if manual and native is not None:
        layer = project.mapLayer(manual["layer_id"])
        check("couche métrique : même système que native:buffer seul",
              layer.crs().authid() == native.crs().authid() == "EPSG:2154",
              (layer.crs().authid(), native.crs().authid()))
        check("couche métrique : même nombre d'entités", layer.featureCount() == native.featureCount(),
              (layer.featureCount(), native.featureCount()))
        mine = by_name(layer)
        theirs = by_name(native)
        same = all(name in theirs and mine[name].hausdorffDistance(theirs[name]) < 1e-6 for name in mine)
        check("couche métrique : même géométrie", same and len(mine) == len(theirs))
    _, error, path = outcomes["refus"]
    check("work_epsg 4326 : analysis.crs_not_metric", error is not None and error.code == "analysis.crs_not_metric",
          error and error.code)
    check("work_epsg 4326 : aucun fichier", not os.path.exists(path), path)
    check("la couche d'origine reste en EPSG:4326", gares.crs().authid() == "EPSG:4326")
    check("la couche d'origine garde ses entités", gares.featureCount() == len(POINTS))
    result, error, _ = outcomes["collection"]
    check("collection de géométries : le tampon passe", error is None, error and (error.code, error.message))
    check("collection de géométries : une entité par entité d'origine",
          result and result["feature_count"] == outcomes["collection_count"] == 2, (result, outcomes["collection_count"]))
    check("collection de géométries : tampon en EPSG:2154", result and result["crs"] == "EPSG:2154", result)
    finish()


QTimer.singleShot(0, protect(step_buffer_degrees))
QTimer.singleShot(120000, QCoreApplication.quit)
QCoreApplication.exec()

done_checks = dict(checks)
missing = [name for name in EXPECTED if name not in done_checks]
for name in missing:
    print("ECHEC  contrôle non joué : " + name)
ok = not missing and all(v for _, v in checks)
project.clear()
shutil.rmtree(work, ignore_errors=True)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
