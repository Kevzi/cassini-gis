"""Télécharge le modèle publié par 7.3 et vérifie son SHA-256 (ticket 7.2).

Appelé par la recette seulement (jamais par l'installeur, qui n'a aucun code réseau).
Adresse, empreinte et taille sont fixées dans `installer/modele.json`. Échoue, avec un
message en français et sans laisser de fichier, si le modèle est introuvable, altéré ou si
l'espace disque manque.

Usage :
    python -I installer/telecharger_modele.py --sortie modele/cassini-composition-4b-Q4_K_M.gguf
"""

import argparse
import hashlib
import http.client
import json
import re
import shutil
import sys
import urllib.request
from pathlib import Path

DECLARATION = Path(__file__).resolve().parent / "modele.json"
# Place à garder libre en plus du modèle : installeur complet, installation d'essai.
MARGE_DISQUE = 8 * 1024**3


class ModeleErreur(Exception):
    pass


def lire_declaration(chemin: Path = DECLARATION) -> dict:
    d = json.loads(chemin.read_text(encoding="utf-8"))
    for cle in ("url", "sha256", "taille", "nom_installe"):
        if cle not in d:
            raise ModeleErreur(f"{chemin} : champ « {cle} » absent")
    if not isinstance(d["sha256"], str) or not re.fullmatch(r"[0-9a-f]{64}", d["sha256"]):
        raise ModeleErreur(f"{chemin} : SHA-256 invalide (64 caractères hexadécimaux minuscules)")
    if not isinstance(d["taille"], int) or isinstance(d["taille"], bool) or d["taille"] <= 0:
        raise ModeleErreur(f"{chemin} : taille invalide (entier strictement positif)")
    return d


def verifier_espace(dossier: Path, taille: int, marge: int = MARGE_DISQUE) -> None:
    dossier.mkdir(parents=True, exist_ok=True)
    libre = shutil.disk_usage(dossier).free
    if libre < taille + marge:
        raise ModeleErreur(
            f"espace disque insuffisant : {libre} octets libres, {taille + marge} nécessaires"
        )


def telecharger(decl: dict, sortie: Path, marge: int = MARGE_DISQUE) -> None:
    verifier_espace(sortie.parent, decl["taille"], marge)
    h = hashlib.sha256()
    total = 0
    try:
        with urllib.request.urlopen(decl["url"], timeout=60) as r, sortie.open("wb") as f:
            for bloc in iter(lambda: r.read(1 << 20), b""):
                h.update(bloc)
                f.write(bloc)
                total += len(bloc)
    except (OSError, http.client.HTTPException) as e:
        sortie.unlink(missing_ok=True)
        raise ModeleErreur(f"modèle introuvable ou téléchargement interrompu ({decl['url']}) : {e}") from e
    if total != decl["taille"] or h.hexdigest() != decl["sha256"]:
        sortie.unlink(missing_ok=True)
        raise ModeleErreur(
            f"modèle altéré : {total} octets et SHA-256 {h.hexdigest()}, "
            f"attendu {decl['taille']} octets et {decl['sha256']}"
        )


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--declaration", type=Path, default=DECLARATION)
    p.add_argument("--sortie", type=Path, required=True)
    a = p.parse_args(argv)
    try:
        telecharger(lire_declaration(a.declaration), a.sortie)
    except (ModeleErreur, OSError, ValueError) as e:
        print(f"modèle refusé : {e}", file=sys.stderr)
        return 1
    print(f"{a.sortie} : modèle vérifié")
    return 0


if __name__ == "__main__":
    sys.exit(main())
