"""Sonde manuelle sous le vrai QGIS et le vrai noyau : mise en forme topographique.

Ticket 11.21. À lancer avec le Python de QGIS (pas par `unittest` : le nom ne commence pas
par `test`), avec `QT_QPA_PLATFORM=windows` (sous `offscreen`, les textes du PDF sortent en
pavés noirs) :

    set QT_QPA_PLATFORM=windows
    set CASSINI_CORE=C:\\ct-wt\\release\\cassini.exe
    "C:\\Program Files\\QGIS 3.44.15\\bin\\python-qgis-ltr.bat" qgis-plugin/tests/probe_style.py

Le noyau est cherché dans `CASSINI_CORE`, sinon `target/release/`, sinon `target/debug/`.
Elle demande `../donnees-test-cassini/` avec les deux projets de test, Villeneuve-le-Roi et
Annecy (hors dépôt, voir CLAUDE.md §5). Pour chacun :

1. le projet est lu, puis enregistré en copie, chemins absolus, dans un dossier temporaire :
   le journal s'écrit à côté de la copie, l'original et ses données ne sont jamais réécrits ;
2. par le canal, avec les gestes de l'extension : `map.topographic_style`,
   `layout.export_pdf`, puis `map.graduated_style` sur un champ numérique d'une couche de
   surface et un second `layout.export_pdf` (la carte topographique et le dégradé ont chacun
   leur PDF). Chaque couche reconnue a pris le rendu de son `StyleSpec` ; pour une couche de
   chaque forme, les couleurs du rendu sont celles dictées (catégorie « autres valeurs »
   comprise, fondu « produit » et opacité de l'ombrage) ; les groupes de l'arbre sont ceux du
   résultat, dans l'ordre, « Relief » en dessous ; le dégradé a ses classes, calculées par
   QGIS ; le bâti et les parcelles ne sont dessinés qu'à partir du 1:100 000 (`max_scale`),
   et une classe à seuil propre (les petites routes) suit l'échelle de la carte exportée ;
3. le projet stylé est enregistré, rouvert dans une session neuve et exporté sans nouveau
   `layer.set_style` : la légende garde « Altitude (m) », ses bornes réelles et « Ombrage » ;
4. la copie est fermée puis rouverte (sans styles : rien n'est enregistré), un nouveau noyau
   rejoue le journal : `identical: true`, les mêmes rendus et les mêmes groupes reviennent,
   et les PDF rejoués commencent par `%PDF`.

À la fin, aucun fichier de `../donnees-test-cassini/` n'a changé (date et taille), et la
légende de chaque PDF, relevée à l'export, reste au-dessus des sources. Le
dossier temporaire est gardé : ses PDF sont à rendre en PNG et à regarder à l'œil (chemins
imprimés).
"""

import json
import os
import sys
import tempfile
import traceback

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "qgis-plugin", "cassini_qgis"))
DATA = os.path.normpath(os.path.join(ROOT, "..", "donnees-test-cassini"))
# (nom, projet, candidats (couche, champ) du dégradé, dans l'ordre de préférence)
SETS = [
    (
        "villeneuve",
        os.path.join(DATA, "projet-test-villeneuve.qgz"),
        [("parcelles", "contenance")],
    ),
    (
        "annecy",
        os.path.join(DATA, "annecy", "projet-test-annecy.qgz"),
        [("communes", "population")],
    ),
]

from qgis.analysis import QgsNativeAlgorithms  # noqa: E402
from qgis.core import QgsApplication, QgsLayerTree, QgsProject, QgsVectorLayer  # noqa: E402
from qgis.core import Qgis, QgsLayoutExporter, QgsLayoutItemLegend, QgsLayoutItemMap  # noqa: E402
from qgis.core import QgsRasterLayer  # noqa: E402
from qgis.PyQt.QtCore import QCoreApplication, QProcess, Qt, QTimer  # noqa: E402
from qgis.PyQt.QtGui import QPainter  # noqa: E402

