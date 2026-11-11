"""Une commande pour ré-entraîner le modèle de composition (ticket 5.10, tâche 2).

    uv run --project training/entrainement python training/entrainement/pipeline.py --sortie <dossier hors dépôt>

Étapes, dans l'ordre, chacune sautée sur `--reprendre` si son produit existe déjà :

 1. contrôle du GPU, avant tout téléchargement et avant de toucher au dossier de sortie ;
 2. dossier de sortie : refus s'il est déjà rempli (sauf `--reprendre`) ou dans le dépôt ;
 3. génération des jeux par `cassini-training --format chat` : graine 1 pour l'entraînement,
    graine 2 pour la validation (les prompts déjà vus à l'entraînement en sont retirés) ;
 4. base Qwen3.5-4B, épinglée à une révision, dans le cache de Hugging Face ;
 5. QLoRA (nf4, gradient checkpointing, graine fixe), perte sur la réponse seule ;
 6. score de validation, exact (`scoring.py`), décodage glouton ;
 7. llama.cpp cloné à un commit fixe dans un cache hors dépôt, construit sur CPU ;
 8. base et adaptateur convertis en GGUF, fusion, quantification Q4_K_M ; chaque fichier
    s'écrit sous un nom temporaire et ne prend son nom final qu'une fois complet ;
 9. essai de `llama-completion` sur une demande de validation, par les deux chemins :
    base GGUF avec l'adaptateur GGUF, et modèle fusionné ;
10. `rapport.json`.

Avec `--exemples-externes <fichier> --nom <nom>` (ticket 5.23), la même commande entraîne un
adaptateur de domaine sur un fichier d'exemples `{prompt, completion}` venu d'ailleurs : même
base, mêmes graines, mêmes hyperparamètres. Par défaut le jeu civil engendré s'y ajoute
(`--externe-seul` le retire) et une part du fichier est mise de côté pour la validation
(`--validation-externe` donne un second fichier à la place). La sortie contient la base GGUF,
l'adaptateur et sa déclaration, sous le nom choisi, sans fusion. Le fichier est lu et vérifié
avant le GPU ; le rapport n'en garde que l'empreinte et les comptes, jamais le contenu.

Avec `--depuis <dossier> --familles a,b` (ticket 5.54, ajustement continu), l'entraînement repart
de l'adaptateur LoRA d'un entraînement précédent (un dossier de sortie de cette commande, jamais
modifié) au lieu de partir de zéro : jeu de séance = familles nommées (graine 3) + rappel tiré de
toutes les familles (préfixe du jeu graine 1, au moins 40 %, `--part-rappel`), 3 000 exemples par
défaut, taux d'apprentissage 5e-5, une graine. Le rapport porte la lignée (`lignee`, au plus trois
maillons) ; un parent d'une autre base, d'une autre révision ou d'autres modules est refusé, avant
le GPU.

Python ne met rien en forme : le texte vu par le modèle vient de `training/src/prompt.rs`.
Les modèles et poids sortent hors du dépôt. torch et transformers ne sont importés que dans
les étapes qui en ont besoin : les gardes se testent sans eux (`tests/`).
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import random
import re
import shutil
import subprocess
import sys
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))

import data  # noqa: E402
import scoring  # noqa: E402

# -- choix épinglés ------------------------------------------------------------------------

BASE_REPO = "Qwen/Qwen3.5-4B"  # Apache-2.0
BASE_REVISION = "851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a"
# Le commit du sous-module llama.cpp de llama-cpp-sys-2, celui que lie `cassini-ai` ; un test
# vérifie que `Cargo.lock` tire toujours cette version de llama-cpp-sys-2.
LLAMA_CPP_URL = "https://github.com/ggml-org/llama.cpp"
LLAMA_CPP_COMMIT = "d81235049384534c167caea52b85a694f6103d14"
LLAMA_CPP_SYS_VERSION = "0.1.160"

TRAINING_SEED = 1
VALIDATION_SEED = 2
# Graine et taille maximale du jeu ciblé d'une séance continue (`FOCUS_SEED` et `FOCUS_COUNT` de
# `training/src/lib.rs`, gardés par `training/tests/recouvrement.rs` ; un test les relie).
FOCUS_SEED = 3
FOCUS_COUNT = 1800
# Graine de l'entraînement (torch, ordre des exemples, LoRA, llama.cpp), réglable par
# `--graine-torch` : deux entraînements la même nuit sur le même jeu (décision du propriétaire,
# 2026-11-10). Les graines du générateur (1 et 2) ne changent pas : le jeu reste celui que
# `training/tests/recouvrement.rs` contrôle.
TORCH_SEED = 1
TRAINING_COUNT = 8600  # décision du propriétaire, 5.15 ; 8 600 depuis le 2026-11-10
VALIDATION_COUNT = 300
# Gros projets de 20 à 60 couches (5.47, décision du propriétaire du 2026-11-10) : 4 608, tenu
# par l'essai du 2026-11-10 sur la carte de 8 Go (`--longs-d-abord --pas-max 100` : pic 5,72 Gio).
# 5 632 depuis 5.49 (tranchée en party le 2026-11-10, à confirmer par le propriétaire) : les gros
# projets vus en entier, jusqu'à la taille d'Annecy, entrent au jeu. Le générateur écarte déjà tout
# prompt au-delà de `MAX_PROMPT_CHARS` (`training/src/lib.rs`), pour qu'aucune séquence n'atteigne
# cette borne (mesures et essai mémoire : `docs/entrainement-du-modele.md`, 5.49).
MAX_LENGTH = 5632
# Le GPU visé a 8 Go ; on refuse en dessous de 7 Gio de mémoire totale.
MIN_GPU_MEMORY = 7 * 1024**3

HYPERPARAMETERS: dict[str, Any] = {
    "methode": "QLoRA nf4, double quantification, calcul en bf16",
    "rang": 16,
    "alpha": 32,
    "dropout": 0.05,
    # Projections du modèle de langue seulement (la tour de vision n'est pas touchée).
    "modules": (
        r".*language_model\.layers\.\d+\."
        r"(self_attn\.(q_proj|k_proj|v_proj|o_proj)"
        r"|linear_attn\.(in_proj_z|out_proj)"
        r"|mlp\.(gate_proj|up_proj|down_proj))"
    ),
    "taux_apprentissage": 2e-4,
    "echauffement": 0.03,
    "planning": "cosinus",
    "epoques": 1,
    "lot": 1,
    "accumulation": 16,
    "norme_max_gradient": 1.0,
    "longueur_max": MAX_LENGTH,
    "jetons_max_generes": 512,
}

# Ajustement continu (5.54, décisions du propriétaire du 2026-11-11) : séance de 3 000 exemples
# par défaut (D2), dont une part de rappel d'au moins 0,4 (D3), au taux de 5e-5 ; une chaîne de
# trois maillons au plus, puis recalage par un entraînement de zéro sur le jeu complet.
CONTINUOUS_COUNT = 3000
RECALL_SHARE = 0.4
MIN_RECALL_SHARE = 0.4
CONTINUOUS_LEARNING_RATE = 5e-5
MAX_LINKS = 3
# Rappel tiré en plus, puis coupé au compte une fois les exemples trop longs écartés : la part
# du rappel tient le plancher après filtrage (le générateur borne déjà la longueur des prompts).
RECALL_MARGIN = 60
# Un dossier de sortie de cette commande, tel que `--depuis` le lit.
PARENT_REPORT = "rapport.json"
PARENT_ADAPTER_DIR = Path("travail") / "adaptateur-hf"
ADAPTER_CONFIG = "adapter_config.json"
ADAPTER_WEIGHTS = "adapter_model.safetensors"
FAMILIES_SOURCE = ROOT / "training" / "src" / "families.rs"
GENERATOR_SOURCE = ROOT / "training" / "src" / "lib.rs"
FOCUS_EXCLUDED_LIST = re.compile(r"pub const FOCUS_EXCLUDED: &\[&str\] = &\[(.*?)\];", re.S)
# `Family {`, des lignes de commentaire éventuelles, puis `name: "…"` : la forme que le test
# `the_pipeline_reads_the_family_names_and_constants_as_written_here` de `lib.rs` tient égale à
# `FAMILIES`.
FAMILY_NAME = re.compile(r'Family \{\s*(?://[^\n]*\n\s*)*name: "([a-z0-9_]+)"')
SNAPSHOT_PATH = re.compile(r"models--([^/]+?)--([^/]+)/snapshots/([0-9a-f]{40})/?$")
BROKEN_CHAIN = "chaîne coupée"

QUANTIZATION = "Q4_K_M"
BASE_NAME = "qwen3.5-4b-base-Q4_K_M.gguf"
ADAPTER_NAME = "cassini-composition-adaptateur-f16.gguf"
MERGED_NAME = "cassini-composition-4b-Q4_K_M.gguf"
# Déclarations de modèle (tâche 3) : lues par `model.list` du noyau, posées à côté des GGUF.
# Noms de fichiers seuls, même dossier ; le modèle ajusté attend le prompt court.
ADAPTED_DECLARATION_NAME = "cassini-composition-4b-adaptateur.modele.json"
MERGED_DECLARATION_NAME = "cassini-composition-4b.modele.json"
DECLARATIONS = {
    ADAPTED_DECLARATION_NAME: {"model": BASE_NAME, "adapter": ADAPTER_NAME, "prompt": "short"},
    MERGED_DECLARATION_NAME: {"model": MERGED_NAME, "prompt": "short"},
}

# Adaptateur de domaine (5.23) : part du fichier externe mise de côté pour le score, tirée
# par la graine de validation ; nom choisi, en minuscules, sans séparateur de dossier.
EXTERNAL_VALIDATION_SHARE = 0.1
NAME_PATTERN = re.compile(r"[a-z0-9][a-z0-9_-]{0,62}")
CIVIL_PREFIX = "cassini-composition"


@dataclass(frozen=True)
class Outputs:
    """Ce que la commande écrit dans le dossier de sortie : la base GGUF, un adaptateur, le
    modèle fusionné s'il y en a un, et les déclarations lues par `model.list`."""

    adapter: str
    merged: str | None
    declarations: dict[str, dict[str, str]]

    @property
    def files(self) -> list[str]:
        return [BASE_NAME, self.adapter] + ([self.merged] if self.merged else [])


CIVIL_OUTPUTS = Outputs(adapter=ADAPTER_NAME, merged=MERGED_NAME, declarations=DECLARATIONS)


def external_outputs(name: str) -> Outputs:
    """Sorties d'un adaptateur de domaine nommé `name` : la même base, son adaptateur et sa
    déclaration (base + adaptateur, prompt court), sans fusion."""
    adapter = f"{name}-adaptateur-f16.gguf"
    return Outputs(
        adapter=adapter,
        merged=None,
        declarations={f"{name}.modele.json": {"model": BASE_NAME, "adapter": adapter, "prompt": "short"}},
    )


class PipelineStop(RuntimeError):
    """Arrêt volontaire, avec un message clair pour le propriétaire."""


def cache_dir() -> Path:
    """Cache hors dépôt : clone de llama.cpp et environnement."""
    return Path(os.environ.get("CASSINI_CACHE_ENTRAINEMENT", Path.home() / ".cache" / "cassini-entrainement"))


def log(message: str) -> None:
    print(f"[{time.strftime('%H:%M:%S')}] {message}", flush=True)


# -- gardes (testées sans torch) -----------------------------------------------------------


