"""Gestes `project.inventory` et `layer.describe` : lecture seule du projet QGIS.

Le noyau décide ; ces gestes exécutent seulement : ils demandent à QGIS ce que le
projet contient et rendent des données simples (SCR en code explicite, AD-19).
Aucune couche et aucun fichier n'est modifié. Une source de couche vecteur est rendue
par son nom de fichier, une source raster par son chemin relatif au dossier du fichier de
projet (le noyau la recopie dans `dem`), jamais par un chemin absolu. Une couche se désigne par son id
QGIS, valable pour la session seulement (AD-9).

L'inventaire rapporte aussi des faits bruts (sorte de couche, géométrie WKB, nombre
d'objets, champs avec leur type QGIS, quelques valeurs des champs texte, nom du
projet) : c'est le noyau qui les met dans la forme vue par le modèle et qui décide du
plafond de valeurs. L'étendue d'ensemble porte aussi sa version en degrés WGS84
(`extent.lonlat`), reprojetée par QGIS : le noyau en déduit le fuseau UTM du projet sans
convertir lui-même une coordonnée (AD-19).

Le travail avance par un ordonnanceur (par défaut `QTimer.singleShot`). L'accès à
QGIS passe par un « fond » injectable : `QgisBackend` est le seul endroit qui
importe `qgis.core`, les tests mettent une doublure.
"""

import copy
import math
import ntpath
import os
import posixpath
import re

try:  # `gestures` est importé à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
except ImportError:  # pragma: no cover
    from gestures import GestureError

NOT_FOUND = "layer.not_found"
NOT_VECTOR = "layer.not_vector"
CANCELLED = "qgis.cancelled"

# Valeurs lues par champ texte : une de plus que le plafond du noyau (10), pour qu'il sache
# qu'un champ en a trop. La lecture s'arrête là, quelle que soit la taille de la couche.
VALUES_LIMIT = 11


