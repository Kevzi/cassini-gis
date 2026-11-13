"""Refuse l'étiquette de la variable d'environnement TAG si elle diffère de la version Cargo."""

import os
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import manifeste  # noqa: E402


def main() -> int:
    try:
        manifeste.verifier_etiquette(
            os.environ.get("TAG", ""), manifeste.version_cargo(manifeste.RACINE / "Cargo.toml")
        )
    except manifeste.ManifesteErreur as e:
        print(f"release refusée : {e}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