def prepare_output(output: Path, resume: bool) -> None:
    """Crée le dossier de sortie. Refuse un dossier dans le dépôt (les poids ne s'y versionnent
    jamais) et un dossier déjà rempli, sauf `--reprendre` : rien n'est jamais écrasé."""
    output = output.resolve()
    if output == ROOT or ROOT in output.parents:
        raise PipelineStop(f"le dossier de sortie {output} est dans le dépôt : choisissez-en un hors du dépôt")
    if output.exists() and any(output.iterdir()) and not resume:
        raise PipelineStop(
            f"le dossier de sortie {output} n'est pas vide : rien n'est écrasé. "
            "Choisissez un dossier vide, ou relancez avec --reprendre pour finir un entraînement commencé."
        )
    output.mkdir(parents=True, exist_ok=True)


def check_gpu(total_memory: int | None) -> None:
    """Arrête la chaîne si aucun GPU CUDA n'est vu, ou s'il a moins de 7 Gio."""
    if total_memory is None:
        raise PipelineStop(
            "aucun GPU CUDA visible : l'entraînement exige le GPU de 8 Go (vérifiez nvidia-smi sous WSL)"
        )
    if total_memory < MIN_GPU_MEMORY:
        raise PipelineStop(
            f"GPU trop petit : {total_memory / 1024**3:.1f} Gio, il en faut au moins "
            f"{MIN_GPU_MEMORY / 1024**3:.0f}"
        )


def check_options(args: argparse.Namespace) -> Outputs:
    """Vérifie les options du fichier externe et rend les sorties prévues. Sans
    `--exemples-externes`, les sorties de 5.17, inchangées. Un nom qui reprendrait une sortie
    du modèle civil est refusé avant tout travail."""
    if args.external is None:
        extra = [flag for flag, value in (("--nom", args.name), ("--externe-seul", args.external_only),
                                          ("--validation-externe", args.external_validation)) if value]
        if extra:
            raise PipelineStop(f"{', '.join(extra)} n'a de sens qu'avec --exemples-externes <fichier>")
        return CIVIL_OUTPUTS
    if not args.name:
        raise PipelineStop("--exemples-externes exige --nom <nom> : le nom de l'adaptateur et de sa déclaration")
    if not NAME_PATTERN.fullmatch(args.name):
        raise PipelineStop(
            f"nom refusé : {args.name!r}. Lettres minuscules, chiffres, - et _ seulement, 63 au plus, "
            "sans dossier ni extension"
        )
    outputs = external_outputs(args.name)
    civil = {ADAPTER_NAME, MERGED_NAME, *DECLARATIONS}
    clashes = sorted(civil & {outputs.adapter, *outputs.declarations})
    if clashes or args.name.startswith(CIVIL_PREFIX):
        raise PipelineStop(
            f"nom refusé : {args.name!r} reprend le nom du modèle civil ({CIVIL_PREFIX}…"
            + (f", {', '.join(clashes)}" if clashes else "")
            + ") et l'écraserait dans un dossier de modèles. Choisissez un autre nom."
        )
    return outputs

# -- ajustement continu (5.54) : gardes du parent, testées sans torch ----------------------


@dataclass(frozen=True)
class Parent:
    """Un entraînement précédent, lu dans son dossier de sortie (jamais modifié) : son rapport,
    la configuration de son adaptateur, l'empreinte de ses poids et sa lignée."""

    folder: Path
    report: dict[str, Any]
    config: dict[str, Any]
    adapter_sha256: str
    lineage: list[dict[str, Any]]

    @property
    def adapter_dir(self) -> Path:
        return self.folder / PARENT_ADAPTER_DIR


@dataclass(frozen=True)
class Continuation:
    """Une séance continue : le parent, les familles du jeu ciblé et la part du rappel."""

    parent: Parent
    families: list[str]
    recall_share: float
    count: int = 3000  # CONTINUOUS_COUNT


def read_json_file(path: Path) -> Any:
    if not path.is_file():
        raise PipelineStop(f"parent incomplet : {path} absent")
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
        raise PipelineStop(f"parent illisible : {path} ({error})") from error


def safetensors_keys(path: Path) -> list[str]:
    """Les noms des tenseurs d'un fichier safetensors, lus dans son en-tête (8 octets de
    longueur, puis du JSON), sans torch."""
    if not path.is_file():
        raise PipelineStop(f"parent incomplet : {path} absent")
    try:
        with open(path, "rb") as f:
            size = int.from_bytes(f.read(8), "little")
            if not 2 <= size <= 100 * 1024**2:
                raise ValueError(f"en-tête de {size} octets")
            header = json.loads(f.read(size))
        if not isinstance(header, dict):
            raise ValueError("en-tête qui n'est pas un objet")
    except (OSError, UnicodeDecodeError, ValueError) as error:
        raise PipelineStop(f"parent illisible : {path} ({error})") from error
    return [k for k in header if k != "__metadata__"]


SHA256_HEX = re.compile(r"[0-9a-f]{64}")


def rebuilt_first_link(report: dict[str, Any], adapter_sha256: str, path: Path) -> list[dict[str, Any]]:
    """La lignée d'un parent d'avant 5.54 (sans clé `lignee`) : un entraînement de zéro, le
    maillon 1, reconstruit depuis son rapport (`depot_cassini.commit`, ou l'ancien
    `commit_cassini`). Sans commit ou sans graine torch, la chaîne s'arrête en le disant."""
    commit = report.get("depot_cassini", {}).get("commit") or report.get("commit_cassini")
    seed = report.get("graines", {}).get("torch")
    if not isinstance(commit, str) or not commit:
        raise PipelineStop(f"parent illisible : {path} : ni depot_cassini.commit ni commit_cassini")
    if not isinstance(seed, int):
        raise PipelineStop(f"parent illisible : {path} : graines.torch absente")
    return [{
        "maillon": 1,
        "depart": "base",
        "commit": commit,
        "graine_torch": seed,
        "adaptateur_sha256": adapter_sha256,
        "reconstruit": True,
    }]


def check_lineage(lineage: Any, path: Path) -> None:
    """Une lignée se lit maillon par maillon : numéros 1, 2, …, empreintes de 64 chiffres
    hexadécimaux, le maillon 1 parti de la base et chaque suivant de l'adaptateur du précédent."""
    def broken(reason: str) -> PipelineStop:
        return PipelineStop(f"parent illisible : {path} : lignée mal formée ({reason})")

    if not isinstance(lineage, list) or not lineage:
        raise broken("pas une liste de maillons")
    for rank, link in enumerate(lineage, start=1):
        if not isinstance(link, dict) or link.get("maillon") != rank:
            raise broken(f"maillon {rank} absent ou mal numéroté")
        if not (isinstance(link.get("adaptateur_sha256"), str) and SHA256_HEX.fullmatch(link["adaptateur_sha256"])):
            raise broken(f"maillon {rank} : empreinte de l'adaptateur invalide")
        before = "base" if rank == 1 else lineage[rank - 2]["adaptateur_sha256"]
        if link.get("depart") != before:
            raise broken(f"maillon {rank} : départ {link.get('depart')!r}, attendu {before!r}")


def read_parent(folder: Path) -> Parent:
    """Lit un dossier de sortie de cette commande : `rapport.json` et
    `travail/adaptateur-hf/{adapter_config.json, adapter_model.safetensors}`. Un fichier absent
    ou illisible arrête la chaîne en le nommant. Rien n'y est écrit."""
    folder = folder.resolve()
    if not folder.is_dir():
        raise PipelineStop(f"parent introuvable : {folder} n'est pas un dossier")
    report = read_json_file(folder / PARENT_REPORT)
    config = read_json_file(folder / PARENT_ADAPTER_DIR / ADAPTER_CONFIG)
    for path, value in ((folder / PARENT_REPORT, report), (folder / PARENT_ADAPTER_DIR / ADAPTER_CONFIG, config)):
        if not isinstance(value, dict):
            raise PipelineStop(f"parent illisible : {path} n'est pas un objet JSON")
    report_path = folder / PARENT_REPORT
    for key in ("base", "hyperparametres", "donnees", "graines", "depot_cassini"):
        if key in report and not isinstance(report[key], dict):
            raise PipelineStop(f"parent illisible : {report_path} : « {key} » n'est pas un objet")
    for key in ("base", "hyperparametres", "donnees"):
        if key not in report:
            raise PipelineStop(f"parent illisible : {report_path} : « {key} » absent")
    # Un essai court n'est pas un parent, ni un entraînement inachevé (sans score de validation).
    if report["hyperparametres"].get("pas_max") is not None:
        raise PipelineStop(f"parent refusé : {report_path} : essai court (pas_max "
                           f"{report['hyperparametres']['pas_max']}), pas un entraînement complet")
    if not isinstance(report.get("validation"), dict):
        raise PipelineStop(f"parent refusé : {report_path} : pas de score de validation, entraînement inachevé")
    weights = folder / PARENT_ADAPTER_DIR / ADAPTER_WEIGHTS
    keys = safetensors_keys(weights)
    if not keys or any("lora_" not in k for k in keys):
        raise PipelineStop(f"parent illisible : {weights} ne contient pas que des poids LoRA ({len(keys)} tenseurs)")
    adapter_sha256 = sha256(weights)
    lineage = report.get("lignee")
    if lineage is None:
        lineage = rebuilt_first_link(report, adapter_sha256, report_path)
    check_lineage(lineage, report_path)
    return Parent(folder=folder, report=report, config=config, adapter_sha256=adapter_sha256, lineage=lineage)


def base_of_adapter(config: dict[str, Any]) -> tuple[str | None, str | None]:
    """Dépôt et révision de la base d'un adaptateur. Hugging Face écrit le chemin du cache
    (`…/models--Qwen--Qwen3.5-4B/snapshots/<révision>`) dans `base_model_name_or_path` et laisse
    `revision` vide ; un nom de dépôt prend la révision de `revision`."""
    name = config.get("base_model_name_or_path")
    if not isinstance(name, str):
        return None, None
    match = SNAPSHOT_PATH.search(name.replace("\\", "/"))
    if match:
        return f"{match[1]}/{match[2]}", match[3]
    return name, config.get("revision")


