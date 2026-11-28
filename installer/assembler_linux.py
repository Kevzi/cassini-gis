"""Assemble les deux archives Linux de Cassini (ticket 7.7) par liste fermée.

- `cassini-noyau-linux-<version>.tar.gz` : le noyau processeur, `cassini` et les deux textes de
  licences (porte 1 : `cassini.sh` le télécharge depuis la release GitHub).
- `cassini-complet-linux-<version>.tar` : la porte 2, `noyau/cassini`, `modele/<nom>.gguf`,
  `cassini.sh`, `VERSION`, `empreintes.txt` (SHA-256 du noyau et du modèle, vérifiés par le script
  avant toute pose) et les deux textes de licences, posés sans empreinte. Sans compression : le modèle quantifié ne se comprime presque pas. Hébergée
  sur Hugging Face (plus de 2 Gio), pas sur la release GitHub.

Le binaire doit être un exécutable ELF nommé `cassini`, et ne demander aucune version de la glibc
au-delà de 2.35 (Ubuntu 22.04) : la garde lit `objdump -T`.

Usage :
    python -I installer/assembler_linux.py --binaire target/release/cassini \\
        --modele modele/cassini-composition-4b-0.3.0-Q4_K_M.gguf --textes dist/licences --version 0.1.5 --sortie dist-linux
"""

import argparse
import gzip
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tarfile
from pathlib import Path

INSTALLER = Path(__file__).resolve().parent
# Produits par installer/licences.py ; dans chaque archive, l'assemblage échoue sans eux.
TEXTES = ("LICENCES-TIERCES.txt", "FRONTIERE-GPL.txt")
PLAFOND_GLIBC = (2, 35)
MOTIF_GLIBC = re.compile(r"GLIBC_(\d+)\.(\d+)(?:\.(\d+))?")
MAGIE_ELF = b"\x7fELF"
# Bibliothèques partagées qu'un Ubuntu 22.04 propre fournit toujours : liste fermée.
BIBLIOTHEQUES_PERMISES = (
    "libc.so.6",
    "libm.so.6",
    "libdl.so.2",
    "libpthread.so.0",
    "librt.so.1",
    "libgcc_s.so.1",
    "libstdc++.so.6",
    "ld-linux-x86-64.so.2",
)
MOTIF_NEEDED = re.compile(r"^\s*NEEDED\s+(\S+)\s*$", re.M)


class AssemblageErreur(Exception):
    pass


def noms_archives(version: str) -> tuple[str, str]:
    return f"cassini-noyau-linux-{version}.tar.gz", f"cassini-complet-linux-{version}.tar"


def plafond_glibc(sortie_objdump: str) -> tuple[int, int]:
    """La plus haute version de glibc demandée par `objdump -T` ((0, 0) si aucune)."""
    plus_haut = (0, 0)
    for m in MOTIF_GLIBC.finditer(sortie_objdump):
        v = (int(m.group(1)), int(m.group(2)))
        plus_haut = max(plus_haut, v)
    return plus_haut


def verifier_glibc(binaire: Path) -> tuple[int, int]:
    """Échoue si le binaire exige une glibc plus récente que le plafond."""
    try:
        r = subprocess.run(
            ["objdump", "-T", str(binaire)], capture_output=True, text=True, check=False
        )
    except FileNotFoundError as e:
        raise AssemblageErreur("objdump est introuvable (paquet binutils)") from e
    if r.returncode != 0:
        raise AssemblageErreur(f"objdump -T a échoué sur {binaire} : {r.stderr.strip()}")
    plus_haut = plafond_glibc(r.stdout)
    if plus_haut > PLAFOND_GLIBC:
        raise AssemblageErreur(
            f"le noyau demande la glibc {plus_haut[0]}.{plus_haut[1]}, "
            f"au-delà du plafond {PLAFOND_GLIBC[0]}.{PLAFOND_GLIBC[1]} (Ubuntu 22.04)"
        )
    return plus_haut


