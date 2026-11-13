"""Barre de non-oubli : un candidat tient chaque barre de son parent (ticket 5.53, livrable 2).

Outil hors ligne, sans modèle ni GPU : il lit les relevés déjà produits pour deux modèles, le
parent (installé) et le candidat (le nouvel entraînement), et dit, barre par barre, si le
candidat a oublié ce que le parent savait faire.

Trois sortes de relevés, reconnues au contenu (jamais au nom du fichier) :

- **essais GPU** (`essais-gpu-*.log`) : sections `=== <barre> ===` … `=== fin <barre> : <code> ===`,
  une ligne `SCORE <justes>/<total> …` par section (la première fraction compte) ; les lignes
  `SCORE <commande> <j>/<n>` et `SCORE total <j>/<n>` de `par_commande_reel` donnent une barre
  `par_commande:<commande>` par commande et la barre `par_commande_reel` ;
- **recette QGIS** (`recette-*.txt`) : `SCORE : <j>/<n>` ou, plus ancienne, `PLANS EXECUTES : <j>/<n>`,
  barre `recette:<nom>`, et la ligne `FIN :`, barre `recette:<nom>:rejeu` (1/1 si « rejeu
  identique », 0/1 sinon ; pas de barre sans ligne `FIN`) ;
- **rapport d'entraînement** (`rapport.json`) : `validation.justes` et `validation.total`, barre
  `validation`, et `validation.par_sorte`, la composition du jeu.

Seuil propre : le code de fin d'une section d'essais (0 : passée) et « barre tenue » ou « SOUS LA
BARRE » (« barre non tenue ») d'une recette. Un candidat qui échoue à un seuil que le parent
passait est en baisse, même à une phrase près.

Règle : une barre tient si le candidat a au plus une phrase juste de moins que le parent, sur le
même total (aucune pour le rejeu, qui tient ou ne tient pas). Une barre du parent absente du
candidat refuse. Une barre sans parent, ou de totaux différents, rend « à décider » : le
propriétaire tranche. La recette du gros projet est indicative : son verdict est montré, il ne
compte pas dans l'ensemble ; la validation aussi, sauf si les deux jeux ont la même composition
(mêmes totaux par sorte), car le jeu de validation est tiré du générateur à chaque entraînement.
Aucune barre comptée : à décider.

Codes de sortie : 0 installable, 1 refusé, 2 relevé illisible, usage faux ou erreur imprévue,
3 à décider.

    python3 -I training/entrainement/non_oubli.py --parent <relevés> --candidat <relevés> [--json <fichier>]

Python standard seul. Rien n'est chargé, installé ni déplacé.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path, PureWindowsPath
from typing import Any

# Phrases justes que le candidat peut perdre sur une barre sans la baisser.
TOLERANCE = 1
# Le rejeu identique ne se perd pas.
TOLERANCE_REJEU = 0
# Barres montrées sans compter dans le verdict d'ensemble, tant que 5.50 n'en fait pas une barre.
INDICATIVES = {"recette:gros", "recette:gros:rejeu"}
# Préfixe des identifiants de phrase d'une recette (`== RA01 : …`) → nom de la recette.
PREFIXES_RECETTE = {"RA": "annecy", "G": "gros"}

TENUE = "tenue"
BAISSE = "baisse"
ABSENTE = "absente"
SANS_PARENT = "sans parent"
NON_COMPARABLE = "non comparable"

INSTALLABLE = "installable"
REFUSE = "refusé"
A_DECIDER = "à décider"
CODES = {INSTALLABLE: 0, REFUSE: 1, A_DECIDER: 3}
CODE_ILLISIBLE = 2

SECTION = re.compile(r"^=== (\S+) ===$")
FIN_SECTION = re.compile(r"^=== fin (\S+) : (-?\d+) ===$")
FRACTION = re.compile(r"(\d+)\s*/\s*(\d+)")
SCORE_PAR_COMMANDE = re.compile(r"^SCORE ([A-Za-z_][\w.]*) (\d+)/(\d+)\s*$")
SCORE_ESSAI = re.compile(r"^SCORE (\d+\s*/\s*\d+.*)$")
# Ligne d'identité d'un essai, lue en ASCII seul (« Modèle » y devient « Modle »).
MODELE_ESSAI = re.compile(r"^Mod[A-Za-z]*\s*:\s*(.+?)\s*;\s*adaptateur\s*:\s*([^;]*?)\s*(?:;|$)")
HORODATAGE = re.compile(r"^\[[^\]]*\]\s*")
SCORE_RECETTE = re.compile(r"^(?:SCORE|PLANS EXECUTES)\s*:\s*(\d+)\s*/\s*(\d+)")
PHRASE_RECETTE = re.compile(r"^== ([A-Za-z]+)\s*\d+ :")
FIN_REJEU = re.compile(r"^FIN\s*:\s*rejeu identique")
SEUIL_RECETTE_TENU = re.compile(r":\s*barre tenue\b", re.IGNORECASE)
SEUIL_RECETTE_RATE = re.compile(r"SOUS LA BARRE|barre non tenue", re.IGNORECASE)
LIGNE_FIN = re.compile(r"^FIN\s*:")
LIGNE_PHRASES = re.compile(r"^phrases\s*:\s*(.+?)\s*\((\d+)\)")
BARRE_RECETTE = re.compile(r"\(barre (\d+)\)")
MODELES_RECETTE = re.compile(r";\s*mod[A-Za-z]*\s*:\s*(\S.*?)\s*$")
NOM_FICHIER_RECETTE = re.compile(r"^recette-([a-z0-9]+)")


class ReleveIllisible(Exception):
    """Un relevé vide, introuvable ou de forme inconnue ; le message nomme le fichier."""


def fraction(chemin: Path, justes: int, total: int) -> tuple[int, int]:
    """Une fraction de relevé ; un total nul ou plus de justes que de phrases rend le relevé
    illisible."""
    if total <= 0 or justes < 0 or justes > total:
        raise ReleveIllisible(f"{chemin} : fraction impossible {justes}/{total}.")
    return justes, total


@dataclass
class Score:
    justes: int
    total: int
    source: str
    code: int | None = None  # code de fin de la section d'un essai, s'il est écrit
    seuil: bool | None = None  # le relevé passe-t-il son propre seuil ? None : inconnu
    composition: dict[str, int] | None = None  # validation : total de chaque sorte du jeu

    def texte(self) -> str:
        return f"{self.justes}/{self.total}"


@dataclass
class Releves:
    """Les barres et les identités lues dans les relevés d'un modèle."""

    barres: dict[str, Score] = field(default_factory=dict)
    identites: list[dict[str, str]] = field(default_factory=list)
    avertissements: list[str] = field(default_factory=list)


