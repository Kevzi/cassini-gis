"""Garde statique du Python de l'extension QGIS (AD-14, AD-18).

Elle lit les sources par analyse `ast` et n'exécute rien de l'extension.
Usage : python -I qgis-plugin/tests/check_extension.py [dossier_extension]
Bibliothèque standard seule. Code 0 si tout est propre, 1 sinon.
"""

import ast
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
DEFAULT_PKG = os.path.join(HERE, "..", "cassini_qgis")

# Racine du module -> raison du refus.
FORBIDDEN = {
    "urllib": "accès réseau interdit (principe 5, hors ligne total)",
    "requests": "accès réseau interdit (principe 5, hors ligne total)",
    "socket": "accès réseau interdit (principe 5, hors ligne total)",
    "http": "accès réseau interdit (principe 5, hors ligne total)",
    "ftplib": "accès réseau interdit (principe 5, hors ligne total)",
    "smtplib": "accès réseau interdit (principe 5, hors ligne total)",
    "telnetlib": "accès réseau interdit (principe 5, hors ligne total)",
    "xmlrpc": "accès réseau interdit (principe 5, hors ligne total)",
    "socketserver": "accès réseau interdit (principe 5, hors ligne total)",
    "selectors": "accès réseau interdit (principe 5, hors ligne total)",
    "webbrowser": "accès réseau interdit (principe 5, hors ligne total)",
    "ssl": "accès réseau interdit (principe 5, hors ligne total)",
    "asyncio": "accès réseau interdit (principe 5, hors ligne total)",
    "PyQt5": "import direct interdit : passer par qgis.PyQt",
    "PyQt6": "import direct interdit : passer par qgis.PyQt",
}

# Sous-modules de qgis.PyQt qui ouvrent le réseau.
FORBIDDEN_QT = {"QtNetwork", "QtWebSockets", "QtWebEngineCore", "QtWebEngineWidgets", "QtWebKit", "QtWebChannel"}

# Appels qui importent par un nom calculé : la garde ne peut plus lire le module.
DYNAMIC_IMPORTS = {"__import__", "import_module"}

MAX_BYTES = 25 * 1024 * 1024
EXPECTED_META = {"qgisMinimumVersion": "3.44", "qgisMaximumVersion": "4.99"}
# Exigences du dépôt d'extensions QGIS (7.10) : clés à remplir et valeur fixée.
REQUIRED_PUBLICATION = ("email", "homepage", "repository", "tracker", "icon")


def check_imports(pkg):
    errors = []
    for dirpath, _dirs, files in os.walk(pkg):
        for name in sorted(files):
            if not name.endswith(".py"):
                continue
            path = os.path.join(dirpath, name)
            try:
                with open(path, "r", encoding="utf-8") as f:
                    tree = ast.parse(f.read(), filename=path)
            except (SyntaxError, UnicodeDecodeError) as e:
                errors.append(f"{path} : source illisible ({e})")
                continue
            for node in ast.walk(tree):
                mods = []
                if isinstance(node, ast.Import):
                    mods = [a.name for a in node.names]
                elif isinstance(node, ast.ImportFrom) and node.level == 0 and node.module:
                    mods = [node.module]
                if isinstance(node, ast.ImportFrom) and node.module == "qgis.PyQt":
                    mods += ["qgis.PyQt." + a.name for a in node.names]
                if isinstance(node, ast.Call):
                    func = node.func
                    called = func.id if isinstance(func, ast.Name) else getattr(func, "attr", None)
                    if called in DYNAMIC_IMPORTS:
                        errors.append(
                            f"{path}:{node.lineno} : import dynamique « {called} » refusé, "
                            "la garde ne peut pas le lire"
                        )
                for mod in mods:
                    parts = mod.split(".")
                    root = parts[0]
                    if root == "qgis" and len(parts) > 2 and parts[1] == "PyQt" and parts[2] in FORBIDDEN_QT:
                        errors.append(
                            f"{path}:{node.lineno} : import du module « {mod} » refusé, "
                            "accès réseau interdit (principe 5, hors ligne total)"
                        )
                    elif root in FORBIDDEN:
                        errors.append(
                            f"{path}:{node.lineno} : import du module « {mod} » refusé, "
                            f"{FORBIDDEN[root]}"
                        )
    return errors


def check_size(pkg):
    total = 0
    for dirpath, _dirs, files in os.walk(pkg):
        for name in files:
            total += os.path.getsize(os.path.join(dirpath, name))
    if total >= MAX_BYTES:
        return [
            f"{pkg} : paquet trop gros, {total / (1024 * 1024):.1f} Mo "
            f"pour une limite de {MAX_BYTES // (1024 * 1024)} Mo"
        ]
    return []


def check_metadata(pkg):
    path = os.path.join(pkg, "metadata.txt")
    if not os.path.isfile(path):
        return [f"{path} : fichier absent"]
    values = {}
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            if "=" in line and not line.lstrip().startswith(("#", ";", "[")):
                k, v = line.split("=", 1)
                values[k.strip()] = v.strip()
    errors = []
    for key, expected in EXPECTED_META.items():
        if values.get(key) != expected:
            errors.append(
                f"{path} : clé {key} = « {values.get(key)} », attendu « {expected} »"
            )
    return errors


def check_publication(pkg):
    """Exigences du dépôt d'extensions QGIS : métadonnées complètes, icône et LICENSE présents."""
    path = os.path.join(pkg, "metadata.txt")
    if not os.path.isfile(path):
        return [f"{path} : fichier absent"]
    values = {}
    with open(path, "r", encoding="utf-8") as f:
        for line in f:
            if "=" in line and not line.lstrip().startswith(("#", ";", "[")):
                k, v = line.split("=", 1)
                values[k.strip()] = v.strip()
    errors = [f"{path} : clé {k} vide ou absente" for k in REQUIRED_PUBLICATION if not values.get(k)]
    if values.get("experimental", "").lower() != "false":
        errors.append(f"{path} : experimental doit valoir False")
    if "traceur" in values.get("description", "").lower():
        errors.append(f"{path} : la description ne doit plus dire « traceur »")
    icon = values.get("icon")
    if icon and not os.path.isfile(os.path.join(pkg, icon)):
        errors.append(f"{path} : icône {icon} introuvable dans le paquet")
    if not os.path.isfile(os.path.join(pkg, "LICENSE")):
        errors.append(f"{pkg} : LICENSE absent")
    return errors


def run(pkg, publication=False):
    errors = check_imports(pkg) + check_size(pkg) + check_metadata(pkg)
    return errors + check_publication(pkg) if publication else errors


def main(argv):
    pkg = argv[1] if len(argv) > 1 else DEFAULT_PKG
    errors = run(pkg, publication=True)
    for e in errors:
        print(f"ECHEC {e}", file=sys.stderr)
    if errors:
        return 1
    print("Garde de l'extension : propre.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