def bibliotheques_requises(sortie_objdump_p: str) -> list[str]:
    """Les bibliothèques `NEEDED` lues dans `objdump -p`."""
    return MOTIF_NEEDED.findall(sortie_objdump_p)


def verifier_bibliotheques(binaire: Path) -> list[str]:
    """Échoue si le binaire exige une bibliothèque hors de la liste fermée."""
    try:
        r = subprocess.run(
            ["objdump", "-p", str(binaire)], capture_output=True, text=True, check=False
        )
    except FileNotFoundError as e:
        raise AssemblageErreur("objdump est introuvable (paquet binutils)") from e
    if r.returncode != 0:
        raise AssemblageErreur(f"objdump -p a échoué sur {binaire} : {r.stderr.strip()}")
    requises = bibliotheques_requises(r.stdout)
    en_trop = [b for b in requises if b not in BIBLIOTHEQUES_PERMISES]
    if en_trop:
        raise AssemblageErreur(
            f"le noyau dépend de bibliothèques absentes d'un Ubuntu 22.04 propre : {', '.join(en_trop)}"
        )
    return requises


def verifier_binaire(binaire: Path) -> None:
    if binaire.name != "cassini":
        raise AssemblageErreur(f"le binaire doit s'appeler `cassini`, pas `{binaire.name}`")
    if not binaire.is_file():
        raise AssemblageErreur(f"binaire introuvable : {binaire}")
    with binaire.open("rb") as f:
        if f.read(4) != MAGIE_ELF:
            raise AssemblageErreur(f"{binaire} n'est pas un exécutable ELF")


def sha256_de(chemin: Path) -> str:
    h = hashlib.sha256()
    with chemin.open("rb") as f:
        for bloc in iter(lambda: f.read(1 << 20), b""):
            h.update(bloc)
    return h.hexdigest()


def _neutre(info: tarfile.TarInfo) -> tarfile.TarInfo:
    """Archive reproductible : ni propriétaire ni date, droits fixes."""
    info.uid = info.gid = 0
    info.uname = info.gname = ""
    info.mtime = 0
    info.mode = 0o755 if info.name.endswith(("/cassini", "cassini.sh")) or info.name == "cassini" else 0o644
    return info


def _ecrire_tar(sortie: Path, ajouts: list[tuple[Path, str]], compresse: bool) -> None:
    """Écrit exactement `ajouts` (fichier source, nom dans l'archive), dans cet ordre."""
    sortie.parent.mkdir(parents=True, exist_ok=True)
    with sortie.open("wb") as brut:
        if compresse:
            # mtime fixe : l'en-tête gzip ne doit pas rendre l'archive non reproductible.
            flux = gzip.GzipFile(filename="", mode="wb", fileobj=brut, mtime=0)
        else:
            flux = brut
        try:
            with tarfile.open(fileobj=flux, mode="w", format=tarfile.PAX_FORMAT) as t:
                for source, nom in ajouts:
                    t.add(source, arcname=nom, filter=_neutre, recursive=False)
        finally:
            if compresse:
                flux.close()