app = QgsApplication([], False)
QgsApplication.initQgis()
QgsApplication.processingRegistry().addProvider(QgsNativeAlgorithms())
import client  # noqa: E402
import delegated  # noqa: E402

# Rendu attendu de QGIS pour chaque forme de `StyleSpec`.
RENDERERS = {
    "single": "singleSymbol",
    "categories": "categorizedSymbol",
    "graduated": "graduatedSymbol",
    "elevation_ramp": "singlebandpseudocolor",
    "hillshade": "singlebandgray",
}
# Vérifications faites pour chaque jeu, hors celles qui se comptent : une par PDF à l'exécution
# et au rejeu, une par forme de style présente à l'exécution et au rejeu.
FIXED_RUN = 11  # lecture, copie, reconnues, journal sans erreur, lignes, rendus, groupes,
# Relief en dessous, classes du dégradé, classes à seuil selon l'échelle, projet rouvert
FIXED_REPLAY = 6  # rouverture, départ, identical, journal inchangé, rendus, groupes


def find_core():
    candidates = [os.environ.get("CASSINI_CORE")]
    for profile in ("release", "debug"):
        candidates.append(os.path.join(ROOT, "target", profile, "cassini.exe"))
        candidates.append(os.path.join(ROOT, "target", profile, "cassini"))
    for path in candidates:
        if path and os.path.isfile(path):
            return path
    sys.exit("Noyau introuvable : lancez `cargo build --release`.")


def stamps(top):
    """{chemin relatif: (date, taille)} de chaque fichier sous `top`."""
    out = {}
    for base, _, files in os.walk(top):
        for name in files:
            path = os.path.join(base, name)
            st = os.stat(path)
            out[os.path.relpath(path, top)] = (st.st_mtime_ns, st.st_size)
    return out


# Bas de la légende de chaque PDF exporté, relevé au moment de l'export (exécution et rejeu).
legend_bottoms = []
map_scales = []  # échelle de la carte principale de chaque PDF exporté
_export_to_pdf = QgsLayoutExporter.exportToPdf


legend_rasters = []  # pour chaque export : {couche: ce que sa légende montre} des rasters


def raster_legend_of(legend):
    """{nom de couche: (textes des nœuds, libellés et couleur haute de la barre)} des rasters de
    la légende de mise en page, tels que QGIS les a dessinés."""
    import style_gestures

    model = legend.model()
    out = {}
    for node in model.rootGroup().findLayers():
        layer = node.layer()
        if not isinstance(layer, QgsRasterLayer):
            continue
        nodes = model.layerLegendNodes(node)
        texts = [n.data(Qt.ItemDataRole.DisplayRole) for n in nodes]
        bar = None
        for n in nodes:
            if hasattr(n, "ramp") and hasattr(n, "settings"):
                settings = n.settings()
                bar = (settings.minimumLabel(), settings.maximumLabel(), n.ramp().color2().name())
        out[layer.name()] = (texts, bar, layer.customProperty(style_gestures.ELEVATION_SPAN))
    return out


def _spy_export(self, *args, **kwargs):
    """Relève le bas de la légende après l'export : QGIS la redimensionne en la dessinant."""
    result = _export_to_pdf(self, *args, **kwargs)
    page_height = self.layout().pageCollection().page(0).pageSize().height()
    import layout_export

    limit = layout_export.LEGEND_BOTTOM * page_height / layout_export.BASE_PAGE[1]
    scales = []
    for item in self.layout().items():
        if isinstance(item, QgsLayoutItemLegend):
            bottom = item.positionWithUnits().y() + item.sizeWithUnits().height()
            legend_bottoms.append((round(bottom, 1), round(limit, 1)))
            legend_rasters.append(raster_legend_of(item))
        if isinstance(item, QgsLayoutItemMap) and item.layers() and item.hasBackground():
            scales.append(item.scale())  # la carte du carroyage n'a pas de fond
    map_scales.append(scales[0] if scales else None)
    return result


