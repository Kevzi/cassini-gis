r"""Sonde manuelle sous le vrai QGIS : `network.prepare`.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_prepare.py

Constats de la sonde sous QGIS 3.44 (consignés ici, assertés plus bas) :

- `native:snapgeometries` a pour paramètres `INPUT`, `REFERENCE_LAYER`, `TOLERANCE`, `BEHAVIOR`, `OUTPUT`.
  `BEHAVIOR` 4 = « Move end points only, prefer aligning nodes » : la couche s'accroche contre elle-même, seules
  les extrémités bougent. `TOLERANCE` est dans l'unité de la couche (donc des mètres après reprojection).
- `native:splitwithlines` a pour paramètres `INPUT`, `LINES`, `OUTPUT` ; la couche découpée par elle-même reçoit un
  nœud à chaque croisement et à chaque jonction en T, sans casser les tronçons déjà bons.
- `native:removenullgeometries` : `INPUT`, `REMOVE_EMPTY`, `OUTPUT`, et `NULL_OUTPUT` (puits facultatif, non demandé).
- `native:deleteduplicategeometries` : `INPUT`, `OUTPUT`, sorties `RETAINED_COUNT` et `DUPLICATE_COUNT` ; l'égalité
  est topologique : un tronçon à l'envers est un doublon.
- Aucun algorithme GRASS ni `qgis:` dans la chaîne.

Réseau brut synthétique (Lambert-93) : H1 de (X, Y) à (X+100, Y), H2 de (X+100,5, Y) à (X+200, Y) (extrémités à
0,5 m l'une de l'autre), V de (X+150, Y-50) à (X+150, Y+50) qui croise H2 sans nœud, et H1 une seconde fois, à
l'envers. Attendu à la main : cinq tronçons après préparation (H1, H2 en deux, V en deux) ; de (X, Y) à
(X+150, Y+50), 200 m, soit 144 s à pied ; l'isochrone de 10 min à pied couvre l'enveloppe convexe
(0,0)-(150,-50)-(200,0)-(150,50), soit 10 000 m². Avant préparation : aucun chemin, et l'enveloppe de la seule H1
est une ligne, de surface nulle.
"""

