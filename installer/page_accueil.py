"""Écrit `site/index.html` et `site/faq.html` à partir du manifeste d'une release.

Les empreintes SHA-256 viennent du `manifeste.json` (porte 2 Windows et Linux) ou sont calculées
sur le fichier (installeur en ligne) : jamais écrites à la main. La page est en HTML et CSS en
ligne, sans script ni ressource externe ; la vidéo `cassini-presentation.mp4`, si elle est dans
le dossier de sortie, est servie par la page elle-même.

Usage :
    python -I installer/page_accueil.py --manifeste manifeste.json \
        --installeur-en-ligne cassini-installeur-en-ligne.exe --sortie site
"""

import argparse
import hashlib
import html
import json
import sys
from pathlib import Path

DEPOT = "https://github.com/Kevzi/cassini-gis"
# Adresse de la release de ce manifeste, pas `latest` : l'empreinte affichée est celle de CE fichier.
URL_EN_LIGNE = DEPOT + "/releases/download/v{version}/cassini-installeur-en-ligne.exe"
URL_MODELE = "https://huggingface.co/KevziFar/cassini-composition-4b-GGUF"
CONTACT = "cassini-sig@proton.me"
VIDEO = "cassini-presentation.mp4"
VIDEO_MAX = 25 * 1024 * 1024  # 25 Mo : au-delà, refus


class PageErreur(Exception):
    pass


STYLE = """  :root { --fond: #fff; --texte: #1b1f24; --lien: #0b5cad; --bord: #d0d7de; }
  @media (prefers-color-scheme: dark) { :root { --fond: #14171a; --texte: #e6e8ea; --lien: #6cb0f5; --bord: #3a4046; } }
  body { background: var(--fond); color: var(--texte); font: 1.0625rem/1.55 system-ui, sans-serif; margin: 0; }
  main { max-width: 42rem; margin: 0 auto; padding: 2rem 1rem; }
  a { color: var(--lien); }
  video { width: 100%; height: auto; border: 1px solid var(--bord); border-radius: .4rem; }
  code { font-family: ui-monospace, monospace; font-size: .85rem; word-break: break-all; }
  dt { font-weight: 600; margin-top: 1rem; }
  dd { margin: .2rem 0 0; }"""


def sha256_de(chemin: Path) -> str:
    h = hashlib.sha256()
    with chemin.open("rb") as f:
        for bloc in iter(lambda: f.read(1 << 20), b""):
            h.update(bloc)
    return h.hexdigest()


def e(texte: str) -> str:
    return html.escape(str(texte), quote=True)


def _ligne(libelle: str, url: str, sha: str, taille: int | None = None) -> str:
    mo = f" ({taille / (1024 * 1024):.0f} Mo)" if taille else ""
    return (
        f'    <li><a href="{e(url)}">{e(libelle)}</a>{mo}<br>'
        f"SHA-256 : <code>{e(sha)}</code></li>"
    )


def installeurs(manifeste: dict, sha_en_ligne: str) -> list[str]:
    """Les lignes d'installeurs : en ligne (calculée), puis porte 2 Windows et Linux si présentes."""
    lignes = [_ligne(
        "Installeur léger pour Windows", URL_EN_LIGNE.format(version=manifeste["version"]), sha_en_ligne
    )]
    p2 = manifeste.get("porte2")
    if p2:
        lignes.append(
            _ligne("Installeur complet hors ligne pour Windows", p2["url"], p2["sha256"], p2.get("taille"))
        )
    linux = manifeste.get("linux")
    if linux:
        if linux.get("noyau"):
            n = linux["noyau"]
            lignes.append(_ligne("Linux : moteur seul (tar)", n["url"], n["sha256"], n.get("taille")))
        if linux.get("porte2"):
            c = linux["porte2"]
            lignes.append(_ligne("Linux : installeur complet (tar)", c["url"], c["sha256"], c.get("taille")))
    return lignes


