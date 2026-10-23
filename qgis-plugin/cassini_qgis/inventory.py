"""Gestes `project.inventory` et `layer.describe` : lecture seule du projet QGIS.

Le noyau décide ; ces gestes exécutent seulement : ils demandent à QGIS ce que le
projet contient et rendent des données simples (SCR en code explicite, AD-19).
Aucune couche et aucun fichier n'est modifié. Une source de couche est rendue par
son nom de fichier, jamais par un chemin absolu. Une couche se désigne par son id
QGIS, valable pour la session seulement (AD-9).

L'inventaire rapporte aussi des faits bruts (sorte de couche, géométrie WKB, nombre
d'objets, champs avec leur type QGIS, quelques valeurs des champs texte, nom du
projet) : c'est le noyau qui les met dans la forme vue par le modèle et qui décide du
plafond de valeurs.

Le travail avance par un ordonnanceur (par défaut `QTimer.singleShot`). L'accès à
QGIS passe par un « fond » injectable : `QgisBackend` est le seul endroit qui
importe `qgis.core`, les tests mettent une doublure.
"""

import os

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

    def geometry_of(self, layer):
        from qgis.core import QgsWkbTypes

        name = QgsWkbTypes.geometryDisplayString(layer.geometryType())
        return "NoGeometry" if name == "Null" else name


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


def _distinct_texts(found, limit):
    """Textes non vides de `found`, sans doublon, triés, au plus `limit` ; None s'il n'en
    reste aucun (champ booléen, champ tout nul)."""
    texts = sorted({v for v in found if isinstance(v, str) and v})[:limit]
    return texts or None


def _extent(box, crs):
    if box is None:
        return None
    return {"xmin": box[0], "ymin": box[1], "xmax": box[2], "ymax": box[3], "crs": crs}


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
        layers = []
        boxes = []
        for layer in backend.layers():
            entry = {
                "layer_id": backend.id_of(layer),
                "name": backend.name_of(layer),
                "source": source_name(backend.source_of(layer)),
                "crs": backend.crs_of(layer),
            }
            entry.update(self._facts(layer))
            layers.append(entry)
            boxes.append(backend.extent_of(layer, project_crs))
        return {
            "project_name": backend.project_name() or None,
            "layers": layers,
            "project_crs": project_crs,
            "extent": _extent(_union(boxes), project_crs),
        }

    def _facts(self, layer):
        """Faits bruts d'une couche : sorte, géométrie, nombre d'objets, champs."""
        backend = self._backend
        kind = backend.kind_of(layer)
        if kind != "vector":
            return {"kind": kind, "geometry": None, "fields": []}
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