def check_parent(parent: Parent) -> None:
    """Refuse un parent dont l'adaptateur ne se continue pas : autre base ou révision, autres
    modules, rang ou alpha, adaptateur de domaine, poids substitués depuis son rapport, ou
    lignée déjà à trois maillons."""
    report, config = parent.report, parent.config
    where = parent.folder
    base = report.get("base", {})
    if base.get("depot") != BASE_REPO or base.get("revision") != BASE_REVISION:
        raise PipelineStop(
            f"{BROKEN_CHAIN} : le parent {where} a la base {base.get('depot')} @ {base.get('revision')}, "
            f"la chaîne {BASE_REPO} @ {BASE_REVISION}")
    repo, revision = base_of_adapter(config)
    if repo != BASE_REPO or revision != BASE_REVISION:
        raise PipelineStop(
            f"{BROKEN_CHAIN} : l'adaptateur du parent {where} a été posé sur {repo} @ {revision}, "
            f"la chaîne sur {BASE_REPO} @ {BASE_REVISION}")
    h = report.get("hyperparametres", {})
    expected = {"modules": HYPERPARAMETERS["modules"], "rang": HYPERPARAMETERS["rang"],
                "alpha": HYPERPARAMETERS["alpha"]}
    found = {
        "rapport": {"modules": h.get("modules"), "rang": h.get("rang"), "alpha": h.get("alpha")},
        ADAPTER_CONFIG: {"modules": config.get("target_modules"), "rang": config.get("r"),
                         "alpha": config.get("lora_alpha")},
    }
    for source, values in found.items():
        different = [k for k in expected if values[k] != expected[k]]
        if different:
            raise PipelineStop(
                f"{BROKEN_CHAIN} : {', '.join(different)} du parent ({source}) diffèrent de ceux de la chaîne : "
                + "; ".join(f"{k} {values[k]!r} au lieu de {expected[k]!r}" for k in different))
    # Le reste de l'identité de l'adaptateur, celle que pose `get_peft_model` dans `train`.
    identity = {"lora_dropout": HYPERPARAMETERS["dropout"], "bias": "none", "use_dora": False,
                "use_rslora": False, "modules_to_save": None}
    defaults = {"use_dora": False, "use_rslora": False, "modules_to_save": None}
    different = [k for k, v in identity.items() if config.get(k, defaults.get(k)) != v]
    if different:
        raise PipelineStop(
            f"{BROKEN_CHAIN} : {', '.join(different)} de {ADAPTER_CONFIG} du parent diffèrent de ceux de la chaîne : "
            + "; ".join(f"{k} {config.get(k, defaults.get(k))!r} au lieu de {identity[k]!r}" for k in different))
    if report.get("donnees", {}).get("externe") is not None:
        raise PipelineStop(f"{BROKEN_CHAIN} : le parent {where} est un adaptateur de domaine, hors de la chaîne")
    if len(parent.lineage) >= MAX_LINKS:
        raise PipelineStop(
            f"le parent {where} a déjà {len(parent.lineage)} maillons (au plus {MAX_LINKS}) : "
            "recalage : entraînement de zéro sur le jeu complet")
    recorded = parent.lineage[-1]["adaptateur_sha256"]
    if recorded != parent.adapter_sha256:
        raise PipelineStop(
            f"adaptateur substitué : {where / PARENT_ADAPTER_DIR / ADAPTER_WEIGHTS} a l'empreinte "
            f"{parent.adapter_sha256}, le rapport du parent {recorded}")


def known_families() -> list[str]:
    """Les noms des familles du générateur, lus dans `training/src/families.rs` (le
    générateur refuse aussi tout nom inconnu, en code 2)."""
    try:
        names = FAMILY_NAME.findall(FAMILIES_SOURCE.read_text(encoding="utf-8"))
    except OSError as error:
        raise PipelineStop(f"familles illisibles : {FAMILIES_SOURCE} ({error})") from error
    if not names:
        raise PipelineStop(f"familles illisibles : aucune famille lue dans {FAMILIES_SOURCE}")
    return names


def focus_excluded() -> list[str]:
    """Les familles que le générateur refuse au jeu ciblé (`FOCUS_EXCLUDED`,
    `training/src/lib.rs`) : tirées seules, elles engendrent une phrase d'une barre."""
    try:
        match = FOCUS_EXCLUDED_LIST.search(GENERATOR_SOURCE.read_text(encoding="utf-8"))
    except OSError as error:
        raise PipelineStop(f"générateur illisible : {GENERATOR_SOURCE} ({error})") from error
    if match is None:
        raise PipelineStop(f"générateur illisible : FOCUS_EXCLUDED absent de {GENERATOR_SOURCE}")
    return re.findall(r'"([a-z0-9_]+)"', match[1])


def parse_families(text: str) -> list[str]:
    """`a,b` → les familles nommées ; un nom inconnu ou en double, ou une famille refusée au jeu
    ciblé par le générateur, est refusé avant le GPU (le générateur les refuse aussi)."""
    names = [n.strip() for n in text.split(",")]
    known = known_families()
    unknown = [n for n in names if n not in known]
    if unknown:
        raise PipelineStop(
            f"famille inconnue : {', '.join(repr(n) for n in unknown)} ; familles connues : {', '.join(known)}")
    twice = sorted({n for n in names if names.count(n) > 1})
    if twice:
        raise PipelineStop(f"famille nommée deux fois : {', '.join(twice)}")
    excluded = [n for n in names if n in focus_excluded()]
    if excluded:
        raise PipelineStop(
            f"famille refusée au jeu ciblé : {', '.join(excluded)} (tirée seule, elle engendre une phrase "
            "d'une barre ; FOCUS_EXCLUDED de training/src/lib.rs, à reprendre au lot 2 de 5.54)")
    return names


def check_continuation(args: argparse.Namespace) -> Continuation | None:
    """Les options de l'ajustement continu, puis le parent, lu et comparé : tout refus tombe
    avant le GPU et avant de toucher au dossier de sortie. Sans `--depuis`, rien."""
    parent_dir = getattr(args, "parent", None)
    families = getattr(args, "families", None)
    share = getattr(args, "recall_share", None)
    if parent_dir is None:
        extra = [flag for flag, value in (("--familles", families), ("--part-rappel", share)) if value is not None]
        if extra:
            raise PipelineStop(f"{', '.join(extra)} n'a de sens qu'avec --depuis <dossier d'un entraînement>")
        return None
    if args.external is not None:
        raise PipelineStop("--depuis et --exemples-externes sont exclusifs : l'adaptateur de domaine reste hors de la chaîne")
    if not families:
        raise PipelineStop("--depuis exige --familles a,b : les familles du jeu ciblé de la séance")
    share = RECALL_SHARE if share is None else share
    if not MIN_RECALL_SHARE <= share < 1:
        raise PipelineStop(
            f"--part-rappel {share} refusée : au moins {MIN_RECALL_SHARE} (le rappel garde ce que le parent sait), "
            "moins de 1 (il faut des exemples ciblés)")
    names = parse_families(families)
    count = CONTINUOUS_COUNT if args.examples is None else args.examples
    session_counts(count, share)
    parent = read_parent(parent_dir)
    check_parent(parent)
    return Continuation(parent=parent, families=names, recall_share=share, count=count)


GENERATOR_CONSTANT = re.compile(r"pub const (FOCUS_SEED|FOCUS_COUNT|DEFAULT_COUNT): \w+ = ([0-9_]+);")


def generator_constants() -> dict[str, int]:
    """`FOCUS_SEED`, `FOCUS_COUNT` et `DEFAULT_COUNT`, lus dans `training/src/lib.rs` : les
    bornes de ce que `training/tests/recouvrement.rs` garde."""
    try:
        found = {name: int(value.replace("_", ""))
                 for name, value in GENERATOR_CONSTANT.findall(GENERATOR_SOURCE.read_text(encoding="utf-8"))}
    except OSError as error:
        raise PipelineStop(f"générateur illisible : {GENERATOR_SOURCE} ({error})") from error
    if set(found) != {"FOCUS_SEED", "FOCUS_COUNT", "DEFAULT_COUNT"}:
        raise PipelineStop(f"générateur illisible : constantes absentes de {GENERATOR_SOURCE} ({sorted(found)})")
    return found


def session_counts(count: int, share: float) -> tuple[int, int]:
    """Exemples de rappel et exemples ciblés d'une séance de `count` exemples : le rappel est
    arrondi au-dessus, pour que sa part ne tombe jamais sous `share`. Le jeu ciblé ne dépasse pas
    `FOCUS_COUNT`, ni le rappel tiré (marge comprise) `DEFAULT_COUNT` : au-delà, ce qui est
    entraîné sortirait de ce que `recouvrement.rs` contrôle."""
    recall = math.ceil(count * share - 1e-9)
    focus = count - recall
    if recall < 1 or focus < 1:
        raise PipelineStop(f"--exemples {count} : trop peu pour une séance avec rappel et jeu ciblé")
    bounds = generator_constants()
    if focus > bounds["FOCUS_COUNT"]:
        raise PipelineStop(f"--exemples {count} : {focus} exemples ciblés, au plus {bounds['FOCUS_COUNT']} "
                           "(FOCUS_COUNT, la borne gardée par recouvrement.rs)")
    if recall + RECALL_MARGIN > bounds["DEFAULT_COUNT"]:
        raise PipelineStop(f"--exemples {count} : {recall} exemples de rappel et {RECALL_MARGIN} de marge, au plus "
                           f"{bounds['DEFAULT_COUNT']} (DEFAULT_COUNT, le jeu graine 1 gardé par recouvrement.rs)")
    return recall, focus


def session_mix(recall: list[data.Example], focus: list[data.Example], recall_count: int | None = None,
                min_share: float = MIN_RECALL_SHARE) -> tuple[list[data.Example], dict[str, Any]]:
    """Le jeu d'une séance continue : le rappel, coupé à `recall_count` (il est tiré avec une
    marge, pour que des exemples trop longs écartés ne le fassent pas tomber sous le plancher),
    puis le jeu ciblé, moins les demandes déjà vues (même prompt dans le rappel ou plus haut dans
    le jeu ciblé, quelle que soit la réponse), comptées. La part du rappel est comptée sur le jeu
    final ; sous `min_share`, la chaîne s'arrête."""
    drawn = len(recall)
    if recall_count is not None:
        recall = recall[:recall_count]
    recall_prompts = {e.prompt for e in recall}
    seen = set(recall_prompts)
    kept, from_recall, internal = [], 0, 0
    for example in focus:
        if example.prompt in seen:
            if example.prompt in recall_prompts:
                from_recall += 1
            else:
                internal += 1
            continue
        seen.add(example.prompt)
        kept.append(example)
    training = recall + kept
    share = len(recall) / len(training) if training else 0.0
    if share < min_share:
        raise PipelineStop(f"part du rappel comptée {share:.3f}, sous {min_share} : la séance oublierait")
    return training, {"exemples_rappel": len(recall), "exemples_cible": len(kept),
                      "rappel_marge_non_prise": drawn - len(recall),
                      "cible_doublons_du_rappel": from_recall, "cible_doublons_internes": internal,
                      "part_rappel_comptee": round(share, 4)}


def check_counts(plan: dict[str, tuple[Path, int, int, list[str]]], read: dict[str, list[data.Example]]) -> None:
    """Chaque jeu engendré a le compte demandé : un jeu plus court (familles épuisées, fichier
    coupé) arrête la chaîne avant l'entraînement, en nommant le fichier."""
    for name, (path, _, wanted, _) in plan.items():
        if name in read and len(read[name]) != wanted:
            raise PipelineStop(f"{path} : {len(read[name])} exemples, {wanted} demandés")


def lineage(continuation: Continuation | None, commit: str, torch_seed: int) -> list[dict[str, Any]]:
    """La lignée du rapport : un maillon pour un entraînement de zéro, celle du parent plus un
    maillon pour une séance continue. L'empreinte de l'adaptateur produit se remplit après
    l'entraînement."""
    link: dict[str, Any] = {"maillon": 1, "depart": "base", "commit": commit, "graine_torch": torch_seed,
                            "adaptateur_sha256": None}
    if continuation is None:
        return [link]
    links = [dict(item) for item in continuation.parent.lineage]
    link |= {"maillon": len(links) + 1, "depart": continuation.parent.adapter_sha256,
             "familles": continuation.families, "part_rappel": continuation.recall_share,
             "exemples": continuation.count}
    return links + [link]


def effective_hyperparameters(continuation: Continuation | None) -> dict[str, Any]:
    return HYPERPARAMETERS | ({"taux_apprentissage": CONTINUOUS_LEARNING_RATE} if continuation else {})


def check_resume_parent(report: dict[str, Any], continuation: Continuation | None) -> None:
    """À la reprise, refuse un autre parent ou d'autres familles (ou une autre part de rappel)
    que ceux de l'entraînement commencé : l'adaptateur et les jeux déjà écrits ne leur
    correspondraient pas. Un rapport d'avant 5.54 est un départ de zéro."""
    if not any(key in report for key in ("depart", "lignee", "donnees")):
        return
    lignee = report.get("lignee")
    last = lignee[-1] if isinstance(lignee, list) and lignee and isinstance(lignee[-1], dict) else {}
    families = last.get("familles")
    before = (report.get("depart", "base"), sorted(families) if isinstance(families, list) else families,
              last.get("part_rappel"), last.get("exemples"))
    now = ((continuation.parent.adapter_sha256, sorted(continuation.families), continuation.recall_share,
            continuation.count) if continuation else ("base", None, None, None))
    if before != now:
        raise PipelineStop(
            "--reprendre avec un autre parent, d'autres familles, une autre part de rappel ou un autre "
            "nombre d'exemples que l'entraînement commencé : relancez avec les mêmes options, ou dans un dossier vide")


