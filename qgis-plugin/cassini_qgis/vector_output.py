"""Geste `layer.open_vector` : ouvre dans le projet QGIS un GeoJSON écrit par le noyau (AD-30).

Le noyau dicte ce geste lui-même, juste après une commande du noyau dont la sortie est un
GeoJSON, et le journalise avec le `layer_id` rendu : le rejeu le rejoue et sait laisser la
couche d'origine hors de la carte, comme `layer.open_raster` (ticket 4.9).

Aucune décision ici : le chemin absolu est celui que le noyau donne (AD-9, AD-10), la couche
est nommée du radical du fichier. Chaque ordre ajoute une couche neuve, même si le fichier est
déjà ouvert. Le système est celui que déclare le fichier (membre `crs` de 2008), lu par le
pilote OGR de QGIS. Ce pilote prend WGS84 en silence quand le membre manque ou nomme un code
qu'il ne connaît pas : le geste compare donc le système lu par QGIS au code déclaré, et refuse
toute différence. Un fichier absent, illisible, sans système déclaré ou dont QGIS ne reconnaît
pas le système est une erreur nommée, qui ne cite que le nom du fichier, jamais son chemin (AD-9, AD-21), et
n'ajoute rien au projet. Aucune reprojection (AD-19).

Le travail avance par un ordonnanceur (par défaut `QTimer.singleShot`), jamais par une attente.
`QgisBackend` est le seul endroit qui importe `qgis.core`, les tests mettent une doublure.
"""

import json
import os
import re

try:  # importé à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
except ImportError:  # pragma: no cover
    from gestures import GestureError

INVALID = "layer.invalid_source"
FAILED = "qgis.gesture_failed"
CANCELLED = "qgis.cancelled"

# Forme du membre `crs` qu'écrit l'écrivain du noyau (AD-30).
_DECLARED = re.compile(r"urn:ogc:def:crs:EPSG::([1-9][0-9]*)")


def declared_crs(path):
    """Le système que déclare le GeoJSON (`EPSG:<code>`), ou None s'il ne se lit pas ou n'en
    déclare pas sous la forme de 2008 qu'écrit le noyau."""
    try:
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
    except (OSError, ValueError, RecursionError):
        return None
    crs = data.get("crs") if isinstance(data, dict) else None
    props = crs.get("properties") if isinstance(crs, dict) else None
    name = props.get("name") if isinstance(props, dict) else None
    match = _DECLARED.fullmatch(name) if isinstance(name, str) else None
    return "EPSG:%s" % match.group(1) if match else None


class QgisBackend:
    """Accès réel à QGIS. Seul ce type touche `qgis.core`."""

    def add_vector(self, path, name, expected_crs):
        """Ajoute le GeoJSON au projet ; rend `(layer_id, authid)`, ou None si la couche est
        illisible ou si le système lu n'est pas `expected_crs` (`authid()` vide compris) : rien
        n'est alors ajouté."""
        from qgis.core import QgsCoordinateReferenceSystem, QgsProject, QgsVectorLayer

        layer = QgsVectorLayer(path, name, "ogr")
        if not layer.isValid():
            return None
        if not layer.crs().isValid() and layer.featureCount() == 0:
            # Collection vide (aucun point haut trouvé, par exemple) : QGIS n'en lit pas le
            # système, qu'OGR voit pourtant ; on pose celui que le fichier déclare.
            declared = QgsCoordinateReferenceSystem(expected_crs)
            if declared.isValid():
                layer.setCrs(declared)
        crs = layer.crs()
        authid = crs.authid() if crs.isValid() else ""
        if not authid or authid != expected_crs:
            return None
        QgsProject.instance().addMapLayer(layer)
        return layer.id(), authid


def _singleshot(fn):
    from qgis.PyQt.QtCore import QTimer

    QTimer.singleShot(0, fn)


class LayerOpenVectorGesture:
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
        # Commande non annulable : le noyau ne l'annule pas ; par prudence, l'ordre est soldé.
        self._finish(error=GestureError(CANCELLED, "Geste annulé."))

    def _run(self):
        if self._done is None:  # déjà soldé (annulation)
            return
        try:
            self._finish(result=self._open())
        except GestureError as exc:
            self._finish(error=exc)
        except Exception:  # un ordre sans réponse bloquerait le noyau
            self._finish(error=GestureError(FAILED, "Échec inattendu de l'ouverture du GeoJSON."))

    def _open(self):
        path = (self._params or {}).get("path")
        if not isinstance(path, str) or not path:
            raise GestureError(INVALID, "Aucun GeoJSON à ouvrir.")
        file_name = os.path.basename(path.replace("\\", "/"))
        unreadable = GestureError(
            INVALID,
            "Le GeoJSON « %s » n'est pas lisible ou son système n'est pas reconnu : "
            "il n'est pas ouvert dans le projet." % file_name,
        )
        if not os.path.isfile(path):
            raise unreadable
        expected = declared_crs(path)
        if expected is None:
            raise unreadable
        name = os.path.splitext(file_name)[0]
        added = self._backend.add_vector(path, name, expected)
        if not added:
            raise unreadable
        layer_id, crs = added
        if not layer_id or crs != expected:
            raise unreadable
        return {"layer_id": layer_id, "name": name, "crs": crs}

    def _finish(self, result=None, error=None):
        done, self._done = self._done, None
        if done is not None:
            done(result=result, error=error)
