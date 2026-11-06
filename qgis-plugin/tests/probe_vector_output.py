"""Sonde manuelle sous le vrai QGIS : un GeoJSON du noyau s'ouvre dans le projet (AD-30, 11.8).

À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas par `test`),
plateforme offscreen :

    set QT_QPA_PLATFORM=offscreen
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_vector_output.py

Dans un projet temporaire, une copie de `fixtures/points-2154.geojson` (les octets mêmes que
l'écrivain du noyau produit, `core/src/geojson.rs`) sous `p.cassini/`. Le geste
`layer.open_vector` (le vrai `QgisBackend`) l'ajoute au projet : une couche vecteur valide nommée
`1-terrain.peaks`, en `EPSG:2154` (le membre `crs` de 2008 lu par OGR), 3 points, champs `rank`,
`elevation`, `prominence` et leurs valeurs ; une couche neuve à chaque ordre. Un GeoJSON corrompu,
ou dont le système n'est pas reconnu (OGR lirait WGS84 en silence), est l'erreur `layer.invalid_source`, sans chemin, et n'ajoute
rien. Aucune donnée hors dépôt, aucun noyau nécessaire.
"""

import os
import shutil
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
FIXTURE = os.path.join(ROOT, "fixtures", "points-2154.geojson")

from qgis.core import Qgis, QgsApplication, QgsProject, QgsVectorLayer  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
import vector_output  # noqa: E402

FAILURES = []


def check(label, got, expected):
    ok = got == expected
    print("%s %s : %r" % ("OK  " if ok else "ÉCHEC", label, got))
    if not ok:
        FAILURES.append(label)
        print("     attendu : %r" % (expected,))


def open_now(path):
    """Le geste `layer.open_vector` avec le vrai fond QGIS ; rend (résultat, erreur)."""
    calls = []
    gesture = vector_output.LayerOpenVectorGesture(None, lambda fn: fn())
    gesture.start({"path": path}, lambda result=None, error=None: calls.append((result, error)))
    return calls[0]


def main():
    work = tempfile.mkdtemp(prefix="cassini-sonde-vecteur-")
    try:
        out = os.path.join(work, "p.cassini")
        os.makedirs(out)
        path = os.path.join(out, "1-terrain.peaks.geojson")
        shutil.copyfile(FIXTURE, path)
        project = QgsProject.instance()
        check("projet enregistré", project.write(os.path.join(work, "p.qgz")), True)

        result, error = open_now(path)
        check("ouverture sans erreur", error, None)
        result = result or {}
        check("nom rendu", result.get("name"), "1-terrain.peaks")
        check("crs rendu", result.get("crs"), "EPSG:2154")
        layers = project.mapLayersByName("1-terrain.peaks")
        check("une couche 1-terrain.peaks", len(layers), 1)
        if layers:
            layer = layers[0]
            check("couche vecteur", isinstance(layer, QgsVectorLayer), True)
            check("couche valide", layer.isValid(), True)
            check("id rendu", result.get("layer_id"), layer.id())
            check("système de la couche", layer.crs().authid(), "EPSG:2154")
            check("géométrie point", layer.geometryType() == Qgis.GeometryType.Point, True)
            check("3 entités", layer.featureCount(), 3)
            names = [f.name() for f in layer.fields()]
            check("champs", [n for n in names if n in ("rank", "elevation", "prominence")],
                  ["rank", "elevation", "prominence"])
            rows = sorted(
                (int(f["rank"]), float(f["elevation"]), float(f["prominence"]),
                 f.geometry().asPoint().x(), f.geometry().asPoint().y())
                for f in layer.getFeatures()
            )
            check("valeurs lues", rows, [
                (1, 150.0, 100.0, 652250.0, 6850250.0),
                (2, 87.5, 12.25, 652100.0, 6850400.0),
                (3, 62.0, 4.5, 652400.0, 6850100.0),
            ])
        again, _ = open_now(path)
        check("reçu deux fois : couche neuve", (again or {}).get("layer_id") != result.get("layer_id"), True)
        check("deux couches", len(project.mapLayersByName("1-terrain.peaks")), 2)

        # Collection vide (aucun point trouvé) : un résultat juste, pas une erreur.
        empty = os.path.join(out, "5-terrain.peaks.geojson")
        with open(FIXTURE, encoding="utf-8") as f:
            text = f.read()
        head = text[: text.index('"features":[')]
        with open(empty, "w", encoding="utf-8", newline="") as f:
            f.write(head + '"features":[]}\n')
        result, error = open_now(empty)
        check("collection vide sans erreur", error, None)
        check("crs rendu (vide)", (result or {}).get("crs"), "EPSG:2154")
        layers = project.mapLayersByName("5-terrain.peaks")
        check("une couche vide", len(layers), 1)
        if layers:
            check("couche vide valide", layers[0].isValid(), True)
            check("système de la couche vide", layers[0].crs().authid(), "EPSG:2154")
            check("0 entité", layers[0].featureCount(), 0)

        broken = os.path.join(out, "9-terrain.peaks.geojson")
        with open(broken, "w", encoding="utf-8") as f:
            f.write('{"type":"FeatureCollection","features":[{"type":')
        _, bad = open_now(broken)
        check("GeoJSON corrompu", getattr(bad, "code", None), "layer.invalid_source")
        check("message sans chemin", work in getattr(bad, "message", ""), False)
        check("rien d'ajouté (corrompu)", len(project.mapLayersByName("9-terrain.peaks")), 0)

        unknown = os.path.join(out, "8-terrain.peaks.geojson")
        with open(FIXTURE, encoding="utf-8") as f:
            text = f.read().replace("urn:ogc:def:crs:EPSG::2154", "urn:ogc:def:crs:EPSG::999999")
        with open(unknown, "w", encoding="utf-8", newline="") as f:
            f.write(text)
        _, bad = open_now(unknown)
        check("système inconnu", getattr(bad, "code", None), "layer.invalid_source")
        check("message sans chemin (système)", work in getattr(bad, "message", ""), False)
        check("rien d'ajouté (système)", len(project.mapLayersByName("8-terrain.peaks")), 0)
        project.clear()
    finally:
        shutil.rmtree(work, ignore_errors=True)


try:
    main()
except Exception:
    import traceback

    traceback.print_exc()
    FAILURES.append("exception")
finally:
    QgsApplication.exitQgis()
print("RÉSULTAT :", "ÉCHEC (" + ", ".join(FAILURES) + ")" if FAILURES else "OK")
sys.exit(1 if FAILURES else 0)