QgsLayoutExporter.exportToPdf = _spy_export

CORE = find_core()
WORK = tempfile.mkdtemp(prefix="cassini-style-")
project = QgsProject.instance()
checks = []
# 2 : les contrôles finaux, données inchangées et légendes dans la page
state = {"set": -1, "client": None, "expected": 2, "exports": 0}
pdfs = []


def check(label, condition, detail=""):
    checks.append((label, bool(condition)))
    mark = "ok     " if condition else "ECHEC  "
    print(mark + label + (" : %s" % (detail,) if detail and not condition else ""))


def finish():
    QTimer.singleShot(0, QCoreApplication.quit)


def guarded(handler):
    def wrapper(*args, **kwargs):
        try:
            handler(*args, **kwargs)
        except Exception:
            traceback.print_exc()
            check("aucune exception dans la sonde", False)
            finish()

    return wrapper


class NoProbe:  # le geste de test n'est jamais dicté ici
    def start(self, params, done):
        done(result={})

    def cancel(self):
        pass


def new_client(on_event):
    if state["client"] is not None:
        state["client"].stop()
    process = QProcess()
    state["process"] = process
    registry = delegated.build_registry(NoProbe)
    state["client"] = client.CoreClient(process, guarded(on_event), registry)
    state["client"].start(CORE, state["copy"])


def read_journal():
    with open(os.path.join(state["cassini_dir"], "journal.jsonl"), encoding="utf-8") as f:
        return [json.loads(line) for line in f if line.strip()]


def is_pdf(path):
    if not os.path.isfile(path):
        return False
    with open(path, "rb") as f:
        return f.read(4) == b"%PDF"


def tree_groups():
    """[(nom, [ids])] des groupes de la racine, de haut en bas."""
    root = project.layerTreeRoot()
    return [
        (node.name(), [n.layerId() for n in node.findLayers()])
        for node in root.children()
        if QgsLayerTree.isGroup(node)
    ]


def renderer_type(layer_id):
    layer = project.mapLayer(layer_id)
    if layer is None or layer.renderer() is None:
        return None
    return layer.renderer().type()


# -- couleurs du rendu ----------------------------------------------------------
def geometry_of(layer):
    return {
        Qgis.GeometryType.Point: "point",
        Qgis.GeometryType.Line: "line",
        Qgis.GeometryType.Polygon: "polygon",
    }.get(layer.geometryType())


def drawn(symbol, geometry):
    """(remplissage, trait) que QGIS dessine pour un symbole simple ; None : pas dessiné."""
    sl = symbol.symbolLayer(0)
    if geometry == "line":
        return (None, sl.color().name())
    stroke = None if sl.strokeStyle() == Qt.PenStyle.NoPen else sl.strokeColor().name()
    if geometry == "polygon":
        fill = None if sl.brushStyle() == Qt.BrushStyle.NoBrush else sl.fillColor().name()
        return (fill, stroke)
    return (sl.fillColor().name(), stroke)


def dictated(symbol, geometry):
    """(remplissage, trait) attendus d'un symbole dicté, couleurs en minuscules."""
    stroke = (symbol.get("stroke") or "").lower() or None
    fill = (symbol.get("fill") or "").lower() or None
    if geometry == "line":
        return (None, stroke or fill)
    if geometry == "polygon":
        return (fill, stroke)
    return (fill or stroke, stroke)


def close(a, b):
    return abs(a - b) < 1e-6


def visibility(layer):
    """L'échelle au-delà de laquelle QGIS ne dessine plus la couche ; None : toujours."""
    return int(layer.minimumScale()) if layer.hasScaleBasedVisibility() else None


def colours_of(layer, style):
    """Rend (attendu, obtenu) pour la couche et le style dicté, comparables par `==`."""
    want, got = rendering_of(layer, style)
    if style["kind"] == "categories":  # l'opacité de la couche, celle des zones d'un PLU
        want, got = (want, style.get("opacity", 1.0)), (got, round(layer.opacity(), 6))
    if style["kind"] in ("single", "categories", "graduated"):
        return (want, style.get("max_scale")), (got, visibility(layer))
    return want, got