def check_loaded_keys(missing: list[str], unexpected: list[str], loaded: int) -> None:
    """Après le chargement des poids du parent : aucune clé inattendue, aucun paramètre LoRA
    resté à son initialisation (les poids de la base, absents de l'adaptateur, ne comptent pas)."""
    lora_missing = [k for k in missing if "lora_" in k]
    if unexpected or lora_missing or loaded == 0:
        raise PipelineStop(
            f"poids du parent mal chargés : {len(unexpected)} clés inattendues {unexpected[:3]}, "
            f"{len(lora_missing)} paramètres LoRA non chargés {lora_missing[:3]}, {loaded} tenseurs lus")


def same_tensor(a, b) -> bool:
    import torch

    return torch.equal(a.detach().to("cpu", b.dtype), b.detach().to("cpu"))


def check_loaded_values(parent: dict[str, Any], now: dict[str, Any], same=same_tensor) -> int:
    """Après le chargement : chaque matrice `lora_B` de l'adaptateur (nulle à l'initialisation)
    égale celle du parent. Rend le nombre de matrices comparées."""
    keys = sorted(k for k in parent if "lora_B" in k)
    if not keys:
        raise PipelineStop("poids du parent mal chargés : aucune matrice lora_B dans l'adaptateur du parent")
    for key in keys:
        if key not in now or not same(now[key], parent[key]):
            raise PipelineStop(f"poids du parent mal chargés : {key} n'égale pas celle du parent")
    return len(keys)


def load_parent_weights(model, adapter_dir: Path, expected_sha256: str, load_file=None, set_state=None,
                        get_state=None, same=None) -> int:
    """Pose les poids LoRA du parent sur l'adaptateur que `get_peft_model` vient de créer (même
    configuration) : le chemin d'Unsloth reste celui d'un entraînement de zéro. L'empreinte du
    fichier est revérifiée juste avant la lecture ; après, les clés et les matrices `lora_B`.
    Les fonctions de peft et de safetensors se remplacent pour les tests."""
    if load_file is None:
        from safetensors.torch import load_file
    if set_state is None:
        from peft import set_peft_model_state_dict as set_state
    if get_state is None:
        from peft import get_peft_model_state_dict as get_state
    path = adapter_dir / ADAPTER_WEIGHTS
    found = sha256(path)
    if found != expected_sha256:
        raise PipelineStop(f"adaptateur substitué : {path} a l'empreinte {found}, lue avant le GPU {expected_sha256}")
    state = load_file(str(path))
    result = set_state(model, state)
    check_loaded_keys(list(result.missing_keys), list(result.unexpected_keys), len(state))
    return check_loaded_values(state, get_state(model), same or same_tensor)


@dataclass(frozen=True)
class External:
    """Le fichier externe lu et découpé, et ce que le rapport en garde : empreintes et
    comptes, jamais un exemple."""

    training: list[data.Example]
    validation: list[data.Example]
    info: dict[str, Any]


def read_dataset(path: Path) -> list[data.Example]:
    """Lit un fichier d'exemples externe ; toute ligne mal formée arrête la chaîne avec le nom
    du fichier et le numéro de la ligne."""
    if not path.is_file():
        raise PipelineStop(f"fichier d'exemples introuvable : {path}")
    try:
        examples = data.read_jsonl(path)
    except data.UnreadableDataset as error:
        raise PipelineStop(f"{path} : {error}") from error
    except (OSError, UnicodeDecodeError) as error:
        raise PipelineStop(f"{path} : illisible ({error})") from error
    if not examples:
        raise PipelineStop(f"{path} : aucun exemple")
    # La réponse doit se lire comme un plan : sinon la notation échouerait après l'entraînement.
    numbers = [n for n, line in enumerate(path.read_text(encoding="utf-8").split("\n"), start=1) if line.strip()]
    for number, example in zip(numbers, examples):
        if scoring.read_plan(data.expected_answer(example)) is None:
            raise PipelineStop(f"{path} : ligne {number} : la réponse n'est pas un plan lisible")
    return examples


def read_external(path: Path, validation_path: Path | None) -> External:
    """Lit le fichier externe et sa validation : le second fichier s'il est donné (rien
    n'est alors retiré du premier), sinon une part du premier tirée par la graine."""
    examples = read_dataset(path)
    info: dict[str, Any] = {"empreinte": sha256(path), "exemples": len(examples)}
    if validation_path is not None:
        training, validation = examples, read_dataset(validation_path)
        info |= {"validation_source": "fichier", "empreinte_validation": sha256(validation_path)}
    else:
        if len(examples) < 2:
            raise PipelineStop(f"{path} : au moins deux exemples, pour en garder un pour la validation")
        training, validation = data.split_validation(examples, EXTERNAL_VALIDATION_SHARE, VALIDATION_SEED)
        info |= {"validation_source": "part du fichier", "part_validation": EXTERNAL_VALIDATION_SHARE}
    info["exemples_validation"] = len(validation)
    return External(training=training, validation=validation, info=info)


def mix(
    civil_training: list[data.Example],
    civil_validation: list[data.Example],
    external: External | None,
) -> dict[str, Any]:
    """Assemble les jeux : l'entraînement réunit le civil et l'externe, et chaque validation
    perd les prompts vus à l'entraînement, comptés. Sans fichier externe, le seul retrait de 5.17."""
    external_training = external.training if external else []
    training = civil_training + external_training
    civil_kept, civil_overlap = data.drop_overlap(civil_validation, training)
    external_kept, external_overlap = data.drop_overlap(external.validation if external else [], training)
    return {
        "civil_training": civil_training,
        "external_training": external_training,
        "civil_validation": civil_kept,
        "external_validation": external_kept,
        "civil_overlap": civil_overlap,
        "external_overlap": external_overlap,
    }


def check_resume(report: dict[str, Any], external: External | None, civil: bool) -> None:
    """À la reprise, refuse un autre fichier externe ou un autre mélange que ceux de
    l'entraînement commencé (lus dans son rapport) : l'adaptateur déjà écrit ne leur
    correspondrait pas."""
    if "donnees" not in report:
        return
    before = report["donnees"].get("externe")
    now = external.info | {"melange": mode_label(civil)} if external is not None else None
    keys = ("empreinte", "empreinte_validation", "melange")
    if (before is None) != (now is None) or (
        before is not None and any(before.get(k) != now.get(k) for k in keys)
    ):
        raise PipelineStop(
            "--reprendre avec d'autres exemples externes ou un autre mélange que l'entraînement commencé : "
            "relancez avec les mêmes options, ou dans un dossier vide"
        )


def mode_label(civil: bool) -> str:
    return "civil + externe" if civil else "externe seul"


def trial_example(external_validation: list[data.Example],
                  civil_validation: list[data.Example]) -> tuple[data.Example, bool]:
    """L'exemple de l'essai `llama-completion`, et s'il est externe (son contenu ne s'écrit
    alors ni au rapport ni au journal). Un plan d'appels externe d'abord, puis civil, puis le
    premier exemple externe."""
    for examples, private in ((external_validation, True), (civil_validation, False)):
        try:
            return first_calls_example(examples), private
        except PipelineStop:
            pass
    if external_validation:
        return external_validation[0], True
    raise PipelineStop("aucun exemple de validation pour l'essai")


def gpu_memory() -> tuple[int | None, str | None]:
    import torch

    if not torch.cuda.is_available():
        return None, None
    properties = torch.cuda.get_device_properties(0)
    return properties.total_memory, properties.name


def run(command: list[str], **options: Any) -> subprocess.CompletedProcess:
    """Lance un outil ; en cas d'échec, arrête la chaîne en citant la fin de sa sortie d'erreur
    quand elle a été capturée."""
    log("$ " + " ".join(str(c) for c in command))
    result = subprocess.run(command, **options)
    if result.returncode != 0:
        message = f"échec ({result.returncode}) : {' '.join(str(c) for c in command)}"
        stderr = result.stderr
        if isinstance(stderr, bytes):
            stderr = stderr.decode("utf-8", errors="replace")
        if stderr:
            message += f"\n--- fin de la sortie d'erreur ---\n{stderr[-2000:]}"
        raise PipelineStop(message)
    return result


def sha256(path: Path) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def temporary(path: Path) -> Path:
    """Nom temporaire, dans le même dossier, sous lequel un outil écrit avant le renommage."""
    return path.with_name(path.name + ".tmp")


def write_atomically(path: Path, command: list[str], **options: Any) -> None:
    """Lance `command`, qui écrit dans `temporary(path)`, puis renomme : un fichier coupé en
    pleine écriture ne porte jamais le nom final, et `--reprendre` ne le reprend jamais."""
    partial = temporary(path)
    partial.unlink(missing_ok=True)
    run(command, **options)
    if not partial.exists():
        raise PipelineStop(f"l'outil n'a pas écrit {partial}")
    partial.replace(path)


# -- étape 3 : génération ------------------------------------------------------------------


def generation_plan(folder: Path, count: int, validation_count: int,
                    continuation: Continuation | None = None) -> dict[str, tuple[Path, int, int, list[str]]]:
    """Les jeux à engendrer, dans l'ordre (la validation en dernier : sa présence dit, à la
    reprise, que les autres sont écrits). De zéro : graine 1 pour l'entraînement. Séance
    continue : le rappel, préfixe du jeu graine 1 (le générateur est déterministe, ses premiers
    exemples ne dépendent pas du compte), puis le jeu ciblé, graine 3, sur les familles nommées."""
    if continuation is None:
        sets = {"entrainement": (folder / "entrainement.jsonl", TRAINING_SEED, count, [])}
    else:
        recall, focus = session_counts(count, continuation.recall_share)
        sets = {
            "rappel": (folder / "rappel.jsonl", TRAINING_SEED, recall + RECALL_MARGIN, []),
            "cible": (folder / "cible.jsonl", FOCUS_SEED, focus, ["--familles", ",".join(continuation.families)]),
        }
    return sets | {"validation": (folder / "validation.jsonl", VALIDATION_SEED, validation_count, [])}


def generate(folder: Path, count: int, validation_count: int,
             continuation: Continuation | None = None) -> dict[str, Path]:
    folder.mkdir(parents=True, exist_ok=True)
    sets = generation_plan(folder, count, validation_count, continuation)
    for path, seed, n, extra in sets.values():
        # La sortie d'erreur est capturée : un refus du générateur (famille inconnue, code 2 ;
        # jeu ciblé qui reprend une phrase de barre, code 3, identifiant nommé) entre au message.
        result = run(
            ["cargo", "run", "-q", "-p", "cassini-training", "--release", "--",
             "--format", "chat", "--seed", str(seed), "--count", str(n), *extra, "--out", str(path)],
            cwd=ROOT, capture_output=True, text=True,
        )
        sys.stderr.write(result.stderr or "")
    return {name: path for name, (path, _, _, _) in sets.items()}


# -- étapes 5 et 6 : entraînement et validation --------------------------------------------


def set_seeds(seed: int) -> None:
    import numpy
    import torch
    import transformers

    random.seed(seed)
    numpy.random.seed(seed)
    torch.manual_seed(seed)
    torch.cuda.manual_seed_all(seed)
    transformers.set_seed(seed)