class QgisBackend:
    """Accès réel à QGIS. Seul ce type touche `qgis.core`."""

    def project_crs(self):
        from qgis.core import QgsProject

        return QgsProject.instance().crs().authid()

    def project_crs_unit(self):
        """Unité du système du projet : `m`, `deg` ou `other` (entrée 9.16)."""
        from qgis.core import QgsProject

        return crs_unit(QgsProject.instance().crs())

    def crs_unit_of(self, layer):
        """Unité du système de la couche : `m`, `deg` ou `other` (entrée 9.16)."""
        return crs_unit(layer.crs())

    def project_dir(self):
        """Dossier du fichier de projet ; vide si le projet n'est pas enregistré."""
        from qgis.core import QgsProject

        return QgsProject.instance().absolutePath()

    def project_name(self):
        """Nom du projet : nom de son fichier sans extension ; vide s'il n'est pas enregistré."""
        from qgis.core import QgsProject

        return QgsProject.instance().baseName()

    def layers(self):
        """Couches dans l'ordre de la légende, de haut en bas."""
        from qgis.core import QgsProject

        found = []
        for node in QgsProject.instance().layerTreeRoot().findLayers():
            layer = node.layer()
            if layer is not None:
                found.append(layer)
        return found

    def layer(self, layer_id):
        from qgis.core import QgsProject

        return QgsProject.instance().mapLayer(layer_id)

    def id_of(self, layer):
        return layer.id()

    def name_of(self, layer):
        return layer.name()

    def source_of(self, layer):
        return layer.source()

    def crs_of(self, layer):
        return layer.crs().authid()

    def extent_of(self, layer, to_authid=None):
        """Étendue (xmin, ymin, xmax, ymax) ou None ; reprojetée par QGIS si demandé."""
        from qgis.core import QgsCoordinateReferenceSystem, QgsCoordinateTransform, QgsProject

        rect = layer.extent()
        if rect is None or rect.isNull():  # un point seul a une étendue de largeur nulle : valide
            return None
        if to_authid and to_authid != layer.crs().authid():
            target = QgsCoordinateReferenceSystem(to_authid)
            if not target.isValid():
                return None
            transform = QgsCoordinateTransform(layer.crs(), target, QgsProject.instance())
            rect = transform.transformBoundingBox(rect)
        return (rect.xMinimum(), rect.yMinimum(), rect.xMaximum(), rect.yMaximum())

    def is_vector(self, layer):
        from qgis.core import QgsMapLayerType

        return layer.type() == QgsMapLayerType.VectorLayer

    def kind_of(self, layer):
        """`vector`, `raster` ou `other` (maillage, nuage de points, tuiles...)."""
        from qgis.core import QgsMapLayerType

        if layer.type() == QgsMapLayerType.VectorLayer:
            return "vector"
        if layer.type() == QgsMapLayerType.RasterLayer:
            return "raster"
        return "other"

    def wkb_name(self, layer):
        """Nom WKB de la géométrie (ex. `MultiPolygon`, `PointZ`, `NoGeometry`)."""
        from qgis.core import QgsWkbTypes

        return QgsWkbTypes.displayString(layer.wkbType())

    def text_values(self, layer, name, limit):
        """Valeurs distinctes non nulles d'un champ texte, au plus `limit` ; None sinon."""
        idx = layer.fields().indexOf(name)
        if idx < 0:
            return None
        field = layer.fields().at(idx)
        if field.isNumeric() or field.isDateOrTime():
            return None
        # Deux places de plus : la valeur nulle et le texte vide, que `uniqueValues` compte
        # aussi et que `_distinct_texts` écarte.
        return _distinct_texts(layer.uniqueValues(idx, limit + 2), limit)

    def fields_of(self, layer):
        return [(f.name(), f.typeName()) for f in layer.fields()]

    def feature_count(self, layer):
        return max(0, int(layer.featureCount()))  # -1 : le fournisseur ne sait pas compter

    def facts_key(self, layer):
        """Clé sous laquelle les faits d'une couche vectorielle restent valables : id, source,
        filtre, nombre d'objets, champs et types, dates et tailles du fichier source et de ses
        compagnons (voir `source_stamps`). None (rien n'est gardé) pour une couche qui a des
        modifications non enregistrées ou dont la source n'est pas un fichier local."""
        if layer.isModified():
            return None
        source = layer.source()
        stamps = source_stamps(source)
        if stamps is None:
            return None
        return (
            layer.id(),
            source,
            layer.subsetString(),
            int(layer.featureCount()),
            tuple((f.name(), f.typeName()) for f in layer.fields()),
            stamps,
        )

    def geometry_of(self, layer):
        from qgis.core import QgsWkbTypes

        name = QgsWkbTypes.geometryDisplayString(layer.geometryType())
        return "NoGeometry" if name == "Null" else name


def crs_unit(crs):
    """Unité d'un système de QGIS : `m` (mètres), `deg` (degrés) ou `other` (autre unité, ou
    système invalide). Un fait lu par QGIS (`mapUnits`) : le noyau en tire le système de calcul
    d'une distance en mètres, sans rien déduire d'un code (changement de cap du 2026-11-12)."""
    from qgis.core import Qgis

    if crs is None or not crs.isValid():
        return "other"
    units = crs.mapUnits()
    if units == Qgis.DistanceUnit.Meters:
        return "m"
    if units == Qgis.DistanceUnit.Degrees:
        return "deg"
    return "other"


def source_file(raw):
    """Chemin du fichier local d'une source de couche (`x.gpkg|layername=…`,
    `file:///d/x.csv?type=csv…`), ou None si la source n'est pas un fichier existant."""
    if not isinstance(raw, str) or not raw:
        return None
    text = raw.split("|", 1)[0]
    if text.startswith("file://"):
        text = _percent_decoded(text.split("?", 1)[0][len("file://"):])
        if re.match(r"^/[A-Za-z]:/", text):  # file:///C:/d/x.csv -> C:/d/x.csv
            text = text[1:]
    return text if os.path.isfile(text) else None