def rendering_of(layer, style):
    kind = style["kind"]
    renderer = layer.renderer()
    if kind == "single":
        geometry = geometry_of(layer)
        symbol = renderer.symbol()
        want = (dictated(style, geometry), style.get("opacity", 1.0))
        return want, (drawn(symbol, geometry), round(symbol.opacity(), 6))
    if kind == "categories" and has_class_scales(style):
        # Rendu par règles : une règle par classe, de même seuil, puis la règle SINON.
        geometry = geometry_of(layer)
        rules = renderer.rootRule().children()
        other = style.get("other")
        want = [
            (c["label"], dictated(c, geometry), c.get("max_scale") or 0, False)
            for c in style["classes"]
        ]
        if other:
            want.append((other["label"], dictated(other, geometry), other.get("max_scale") or 0,
                         True))
        got = [
            (r.label(), drawn(r.symbol(), geometry), int(r.minimumScale()), r.isElse())
            for r in rules
        ]
        return want, got
    if kind == "categories":
        geometry = geometry_of(layer)
        cats = renderer.categories()
        want = [(c["label"], dictated(c, geometry)) for c in style["classes"]]
        got = [(c.label(), drawn(c.symbol(), geometry)) for c in cats[: len(style["classes"])]]
        catch_all = [c for c in cats if c.value() == ""]
        other = style.get("other")
        want_other = [(other["label"], dictated(other, geometry))] if other else []
        got_other = [(c.label(), drawn(c.symbol(), geometry)) for c in catch_all]
        # Le fourre-tout est la dernière catégorie, et il n'y en a pas d'autre.
        last_is_catch_all = not other or (cats and cats[-1].value() == "")
        return (want, want_other, len(style["classes"]) + bool(other), True), (
            got,
            got_other,
            len(cats),
            bool(last_is_catch_all),
        )
    if kind == "graduated":
        ranges = renderer.ranges()
        want = (style["ramp"]["from"].lower(), style["ramp"]["to"].lower())
        got = (ranges[0].symbol().color().name(), ranges[-1].symbol().color().name())
        return want, got
    if kind == "elevation_ramp":
        items = renderer.shader().rasterShaderFunction().colorRampItemList()
        want = ([(s["value"], s["color"].lower()) for s in style["stops"]],
                style.get("opacity", 1.0))
        got = ([(i.value, i.color.name()) for i in items], round(renderer.opacity(), 6))
        return want, got
    # hillshade
    want = (True, style.get("opacity", 1.0))
    multiply = layer.blendMode() == QPainter.CompositionMode.CompositionMode_Multiply
    return want, (multiply, round(renderer.opacity(), 6))


def has_class_scales(style):
    """Un style par catégories dont une classe (ou `other`) a son propre seuil d'échelle."""
    if style["kind"] != "categories":
        return False
    other = style.get("other") or {}
    return bool(other.get("max_scale")) or any(c.get("max_scale") for c in style["classes"])


def expected_renderer(style, got=None):
    if has_class_scales(style):
        return "RuleRenderer"
    # Un MNT que le projet ombrait déjà par le rendu d'ombrage de QGIS le garde.
    if style["kind"] == "hillshade" and got == "hillshade":
        return "hillshade"
    return RENDERERS[style["kind"]]


def check_class_scales(rows, scale):
    """À l'échelle de la carte exportée, une classe à seuil n'est dessinée que si l'échelle est
    plus grande que 1:seuil (par exemple les routes d'importance 5 et 6 au-delà du 1:100 000)."""
    wrong, hidden = [], []
    for layer_id, style in last_styles(rows).items():
        if not has_class_scales(style):
            continue
        layer = project.mapLayer(layer_id)
        for rule in layer.renderer().rootRule().children():
            limit = rule.minimumScale()
            want = limit == 0 or scale <= limit
            if rule.isScaleOK(scale) != want:
                wrong.append((layer.name(), rule.label(), limit))
            if not want:
                hidden.append("%s : %s" % (layer.name(), rule.label()))
    print("   échelle de la carte : 1:%d ; classes non dessinées : %s" % (scale, hidden))
    check(
        "%s : les classes à seuil suivent l'échelle de la carte" % state["name"],
        not wrong,
        wrong,
    )


