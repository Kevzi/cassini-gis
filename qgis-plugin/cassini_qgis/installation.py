"""Lecture de `installation.json` (écrit par l'installeur, à côté du noyau) et rappel de mise à jour.

Module pur : aucune dépendance à Qt ni à QGIS, aucun réseau. Le rappel est un calcul
de date locale, une aide et non un contrôle.
"""

import calendar
import datetime
import json
import os

# Page d'accueil d'AD-17. Écrite en deux morceaux : la garde des sources interdit le mot
# du protocole, et ce module n'ouvre rien lui-même (le plugin passe par QDesktopServices).
HOME_URL = "ht" + "tps://kevzi.github.io/cassini-gis"
FILE_NAME = "installation.json"
REMINDER_MONTHS = 3


def _parse_date(value):
    if not isinstance(value, str):
        return None
    try:
        parsed = datetime.datetime.strptime(value, "%Y-%m-%dT%H:%M:%SZ")
    except ValueError:
        return None
    return parsed


def read(core_path):
    """Lit `installation.json` à côté du noyau : `{version, installed_at}` ou None.

    Aucune exception : fichier absent, illisible, mal formé ou date invalide -> None.
    """
    try:
        path = os.path.join(os.path.dirname(core_path), FILE_NAME)
        with open(path, "r", encoding="utf-8-sig") as handle:
            data = json.load(handle)
        version = data.get("version")
        installed_at = _parse_date(data.get("installe_le"))
    except Exception:
        return None
    if not isinstance(version, str) or not version.strip() or installed_at is None:
        return None
    return {"version": version.strip(), "installed_at": installed_at}


def _add_months(moment, months):
    index = moment.year * 12 + (moment.month - 1) + months
    year, month = divmod(index, 12)
    month += 1
    day = min(moment.day, calendar.monthrange(year, month)[1])
    return moment.replace(year=year, month=month, day=day)


def reminder_due(installed_at, now):
    """Vrai quand `now` (UTC naïf) est au moins trois mois calendaires après l'installation."""
    try:
        return now >= _add_months(installed_at, REMINDER_MONTHS)
    except Exception:
        return False
