"""Geste `layer.remove` : retire une couche du projet QGIS ouvert (epic 10, entrée 27).

Changement de cap du 2026-11-10. Le geste retire la couche du projet par
`QgsProject.removeMapLayer`, et rien d'autre : son fichier n'est ni effacé, ni déplacé, ni
modifié (principe 6). Une couche déjà absente du projet n'est pas une erreur : le résultat est
le même, `{"layer_id"}`, ce qui garde identique le rejeu dans la même session (AD-11). Aucune
décision ici : la couche est celle que le noyau dicte (AD-9).

Une couche que le retrait détruirait sans recours est refusée par `layer.unsaved`, avant tout
retrait (changement de cap du 2026-11-10, `change-refus-retrait-couche-non-enregistree`) : une
couche en mémoire, sans fichier, et une couche en édition avec des modifications non
enregistrées. La couche reste alors au projet, intacte ; le message dit quoi faire.

Le nom de la couche retirée est gardé dans une mémoire d'affichage (`removed_name`), remplie
avant `removeMapLayer` : le panneau la lit pour montrer « Routes » plutôt que l'identifiant,
la couche n'étant plus au projet. Elle ne sert qu'à l'affichage ; le résultat ne porte que
`layer_id` (fiche du catalogue).

Le déroulé (ordonnanceur, annulation, ordre soldé une fois) est celui d'`open_output.py`.
`QgisBackend` est le seul endroit qui importe `qgis.core`, les tests mettent une doublure.
"""

try:  # importé à plat par les tests, en paquet par QGIS
    from .gestures import GestureError
    from .open_output import FAILED, OpenOutputGesture
except ImportError:  # pragma: no cover
    from gestures import GestureError
    from open_output import FAILED, OpenOutputGesture

UNSAVED = "layer.unsaved"

# Noms des couches retirées par ce geste, par identifiant : affichage seul.
_REMOVED_NAMES = {}

_UNSAVED_MESSAGES = {
    "memory": (
        "La couche « %s » est une couche temporaire, sans fichier : la retirer du projet "
        "perdrait tout son contenu. Rendez-la d'abord permanente en l'enregistrant dans un fichier "
        "(icône à côté de son nom, dans le panneau des couches), puis demandez de nouveau son "
        "retrait."
    ),
    "edits": (
        "La couche « %s » a des modifications non enregistrées : la retirer du projet les "
        "perdrait. Enregistrez ou annulez d'abord ses modifications, puis demandez de nouveau "
        "son retrait."
    ),
}


def removed_name(layer_id):
    """Nom qu'avait la couche `layer_id` quand ce geste l'a retirée du projet, ou None."""
    return _REMOVED_NAMES.get(layer_id)


def display_name(layer_id, live_name):
    """Nom affiché d'une couche : celui que rend `live_name(layer_id)` (QGIS), sinon celui
    qu'avait la couche quand ce geste l'a retirée, sinon None. Jamais d'exception."""
    try:
        name = live_name(layer_id)
    except Exception:
        name = None
    return name if name is not None else removed_name(layer_id)


class QgisBackend:
    """Accès réel à QGIS. Seul ce type touche `qgis.core`, et seulement dans ses méthodes."""

    def layer(self, layer_id):
        from qgis.core import QgsProject

        return QgsProject.instance().mapLayer(layer_id)

    def name_of(self, layer):
        return layer.name()

    def unsaved(self, layer):
        """Ce que le retrait détruirait : "memory" (couche en mémoire, sans fichier), "edits"
        (modifications non enregistrées), ou None. Un raster n'a pas toujours `isModified`."""
        if layer.providerType() == "memory":
            return "memory"
        editable = getattr(layer, "isEditable", None)
        modified = getattr(layer, "isModified", None)
        if editable is not None and modified is not None and editable() and modified():
            return "edits"
        return None

    def remove(self, layer_id):
        """Retire la couche du projet ; le fichier de données n'est pas touché."""
        from qgis.core import QgsProject

        QgsProject.instance().removeMapLayer(layer_id)


class LayerRemoveGesture(OpenOutputGesture):
    """`layer.remove` : retire une couche du projet ; rend `{"layer_id"}`."""

    UNEXPECTED = "Échec inattendu du retrait de la couche."

    def __init__(self, backend=None, schedule=None):
        super().__init__(backend if backend is not None else QgisBackend(), schedule)

    def _open(self):
        layer_id = (self._params or {}).get("layer_id")
        if not isinstance(layer_id, str):
            # Le noyau valide la fiche avant de dicter : ce cas ne vient que d'un ordre mal formé.
            # Toute chaîne, même vide, est une recherche ordinaire (absente : même résultat).
            raise GestureError(FAILED, "Aucune couche à retirer.")
        backend = self._backend
        layer = backend.layer(layer_id) if layer_id else None
        if layer is not None:
            name = backend.name_of(layer)
            lost = backend.unsaved(layer)
            if lost is not None:
                # Refus avant tout retrait : la couche reste au projet, intacte.
                raise GestureError(UNSAVED, _UNSAVED_MESSAGES[lost] % name)
            _REMOVED_NAMES[layer_id] = name
            backend.remove(layer_id)
        # Déjà absente : même résultat, sans erreur.
        return {"layer_id": layer_id}
