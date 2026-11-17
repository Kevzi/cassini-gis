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


def section_modele(modele_json: Path) -> dict:
    """La section `modele` : valeurs de `installer/modele.json`, seule source."""
    d = json.loads(modele_json.read_text(encoding="utf-8"))
    try:
        return {
            "nom": d["nom_installe"],
            "url": d["url"],
            "sha256": d["sha256"].lower(),
            "taille": int(d["taille"]),
        }
    except (KeyError, ValueError, TypeError, AttributeError) as e:
        raise ManifesteErreur(f"{modele_json} incomplet ({e!r})") from e


def section_porte2(version: str, fichier: Path, url_base: str) -> dict:
    """La section `porte2` : l'installeur complet, sous un nom versionné."""
    nom = f"cassini-installeur-complet-{version}.exe"
    return {
        "nom": nom,
        "url": f"{url_base.rstrip('/')}/{nom}",
        "sha256": sha256_de(fichier),
        "taille": fichier.stat().st_size,
    }


def section_linux(
    noyau: Path, porte2: Path, noyau_url_base: str, porte2_url_base: str
) -> dict:
    """La section `linux` (7.7) : le tar du noyau (release GitHub) et le tar complet (Hugging Face).

    Les noms sont ceux des fichiers construits par `assembler_linux.py`. `cassini.sh` seul la lit ;
    l'installeur Windows ne la lit pas.
    """
    return {
        "noyau": {
            "nom": noyau.name,
            "url": f"{noyau_url_base.rstrip('/')}/{noyau.name}",
            "sha256": sha256_de(noyau),
        },
        "porte2": {
            "nom": porte2.name,
            "url": f"{porte2_url_base.rstrip('/')}/{porte2.name}",
            "sha256": sha256_de(porte2),
            "taille": porte2.stat().st_size,
        },
    }


def construire(
    version: str,
    fichiers: list[Path],
    base_url: str,
    modele: dict | None = None,
    porte2: dict | None = None,
    linux: dict | None = None,
) -> str:
    """Le texte du manifeste, un champ par ligne, retour à la ligne final.

    `modele`, `porte2` et `linux` viennent après `fichiers`, dans cet ordre (l'installeur Windows
    coupe le texte au premier `"modele"` ou `"porte2"` pour sa boucle de fichiers, et `linux`
    reste après eux) ; absents quand non fournis.
    """
    base = base_url.rstrip("/")
    entrees = [
        {"nom": f.name, "url": f"{base}/{f.name}", "sha256": sha256_de(f)} for f in fichiers
    ]
    doc: dict = {"version": version, "fichiers": entrees}
    if modele is not None:
        doc["modele"] = modele
    if porte2 is not None:
        doc["porte2"] = porte2
    if linux is not None:
        doc["linux"] = linux
    return json.dumps(doc, indent=2, ensure_ascii=False) + "\n"


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--etiquette", required=True, help="étiquette de version, par exemple v0.1.0")
    p.add_argument("--cargo", type=Path, default=RACINE / "Cargo.toml")
    p.add_argument("--base-url", help="préfixe des adresses (défaut : la release de l'étiquette)")
    p.add_argument(
        "--modele",
        type=Path,
        nargs="?",
        const=RACINE / "installer" / "modele.json",
        help="ajoute la section modele (défaut : installer/modele.json)",
    )
    p.add_argument("--porte2", type=Path, help="installeur complet : ajoute la section porte2")
    p.add_argument("--porte2-url-base", help="préfixe de l'adresse de la porte 2 (défaut : dossier de l'url du modèle)")
    p.add_argument(
        "--linux",
        type=Path,
        nargs=2,
        metavar=("NOYAU_TAR", "COMPLET_TAR"),
        help="ajoute la section linux : tar du noyau et tar complet (porte 2 Linux)",
    )
    p.add_argument("--sortie", type=Path, required=True)
    p.add_argument("fichiers", nargs="+", type=Path)
    a = p.parse_args(argv)
    try:
        version = version_cargo(a.cargo)
        verifier_etiquette(a.etiquette, version)
        base = a.base_url or f"{DEPOT}/releases/download/{a.etiquette}"
        modele = section_modele(a.modele) if a.modele else None
        porte2 = None
        if a.porte2:
            if modele is None:
                raise ManifesteErreur("--porte2 demande --modele (dossier Hugging Face de la porte 2)")
            url_base = a.porte2_url_base or modele["url"].rsplit("/", 1)[0]
            porte2 = section_porte2(version, a.porte2, url_base)
        linux = None
        if a.linux:
            if modele is None:
                raise ManifesteErreur("--linux demande --modele (dossier Hugging Face de la porte 2)")
            url_base = a.porte2_url_base or modele["url"].rsplit("/", 1)[0]
            linux = section_linux(a.linux[0], a.linux[1], base, url_base)
        texte = construire(version, a.fichiers, base, modele, porte2, linux)
    except (ManifesteErreur, OSError) as e:
        print(f"manifeste : {e}", file=sys.stderr)
        return 1
    a.sortie.write_text(texte, encoding="utf-8", newline="\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
