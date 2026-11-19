r"""Sonde manuelle sous le vrai QGIS : `analysis.isochrone` sur un réseau en degrés (entrée 10.32).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_isochrone_degrees.py

Le réseau de référence est `fixtures/reseau-grille-2154.geojson` (rues tous les 100 m, 1 km de
côté, origine au centre). On en fait à la main une copie en EPSG:4326 (chaque géométrie transformée
par QGIS), puis on compare :

* l'isochrone sur la copie en degrés, avec `work_epsg` = 2154 comme le noyau le remplit, à
  celle du réseau métrique d'origine : même emprise à quelques mètres près ;
* le même calcul sans `work_epsg` : refus `analysis.crs_not_metric` (rien n'est lancé) ;
* l'accrochage de l'origine se vérifie en mètres : à 200 m du réseau, refusé à 100 m de tolérance,
  accepté à 250 m (un rayon en mètres pris pour des degrés accepterait tout) ;
* l'origine donnée en WGS84 est transformée vers `work_epsg` ;
* un `work_epsg` qui n'est pas en mètres est refusé avant tout lancement.
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
    QgsPointXY,
    QgsProject,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QCoreApplication, QTimer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402

grid = QgsVectorLayer(GRID, "reseau", "ogr")
assert grid.isValid(), "grille illisible"
QgsProject.instance().addMapLayer(grid)

lambert = QgsCoordinateReferenceSystem("EPSG:2154")
wgs84 = QgsCoordinateReferenceSystem("EPSG:4326")
to_wgs = QgsCoordinateTransform(lambert, wgs84, QgsProject.instance())

# Copie à la main du réseau en degrés, champs compris.
degrees = QgsVectorLayer("LineString?crs=EPSG:4326", "reseau-degres", "memory")
degrees.dataProvider().addAttributes(grid.fields().toList())
degrees.updateFields()
copies = []
for source in grid.getFeatures():
    feature = QgsFeature(degrees.fields())
    feature.setAttributes(source.attributes())
    geometry = source.geometry()
    geometry.transform(to_wgs)
    feature.setGeometry(geometry)
    copies.append(feature)
degrees.dataProvider().addFeatures(copies)
degrees.updateExtents()
QgsProject.instance().addMapLayer(degrees)
assert degrees.featureCount() == grid.featureCount() > 0

CX, CY = 652500.0, 6850500.0
TOL = 5.0  # mètres : écart toléré entre l'emprise en degrés et la référence métrique
centre_wgs = to_wgs.transform(QgsPointXY(CX, CY))


def network(mode, **extra):
    """Contrat de réseau tel que le noyau le résout : l'extension n'a aucune valeur par défaut."""
    n = {
        "speed": {"value": {"walk": 5.0, "car": 50.0}[mode], "unit": "km/h"},
        "tolerance": {"value": 100.0, "unit": "m"},
    }
    n.update(extra)
    return n


def base(layer, **extra):
    p = {
        "layer_id": layer.id(),
        "origin": {"x": CX, "y": CY, "epsg": 2154},
        "mode": "walk",
        "duration": {"value": 3, "unit": "min"},
    }
    p.update(extra)
    p.setdefault("network", network(p["mode"]))
    return p


def extent_of(result):
    """Emprise du résultat, en EPSG:2154 (le résultat est dans `work_epsg`)."""
    layer = QgsProject.instance().mapLayer(result["layer_id"])
    box = layer.extent()
    return box.xMinimum(), box.yMinimum(), box.xMaximum(), box.yMaximum()


state = {"reference": None}


def check_reference(result, error):
    if error is not None or result["feature_count"] != 1 or result["crs"] != "EPSG:2154":
        return False
    state["reference"] = extent_of(result)
    # Référence calculée à la main : 250 m à pied en 3 min.
    want = (CX - 250.0, CY - 250.0, CX + 250.0, CY + 250.0)
    return all(abs(a - b) <= TOL for a, b in zip(state["reference"], want))


def same_as_reference(result, error):
    if error is not None or result["feature_count"] != 1 or result["crs"] != "EPSG:2154":
        return False
    box = extent_of(result)
    print("  emprise en degrés -> 2154 :", ["%.2f" % v for v in box], "écart max (m) : %.3f" % max(
        abs(a - b) for a, b in zip(box, state["reference"])
    ))
    return state["reference"] is not None and all(abs(a - b) <= TOL for a, b in zip(box, state["reference"]))


def ok_only(result, error):
    return error is None and result["feature_count"] == 1


def code(expected):
    return lambda result, error: error is not None and error.code == expected


far = {"x": CX, "y": CY + 500.0 + 200.0, "epsg": 2154}  # 200 m au nord du bord du réseau
steps = [
    ("référence : réseau métrique", base(grid), check_reference),
    ("réseau en degrés, work_epsg 2154", base(degrees, work_epsg=2154), same_as_reference),
    (
        "réseau en degrés, origine en WGS84",
        base(degrees, work_epsg=2154, origin={"x": centre_wgs.x(), "y": centre_wgs.y(), "epsg": 4326}),
        same_as_reference,
    ),
    ("réseau en degrés, voiture", base(degrees, work_epsg=2154, mode="car"), ok_only),
    ("réseau en degrés sans work_epsg", base(degrees), code("analysis.crs_not_metric")),
    ("work_epsg en degrés (4326)", base(degrees, work_epsg=4326), code("analysis.crs_not_metric")),
    ("work_epsg inconnu", base(degrees, work_epsg=999999), code("data.unknown_crs")),
    # Accrochage en mètres : 200 m hors réseau, refusé à 100 m, accepté à 250 m.
    ("200 m du réseau, tolérance 100 m", base(degrees, work_epsg=2154, origin=far), code("analysis.origin_off_network")),
    (
        "200 m du réseau, tolérance 250 m",
        base(degrees, work_epsg=2154, origin=far, network=network("walk", tolerance={"value": 250.0, "unit": "m"})),
        ok_only,
    ),
    (
        "origine à 5 km",
        base(degrees, work_epsg=2154, origin={"x": CX + 5000.0, "y": CY + 5000.0, "epsg": 2154}),
        code("analysis.origin_off_network"),
    ),
]
results = []
state["i"] = 0
out_dir = tempfile.mkdtemp()


def next_step():
    i = state["i"]
    if i >= len(steps):
        QCoreApplication.quit()
        return
    label, params, check = steps[i]
    out = os.path.join(out_dir, "%d-analysis.isochrone.gpkg" % (i + 1))
    print("étape %d : %s" % (i + 1, label))

    def done(result=None, error=None):
        ok = bool(check(result, error))
        detail = error.code if error is not None else (result["feature_count"], result["crs"])
        # Un refus ne laisse aucun fichier ; un succès n'en laisse qu'un, jamais d'annexe.
        mine = [f for f in sorted(os.listdir(out_dir)) if f.startswith("%d-" % (i + 1))]
        ok = ok and len(mine) == (1 if error is None else 0)
        results.append((label, detail, ok))
        state["i"] += 1
        QTimer.singleShot(0, guarded)

    state["gesture"] = analysis.ProcessingGesture(analysis.ALGORITHMS["analysis.isochrone"])
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

for row in results:
    print(row)
ok = len(results) == len(steps) and all(r[2] for r in results)
print("VERDICT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
