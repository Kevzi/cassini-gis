"""Geste `layer.add` : ajoute au projet une couche issue d'un fichier de l'utilisateur.

Le noyau décide (chemin déjà résolu, AD-10) ; ce geste exécute seulement : il lit
l'extension du fichier, demande à QGIS d'ouvrir la couche en lecture seule, puis
rend son id, son SCR et si QGIS la reprojette. Le fichier n'est jamais modifié et
la couche garde son SCR : la reprojection est celle de QGIS (AD-19).

Le travail avance par un ordonnanceur (par défaut `QTimer.singleShot`), jamais par
une attente. L'accès à QGIS passe par un « fond » injectable : `QgisBackend` est
le seul endroit qui importe `qgis.core`, les tests mettent une doublure.
"""

import os
import re

try:  # `gestures` est importé à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
except ImportError:  # pragma: no cover
    from gestures import GestureError

# Extension -> fournisseur de données (tous lus par OGR dans QGIS).
FORMATS = {
    ".gpkg": "ogr",
    ".shp": "ogr",
    ".geojson": "ogr",
    ".parquet": "ogr",
    ".csv": "delimitedtext",  # colonnes X/Y, lu par le fournisseur de texte délimité
}
RASTER_EXTENSIONS = (".tif", ".tiff")  # un MNT GeoTIFF, lu par GDAL dans QGIS
DEFAULT_X, DEFAULT_Y = "X", "Y"

UNSUPPORTED = "layer.unsupported_format"
INVALID = "layer.invalid_source"
UNKNOWN_CRS = "layer.unknown_crs"
INVALID_CRS = "layer.invalid_crs"
INVALID_COLUMN = "layer.invalid_column"
ONE_INPUT = "layer.one_input"
CRS_CONFLICT = "layer.crs_conflict"
CANCELLED = "qgis.cancelled"
CRS_FORMAT = re.compile(r"^EPSG:[0-9]+$")


class QgisBackend:
    """Accès réel à QGIS. Seul ce type touche `qgis.core`."""

    def load(self, path, name, provider, csv=None):
        from qgis.core import QgsVectorLayer
        from qgis.PyQt.QtCore import QUrl

        source = path
        if csv is not None:  # URI du fournisseur delimitedtext
            url = QUrl.fromLocalFile(path)
            if csv.get("table"):
                # Tableau sans géométrie : aucun SCR, aucun type deviné (`detectTypes=no` : tous les
                # champs en texte, les zéros de tête d'un code INSEE sont conservés).
                url.setQuery("type=csv&delimiter=,;&detectTypes=no&geomType=none")
            else:
                # Le fournisseur décode lui-même les noms de colonnes ; le SCR est déjà validé.
                enc = lambda v: bytes(QUrl.toPercentEncoding(v)).decode("ascii")  # noqa: E731
                url.setQuery(
                    "type=csv&delimiter=,;&detectTypes=yes&geomType=point"
                    "&xField=%s&yField=%s&crs=%s" % (enc(csv["x"]), enc(csv["y"]), csv["crs"])
                )
            source = url.toString()
        options = QgsVectorLayer.LayerOptions()
        options.forceReadOnly = True  # ouverture en lecture seule, pas seulement l'édition
        layer = QgsVectorLayer(source, name, provider, options)
        if layer.isValid():
            layer.setReadOnly(True)  # jamais d'édition du fichier d'origine
        return layer

    def load_raster(self, path, name):
        """Ouvre un GeoTIFF par GDAL, en lecture. QGIS calcule les statistiques du raster à
        l'ouverture et GDAL les écrirait dans un `.aux.xml` à côté du fichier : GDAL n'est donc
        invité à ne pas écrire ce fichier (`GDAL_PAM_ENABLED=NO`) que le temps de l'ouverture,
        puis l'option est rendue telle quelle, sans toucher aux autres rasters de la session."""
        from qgis.core import QgsRasterLayer

        try:
            from osgeo import gdal
        except ImportError:  # pragma: no cover - GDAL accompagne toujours QGIS
            return QgsRasterLayer(path, name, "gdal")
        previous = gdal.GetConfigOption("GDAL_PAM_ENABLED")
        gdal.SetConfigOption("GDAL_PAM_ENABLED", "NO")
        try:
            layer = QgsRasterLayer(path, name, "gdal")
            if layer.isValid():  # lit tout ce que l'inventaire lira, pendant que l'option tient
                layer.width(), layer.height(), layer.crs()
            return layer
        finally:
            gdal.SetConfigOption("GDAL_PAM_ENABLED", previous)

    def is_valid(self, layer):
        return layer is not None and layer.isValid()

    def crs_of(self, layer):
        return layer.crs().authid()

    def assign_crs(self, layer, authid):
        """Donne un SCR à la couche en mémoire (le fichier n'est pas touché)."""
        from qgis.core import QgsCoordinateReferenceSystem

        crs = QgsCoordinateReferenceSystem(authid)
        if not crs.isValid():
            return ""
        layer.setCrs(crs)
        return layer.crs().authid()

    def project_crs(self):
        from qgis.core import QgsProject

        return QgsProject.instance().crs().authid()

    def add(self, layer):
        from qgis.core import QgsProject

        QgsProject.instance().addMapLayer(layer)
        return layer.id()

    def name_of(self, layer):
        return layer.name()


