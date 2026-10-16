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
UNKNOWN_FIELD = "analysis.unknown_field"
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


def translate_intersect(params, layer):
    """Paramètres neutres de `analysis.intersect` -> paramètres de `native:intersection`.

    La couche de recouvrement est ajoutée par le geste (clé `overlay` de la table).
    """
    return {
        "INPUT": layer,
        "INPUT_FIELDS": [],  # liste vide : tous les champs
        "OVERLAY_FIELDS": [],
        "OVERLAY_FIELDS_PREFIX": "",
    }


# Opérateur du noyau -> code de `native:extractbyattribute` (jamais d'expression).
_OPERATOR_CODES = {
    "eq": 0,
    "ne": 1,
    "gt": 2,
    "gte": 3,
    "lt": 4,
    "lte": 5,
    "begins_with": 6,
    "contains": 7,
}


def translate_select_by_attribute(params, layer):
    """Champ, opérateur et valeur -> paramètres séparés de `native:extractbyattribute`.

    Le texte de l'utilisateur n'entre jamais dans une expression : QGIS reçoit le nom du
    champ, le code de l'opérateur et la valeur comme trois paramètres distincts.
    """
    operator = params.get("operator")
    value = params.get("value")
    if operator not in _OPERATOR_CODES or not isinstance(value, str):
        raise GestureError(BAD_PARAMS, "Le filtre doit avoir un opérateur connu et une valeur en texte.")
    return {
        "INPUT": layer,
        "FIELD": params.get("field"),
        "OPERATOR": _OPERATOR_CODES[operator],
        "VALUE": value,
    }


# Champ de décompte écrit par `native:countpointsinpolygon` ; le noyau ne le connaît pas.
_COUNT_FIELD = "NUMPOINTS"
_MAX_ZONES = 100  # borne du schéma de sortie du noyau (AD-8)


def translate_zonal_statistics(params, layer):
    """Paramètres neutres de `analysis.zonal_statistics` -> `native:countpointsinpolygon`.

    La couche de points est ajoutée par le geste (clé `overlay` de la table).
    """
    return {"POLYGONS": layer, "WEIGHT": "", "CLASSFIELD": "", "FIELD": _COUNT_FIELD}


def summarize_zonal_statistics(backend, layer, params):
    """Décomptes des 100 premières zones de la couche produite (clé `summary` de la table)."""
    pairs, truncated = backend.counts_of(layer, _COUNT_FIELD, params.get("label_field") or None, _MAX_ZONES)
    return {"zones": [{"label": label, "count": count} for label, count in pairs], "truncated": truncated}


# Table fermée : id de commande -> algorithme et traduction.
#   algorithm : id de l'algorithme du cadre de traitement ;
#   translate : (params du noyau, couche d'entrée) -> paramètres de l'algorithme, sans la sortie ;
#   output    : nom du paramètre de sortie de l'algorithme ;
#   input     : nom du paramètre du noyau qui désigne la couche d'entrée ;
#   metric    : l'algorithme mesure en unités de la couche, donc le SCR doit être en mètres ;
#   overlay   : (facultatif) paramètre du noyau qui désigne une seconde couche, et clé de
#               l'algorithme qui la reçoit (`overlay_key`) ;
#   field     : (facultatif) paramètre du noyau qui nomme un champ de la couche d'entrée ;
#               avec `field_optional`, il peut être absent ;
#   geometry  : (facultatif) type de géométrie exigé de la couche d'entrée (ex. `Polygon`) ;
#   summary   : (facultatif) (fond, couche produite, params) -> dict qui complète le résultat.
ALGORITHMS = {
    "analysis.buffer": {
        "algorithm": "native:buffer",
        "translate": translate_buffer,
        "output": "OUTPUT",
        "input": "layer_id",
        "metric": True,
    },
    "analysis.intersect": {
        "algorithm": "native:intersection",
        "translate": translate_intersect,
        "output": "OUTPUT",
        "input": "layer_id",
        "overlay": "overlay_layer_id",
        "overlay_key": "OVERLAY",
    },
    "analysis.select_by_attribute": {
        "algorithm": "native:extractbyattribute",
        "translate": translate_select_by_attribute,
        "output": "OUTPUT",
        "input": "layer_id",
        "field": "field",
    },
    "analysis.zonal_statistics": {
        "algorithm": "native:countpointsinpolygon",
        "translate": translate_zonal_statistics,
        "output": "OUTPUT",
        "input": "layer_id",
        "overlay": "points_layer_id",
        "overlay_key": "POINTS",
        "field": "label_field",
        "field_optional": True,
        "geometry": "Polygon",
        "summary": summarize_zonal_statistics,
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

    def counts_of(self, layer, count_field, label_field, limit):
        """Couples (étiquette, décompte) des `limit` premières entités, et si la couche en a plus.

        L'étiquette est la valeur de `label_field`, ou le numéro d'entité si le champ est
        absent, vide ou nul.
        """
        from qgis.core import QgsFeatureRequest

        request = QgsFeatureRequest()
        request.setLimit(limit)
        pairs = []
        for feature in layer.getFeatures(request):
            label = feature[label_field] if label_field else None
            text = "" if label is None else str(label).strip()
            if text in ("", "NULL"):
                text = str(feature.id())
            count = feature[count_field]
            pairs.append((text, int(count) if count is not None else 0))
        return pairs, self.feature_count(layer) > limit

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
        if entry.get("geometry") and backend.geometry_of(layer) != entry["geometry"]:
            raise GestureError(
                BAD_PARAMS,
                "La couche « %s » n'est pas une couche de polygones." % backend.name_of(layer),
            )
        overlay = None
        if entry.get("overlay"):
            overlay = self._overlay(params, layer_id)
        if entry.get("field") and not (entry.get("field_optional") and not params.get(entry["field"])):
            self._check_field(params.get(entry["field"]), layer)
        algorithm_params = entry["translate"](params, layer)
        if overlay is not None:
            algorithm_params[entry["overlay_key"]] = overlay
        algorithm_params[entry["output"]] = out
        self._out = out
        return backend.run_algorithm(entry["algorithm"], algorithm_params, self._on_finished)

    def _overlay(self, params, layer_id):
        """Seconde couche vecteur, distincte de l'entrée."""
        backend = self._backend
        overlay_id = params.get(self._entry["overlay"])
        if overlay_id == layer_id:
            raise GestureError(BAD_PARAMS, "Les deux couches doivent être différentes.")
        overlay = backend.layer(overlay_id) if isinstance(overlay_id, str) and overlay_id else None
        if overlay is None:
            raise GestureError(NOT_FOUND, "Aucune couche du projet ne porte cet identifiant.")
        if not backend.is_vector(overlay):
            raise GestureError(
                NOT_VECTOR, "La couche « %s » n'est pas une couche vecteur." % backend.name_of(overlay)
            )
        return overlay

    def _check_field(self, name, layer):
        backend = self._backend
        if not isinstance(name, str) or name not in [n for n, _ in backend.fields_of(layer)]:
            raise GestureError(
                UNKNOWN_FIELD,
                "La couche « %s » n'a pas de champ « %s »." % (backend.name_of(layer), name),
            )

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
        result = {
            "layer_id": backend.id_of(layer),
            "name": backend.name_of(layer),
            "feature_count": backend.feature_count(layer),
            "crs": backend.crs_of(layer),
        }
        summary = self._entry.get("summary")
        if summary is not None:
            result.update(summary(backend, layer, self._params or {}))
        return result

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