import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import (  # noqa: E402
    QgsApplication,
    QgsCoordinateReferenceSystem,
    QgsCoordinateTransform,
    QgsFeature,
    QgsGeometry,
    QgsPointXY,
    QgsProject,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

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
registry = QgsApplication.processingRegistry()
NEEDED = {
    "native:snapgeometries": ({"INPUT", "REFERENCE_LAYER", "TOLERANCE", "BEHAVIOR", "OUTPUT"}, {"OUTPUT"}),
    "native:splitwithlines": ({"INPUT", "LINES", "OUTPUT"}, {"OUTPUT"}),
    "native:removenullgeometries": ({"INPUT", "REMOVE_EMPTY", "OUTPUT", "NULL_OUTPUT"}, {"OUTPUT", "NULL_OUTPUT"}),
    "native:deleteduplicategeometries": ({"INPUT", "OUTPUT"}, {"OUTPUT", "RETAINED_COUNT", "DUPLICATE_COUNT"}),
    "native:reprojectlayer": ({"INPUT", "TARGET_CRS", "OUTPUT"}, {"OUTPUT"}),
}
for algorithm_id, (parameters, outputs) in NEEDED.items():
    algorithm = registry.algorithmById(algorithm_id)
    expect("algorithme %s présent" % algorithm_id, algorithm is not None)
    if algorithm is None:
        continue
    names = {p.name() for p in algorithm.parameterDefinitions()}
    expect("paramètres de %s" % algorithm_id, parameters <= names)
    expect("sorties de %s" % algorithm_id, outputs <= {o.name() for o in algorithm.outputDefinitions()})
    expect("%s annulable pour l'extension" % algorithm_id, algorithm_id in analysis.CANCELLABLE_ALGORITHMS)
snap = registry.algorithmById("native:snapgeometries")
behaviors = [p for p in snap.parameterDefinitions() if p.name() == "BEHAVIOR"][0].options()
expect("BEHAVIOR 4 : extrémités seules, nœuds d'abord", behaviors[4] == "Move end points only, prefer aligning nodes")
expect(
    "aucun algorithme hors native: dans la chaîne",
    all(a.startswith("native:") for a in analysis.required_algorithms(analysis.ALGORITHMS["network.prepare"])),
)

X, Y = 652000.0, 6850000.0
H1 = ((X, Y), (X + 100.0, Y))
H2 = ((X + 100.5, Y), (X + 200.0, Y))
V = ((X + 150.0, Y - 50.0), (X + 150.0, Y + 50.0))
H2_WIDE = ((X + 103.0, Y), (X + 200.0, Y))  # écart de 3 m


def memory_lines(name, crs, segments):
    layer = QgsVectorLayer("LineString?crs=%s" % crs, name, "memory")
    features = []
    for a, b in segments:
        f = QgsFeature(layer.fields())
        f.setGeometry(QgsGeometry.fromPolylineXY([QgsPointXY(*a), QgsPointXY(*b)]))
        features.append(f)
    layer.dataProvider().addFeatures(features)
    QgsProject.instance().addMapLayer(layer)
    return layer


raw = memory_lines("chemins-bruts", "EPSG:2154", [H1, H2, V, (H1[1], H1[0])])
wide = memory_lines("chemins-ecart-3m", "EPSG:2154", [H1, H2_WIDE, V])
points = QgsVectorLayer("Point?crs=EPSG:2154", "points", "memory")
polygons = QgsVectorLayer("Polygon?crs=EPSG:2154", "polygones", "memory")
QgsProject.instance().addMapLayer(points)
QgsProject.instance().addMapLayer(polygons)

to_wgs = QgsCoordinateTransform(
    QgsCoordinateReferenceSystem("EPSG:2154"), QgsCoordinateReferenceSystem("EPSG:4326"), QgsProject.instance()
)
degrees = QgsVectorLayer("LineString?crs=EPSG:4326", "chemins-degres", "memory")
degree_features = []
for source in raw.getFeatures():
    g = QgsGeometry(source.geometry())
    g.transform(to_wgs)
    f = QgsFeature(degrees.fields())
    f.setGeometry(g)
    degree_features.append(f)
degrees.dataProvider().addFeatures(degree_features)
QgsProject.instance().addMapLayer(degrees)

raw_before = sorted(f.geometry().asWkt(3) for f in raw.getFeatures())
START, END = (X, Y), (X + 150.0, Y + 50.0)
TOL = 0.5
NETWORK = {"speed": {"value": 5.0, "unit": "km/h"}, "tolerance": {"value": 100.0, "unit": "m"}}
ONE_M = {"value": 1.0, "unit": "m"}


def pt(xy, epsg=2154):
    return {"x": xy[0], "y": xy[1], "epsg": epsg}


def layer_of(result):
    return QgsProject.instance().mapLayer(result["layer_id"])


def code(expected):
    return lambda result, error: error is not None and error.code == expected


def prepared(result, error, count=5, crs="EPSG:2154"):
    return error is None and result["feature_count"] == count and result["crs"] == crs and result["name"]


def path_200(result, error):
    return error is None and abs(result["length"]["value"] - 200.0) <= TOL and abs(result["duration"]["value"] - 144.0) <= 0.5


def area_zero(result, error):
    # Constat : sur le réseau brut, QGIS atteint la seule H1 ; son enveloppe convexe est une ligne (surface nulle),
    # ou l'origine est refusée (`analysis.origin_off_network`) : dans les deux cas, pas d'isochrone.
    if error is not None:
        return error.code == "analysis.origin_off_network"
    return sum(f.geometry().area() for f in layer_of(result).getFeatures()) < 1.0


def area_10000(result, error):
    if error is not None:
        return False
    area = sum(f.geometry().area() for f in layer_of(result).getFeatures())
    return abs(area - 10000.0) <= 5.0


# Chaque étape : (libellé, commande, paramètres [ou fonction des résultats précédents], contrôle, fichiers attendus).
steps = []


def step(label, command, params, check, files=None):
    steps.append((label, command, params, check, files))


def path_params(layer_key, **extra):
    return lambda r: dict(
        {"layer_id": r[layer_key]["layer_id"] if layer_key in r else layer_key, "start": pt(START), "end": pt(END),
         "mode": "walk", "network": NETWORK},
        **extra
    )


def isochrone_params(layer_key):
    return lambda r: {
        "layer_id": r[layer_key]["layer_id"] if layer_key in r else layer_key,
        "origin": pt(START), "mode": "walk", "duration": {"value": 10, "unit": "min"}, "network": NETWORK,
    }


# 1. Témoin : avant préparation, ni chemin ni isochrone.
step("témoin : plus court chemin brut", "network.shortest_path", path_params(raw.id()), code("analysis.no_path"))
step("témoin : isochrone brute", "analysis.isochrone", isochrone_params(raw.id()), area_zero)
# 2. Nominal.
step("préparation nominale (1 m)", "network.prepare", {"layer_id": raw.id(), "tolerance": ONE_M}, prepared)
step("plus court chemin préparé", "network.shortest_path", path_params("préparation nominale (1 m)"), path_200)
step("isochrone préparée", "analysis.isochrone", isochrone_params("préparation nominale (1 m)"), area_10000)
# 3. Tolérance : trop petite, puis suffisante (écart de 3 m).
step(
    "tolérance trop petite : couche rendue sans relier l'écart",
    "network.prepare",
    {"layer_id": wide.id(), "tolerance": ONE_M},
    lambda r, e: e is None and r["feature_count"] == 5,  # H1, H2 et V découpés au croisement, mais non reliés
)
step("écart de 3 m : toujours aucun chemin", "network.shortest_path", path_params("tolérance trop petite : couche rendue sans relier l'écart"), code("analysis.no_path"))
step(
    "tolérance 5 m",
    "network.prepare",
    {"layer_id": wide.id(), "tolerance": {"value": 5.0, "unit": "m"}},
    lambda r, e: e is None and r["feature_count"] == 5,
)
step("écart de 3 m relié par 5 m", "network.shortest_path", path_params("tolérance 5 m"), lambda r, e: e is None and abs(r["length"]["value"] - 200.0) <= 3.5)
# 4. Couche en degrés : calcul dans work_epsg, résultat dans le système de la couche.
step("couche en degrés", "network.prepare", {"layer_id": degrees.id(), "tolerance": ONE_M, "work_epsg": 2154},
     lambda r, e: prepared(r, e, crs="EPSG:4326"))
step(
    "plus court chemin sur le réseau préparé en degrés",
    "network.shortest_path",
    lambda r: {"layer_id": r["couche en degrés"]["layer_id"], "start": pt(START), "end": pt(END), "mode": "walk",
               "network": NETWORK, "work_epsg": 2154},
    path_200,
)
step("degrés sans work_epsg", "network.prepare", {"layer_id": degrees.id(), "tolerance": ONE_M}, code("analysis.crs_not_metric"), files=0)
# 5. Refus avant lancement, aucun fichier.
step("polygones", "network.prepare", {"layer_id": polygons.id(), "tolerance": ONE_M}, code("analysis.bad_params"), files=0)
step("points", "network.prepare", {"layer_id": points.id(), "tolerance": ONE_M}, code("analysis.bad_params"), files=0)
step("couche absente", "network.prepare", {"layer_id": "nope", "tolerance": ONE_M}, code("layer.not_found"), files=0)
step("sans tolérance (le noyau l'écrit)", "network.prepare", {"layer_id": raw.id()}, code("analysis.bad_params"), files=0)
# 6. Échec en dernière étape : aucun fichier.
step("échec : dossier de sortie absent", "network.prepare", {"layer_id": raw.id(), "tolerance": ONE_M, "out": "ABSENT"}, code("analysis.failed"), files=0)

results = {}
rows = []
state = {"i": 0}
out_dir = tempfile.mkdtemp()


def next_step():
    i = state["i"]
    if i >= len(steps):
        QCoreApplication.quit()
        return
    label, command, params, check, files = steps[i]
    p = params(results) if callable(params) else dict(params)
    if p.get("out") == "ABSENT":
        out = os.path.join(out_dir, "dossier-absent", "%d-%s.gpkg" % (i + 1, command))
    else:
        out = os.path.join(out_dir, "%d-%s.gpkg" % (i + 1, command))
    p["out"] = out

    def done(result=None, error=None):
        ok = bool(check(result, error))
        if error is None:
            results[label] = result
        detail = error.code if error is not None else {k: result[k] for k in result if k in ("feature_count", "crs", "length", "duration")}
        mine = [f for f in sorted(os.listdir(out_dir)) if f.startswith("%d-" % (i + 1))]
        expected = (1 if error is None else 0) if files is None else files
        ok = ok and len(mine) == expected  # un échec ne laisse aucun fichier, un succès un seul (jamais d'annexe)
        rows.append((label, detail, ok))
        state["i"] += 1
        QTimer.singleShot(0, guarded)

    state["gesture"] = analysis.ProcessingGesture(analysis.ALGORITHMS[command])
    state["gesture"].start(p, done)


def guarded():
    import traceback

    try:
        next_step()
    except Exception:
        traceback.print_exc()
        sys.stdout.flush()
        QCoreApplication.quit()


QTimer.singleShot(0, guarded)
QTimer.singleShot(180000, QCoreApplication.quit)
QCoreApplication.exec()

# L'original n'est jamais touché.
expect("la couche brute est intacte", sorted(f.geometry().asWkt(3) for f in raw.getFeatures()) == raw_before)
for row in rows:
    print(row)
ok = len(rows) == len(steps) and all(r[2] for r in rows) and not failures
print("VERDICT", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