def load_model(base_dir: str):
    import torch
    from transformers import AutoModelForImageTextToText, BitsAndBytesConfig

    quantization = BitsAndBytesConfig(
        load_in_4bit=True,
        bnb_4bit_quant_type="nf4",
        bnb_4bit_use_double_quant=True,
        bnb_4bit_compute_dtype=torch.bfloat16,
    )
    install_fast_kernels()
    return AutoModelForImageTextToText.from_pretrained(
        base_dir,
        quantization_config=quantization,
        dtype=torch.bfloat16,
        device_map={"": 0},
    )


# `--noyaux-reference` : les implémentations de référence de transformers, pour comparer.
FAST_KERNELS = True
# `--sans-unsloth` : le chargement et l'adaptateur par transformers et peft seuls, comme avant.
USE_UNSLOTH = True
# Mémoire de la carte (2026-11-09). Sans segments extensibles, le cache de torch se fragmente
# sur les exemples longs : son pic passait de 6,4 à 7,5 Gio, la carte de 8 Go débordait en
# mémoire centrale sous WSL et chaque pas devenait 2,5 fois plus lent. Unsloth les retire sous
# WSL (une erreur du pilote, d'après lui) : mesurés ici sans erreur, ils sont reposés après son
# import. `CASSINI_SEGMENTS_EXTENSIBLES=0` les retire, pour comparer.
EXPANDABLE_SEGMENTS = os.environ.get("CASSINI_SEGMENTS_EXTENSIBLES", "1") == "1"


def configure_allocator() -> dict[str, Any]:
    """Réglages de l'allocateur de torch, posés après l'import d'Unsloth (qui retire
    `expandable_segments` sous WSL) et avant le chargement du modèle."""
    import torch

    if EXPANDABLE_SEGMENTS:
        torch.cuda.memory._set_allocator_settings("expandable_segments:True")
    return {"segments_extensibles": EXPANDABLE_SEGMENTS}


def import_unsloth() -> str:
    """Importe Unsloth, qui doit passer avant transformers pour le corriger. La boucle calcule
    sa perte elle-même : Unsloth doit donc rendre les logits (sinon il les fusionne à la perte)."""
    os.environ.setdefault("UNSLOTH_RETURN_LOGITS", "1")
    # Unsloth écrit le code qu'il compile dans `unsloth_compiled_cache/` du dossier courant (la
    # racine du dépôt) : on l'envoie dans le cache hors dépôt.
    os.environ.setdefault("UNSLOTH_COMPILE_LOCATION", str(cache_dir() / "unsloth_compiled_cache"))
    if "transformers" in sys.modules:
        raise PipelineStop("Unsloth doit être importé avant transformers (erreur de la chaîne)")
    import unsloth

    return unsloth.__version__


def training_path() -> dict[str, str]:
    """Ce que le rapport garde du chemin d'entraînement."""
    if USE_UNSLOTH:
        return {"chargement": "Unsloth FastModel (nf4, double quantification, bf16)",
                "adaptateur": "Unsloth FastModel.get_peft_model", "checkpointing": "unsloth (activations en mémoire centrale)",
                "unsloth": True}
    return {"chargement": "transformers + bitsandbytes (nf4, double quantification, bf16)",
            "adaptateur": "peft get_peft_model", "checkpointing": "gradient checkpointing de transformers",
            "unsloth": False}
LINEAR_ATTENTION_FUNCTIONS = ("torch_chunk_gated_delta_rule", "torch_recurrent_gated_delta_rule",
                              "causal_conv1d_fn", "causal_conv1d_update")
_ORIGINAL_FUNCTIONS: dict[str, Any] = {}


def install_fast_kernels() -> dict[str, str]:
    """Branche les noyaux rapides des couches à attention linéaire de Qwen3.5, et dit lesquels.

    `flash-linear-attention` (Triton, sans compilation) est pris par transformers de lui-même
    pour la règle delta. La convolution causale, elle, n'est cherchée que dans le paquet
    `causal-conv1d`, sans roue pour torch 2.14 : on la remplace par celle, en Triton, de
    `flash-linear-attention`. Le calcul reste celui de l'implémentation de référence (même
    dtype, mêmes valeurs aux arrondis près).
    Avec `FAST_KERNELS` faux, les quatre fonctions reprennent leur implémentation PyTorch.
    """
    reference = {"regle_delta": "PyTorch (référence)", "convolution": "PyTorch (référence)"}
    try:
        from fla.modules.causal_conv1d import causal_conv1d as fla_causal_conv1d
    except ImportError:
        return reference
    from transformers.models.qwen3_5 import modeling_qwen3_5 as qwen

    if not _ORIGINAL_FUNCTIONS:
        _ORIGINAL_FUNCTIONS.update({name: getattr(qwen, name) for name in LINEAR_ATTENTION_FUNCTIONS})
    if not FAST_KERNELS:
        for name, function in _ORIGINAL_FUNCTIONS.items():
            # Le décorateur de transformers garde la fonction PyTorch d'origine sous `__wrapped__`.
            setattr(qwen, name, getattr(function, "__wrapped__", function))
        return reference
    for name, function in _ORIGINAL_FUNCTIONS.items():
        setattr(qwen, name, function)

    def causal_conv1d_fn(hidden_states, weight, bias=None, activation=None, **kwargs):
        # transformers passe [lot, canaux, temps] ; flash-linear-attention attend [lot, temps, canaux].
        # Comme la référence, le calcul se fait dans le dtype des poids, puis revient à celui de l'entrée.
        out, _ = fla_causal_conv1d(
            x=hidden_states.transpose(1, 2).to(weight.dtype).contiguous(),
            weight=weight,
            bias=bias,
            activation=activation,
        )
        return out.transpose(1, 2).to(hidden_states.dtype)

    qwen.causal_conv1d_fn = causal_conv1d_fn
    return {"regle_delta": "flash-linear-attention", "convolution": "flash-linear-attention (Triton)"}


def encode(tokenizer, example: data.Example) -> tuple[list[int], int]:
    """Jetons du prompt puis de la réponse, et la longueur de la réponse. Le prompt et la
    réponse se découpent à part : le modèle répond après les jetons du prompt, comme en
    production. Les balises chatml sont lues comme jetons spéciaux."""
    prompt = tokenizer(example.prompt, add_special_tokens=False)["input_ids"]
    answer = tokenizer(example.completion, add_special_tokens=False)["input_ids"]
    return prompt + answer, len(answer)


def train(base_dir: str, examples: list[data.Example], adapter_dir: Path, max_steps: int | None,
          long_first: bool = False, hyperparameters: dict[str, Any] | None = None,
          start_adapter: Path | None = None, start_sha256: str | None = None) -> dict[str, Any]:
    import torch
    import torch.nn.functional as F
    from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
    from transformers import AutoTokenizer, get_cosine_schedule_with_warmup

    set_seeds(TORCH_SEED)
    h = HYPERPARAMETERS if hyperparameters is None else hyperparameters
    allocator = configure_allocator()
    log(f"allocateur : {allocator}")
    tokenizer = AutoTokenizer.from_pretrained(base_dir)
    encoded = [encode(tokenizer, e) for e in examples]

    if USE_UNSLOTH:
        # Même méthode (QLoRA nf4, double quantification, calcul en bf16), mêmes rang, alpha,
        # dropout et modules ; Unsloth compile chaque couche et décharge les activations.
        from unsloth import FastModel

        model, _ = FastModel.from_pretrained(model_name=base_dir, max_seq_length=MAX_LENGTH, load_in_4bit=True,
                                             dtype=torch.bfloat16, full_finetuning=False)
        install_fast_kernels()
        model = FastModel.get_peft_model(
            model, r=h["rang"], lora_alpha=h["alpha"], lora_dropout=h["dropout"], target_modules=h["modules"],
            bias="none", finetune_vision_layers=False, finetune_language_layers=True,
            use_gradient_checkpointing="unsloth", random_state=TORCH_SEED)
    else:
        model = load_model(base_dir)
        model = prepare_model_for_kbit_training(
            model, use_gradient_checkpointing=True, gradient_checkpointing_kwargs={"use_reentrant": False}
        )
        model = get_peft_model(
            model,
            LoraConfig(r=h["rang"], lora_alpha=h["alpha"], lora_dropout=h["dropout"],
                       target_modules=h["modules"], bias="none", task_type="CAUSAL_LM"),
        )
    loaded = None
    if start_adapter is not None:
        # Séance continue (5.54) : l'adaptateur vient d'être posé, à la même configuration ; il
        # prend les poids du parent avant que l'optimiseur ne voie ses paramètres.
        loaded = load_parent_weights(model, start_adapter, start_sha256)
        log(f"poids du parent chargés : {loaded} matrices lora_B vérifiées, de {start_adapter}")
    trainable, total = model.get_nb_trainable_parameters()
    log(f"paramètres entraînés : {trainable:,} sur {total:,}")

    order = []
    for epoch in range(h["epoques"]):
        indices = list(range(len(encoded)))
        random.Random(TORCH_SEED * 1000 + epoch).shuffle(indices)
        order.extend(indices)
    if long_first:
        # Mesure de la mémoire (`--longs-d-abord`) : les exemples les plus longs d'abord, pour
        # que le pic se voie en quelques pas ; l'ordre d'un vrai entraînement reste tiré au sort.
        order.sort(key=lambda i: -len(encoded[i][0]))
    total_steps = len(order) // h["accumulation"]
    if max_steps is not None:
        total_steps = min(total_steps, max_steps)
    # Seuls les exemples d'un pas complet sont vus : un reste sans pas d'optimiseur ne compterait pas.
    order = order[: total_steps * h["accumulation"]]
    parameters = [p for p in model.parameters() if p.requires_grad]
    optimizer = torch.optim.AdamW(parameters, lr=h["taux_apprentissage"], weight_decay=0.0)
    schedule = get_cosine_schedule_with_warmup(optimizer, int(total_steps * h["echauffement"]), total_steps)

    model.train()
    torch.cuda.reset_peak_memory_stats()
    history, window, start = [], [], time.time()
    for rank, index in enumerate(order, start=1):
        tokens, answer_length = encoded[index]
        inputs = torch.tensor([tokens], device="cuda")
        # Logits sur les seules positions qui prédisent la réponse : la perte ne porte que
        # sur elle, et on évite des logits de 248 000 entrées sur tout le prompt.
        outputs = model(input_ids=inputs, logits_to_keep=answer_length + 1, use_cache=False)
        # Les positions qui prédisent la réponse, que le modèle rende les seuls logits gardés
        # ou toute la séquence.
        logits = outputs.logits[0, -answer_length - 1:-1].float()
        target = inputs[0, -answer_length:]
        loss = F.cross_entropy(logits, target)
        (loss / h["accumulation"]).backward()
        window.append(loss.item())
        if rank % h["accumulation"] == 0:
            torch.nn.utils.clip_grad_norm_(parameters, h["norme_max_gradient"])
            optimizer.step()
            schedule.step()
            optimizer.zero_grad(set_to_none=True)
            step = rank // h["accumulation"]
            if step % 10 == 0 or step == total_steps:
                mean = sum(window) / len(window)
                history.append({"pas": step, "perte": round(mean, 5)})
                elapsed = time.time() - start
                log(f"pas {step}/{total_steps} perte {mean:.4f} "
                    f"({elapsed / 60:.1f} min, reste ~{elapsed / step * (total_steps - step) / 60:.0f} min, "
                    f"pic {torch.cuda.max_memory_reserved() / 1024**3:.2f} Gio)")
                window = []

    model.save_pretrained(adapter_dir)
    return {
        "modele": model,
        "pas": total_steps,
        "exemples_vus": len(order),
        "perte": history,
        "perte_finale": history[-1]["perte"] if history else None,
        "duree_s": round(time.time() - start),
        "pic_memoire_gpu_octets": torch.cuda.max_memory_reserved(),
        "parametres_entraines": trainable,
        "allocateur": allocator,
        **({"matrices_lora_b_du_parent_verifiees": loaded} if loaded is not None else {}),
    }