def _percent_decoded(text):
    """`%20` et autres échappements d'une adresse `file://`, en UTF-8 (sans module réseau)."""
    return re.sub(
        r"(?:%[0-9A-Fa-f]{2})+",
        lambda m: bytes.fromhex(m.group(0).replace("%", "")).decode("utf-8", "replace"),
        text,
    )


def source_stamps(raw):
    """(nom, date, taille) du fichier d'une source et de ses compagnons qui portent les données :
    `.dbf` d'un Shapefile (les attributs), `-wal` d'un GeoPackage ou d'une base SQLite (les
    écritures pas encore reportées). None si la source n'est pas un fichier local."""
    path = source_file(raw)
    if path is None:
        return None
    stem = os.path.splitext(path)[0]
    stamps = []
    for candidate in (path, stem + ".dbf", stem + ".DBF", path + "-wal"):
        try:
            st = os.stat(candidate)
        except OSError:
            continue
        stamps.append((os.path.basename(candidate), st.st_mtime_ns, st.st_size))
    return tuple(stamps)


def source_name(raw):
    """Nom de fichier d'une source de couche, sans dossier ni option du fournisseur."""
    if not isinstance(raw, str) or not raw:
        return None
    text = raw.split("|", 1)[0]  # `fichier.gpkg|layername=x` -> `fichier.gpkg`
    text = text.split("?", 1)[0]  # URI `file:///d/x.csv?type=csv&...` -> `file:///d/x.csv`
    if "=" in text or "://" in text and not text.startswith("file://"):
        return None  # base distante, flux ou couche en mémoire : pas un fichier (et jamais d'identifiants)
    text = text.replace("\\", "/").rstrip("/")
    name = os.path.basename(text)
    return name if name and os.path.splitext(name)[1] else None


_WINDOWS_ABSOLUTE = re.compile(r"^(?:[A-Za-z]:[\\/]|[\\/]{2}[^\\/])")


def _flavour(path):
    """Module de chemins (`ntpath`, `posixpath`) d'un chemin absolu ; None s'il est relatif."""
    if _WINDOWS_ABSOLUTE.match(path):
        return ntpath
    if path.startswith("/"):
        return posixpath
    return None


def raster_source(raw, project_dir):
    """Source d'une couche raster : chemin relatif au dossier du fichier de projet, séparateurs
    `/`, `..` permis. Sans chemin relatif possible (projet non enregistré, autre lecteur sous
    Windows, chemins de deux systèmes), le nom de fichier seul. Jamais un chemin absolu."""
    name = source_name(raw)
    if name is None or not isinstance(project_dir, str) or not project_dir:
        return name
    path = raw.split("|", 1)[0].split("?", 1)[0]
    if path.startswith("file://"):
        path = path[len("file://"):]
        if re.match(r"^/[A-Za-z]:", path):  # `file:///C:/x.tif` -> `C:/x.tif`
            path = path[1:]
    if path.startswith("/vsi"):  # source virtuelle de GDAL (`/vsizip/`, `/vsicurl/`...)
        return name
    flavour = _flavour(path)
    if flavour is None or _flavour(project_dir) is not flavour:
        return name
    try:
        relative = flavour.relpath(path, project_dir)
    except ValueError:  # autre lecteur sous Windows
        return name
    relative = relative.replace("\\", "/")
    if not relative or relative == "." or _flavour(relative) is not None:
        return name
    return relative


def _distinct_texts(found, limit):
    """Textes non vides de `found`, sans doublon, triés, au plus `limit` ; None s'il n'en
    reste aucun (champ booléen, champ tout nul)."""
    texts = sorted({v for v in found if isinstance(v, str) and v})[:limit]
    return texts or None


# Degrés WGS84, ordre longitude puis latitude (celui des API de QGIS).
LONLAT_AUTHID = "EPSG:4326"


