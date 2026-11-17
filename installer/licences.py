"""Écrit les licences de tout ce qui est livré (ticket 9, R7) : LICENCES-TIERCES.txt et FRONTIERE-GPL.txt.

LICENCES-TIERCES.txt liste, triées, les crates du noyau (`core`, `ai`, `cli`, sans `cassini-app`,
comme `deny.toml`) lues dans `cargo metadata`, puis une liste fermée de composants hors Cargo
(llama.cpp, le modèle Qwen, l'outil des installeurs). FRONTIERE-GPL.txt est la copie de
`docs/frontiere-gpl.md`. Bibliothèque standard de Python et `cargo metadata` seulement, aucun
réseau, sortie déterministe (aucune date).

Échoue, code non nul, si une crate livrée n'a pas de licence déclarée, ou si sa licence impose
le copyleft (GPL, LGPL, AGPL) sans alternative permissive : la crate est nommée.

Usage :
    python -I installer/licences.py --sortie dist/licences
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

RACINE = Path(__file__).resolve().parent.parent
RACINES_NOYAU = ("cassini-core", "cassini-ai", "cassini")  # `cassini-app` (V2) n'est pas livré
NOM_TIERCES = "LICENCES-TIERCES.txt"
NOM_FRONTIERE = "FRONTIERE-GPL.txt"
SOURCE_FRONTIERE = RACINE / "docs" / "frontiere-gpl.md"
COPYLEFT = re.compile(r"^(A|L)?GPL", re.I)

# Composants livrés qui ne passent pas par Cargo : liste fermée (nom, version, licence, remarque).
HORS_CARGO = (
    ("llama.cpp", "tirée par llama-cpp-sys-2", "MIT",
     "moteur d'inférence, compilé dans le noyau ; DLL ggml et llama sur Windows"),
    ("Qwen3.5-4B (modèle modifié)", "voir le dépôt du modèle", "Apache-2.0",
     "modèle ajusté, converti en GGUF ; NOTICE du modèle : mention des modifications"),
    ("Inno Setup", "6 (poste GitHub)", "licence Inno Setup (permissive)",
     "outil des installeurs Windows ; son lanceur est embarqué dans les .exe ; non modifié"),
    ("Vulkan SDK", "1.4.363.0", "composants libres, majorité MIT ou Apache-2.0",
     "outil de construction, jamais livré ; shaders de ggml-vulkan.dll compilés par glslc"),
)


class LicencesErreur(Exception):
    pass


def alternatives(expression: str) -> list[set[str]]:
    """Les choix possibles d'une expression SPDX : chaque choix est l'ensemble des licences à respecter."""
    jetons = re.findall(r"\(|\)|[^\s()]+", expression.replace("/", " OR "))
    pos = 0

    def ou() -> list[set[str]]:
        nonlocal pos
        res = et()
        while pos < len(jetons) and jetons[pos].upper() == "OR":
            pos += 1
            res = res + et()
        return res

    def et() -> list[set[str]]:
        nonlocal pos
        res = atome()
        while pos < len(jetons) and jetons[pos].upper() == "AND":
            pos += 1
            suite = atome()
            res = [a | b for a in res for b in suite]
        return res

    def atome() -> list[set[str]]:
        nonlocal pos
        if pos >= len(jetons):
            raise LicencesErreur(f"expression de licence illisible : {expression!r}")
        j = jetons[pos]
        pos += 1
        if j == "(":
            res = ou()
            if pos >= len(jetons) or jetons[pos] != ")":
                raise LicencesErreur(f"expression de licence illisible : {expression!r}")
            pos += 1
            return res
        if j in (")",) or j.upper() in ("AND", "OR"):
            raise LicencesErreur(f"expression de licence illisible : {expression!r}")
        if j.upper() == "WITH":
            raise LicencesErreur(f"expression de licence illisible : {expression!r}")
        # « GPL-2.0 WITH exception » : l'exception suit son identifiant.
        if pos + 1 < len(jetons) and jetons[pos].upper() == "WITH":
            pos += 2
        return [{j}]

    res = ou()
    if pos != len(jetons):
        raise LicencesErreur(f"expression de licence illisible : {expression!r}")
    return res


def permissive_possible(expression: str) -> bool:
    """Vrai s'il existe un choix de l'expression sans aucune licence copyleft."""
    return any(not any(COPYLEFT.match(l) for l in choix) for choix in alternatives(expression))


def crates_livrees(meta: dict) -> list[dict]:
    """Les crates atteintes depuis le noyau par des dépendances normales (ni dev ni build)."""
    paquets = {p["id"]: p for p in meta["packages"]}
    noeuds = {n["id"]: n for n in meta["resolve"]["nodes"]}
    racines = [p["id"] for p in meta["packages"] if p["name"] in RACINES_NOYAU and p["source"] is None]
    if not racines:
        raise LicencesErreur("crates du noyau introuvables dans cargo metadata")
    vus: set[str] = set()
    pile = list(racines)
    while pile:
        i = pile.pop()
        if i in vus:
            continue
        vus.add(i)
        for d in noeuds[i]["deps"]:
            if any(k.get("kind") is None for k in d["dep_kinds"]):
                pile.append(d["pkg"])
    return [paquets[i] for i in vus]


def lignes_crates(meta: dict) -> list[tuple[str, str, str]]:
    """(nom, version, SPDX) trié ; lève LicencesErreur au premier problème, avec la crate nommée."""
    manques, copyleft, lignes = [], [], []
    for p in crates_livrees(meta):
        lic = (p.get("license") or "").strip()
        if not lic:
            manques.append(f"{p['name']} {p['version']}")
            continue
        if not permissive_possible(lic):
            copyleft.append(f"{p['name']} {p['version']} ({lic})")
            continue
        lignes.append((p["name"], p["version"], lic))
    if manques:
        raise LicencesErreur("crate livrée sans licence déclarée : " + ", ".join(sorted(manques)))
    if copyleft:
        raise LicencesErreur("crate livrée sous licence copyleft (GPL, LGPL ou AGPL) : " + ", ".join(sorted(copyleft)))
    return sorted(lignes)


def lire_metadata(racine: Path = RACINE) -> dict:
    r = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--offline"],
        cwd=racine, capture_output=True, check=False,
    )
    if r.returncode != 0:
        raise LicencesErreur("cargo metadata a échoué : " + r.stderr.decode("utf-8", "replace").strip())
    return json.loads(r.stdout.decode("utf-8"))


def rendre_tierces(lignes: list[tuple[str, str, str]]) -> str:
    sortie = [
        "LICENCES DES COMPOSANTS LIVRES AVEC CASSINI",
        "",
        "Le noyau Cassini est sous licence Apache-2.0 (fichier LICENSE). L'extension QGIS est sous",
        "licence GPL, comme QGIS : elle est livree separement et ne contient aucun binaire (voir",
        f"{NOM_FRONTIERE}). Les avis propres au noyau (ColorBrewer, WMM2025) sont dans le fichier NOTICE.",
        "",
        f"== Crates Rust du noyau ({len(lignes)}) ==",
        "nom version licence SPDX",
        "",
    ]
    sortie += [f"{n} {v} {l}" for n, v, l in lignes]
    sortie += ["", f"== Composants hors Cargo ({len(HORS_CARGO)}) ==", ""]
    for nom, version, lic, remarque in HORS_CARGO:
        sortie += [f"{nom} | {version} | {lic}", f"    {remarque}"]
    return "\n".join(sortie) + "\n"


def rendre_frontiere() -> str:
    if not SOURCE_FRONTIERE.is_file():
        raise LicencesErreur(f"texte de la frontière introuvable : {SOURCE_FRONTIERE}")
    texte = SOURCE_FRONTIERE.read_text(encoding="utf-8-sig").replace("\r\n", "\n").replace("\r", "\n")
    return texte if texte.endswith("\n") else texte + "\n"


def ecrire(sortie: Path, tierces: str, frontiere: str) -> list[Path]:
    sortie.mkdir(parents=True, exist_ok=True)
    chemins = []
    for nom, texte in ((NOM_TIERCES, tierces), (NOM_FRONTIERE, frontiere)):
        chemin = sortie / nom
        chemin.write_bytes(texte.encode("utf-8"))
        chemins.append(chemin)
    return chemins


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--sortie", type=Path, required=True)
    a = p.parse_args(argv)
    try:
        tierces = rendre_tierces(lignes_crates(lire_metadata()))
        chemins = ecrire(a.sortie, tierces, rendre_frontiere())
    except (LicencesErreur, OSError, ValueError, FileNotFoundError) as e:
        print(f"licences refusées : {e}", file=sys.stderr)
        return 1
    print("licences écrites : " + ", ".join(c.name for c in chemins))
    return 0


if __name__ == "__main__":
    sys.exit(main())