def validate(model, tokenizer, examples: list[data.Example], outputs_file: Path) -> dict[str, Any]:
    import torch

    end = tokenizer.convert_tokens_to_ids(data.END_OF_ANSWER)
    model.eval()
    model.config.use_cache = True
    score, start = scoring.Score(), time.time()
    with open(outputs_file, "w", encoding="utf-8") as outputs, torch.no_grad():
        for rank, example in enumerate(examples, start=1):
            inputs = tokenizer(example.prompt, add_special_tokens=False, return_tensors="pt")["input_ids"].to("cuda")
            produced = model.generate(
                input_ids=inputs,
                max_new_tokens=HYPERPARAMETERS["jetons_max_generes"],
                do_sample=False,
                eos_token_id=end,
                pad_token_id=end,
            )
            text = tokenizer.decode(produced[0, inputs.shape[1]:], skip_special_tokens=False)
            expected = data.expected_answer(example)
            match = score.record(text, expected)
            outputs.write(json.dumps({"rang": rank, "juste": match, "attendu": expected, "sortie": text},
                                     ensure_ascii=False) + "\n")
            if rank % 25 == 0:
                log(f"validation {rank}/{len(examples)} : {sum(score.correct.values())} justes")
    result = score.to_dict()
    result["duree_s"] = round(time.time() - start)
    return result


# -- étapes 7 à 9 : llama.cpp --------------------------------------------------------------


def prepare_llama_cpp() -> Path:
    """Clone llama.cpp au commit fixé (hors dépôt) et construit les outils sur CPU."""
    folder = cache_dir() / "llama.cpp"
    if not (folder / ".git").exists():
        folder.mkdir(parents=True, exist_ok=True)
        run(["git", "init", "-q"], cwd=folder)
        run(["git", "remote", "add", "origin", LLAMA_CPP_URL], cwd=folder)
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=folder, capture_output=True, text=True).stdout.strip()
    if head != LLAMA_CPP_COMMIT:
        run(["git", "fetch", "-q", "--depth", "1", "origin", LLAMA_CPP_COMMIT], cwd=folder)
        run(["git", "checkout", "-q", "--detach", LLAMA_CPP_COMMIT], cwd=folder)
        shutil.rmtree(folder / "build", ignore_errors=True)
        shutil.rmtree(folder / "build-cuda", ignore_errors=True)
    # llama-tokenize sert à `parite_decoupeurs.py` (tâche 4b).
    tools = ["llama-quantize", "llama-export-lora", "llama-completion", "llama-tokenize"]
    if not all((folder / "build" / "bin" / t).exists() for t in tools):
        run(["cmake", "-S", ".", "-B", "build", "-DGGML_CUDA=OFF", "-DLLAMA_CURL=OFF",
             "-DLLAMA_BUILD_TESTS=OFF", "-DCMAKE_BUILD_TYPE=Release"], cwd=folder)
        run(["cmake", "--build", "build", "-j", str(os.cpu_count() or 4), "--target", *tools], cwd=folder)
    return folder


def convert(llama: Path, base_dir: str, adapter_dir: Path, output: Path, work: Path, resume: bool,
            outputs: Outputs = CIVIL_OUTPUTS) -> None:
    py = sys.executable
    base_f16 = work / "base-f16.gguf"
    merged_f16 = work / "fusionne-f16.gguf"
    adapter = output / outputs.adapter
    quantize = llama / "build" / "bin" / "llama-quantize"

    def todo(path: Path) -> bool:
        return not (resume and path.exists())

    merge = outputs.merged is not None
    if todo(output / BASE_NAME) or (merge and todo(output / outputs.merged)):
        if todo(base_f16):
            write_atomically(base_f16, [py, str(llama / "convert_hf_to_gguf.py"), base_dir, "--outtype", "f16",
                                        "--outfile", str(temporary(base_f16))])
    if todo(adapter):
        write_atomically(adapter, [py, str(llama / "convert_lora_to_gguf.py"), str(adapter_dir), "--base", base_dir,
                                   "--outtype", "f16", "--outfile", str(temporary(adapter))])
    if merge and todo(output / outputs.merged):
        if todo(merged_f16):
            write_atomically(merged_f16, [str(llama / "build" / "bin" / "llama-export-lora"), "-m", str(base_f16),
                                          "--lora", str(adapter), "-o", str(temporary(merged_f16))])
        write_atomically(output / outputs.merged,
                         [str(quantize), str(merged_f16), str(temporary(output / outputs.merged)), QUANTIZATION])
    if todo(output / BASE_NAME):
        write_atomically(output / BASE_NAME,
                         [str(quantize), str(base_f16), str(temporary(output / BASE_NAME)), QUANTIZATION])
    # Les intermédiaires f16 (plusieurs Go chacun) ne servent plus.
    for intermediate in (base_f16, merged_f16):
        intermediate.unlink(missing_ok=True)


def first_calls_example(examples: list[data.Example]) -> data.Example:
    """Le premier exemple de validation dont la réponse attendue est un plan d'appels."""
    for example in examples:
        plan = scoring.read_plan(data.expected_answer(example))
        if plan is not None and plan["kind"] == "calls":
            return example
    raise PipelineStop("aucun exemple de validation n'attend un plan d'appels")


def try_model(llama: Path, model_args: list[str], example: data.Example, work: Path, label: str,
              private: bool = False) -> dict[str, Any]:
    """Une demande de validation passée à `llama-completion` (prompt brut, déjà en chatml,
    décodage glouton) : la sortie doit se lire comme un plan JSON. Sur un exemple externe
    (`private`), ni la sortie ni l'attendu ne s'écrivent au journal ou au rapport."""
    prompt_file = work / "essai-prompt.txt"
    prompt_file.write_text(example.prompt, encoding="utf-8")
    result = run(
        [str(llama / "build" / "bin" / "llama-completion"), *model_args, "-f", str(prompt_file),
         "-n", str(HYPERPARAMETERS["jetons_max_generes"]), "--temp", "0", "-no-cnv", "--no-display-prompt",
         "-c", str(MAX_LENGTH + HYPERPARAMETERS["jetons_max_generes"]), "--seed", str(TORCH_SEED)],
        capture_output=True, text=True,
    )
    output = result.stdout.strip()
    # llama-completion peut écrire la marque de fin ou « [end of text] » après le plan.
    for end in ("[end of text]", data.END_OF_ANSWER):
        output = output.split(end)[0].strip()
    expected = data.expected_answer(example)
    trial = {
        "outil": "llama-completion",
        "modele": label,
        "sortie": output,
        "attendu": expected,
        "se_lit_comme_un_plan": scoring.read_plan(output) is not None,
        "juste": scoring.plan_matches(output, expected),
    }
    if private:
        del trial["sortie"], trial["attendu"]
        trial["exemple"] = "externe, contenu non consigné"
        log(f"essai llama-completion ({label}) : plan lisible {trial['se_lit_comme_un_plan']}, juste {trial['juste']}")
    else:
        log(f"essai llama-completion ({label}) : {output}")
    return trial


# -- validation par llama.cpp sur le GPU ---------------------------------------------------


def cuda_toolkit() -> Path:
    """Le compilateur CUDA des roues `nvidia-cuda-nvcc`, `-nvvm`, `-crt`, `-cccl` épinglées dans
    `uv.lock`, posé par uv dans l'environnement à côté des bibliothèques de torch : rien n'est
    installé au niveau du système. nvcc cherche ses bibliothèques sous `lib64` et l'éditeur de
    liens leurs noms sans numéro : deux liens symboliques, idempotents, les lui donnent."""
    import nvidia

    root = Path(next(iter(nvidia.__path__))) / "cu13"
    if not (root / "bin" / "nvcc").exists():
        raise PipelineStop(f"nvcc introuvable sous {root} : relancez `uv sync --project training/entrainement`")
    if not (root / "lib64").exists():
        (root / "lib64").symlink_to("lib")
    for name in ("cudart", "cublas", "cublasLt"):
        link = root / "lib" / f"lib{name}.so"
        if not link.exists():
            versioned = sorted((root / "lib").glob(f"lib{name}.so.*"))
            if not versioned:
                raise PipelineStop(f"lib{name}.so introuvable sous {root / 'lib'}")
            link.symlink_to(versioned[0].name)
    return root


def cuda_toolkit_libraries() -> Path | None:
    """Les bibliothèques CUDA des roues de l'environnement, s'il y en a (sinon le serveur prend
    celles du système, s'il en a)."""
    try:
        return cuda_toolkit() / "lib"
    except (ImportError, PipelineStop):
        return None


def server_environment() -> dict[str, str]:
    """L'environnement de `llama-server` : les bibliothèques CUDA des roues de l'environnement
    courant, puis `libcuda.so`, celle du pilote Windows, exposée par WSL."""
    libraries = cuda_toolkit_libraries()
    paths = (str(libraries) if libraries else None, "/usr/lib/wsl/lib", os.environ.get("LD_LIBRARY_PATH"))
    return os.environ | {"LD_LIBRARY_PATH": ":".join(p for p in paths if p)}


def prepare_llama_cpp_gpu(folder: Path) -> Path:
    """Construit `llama-server` sur CUDA, au même commit que les outils sur CPU, dans
    `build-cuda/` (hors dépôt), pour la seule carte de ce poste, et vérifie qu'il voit le GPU.
    Rend le dossier des binaires. CMake efface sa configuration quand le chemin de nvcc change
    (un autre environnement) : la construction est alors refaite de zéro, jamais à moitié."""
    build = folder / "build-cuda"
    server = build / "bin" / "llama-server"
    stamp = build / "cassini-nvcc.txt"
    toolkit = cuda_toolkit()
    nvcc = toolkit / "bin" / "nvcc"
    if not (server.exists() and stamp.exists() and stamp.read_text(encoding="utf-8") == str(nvcc)):
        shutil.rmtree(build, ignore_errors=True)
        run(["cmake", "-S", ".", "-B", "build-cuda", "-DGGML_CUDA=ON", "-DLLAMA_CURL=OFF", "-DLLAMA_OPENSSL=OFF",
             "-DLLAMA_BUILD_TESTS=OFF", "-DCMAKE_BUILD_TYPE=Release", f"-DCUDAToolkit_ROOT={toolkit}",
             f"-DCMAKE_CUDA_COMPILER={nvcc}", "-DCMAKE_CUDA_ARCHITECTURES=native",
             # libcuda.so vient du pilote Windows, exposé par WSL ; les bibliothèques CUDA, des roues.
             # Elles ne sont citées qu'à l'édition de liens (`-rpath-link`) : à l'exécution,
             # `server_environment` les prend dans l'environnement courant.
             "-DCMAKE_LIBRARY_PATH=/usr/lib/wsl/lib",
             f"-DCMAKE_EXE_LINKER_FLAGS=-Wl,-rpath-link,{toolkit / 'lib'}",
             f"-DCMAKE_SHARED_LINKER_FLAGS=-Wl,-rpath-link,{toolkit / 'lib'}"], cwd=folder)
        run(["cmake", "--build", "build-cuda", "-j", str(os.cpu_count() or 4), "--target", "llama-server"],
            cwd=folder)
        stamp.write_text(str(nvcc), encoding="utf-8")
    # Sans GPU vu, llama-server se rabattrait en silence sur le CPU, dix fois plus lent.
    devices = run([str(server), "--list-devices"], capture_output=True, text=True, env=server_environment())
    if "CUDA" not in devices.stdout + devices.stderr:
        raise PipelineStop(f"llama-server ne voit aucun GPU CUDA :\n{devices.stdout}{devices.stderr}")
    return server.parent