def lire_ascii(chemin: Path) -> str:
    """Le texte d'un relevé, réduit à l'ASCII : la marque d'ordre et les accents abîmés par la
    console Windows tombent, les retours chariot aussi."""
    try:
        octets = chemin.read_bytes()
    except OSError as erreur:
        raise ReleveIllisible(f"{chemin} : relevé illisible ({erreur.strerror or erreur}).") from None
    texte = octets.decode("utf-8", errors="ignore").replace("\r", "")
    return "".join(c for c in texte if ord(c) < 128)


def nom_de_fichier(chemin: str) -> str:
    """Le dernier élément d'un chemin, Windows ou POSIX."""
    return PureWindowsPath(chemin).name if "\\" in chemin else Path(chemin).name


def nom_de_modele(chemin: str) -> str:
    """`C:\\…\\ajuste-5-49-a\\cassini-composition-4b-Q4_K_M.gguf` → `ajuste-5-49-a`."""
    chemin = chemin.strip().rstrip("\\/")
    morceaux = PureWindowsPath(chemin).parts if "\\" in chemin else Path(chemin).parts
    if not morceaux:
        return chemin
    if morceaux[-1].lower().endswith((".gguf", ".json")) and len(morceaux) > 1:
        return morceaux[-2]
    return morceaux[-1]


