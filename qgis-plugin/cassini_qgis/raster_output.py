"""Geste `layer.open_raster` : ouvre dans le projet QGIS un raster écrit par le noyau (ticket 4.9).

Le noyau dicte ce geste lui-même, juste après une commande du noyau qui écrit un GeoTIFF
(`terrain.slope`, `terrain.viewshed`), et le journalise avec le `layer_id` rendu : le rejeu le
rejoue et sait laisser la couche d'origine hors de la carte. L'extension n'ouvre plus aucun
raster d'elle-même (le branchement du ticket 4.3 est retiré).

Aucune décision ici : le chemin absolu est celui que le noyau donne (AD-9, AD-10), la couche
est nommée du radical du fichier (`1-terrain.slope`). Chaque ordre ajoute une couche neuve,
même si le fichier est déjà ouvert : un second rejeu qui réécrit le même fichier de `replay/`
doit obtenir une couche à lui, que le rejeu suivant pourra laisser hors de la carte, et son nom
ne dépend jamais d'un renommage fait à la main. Un fichier absent ou illisible est une erreur nommée, qui ne
cite que le nom du fichier, jamais son chemin (AD-9, AD-21).

Le travail avance par un ordonnanceur (par défaut `QTimer.singleShot`), jamais par une attente.
`QgisBackend` est le seul endroit qui importe `qgis.core`, les tests mettent une doublure.
"""

import os

try:  # importé à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
except ImportError:  # pragma: no cover
    from gestures import GestureError

INVALID = "layer.invalid_source"
FAILED = "qgis.gesture_failed"
CANCELLED = "qgis.cancelled"


class QgisBackend:
    """Accès réel à QGIS. Seul ce type touche `qgis.core`."""

    def add_raster(self, path, name):
        """Ajoute le raster au projet ; rend son id, ou None s'il est illisible."""
        from qgis.core import QgsProject, QgsRasterLayer

        layer = QgsRasterLayer(path, name, "gdal")
        if not layer.isValid():
            return None
        QgsProject.instance().addMapLayer(layer)
        return layer.id()


def _singleshot(fn):
    from qgis.PyQt.QtCore import QTimer

    QTimer.singleShot(0, fn)


class LayerOpenRasterGesture:
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
            self._finish(error=GestureError(FAILED, "Échec inattendu de l'ouverture du raster."))

    def _open(self):
        path = (self._params or {}).get("path")
        if not isinstance(path, str) or not path:
            raise GestureError(INVALID, "Aucun raster à ouvrir.")
        file_name = os.path.basename(path.replace("\\", "/"))
        unreadable = GestureError(
            INVALID, "Le raster « %s » n'est pas lisible : il n'est pas ouvert dans le projet." % file_name
        )
        if not os.path.isfile(path):
            raise unreadable
        name = os.path.splitext(file_name)[0]
        layer_id = self._backend.add_raster(path, name)
        if not layer_id:
            raise unreadable
        return {"layer_id": layer_id, "name": name}

    def _finish(self, result=None, error=None):
        done, self._done = self._done, None
        if done is not None:
            done(result=result, error=error)
