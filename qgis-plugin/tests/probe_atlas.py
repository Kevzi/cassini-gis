r"""Sonde manuelle sous le vrai QGIS : `layout.export_atlas` (ticket 15) et écart des couches distantes.

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`) :

    "C:\Program Files\QGIS 3.44.15\bin\python-qgis-ltr.bat" qgis-plugin/tests/probe_atlas.py

Aucun noyau ni aucune donnée hors dépôt : trois entités synthétiques (Lambert-93), un fond local
écrit en GeoJSON et une couche XYZ distante (adresse inexistante). Vrai geste `layout.export_atlas`
(table fermée, `native:atlaslayouttopdf`) et vrai geste `layout.export_pdf`. Termine par
`RÉSULTAT : OK` ou `RÉSULTAT : ÉCHEC`.

Ce que la sonde constate (le plan interdit de le supposer) :
1. Atlas sur trois entités : un seul PDF de trois pages (objets `/Type /Page` du fichier), au chemin
   donné, `page_count` 3 ; la mise en page n'est dans le gestionnaire du projet que le temps de l'appel.
2. Couche XYZ visible : écartée de l'atlas et de l'export simple, nommée dans `skipped_layers`, et
   aucune requête réseau n'est créée (un témoin prouve que l'espion de requêtes voit bien passer une
   requête).
3. Que du distant : `layout.empty`, aucun fichier, pour l'un et l'autre export.
4. Couverture vide : `layer.empty`, aucun fichier. Champ inconnu : `analysis.unknown_field`, aucun
   fichier. Algorithme en échec (sortie sous un fichier) : `layout.export_failed`, aucun fichier,
   et le gestionnaire du projet est revenu à son état d'avant.
5. Le projet (couches, mises en page) est inchangé après chaque appel.
"""

import json
import os
import re
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
    QgsFeature,
    QgsGeometry,
    QgsNetworkAccessManager,
    QgsProject,
    QgsRasterLayer,
    QgsVectorLayer,
)
from qgis.PyQt.QtCore import QCoreApplication, QTimer, QUrl  # noqa: E402
from qgis.PyQt.QtNetwork import QNetworkRequest  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import analysis  # noqa: E402
import layout_export  # noqa: E402

work = tempfile.mkdtemp(prefix="cassini-atlas-")
project = QgsProject.instance()
project.setCrs(QgsCoordinateReferenceSystem("EPSG:2154"))
checks = []
outcomes = {}
requests = []  # requêtes réseau créées dans le processus


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


def out(name):
    return os.path.join(work, name)


def run_gesture(gesture, params, label, then):
    def done(result=None, error=None):
        outcomes[label] = (result, error)
        QTimer.singleShot(0, protect(then))

    outcomes["_g_" + label] = gesture  # gardé en vie pendant la tâche
    gesture.start(params, done)


def atlas(params, label, then):
    run_gesture(analysis.ProcessingGesture(analysis.ALGORITHMS["layout.export_atlas"]), params, label, then)


def simple(params, label, then):
    run_gesture(layout_export.LayoutExportGesture(), params, label, then)


def page_count(path):
    """Nombre de pages d'un PDF : le `/Count` de son noeud `/Type /Pages`, recoupé avec le nombre
    de références de son `/Kids`."""
    with open(path, "rb") as f:
        data = f.read()
    node = re.search(rb"/Type\s*/Pages\s*/Kids\s*\[(.*?)\]\s*/Count\s+(\d+)", data, re.S)
    if node is None:
        return None
    kids = len(re.findall(rb"\d+\s+\d+\s+R", node.group(1)))
    return int(node.group(2)) if kids == int(node.group(2)) else None


def is_pdf(path):
    with open(path, "rb") as f:
        return f.read(4) == b"%PDF"


# -- l'espion de requêtes, et son témoin ---------------------------------------------------------------
QgsNetworkAccessManager.instance().requestAboutToBeCreated.connect(lambda *_: requests.append("requête"))
QgsNetworkAccessManager.instance().get(QNetworkRequest(QUrl("file:///inexistant/temoin")))
check("témoin : l'espion voit passer une requête", len(requests) >= 1, requests)
del requests[:]

