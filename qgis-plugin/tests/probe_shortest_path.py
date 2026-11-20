r"""Sonde manuelle sous le vrai QGIS : `network.shortest_path`.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_shortest_path.py

Constats de la sonde sous QGIS 3.44 (consignés ici, assertés plus bas) :

- `native:shortestpathpointtopoint` a pour paramètres `INPUT`, `STRATEGY`, `DIRECTION_FIELD`,
  `VALUE_FORWARD`, `VALUE_BACKWARD`, `VALUE_BOTH`, `DEFAULT_DIRECTION`, `SPEED_FIELD`, `DEFAULT_SPEED`,
  `TOLERANCE`, `START_POINT`, `END_POINT`, `OUTPUT` ; et pour sorties `OUTPUT` et `TRAVEL_COST`.
- Stratégie 1 (« plus rapide ») : `TRAVEL_COST` est en HEURES (600 m à 5 km/h = 0,12), et la couche porte
  un champ `cost` de même valeur ; la longueur ne vient jamais d'un coût mais de la géométrie.
- Deux points sans chemin (composantes non connectées, sens interdit, ou points confondus) : l'algorithme
  échoue (« There is no route from start point to end point »), il ne rend jamais une couche vide.
- Un point loin du réseau est accroché au nœud le plus proche sans tolérance : c'est la garde d'accrochage
  de l'extension, avant lancement, qui le refuse.

Sur `fixtures/reseau-grille-2154.geojson` (rues tous les 100 m, 1 km de côté) : de (652100, 6850100) à
(652400, 6850400), 600 m en Manhattan, soit 432 s à pied (5 km/h) et 43,2 s en voiture (50 km/h). Les
valeurs attendues sont calculées ici à la main, sans passer par QGIS. Puis : sens unique (contournement
de 300 m), composantes non connectées, départ ou arrivée hors réseau, points confondus, réseau en degrés.
"""

