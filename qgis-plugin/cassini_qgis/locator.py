"""Localisation du noyau Cassini, dans l'ordre fixé par AD-17 et aucun autre.

Module pur : aucune dépendance à Qt ni à QGIS.
"""

import os

SETTING_KEY = "cassini/core_path"
ENV_VAR = "CASSINI_CORE"


def install_path(platform, env, home):
    """Emplacement d'installation du paquet Cassini (décision 1 du ticket)."""
    if platform.startswith("win"):
        base = env.get("LOCALAPPDATA")
        if not base:
            return None
        return os.path.join(base, "Cassini", "bin", "cassini.exe")
    return os.path.join(home, ".local", "share", "cassini", "bin", "cassini")


def _on_path(platform, env, is_file):
    name = "cassini.exe" if platform.startswith("win") else "cassini"
    for folder in env.get("PATH", "").split(os.pathsep):
        if folder:
            candidate = os.path.join(folder, name)
            if is_file(candidate):
                return candidate
    return None


def candidates(setting, env, platform, home):
    """Les trois premiers emplacements, dans l'ordre (le PATH vient après)."""
    return [
        ("réglage " + SETTING_KEY, setting or None),
        ("variable " + ENV_VAR, env.get(ENV_VAR) or None),
        ("emplacement d'installation", install_path(platform, env, home)),
    ]


def locate(setting, env=None, platform=None, home=None, is_file=os.path.isfile):
    """Rend (chemin, origine) du premier noyau existant, ou None."""
    import sys

    env = os.environ if env is None else env
    platform = sys.platform if platform is None else platform
    home = os.path.expanduser("~") if home is None else home
    for origin, path in candidates(setting, env, platform, home):
        if path and is_file(path):
            return path, origin
    found = _on_path(platform, env, is_file)
    if found:
        return found, "PATH"
    return None
