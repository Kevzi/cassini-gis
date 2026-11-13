"""Écrit `manifeste.json`, le manifeste des versions d'une release Cassini (AD-17).

Mise en forme FIXE : l'installeur Inno Setup n'a pas d'analyseur JSON et y cherche
`"nom"`, `"url"` et `"sha256"` par recherche de texte. Ne pas changer cette forme
sans changer `cassini-en-ligne.iss`.

Usage :
    python -I installer/manifeste.py --etiquette v0.1.0 --sortie manifeste.json cassini.exe
"""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

DEPOT = "https://github.com/Kevzi/cassini-gis"
RACINE = Path(__file__).resolve().parent.parent


class ManifesteErreur(Exception):
    pass


def version_cargo(cargo_toml: Path) -> str:
    """La version du workspace, source de vérité de l'étiquette."""
    texte = cargo_toml.read_text(encoding="utf-8")
    section = re.search(r"^\[workspace\.package\](.*?)(?=^\[|\Z)", texte, re.S | re.M)
    m = re.search(r'^version\s*=\s*"([^"]+)"', section.group(1) if section else "", re.M)
    if not m:
        raise ManifesteErreur(f"version introuvable dans {cargo_toml}")
    return m.group(1)


def verifier_etiquette(etiquette: str, version: str) -> None:
    if etiquette != f"v{version}":
        raise ManifesteErreur(
            f"l'étiquette {etiquette} ne correspond pas à la version Cargo {version} "
            f"(attendu : v{version})"
        )


def sha256_de(chemin: Path) -> str:
    h = hashlib.sha256()
    with chemin.open("rb") as f:
        for bloc in iter(lambda: f.read(1 << 20), b""):
            h.update(bloc)
    return h.hexdigest()


def construire(version: str, fichiers: list[Path], base_url: str) -> str:
    """Le texte du manifeste, un champ par ligne, retour à la ligne final."""
    base = base_url.rstrip("/")
    entrees = [
        {"nom": f.name, "url": f"{base}/{f.name}", "sha256": sha256_de(f)} for f in fichiers
    ]
    return json.dumps({"version": version, "fichiers": entrees}, indent=2, ensure_ascii=False) + "\n"


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--etiquette", required=True, help="étiquette de version, par exemple v0.1.0")
    p.add_argument("--cargo", type=Path, default=RACINE / "Cargo.toml")
    p.add_argument("--base-url", help="préfixe des adresses (défaut : la release de l'étiquette)")
    p.add_argument("--sortie", type=Path, required=True)
    p.add_argument("fichiers", nargs="+", type=Path)
    a = p.parse_args(argv)
    try:
        version = version_cargo(a.cargo)
        verifier_etiquette(a.etiquette, version)
        base = a.base_url or f"{DEPOT}/releases/download/{a.etiquette}"
        texte = construire(version, a.fichiers, base)
    except (ManifesteErreur, OSError) as e:
        print(f"manifeste : {e}", file=sys.stderr)
        return 1
    a.sortie.write_text(texte, encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
