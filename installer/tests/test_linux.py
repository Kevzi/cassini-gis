"""Gardes de `cassini.sh` et de `assembler_linux.py` (ticket 7.7).

Gardes textuelles du script, table de versions, puis essai réel avec `sh` sur un HOME temporaire :
installation par la porte 1 (serveur local), porte 2 (réseau coupé par des faux curl/wget qui
laissent une trace), octet changé, mise à jour, déjà à jour, non installé. L'essai réel est ignoré
quand `sh`, `tar`, `sha256sum` ou `curl` manquent (poste Windows sans Git Bash, par exemple).
Variable CASSINI_SH : interpréteur à essayer à la place de `sh` (par exemple `dash`).
"""

import hashlib
import http.server
import io
import json
import os
import re
import shutil
import subprocess
import sys
import tarfile
import tempfile
import threading
import unittest
from functools import partial
from pathlib import Path

INSTALLER = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(INSTALLER))
import assembler_linux  # noqa: E402
import manifeste  # noqa: E402

SCRIPT = INSTALLER / "cassini.sh"
TEXTE = SCRIPT.read_text(encoding="utf-8")
SH = os.environ.get("CASSINI_SH") or shutil.which("sh")
OUTILS = all(shutil.which(o) for o in ("tar", "sha256sum", "curl"))
EN_LIGNE = (INSTALLER / "cassini-en-ligne.iss").read_text(encoding="utf-8-sig")

NOM_ANCIEN = "cassini-composition-4b-Q4_K_M.gguf"
NOM_NOUVEAU = "cassini-composition-4b-0.2.0-Q4_K_M.gguf"


def sha(octets: bytes) -> str:
    return hashlib.sha256(octets).hexdigest()


class TestGardesTextuelles(unittest.TestCase):
    def test_premiere_ligne_et_fins_de_ligne(self):
        self.assertTrue(TEXTE.startswith("#!/bin/sh\n"))
        self.assertNotIn("\r", TEXTE)

    def test_pas_de_root_ni_de_dependance_en_plus(self):
        code = "\n".join(l for l in TEXTE.splitlines() if not l.lstrip().startswith("#"))
        for interdit in ("sudo", "jq ", "python", "perl", "awk", "/usr/", "/opt/", "/etc/"):
            self.assertNotIn(interdit, code, interdit)
        self.assertIn('RACINE="$HOME/.local/share/cassini"', code)
        self.assertIn('BIN="$RACINE/bin"', code)
        self.assertNotIn("id -u", code)

    def test_empreintes_et_pose_atomique(self):
        self.assertIn("sha256sum", TEXTE)
        self.assertIn('mv -f "$BIN/.cassini.nouveau" "$BIN/cassini"', TEXTE)
        # Aucun modèle n'est supprimé ni écrasé : pas de rm dans models, pose seulement si absent.
        self.assertNotIn('rm -rf "$MODELES', TEXTE)
        self.assertNotIn('rm -f "$MODELES/$MODELE_NOM"', TEXTE)
        self.assertIn('[ ! -e "$MODELES/$MODELE_NOM" ]', TEXTE)

    def test_memes_formats_que_l_installeur_windows(self):
        decl = re.search(r"DeclarationModele = '([^']+)'", EN_LIGNE).group(1)
        self.assertIn(f'DECLARATION="{decl}"', TEXTE)
        self.assertIn('{"model": "%s", "prompt": "short", "model_sha256": "%s"}', TEXTE)
        self.assertIn('{"version": "%s", "installe_le": "%s"}', TEXTE)
        self.assertIn("date -u +%Y-%m-%dT%H:%M:%SZ", TEXTE)
        defaut = re.search(r'#define ManifesteParDefaut "([^"]+)"', EN_LIGNE).group(1)
        self.assertIn(f'MANIFESTE_DEFAUT="{defaut}"', TEXTE)

    def test_lit_la_section_linux_du_manifeste(self):
        self.assertIn("bloc linux", TEXTE)
        self.assertIn("bloc noyau", TEXTE)
        self.assertIn("bloc modele", TEXTE)

    def test_aucun_nom_du_manifeste_n_est_pris_sans_controle(self):
        self.assertIn("nom_sain", TEXTE)