def last_styles(rows):
    """{layer_id: StyleSpec} du dernier `layer.set_style` de chaque couche (le dégradé passe
    après la mise en forme topographique de sa couche)."""
    last = {}
    for row in rows:
        if row["id"] == "layer.set_style" and "error" not in row:
            last[row["params"]["layer_id"]] = row["params"]["style"]
    return last


def check_styles(rows, label):
    last = last_styles(rows)
    wrong = []
    for layer_id, style in last.items():
        got = renderer_type(layer_id)
        if got != expected_renderer(style, got):
            wrong.append((layer_id, expected_renderer(style, got), got))
    name = state["name"]
    check("%s : chaque couche a le rendu de son style%s" % (name, label), not wrong, wrong)


def forms_present(rows):
    """{forme: (layer_id, style)} : la première couche de chaque forme présente."""
    out = {}
    for layer_id, style in last_styles(rows).items():
        kind = style["kind"] + (" à seuils" if has_class_scales(style) else "")
        out.setdefault(kind, (layer_id, style))
    return out


def check_colours(rows, label):
    for kind, (layer_id, style) in sorted(forms_present(rows).items()):
        layer = project.mapLayer(layer_id)
        want, got = colours_of(layer, style)
        check(
            "%s : couleurs de « %s » (%s) comme dictées%s" % (state["name"], layer.name(), kind,
                                                            label),
            want == got,
            "attendu %s, obtenu %s" % (want, got),
        )


# -- un jeu de données ---------------------------------------------------------
def next_set():
    state["set"] += 1
    if state["set"] >= len(SETS):
        finish()
        return
    name, path, candidates = SETS[state["set"]]
    print("\n== %s ==" % name)
    folder = os.path.join(WORK, name)
    os.makedirs(folder)
    state["name"], state["candidates"] = name, candidates
    state["copy"] = os.path.join(folder, "essai-%s.qgz" % name).replace("\\", "/")
    state["cassini_dir"] = os.path.join(folder, "essai-%s.cassini" % name)
    state["expected"] += FIXED_RUN + FIXED_REPLAY
    state["scales_from"] = len(map_scales)
    project.clear()
    check("%s : le projet de test se lit" % name, project.read(path))
    project.writeEntryBool("Paths", "/Absolute", True)  # la copie cite les données à leur place
    check(
        "%s : la copie s'enregistre hors du dossier de données" % name,
        project.write(state["copy"]),
    )
    state["results"] = []
    new_client(on_first)


def graduated_params():
    for layer_name, field in state["candidates"]:
        for layer in project.mapLayersByName(layer_name):
            if isinstance(layer, QgsVectorLayer) and layer.fields().lookupField(field) >= 0:
                return {
                    "layer_id": layer.id(),
                    "field": field,
                    "method": "jenks",
                    "class_count": 5,
                    "ramp": "yellow_red",
                }
    return None


def steps():
    name = state["name"]
    return [
        ("map.topographic_style", lambda: {}),
        ("layout.export_pdf", lambda: {"title": "Cassini : mise en forme topographique (%s)"
                                       % name}),
        ("map.graduated_style", graduated_params),
        ("layout.export_pdf", lambda: {"title": "Cassini : dégradé (%s)" % name}),
    ]


def run_next():
    command, make = steps()[len(state["results"])]
    params = make()
    if params is None:
        name = state["name"]
        check("%s : une couche de surface à champ numérique pour le dégradé" % name, False)
        finish()
        return
    state["client"].run_command(command, params)


