"""Sonde manuelle sous le vrai QGIS : les lecteurs de `project.inventory` (`QgisBackend`).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`),
plateforme offscreen :

    set QT_QPA_PLATFORM=offscreen
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_inventory.py

Dans un dossier temporaire, un GeoPackage synthétique : une couche multipolygone avec un
champ texte de 3 valeurs plus NULL et "", un champ texte de 15 valeurs, un champ entier et un
champ date. Vérifie `kind_of`, `wkb_name`, `text_values` (3 valeurs exactes ; 11 pour le
champ trop varié, que le noyau écartera ; rien pour l'entier et la date), `project_name`,
puis le geste entier. Aucune donnée hors dépôt n'est utilisée.
"""

import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))

from qgis.core import (  # noqa: E402
    QgsApplication,
    QgsCoordinateTransformContext,
    QgsFeature,
    QgsGeometry,
    QgsProject,
    QgsVectorFileWriter,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QDate  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
import inventory  # noqa: E402

FAILURES = []


def check(label, got, expected):
    ok = got == expected
    print("%s %s : %r" % ("OK  " if ok else "ÉCHEC", label, got))
    if not ok:
        FAILURES.append(label)
        print("     attendu : %r" % (expected,))


def build_gpkg(path):
    mem = QgsVectorLayer(
        "MultiPolygon?crs=EPSG:2154&field=zone:string&field=ref:string"
        "&field=n:integer&field=d:date",
        "zonage",
        "memory",
    )
    zones = ["U", "N", "Nh", None, ""]
    features = []
    for i in range(15):
        f = QgsFeature(mem.fields())
        x = float(i * 10)
        f.setGeometry(QgsGeometry.fromWkt(
            "MULTIPOLYGON(((%f 0,%f 0,%f 10,%f 10,%f 0)))" % (x, x + 5, x + 5, x, x)
        ))
        f.setAttributes([zones[i % len(zones)], "R%02d" % i, i, QDate(2024, 1, 1 + i)])
        features.append(f)
    assert mem.dataProvider().addFeatures(features)
    options = QgsVectorFileWriter.SaveVectorOptions()
    options.driverName = "GPKG"
    options.layerName = "zonage"
    error = QgsVectorFileWriter.writeAsVectorFormatV3(
        mem, path, QgsCoordinateTransformContext(), options
    )
    assert error[0] == QgsVectorFileWriter.NoError, error


def main():
    work = tempfile.mkdtemp(prefix="cassini-sonde-inventaire-")
    try:
        gpkg = os.path.join(work, "zonage.gpkg")
        build_gpkg(gpkg)
        layer = QgsVectorLayer(gpkg + "|layername=zonage", "zonage", "ogr")
        assert layer.isValid(), "GeoPackage illisible"
        project = QgsProject.instance()
        project.addMapLayer(layer)
        assert project.write(os.path.join(work, "sonde-inventaire.qgz"))

        backend = inventory.QgisBackend()
        limit = inventory.VALUES_LIMIT
        check("kind_of", backend.kind_of(layer), "vector")
        check("wkb_name", backend.wkb_name(layer), "MultiPolygon")
        check("project_name", backend.project_name(), "sonde-inventaire")
        check("text_values zone", backend.text_values(layer, "zone", limit), ["N", "Nh", "U"])
        ref = backend.text_values(layer, "ref", limit)
        check("text_values ref (nombre)", len(ref or []), limit)
        check("text_values n", backend.text_values(layer, "n", limit), None)
        check("text_values d", backend.text_values(layer, "d", limit), None)

        calls = []
        gesture = inventory.ProjectInventoryGesture(None, lambda fn: fn())
        gesture.start({}, lambda result=None, error=None: calls.append((result, error)))
        result, error = calls[0]
        check("geste sans erreur", error, None)
        fields = {f["name"]: f for f in result["layers"][0]["fields"]}
        check("geste zone", fields["zone"].get("values"), ["N", "Nh", "U"])
        check("geste n sans valeurs", "values" in fields["n"], False)
        check("geste nombre d'objets", result["layers"][0]["feature_count"], 15)
        project.clear()
    finally:
        shutil.rmtree(work, ignore_errors=True)


try:
    main()
finally:
    QgsApplication.exitQgis()
print("SONDE %s" % ("ÉCHOUÉE : " + ", ".join(FAILURES) if FAILURES else "RÉUSSIE"))
sys.exit(1 if FAILURES else 0)
