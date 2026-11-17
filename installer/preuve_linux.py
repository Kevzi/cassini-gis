"""Preuve de `cassini.sh` dans la recette Linux (ticket 7.7), sur le noyau et les archives livrés.

Sur un HOME vide et un serveur local (le vrai modèle de 2,8 Go n'est pas servi : un petit fichier
de preuve, sous le nom de `installer/modele.json`) :
  1. porte 1, manifeste valide : le noyau répond à `run system.about`, `model.list` liste le modèle
     et ne signale pas `models.tampered`, `installation.json` porte la version ;
  2. un octet changé dans le noyau, puis dans le modèle : refus, rien n'est posé ;
  3. mise à jour d'une version A vers B : noyau remplacé, ancien modèle gardé, nouveau à côté ;
  4. même manifeste : « à jour », rien téléchargé (hors manifeste) ni réécrit ;
  5. mise à jour sans installation : refus, rien n'est posé ;
  6. porte 2 (si `--complet` est donné) : l'archive complète dépliée s'installe avec un faux curl et
     un faux wget qui échouent et laissent une trace : aucun appel réseau, même résultat.

Usage :
    python -I installer/preuve_linux.py --etiquette v0.1.1 --noyau dist-linux/cassini-noyau-linux-0.1.1.tar.gz \\
        --script installer/cassini.sh [--complet dist-linux/cassini-complet-linux-0.1.1.tar]
"""

import argparse
import hashlib
import http.server
import json
import os
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
from functools import partial
from pathlib import Path

INSTALLER = Path(__file__).resolve().parent
sys.path.insert(0, str(INSTALLER))
import manifeste  # noqa: E402

NOM_ANCIEN = "cassini-composition-4b-Q4_K_M.gguf"


class Echec(Exception):
    pass


def exiger(condition: bool, message: str) -> None:
    if not condition:
        raise Echec(message)


def sha(octets: bytes) -> str:
    return hashlib.sha256(octets).hexdigest()


def changer_un_octet(chemin: Path, position: int = 1000) -> None:
    octets = bytearray(chemin.read_bytes())
    octets[position] ^= 1
    chemin.write_bytes(bytes(octets))


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--etiquette", required=True)
    p.add_argument("--noyau", type=Path, required=True, help="le tar.gz du noyau livré")
    p.add_argument("--script", type=Path, required=True)
    p.add_argument("--complet", type=Path, help="le tar complet : preuve de la porte 2")
    p.add_argument("--prefixe-execution", help="répétition locale seulement : lance le noyau via cet interpréteur (sh)")
    a = p.parse_args(argv)
    sh = shutil.which("sh")
    exiger(sh is not None, "sh introuvable")
    version = a.etiquette.lstrip("v")
    nom_modele = json.loads((INSTALLER / "modele.json").read_text(encoding="utf-8"))["nom_installe"]

    with tempfile.TemporaryDirectory() as t:
        d = Path(t)
        srv = d / "srv"
        srv.mkdir()
        requetes: list[str] = []

        class H(http.server.SimpleHTTPRequestHandler):
            def log_message(self, fmt, *args):
                requetes.append(self.path)

        httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), partial(H, directory=str(srv)))
        base = f"http://127.0.0.1:{httpd.server_address[1]}"
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        try:
            return _preuves(a, sh, d, srv, base, requetes, version, nom_modele)
        except Echec as e:
            print(f"preuve Linux : ECHEC, {e}", file=sys.stderr)
            return 1
        finally:
            httpd.shutdown()
            httpd.server_close()