def lire_rapport(chemin: Path) -> tuple[dict[str, Score], dict[str, str]]:
    try:
        rapport = json.loads(chemin.read_text(encoding="utf-8-sig"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError):
        rapport = None
    validation = rapport.get("validation") if isinstance(rapport, dict) else None
    if not isinstance(validation, dict) or not all(
        isinstance(validation.get(k), int) and not isinstance(validation.get(k), bool)
        for k in ("justes", "total")
    ):
        raise ReleveIllisible(
            f"{chemin} : JSON sans `validation.justes` ni `validation.total` entiers, "
            "ce n'est pas un rapport d'entraînement."
        )
    justes, total = fraction(chemin, validation["justes"], validation["total"])
    composition = None
    par_sorte = validation.get("par_sorte")
    if isinstance(par_sorte, dict):
        composition = {
            sorte: v["total"]
            for sorte, v in par_sorte.items()
            if isinstance(v, dict) and isinstance(v.get("total"), int)
        } or None
    identite = {"releve": str(chemin), "sorte": "rapport", "modele": chemin.resolve().parent.name}
    fichier = validation.get("modele")
    sorties = rapport.get("sorties")
    if isinstance(fichier, str) and isinstance(sorties, dict) and isinstance(sorties.get(fichier), dict):
        empreinte = sorties[fichier].get("sha256")
        if isinstance(empreinte, str):
            identite["fichier"] = fichier
            identite["sha256"] = empreinte
    score = Score(justes, total, str(chemin), composition=composition)
    return {"validation": score}, identite


def lire_recette(chemin: Path, lignes: list[str]) -> tuple[dict[str, Score], dict[str, str], list[str]]:
    avertissements: list[str] = []
    score = None
    seuil = None
    rejeu = None
    prefixes: dict[str, int] = {}
    phrases = modeles = annonce = None
    for brute in lignes:
        ligne = HORODATAGE.sub("", brute.strip())
        if m := SCORE_RECETTE.match(ligne):
            if score is not None:
                avertissements.append(f"{chemin} : plusieurs lignes de score, la dernière compte.")
            score = fraction(chemin, int(m.group(1)), int(m.group(2)))
            if mb := BARRE_RECETTE.search(ligne):
                seuil = score[0] >= int(mb.group(1))
            elif SEUIL_RECETTE_TENU.search(ligne):
                seuil = True
            elif SEUIL_RECETTE_RATE.search(ligne):
                seuil = False
            else:
                seuil = None
                avertissements.append(f"{chemin} : ligne de score sans seuil (ni « (barre N) », ni « barre tenue »).")
        elif m := PHRASE_RECETTE.match(ligne):
            prefixes[m.group(1)] = prefixes.get(m.group(1), 0) + 1
        elif LIGNE_FIN.match(ligne):
            rejeu = bool(FIN_REJEU.match(ligne))
        elif phrases is None and (m := LIGNE_PHRASES.match(ligne)):
            phrases, annonce = m.group(1), int(m.group(2))
            if mm := MODELES_RECETTE.search(ligne):
                modeles = mm.group(1)
    nom = None
    if prefixes:
        prefixe = max(prefixes, key=prefixes.get)
        nom = PREFIXES_RECETTE.get(prefixe)
    if nom is None and phrases:
        # `…\annecy.json` → `annecy` ; `…\phrases-gros.json` → `gros`.
        nom = re.sub(r"^phrases-", "", nom_de_fichier(phrases).rsplit(".", 1)[0]) or None
    if nom is None and (m := NOM_FICHIER_RECETTE.match(chemin.name)):
        nom = m.group(1)
        avertissements.append(f"{chemin} : recette nommée « {nom} » d'après le nom du fichier.")
    if nom is None:
        raise ReleveIllisible(f"{chemin} : recette sans nom (ni identifiant de phrase connu, ni ligne `phrases :`).")
    if nom not in PREFIXES_RECETTE.values():
        avertissements.append(
            f"{chemin} : recette « {nom} » inconnue (ni annecy, ni gros) : elle compte comme une barre ordinaire."
        )
    if annonce is not None and annonce != score[1]:
        avertissements.append(f"{chemin} : {annonce} phrases annoncées, score sur {score[1]}.")
    identite = {"releve": str(chemin), "sorte": "recette"}
    if modeles:
        identite["modele"] = nom_de_modele(modeles)
        identite["chemin"] = modeles
    barres = {f"recette:{nom}": Score(score[0], score[1], str(chemin), seuil=seuil)}
    if rejeu is None:
        avertissements.append(f"{chemin} : recette sans ligne `FIN` (interrompue ?), pas de barre de rejeu.")
    else:
        barres[f"recette:{nom}:rejeu"] = Score(int(rejeu), 1, str(chemin))
    return barres, identite, avertissements


def lire_essais(chemin: Path, lignes: list[str]) -> tuple[dict[str, Score], list[dict[str, str]], list[str]]:
    barres: dict[str, Score] = {}
    modeles: dict[str, dict[str, str]] = {}
    avertissements: list[str] = []
    section = ouverte = None
    vues: set[str] = set()
    for brute in lignes:
        ligne = brute.strip()
        if m := FIN_SECTION.match(ligne):
            nom, code = m.group(1), int(m.group(2))
            ouverte = None
            if nom in barres:
                barres[nom].code = code
                barres[nom].seuil = code == 0
            else:
                avertissements.append(f"{chemin} : section « {nom} » finie (code {code}) sans ligne SCORE.")
            section = None
        elif m := SECTION.match(ligne):
            if ouverte is not None:
                avertissements.append(f"{chemin} : section « {ouverte} » sans ligne `=== fin … ===` (relevé coupé ?).")
            section = ouverte = m.group(1)
            if section in vues:
                avertissements.append(f"{chemin} : section « {section} » écrite deux fois, la dernière compte.")
                barres.pop(section, None)
                if section == "par_commande_reel":
                    for barre in [b for b in barres if b.startswith("par_commande:")]:
                        del barres[barre]
            vues.add(section)
        elif m := SCORE_PAR_COMMANDE.match(ligne):
            if section != "par_commande_reel":
                avertissements.append(
                    f"{chemin} : ligne `SCORE {m.group(1)}` hors de la section par_commande_reel, ignorée."
                )
                continue
            barre = "par_commande_reel" if m.group(1) == "total" else f"par_commande:{m.group(1)}"
            if int(m.group(3)) == 0:
                avertissements.append(f"{chemin} : ligne « {ligne} » sur zéro phrase, ignorée.")
                continue
            justes, total = fraction(chemin, int(m.group(2)), int(m.group(3)))
            barres[barre] = Score(justes, total, str(chemin))
        elif m := SCORE_ESSAI.match(ligne):
            if section is None:
                avertissements.append(f"{chemin} : ligne SCORE hors de toute section, ignorée.")
            else:
                if section in barres:
                    avertissements.append(f"{chemin} : section « {section} » à plusieurs lignes SCORE, la dernière compte.")
                f = FRACTION.search(m.group(1))
                justes, total = fraction(chemin, int(f.group(1)), int(f.group(2)))
                barres[section] = Score(justes, total, str(chemin))
        elif ligne.startswith("SCORE "):
            avertissements.append(f"{chemin} : ligne « {ligne} » de forme inconnue, ignorée.")
        elif m := MODELE_ESSAI.match(ligne):
            cle = (m.group(1), m.group(2))
            modeles.setdefault(
                "|".join(cle),
                {
                    "releve": str(chemin),
                    "sorte": "essais",
                    "modele": nom_de_modele(m.group(1)),
                    "chemin": m.group(1),
                    "adaptateur": m.group(2) or "aucun",
                },
            )
    if ouverte is not None:
        avertissements.append(f"{chemin} : section « {ouverte} » sans ligne `=== fin … ===` (relevé coupé ?).")
    if not barres:
        raise ReleveIllisible(f"{chemin} : aucune barre lue (ni section `=== … ===` notée, ni ligne `SCORE <commande>`).")
    return barres, list(modeles.values()), avertissements


def lire_releve(chemin: Path) -> tuple[dict[str, Score], list[dict[str, str]], list[str]]:
    """Lit un relevé, reconnu à son contenu : ses barres, les identités qu'il nomme et ses
    avertissements. `ReleveIllisible` si le fichier est vide, introuvable ou d'une forme inconnue."""
    texte = lire_ascii(chemin)
    if not texte.strip():
        raise ReleveIllisible(f"{chemin} : relevé vide.")
    if texte.lstrip().startswith("{"):
        barres, identite = lire_rapport(chemin)
        return barres, [identite], []
    lignes = texte.split("\n")
    if any(SCORE_RECETTE.match(HORODATAGE.sub("", l.strip())) for l in lignes):
        barres, identite, avertissements = lire_recette(chemin, lignes)
        return barres, [identite], avertissements
    if any(l.strip().startswith("SCORE ") for l in lignes):
        return lire_essais(chemin, lignes)
    raise ReleveIllisible(
        f"{chemin} : forme inconnue (ni essais GPU, ni recette QGIS, ni rapport d'entraînement)."
    )


def lire_releves(chemins: list[Path]) -> Releves:
    """Réunit les relevés d'un même modèle. Une barre lue dans deux fichiers est une erreur :
    on ne choisit pas en silence entre deux mesures."""
    releves = Releves()
    for chemin in chemins:
        barres, identites, avertissements = lire_releve(chemin)
        for barre, score in barres.items():
            if barre in releves.barres:
                raise ReleveIllisible(
                    f"{chemin} : la barre « {barre} » est déjà lue dans {releves.barres[barre].source}."
                )
            releves.barres[barre] = score
        releves.identites.extend(identites)
        releves.avertissements.extend(avertissements)
    return releves


def modeles_nommes(releves: Releves) -> list[str]:
    noms: list[str] = []
    for identite in releves.identites:
        nom = identite.get("modele")
        if nom and nom not in noms:
            noms.append(nom)
    return noms


def verdict_barre(barre: str, parent: Score | None, candidat: Score | None) -> str:
    if parent is None:
        return SANS_PARENT
    if candidat is None:
        return ABSENTE
    if parent.seuil is True and candidat.seuil is False:
        return BAISSE  # seuil propre perdu, même à une phrase près ou sur un autre total
    if parent.total != candidat.total:
        return NON_COMPARABLE
    if parent.seuil is True and candidat.seuil is None:
        return NON_COMPARABLE  # seuil du candidat inconnu (section coupée) : rien ne dit qu'il tient
    tolerance = TOLERANCE_REJEU if barre.endswith(":rejeu") else TOLERANCE
    return TENUE if candidat.justes >= parent.justes - tolerance else BAISSE


def ordre(barre: str) -> tuple[int, str]:
    if barre == "validation":
        return (1, barre)
    if barre.startswith("par_commande:"):
        return (3, barre)
    if barre.startswith("recette:"):
        return (4, barre)
    if barre == "par_commande_reel":
        return (2, "~")
    return (0, "")  # les essais GPU gardent l'ordre du relevé


def detail(score: Score | None) -> dict[str, Any] | None:
    if score is None:
        return None
    sortie: dict[str, Any] = {
        "justes": score.justes,
        "total": score.total,
        "code": score.code,
        "seuil": score.seuil,
        "releve": score.source,
    }
    if score.composition is not None:
        sortie["composition"] = score.composition
    return sortie


def composition_texte(score: Score) -> str:
    if score.composition is None:
        return "composition inconnue"
    return "/".join(f"{sorte} {n}" for sorte, n in sorted(score.composition.items()))


def comparer(parent: Releves, candidat: Releves) -> dict[str, Any]:
    """Le verdict de chaque barre et le verdict d'ensemble, en dictionnaire prêt pour le JSON."""
    avertissements = parent.avertissements + candidat.avertissements
    noms = list(parent.barres)
    noms += [b for b in candidat.barres if b not in parent.barres]
    noms.sort(key=ordre)
    lignes = []
    for barre in noms:
        p, c = parent.barres.get(barre), candidat.barres.get(barre)
        indicative = barre in INDICATIVES
        if barre == "validation":
            if p is None or c is None:
                indicative = True
                avertissements.append("validation indicative : lue d'un seul côté, composition inconnue de l'autre.")
            elif p.composition is None or p.composition != c.composition:
                indicative = True
                avertissements.append(
                    "validation indicative : les deux jeux n'ont pas la même composition "
                    f"({composition_texte(p)} contre {composition_texte(c)})."
                )
        lignes.append(
            {
                "barre": barre,
                "parent": detail(p),
                "candidat": detail(c),
                "verdict": verdict_barre(barre, p, c),
                "indicative": indicative,
            }
        )
    comptees = [l["verdict"] for l in lignes if not l["indicative"]]
    if any(v in (BAISSE, ABSENTE) for v in comptees):
        ensemble = REFUSE
    elif not comptees or any(v in (SANS_PARENT, NON_COMPARABLE) for v in comptees):
        ensemble = A_DECIDER
    else:
        ensemble = INSTALLABLE
    if not comptees:
        avertissements.append("Aucune barre comptée (toutes indicatives) : à décider.")
    communs = [n for n in modeles_nommes(parent) if n in modeles_nommes(candidat)]
    if communs:
        avertissements.append(f"Le même modèle est nommé des deux côtés : {', '.join(communs)}.")
    melange = False
    for cote, releves in (("parent", parent), ("candidat", candidat)):
        noms_modeles = modeles_nommes(releves)
        if len(noms_modeles) > 1:
            melange = True
            avertissements.append(f"Les relevés du {cote} nomment plusieurs modèles : {', '.join(noms_modeles)}.")
    if (communs or melange) and ensemble == INSTALLABLE:
        ensemble = A_DECIDER
        avertissements.append("Modèles mélangés ou identiques : au mieux à décider.")
    return {
        "verdict": ensemble,
        "code": CODES[ensemble],
        "regle": (
            f"une barre tient si le candidat a au plus {TOLERANCE} phrase juste de moins que le parent, "
            "sur le même total ; le rejeu identique ne se perd pas ; un seuil propre que le parent passait "
            "et que le candidat manque est une baisse ; une barre du parent absente du candidat refuse ; "
            "une barre sans parent ou de totaux différents est à décider ; la validation ne compte que "
            "sur deux jeux de même composition"
        ),
        "parent": {"modeles": modeles_nommes(parent), "identites": parent.identites},
        "candidat": {"modeles": modeles_nommes(candidat), "identites": candidat.identites},
        "barres": lignes,
        "avertissements": avertissements,
    }


def seuil_texte(detail_: dict[str, Any] | None) -> str:
    """`ok` ou `raté` (avec le code de fin d'un essai quand il n'est pas nul) ; `—` inconnu."""
    if detail_ is None or detail_.get("seuil") is None:
        return "—"
    if detail_["seuil"]:
        return "ok"
    return "raté" if detail_.get("code") in (None, 0) else f"raté ({detail_['code']})"


def en_texte(resultat: dict[str, Any]) -> str:
    def modeles(cote: str) -> str:
        return ", ".join(resultat[cote]["modeles"]) or "(non nommé dans les relevés)"

    sortie = [f"Parent   : {modeles('parent')}", f"Candidat : {modeles('candidat')}", ""]
    largeur = max([20] + [len(l["barre"]) for l in resultat["barres"]])
    for ligne in resultat["barres"]:
        p = "—" if ligne["parent"] is None else f"{ligne['parent']['justes']}/{ligne['parent']['total']}"
        c = "—" if ligne["candidat"] is None else f"{ligne['candidat']['justes']}/{ligne['candidat']['total']}"
        verdict = ligne["verdict"] + (" (indicative)" if ligne["indicative"] else "")
        texte = f"{ligne['barre']:<{largeur}} {p:>7} → {c:<7}  {verdict}"
        sp, sc = (seuil_texte(ligne[cote]) for cote in ("parent", "candidat"))
        if sp != "—" or sc != "—":
            texte = f"{texte:<{largeur + 44}} seuil {sp} → {sc}"
        sortie.append(texte.rstrip())
    sortie.append("")
    for avertissement in resultat["avertissements"]:
        sortie.append(f"attention : {avertissement}")
    sortie.append(f"Verdict : {resultat['verdict']}")
    return "\n".join(sortie)


def main(argv: list[str] | None = None) -> int:
    lecteur = argparse.ArgumentParser(
        description="Barre de non-oubli : le candidat tient-il chaque barre de son parent ?"
    )
    lecteur.add_argument("--parent", nargs="+", required=True, type=Path, help="relevés du modèle installé")
    lecteur.add_argument("--candidat", nargs="+", required=True, type=Path, help="relevés du nouveau modèle")
    lecteur.add_argument("--json", type=Path, help="écrit aussi le résultat en JSON dans ce fichier")
    arguments = lecteur.parse_args(argv)
    try:
        if arguments.json:
            cible = arguments.json.resolve()
            if any(cible == r.resolve() for r in arguments.parent + arguments.candidat):
                raise ReleveIllisible(f"{arguments.json} : `--json` désigne un des relevés, il serait écrasé.")
            # Un verdict périmé ne doit pas survivre à un lancement qui échoue.
            arguments.json.unlink(missing_ok=True)
        resultat = comparer(lire_releves(arguments.parent), lire_releves(arguments.candidat))
        print(en_texte(resultat))
        if arguments.json:
            # En dernier, par un fichier provisoire renommé : un JSON n'existe, entier, que pour
            # un lancement allé au bout.
            provisoire = arguments.json.with_name(arguments.json.name + ".partiel")
            try:
                provisoire.write_text(json.dumps(resultat, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
                provisoire.replace(arguments.json)
            finally:
                provisoire.unlink(missing_ok=True)
    except ReleveIllisible as erreur:
        print(f"erreur : {erreur}", file=sys.stderr)
        return CODE_ILLISIBLE
    except Exception as erreur:  # noqa: BLE001 — jamais le code 1, qui voudrait dire « refusé »
        print(f"erreur imprévue : {type(erreur).__name__} : {erreur}", file=sys.stderr)
        return CODE_ILLISIBLE
    return resultat["code"]


if __name__ == "__main__":
    try:
        sys.stdout.reconfigure(encoding="utf-8")
        sys.stderr.reconfigure(encoding="utf-8")
    except (AttributeError, ValueError):
        pass
    sys.exit(main())
