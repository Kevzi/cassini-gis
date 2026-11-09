"""Table fermée de l'extension : les gestes qu'elle sait exécuter (AD-24).

Chaque id enregistré ici est annoncé au noyau à `session.hello`. La recette
(`tests/test_delegated.py`) vérifie que ces ids sont exactement les commandes
déclarées `runs_in: qgis` du registre, ni plus ni moins. Aucun import de QGIS au
chargement : la recette construit la table sans QGIS.
"""

try:  # importés à plat par les tests, en paquet par QGIS
    from . import (
        analysis,
        gestures,
        inventory,
        layer_add,
        layout_export,
        point_transform,
        raster_output,
        style_gestures,
        vector_output,
    )
except ImportError:  # pragma: no cover
    import analysis
    import gestures
    import inventory
    import layer_add
    import layout_export
    import point_transform
    import raster_output
    import style_gestures
    import vector_output

PROBE_ID = "test.qgis_probe"


def build_registry(probe):
    """Rend le registre des gestes ; `probe` est la fabrique du geste de test."""
    registry = gestures.GestureRegistry()
    registry.register(PROBE_ID, probe)
    registry.register("layer.add", layer_add.LayerAddGesture)
    registry.register("layer.open_raster", raster_output.LayerOpenRasterGesture)
    registry.register("layer.open_vector", vector_output.LayerOpenVectorGesture)
    registry.register("project.inventory", inventory.ProjectInventoryGesture)
    registry.register("layer.describe", inventory.LayerDescribeGesture)
    analysis.register_all(registry)
    registry.register("layout.export_pdf", layout_export.LayoutExportGesture)
    registry.register("data.transform_point", point_transform.PointTransformGesture)
    registry.register("layer.set_style", style_gestures.SetStyleGesture)
    # Les formes de style que ce geste sait appliquer, dont `result_ramp` : sans cette annonce,
    # le noyau ouvre ses rasters de résultat sans style (décision du 2026-11-09).
    registry.announce_style_kinds(style_gestures.STYLE_KINDS)
    registry.register("layer.arrange", style_gestures.ArrangeGesture)
    return registry
