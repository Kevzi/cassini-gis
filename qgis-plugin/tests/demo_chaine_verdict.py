"""Verdict de la sonde de la démonstration de bout en bout (`probe_demo_chaine.py`), sans QGIS.

Une seule phrase française enchaîne conversion MGRS, reprojection vers le système du MNT, champ de
vision cumulé et export PDF A3 carroyé. La sonde relève les faits sous QGIS (plan proposé, début et
boîte de page du PDF, grilles posées sur la carte) ; ce module dit si la chaîne est juste. Chaque
fonction rend la raison d'un ECHEC, ou `None` quand tout va bien. Le rejeu, le réseau et les
événements d'échec sont jugés par `demo_verdict.py`, repris tel quel.
"""

import re

import demo_verdict

CONVERT = "coord.convert"
REPROJECTION = ("data.transform_point", "data.reproject")
VIEWSHED = "terrain.viewshed_cumulative"
EXPORT = demo_verdict.EXPORT

# Les quatre familles de la chaîne, dans l'ordre où le plan doit les dicter.
FAMILIES = (
    ("conversion MGRS", (CONVERT,)),
    ("reprojection", REPROJECTION),
    ("champ de vision cumulé", (VIEWSHED,)),
    ("export PDF", (EXPORT,)),
)

# A3 en points PDF : 297 x 420 mm, une page, dans un sens ou dans l'autre.
A3_POINTS = (841.89, 1190.55)
A3_TOLERANCE = 2.0
# Carroyage demandé : 1 km, en projection UTM (EPSG:326xx nord, 327xx sud).
GRID_INTERVAL_M = 1000.0
_UTM_AUTHID = re.compile(r"^EPSG:32[67](?:0[1-9]|[1-5]\d|60)$")


def _commands(calls):
    return [c.get("command") if isinstance(c, dict) else None for c in (calls or [])]


def judge_plan(calls):
    """Le plan contient les quatre familles, dans l'ordre : conversion, reprojection, champ de
    vision cumulé, export. `calls` : les appels du plan proposé."""
    commands = _commands(calls)
    position = -1
    for label, ids in FAMILIES:
        found = next((i for i, c in enumerate(commands) if c in ids and i > position), None)
        if found is None:
            where = "" if not any(c in ids for c in commands) else " (ou hors de l'ordre)"
            return "plan : %s absente%s ; plan obtenu %s" % (label, where, commands)
        position = found
    return None


def export_params(calls):
    """Paramètres du dernier appel d'export du plan, `{}` s'il n'y en a pas."""
    for call in reversed(calls or []):
        if isinstance(call, dict) and call.get("command") == EXPORT:
            params = call.get("params")
            return params if isinstance(params, dict) else {}
    return {}


def judge_requested(calls):
    """Le plan demande bien l'A3 et un carroyage de 1 km."""
    params = export_params(calls)
    if str(params.get("page_size", "")).lower() != "a3":
        return "plan : page_size %s au lieu de a3" % params.get("page_size")
    grid = params.get("grid")
    if not isinstance(grid, dict):
        return "plan : l'export ne demande aucun carroyage"
    interval = grid.get("interval")
    if not isinstance(interval, dict) or (interval.get("unit"), interval.get("value")) not in (
            ("km", 1), ("m", 1000)):
        return "plan : carroyage d'intervalle %s au lieu de 1 km" % interval
    return None


def pdf_media_box(data):
    """(largeur, hauteur) en points de la première boîte de page d'un PDF lu en octets, sinon `None`."""
    m = re.search(rb"/MediaBox\s*\[\s*([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s+([-\d.]+)\s*\]", data or b"")
    if m is None:
        return None
    x0, y0, x1, y1 = (float(v) for v in m.groups())
    return (abs(x1 - x0), abs(y1 - y0))


def pdf_page_count(data):
    """Nombre de pages d'un PDF lu en octets (objets `/Type /Page`, hors `/Pages`)."""
    return len(re.findall(rb"/Type\s*/Page(?![s\w])", data or b""))


def is_a3(box):
    if box is None:
        return False
    small, large = sorted(box)
    return abs(small - A3_POINTS[0]) <= A3_TOLERANCE and abs(large - A3_POINTS[1]) <= A3_TOLERANCE


def judge_pdf(data, grids):
    """Le PDF est lisible, d'une page, en A3, et la carte porte un carroyage UTM de 1 km actif.

    `data` : octets du PDF (ou `None` s'il est illisible) ; `grids` : liste de
    (authid, intervalle X, intervalle Y, actif) relevée sur les cartes de la mise en page."""
    if not data or not data.startswith(b"%PDF-"):
        return "PDF illisible ou vide"
    box = pdf_media_box(data)
    if box is None:
        return "PDF sans boîte de page lisible"
    if not is_a3(box):
        return "PDF hors A3 : %.0f x %.0f pt au lieu de 842 x 1191 pt" % box
    pages = pdf_page_count(data)
    if pages != 1:
        return "PDF de %d page(s) au lieu d'une" % pages
    if not grids:
        return "PDF sans carroyage : aucune grille sur la carte"
    for authid, x, y, enabled in grids:
        if enabled and _UTM_AUTHID.match(str(authid)) and x == GRID_INTERVAL_M and y == GRID_INTERVAL_M:
            return None
    return "PDF sans carroyage UTM de 1 km actif : grilles posées %s" % list(grids)


def judge_steps(calls, steps):
    """Chaque appel du plan a rendu une étape, et l'étape d'export porte un fichier."""
    if len(steps or []) < len(calls or []):
        return "exécution : %d étape(s) rendue(s) pour %d appel(s)" % (len(steps or []), len(calls or []))
    if demo_verdict.pdf_file(steps) is None:
        return "aucun PDF : le plan n'a pas d'étape %s" % EXPORT
    return None


def first_reason(*reasons):
    """Première raison d'ECHEC non nulle."""
    return next((r for r in reasons if r is not None), None)
