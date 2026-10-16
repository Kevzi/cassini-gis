"""Exécuteur générique des analyses : une commande du noyau, un algorithme du cadre de
traitement de QGIS.

Le noyau décide (AD-22 : ses commandes ont des noms neutres, jamais `native:buffer`) ;
ce module traduit. La table `ALGORITHMS` est fermée (AD-23) : seul un id de commande
présent dans la table peut lancer un algorithme, jamais un nom venu du modèle. Ajouter
une analyse = une ligne de table (+ sa fonction de traduction) ; `ProcessingGesture`
ne change pas.

Le noyau remplit `out` (AD-10) : le geste écrit exactement ce fichier, ne l'invente pas
et n'en dérive aucun autre. Le travail avance en tâche QGIS annulable
(`QgsProcessingAlgRunnerTask`), jamais en bloquant l'interface. L'entrée est lue seule.

Le fond `QgisBackend` est le seul endroit de ce module qui touche `qgis.core`.
"""

import os

try:  # importés à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
    from .inventory import QgisBackend as _ReadBackend
except ImportError:  # pragma: no cover
    from gestures import GestureError
    from inventory import QgisBackend as _ReadBackend

NOT_FOUND = "layer.not_found"
NOT_VECTOR = "layer.not_vector"
CRS_NOT_METRIC = "analysis.crs_not_metric"
BAD_PARAMS = "analysis.bad_params"
FAILED = "analysis.failed"
CANCELLED = "qgis.cancelled"

_UNIT_TO_METERS = {"m": 1.0, "km": 1000.0}


def _distance_in_meters(distance):
    """Distance `{value, unit}` en mètres ; le noyau a déjà vérifié qu'elle est positive."""
    try:
        meters = float(distance["value"]) * _UNIT_TO_METERS[distance["unit"]]
    except (KeyError, TypeError, ValueError):
        raise GestureError(BAD_PARAMS, "La distance doit être de la forme {value, unit}, en m ou km.")
    if not meters > 0:
        raise GestureError(BAD_PARAMS, "La distance doit être strictement positive.")
    return meters


def translate_buffer(params, layer):
    """Paramètres neutres de `analysis.buffer` -> paramètres de `native:buffer`."""
    return {
        "INPUT": layer,
        "DISTANCE": _distance_in_meters(params.get("distance")),
        "SEGMENTS": 5,
        "END_CAP_STYLE": 0,
        "JOIN_STYLE": 0,
        "MITER_LIMIT": 2,
        "DISSOLVE": bool(params.get("dissolve")),
    }


# Table fermée : id de commande -> algorithme et traduction.
#   algorithm : id de l'algorithme du cadre de traitement ;
#   translate : (params du noyau, couche d'entrée) -> paramètres de l'algorithme, sans la sortie ;
#   output    : nom du paramètre de sortie de l'algorithme ;
#   input     : nom du paramètre du noyau qui désigne la couche d'entrée ;
#   metric    : l'algorithme mesure en unités de la couche, donc le SCR doit être en mètres.
ALGORITHMS = {
    "analysis.buffer": {
        "algorithm": "native:buffer",
        "translate": translate_buffer,
        "output": "OUTPUT",
        "input": "layer_id",
        "metric": True,
    },
}


class QgisBackend(_ReadBackend):
    """Accès réel à QGIS. Seul ce type touche `qgis.core`."""

    def crs_is_metric(self, layer):
        from qgis.core import Qgis

        return layer.crs().mapUnits() == Qgis.DistanceUnit.Meters

    def run_algorithm(self, algorithm_id, parameters, finished):
        """Lance l'algorithme en tâche annulable ; `finished(ok, results)` à la fin.

        Rend un objet à `cancel()`, qui garde la tâche, son contexte et son retour
        référencés tant que le travail dure.
        """
        from qgis.core import (
            QgsApplication,
            QgsProcessingAlgRunnerTask,
            QgsProcessingContext,
            QgsProcessingFeedback,
            QgsProject,
        )

        algorithm = QgsApplication.processingRegistry().algorithmById(algorithm_id)
        if algorithm is None:
            raise GestureError(FAILED, "Le cadre de traitement de QGIS ne fournit pas cet algorithme.")
        context = QgsProcessingContext()
        context.setProject(QgsProject.instance())
        feedback = QgsProcessingFeedback()
        task = QgsProcessingAlgRunnerTask(algorithm, dict(parameters), context, feedback)
        handle = _TaskHandle(task, context, feedback)
        task.executed.connect(lambda ok, results: finished(bool(ok), results or {}))
        QgsApplication.taskManager().addTask(task)
        return handle

    def load_output(self, path, name):
        """Ouvre la couche écrite et l'ajoute au projet ; None si illisible."""
        from qgis.core import QgsProject, QgsVectorLayer

        layer = QgsVectorLayer(path, name, "ogr")
        if not layer.isValid():
            return None
        QgsProject.instance().addMapLayer(layer)
        return layer