@unittest.skipUnless(SH and OUTILS, "sh, tar, sha256sum et curl requis")
class TestTableDeVersions(unittest.TestCase):
    def comparer(self, a: str, b: str) -> str:
        i = TEXTE.index("# Une partie (1 a 3)")
        j = TEXTE.index("# Lecture du manifeste")
        prog = TEXTE[i:j] + '\ncomparer "$1" "$2"\n'
        r = subprocess.run([SH, "-c", prog, "x", a, b], capture_output=True, text=True)
        self.assertEqual(r.returncode, 0, r.stderr)
        return r.stdout.strip()

    def test_table(self):
        table = [
            ("0.1.0", "0.2.0", "-1"),
            ("0.2.0", "0.1.0", "1"),
            ("0.1.1", "0.1.1", "0"),
            ("0.1.9", "0.1.10", "-1"),  # comparaison de nombres, pas de textes
            ("0.10.0", "0.9.0", "1"),
            ("1.0", "1.0.0", "0"),  # partie absente = 0
            ("1", "1.0.1", "-1"),
            ("0.01.0", "0.1.0", "0"),  # zéros de tête
            ("0.1.0", "", "?"),
            ("abc", "0.1.0", "?"),
            ("0.1.0-rc1", "0.1.0", "?"),
        ]
        for a, b, attendu in table:
            with self.subTest(a=a, b=b):
                self.assertEqual(self.comparer(a, b), attendu)


class Serveur:
    """Serveur HTTP local ; `requetes` garde le chemin de chaque requête reçue."""

    def __init__(self, dossier: Path):
        self.requetes: list[str] = []
        outer = self

        class H(http.server.SimpleHTTPRequestHandler):
            def log_message(self, fmt, *args):
                outer.requetes.append(self.path)

        self.httpd = http.server.ThreadingHTTPServer(("127.0.0.1", 0), partial(H, directory=str(dossier)))
        self.base = f"http://127.0.0.1:{self.httpd.server_address[1]}"
        self.t = threading.Thread(target=self.httpd.serve_forever, daemon=True)
        self.t.start()

    def arreter(self):
        self.httpd.shutdown()
        self.httpd.server_close()


def tar_noyau(chemin: Path, contenu: bytes) -> None:
    with tarfile.open(chemin, "w:gz") as t:
        info = tarfile.TarInfo("cassini")
        info.size = len(contenu)
        info.mode = 0o755
        t.addfile(info, io.BytesIO(contenu))