def stop_with_parent() -> None:
    """Dans le processus du serveur, avant son lancement : s'il perd son parent (chaîne
    interrompue), Linux lui envoie SIGTERM, et il ne garde pas le GPU."""
    import ctypes
    import signal

    PR_SET_PDEATHSIG = 1
    ctypes.CDLL("libc.so.6", use_errno=True).prctl(PR_SET_PDEATHSIG, signal.SIGTERM)


def free_port() -> int:
    import socket

    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def validate_gguf(binaries: Path, model_args: list[str], label: str, examples: list[data.Example],
                  outputs_file: Path, log_file: Path) -> dict[str, Any]:
    """Score de validation du GGUF par `llama-server`, sur le GPU : même prompt brut, décodage
    glouton, même borne de jetons, même notation (`scoring.py`) que la validation par PyTorch.
    Le serveur n'écoute que 127.0.0.1, sur un port libre, le temps de la validation."""
    import urllib.request

    port = free_port()
    command = [str(binaries / "llama-server"), *model_args, "--host", "127.0.0.1", "--port", str(port),
               "-ngl", "99", "-c", str(MAX_LENGTH + HYPERPARAMETERS["jetons_max_generes"]), "-np", "1",
               "--no-webui", "--seed", str(TORCH_SEED)]
    log("$ " + " ".join(command))
    with open(log_file, "w", encoding="utf-8") as server_log:
        # `prctl` n'existe que sous Linux, où tourne l'entraînement ; Windows refuse `preexec_fn`
        # (recette Windows de l'extension, qui lance aussi ces tests).
        stop = stop_with_parent if sys.platform.startswith("linux") else None
        server = subprocess.Popen(command, stdout=server_log, stderr=subprocess.STDOUT, env=server_environment(),
                                  preexec_fn=stop)
    url = f"http://127.0.0.1:{port}"

    def post(body: dict[str, Any]) -> dict[str, Any]:
        request = urllib.request.Request(f"{url}/completion", data=json.dumps(body).encode("utf-8"),
                                         headers={"Content-Type": "application/json"})
        with urllib.request.urlopen(request, timeout=600) as response:
            return json.loads(response.read())

    try:
        for _ in range(600):
            if server.poll() is not None:
                raise PipelineStop(f"llama-server s'est arrêté ({server.returncode}) : voir {log_file}")
            try:
                with urllib.request.urlopen(f"{url}/health", timeout=5) as response:
                    if response.status == 200:
                        break
            except OSError:
                pass
            time.sleep(1)
        else:
            raise PipelineStop(f"llama-server ne répond pas : voir {log_file}")
        score, start = scoring.Score(), time.time()
        with open(outputs_file, "w", encoding="utf-8") as outputs:
            for rank, example in enumerate(examples, start=1):
                answer = post({"prompt": example.prompt, "n_predict": HYPERPARAMETERS["jetons_max_generes"],
                               "temperature": 0, "top_k": 1, "seed": TORCH_SEED,
                               "stop": [data.END_OF_ANSWER], "cache_prompt": True})
                text = answer["content"]
                expected = data.expected_answer(example)
                match = score.record(text, expected)
                outputs.write(json.dumps({"rang": rank, "juste": match, "attendu": expected, "sortie": text},
                                         ensure_ascii=False) + "\n")
                if rank % 25 == 0:
                    log(f"validation GGUF {rank}/{len(examples)} : {sum(score.correct.values())} justes")
    finally:
        server.terminate()
        try:
            server.wait(timeout=30)
        except subprocess.TimeoutExpired:
            server.kill()
    result = score.to_dict()
    result["duree_s"] = round(time.time() - start)
    result["outil"] = "llama-server (CUDA)"
    result["modele"] = label
    return result


# -- orchestration -------------------------------------------------------------------------


def read_report(path: Path) -> dict[str, Any]:
    if path.exists():
        return json.loads(path.read_text(encoding="utf-8"))
    return {}


def write_declarations(output: Path, declarations: dict[str, dict[str, str]] = DECLARATIONS) -> list[str]:
    """Écrit les déclarations de modèle à côté des GGUF (base + adaptateur, fusionné), sous un
    nom temporaire puis renommées, et rend leurs noms. Réécrites à l'identique à la reprise."""
    for name, declaration in declarations.items():
        partial = temporary(output / name)
        partial.write_text(json.dumps(declaration, ensure_ascii=False) + "\n", encoding="utf-8")
        partial.replace(output / name)
    return list(declarations)


def write_report(path: Path, report: dict[str, Any]) -> None:
    partial = path.with_suffix(".tmp")
    partial.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    partial.replace(path)


def versions() -> dict[str, str]:
    from importlib.metadata import version

    from importlib.metadata import PackageNotFoundError

    packages = ["torch", "transformers", "peft", "accelerate", "bitsandbytes", "safetensors", "numpy",
                "huggingface-hub", "unsloth", "unsloth-zoo", "flash-linear-attention", "fla-core", "triton",
                "nvidia-cuda-nvcc"]
    found = {}
    for p in packages:
        try:
            found[p] = version(p)
        except PackageNotFoundError:
            found[p] = None
    return found | {"python": platform.python_version()}


def repository_state() -> dict[str, Any]:
    """Commit du dépôt, et si l'arbre de travail diffère de ce commit (chemins modifiés)."""
    def git(*args: str) -> str:
        return subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True).stdout

    changed = [line[3:] for line in git("status", "--porcelain").splitlines() if line.strip()]
    return {
        "commit": git("rev-parse", "HEAD").strip(),
        "arbre_modifie": bool(changed),
        "chemins_modifies": changed,
        "uv_lock_sha256": sha256(HERE / "uv.lock"),
    }


