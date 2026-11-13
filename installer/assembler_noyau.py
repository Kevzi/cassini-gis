"""Assemble le paquet du noyau gpu (ticket 7.6) par liste fermée.

Copie de `C:\\ct\\release` vers `dist\\noyau` exactement : `cassini.exe`, `llama.dll`,
`ggml.dll`, `ggml-base.dll`, tous les `ggml-cpu-*.dll` (au moins un) et `ggml-vulkan.dll`.
Échoue si l'un manque, et si un autre fichier (DLL ou exécutable) est présent dans le dossier
de construction : jamais `llama-common.dll`, qui embarque un client HTTP. Seuls les `.dll` et
`.exe` comptent : les autres fichiers de construction (`.pdb`, `.lib`, `.d`, dossiers) sont ignorés.

Usage :
    python -I installer/assembler_noyau.py --source C:\\ct\\release --sortie dist/noyau
"""

import argparse
import re
import shutil
import sys
from pathlib import Path

FIXES = ("cassini.exe", "llama.dll", "ggml.dll", "ggml-base.dll", "ggml-vulkan.dll")
MOTIF_CPU = re.compile(r"^ggml-cpu-[^/\\]+\.dll$")


class AssemblageErreur(Exception):
    pass


def fichiers_attendus(noms: set[str]) -> list[str]:
    return sorted(set(FIXES) | {n for n in noms if MOTIF_CPU.match(n)})


def assembler(source: Path, sortie: Path) -> list[str]:
    """Copie le paquet ; rien n'est écrit si le dossier source ne convient pas."""
    if not source.is_dir():
        raise AssemblageErreur(f"dossier de construction introuvable : {source}")
    presents = {f.name for f in source.iterdir() if f.is_file()}
    attendus = fichiers_attendus(presents)
    manquants = [n for n in FIXES if n not in presents]
    if manquants:
        raise AssemblageErreur(f"fichier manquant dans le paquet : {', '.join(manquants)}")
    if not any(MOTIF_CPU.match(n) for n in presents):
        raise AssemblageErreur("aucun moteur processeur (ggml-cpu-*.dll) dans le paquet")
    en_trop = sorted(
        n for n in presents
        if n not in attendus and Path(n).suffix.lower() in {".dll", ".exe"}
    )
    if en_trop:
        raise AssemblageErreur(f"fichier en trop dans le paquet : {', '.join(en_trop)}")
    sortie.mkdir(parents=True, exist_ok=True)
    for ancien in sortie.iterdir():
        if ancien.is_file():
            ancien.unlink()
    for n in attendus:
        shutil.copy2(source / n, sortie / n)
    return attendus


def verifier_dossier(dossier: Path) -> list[str]:
    """Le dossier assemblé contient exactement la liste fermée, sans rien d'autre."""
    if not dossier.is_dir():
        raise AssemblageErreur(f"paquet introuvable : {dossier}")
    presents = {f.name for f in dossier.iterdir() if f.is_file()}
    autres = [d.name for d in dossier.iterdir() if not d.is_file()]
    attendus = fichiers_attendus(presents)
    if autres or presents != set(attendus) or not any(MOTIF_CPU.match(n) for n in presents):
        raise AssemblageErreur(f"le paquet n'est pas la liste fermée : {sorted(presents)}")
    return attendus


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--source", type=Path, required=True)
    p.add_argument("--sortie", type=Path, required=True)
    a = p.parse_args(argv)
    try:
        noms = assembler(a.source, a.sortie)
        verifier_dossier(a.sortie)
    except (AssemblageErreur, OSError) as e:
        print(f"paquet du noyau refusé : {e}", file=sys.stderr)
        return 1
    print("paquet assemblé : " + ", ".join(noms))
    return 0


if __name__ == "__main__":
    sys.exit(main())
