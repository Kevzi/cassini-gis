"""Geste `data.transform_point` : reprojette un seul point vers un code EPSG, par QGIS.

Le noyau décide (point et code cible déjà validés par son schéma) ; ce geste exécute
seulement : il fait vérifier par QGIS les deux codes EPSG, puis reprojette le point par
`QgsCoordinateTransform`, dans le contexte de transformation du projet. Aucune formule de
projection ici, aucune couche créée, aucun fichier écrit (AD-19 : la reprojection est celle
de QGIS).

Le travail avance par un ordonnanceur (par défaut `QTimer.singleShot`), jamais par une
attente. L'accès à QGIS passe par un « fond » injectable : `QgisBackend` est le seul endroit
qui importe `qgis.core`, les tests mettent une doublure.
"""

import math

try:  # `gestures` est importé à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
except ImportError:  # pragma: no cover
    from gestures import GestureError

UNKNOWN_CRS = "data.unknown_crs"
TRANSFORM_FAILED = "data.transform_failed"
CANCELLED = "qgis.cancelled"
FAILED = "qgis.gesture_failed"


class TransformError(Exception):
    """Levée par le fond quand QGIS ne sait pas reprojeter le point."""


class QgisBackend:
    """Accès réel à QGIS. Seul ce type touche `qgis.core`."""

    def knows(self, epsg):
        """Vrai si QGIS connaît le code EPSG `epsg`."""
        from qgis.core import QgsCoordinateReferenceSystem

        return QgsCoordinateReferenceSystem("EPSG:%d" % epsg).isValid()

    def transform(self, x, y, source_epsg, target_epsg):
        """(x, y) du système `source_epsg` dans le système `target_epsg`, par QGIS ; lève
        `TransformError` si QGIS ne sait pas le reprojeter (point hors du domaine)."""
        from qgis.core import (
            QgsCoordinateReferenceSystem,
            QgsCoordinateTransform,
            QgsPointXY,
            QgsProject,
        )

        source = QgsCoordinateReferenceSystem("EPSG:%d" % source_epsg)
        target = QgsCoordinateReferenceSystem("EPSG:%d" % target_epsg)
        try:
            point = QgsCoordinateTransform(source, target, QgsProject.instance()).transform(
                QgsPointXY(x, y)
            )
        except Exception as exc:  # QgsCsException : point hors du domaine du système
            raise TransformError(str(exc))
        return (point.x(), point.y())


def _singleshot(fn):
    from qgis.PyQt.QtCore import QTimer

    QTimer.singleShot(0, fn)


def _is_code(value):
    return isinstance(value, int) and not isinstance(value, bool) and value >= 0


def _is_number(value):
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


class PointTransformGesture:
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
            self._finish(result=self._transform())
        except GestureError as exc:
            self._finish(error=exc)
        except Exception:  # un ordre sans réponse bloquerait le noyau
            self._finish(error=GestureError(FAILED, "Échec inattendu de la reprojection du point."))

    def _transform(self):
        params = self._params or {}
        point = params.get("point")
        target = params.get("target_epsg")
        # Le schéma du noyau a déjà validé ces champs ; une forme inattendue reste un échec
        # nommé, jamais une exception.
        if not isinstance(point, dict) or not _is_code(point.get("epsg")) or not _is_code(target):
            raise GestureError(FAILED, "Point ou code EPSG illisible.")
        x, y, source = point.get("x"), point.get("y"), point["epsg"]
        if not (_is_number(x) and _is_number(y)):
            raise GestureError(TRANSFORM_FAILED, "Coordonnées du point non finies.")
        for code in (source, target):
            if not self._backend.knows(code):
                raise GestureError(UNKNOWN_CRS, "Système inconnu de QGIS : EPSG:%d." % code)
        if self._cancelled:
            raise GestureError(CANCELLED, "Geste annulé.")
        if source == target:
            return {"point": {"x": x, "y": y, "epsg": target}}
        try:
            nx, ny = self._backend.transform(x, y, source, target)
        except TransformError:
            nx = ny = None
        if not (_is_number(nx) and _is_number(ny)):
            raise GestureError(
                TRANSFORM_FAILED,
                "QGIS ne sait pas reprojeter ce point de EPSG:%d en EPSG:%d "
                "(hors du domaine du système)." % (source, target),
            )
        return {"point": {"x": nx, "y": ny, "epsg": target}}

    def _finish(self, result=None, error=None):
        done, self._done = self._done, None
        if done is not None:
            done(result=result, error=error)
