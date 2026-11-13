"""Archive de l'extension QGIS pour la porte 2 (ticket 7.2, AD-17).

Zippe `qgis-plugin/cassini_qgis` sous un dossier `cassini_qgis/`, sans binaire et sous
25 Mo, la limite du dépôt d'extensions. Refuse (code 1, message en français) un binaire
ou une archive trop lourde.

Usage :
    python -I installer/archive_extension.py --sortie dist/cassini_qgis-0.1.0.zip
"""

import argparse
import sys
import zipfile
from pathlib import Path

RACINE = Path(__file__).resolve().parent.parent
SOURCE = RACINE / "qgis-plugin" / "cassini_qgis"
LIMITE = 25 * 1024 * 1024

CLES_PUBLICATION = ("email", "homepage", "repository", "tracker", "icon")
EXTENSIONS_BINAIRES = {".exe", ".dll", ".so", ".dylib", ".pyd", ".gguf", ".bin", ".a", ".lib", ".o"}
# Signatures d'exécutables : PE (MZ), ELF, Mach-O.
SIGNATURES = (b"MZ", b"\x7fELF", b"\xfe\xed\xfa\xce", b"\xfe\xed\xfa\xcf", b"\xcf\xfa\xed\xfe", b"\xce\xfa\xed\xfe")


class ArchiveErreur(Exception):
    pass


def est_binaire(chemin: Path) -> bool:
    if chemin.suffix.lower() in EXTENSIONS_BINAIRES:
        return True
    with chemin.open("rb") as f:
        return f.read(4).startswith(SIGNATURES)


def fichiers_a_archiver(source: Path) -> list[Path]:
    return sorted(
        p
        for p in source.rglob("*")
        if p.is_file() and "__pycache__" not in p.parts and p.suffix != ".pyc"
    )


def construire(source: Path, sortie: Path, limite: int = LIMITE) -> int:
    """Écrit l'archive et rend sa taille ; ne laisse aucun fichier si elle est refusée."""
    if not source.is_dir():
        raise ArchiveErreur(f"dossier de l'extension introuvable : {source}")
    fichiers = fichiers_a_archiver(source)
    if not fichiers:
        raise ArchiveErreur(f"extension vide : aucun fichier à archiver dans {source}")
    for f in fichiers:
        if est_binaire(f):
            raise ArchiveErreur(
                f"l'extension ne doit contenir aucun binaire : {f.relative_to(source)}"
            )
    sortie.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(sortie, "w", zipfile.ZIP_DEFLATED) as z:
        for f in fichiers:
            z.write(f, f"{source.name}/{f.relative_to(source).as_posix()}")
    taille = sortie.stat().st_size
    if taille >= limite:
        sortie.unlink()
        raise ArchiveErreur(
            f"l'archive de l'extension pèse {taille} octets, la limite est de {limite} (moins de 25 Mo)"
        )
    return taille


def verifier_publication(source: Path) -> None:
    """Exigences du dépôt d'extensions QGIS : LICENSE, icône et clés de `metadata.txt`."""
    meta = source / "metadata.txt"
    valeurs = {}
    if meta.is_file():
        for ligne in meta.read_text(encoding="utf-8").splitlines():
            if "=" in ligne and not ligne.lstrip().startswith(("#", ";", "[")):
                k, v = ligne.split("=", 1)
                valeurs[k.strip()] = v.strip()
    manques = [k for k in CLES_PUBLICATION if not valeurs.get(k)]
    if manques:
        raise ArchiveErreur(f"metadata.txt : clés vides ou absentes : {', '.join(manques)}")
    if not (source / valeurs["icon"]).is_file():
        raise ArchiveErreur(f"metadata.txt : icône introuvable : {valeurs['icon']}")
    if not (source / "LICENSE").is_file():
        raise ArchiveErreur("LICENSE absent de l'extension")


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--source", type=Path, default=SOURCE)
    p.add_argument("--sortie", type=Path, required=True)
    a = p.parse_args(argv)
    try:
        verifier_publication(a.source)
        taille = construire(a.source, a.sortie)
    except (ArchiveErreur, OSError) as e:
        print(f"archive de l'extension refusée : {e}", file=sys.stderr)
        return 1
    print(f"{a.sortie} : {taille} octets")
    return 0


if __name__ == "__main__":
    sys.exit(main())
