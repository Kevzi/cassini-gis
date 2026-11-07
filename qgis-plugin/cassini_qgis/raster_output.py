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

Le déroulé (ordonnanceur, annulation, ordre soldé une fois) est celui d'`open_output.py`.
`QgisBackend` est le seul endroit qui importe `qgis.core`, les tests mettent une doublure.
"""

import os

try:  # importé à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
    from .open_output import INVALID, OpenOutputGesture
except ImportError:  # pragma: no cover
    from gestures import GestureError
    from open_output import INVALID, OpenOutputGesture


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


class LayerOpenRasterGesture(OpenOutputGesture):
    UNEXPECTED = "Échec inattendu de l'ouverture du raster."

    def __init__(self, backend=None, schedule=None):
        super().__init__(backend if backend is not None else QgisBackend(), schedule)

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