def _extent(box, crs, lonlat=None):
    if box is None:
        return None
    out = {"xmin": box[0], "ymin": box[1], "xmax": box[2], "ymax": box[3], "crs": crs}
    if lonlat is not None:
        out["lonlat"] = {
            "west": lonlat[0], "south": lonlat[1], "east": lonlat[2], "north": lonlat[3],
            "crs": LONLAT_AUTHID,
        }
    return out


def _finite_box(box):
    """`box` si ses quatre nombres sont finis, None sinon (un NaN casserait le JSON du canal)."""
    if box is None:
        return None
    try:
        if all(math.isfinite(v) for v in box):
            return tuple(float(v) for v in box)
    except TypeError:
        return None
    return None


def _degrees_box(box):
    """`box` si c'est une étendue en degrés (longitudes dans [-180, 180], latitudes dans
    [-90, 90]), None sinon : une couche sans SCR rend des mètres, finis mais hors bornes."""
    box = _finite_box(box)
    if box is None:
        return None
    west, south, east, north = box
    if all(-180.0 <= v <= 180.0 for v in (west, east)) and all(-90.0 <= v <= 90.0 for v in (south, north)):
        return box
    return None


def _union(boxes):
    boxes = [b for b in boxes if b is not None]
    if not boxes:
        return None
    return (
        min(b[0] for b in boxes),
        min(b[1] for b in boxes),
        max(b[2] for b in boxes),
        max(b[3] for b in boxes),
    )


def _singleshot(fn):
    from qgis.PyQt.QtCore import QTimer

    QTimer.singleShot(0, fn)


# Faits des couches vectorielles déjà lus, d'une phrase à l'autre : `uniqueValues` sur des centaines
# de milliers d'objets coûte des secondes par champ (20 s sur le projet d'Annecy, 2026-11-08). La
# clé (`facts_key`, premier élément : l'id de la couche) change dès que la couche change ; une
# seule entrée par couche, et celles des couches sorties du projet sont retirées à chaque
# inventaire : la table ne dépasse jamais le projet.
_FACTS = {}


class _ReadGesture:
    """Squelette commun : un travail court, planifié, qui rend un résultat ou une erreur."""

    failure = "Échec inattendu de la lecture du projet."

    def __init__(self, backend=None, schedule=None):
        self._backend = backend if backend is not None else QgisBackend()
        self._schedule = schedule if schedule is not None else _singleshot
        self._done = None
        self._params = None

    def start(self, params, done):
        self._done = done
        self._params = params
        self._schedule(self._run)

    def cancel(self):
        self._finish(error=GestureError(CANCELLED, "Geste annulé."))

    def _run(self):
        if self._done is None:  # déjà soldé (annulation)
            return
        try:
            self._finish(result=self._read())
        except GestureError as exc:
            self._finish(error=exc)
        except Exception:  # un ordre sans réponse bloquerait le noyau
            self._finish(error=GestureError("qgis.gesture_failed", self.failure))

    def _read(self):  # pragma: no cover
        raise NotImplementedError

    def _finish(self, result=None, error=None):
        done, self._done = self._done, None
        if done is not None:
            done(result=result, error=error)