def launch(args: argparse.Namespace) -> dict[str, Any]:
    output: Path = args.output.resolve()
    # Les options et le fichier externe avant tout coût : un nom refusé ou une ligne mal formée
    # n'attend ni le GPU, ni un téléchargement.
    outputs = check_options(args)
    # Le parent d'une séance continue, lu et comparé avant le GPU et avant le dossier de sortie.
    continuation = check_continuation(args)
    external = read_external(args.external, args.external_validation) if args.external else None
    civil = not args.external_only
    # Le compte de la séance (bornes comprises) est vérifié par `check_continuation`.
    count = continuation.count if continuation else (
        args.examples if args.examples is not None else TRAINING_COUNT)
    report_path = output / "rapport.json"
    if args.resume:
        # Lecture seule : un autre parent ou d'autres familles que l'entraînement commencé.
        check_resume_parent(read_report(report_path), continuation)
    # Le GPU ensuite : un refus ne laisse ni dossier rempli, ni téléchargement.
    memory, gpu_name = gpu_memory()
    check_gpu(memory)
    log(f"GPU : {gpu_name}, {memory / 1024**3:.1f} Gio")

    prepare_output(output, args.resume)
    work = output / "travail"
    work.mkdir(exist_ok=True)
    report = read_report(report_path) if args.resume else {}
    started_seed = report.get("graines", {}).get("torch")
    if started_seed is not None and started_seed != TORCH_SEED:
        # L'adaptateur déjà écrit a été entraîné avec l'autre graine (deux graines par nuit).
        raise PipelineStop(
            f"--reprendre avec la graine d'entraînement {TORCH_SEED}, l'entraînement commencé a "
            f"la graine {started_seed} : relancez avec --graine-torch {started_seed}"
        )

    hyperparameters = effective_hyperparameters(continuation)
    state = repository_state()
    started_lineage = report.get("lignee")
    report.update({
        "base": {"depot": BASE_REPO, "revision": BASE_REVISION, "licence": "Apache-2.0"},
        "llama_cpp": {"url": LLAMA_CPP_URL, "commit": LLAMA_CPP_COMMIT,
                      "llama_cpp_sys_2": LLAMA_CPP_SYS_VERSION},
        "depot_cassini": state,
        "graines": {"entrainement": TRAINING_SEED, "validation": VALIDATION_SEED, "torch": TORCH_SEED}
        | ({"cible": FOCUS_SEED} if continuation else {}),
        "hyperparametres": hyperparameters | ({"pas_max": args.max_steps} if args.max_steps else {})
        | ({"longs_d_abord": True} if args.long_first else {}),
        "gpu": {"nom": gpu_name, "memoire_totale_octets": memory},
        "versions": versions(),
        # Ajustement continu (5.54) : la lignée, que la séance suivante lit.
        "lignee": lineage(continuation, state["commit"], TORCH_SEED),
    })
    if (isinstance(started_lineage, list) and len(started_lineage) == len(report["lignee"])
            and isinstance(started_lineage[-1], dict) and started_lineage[-1].get("commit")):
        # En reprise, le maillon garde le commit de l'entraînement commencé.
        report["lignee"][-1]["commit"] = started_lineage[-1]["commit"]
    if continuation:
        # L'adaptateur de départ et son dossier ; un entraînement de zéro n'en a pas.
        report["depart"] = continuation.parent.adapter_sha256
        report["depuis"] = str(continuation.parent.folder)
    else:
        report.pop("depart", None)
        report.pop("depuis", None)
    report.pop("commit_cassini", None)
    check_resume(report, external, civil)
    write_report(report_path, report)

    # 3. jeux
    data_dir = work / "donnees"
    if not civil:
        sets = {}
    elif args.resume and (data_dir / "validation.jsonl").exists():
        sets = {name: path for name, (path, _, _, _)
                in generation_plan(data_dir, count, args.validation, continuation).items()}
    else:
        sets = generate(data_dir, count, args.validation, continuation)
    read = {name: data.read_jsonl(path) for name, path in sets.items()}
    if continuation:
        # Un jeu plus court que demandé arrête la chaîne avant l'entraînement.
        check_counts(generation_plan(data_dir, count, args.validation, continuation), read)
        recall_all, focus_all = read["rappel"], read["cible"]
        # `mix` ne touche pas au côté entraînement : il n'y sert qu'à retirer de la validation
        # tout prompt du rappel tiré ou du jeu ciblé.
        civil_training = recall_all + focus_all
    else:
        civil_training = read["entrainement"] if civil else []
    mixed = mix(civil_training, read["validation"] if civil else [], external)

    # 4. base (Unsloth avant transformers)
    if USE_UNSLOTH:
        log(f"Unsloth {import_unsloth()}")
    from huggingface_hub import snapshot_download
    from transformers import AutoTokenizer

    log(f"base {BASE_REPO} @ {BASE_REVISION}")
    base_dir = snapshot_download(BASE_REPO, revision=BASE_REVISION)
    tokenizer = AutoTokenizer.from_pretrained(base_dir)

    def length(e: data.Example) -> int:
        return len(encode(tokenizer, e)[0])

    session = None
    if continuation:
        # La part du rappel se compte sur le jeu final, une fois les exemples trop longs écartés.
        recall_kept, recall_too_long = data.drop_too_long(recall_all, length, MAX_LENGTH)
        focus_kept, focus_too_long = data.drop_too_long(focus_all, length, MAX_LENGTH)
        training, session = session_mix(recall_kept, focus_kept,
                                        session_counts(count, continuation.recall_share)[0])
        too_long = recall_too_long + focus_too_long
    else:
        training, too_long = data.drop_too_long(mixed["civil_training"], length, MAX_LENGTH)
    validation, too_long_validation = data.drop_too_long(mixed["civil_validation"], length, MAX_LENGTH)
    external_training, external_too_long = data.drop_too_long(mixed["external_training"], length, MAX_LENGTH)
    external_validation, external_too_long_validation = data.drop_too_long(
        mixed["external_validation"], length, MAX_LENGTH)
    training = training + external_training
    if not training:
        raise PipelineStop("aucun exemple d'entraînement ne tient dans la longueur maximale")
    # Un jeu de validation vide se voit avant l'entraînement, pas après.
    for scored, examples, label in ((civil, validation, "de validation"),
                                    (external is not None, external_validation, "de validation externe")):
        if scored and not examples:
            raise PipelineStop(f"aucun exemple {label} : le score ne peut pas être mesuré")
    lengths = sorted(length(e) for e in training)
    report["donnees"] = {
        "entrainement": len(training),
        **({
            "validation": len(validation),
            "ecartes_trop_longs": too_long,
            "ecartes_trop_longs_validation": too_long_validation,
            "validation_retires_car_vus_a_l_entrainement": mixed["civil_overlap"],
        } if civil else {}),
        "jetons_mediane": lengths[len(lengths) // 2],
        "jetons_max": lengths[-1],
        **({f"empreinte_{name}": sha256(path) for name, path in sets.items()} if civil else {}),
    }
    if continuation:
        report["donnees"]["seance"] = session | {
            "familles": continuation.families,
            "part_rappel_demandee": continuation.recall_share,
            "rappel": f"premiers exemples du jeu graine {TRAINING_SEED}, toutes familles",
            "cible": f"graine {FOCUS_SEED}, familles nommées seules",
        }
    if external is not None:
        # Empreintes et comptes seulement : aucun exemple externe n'entre au rapport.
        report["donnees"]["externe"] = {
            "nom": args.name,
            "melange": mode_label(civil),
            **external.info,
            "entrainement": len(external_training),
            "validation": len(external_validation),
            "ecartes_trop_longs": external_too_long,
            "ecartes_trop_longs_validation": external_too_long_validation,
            "validation_retires_car_vus_a_l_entrainement": mixed["external_overlap"],
        }
    write_report(report_path, report)
    log(f"{len(training)} exemples d'entraînement, {len(validation)} de validation, "
        f"{too_long} écartés car trop longs")
    if external is not None:
        log(f"dont {len(external_training)} exemples externes ; {len(external_validation)} de validation externe, "
            f"{external_too_long} écartés car trop longs")

    # 5 et 6. entraînement et validation
    adapter_dir = work / "adaptateur-hf"
    if not (args.resume and (adapter_dir / "adapter_model.safetensors").exists()):
        report["chemin_entrainement"] = training_path()
        report["noyaux"] = install_fast_kernels()
        log(f"noyaux de l'attention linéaire : {report['noyaux']}")
        result = train(base_dir, training, adapter_dir, args.max_steps, args.long_first, hyperparameters,
                       continuation.parent.adapter_dir if continuation else None,
                       continuation.parent.adapter_sha256 if continuation else None)
        model = result.pop("modele")
        report["entrainement"] = result
        write_report(report_path, report)
    else:
        model = None
    # La lignée porte l'empreinte de l'adaptateur produit : la séance suivante la vérifie.
    report["lignee"][-1]["adaptateur_sha256"] = sha256(adapter_dir / ADAPTER_WEIGHTS)
    write_report(report_path, report)
    # Le score civil (5.17) quand le jeu civil est mélangé, et le score externe à part : les deux
    # montrent si l'adaptateur de domaine oublie les commandes ordinaires.
    scores = ([("validation", validation, "validation-sorties.jsonl", "de validation")] if civil else []) + (
        [("validation_externe", external_validation, "validation-externe-sorties.jsonl", "de validation externe")]
        if external is not None else [])
    # Par défaut, le score se mesure plus bas, sur le GGUF, par llama.cpp sur le GPU (10 fois
    # plus vite) ; `--validation-pytorch` le mesure ici, sur l'adaptateur, comme avant.
    for key, examples, outputs_file, label in scores if args.pytorch_validation else []:
        if args.resume and key in report:
            continue
        if not examples:
            raise PipelineStop(f"aucun exemple {label} : le score ne peut pas être mesuré")
        if model is None:
            from peft import PeftModel

            model = PeftModel.from_pretrained(load_model(base_dir), adapter_dir)
        log(f"{label.removeprefix('de ')} sur {len(examples)} exemples")
        report[key] = validate(model, tokenizer, examples, output / outputs_file)
        write_report(report_path, report)
        log(f"score {label} : {report[key]['justes']}/{report[key]['total']}")
    del model
    import torch

    torch.cuda.empty_cache()

    # 7 et 8. GGUF
    llama = prepare_llama_cpp()
    convert(llama, base_dir, adapter_dir, output, work, args.resume, outputs)
    report["sorties"] = {
        name: {"fichier": name, "octets": (output / name).stat().st_size, "sha256": sha256(output / name)}
        for name in outputs.files
    }
    report["sorties"][BASE_NAME]["role"] = "base, sur laquelle se charge l'adaptateur"
    report["sorties"][outputs.adapter]["role"] = (
        "adaptateur LoRA de composition" if external is None else "adaptateur LoRA de domaine")
    if outputs.merged:
        report["sorties"][outputs.merged]["role"] = "base et adaptateur fusionnés, quantifiés"
    # Les déclarations : le dossier de sortie, copié dans un dossier de modèles, est prêt.
    report["declarations"] = write_declarations(output, outputs.declarations)
    write_report(report_path, report)

    # 6 bis. score de validation sur le GGUF : le modèle fusionné pour le modèle civil (celui qui
    # est installé), la base et l'adaptateur pour un adaptateur de domaine.
    if not args.pytorch_validation:
        model_args = (["-m", str(output / outputs.merged)] if outputs.merged
                      else ["-m", str(output / BASE_NAME), "--lora", str(output / outputs.adapter)])
        label_gguf = outputs.merged or f"{BASE_NAME} + {outputs.adapter}"
        binaries = None
        for key, examples, outputs_file, label in scores:
            if args.resume and key in report:
                continue
            binaries = binaries or prepare_llama_cpp_gpu(llama)
            log(f"{label.removeprefix('de ')} sur {len(examples)} exemples, par llama.cpp sur le GPU")
            report[key] = validate_gguf(binaries, model_args, label_gguf, examples, output / outputs_file,
                                        work / f"llama-server-{key}.log")
            write_report(report_path, report)
            log(f"score {label} : {report[key]['justes']}/{report[key]['total']}")

    # 9. essai sur une demande qui attend un plan d'appels : par les deux chemins pour le modèle
    # civil, par la base et l'adaptateur seuls pour un adaptateur de domaine (pas de fusion).
    if external is None:
        example, private = first_calls_example(validation), False
    else:
        example, private = trial_example(external_validation, validation)
    report["essai"] = {
        "base_et_adaptateur": try_model(
            llama, ["-m", str(output / BASE_NAME), "--lora", str(output / outputs.adapter)],
            example, work, "base GGUF + adaptateur GGUF", private),
    }
    if outputs.merged:
        report["essai"]["fusionne"] = try_model(
            llama, ["-m", str(output / outputs.merged)], example, work, "modèle fusionné", private)
    write_report(report_path, report)
    unreadable = [name for name, trial in report["essai"].items() if not trial["se_lit_comme_un_plan"]]
    if unreadable and args.max_steps is not None:
        # Essai rapide (`--pas-max`) : un adaptateur à peine entraîné ne sait pas encore répondre ;
        # la chaîne a fait son travail, l'essai est seulement consigné.
        log(f"essai rapide : pas de plan JSON lisible ({', '.join(unreadable)}), attendu après si peu de pas")
    elif unreadable:
        raise PipelineStop(f"pas de plan JSON lisible ({', '.join(unreadable)}) : voir rapport.json")
    return report


def main(argv: list[str] | None = None) -> int:
    global FAST_KERNELS, USE_UNSLOTH, TORCH_SEED
    parser = argparse.ArgumentParser(description="Ré-entraîne le modèle de composition de Cassini.")
    parser.add_argument("--sortie", dest="output", type=Path, required=True,
                        help="dossier de sortie, vide et hors du dépôt")
    parser.add_argument("--reprendre", dest="resume", action="store_true",
                        help="finir un entraînement commencé dans --sortie")
    parser.add_argument("--exemples", dest="examples", type=int, default=None,
                        help=f"exemples d'entraînement ({TRAINING_COUNT} de zéro, {CONTINUOUS_COUNT} par séance continue)")
    parser.add_argument("--validation", dest="validation", type=int, default=VALIDATION_COUNT,
                        help="exemples de validation engendrés")
    parser.add_argument("--pas-max", dest="max_steps", type=int, default=None,
                        help="borne de pas (essai rapide de la chaîne)")
    parser.add_argument("--validation-pytorch", dest="pytorch_validation", action="store_true",
                        help="score de validation par PyTorch sur l'adaptateur, au lieu de llama.cpp sur le GGUF")
    parser.add_argument("--longs-d-abord", dest="long_first", action="store_true",
                        help="mesure de la mémoire : les exemples les plus longs d'abord (avec --pas-max)")
    parser.add_argument("--sans-unsloth", dest="without_unsloth", action="store_true",
                        help="charge et entraîne par transformers et peft seuls, sans Unsloth")
    parser.add_argument("--noyaux-reference", dest="reference_kernels", action="store_true",
                        help="implémentations PyTorch de référence de l'attention linéaire, sans noyaux rapides")
    parser.add_argument("--exemples-externes", dest="external", type=Path, default=None,
                        help="fichier d'exemples {prompt, completion} : entraîne un adaptateur de domaine")
    parser.add_argument("--nom", dest="name", default=None,
                        help="nom de l'adaptateur de domaine et de sa déclaration (avec --exemples-externes)")
    parser.add_argument("--externe-seul", dest="external_only", action="store_true",
                        help="n'entraîne que sur le fichier externe, sans le jeu civil engendré")
    parser.add_argument("--graine-torch", dest="torch_seed", type=int, default=TORCH_SEED,
                        help="graine de l'entraînement seul ; le jeu engendré ne change pas")
    parser.add_argument("--validation-externe", dest="external_validation", type=Path, default=None,
                        help="second fichier pour le score externe, au lieu d'une part du premier")
    parser.add_argument("--depuis", dest="parent", type=Path, default=None,
                        help="dossier de sortie d'un entraînement précédent : la séance repart de son adaptateur")
    parser.add_argument("--familles", dest="families", default=None,
                        help="familles du jeu ciblé, séparées par des virgules (avec --depuis)")
    parser.add_argument("--part-rappel", dest="recall_share", type=float, default=None,
                        help=f"part du rappel tiré de toutes les familles (avec --depuis ; {RECALL_SHARE} par défaut, "
                             f"{MIN_RECALL_SHARE} au moins)")
    args = parser.parse_args(argv)
    TORCH_SEED = args.torch_seed
    FAST_KERNELS = not args.reference_kernels
    USE_UNSLOTH = not args.without_unsloth
    start = time.time()
    try:
        report = launch(args)
    except PipelineStop as stop:
        print(f"arrêt : {stop}", file=sys.stderr)
        return 1
    report["duree_totale_s"] = round(time.time() - start)
    write_report(args.output.resolve() / "rapport.json", report)
    log(f"terminé en {report['duree_totale_s'] / 60:.0f} min ; rapport : {args.output / 'rapport.json'}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
