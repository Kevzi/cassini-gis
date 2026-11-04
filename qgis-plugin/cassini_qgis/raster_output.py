"""Ouverture dans le projet QGIS des rasters écrits par le noyau (ticket 4.3).

Quand une commande exécutée par le noyau (`runs_in: core`, comme `terrain.slope`) rend un
`file` GeoTIFF, l'extension l'ajoute au projet comme couche raster, nommée du radical du
fichier que le noyau a choisi (`1-terrain.slope`), comme une sortie déléguée à QGIS (AD-10).
Le résultat arrive par `command.run`, `plan.execute` ou un appel de l'IA externe ; le noyau y
donne le chemin absolu (AD-9, décision du 2026-11-04).

Aucune décision ici : l'extension ouvre ce que le noyau nomme, rien d'autre. L'ouverture n'est
pas journalisée, c'est un affichage. `QgisBackend` est le seul endroit qui importe
`qgis.core`, les tests mettent une doublure.
"""

import os

try:  # importé à plat par les tests, en paquet par QGIS
    from . import client, runlog, texts
except ImportError:  # pragma: no cover
    import client
    import runlog
    import texts

RASTER_EXTENSIONS = (".tif", ".tiff")

# Issues de l'ouverture d'un raster.
ADDED = "added"
ALREADY_OPEN = "already_open"
UNREADABLE = "unreadable"


def _runs_in_core(command, catalogue):
    for entry in catalogue or []:
        if isinstance(entry, dict) and entry.get("id") == command:
            return entry.get("runs_in") == "core"
    return False


def rasters_to_open(command, result, catalogue):
    """`[(chemin, nom)]` du raster à ouvrir pour ce résultat ; `[]` sinon.

    Seulement pour une commande du noyau au catalogue, dont le `file` est un chemin absolu
    d'extension `.tif`/`.tiff`. Le nom est le radical du fichier. Un fichier absent est rendu
    quand même : `open_rasters` le signale, `report` le porte au journal du panneau.
    """
    if not _runs_in_core(command, catalogue) or not isinstance(result, dict):
        return []
    path = result.get("file")
    if not isinstance(path, str) or not path or not os.path.isabs(path):
        return []
    if os.path.splitext(path)[1].lower() not in RASTER_EXTENSIONS:
        return []
    return [(path, os.path.splitext(os.path.basename(path))[0])]


def outputs_of(kind, payload):
    """`[(commande, résultat)]` portés par un événement du client.

    `RUN_DONE` : la commande lancée à la main ; `PLAN_DONE` : chaque étape du plan ;
    `EXTERNAL_CALL_FINISHED` sans erreur : l'appel de l'IA externe. Tout autre événement : `[]`.
    """
    if not isinstance(payload, dict):
        return []
    if kind in (client.RUN_DONE, client.EXTERNAL_CALL_FINISHED):
        if kind == client.EXTERNAL_CALL_FINISHED and payload.get("error"):
            return []
        return [(payload.get("command"), payload.get("result"))]
    if kind == client.PLAN_DONE:
        steps = payload.get("steps")
        return [
            (step.get("command"), step.get("result"))
            for step in steps if isinstance(step, dict)
        ] if isinstance(steps, list) else []
    return []


def to_open(kind, payload, catalogue):
    """Les rasters du noyau à ouvrir pour cet événement, dans l'ordre, sans doublon."""
    found = []
    for command, result in outputs_of(kind, payload):
        for item in rasters_to_open(command, result, catalogue):
            if item not in found:
                found.append(item)
    return found


def open_rasters(items, backend=None):
    """Ouvre chaque `(chemin, nom)` ; rend `[(chemin, issue)]`. Ne lève jamais.

    Un fichier déjà ouvert dans le projet n'est pas ajouté une seconde fois ; un fichier absent
    ou illisible est rendu `UNREADABLE`, que `report` porte au journal du panneau.
    """
    backend = backend if backend is not None else QgisBackend()
    outcomes = []
    for path, name in items:
        try:
            if not os.path.isfile(path):
                outcome = UNREADABLE
            elif backend.is_open(path):
                outcome = ALREADY_OPEN
            elif backend.add_raster(path, name):
                outcome = ADDED
            else:
                outcome = UNREADABLE
        except Exception:  # un affichage raté ne doit jamais faire tomber le panneau
            outcome = UNREADABLE
        outcomes.append((path, outcome))
    return outcomes


# Commande de la ligne du journal d'un raster non ouvert : un affichage de l'extension, pas une
# commande du registre.
RASTER_OPEN = "raster.open"


def report(log, outcomes, lang, when):
    """Une ligne en erreur au journal du panneau par raster non ouvert ; rend ces lignes.

    La ligne ne nomme que le fichier, jamais son chemin (AD-9).
    """
    entries = []
    for path, outcome in outcomes:
        if outcome != UNREADABLE:
            continue
        name = os.path.basename(path.replace("\\", "/"))
        entry = log.start(RASTER_OPEN, texts.tr(lang, "raster_unreadable", name=name), when)
        entry.status = runlog.ERROR
        entries.append(entry)
    return entries


def same_file(a, b):
    """Vrai si deux chemins désignent le même fichier (casse et séparateurs normalisés)."""
    if not isinstance(a, str) or not isinstance(b, str) or not a or not b:
        return False
    return _normalized(a) == _normalized(b)


def _normalized(path):
    return os.path.normcase(os.path.normpath(path.split("|", 1)[0].replace("\\", "/")))


class QgisBackend:
    """Accès réel à QGIS. Seul ce type touche `qgis.core`."""

    def is_open(self, path):
        from qgis.core import QgsProject

        return any(
            same_file(layer.source(), path)
            for layer in QgsProject.instance().mapLayers().values()
        )

    def add_raster(self, path, name):
        """Ajoute le raster au projet ; faux s'il est illisible."""
        from qgis.core import QgsProject, QgsRasterLayer

        layer = QgsRasterLayer(path, name, "gdal")
        if not layer.isValid():
            return False
        QgsProject.instance().addMapLayer(layer)
        return True
