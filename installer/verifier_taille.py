"""Refuse un installeur dont la taille approche la limite d'un seul fichier Inno Setup.

Un seul fichier tient jusqu'à 4,2 Go compressés (documentation officielle) ; au-delà,
`DiskSpanning` serait obligatoire. Seuil de sécurité : 4 Go. Repli prévu : zip + `install.cmd`.

Usage :
    python -I installer/verifier_taille.py installer-sortie/cassini-installeur-complet.exe
"""

import sys
from pathlib import Path

SEUIL = 4_000_000_000


def verifier(chemin: Path, seuil: int = SEUIL) -> int:
    taille = chemin.stat().st_size
    if taille > seuil:
        raise ValueError(
            f"{chemin.name} pèse {taille} octets, au-delà du seuil de {seuil} "
            f"(limite d'un seul fichier : 4,2 Go) ; repli : zip et install.cmd"
        )
    return taille


def main(argv: list[str]) -> int:
    if len(argv) != 1:
        print("usage : verifier_taille.py <fichier>", file=sys.stderr)
        return 2
    try:
        print(f"{argv[0]} : {verifier(Path(argv[0]))} octets, sous le seuil")
    except (ValueError, OSError) as e:
        print(f"installeur refusé : {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
