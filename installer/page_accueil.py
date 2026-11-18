"""Écrit `site/index.html`, `site/faq.html` et `site/avertissement-windows.html` à partir du manifeste d'une release.

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


STYLE = """  :root {
    --fond: #f6f8fb; --carte: #ffffff; --texte: #16202c; --doux: #55637a; --lien: #0b5cad;
    --accent: #0f6fdc; --accent-texte: #ffffff; --bord: #dbe2ec; --ombre: 0 1px 2px rgba(16,32,60,.06), 0 8px 24px rgba(16,32,60,.07);
    --halo: radial-gradient(60rem 28rem at 50% -8rem, rgba(15,111,220,.16), transparent 70%);
  }
  @media (prefers-color-scheme: dark) { :root {
    --fond: #0e1319; --carte: #161d26; --texte: #e8edf3; --doux: #9aa9bd; --lien: #7db9ff;
    --accent: #3b8df0; --accent-texte: #06101c; --bord: #28323f; --ombre: 0 1px 2px rgba(0,0,0,.4), 0 8px 24px rgba(0,0,0,.35);
    --halo: radial-gradient(60rem 28rem at 50% -8rem, rgba(59,141,240,.22), transparent 70%);
  } }
  * { box-sizing: border-box; }
  html { scroll-behavior: smooth; }
  body { background: var(--halo) no-repeat, var(--fond); color: var(--texte); font: 1.0625rem/1.6 system-ui, -apple-system, "Segoe UI", sans-serif; margin: 0; }
  a { color: var(--lien); }
  .haut, main, .pied { max-width: 56rem; margin: 0 auto; padding-left: 1rem; padding-right: 1rem; }
  .haut { display: flex; align-items: center; justify-content: space-between; gap: 1rem; padding-top: 1.1rem; padding-bottom: 1.1rem; }
  .marque { display: inline-flex; align-items: center; gap: .6rem; font-weight: 700; font-size: 1.15rem; color: var(--texte); text-decoration: none; letter-spacing: .01em; }
  .marque svg { width: 1.7rem; height: 1.7rem; }
  .haut nav { display: flex; gap: 1.25rem; font-size: .95rem; }
  .haut nav a { color: var(--doux); text-decoration: none; }
  .haut nav a:hover { color: var(--texte); }
  main { padding-top: 1rem; padding-bottom: 3rem; }
  h1 { font-size: clamp(2rem, 5.5vw, 3.1rem); line-height: 1.12; letter-spacing: -.02em; margin: .4rem 0 1rem; }
  h2 { font-size: 1.55rem; letter-spacing: -.01em; margin: 3.2rem 0 1.1rem; }
  .accroche { text-align: center; padding: 2.2rem 0 1rem; }
  .accroche p { color: var(--doux); font-size: 1.15rem; max-width: 40rem; margin: .6rem auto; }
  .accroche h1 span { color: var(--accent); }
  .boutons { display: flex; flex-wrap: wrap; gap: .75rem; justify-content: center; margin-top: 1.6rem; }
  .bouton { display: inline-block; padding: .75rem 1.4rem; border-radius: .6rem; font-weight: 600; text-decoration: none; background: var(--accent); color: var(--accent-texte); box-shadow: var(--ombre); }
  .bouton:hover { filter: brightness(1.08); }
  .bouton.second { background: transparent; color: var(--texte); border: 1px solid var(--bord); box-shadow: none; }
  figure { margin: 2.2rem 0 0; }
  video { display: block; width: 100%; height: auto; border-radius: .9rem; border: 1px solid var(--bord); box-shadow: var(--ombre); background: #000; }
  .phrase { margin: 1.4rem auto 0; max-width: 40rem; padding: .9rem 1.2rem; border-left: 4px solid var(--accent); background: var(--carte); border-radius: .4rem; box-shadow: var(--ombre); font-style: italic; }
  .phrase small { display: block; margin-top: .3rem; font-style: normal; color: var(--doux); }
  .grille { display: grid; gap: 1rem; grid-template-columns: repeat(auto-fit, minmax(min(100%, 15rem), 1fr)); padding: 0; margin: 0; list-style: none; }
  .carte { background: var(--carte); border: 1px solid var(--bord); border-radius: .8rem; padding: 1.2rem 1.3rem; box-shadow: var(--ombre); }
  .carte h3 { margin: 0 0 .4rem; font-size: 1.08rem; }
  .carte p { margin: 0; color: var(--doux); font-size: .98rem; }
  .grille.deux { grid-template-columns: repeat(auto-fit, minmax(min(100%, 20rem), 1fr)); }
  .etapes { counter-reset: etape; }
  .etapes .carte { position: relative; padding-left: 4rem; }
  .etapes .carte::before { counter-increment: etape; content: counter(etape); position: absolute; left: 1.2rem; top: 1.1rem; width: 2rem; height: 2rem; border-radius: 50%; background: var(--accent); color: var(--accent-texte); font-weight: 700; display: grid; place-items: center; }
  .telechargements { display: grid; gap: 1rem; padding: 0; margin: 0; list-style: none; }
  .telechargements .carte { display: flex; flex-wrap: wrap; align-items: center; justify-content: space-between; gap: .6rem 1.5rem; }
  .telechargements .carte > div { flex: 1 1 20rem; min-width: 0; }
  .telechargements .nom { font-weight: 600; }
  .telechargements .taille { color: var(--doux); font-weight: 400; font-size: .92rem; margin-left: .5rem; }
  .sha { margin: .35rem 0 0; font-size: .82rem; color: var(--doux); }
  .note { color: var(--doux); font-size: .95rem; margin-top: 1.2rem; }
  code { font-family: ui-monospace, "Cascadia Mono", Consolas, monospace; font-size: .8rem; word-break: break-all; }
  dt { font-weight: 600; margin-top: 1.4rem; }
  dd { margin: .3rem 0 0; color: var(--doux); }
  .pied { color: var(--doux); font-size: .92rem; border-top: 1px solid var(--bord); padding-top: 1.4rem; padding-bottom: 2.5rem; }
  .pied a { color: var(--doux); }
  @media (max-width: 34rem) { .haut nav { gap: .8rem; font-size: .88rem; } .etapes .carte { padding-left: 3.6rem; } }"""

LOGO = (
    '<svg viewBox="0 0 32 32" aria-hidden="true"><circle cx="16" cy="16" r="6" fill="var(--accent)"/>'
    '<ellipse cx="16" cy="16" rx="14" ry="5" fill="none" stroke="var(--accent)" stroke-width="2" '
    'transform="rotate(-20 16 16)"/></svg>'
)


def entete() -> str:
    """Barre du haut commune à toutes les pages."""
    return (
        f'<header class="haut"><a class="marque" href="index.html">{LOGO}Cassini</a>'
        '<nav><a href="index.html#installer">Installer</a><a href="faq.html">FAQ</a>'
        f'<a href="{DEPOT}">Code source</a></nav></header>'
    )


def pied() -> str:
    return (
        '<footer class="pied">Cassini · moteur Apache-2.0, extension QGIS GPL · '
        f'<a href="faq.html">Questions fréquentes</a> · <a href="{DEPOT}">Code source</a> · '
        f'Contact : <a href="mailto:{CONTACT}">{CONTACT}</a></footer>'
    )


def sha256_de(chemin: Path) -> str:
    h = hashlib.sha256()
    with chemin.open("rb") as f:
        for bloc in iter(lambda: f.read(1 << 20), b""):
            h.update(bloc)
    return h.hexdigest()


def e(texte: str) -> str:
    return html.escape(str(texte), quote=True)


def _ligne(libelle: str, url: str, sha: str, taille: int | None = None) -> str:
    mo = f'<span class="taille">{taille / (1024 * 1024):.0f} Mo</span>' if taille else ""
    return (
        f'    <li class="carte"><div><span class="nom">{e(libelle)}</span>{mo}'
        f'<p class="sha">SHA-256 : <code>{e(sha)}</code></p></div>'
        f'<a class="bouton second" href="{e(url)}">Télécharger</a></li>'
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
        f'  <figure><video controls preload="metadata" src="{VIDEO}">'
        f'Votre navigateur ne lit pas la vidéo : <a href="{VIDEO}">la télécharger</a>.</video></figure>\n'
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
<meta name="description" content="Cassini : un SIG en français, piloté par une IA locale, qui fonctionne entièrement hors ligne.">
<style>
{STYLE}
</style>
</head>
<body>
{entete()}
<main>
  <section class="accroche">
  <h1>Votre SIG, en français.<br><span>Sans réseau.</span></h1>
  <p>Cassini est l'interface en français d'un SIG qui fonctionne entièrement hors ligne : vous décrivez ce que vous voulez, il propose les opérations, vous les validez, il les exécute et les journalise.</p>
  <p>C'est une extension pour QGIS, posée sur un moteur indépendant ; vos données restent sur votre poste et rien n'est envoyé sur le réseau.</p>
  <div class="boutons"><a class="bouton" href="#installer">Installer Cassini</a><a class="bouton second" href="faq.html">Questions fréquentes</a></div>
  </section>
{bloc_video}
  <div class="phrase">« Trouve les parcelles constructibles du PLU à moins de 500 m d'une gare à Annecy et sors-moi la carte en PDF. »<small>Une demande de la vidéo, sur un projet où les fichiers sont déjà ouverts.</small></div>

  <h2>Pourquoi Cassini</h2>
  <ul class="grille deux">
    <li class="carte"><h3>100 % hors ligne</h3><p>L'IA tourne sur votre poste. Aucune fonction n'utilise le réseau : adapté aux postes isolés.</p></li>
    <li class="carte"><h3>Vous gardez la main</h3><p>Chaque demande devient une suite d'opérations affichée étape par étape, que vous validez avant exécution.</p></li>
    <li class="carte"><h3>Rejouable</h3><p>Tout est journalisé à côté du projet QGIS : une analyse se rejoue à l'identique.</p></li>
    <li class="carte"><h3>Vos données, chez vous</h3><p>Vous apportez vos fichiers. Cassini ne les modifie jamais et n'en envoie aucun nulle part.</p></li>
  </ul>

  <h2 id="installer">Installer en trois étapes</h2>
  <ol class="grille etapes">
    <li class="carte"><h3>QGIS</h3><p>Installez <a href="https://qgis.org">QGIS</a> (version 3.44 ou plus récente), si ce n'est pas déjà fait.</p></li>
    <li class="carte"><h3>Le moteur et le modèle</h3><p>Lancez un installeur de Cassini ci-dessous : il pose le moteur et le modèle d'IA (<a href="{URL_MODELE}">modèle sur Hugging Face</a>).</p></li>
    <li class="carte"><h3>L'extension</h3><p>Dans QGIS, installez l'extension Cassini depuis le gestionnaire d'extensions, puis ouvrez son panneau.</p></li>
  </ol>

  <h2>Télécharger</h2>
  <ul class="telechargements">
{lignes}
  </ul>
  <p class="note">Comparez l'empreinte SHA-256 du fichier téléchargé à celle affichée ici : <code>certutil -hashfile fichier SHA256</code> sous Windows, <code>sha256sum fichier</code> sous Linux. Windows affichera sans doute un avertissement à l'ouverture : l'installeur n'est pas encore signé. <a href="avertissement-windows.html">Pourquoi, et comment continuer sans risque</a>.</p>
</main>
{pied()}
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
{entete()}
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
  <p><a href="index.html">Retour à la page d'accueil</a></p>
</main>
{pied()}
</body>
</html>
"""


def page_avertissement() -> str:
    return f"""<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Avertissement de Windows</title>
<style>
{STYLE}
</style>
</head>
<body>
{entete()}
<main>
  <h1>L'avertissement de Windows à l'ouverture de l'installeur</h1>
  <p>En ouvrant l'installeur de Cassini, Windows (SmartScreen) peut afficher « Windows a protégé votre ordinateur ». Ce message apparaît pour tout programme récent qui n'est pas encore signé par un éditeur reconnu ; il ne dit pas que Cassini est dangereux. La signature du programme viendra plus tard.</p>

  <h2>Pour continuer</h2>
  <ol>
    <li>Dans la fenêtre bleue, cliquez sur « Informations complémentaires ».</li>
    <li>Cliquez sur « Exécuter quand même ».</li>
  </ol>

  <h2>Pour vérifier le fichier avant</h2>
  <p>Le manifeste de chaque version, <a href="{DEPOT}/releases/latest/download/manifeste.json">manifeste.json</a>, donne l'empreinte SHA-256 du moteur de Cassini ; l'installeur la vérifie lui-même et ne pose rien si elle ne correspond pas.</p>

  <p><a href="index.html">Retour à la page d'accueil</a></p>
</main>
{pied()}
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
    (sortie / "avertissement-windows.html").write_text(
        page_avertissement(), encoding="utf-8", newline="\n"
    )
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