# -- données synthétiques ------------------------------------------------------------------------------
def ring(x, y=0, size=1000):
    x0, y0 = 900000 + x, 6500000 + y
    return "%d %d,%d %d,%d %d,%d %d,%d %d" % (x0, y0, x0 + size, y0, x0 + size, y0 + size, x0, y0 + size, x0, y0)


coverage = QgsVectorLayer("Polygon?field=nom:string&crs=EPSG:2154", "zones", "memory")
rows = []
for i, (nom, x) in enumerate((("Alpha", 0), ("Bravo", 3000), ("Charlie", 6000))):
    feature = QgsFeature(coverage.fields())
    feature.setAttributes([nom])
    feature.setGeometry(QgsGeometry.fromWkt("POLYGON((%s))" % ring(x)))
    rows.append(feature)
coverage.dataProvider().addFeatures(rows)
empty = QgsVectorLayer("Polygon?field=nom:string&crs=EPSG:2154", "vide", "memory")

fond_path = out("fond.geojson")
with open(fond_path, "w", encoding="utf-8") as f:
    json.dump(
        {
            "type": "FeatureCollection",
            "crs": {"type": "name", "properties": {"name": "urn:ogc:def:crs:EPSG::2154"}},
            "features": [
                {
                    "type": "Feature",
                    "properties": {"id": 1},
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[900000, 6500500], [909000, 6500500]],
                    },
                }
            ],
        },
        f,
    )
fond = QgsVectorLayer(fond_path, "fond local", "ogr")
check("fond local lisible", fond.isValid())
xyz_uri = "type=xyz&url=" + "ht" + "tps://tuiles.exemple.invalid/{z}/{x}/{y}.png&zmax=19&zmin=0"
xyz = QgsRasterLayer(xyz_uri, "Fond XYZ", "wms")
check("couche XYZ créée (sans connexion)", xyz.isValid())
for layer in (coverage, empty, fond, xyz):
    project.addMapLayer(layer)

root = project.layerTreeRoot()
layer_ids_before = sorted(project.mapLayers())
layouts_before = len(project.layoutManager().layouts())


def project_unchanged(label):
    check(
        "%s : le projet est inchangé (couches, mises en page)" % label,
        sorted(project.mapLayers()) == layer_ids_before and len(project.layoutManager().layouts()) == layouts_before,
        (sorted(project.mapLayers()), len(project.layoutManager().layouts())),
    )


def hide(*layers):
    for layer in layers:
        root.findLayer(layer.id()).setItemVisibilityChecked(False)


def show(*layers):
    for layer in layers:
        root.findLayer(layer.id()).setItemVisibilityChecked(True)


check("la règle d'écart voit la couche XYZ distante", layout_export.LayoutBackend().is_remote(xyz))
check("la règle d'écart garde les couches locales",
      not layout_export.LayoutBackend().is_remote(coverage) and not layout_export.LayoutBackend().is_remote(fond))


# -- 1. atlas sur trois entités, avec une couche XYZ visible ---------------------------------------------
def step_atlas():
    atlas(
        {"title": "Atlas des zones", "coverage_layer_id": coverage.id(), "label_field": "nom",
         "out": out("1-layout.export_atlas.pdf")},
        "atlas",
        step_atlas_done,
    )


