"""Socle commun des gestes qui ouvrent dans le projet QGIS un fichier écrit par le noyau :
`layer.open_raster` (`raster_output.py`) et `layer.open_vector` (`vector_output.py`).

Le socle porte le déroulé, identique pour les deux : l'ordre est reçu, le travail avance par un
ordonnanceur (par défaut `QTimer.singleShot`), jamais par une attente, et chaque ordre est soldé
une fois et une seule, par un résultat ou une erreur nommée. Un geste ne fournit que `_open`,
qui ouvre le fichier et rend le résultat, et `UNEXPECTED`, le message d'un échec imprévu.
Aucune décision ici.
"""

try:  # importé à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
except ImportError:  # pragma: no cover
    from gestures import GestureError

INVALID = "layer.invalid_source"
FAILED = "qgis.gesture_failed"
CANCELLED = "qgis.cancelled"


def _singleshot(fn):
    from qgis.PyQt.QtCore import QTimer

    QTimer.singleShot(0, fn)


class OpenOutputGesture:
    """Déroulé d'une ouverture dictée ; `backend` est le seul accès à QGIS, doublé en test."""

    UNEXPECTED = "Échec inattendu de l'ouverture."

    def __init__(self, backend, schedule=None):
        self._backend = backend
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
            self._finish(error=GestureError(FAILED, self.UNEXPECTED))

    def _open(self):  # pragma: no cover - fourni par chaque geste
        raise NotImplementedError

    def _finish(self, result=None, error=None):
        done, self._done = self._done, None
        if done is not None:
            done(result=result, error=error)