def _singleshot(fn):
    from qgis.PyQt.QtCore import QTimer

    QTimer.singleShot(0, fn)


class LayerAddGesture:
    def __init__(self, backend=None, schedule=None):
        self._backend = backend if backend is not None else QgisBackend()
        self._schedule = schedule if schedule is not None else _singleshot
        self._done = None
        self._params = None
        self._cancelled = False

    def start(self, params, done):
        self._done = done
        self._params = params
        self._schedule(self._run)

    def cancel(self):
        self._cancelled = True
        self._finish(error=GestureError(CANCELLED, "Geste annulé."))

    def _run(self):
        if self._done is None:  # déjà soldé (annulation)
            return
        try:
            self._finish(result=self._open())
        except GestureError as exc:
            self._finish(error=exc)
        except Exception:  # un ordre sans réponse bloquerait le noyau
            self._finish(error=GestureError("qgis.gesture_failed", "Échec inattendu de l'ouverture."))

    def _open(self):
        params = self._params or {}
        path, table, raster = params.get("path"), params.get("table"), params.get("raster")
        if sum(v is not None for v in (path, table, raster)) != 1:
            raise GestureError(
                ONE_INPUT,
                "Donnez exactement un de path (couche), table (tableau sans géométrie) et raster (MNT GeoTIFF).",
            )
        is_table = table is not None
        is_raster = raster is not None
        path = table if is_table else raster if is_raster else path
        if not isinstance(path, str) or not path:
            raise GestureError(INVALID, "Aucun fichier à ouvrir.")
        name = os.path.basename(path)
        if is_raster:
            return self._open_raster(path, name)
        provider = FORMATS.get(os.path.splitext(name)[1].lower())
        if is_table and provider != "delimitedtext":
            raise GestureError(
                UNSUPPORTED,
                "Un tableau est un fichier .csv : %s n'en est pas un." % (os.path.splitext(name)[1] or "sans extension"),
            )
        if provider is None:
            raise GestureError(
                UNSUPPORTED,
                "Format non pris en charge : %s (attendu .gpkg, .shp, .geojson, .csv ou .parquet)."
                % (os.path.splitext(name)[1] or "sans extension"),
            )
        backend = self._backend
        csv = None
        if is_table:  # sans géométrie : ni crs, ni colonnes X/Y
            self._refuse_coordinate_options()
            csv = {"table": True}
        elif provider == "delimitedtext":  # un CSV ne porte aucun SCR : il doit être donné
            csv = self._csv_options()
        # Message sans chemin absolu : seul le nom du fichier est cité.
        layer = backend.load(path, os.path.splitext(name)[0], provider, csv)
        if not backend.is_valid(layer):
            raise GestureError(INVALID, "Le fichier « %s » n'est pas lisible." % name)
        if self._cancelled:
            raise GestureError(CANCELLED, "Geste annulé.")
        source_crs = "" if is_table else self._resolve_crs(layer)  # un tableau n'a pas de SCR
        project_crs = backend.project_crs()
        layer_id = backend.add(layer)
        return {
            "kind": "vector",
            "layer_id": layer_id,
            "name": backend.name_of(layer),
            "source_crs": source_crs,
            "project_crs": project_crs,
            "reprojected": bool(source_crs) and source_crs != project_crs,
        }

    def _open_raster(self, path, name):
        """Ouvre un MNT GeoTIFF : `.tif`/`.tiff` seulement, SCR du fichier ou donné (AD-19)."""
        ext = os.path.splitext(name)[1].lower()
        if ext not in RASTER_EXTENSIONS:
            raise GestureError(
                UNSUPPORTED,
                "Format raster non pris en charge : %s (attendu .tif ou .tiff)." % (ext or "sans extension"),
            )
        params = self._params or {}
        for key in ("x_field", "y_field"):
            if params.get(key) is not None:
                raise GestureError(ONE_INPUT, "Un raster n'a ni x_field, ni y_field : retirez %s." % key)
        backend = self._backend
        layer = backend.load_raster(path, os.path.splitext(name)[0])
        if not backend.is_valid(layer):
            raise GestureError(INVALID, "Le fichier « %s » n'est pas lisible." % name)
        if self._cancelled:
            raise GestureError(CANCELLED, "Geste annulé.")
        source_crs = self._resolve_crs(layer)
        project_crs = backend.project_crs()
        layer_id = backend.add(layer)
        return {
            "kind": "raster",
            "layer_id": layer_id,
            "name": backend.name_of(layer),
            "source_crs": source_crs,
            "project_crs": project_crs,
            "reprojected": source_crs != project_crs,
        }

    def _refuse_coordinate_options(self):
        params = self._params or {}
        for key in ("crs", "x_field", "y_field"):
            if params.get(key) is not None:
                raise GestureError(
                    ONE_INPUT, "Un tableau sans géométrie n'a ni crs, ni x_field, ni y_field : retirez %s." % key
                )

    def _csv_options(self):
        params = self._params or {}
        asked = params.get("crs")
        if asked is None:
            raise GestureError(
                UNKNOWN_CRS,
                "Un CSV n'indique pas son système de coordonnées : "
                "ajoutez le paramètre crs (ex. EPSG:2154).",
            )
        if not (isinstance(asked, str) and CRS_FORMAT.match(asked)):
            raise GestureError(INVALID_CRS, "SCR invalide : attendu un code comme EPSG:2154.")
        fields = []
        for key, default in (("x_field", DEFAULT_X), ("y_field", DEFAULT_Y)):
            value = params.get(key, default)
            if not isinstance(value, str) or not value.strip():
                raise GestureError(INVALID_COLUMN, "Nom de colonne invalide pour %s." % key)
            fields.append(value)
        return {"x": fields[0], "y": fields[1], "crs": asked}

    def _resolve_crs(self, layer):
        """SCR explicite de la couche : celui du fichier, ou celui demandé (AD-19)."""
        asked = (self._params or {}).get("crs")
        if asked is not None and not (isinstance(asked, str) and CRS_FORMAT.match(asked)):
            raise GestureError(INVALID_CRS, "SCR invalide : attendu un code comme EPSG:2154.")
        found = self._backend.crs_of(layer)
        if found:
            if asked is not None and asked != found:
                raise GestureError(
                    CRS_CONFLICT,
                    "Le fichier est en %s, pas en %s : retirez le paramètre crs." % (found, asked),
                )
            return found
        if asked is None:
            raise GestureError(
                UNKNOWN_CRS,
                "Le fichier n'indique pas son système de coordonnées : "
                "ajoutez le paramètre crs (ex. EPSG:2154).",
            )
        assigned = self._backend.assign_crs(layer, asked)
        if not assigned:
            raise GestureError(INVALID_CRS, "SCR inconnu de QGIS : %s." % asked)
        return assigned

    def _finish(self, result=None, error=None):
        done, self._done = self._done, None
        if done is not None:
            done(result=result, error=error)