def page_accueil(manifeste: dict, sha_en_ligne: str, video: bool) -> str:
    bloc_video = (
        f'  <video controls preload="metadata" src="{VIDEO}">'
        f'Votre navigateur ne lit pas la vidéo : <a href="{VIDEO}">la télécharger</a>.</video>\n'
        if video
        else ""
    )
    lignes = "\n".join(installeurs(manifeste, sha_en_ligne))
    return f"""<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Cassini</title>
<style>
{STYLE}
</style>
</head>
<body>
<main>
  <h1>Cassini</h1>
  <p>Cassini est l'interface en français d'un SIG qui fonctionne entièrement hors ligne : vous décrivez ce que vous voulez, il propose les opérations, vous les validez, il les exécute et les journalise.</p>
  <p>C'est une extension pour QGIS, posée sur un moteur indépendant ; vos données restent sur votre poste et rien n'est envoyé sur le réseau.</p>
{bloc_video}
  <h2>Installer en trois étapes</h2>
  <ol>
    <li>Installez <a href="https://qgis.org">QGIS</a> (version 3.44 ou plus récente), si ce n'est pas déjà fait.</li>
    <li>Lancez un installeur de Cassini ci-dessous : il pose le moteur et le modèle d'IA (<a href="{URL_MODELE}">modèle sur Hugging Face</a>).</li>
    <li>Dans QGIS, installez l'extension Cassini depuis le gestionnaire d'extensions, puis ouvrez son panneau.</li>
  </ol>

  <h2>Télécharger</h2>
  <ul>
{lignes}
  </ul>
  <p>Comparez l'empreinte SHA-256 du fichier téléchargé à celle affichée ici : <code>certutil -hashfile fichier SHA256</code> sous Windows, <code>sha256sum fichier</code> sous Linux. Windows affichera sans doute un avertissement à l'ouverture : l'installeur n'est pas encore signé. <a href="avertissement-windows.html">Pourquoi, et comment continuer sans risque</a>.</p>

  <p><a href="faq.html">Questions fréquentes</a> · <a href="{DEPOT}">Code source</a> · Contact : <a href="mailto:{CONTACT}">{CONTACT}</a></p>
</main>
</body>
</html>
"""


def page_faq() -> str:
    return f"""<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Questions fréquentes</title>
<style>
{STYLE}
</style>
</head>
<body>
<main>
  <h1>Questions fréquentes</h1>
  <dl>
    <dt>Cassini a-t-il besoin d'Internet ?</dt>
    <dd>Non. Seul l'installeur léger télécharge, une seule fois, pendant l'installation. Ensuite, ni le moteur ni l'extension ne touchent au réseau. L'installeur complet tient en un seul fichier, pour les postes isolés.</dd>
    <dt>Mes données sont-elles envoyées quelque part ?</dt>
    <dd>Non. Vous apportez vos fichiers, Cassini ne les modifie jamais et ne les envoie nulle part.</dd>
    <dt>Faut-il QGIS ?</dt>
    <dd>Oui : Cassini est une extension de QGIS (version 3.44 ou plus récente), qui fournit la carte, les formats et l'export PDF.</dd>
    <dt>Pourquoi Windows affiche-t-il un avertissement ?</dt>
    <dd>L'installeur n'est pas encore signé. <a href="avertissement-windows.html">Voici comment continuer sans risque</a>.</dd>
    <dt>Comment vérifier mon téléchargement ?</dt>
    <dd>Comparez son empreinte SHA-256 à celle de la <a href="index.html">page d'accueil</a>.</dd>
    <dt>Comment mettre à jour ?</dt>
    <dd>Avec « Mettre à jour Cassini », lancé à la main ; un nouveau modèle s'installe à côté de l'ancien.</dd>
    <dt>Quelles données puis-je ouvrir ?</dt>
    <dd>Celles que QGIS lit : GeoPackage, Shapefile, GeoJSON, CSV à coordonnées, GeoParquet, et des modèles d'élévation pour le terrain.</dd>
  </dl>
  <p><a href="index.html">Retour à la page d'accueil</a> · <a href="mailto:{CONTACT}">{CONTACT}</a></p>
</main>
</body>
</html>
"""


def generer(manifeste_chemin: Path, installeur: Path, sortie: Path) -> list[str]:
    """Écrit les pages dans `sortie` ; renvoie les messages à montrer à l'utilisateur."""
    manifeste = json.loads(manifeste_chemin.read_text(encoding="utf-8"))
    if not installeur.is_file():
        raise PageErreur(f"installeur en ligne introuvable : {installeur}")
    messages = []
    video = sortie / VIDEO
    if video.is_file():
        taille = video.stat().st_size
        if taille >= VIDEO_MAX:
            raise PageErreur(
                f"{video.name} pèse {taille / (1024 * 1024):.1f} Mo : 25 Mo ou plus, à réencoder"
            )
        present = True
    else:
        present = False
        messages.append(f"vidéo absente ({video}) : page générée sans balise vidéo")
    sortie.mkdir(parents=True, exist_ok=True)
    (sortie / "index.html").write_text(
        page_accueil(manifeste, sha256_de(installeur), present), encoding="utf-8", newline="\n"
    )
    (sortie / "faq.html").write_text(page_faq(), encoding="utf-8", newline="\n")
    return messages


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--manifeste", type=Path, required=True)
    p.add_argument("--installeur-en-ligne", type=Path, required=True)
    p.add_argument("--sortie", type=Path, required=True)
    a = p.parse_args(argv)
    try:
        for m in generer(a.manifeste, a.installeur_en_ligne, a.sortie):
            print(m, file=sys.stderr)
    except (PageErreur, OSError, ValueError, KeyError) as ex:
        print(f"erreur : {ex}", file=sys.stderr)
        return 1
    print(f"pages écrites dans {a.sortie}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