import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
GRID = os.path.join(ROOT, "fixtures", "reseau-grille-2154.geojson")

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import (  # noqa: E402
    QgsApplication,
    QgsCoordinateReferenceSystem,
    QgsCoordinateTransform,
    QgsFeature,
    QgsField,
    QgsGeometry,
    QgsPointXY,
    QgsProject,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QCoreApplication, QTimer, QVariant  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402

failures = []


def expect(label, condition):
    if not condition:
        failures.append(label)
        print("ECHEC :", label)


# -- noms de paramètres et de sorties : constatés, jamais supposés ---------------------------------------
algorithm = QgsApplication.processingRegistry().algorithmById("native:shortestpathpointtopoint")
expect("algorithme présent", algorithm is not None)
names = {p.name() for p in algorithm.parameterDefinitions()}
for needed in ("INPUT", "STRATEGY", "DIRECTION_FIELD", "VALUE_FORWARD", "VALUE_BACKWARD", "DEFAULT_DIRECTION",
               "SPEED_FIELD", "DEFAULT_SPEED", "TOLERANCE", "START_POINT", "END_POINT", "OUTPUT"):
    expect("paramètre %s" % needed, needed in names)
expect("sorties OUTPUT et TRAVEL_COST", {o.name() for o in algorithm.outputDefinitions()} == {"OUTPUT", "TRAVEL_COST"})
expect("l'extension annonce l'algorithme annulable", "native:shortestpathpointtopoint" in analysis.CANCELLABLE_ALGORITHMS)

grid = QgsVectorLayer(GRID, "reseau", "ogr")
assert grid.isValid(), "grille illisible"
QgsProject.instance().addMapLayer(grid)


def memory_network(name, crs, segments, sens=None):
    """Réseau synthétique en mémoire : `segments` = [((x1, y1), (x2, y2))], `sens` = valeur du champ par segment."""
    layer = QgsVectorLayer("LineString?crs=%s" % crs, name, "memory")
    layer.dataProvider().addAttributes([QgsField("sens", QVariant.String)])
    layer.updateFields()
    features = []
    for i, (a, b) in enumerate(segments):
        f = QgsFeature(layer.fields())
        f.setGeometry(QgsGeometry.fromPolylineXY([QgsPointXY(*a), QgsPointXY(*b)]))
        f["sens"] = sens[i] if sens else "FT"
        features.append(f)
    layer.dataProvider().addFeatures(features)
    QgsProject.instance().addMapLayer(layer)
    return layer


# Boucle de 100 m de côté : AB est interdit dans le sens A -> B (valeur « TF » = sens inverse seul), les trois
# autres côtés sont à double sens (toute autre valeur).
O = (652000.0, 6850000.0)
A, B, C, D = O, (O[0] + 100, O[1]), (O[0] + 100, O[1] + 100), (O[0], O[1] + 100)
oneway = memory_network("boucle", "EPSG:2154", [(A, B), (B, C), (C, D), (D, A)], sens=["TF", "2", "2", "2"])
# Un seul sens interdit : de A vers B, aucun chemin.
dead_end = memory_network("sens-unique", "EPSG:2154", [(A, B)], sens=["TF"])
# Deux composantes sans lien : AB et CD (loin l'un de l'autre).
E, F = (O[0] + 5000, O[1]), (O[0] + 5100, O[1])
split = memory_network("coupe", "EPSG:2154", [(A, B), (E, F)])

# La grille en degrés (même géométrie, déclarée en WGS84) : le noyau choisit EPSG:2154 comme `work_epsg`.
to_wgs = QgsCoordinateTransform(
    QgsCoordinateReferenceSystem("EPSG:2154"), QgsCoordinateReferenceSystem("EPSG:4326"), QgsProject.instance()
)
degrees = QgsVectorLayer("LineString?crs=EPSG:4326", "reseau-degres", "memory")
degree_features = []
for source in grid.getFeatures():
    g = QgsGeometry(source.geometry())
    g.transform(to_wgs)
    f = QgsFeature(degrees.fields())
    f.setGeometry(g)
    degree_features.append(f)
degrees.dataProvider().addFeatures(degree_features)
QgsProject.instance().addMapLayer(degrees)
points = QgsVectorLayer("Point?crs=EPSG:2154", "points", "memory")
QgsProject.instance().addMapLayer(points)

GX, GY = 652000.0, 6850000.0
START, END = (GX + 100.0, GY + 100.0), (GX + 400.0, GY + 400.0)
TOL = 0.5


def network(mode, **extra):
    """Contrat de réseau tel que le noyau le résout : l'extension n'a aucune valeur par défaut."""
    n = {
        "speed": {"value": {"walk": 5.0, "car": 50.0}[mode], "unit": "km/h"},
        "tolerance": {"value": 100.0, "unit": "m"},
    }
    n.update(extra)
    return n


def pt(xy, epsg=2154):
    return {"x": xy[0], "y": xy[1], "epsg": epsg}


def base(**extra):
    p = {"layer_id": grid.id(), "start": pt(START), "end": pt(END), "mode": "walk"}
    p.update(extra)
    p.setdefault("network", network(p["mode"]))
    return p


def layer_of(result):
    return QgsProject.instance().mapLayer(result["layer_id"])


def path_geometry(result):
    features = list(layer_of(result).getFeatures())
    return features[0].geometry() if len(features) == 1 else None


def check_nominal(result, error):
    if error is not None or result["feature_count"] != 1:
        return False
    geometry = path_geometry(result)
    box = geometry.boundingBox()
    vertices = geometry.asPolyline() if not geometry.isMultipart() else geometry.asMultiPolyline()[0]
    first, last = vertices[0], vertices[-1]
    return (
        result["length"] == {"value": result["length"]["value"], "unit": "m"}
        and abs(result["length"]["value"] - 600.0) <= TOL
        and result["duration"]["unit"] == "s"
        and abs(result["duration"]["value"] - 432.0) <= TOL / 5000.0 * 3600.0
        and abs(first.x() - START[0]) <= TOL and abs(first.y() - START[1]) <= TOL
        and abs(last.x() - END[0]) <= TOL and abs(last.y() - END[1]) <= TOL
        and box.xMinimum() >= START[0] - TOL and box.xMaximum() <= END[0] + TOL
        and box.yMinimum() >= START[1] - TOL and box.yMaximum() <= END[1] + TOL
        and result["crs"] == "EPSG:2154"
    )


def check_car(result, error):
    return error is None and abs(result["length"]["value"] - 600.0) <= TOL and abs(result["duration"]["value"] - 43.2) <= 0.05


def check_speed(result, error):
    # 10 km/h : 600 m en 216 s.
    return error is None and abs(result["duration"]["value"] - 216.0) <= 0.5


def check_oneway(result, error):
    # A -> B direct est interdit : A, D, C, B, soit 300 m ; B -> A direct reste permis.
    return error is None and abs(result["length"]["value"] - 300.0) <= TOL


def check_oneway_back(result, error):
    return error is None and abs(result["length"]["value"] - 100.0) <= TOL


def check_degrees(result, error):
    # Calcul dans work_epsg : couche rendue en EPSG:2154, longueur en mètres (pas en degrés).
    return error is None and result["crs"] == "EPSG:2154" and abs(result["length"]["value"] - 600.0) <= 1.0 and abs(result["duration"]["value"] - 432.0) <= 1.0


def code(expected, mention=None):
    def check(result, error):
        return error is not None and error.code == expected and (mention is None or mention in error.message)

    return check


wgs_start = to_wgs.transform(QgsPointXY(*START))
wgs_end = to_wgs.transform(QgsPointXY(*END))
far = (GX + 5000.0, GY + 5000.0)

steps = [
    ("à pied", base(), check_nominal),
    ("en voiture", base(mode="car"), check_car),
    ("vitesse 10 km/h", base(network=network("walk", speed={"value": 10.0, "unit": "km/h"})), check_speed),
    (
        "sens unique : contournement",
        base(layer_id=oneway.id(), start=pt(A), end=pt(B), network=network("walk", direction={"field": "sens", "forward": "FT", "backward": "TF"})),
        check_oneway,
    ),
    (
        "sens unique : trajet inverse permis",
        base(layer_id=oneway.id(), start=pt(B), end=pt(A), network=network("walk", direction={"field": "sens", "forward": "FT", "backward": "TF"})),
        check_oneway_back,
    ),
    (
        "sens unique : aucun chemin",
        base(layer_id=dead_end.id(), start=pt(A), end=pt(B), network=network("walk", direction={"field": "sens", "forward": "FT", "backward": "TF"})),
        code("analysis.no_path"),
    ),
    ("composantes non connectées", base(layer_id=split.id(), start=pt(A), end=pt(E)), code("analysis.no_path")),
    ("points confondus", base(end=pt(START)), code("analysis.no_path")),
    ("départ hors réseau", base(start=pt(far)), code("analysis.origin_off_network", "départ")),
    ("arrivée hors réseau", base(end=pt(far)), code("analysis.origin_off_network", "arrivée")),
    (
        "tolérance 250 m (arrivée à 200 m du bord)",
        base(end=pt((GX + 1000.0 + 200.0, GY + 400.0)), network=network("walk", tolerance={"value": 250.0, "unit": "m"})),
        lambda r, e: e is None and r["feature_count"] == 1,
    ),
    ("200 m du réseau, tolérance 100 m", base(end=pt((GX + 1000.0 + 200.0, GY + 400.0))), code("analysis.origin_off_network", "arrivée")),
    (
        "réseau en degrés, points en WGS84",
        base(layer_id=degrees.id(), work_epsg=2154, start=pt((wgs_start.x(), wgs_start.y()), 4326), end=pt((wgs_end.x(), wgs_end.y()), 4326)),
        check_degrees,
    ),
    ("réseau en degrés sans work_epsg", base(layer_id=degrees.id()), code("analysis.crs_not_metric")),
    ("couche de points", base(layer_id=points.id()), code("analysis.bad_params")),
    ("réseau absent", base(layer_id="nope"), code("layer.not_found")),
]
results = []
state = {"i": 0}
out_dir = tempfile.mkdtemp()


def next_step():
    i = state["i"]
    if i >= len(steps):
        QCoreApplication.quit()
        return
    label, params, check = steps[i]
    out = os.path.join(out_dir, "%d-network.shortest_path.gpkg" % (i + 1))

    def done(result=None, error=None):
        ok = bool(check(result, error))
        detail = error.code if error is not None else (result["feature_count"], result["length"], result["duration"])
        # Un refus ne laisse aucun fichier ; un succès n'en laisse qu'un, jamais d'annexe.
        mine = [f for f in sorted(os.listdir(out_dir)) if f.startswith("%d-" % (i + 1))]
        expected = 1 if error is None else 0
        ok = ok and len(mine) == expected
        results.append((label, detail, ok))
        state["i"] += 1
        QTimer.singleShot(0, guarded)

    state["gesture"] = analysis.ProcessingGesture(analysis.ALGORITHMS["network.shortest_path"])
    state["gesture"].start(dict(params, out=out), done)


def guarded():
    import traceback

    try:
        next_step()
    except Exception:
        traceback.print_exc()
        sys.stdout.flush()
        QCoreApplication.quit()


QTimer.singleShot(0, guarded)
QTimer.singleShot(120000, QCoreApplication.quit)
QCoreApplication.exec()

print("attendu à la main : 600 m, 432 s à pied, 43,2 s en voiture, contournement de 300 m")
for row in results:
    print(row)
ok = len(results) == len(steps) and all(r[2] for r in results) and not failures
print("VERDICT", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