@unittest.skipUnless(SH and OUTILS, "sh, tar, sha256sum et curl requis")
class TestEssaiReel(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.addCleanup(self._t.cleanup)
        self.d = Path(self._t.name)
        self.home = self.d / "home"
        self.home.mkdir()
        self.srv_dir = self.d / "srv"
        self.srv_dir.mkdir()
        self.srv = Serveur(self.srv_dir)
        self.addCleanup(self.srv.arreter)
        self.modeles = {
            NOM_ANCIEN: b"ancien modele " * 1000,
            NOM_NOUVEAU: b"nouveau modele " * 1000,
        }
        for nom, octets in self.modeles.items():
            (self.srv_dir / nom).write_bytes(octets)

    # -- fabrication d'un manifeste local -----------------------------------------------------

    def publier(self, version: str, nom_modele: str, dossier: str, alterer_noyau=False, alterer_modele=False):
        """Pose noyau et manifeste sous srv/<dossier>/ ; rend l'adresse du manifeste."""
        sous = self.srv_dir / dossier
        sous.mkdir(exist_ok=True)
        noyau = f"#!/bin/sh\necho cassini {version}\n".encode()
        tar = sous / f"cassini-noyau-linux-{version}.tar.gz"
        tar_noyau(tar, noyau)
        complet = sous / f"cassini-complet-linux-{version}.tar"
        complet.write_bytes(b"pas servi dans l'essai")
        base = f"{self.srv.base}/{dossier}"
        octets = self.modeles[nom_modele]
        modele = {
            "nom": nom_modele,
            "url": f"{self.srv.base}/{nom_modele}",
            "sha256": sha(octets),
            "taille": len(octets),
        }
        linux = manifeste.section_linux(tar, complet, base, base)
        if alterer_noyau:
            linux["noyau"]["sha256"] = ("0" if linux["noyau"]["sha256"][0] != "0" else "1") + linux["noyau"]["sha256"][1:]
        if alterer_modele:
            modele["sha256"] = ("0" if modele["sha256"][0] != "0" else "1") + modele["sha256"][1:]
        porte2 = {"nom": "x.exe", "url": base + "/x.exe", "sha256": "0" * 64, "taille": 1}
        texte = manifeste.construire(version, [tar], base, modele, porte2, linux)
        (sous / "manifeste.json").write_text(texte, encoding="utf-8", newline="\n")
        return f"{base}/manifeste.json"

    def lancer(self, *args, cwd=None, extra_path=None, script=SCRIPT):
        env = dict(os.environ)
        env["HOME"] = self.home.as_posix()
        env["NO_PROXY"] = "127.0.0.1"
        env["no_proxy"] = "127.0.0.1"
        for k in ("http_proxy", "https_proxy", "HTTP_PROXY", "HTTPS_PROXY", "CASSINI_MANIFESTE"):
            env.pop(k, None)
        if extra_path:
            env["PATH"] = str(extra_path) + os.pathsep + env["PATH"]
        return subprocess.run(
            [SH, Path(script).as_posix(), *args],
            capture_output=True, text=True, env=env, cwd=cwd, stdin=subprocess.DEVNULL, timeout=120,
        )

    @property
    def racine(self) -> Path:
        return self.home / ".local" / "share" / "cassini"

    def instantane(self) -> dict:
        """Empreinte et date de chaque fichier installé (hors dossier temporaire)."""
        out = {}
        for p in sorted(self.racine.rglob("*")):
            if p.is_file() and ".cassini-tmp" not in p.parts:
                out[str(p.relative_to(self.racine))] = (sha(p.read_bytes()), p.stat().st_mtime_ns)
        return out

    def aucun_fichier_pose(self):
        bin_ = self.racine / "bin"
        self.assertFalse((bin_ / "cassini").exists())
        self.assertFalse((bin_ / "installation.json").exists())
        self.assertEqual([p for p in (bin_ / "models").glob("*")] if (bin_ / "models").exists() else [], [])
        self.assertEqual([p.name for p in self.racine.glob(".cassini-tmp*")], [])

    # -- porte 1 ------------------------------------------------------------------------------

    def test_porte_1_installation(self):
        url = self.publier("0.1.1", NOM_NOUVEAU, "v1")
        r = self.lancer("--manifeste", url)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        bin_ = self.racine / "bin"
        self.assertEqual((bin_ / "cassini").read_bytes(), b"#!/bin/sh\necho cassini 0.1.1\n")
        if os.name == "posix":
            self.assertTrue(os.access(bin_ / "cassini", os.X_OK))
        self.assertEqual((bin_ / "models" / NOM_NOUVEAU).read_bytes(), self.modeles[NOM_NOUVEAU])
        decl = json.loads((bin_ / "models" / "cassini-composition-4b.modele.json").read_text(encoding="utf-8"))
        self.assertEqual(decl, {"model": NOM_NOUVEAU, "prompt": "short", "model_sha256": sha(self.modeles[NOM_NOUVEAU])})
        brut = (bin_ / "installation.json").read_text(encoding="utf-8")
        info = json.loads(brut)
        self.assertEqual(list(info), ["version", "installe_le"])
        self.assertEqual(info["version"], "0.1.1")
        self.assertRegex(info["installe_le"], r"^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$")
        self.assertTrue((self.racine / "mettre-a-jour-cassini").is_file())
        self.assertEqual([p.name for p in self.racine.glob(".cassini-tmp*")], [])
        self.assertEqual([p.name for p in (bin_ / "models").glob(".*")], [])

    def test_porte_1_sans_modele(self):
        url = self.publier("0.1.1", NOM_NOUVEAU, "v1")
        r = self.lancer("--manifeste", url, "--sans-modele")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertTrue((self.racine / "bin" / "cassini").is_file())
        self.assertEqual(list((self.racine / "bin" / "models").glob("*")), [])
        self.assertNotIn(f"/{NOM_NOUVEAU}", self.srv.requetes)

    def test_octet_change_dans_le_noyau(self):
        url = self.publier("0.1.1", NOM_NOUVEAU, "v1", alterer_noyau=True)
        r = self.lancer("--manifeste", url)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("cassini-noyau-linux-0.1.1.tar.gz", r.stderr)
        self.aucun_fichier_pose()

    def test_octet_change_dans_le_modele_refuse_tout_noyau_compris(self):
        url = self.publier("0.1.1", NOM_NOUVEAU, "v1", alterer_modele=True)
        r = self.lancer("--manifeste", url)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn(NOM_NOUVEAU, r.stderr)
        self.aucun_fichier_pose()

    def test_manifeste_injoignable(self):
        r = self.lancer("--manifeste", "http://127.0.0.1:9/manifeste.json")
        self.assertNotEqual(r.returncode, 0)
        self.aucun_fichier_pose()

    def test_manifeste_sans_section_linux(self):
        sous = self.srv_dir / "sans"
        sous.mkdir()
        (sous / "manifeste.json").write_text(
            json.dumps({"version": "0.1.1", "fichiers": []}, indent=2) + "\n", encoding="utf-8"
        )
        r = self.lancer("--manifeste", f"{self.srv.base}/sans/manifeste.json")
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("Linux", r.stderr)
        self.aucun_fichier_pose()

    # -- mise a jour --------------------------------------------------------------------------

    def test_mise_a_jour_garde_l_ancien_modele(self):
        a = self.publier("0.1.0", NOM_ANCIEN, "a")
        b = self.publier("0.2.0", NOM_NOUVEAU, "b")
        r = self.lancer("--manifeste", a)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        decl_avant = (self.racine / "bin" / "models" / "cassini-composition-4b.modele.json").read_bytes()
        r = self.lancer("--mise-a-jour", "--manifeste", b)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        bin_ = self.racine / "bin"
        self.assertEqual((bin_ / "cassini").read_bytes(), b"#!/bin/sh\necho cassini 0.2.0\n")
        self.assertEqual((bin_ / "models" / NOM_ANCIEN).read_bytes(), self.modeles[NOM_ANCIEN])
        self.assertEqual((bin_ / "models" / NOM_NOUVEAU).read_bytes(), self.modeles[NOM_NOUVEAU])
        self.assertEqual((bin_ / "models" / "cassini-composition-4b.modele.json").read_bytes(), decl_avant)
        nouvelle = json.loads((bin_ / "models" / f"{NOM_NOUVEAU[:-5]}.modele.json").read_text(encoding="utf-8"))
        self.assertEqual(nouvelle["model"], NOM_NOUVEAU)
        self.assertEqual(nouvelle["model_sha256"], sha(self.modeles[NOM_NOUVEAU]))
        self.assertEqual(json.loads((bin_ / "installation.json").read_text(encoding="utf-8"))["version"], "0.2.0")

    def test_deja_a_jour_ne_telecharge_ni_ne_reecrit_rien(self):
        b = self.publier("0.2.0", NOM_NOUVEAU, "b")
        self.assertEqual(self.lancer("--manifeste", b).returncode, 0)
        avant = self.instantane()
        self.srv.requetes.clear()
        r = self.lancer("--mise-a-jour", "--manifeste", b)
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertIn("a jour", r.stdout)
        self.assertEqual(self.srv.requetes, ["/b/manifeste.json"])
        self.assertEqual(self.instantane(), avant)

    def test_version_installee_plus_haute_vaut_a_jour(self):
        a = self.publier("0.1.0", NOM_ANCIEN, "a")
        b = self.publier("0.2.0", NOM_NOUVEAU, "b")
        self.assertEqual(self.lancer("--manifeste", b).returncode, 0)
        avant = self.instantane()
        r = self.lancer("--mise-a-jour", "--manifeste", a)
        self.assertEqual(r.returncode, 0)
        self.assertEqual(self.instantane(), avant)

    def test_mise_a_jour_refusee_si_octet_change_et_rien_n_est_modifie(self):
        a = self.publier("0.1.0", NOM_ANCIEN, "a")
        b = self.publier("0.2.0", NOM_NOUVEAU, "b", alterer_modele=True)
        self.assertEqual(self.lancer("--manifeste", a).returncode, 0)
        avant = self.instantane()
        r = self.lancer("--mise-a-jour", "--manifeste", b)
        self.assertNotEqual(r.returncode, 0)
        self.assertEqual(self.instantane(), avant)

    def test_mise_a_jour_sans_installation(self):
        b = self.publier("0.2.0", NOM_NOUVEAU, "b")
        r = self.lancer("--mise-a-jour", "--manifeste", b)
        self.assertNotEqual(r.returncode, 0)
        self.assertIn("pas installe", r.stderr)
        self.assertFalse((self.home / ".local").exists())
        self.assertEqual(self.srv.requetes, [])

    def test_copie_du_script_relance_en_mise_a_jour(self):
        a = self.publier("0.1.0", NOM_ANCIEN, "a")
        b = self.publier("0.2.0", NOM_NOUVEAU, "b")
        self.assertEqual(self.lancer("--manifeste", a).returncode, 0)
        r = self.lancer("--mise-a-jour", "--manifeste", b, script=self.racine / "mettre-a-jour-cassini")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertEqual((self.racine / "bin" / "cassini").read_bytes(), b"#!/bin/sh\necho cassini 0.2.0\n")

    # -- porte 2 ------------------------------------------------------------------------------

    def archive_complete(self, version="0.1.1") -> Path:
        """Une archive complète dépliée, faite par l'assembleur (binaire factice, glibc non contrôlée)."""
        binaire = self.d / "src" / "cassini"
        binaire.parent.mkdir(exist_ok=True)
        binaire.write_bytes(b"\x7fELF" + f"#!/bin/sh\necho cassini {version}\n".encode())
        modele = self.d / "src" / NOM_NOUVEAU
        modele.write_bytes(self.modeles[NOM_NOUVEAU])
        sortie = self.d / "dist"
        _, complet = (
            assembler_linux.assembler(binaire, modele, SCRIPT, version, sortie, controler_glibc=False)
        )
        dossier = self.d / "depliee"
        dossier.mkdir()
        with tarfile.open(complet) as t:
            t.extractall(dossier)
        return dossier

    def faux_reseau(self) -> tuple[Path, Path]:
        """Des curl et wget qui laissent une trace et échouent : la porte 2 ne doit pas les appeler."""
        bin_ = self.d / "fauxbin"
        bin_.mkdir()
        trace = self.d / "reseau.trace"
        for nom in ("curl", "wget"):
            f = bin_ / nom
            f.write_text(f'#!/bin/sh\necho {nom} "$@" >> "{trace.as_posix()}"\nexit 99\n', encoding="utf-8", newline="\n")
            f.chmod(0o755)
        return bin_, trace

    def test_porte_2_sans_aucun_appel_reseau(self):
        dossier = self.archive_complete()
        faux, trace = self.faux_reseau()
        r = self.lancer(cwd=dossier, extra_path=faux, script=dossier / "cassini.sh")
        self.assertEqual(r.returncode, 0, r.stdout + r.stderr)
        self.assertFalse(trace.exists(), "la porte 2 a appelé curl ou wget")
        self.assertEqual(self.srv.requetes, [])
        bin_ = self.racine / "bin"
        self.assertEqual((bin_ / "cassini").read_bytes(), (dossier / "noyau" / "cassini").read_bytes())
        self.assertEqual((bin_ / "models" / NOM_NOUVEAU).read_bytes(), self.modeles[NOM_NOUVEAU])
        decl = json.loads((bin_ / "models" / "cassini-composition-4b.modele.json").read_text(encoding="utf-8"))
        self.assertEqual(decl["model_sha256"], sha(self.modeles[NOM_NOUVEAU]))
        self.assertEqual(json.loads((bin_ / "installation.json").read_text(encoding="utf-8"))["version"], "0.1.1")

    def test_porte_2_octet_change(self):
        for cible in ("noyau/cassini", f"modele/{NOM_NOUVEAU}"):
            with self.subTest(cible=cible):
                dossier = self.archive_complete()
                fichier = dossier / cible
                octets = bytearray(fichier.read_bytes())
                octets[10] ^= 1
                fichier.write_bytes(bytes(octets))
                r = self.lancer(cwd=dossier, script=dossier / "cassini.sh")
                self.assertNotEqual(r.returncode, 0)
                self.assertIn(cible, r.stderr)
                self.aucun_fichier_pose()
                shutil.rmtree(dossier)
                shutil.rmtree(self.d / "dist")


class TestAssembleurLinux(unittest.TestCase):
    def setUp(self):
        self._t = tempfile.TemporaryDirectory()
        self.addCleanup(self._t.cleanup)
        self.d = Path(self._t.name)
        (self.d / "src").mkdir()
        self.binaire = self.d / "src" / "cassini"
        self.binaire.write_bytes(b"\x7fELF contenu")
        self.modele = self.d / "src" / NOM_NOUVEAU
        self.modele.write_bytes(b"modele")

    def test_archives_exactes_et_empreintes(self):
        n, c = assembler_linux.assembler(
            self.binaire, self.modele, SCRIPT, "0.1.1", self.d / "out", controler_glibc=False
        )
        self.assertEqual(n.name, "cassini-noyau-linux-0.1.1.tar.gz")
        self.assertEqual(c.name, "cassini-complet-linux-0.1.1.tar")
        self.assertEqual(assembler_linux.noms_dans(n), ["cassini"])
        with tarfile.open(c) as t:
            emp = t.extractfile("empreintes.txt").read().decode()
            self.assertEqual(t.extractfile("VERSION").read().decode(), "0.1.1\n")
            self.assertEqual(t.getmember("noyau/cassini").mode, 0o755)
        self.assertIn(f"{sha(b'modele')}  modele/{NOM_NOUVEAU}\n", emp)
        self.assertIn(f"{sha(self.binaire.read_bytes())}  noyau/cassini\n", emp)
        # Reproductible : mêmes octets deux fois de suite.
        n2, _ = assembler_linux.assembler(
            self.binaire, self.modele, SCRIPT, "0.1.1", self.d / "out2", controler_glibc=False
        )
        self.assertEqual(n.read_bytes(), n2.read_bytes())

    def test_refus(self):
        autre = self.d / "src" / "autre"
        autre.write_bytes(b"\x7fELF")
        with self.assertRaises(assembler_linux.AssemblageErreur):
            assembler_linux.assembler(autre, self.modele, SCRIPT, "0.1.1", self.d / "o", controler_glibc=False)
        exe = self.d / "src2"
        exe.mkdir()
        faux = exe / "cassini"
        faux.write_bytes(b"MZ pas un ELF")
        with self.assertRaises(assembler_linux.AssemblageErreur):
            assembler_linux.assembler(faux, self.modele, SCRIPT, "0.1.1", self.d / "o", controler_glibc=False)
        with self.assertRaises(assembler_linux.AssemblageErreur):
            assembler_linux.assembler(self.binaire, self.d / "absent.gguf", SCRIPT, "0.1.1", self.d / "o", controler_glibc=False)
        with self.assertRaises(assembler_linux.AssemblageErreur):
            assembler_linux.assembler(self.binaire, self.modele, SCRIPT, "v0.1", self.d / "o", controler_glibc=False)
        self.assertFalse((self.d / "o").exists())

    def test_plafond_glibc(self):
        objdump = (
            "0000 g    DF .text  0000 GLIBC_2.2.5 memcpy\n"
            "0000 g    DF .text  0000 (GLIBC_2.34) pthread_create\n"
            "0000 g    DF .text  0000 GLIBC_2.35 __libc_start_main\n"
        )
        self.assertEqual(assembler_linux.plafond_glibc(objdump), (2, 35))
        self.assertLessEqual(assembler_linux.plafond_glibc(objdump), assembler_linux.PLAFOND_GLIBC)
        self.assertGreater(assembler_linux.plafond_glibc(objdump + "GLIBC_2.38 strlcpy\n"), assembler_linux.PLAFOND_GLIBC)
        # Comparaison de nombres : 2.9 < 2.35.
        self.assertEqual(assembler_linux.plafond_glibc("GLIBC_2.9 GLIBC_2.10\n"), (2, 10))
        self.assertEqual(assembler_linux.plafond_glibc("rien\n"), (0, 0))

    def test_bibliotheques_requises(self):
        permises = "\n".join(
            f"  NEEDED               {b}" for b in assembler_linux.BIBLIOTHEQUES_PERMISES
        )
        texte = "Dynamic Section:\n" + permises + "\n  SONAME               x\n"
        self.assertEqual(
            assembler_linux.bibliotheques_requises(texte), list(assembler_linux.BIBLIOTHEQUES_PERMISES)
        )
        autre = texte + "  NEEDED               libgomp.so.1\n"
        refusees = [
            b for b in assembler_linux.bibliotheques_requises(autre)
            if b not in assembler_linux.BIBLIOTHEQUES_PERMISES
        ]
        self.assertEqual(refusees, ["libgomp.so.1"])


if __name__ == "__main__":
    unittest.main()