def assembler(
    binaire: Path,
    modele: Path,
    script: Path,
    version: str,
    sortie: Path,
    textes: Path,
    nom_modele: str | None = None,
    controler_glibc: bool = True,
) -> list[Path]:
    """Construit les deux archives ; rien n'est écrit si une entrée ne convient pas."""
    if not re.fullmatch(r"\d+\.\d+\.\d+", version):
        raise AssemblageErreur(f"version illisible : {version}")
    verifier_binaire(binaire)
    if controler_glibc:
        verifier_glibc(binaire)
        verifier_bibliotheques(binaire)
    if not script.is_file() or script.name != "cassini.sh":
        raise AssemblageErreur(f"installeur introuvable : {script}")
    if not modele.is_file() or modele.suffix != ".gguf":
        raise AssemblageErreur(f"modèle introuvable ou sans extension .gguf : {modele}")
    absents = [n for n in TEXTES if not (textes / n).is_file()]
    if absents:
        raise AssemblageErreur(f"texte de licences manquant dans {textes} : {', '.join(absents)}")
    nom_modele = nom_modele or modele.name
    if nom_modele != modele.name:
        raise AssemblageErreur(f"le modèle s'appelle {modele.name}, pas {nom_modele}")

    sortie.mkdir(parents=True, exist_ok=True)
    nom_noyau, nom_complet = noms_archives(version)
    tar_noyau, tar_complet = sortie / nom_noyau, sortie / nom_complet

    work = sortie / ".assemblage"
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir()
    try:
        (work / "VERSION").write_text(version + "\n", encoding="utf-8", newline="\n")
        empreintes = (
            f"{sha256_de(binaire)}  noyau/cassini\n"
            f"{sha256_de(modele)}  modele/{modele.name}\n"
        )
        (work / "empreintes.txt").write_text(empreintes, encoding="utf-8", newline="\n")
        _ecrire_tar(
            tar_noyau, [(binaire, "cassini")] + [(textes / n, n) for n in TEXTES], compresse=True
        )
        _ecrire_tar(
            tar_complet,
            [
                (script, "cassini.sh"),
                (work / "VERSION", "VERSION"),
                (work / "empreintes.txt", "empreintes.txt"),
                *[(textes / n, n) for n in TEXTES],
                (binaire, "noyau/cassini"),
                (modele, f"modele/{modele.name}"),
            ],
            compresse=False,
        )
    finally:
        shutil.rmtree(work, ignore_errors=True)
    verifier_archives(tar_noyau, tar_complet, modele.name)
    return [tar_noyau, tar_complet]


def noms_dans(chemin: Path) -> list[str]:
    with tarfile.open(chemin, "r:*") as t:
        return t.getnames()


def verifier_archives(tar_noyau: Path, tar_complet: Path, nom_modele: str) -> None:
    """Les archives contiennent exactement la liste fermée, sans rien d'autre."""
    if noms_dans(tar_noyau) != ["cassini", *TEXTES]:
        raise AssemblageErreur(f"{tar_noyau.name} : contenu inattendu {noms_dans(tar_noyau)}")
    attendus = ["cassini.sh", "VERSION", "empreintes.txt", *TEXTES, "noyau/cassini", f"modele/{nom_modele}"]
    if noms_dans(tar_complet) != attendus:
        raise AssemblageErreur(f"{tar_complet.name} : contenu inattendu {noms_dans(tar_complet)}")


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--binaire", type=Path, required=True, help="le binaire ELF `cassini` (processeur)")
    p.add_argument("--modele", type=Path, required=True, help="le fichier .gguf du modèle")
    p.add_argument("--script", type=Path, default=INSTALLER / "cassini.sh")
    p.add_argument("--textes", type=Path, required=True, help="dossier écrit par installer/licences.py")
    p.add_argument("--version", required=True)
    p.add_argument("--sortie", type=Path, required=True)
    p.add_argument("--sans-controle-glibc", action="store_true", help="essais seulement")
    a = p.parse_args(argv)
    try:
        ref = json.loads((INSTALLER / "modele.json").read_text(encoding="utf-8"))
        if a.modele.name != ref["nom_installe"]:
            raise AssemblageErreur(
                f"le modèle s'appelle {a.modele.name}, installer/modele.json attend {ref['nom_installe']}"
            )
        archives = assembler(
            a.binaire, a.modele, a.script, a.version, a.sortie, a.textes,
            controler_glibc=not a.sans_controle_glibc,
        )
    except (AssemblageErreur, OSError, KeyError, ValueError) as e:
        print(f"archives Linux refusées : {e}", file=sys.stderr)
        return 1
    print("archives assemblées : " + ", ".join(x.name for x in archives))
    return 0


if __name__ == "__main__":
    sys.exit(main())