class _TaskHandle:
    def __init__(self, task, context, feedback):
        self._task, self._context, self._feedback = task, context, feedback

    def cancel(self):
        self._feedback.cancel()
        self._task.cancel()


def _remove(path):
    """Retire le fichier de sortie et les fichiers annexes d'un GeoPackage."""
    for suffix in ("", "-wal", "-shm", "-journal"):
        try:
            os.remove(path + suffix)
        except OSError:
            pass


def _singleshot(fn):
    from qgis.PyQt.QtCore import QTimer

    QTimer.singleShot(0, fn)


class ProcessingGesture:
    """Geste générique : valide, traduit, lance l'algorithme de la table, charge la sortie."""

    def __init__(self, entry, backend=None, schedule=None):
        self._entry = entry
        self._backend = backend if backend is not None else QgisBackend()
        self._schedule = schedule if schedule is not None else _singleshot
        self._done = None
        self._params = None
        self._out = None
        self._handle = None
        self._loaded = False
        self._cancelling = False

    def start(self, params, done):
        self._done = done
        self._params = params
        self._schedule(self._launch)

    def cancel(self):
        if self._done is None:  # déjà répondu : la sortie livrée n'est plus à nous
            return
        handle = self._handle
        if handle is not None:
            try:
                handle.cancel()
                # La tâche tient encore le fichier : on répond et on nettoie quand elle
                # rend la main (`_on_finished`), pas avant.
                self._cancelling = True
                return
            except Exception:  # l'ordre doit tout de même recevoir sa réponse
                pass
        self._clean()
        self._finish(error=GestureError(CANCELLED, "Geste annulé."))

    def _launch(self):
        if self._done is None:  # déjà soldé (annulation)
            return
        try:
            self._handle = self._begin()
        except GestureError as exc:
            self._clean()
            self._finish(error=exc)
        except Exception:
            self._clean()
            self._finish(error=GestureError(FAILED, "Échec inattendu du lancement de l'analyse."))

    def _begin(self):
        params = self._params or {}
        entry, backend = self._entry, self._backend
        out = params.get("out")
        if not isinstance(out, str) or not out:
            raise GestureError(BAD_PARAMS, "Le noyau n'a pas donné de fichier de sortie.")
        layer_id = params.get(entry["input"])
        layer = backend.layer(layer_id) if isinstance(layer_id, str) and layer_id else None
        if layer is None:
            raise GestureError(NOT_FOUND, "Aucune couche du projet ne porte cet identifiant.")
        if not backend.is_vector(layer):
            raise GestureError(
                NOT_VECTOR, "La couche « %s » n'est pas une couche vecteur." % backend.name_of(layer)
            )
        if entry.get("metric") and not backend.crs_is_metric(layer):
            raise GestureError(
                CRS_NOT_METRIC,
                "La couche « %s » est en %s, dont l'unité n'est pas le mètre : "
                "reprojetez-la d'abord dans un système en mètres (par exemple EPSG:2154)."
                % (backend.name_of(layer), backend.crs_of(layer)),
            )
        algorithm_params = entry["translate"](params, layer)
        algorithm_params[entry["output"]] = out
        self._out = out
        return backend.run_algorithm(entry["algorithm"], algorithm_params, self._on_finished)

    def _on_finished(self, ok, results):
        if self._done is None:  # annulé entre-temps : le geste a déjà répondu
            return
        if self._cancelling:  # la tâche est arrêtée : plus rien ne tient le fichier
            self._clean()
            self._finish(error=GestureError(CANCELLED, "Geste annulé."))
            return
        try:
            if not ok:
                raise GestureError(FAILED, "L'analyse a échoué dans QGIS.")
            self._finish(result=self._load())
        except GestureError as exc:
            self._fail(exc)
        except Exception:
            self._fail(GestureError(FAILED, "Échec inattendu de l'analyse."))

    def _load(self):
        backend = self._backend
        name = os.path.splitext(os.path.basename(self._out))[0]  # radical donné par le noyau (AD-10)
        layer = backend.load_output(self._out, name)
        if layer is None:
            raise GestureError(FAILED, "La couche produite est illisible.")
        self._loaded = True
        return {
            "layer_id": backend.id_of(layer),
            "name": backend.name_of(layer),
            "feature_count": backend.feature_count(layer),
            "crs": backend.crs_of(layer),
        }

    def _fail(self, error):
        # Une couche déjà ajoutée au projet garde son fichier.
        if not self._loaded:
            self._clean()
        self._finish(error=error)

    def _clean(self):
        # L'écriture est celle de ce geste : un échec ne laisse pas de fichier partiel.
        if self._out:
            _remove(self._out)

    def _finish(self, result=None, error=None):
        done, self._done = self._done, None
        if done is not None:
            done(result=result, error=error)


def register_all(registry, table=None, backend_factory=None):
    """Enregistre chaque commande de la table dans le registre des gestes."""
    for command_id, entry in (ALGORITHMS if table is None else table).items():
        registry.register(
            command_id,
            lambda e=entry: ProcessingGesture(e, backend_factory() if backend_factory else None),
        )
