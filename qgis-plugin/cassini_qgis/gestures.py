"""Registre des gestes QGIS et contrat d'un geste asynchrone.

Un geste est un objet qui possède :

- `start(params, done)` : démarre le travail et rend la main aussitôt. Le travail
  avance par la boucle d'événements de Qt (QTimer, QgsTask), jamais par une
  attente. Quand il est fini, le geste appelle `done(result=<valeur>)` ou
  `done(error=GestureError(...))`, une seule fois ;
- `cancel()` : demande l'arrêt du travail. Le geste doit tout de même rappeler
  `done` (par n'importe quelle réponse) pour que l'ordre reçoive sa réponse.

Le registre associe un id de commande à une fabrique qui rend un geste neuf.
Aucune logique métier ici : le noyau décide, le geste exécute.
"""


class GestureError(Exception):
    """Échec d'un geste : `code` et `message` forment l'ErrorReport du canal."""

    def __init__(self, code, message):
        super().__init__(message)
        self.code = code
        self.message = message


class GestureRegistry:
    def __init__(self):
        self._factories = {}
        self._style_kinds = set()

    def register(self, command_id, factory):
        self._factories[command_id] = factory

    def ids(self):
        return sorted(self._factories)

    def announce_style_kinds(self, kinds):
        """Note les formes de style que le geste `layer.set_style` sait appliquer."""
        self._style_kinds.update(kinds)

    def style_kinds(self):
        """Formes de style annoncées à `session.hello` (`style_kinds`), triées."""
        return sorted(self._style_kinds)

    def create(self, command_id):
        """Rend un geste neuf, ou None si l'id n'est pas enregistré."""
        factory = self._factories.get(command_id)
        return factory() if factory is not None else None