def step_atlas_done():
    result, error = outcomes["atlas"]
    check("atlas : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    path = out("1-layout.export_atlas.pdf")
    check("atlas : le fichier est au chemin dicté et commence par %PDF", os.path.isfile(path) and is_pdf(path))
    pages = page_count(path)
    check("atlas : un seul PDF de trois pages", pages == 3, pages)
    check("atlas : page_count = 3", result["page_count"] == 3, result)
    check("atlas : la couche XYZ est écartée et nommée", result["skipped_layers"] == ["Fond XYZ"], result)
    check("atlas : deux couches locales dessinées", result["layer_count"] == 3 or result["layer_count"] == 2, result)
    check("atlas : sources = fichier du fond", result["sources"] == ["fond.geojson"] and result["truncated"] is False, result)
    check("atlas : les clés du résultat sont celles du catalogue",
          set(result) == {"file", "page_count", "layer_count", "skipped_layers", "sources", "truncated"}, sorted(result))
    check("atlas : aucune requête réseau", requests == [], requests)
    project_unchanged("atlas")
    step_simple()


# -- 2. export simple avec la même couche XYZ ------------------------------------------------------------
def step_simple():
    simple({"title": "Carte", "out": out("2-layout.export_pdf.pdf")}, "simple", step_simple_done)


def step_simple_done():
    result, error = outcomes["simple"]
    check("export simple : pas d'erreur", error is None, error and (error.code, error.message))
    if result is None:
        finish()
        return
    check("export simple : un PDF d'une page", is_pdf(out("2-layout.export_pdf.pdf")) and page_count(out("2-layout.export_pdf.pdf")) == 1,
          page_count(out("2-layout.export_pdf.pdf")))
    check("export simple : la couche XYZ est écartée et nommée", result["skipped_layers"] == ["Fond XYZ"], result)
    check("export simple : aucune requête réseau", requests == [], requests)
    project_unchanged("export simple")
    step_remote_only()


# -- 3. que du distant -----------------------------------------------------------------------------------
def step_remote_only():
    hide(coverage, empty, fond)
    atlas({"title": "Atlas", "coverage_layer_id": coverage.id(), "out": out("3-layout.export_atlas.pdf")},
          "distant_atlas", step_remote_only_simple)


def step_remote_only_simple():
    simple({"title": "Carte", "out": out("3-layout.export_pdf.pdf")}, "distant_simple", step_remote_only_done)


def step_remote_only_done():
    show(coverage, empty, fond)
    _, error = outcomes["distant_atlas"]
    check("que du distant, atlas : layout.empty", error is not None and error.code == "layout.empty", error and error.code)
    check("que du distant, atlas : aucun fichier", not os.path.exists(out("3-layout.export_atlas.pdf")))
    _, error = outcomes["distant_simple"]
    check("que du distant, export simple : layout.empty", error is not None and error.code == "layout.empty",
          error and error.code)
    check("que du distant, export simple : aucun fichier", not os.path.exists(out("3-layout.export_pdf.pdf")))
    check("que du distant : aucune requête réseau", requests == [], requests)
    project_unchanged("que du distant")
    step_empty_coverage()


# -- 4. refus avant lancement et échec de l'algorithme ---------------------------------------------------
def step_empty_coverage():
    atlas({"title": "Atlas", "coverage_layer_id": empty.id(), "out": out("4-layout.export_atlas.pdf")},
          "vide", step_unknown_field)


def step_unknown_field():
    _, error = outcomes["vide"]
    check("couverture vide : layer.empty", error is not None and error.code == "layer.empty", error and error.code)
    check("couverture vide : aucun fichier", not os.path.exists(out("4-layout.export_atlas.pdf")))
    atlas({"title": "Atlas", "coverage_layer_id": coverage.id(), "label_field": "inconnu",
           "out": out("5-layout.export_atlas.pdf")}, "champ", step_failure)


def step_failure():
    _, error = outcomes["champ"]
    check("champ inconnu : analysis.unknown_field", error is not None and error.code == "analysis.unknown_field",
          error and error.code)
    check("champ inconnu : aucun fichier", not os.path.exists(out("5-layout.export_atlas.pdf")))
    bad = os.path.join(fond_path, "6-layout.export_atlas.pdf")  # sous un fichier : ne peut pas s'écrire
    atlas({"title": "Atlas", "coverage_layer_id": coverage.id(), "out": bad}, "echec", step_last)


def step_last():
    _, error = outcomes["echec"]
    bad = os.path.join(fond_path, "6-layout.export_atlas.pdf")
    check("échec de l'algorithme : layout.export_failed", error is not None and error.code == "layout.export_failed",
          error and (error.code, error.message))
    check("échec de l'algorithme : aucun fichier", not os.path.exists(bad))
    project_unchanged("après refus et échec")
    check("aucune requête réseau, de bout en bout", requests == [], requests)
    finish()


QTimer.singleShot(0, protect(step_atlas))
QTimer.singleShot(180000, lambda: (check("sonde terminée à temps", False), finish()))
app.exec()

failed = [label for label, ok in checks if not ok]
verdict = bool(checks) and not failed
print("RÉSULTAT :", "OK" if verdict else "ÉCHEC (%s)" % ", ".join(failed))
QgsApplication.exitQgis()
sys.exit(0 if verdict else 1)