def on_first(kind, payload):
    if kind == client.READY:
        run_next()
    elif kind == client.RUN_DONE:
        state["results"].append(payload["result"])
        if len(state["results"]) < len(steps()):
            run_next()
        else:
            QTimer.singleShot(0, guarded(after_run))
    elif kind in (client.RUN_FAILED, client.RUN_CANCELLED, client.FAILED, client.DIED):
        check("%s : exécution des étapes" % state["name"], False, "%s %s" % (kind, payload))
        finish()


def after_run():
    name = state["name"]
    topo, _, graduated, _ = state["results"]
    rows = read_journal()
    ids = [r["id"] for r in rows]
    styled = topo.get("styled", [])
    print("   journal :", ids)
    print("   reconnues :", [(s["layer_id"].split("_")[0], s["theme"]) for s in styled])
    print(
        "   laissées :",
        [(s["layer_id"].split("_")[0], s["reason"]) for s in topo.get("unchanged", [])],
    )
    check("%s : des couches sont reconnues" % name, len(styled) >= 3, topo)
    errors = [r for r in rows if "error" in r]
    check("%s : aucune ligne du journal en erreur" % name, not errors, errors)
    check(
        "%s : un layer.set_style par couche reconnue, puis un layer.arrange" % name,
        ids.count("layer.set_style") == len(styled) + 1 and ids.count("layer.arrange") == 1,
        ids,
    )
    check_styles(rows, "")
    forms = forms_present(rows)
    print("   formes :", sorted(forms))
    state["forms"] = len(forms)
    check_colours(rows, "")
    check_class_scales(rows, map_scales[state["scales_from"]])
    groups = [(g["name"], g["layers"]) for g in topo["groups"]]
    state["groups"] = groups
    check(
        "%s : les groupes de l'arbre sont ceux du résultat, dans l'ordre" % name,
        tree_groups() == groups,
        tree_groups(),
    )
    check(
        "%s : « Relief » est en dessous de tout" % name,
        groups and groups[-1][0] == "Relief",
        [g for g, _ in groups],
    )
    layer = project.mapLayer(graduated["layer_id"])
    is_graduated = renderer_type(graduated["layer_id"]) == "graduatedSymbol"
    ranges = layer.renderer().ranges() if is_graduated else []
    check("%s : le dégradé a ses 5 classes, calculées par QGIS" % name, len(ranges) == 5,
          len(ranges))
    if ranges:
        print("   seuils :", [round(r.upperValue(), 1) for r in ranges])
    state["export_names"] = [
        "%d-layout.export_pdf.pdf" % (n + 1) for n, i in enumerate(ids) if i == "layout.export_pdf"
    ]
    # Comptées ici, une fois connues : un PDF et une forme se vérifient à l'exécution et au rejeu.
    state["expected"] += 2 * len(state["export_names"]) + 2 * state["forms"]
    state["exports"] += 2 * len(state["export_names"])  # exécution et rejeu
    for export_name in state["export_names"]:
        pdf = os.path.join(state["cassini_dir"], export_name)
        check("%s : le PDF %s commence par %%PDF" % (name, export_name), is_pdf(pdf), pdf)
        pdfs.append(pdf)
    state["journal"] = rows
    state["client"].stop()
    QTimer.singleShot(0, guarded(reopen_and_export))


def expected_bars():
    """{nom de couche: (libellé bas, libellé haut, couleur haute)} attendus des MNT stylés."""
    out = {}
    import style_gestures

    for layer in project.mapLayers().values():
        span = layer.customProperty(style_gestures.ELEVATION_SPAN)
        if not span:
            continue
        low, high = (float(v) for v in str(span).split())
        shader = layer.renderer().shader().rasterShaderFunction()
        _, r, g, b, _ = shader.shade(high)
        out[layer.name()] = ("%d m" % round(low), "%d m" % round(high),
                             "#%02x%02x%02x" % (r, g, b))
    return out