def _preuves(a, sh, d: Path, srv: Path, base: str, requetes: list, version: str, nom_modele: str) -> int:
    home = d / "home"
    racine = home / ".local" / "share" / "cassini"
    bin_ = racine / "bin"

    modeles = {NOM_ANCIEN: os.urandom(1 << 20), nom_modele: os.urandom(1 << 20)}
    for nom, octets in modeles.items():
        (srv / nom).write_bytes(octets)
    dummy_complet = d / "cassini-complet-linux-factice.tar"
    dummy_complet.write_bytes(b"jamais servi")

    def publier(dossier: str, ver: str, nom: str, alterer_noyau=False, alterer_modele=False) -> str:
        sous = srv / dossier
        sous.mkdir()
        tar = sous / a.noyau.name
        shutil.copy2(a.noyau, tar)
        octets = modeles[nom]
        modele = {"nom": nom, "url": f"{base}/{nom}", "sha256": sha(octets), "taille": len(octets)}
        linux = manifeste.section_linux(tar, dummy_complet, f"{base}/{dossier}", f"{base}/{dossier}")
        if alterer_noyau:
            s = linux["noyau"]["sha256"]
            linux["noyau"]["sha256"] = ("0" if s[0] != "0" else "1") + s[1:]
        if alterer_modele:
            s = modele["sha256"]
            modele["sha256"] = ("0" if s[0] != "0" else "1") + s[1:]
        porte2 = {"nom": "x", "url": f"{base}/x", "sha256": "0" * 64, "taille": 1}
        texte = manifeste.construire(ver, [tar], f"{base}/{dossier}", modele, porte2, linux)
        (sous / "manifeste.json").write_text(texte, encoding="utf-8", newline="\n")
        return f"{base}/{dossier}/manifeste.json"

    def lancer(*args, script=a.script, cwd=None, path_avant=None):
        env = dict(os.environ, HOME=str(home), NO_PROXY="127.0.0.1", no_proxy="127.0.0.1")
        for k in ("http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "CASSINI_MANIFESTE"):
            env.pop(k, None)
        if path_avant:
            env["PATH"] = str(path_avant) + os.pathsep + env["PATH"]
        return subprocess.run(
            [sh, str(script), *args], env=env, cwd=cwd, capture_output=True, text=True,
            stdin=subprocess.DEVNULL, timeout=1800,
        )

    def noyau(*args) -> subprocess.CompletedProcess:
        cmd = [str(bin_ / "cassini"), *args]
        if a.prefixe_execution:
            cmd = [a.prefixe_execution, *cmd]
        env = dict(os.environ, HOME=str(home))
        return subprocess.run(cmd, capture_output=True, text=True, env=env, timeout=300)

    def verifier_noyau_et_modele(nom: str, ver: str) -> None:
        r = noyau("run", "system.about")
        exiger(r.returncode == 0, f"run system.about : code {r.returncode} {r.stderr}")
        print(r.stdout.strip())
        r = noyau("run", "model.list")
        exiger(r.returncode == 0, f"run model.list : code {r.returncode} {r.stderr}")
        print(r.stdout.strip())
        exiger("models.tampered" not in r.stdout + r.stderr, "model.list signale models.tampered")
        exiger(nom in r.stdout, f"model.list ne liste pas {nom}")
        info = json.loads((bin_ / "installation.json").read_text(encoding="utf-8"))
        exiger(info["version"] == ver and info["installe_le"], f"installation.json incorrect : {info}")

    def rien_pose() -> bool:
        return (
            not (bin_ / "cassini").exists()
            and not (bin_ / "installation.json").exists()
            and not any((bin_ / "models").glob("*") if (bin_ / "models").exists() else [])
        )

    def etat() -> dict:
        return {
            str(p.relative_to(racine)): (sha(p.read_bytes()), p.stat().st_mtime_ns)
            for p in sorted(racine.rglob("*"))
            if p.is_file() and ".cassini-tmp" not in p.parts
        }

    # 1. Porte 1, manifeste valide.
    valide = publier("valide", version, nom_modele)
    r = lancer("--manifeste", valide)
    exiger(r.returncode == 0, f"installation valide : code {r.returncode}\n{r.stdout}{r.stderr}")
    exiger(
        sha((bin_ / "models" / nom_modele).read_bytes()) == sha(modeles[nom_modele]),
        "modèle installé différent",
    )
    decl = json.loads((bin_ / "models" / "cassini-composition-4b.modele.json").read_text(encoding="utf-8"))
    exiger(
        decl == {"model": nom_modele, "prompt": "short", "model_sha256": sha(modeles[nom_modele])},
        f"déclaration incorrecte : {decl}",
    )
    verifier_noyau_et_modele(nom_modele, version)
    print("1. porte 1 : installée, noyau et modèle répondent")

    # 2. Octet changé : noyau, puis modèle. HOME neuf à chaque fois.
    for quoi, kw in (("noyau", {"alterer_noyau": True}), ("modèle", {"alterer_modele": True})):
        shutil.rmtree(home)
        home.mkdir()
        manif = publier(f"altere-{quoi[0]}", version, nom_modele, **kw)
        r = lancer("--manifeste", manif)
        exiger(r.returncode != 0, f"{quoi} altéré : l'installation aurait dû échouer")
        exiger(rien_pose(), f"{quoi} altéré : un fichier a été posé")
        print(f"2. {quoi} altéré refusé : {r.stderr.strip().splitlines()[-1]}")

    # 3 à 5. Mise à jour A vers B, puis à jour, puis non installé.
    shutil.rmtree(home)
    home.mkdir()
    manif_a = publier("a", "0.0.1", NOM_ANCIEN)
    manif_b = publier("b", "0.0.2", nom_modele)
    r = lancer("--mise-a-jour", "--manifeste", manif_b)
    exiger(r.returncode != 0 and rien_pose() and not (home / ".local").exists(), "mise à jour sans installation : doit refuser sans rien poser")
    print("5. mise à jour sans installation refusée")
    exiger(lancer("--manifeste", manif_a).returncode == 0, "installation de A")
    decl_a = (bin_ / "models" / "cassini-composition-4b.modele.json").read_bytes()
    r = lancer("--mise-a-jour", "--manifeste", manif_b)
    exiger(r.returncode == 0, f"mise à jour : code {r.returncode}\n{r.stdout}{r.stderr}")
    exiger(sha((bin_ / "models" / NOM_ANCIEN).read_bytes()) == sha(modeles[NOM_ANCIEN]), "ancien modèle changé")
    exiger(sha((bin_ / "models" / nom_modele).read_bytes()) == sha(modeles[nom_modele]), "nouveau modèle incorrect")
    exiger((bin_ / "models" / "cassini-composition-4b.modele.json").read_bytes() == decl_a, "ancienne déclaration changée")
    exiger((bin_ / "models" / f"{nom_modele[:-5]}.modele.json").is_file(), "déclaration du nouveau modèle absente")
    verifier_noyau_et_modele(nom_modele, "0.0.2")
    print("3. mise à jour A vers B : ancien modèle gardé, nouveau à côté")
    avant = etat()
    requetes.clear()
    r = lancer("--mise-a-jour", "--manifeste", manif_b)
    exiger(r.returncode == 0 and "a jour" in r.stdout, f"à jour : code {r.returncode}\n{r.stdout}{r.stderr}")
    exiger(requetes == ["/b/manifeste.json"], f"à jour : requêtes inattendues {requetes}")
    exiger(etat() == avant, "à jour : un fichier a été réécrit")
    print("4. déjà à jour : rien téléchargé ni réécrit")

    # 6. Porte 2 : archive complète dépliée, réseau « coupé » par de faux curl et wget.
    if a.complet:
        shutil.rmtree(home)
        home.mkdir()
        dossier = d / "depliee"
        dossier.mkdir()
        with tarfile.open(a.complet) as t:
            t.extractall(dossier)
        faux = d / "fauxbin"
        faux.mkdir()
        trace = d / "reseau.trace"
        for nom in ("curl", "wget"):
            f = faux / nom
            f.write_text(f'#!/bin/sh\necho {nom} "$@" >> "{trace}"\nexit 99\n', encoding="utf-8", newline="\n")
            f.chmod(0o755)
        requetes.clear()
        r = lancer(script=dossier / "cassini.sh", cwd=dossier, path_avant=faux)
        exiger(r.returncode == 0, f"porte 2 : code {r.returncode}\n{r.stdout}{r.stderr}")
        exiger(not trace.exists(), "porte 2 : curl ou wget a été appelé")
        exiger(requetes == [], "porte 2 : requête réseau reçue")
        verifier_noyau_et_modele(nom_modele, version)
        # Place disque : l'archive dépliée (modèle compris) n'est gardée que pour l'essai altéré,
        # qui n'a besoin que du noyau.
        shutil.rmtree(dossier / "modele")
        shutil.rmtree(home)
        home.mkdir()
        print("6. porte 2 : installée sans aucun appel réseau")
        # Un octet changé dans l'archive dépliée : refus.
        changer_un_octet(dossier / "noyau" / "cassini", 10)
        (dossier / "modele").mkdir()
        (dossier / "modele" / nom_modele).write_bytes(b"x")
        r = lancer("--sans-modele", script=dossier / "cassini.sh", cwd=dossier, path_avant=faux)
        exiger(r.returncode != 0 and "noyau/cassini" in r.stderr and rien_pose(), "porte 2 altérée : doit être refusée")
        print("6 bis. porte 2 altérée refusée")

    print("preuve Linux : OK")
    return 0


if __name__ == "__main__":
    sys.exit(main())
