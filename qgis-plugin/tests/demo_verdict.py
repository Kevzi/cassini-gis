"""Verdict de la sonde de démo, sans QGIS : lu par `probe_demo.py`, testé par `test_demo_verdict.py`.

La sonde relève les faits sous QGIS (zone et distance de chaque entité retenue, début du PDF) ;
ce module dit, à partir d'eux et des événements de `client.py`, si la phrase de démo a donné la
bonne réponse. Chaque fonction rend la raison d'un ECHEC, ou `None` quand tout va bien.

Le contrôle est géométrique, pas une liste d'identifiants : il tient quel que soit l'ordre des
intersections choisi par le modèle, et attrape un plan qui oublie la zone ou le tampon.
"""

EXPORT = "layout.export_pdf"
# Seules les analyses créent la couche de réponse ; une étape de style rend la couche qu'elle
# met en forme, qui n'est pas la réponse.
ANSWER_PREFIX = "analysis."
ZONE_FIELD = "typezone"
ZONE = "U"
MAX_DISTANCE_M = 500.0
# Le tampon de QGIS est un polygone inscrit dans le cercle : une tolérance de calcul suffit.
TOLERANCE_M = 0.01

# Noms des événements de `client.py`, recopiés pour rester importable sans QGIS ni Qt.
PLAN_QUESTION = "plan_question"
PLAN_FAILED = "plan_failed"
PLAN_EXEC_FAILED = "plan_exec_failed"
PLAN_CANCELLED = "plan_cancelled"
FAILED = "failed"
DIED = "died"


def verdict_line(reason):
    """La ligne finale de la sonde."""
    return "VERDICT : OK" if reason is None else "VERDICT : ECHEC — %s" % reason


def _export_index(steps):
    for index, step in enumerate(steps):
        if isinstance(step, dict) and step.get("command") == EXPORT:
            return index
    return None


def retained_layer_id(steps):
    """`layer_id` de la dernière analyse avant l'export, sinon `None`."""
    end = _export_index(steps)
    before = steps if end is None else steps[:end]
    for step in reversed(before):
        if not isinstance(step, dict) or not str(step.get("command", "")).startswith(ANSWER_PREFIX):
            continue
        result = step.get("result")
        layer_id = result.get("layer_id") if isinstance(result, dict) else None
        if isinstance(layer_id, str) and layer_id:
            return layer_id
    return None


def pdf_file(steps):
    """Le `file` rendu par l'étape d'export, sinon `None`."""
    end = _export_index(steps)
    if end is None:
        return None
    result = steps[end].get("result")
    file = result.get("file") if isinstance(result, dict) else None
    return file if isinstance(file, str) and file else None


def missing_layers_reason(has_zoning, has_stations, has_parcels):
    """Raison d'ECHEC quand le projet n'a pas les couches de référence du contrôle."""
    if not has_parcels:
        return "couche des parcelles introuvable : aucune couche de polygones nommée « parcelle »"
    if not has_zoning:
        return "couche du PLU introuvable : aucune couche n'a de champ « %s »" % ZONE_FIELD
    if not has_stations:
        return "couche des gares introuvable : aucune couche de points nommée « gare »"
    return None


def judge_done(steps, facts, injected=None):
    """Verdict d'un plan exécuté jusqu'au bout.

    `facts` : `features`, liste de {"name", "zone", "distance"} relevée sur la couche retenue,
    ou `None` si cette couche n'est pas dans le projet ; `parcel_fields`, vrai si la couche
    retenue porte un champ propre aux parcelles (sinon la réponse n'est pas faite de parcelles) ;
    `pdf_head`, les cinq premiers octets du PDF, ou `None` s'il n'a pas pu être lu.
    `injected` : la commande où un échec devait être injecté ; un plan mené au bout malgré
    elle veut dire que l'injection n'a pas eu lieu.
    """
    if injected:
        return "échec injecté sur %s jamais survenu : aucun geste de l'extension pour elle" % injected
    layer_id = retained_layer_id(steps)
    if layer_id is None:
        return "aucune étape du plan ne rend de couche de résultat"
    features = facts.get("features")
    if features is None:
        return "couche retenue %s absente du projet" % layer_id
    if not features:
        return "couche retenue %s vide" % layer_id
    if not facts.get("parcel_fields"):
        return "couche retenue %s sans champ des parcelles : la réponse n'est pas faite de parcelles" % layer_id
    for feature in features:
        name = feature.get("name")
        zone = feature.get("zone")
        if zone != ZONE:
            where = "hors du zonage" if zone is None else "en zone %s" % zone
            return "entité %s %s, pas en zone %s" % (name, where, ZONE)
        distance = feature.get("distance")
        if distance is None or distance > MAX_DISTANCE_M + TOLERANCE_M:
            shown = "inconnue" if distance is None else "%.1f m" % distance
            return "entité %s à %s de la gare la plus proche (plus de %d m)" % (
                name, shown, MAX_DISTANCE_M)
    file = pdf_file(steps)
    if file is None:
        return "aucun PDF : le plan n'a pas d'étape %s" % EXPORT
    if facts.get("pdf_head") != b"%PDF-":
        return "PDF %s illisible ou absent" % file
    return None


def failure_reason(kind, payload, calls=None):
    """Raison d'ECHEC d'un événement qui n'est pas PLAN_DONE, ou `None` s'il n'en est pas un.

    `calls` : les appels du plan proposé, pour nommer la commande de l'étape fautive.
    """
    # FAILED et DIED portent le message seul, à afficher tel quel.
    message = payload if isinstance(payload, str) else ""
    payload = payload if isinstance(payload, dict) else {}
    if kind == PLAN_QUESTION:
        return "question : %s" % payload.get("question")
    if kind == PLAN_FAILED:
        return "refus ou échec de la composition : %s %s" % (
            payload.get("code"), payload.get("message"))
    if kind in (PLAN_EXEC_FAILED, PLAN_CANCELLED):
        step = payload.get("step")
        command = "?"
        if isinstance(step, int) and calls and 1 <= step <= len(calls):
            call = calls[step - 1]
            if isinstance(call, dict) and isinstance(call.get("command"), str):
                command = call["command"]
        code = payload.get("code") if kind == PLAN_EXEC_FAILED else "annulée"
        return "étape %s %s : %s" % (step if step is not None else "?", command, code)
    if kind in (FAILED, DIED):
        return "noyau : %s" % message
    return None
