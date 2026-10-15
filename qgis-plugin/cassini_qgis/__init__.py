"""Extension Cassini pour QGIS (GPL). Point d'entrée."""


def classFactory(iface):
    # Import tardif : le reste du paquet se teste sans QGIS.
    from .plugin import CassiniPlugin

    return CassiniPlugin(iface)
