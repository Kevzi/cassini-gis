"""Télécharge le modèle publié par 7.3 et vérifie son SHA-256 (ticket 7.2).

Appelé par la recette seulement (jamais par l'installeur, qui n'a aucun code réseau).
Adresse, empreinte et taille sont fixées dans `installer/modele.json`. Échoue, avec un
message en français et sans laisser de fichier, si le modèle est introuvable, altéré ou si
l'espace disque manque.

Usage :
    python -I installer/telecharger_modele.py --sortie modele/cassini-composition-4b-0.3.0-Q4_K_M.gguf
"""

import argparse
import hashlib
import http.client
import json
import re
import shutil
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

DECLARATION = Path(__file__).resolve().parent / "modele.json"
# Place à garder libre en plus du modèle : installeur complet, installation d'essai.
MARGE_DISQUE = 8 * 1024**3
# Trois essais au plus, attente croissante (ATTENTE_BASE, puis le double) entre deux essais.
ESSAIS = 3
ATTENTE_BASE = 5.0
# Délai global (essais et attentes compris) : un serveur qui goutte à goutte ne bloque pas la CI.
DELAI_GLOBAL = 3600.0


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


def verifier_adresse(url: str) -> None:
    """https seulement ; `file://` reste accepté pour les tests hors ligne."""
    schema = urllib.parse.urlsplit(url).scheme.lower()
    if schema not in ("https", "file"):
        raise ModeleErreur(f"adresse du modèle refusée (https exigé) : {url}")


class _RedirectionHttps(urllib.request.HTTPRedirectHandler):
    """Refuse toute redirection vers une cible hors https (`file://` toléré, comme à l'entrée)."""

    def redirect_request(self, req, fp, code, msg, headers, newurl):
        verifier_adresse(urllib.parse.urljoin(req.full_url, newurl))
        return super().redirect_request(req, fp, code, msg, headers, newurl)


def _ouvrir(url: str, timeout: float):
    if urllib.parse.urlsplit(url).scheme.lower() == "https":
        return urllib.request.build_opener(_RedirectionHttps).open(url, timeout=timeout)
    return urllib.request.urlopen(url, timeout=timeout)


def _une_fois(decl: dict, sortie: Path, fin: float) -> tuple[int, str]:
    h = hashlib.sha256()
    total = 0
    reste = fin - time.monotonic()
    if reste <= 0:
        raise ModeleErreur("délai global du téléchargement dépassé")
    with _ouvrir(decl["url"], min(60.0, reste)) as r, sortie.open("wb") as f:
        for bloc in iter(lambda: r.read(1 << 20), b""):
            if time.monotonic() > fin:
                raise ModeleErreur("délai global du téléchargement dépassé")
            h.update(bloc)
            f.write(bloc)
            total += len(bloc)
    return total, h.hexdigest()


def telecharger(
    decl: dict,
    sortie: Path,
    marge: int = MARGE_DISQUE,
    delai_global: float | None = None,
    dormir=time.sleep,
) -> None:
    verifier_adresse(decl["url"])
    verifier_espace(sortie.parent, decl["taille"], marge)
    fin = time.monotonic() + (DELAI_GLOBAL if delai_global is None else delai_global)
    total, empreinte = 0, ""
    for essai in range(1, ESSAIS + 1):
        try:
            total, empreinte = _une_fois(decl, sortie, fin)
            break
        except ModeleErreur:
            sortie.unlink(missing_ok=True)
            raise
        except (OSError, http.client.HTTPException) as e:
            sortie.unlink(missing_ok=True)
            if essai == ESSAIS:
                raise ModeleErreur(
                    f"modèle introuvable ou téléchargement interrompu après {ESSAIS} essais "
                    f"({decl['url']}) : {e}"
                ) from e
            attente = ATTENTE_BASE * 2 ** (essai - 1)
            if time.monotonic() + attente > fin:
                raise ModeleErreur(
                    f"délai global du téléchargement dépassé avant le nouvel essai ({decl['url']}) : {e}"
                ) from e
            print(f"téléchargement interrompu ({e}) ; nouvel essai dans {attente:g} s", file=sys.stderr)
            dormir(attente)
    if total != decl["taille"] or empreinte != decl["sha256"]:
        sortie.unlink(missing_ok=True)
        raise ModeleErreur(
            f"modèle altéré : {total} octets et SHA-256 {empreinte}, "
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