def reopen_and_export():
    """Projet stylé enregistré, rouvert dans une session neuve de QgsProject, exporté sans
    nouveau `layer.set_style` : la légende d'altitudes et l'ombrage gardent leurs libellés."""
    import layout_export

    name = state["name"]
    folder = os.path.dirname(state["copy"])
    saved = os.path.join(folder, "relu-%s.qgz" % name).replace("\\", "/")
    project.write(saved)
    project.clear()
    project.read(saved)
    want = expected_bars()
    out = os.path.join(folder, "relu-%s.pdf" % name)
    state["exports"] += 1
    gesture = layout_export.LayoutExportGesture()
    state["relu_gesture"] = gesture

    def done(result=None, error=None):
        seen = legend_rasters[-1] if legend_rasters else {}
        wrong = []
        for layer_name, (low, high, colour) in want.items():
            texts, bar, _ = seen.get(layer_name, ([], None, None))
            if not texts or texts[0] != "Altitude (m)" or bar != (low, high, colour):
                wrong.append((layer_name, texts, bar, (low, high, colour)))
        shade = [v for v in seen.values() if v[0] == ["Ombrage"]]
        check(
            "%s : rouvert, la légende garde « Altitude (m) », ses bornes et « Ombrage »" % name,
            error is None and want and not wrong and shade,
            (error and error.message, wrong, seen),
        )
        pdfs.append(out)
        project.clear()
        check("%s : la copie rouvre" % name, project.read(state["copy"]))
        new_client(on_replay)

    gesture.start({"title": "Cassini : projet rouvert (%s)" % name, "out": out}, guarded(done))


def on_replay(kind, payload):
    name = state["name"]
    if kind == client.READY:
        check("%s : le rejeu part" % name, state["client"].replay())
    elif kind == client.REPLAY_DONE:
        check(
            "%s : le rejeu rend identical: true" % name,
            payload == {"steps": len(state["journal"]), "identical": True},
            payload,
        )
        check("%s : le journal est inchangé par le rejeu" % name,
              read_journal() == state["journal"])
        check_styles(state["journal"], " au rejeu")
        check_colours(state["journal"], " au rejeu")
        check(
            "%s : les mêmes groupes reviennent au rejeu" % name,
            tree_groups() == state["groups"],
            tree_groups(),
        )
        for export_name in state["export_names"]:
            replayed = os.path.join(state["cassini_dir"], "replay", export_name)
            check("%s : le PDF rejoué %s commence par %%PDF" % (name, export_name),
                  is_pdf(replayed), replayed)
            pdfs.append(replayed)
        state["client"].stop()
        state["client"] = None
        QTimer.singleShot(0, guarded(next_set))
    elif kind in (client.REPLAY_FAILED, client.FAILED, client.DIED):
        check("%s : rejeu" % name, False, "%s %s" % (kind, payload))
        finish()


before = stamps(DATA)
QTimer.singleShot(0, guarded(next_set))
QTimer.singleShot(1800000, QCoreApplication.quit)
QCoreApplication.exec()

if state["client"] is not None:
    state["client"].stop()
project.clear()
after = stamps(DATA)
changed = sorted(p for p in set(before) | set(after) if before.get(p) != after.get(p))
check("aucun fichier de donnees-test-cassini n'a changé (date et taille)", not changed,
      changed[:10])
print("Bas des légendes (mm, permis) :", legend_bottoms)
check(
    "chaque légende reste dans la page, au-dessus des sources",
    len(legend_bottoms) == state["exports"]
    and all(bottom <= limit + 0.5 for bottom, limit in legend_bottoms),
    legend_bottoms,
)
print("\nDossier de la sonde :", WORK)
for pdf in pdfs:
    print("PDF :", pdf)
complete = state["set"] >= len(SETS) and len(checks) == state["expected"]
print("Vérifications : %d sur %d attendues" % (len(checks), state["expected"]))
ok = complete and all(v for _, v in checks)
print("RÉSULTAT :", "OK" if ok else "ÉCHEC")
sys.exit(0 if ok else 1)