class ProjectInventoryGesture(_ReadGesture):
    failure = "Échec inattendu de l'inventaire du projet."

    def _read(self):
        backend = self._backend
        project_crs = backend.project_crs()
        project_dir = backend.project_dir()
        layers = []
        boxes = []
        lonlats = []
        for layer in backend.layers():
            kind = backend.kind_of(layer)
            raw = backend.source_of(layer)
            entry = {
                "layer_id": backend.id_of(layer),
                "name": backend.name_of(layer),
                "source": raster_source(raw, project_dir) if kind == "raster" else source_name(raw),
                "crs": backend.crs_of(layer),
            }
            unit = self._unit(lambda: backend.crs_unit_of(layer))
            if unit is not None:
                entry["crs_unit"] = unit
            entry.update(self._facts(layer, kind))
            layers.append(entry)
            boxes.append(backend.extent_of(layer, project_crs))
            lonlats.append(self._lonlat(layer))
        present = {backend.id_of(layer) for layer in backend.layers()}
        for gone in [k for k in _FACTS if k[0] not in present]:
            del _FACTS[gone]
        result = {
            "project_name": backend.project_name() or None,
            "layers": layers,
            "project_crs": project_crs,
            "extent": _extent(_union(boxes), project_crs, _union(lonlats)),
        }
        unit = self._unit(lambda: backend.project_crs_unit())
        if unit is not None:
            result["project_crs_unit"] = unit
        return result

    def _unit(self, read):
        """Unité lue par `read` ; `other` si la lecture échoue, None si le fond ne sait pas la lire.
        Dans les deux cas, le noyau ne remplit pas de système de calcul : le tampon se fait seul,
        sous la garde du geste, comme avant l'entrée 9.16 (amendement RA06)."""
        if not hasattr(self._backend, "crs_unit_of"):
            return None
        try:
            unit = read()
        except Exception:  # une unité illisible ne prive pas le noyau de l'inventaire
            return "other"
        return unit if unit in ("m", "deg", "other") else "other"

    def _lonlat(self, layer):
        """Étendue d'une couche en degrés WGS84, reprojetée par QGIS ; None si elle manque, si la
        couche n'a pas de SCR valide, si la reprojection échoue ou sort des bornes des degrés :
        la couche reste hors de l'union, l'inventaire n'échoue pas pour autant."""
        try:
            if not self._backend.crs_of(layer):
                return None
            return _degrees_box(self._backend.extent_of(layer, LONLAT_AUTHID))
        except Exception:
            return None

    def _facts(self, layer, kind):
        """Faits bruts d'une couche : sorte, géométrie, nombre d'objets, champs. Ceux d'une couche
        vectorielle inchangée sont repris de la phrase précédente (`_FACTS`)."""
        backend = self._backend
        if kind != "vector":
            return {"kind": kind, "geometry": None, "fields": []}
        try:
            key = backend.facts_key(layer) if hasattr(backend, "facts_key") else None
        except Exception:  # une clé illisible : la couche est relue, l'inventaire n'échoue pas
            key = None
        if key is not None and key in _FACTS:
            return copy.deepcopy(_FACTS[key])
        facts = self._read_facts(layer, kind)
        if key is not None:
            # Une seule entrée par couche : l'ancienne clé d'une couche qui a changé sort.
            for old in [k for k in _FACTS if k[0] == key[0]]:
                del _FACTS[old]
            _FACTS[key] = copy.deepcopy(facts)
        return facts

    def _read_facts(self, layer, kind):
        backend = self._backend
        fields = []
        for name, qgis_type in backend.fields_of(layer):
            field = {"name": name, "type": qgis_type}
            try:
                values = backend.text_values(layer, name, VALUES_LIMIT)
            except Exception:  # une lecture ratée ne prive pas le noyau de l'inventaire
                values = None
            if values is not None:
                field["values"] = list(values)
            fields.append(field)
        return {
            "kind": kind,
            "geometry": backend.wkb_name(layer),
            "feature_count": backend.feature_count(layer),
            "fields": fields,
        }


class LayerDescribeGesture(_ReadGesture):
    failure = "Échec inattendu de la description de la couche."

    def _read(self):
        layer_id = (self._params or {}).get("layer_id")
        backend = self._backend
        layer = backend.layer(layer_id) if isinstance(layer_id, str) and layer_id else None
        if layer is None:
            raise GestureError(NOT_FOUND, "Aucune couche du projet ne porte cet identifiant.")
        if not backend.is_vector(layer):
            raise GestureError(
                NOT_VECTOR, "La couche « %s » n'est pas une couche vecteur." % backend.name_of(layer)
            )
        crs = backend.crs_of(layer)
        return {
            "name": backend.name_of(layer),
            "fields": [{"name": n, "type": t} for n, t in backend.fields_of(layer)],
            "crs": crs,
            "extent": _extent(backend.extent_of(layer), crs),
            "feature_count": backend.feature_count(layer),
            "geometry": backend.geometry_of(layer),
        }
