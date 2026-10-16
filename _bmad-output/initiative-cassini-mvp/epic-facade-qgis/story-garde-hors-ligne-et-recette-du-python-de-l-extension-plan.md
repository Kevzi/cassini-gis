---
title: "Garde hors ligne et recette du Python de l'extension"
type: 'feature'
ticket: '3'
created: '2026-10-16'
status: 'built'
baseline_revision: '99ce30fbf39ade4665f9debb5419956f4d6962cd'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/CLAUDE.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la recette (`.github/workflows/recette.yml`) ne contrôle pas le Python de `qgis-plugin/` : rien n'empêche un `import urllib` ou `import PyQt5` d'y entrer, ni une extension trop lourde, ni un noyau qui dépendrait de QGIS (AD-1, AD-14, AD-18).

**Approach:** ajouter un troisième job `extension` à la recette, fondé sur un script de garde statique (imports interdits, taille, `metadata.txt`) et sur les tests existants sans QGIS ; le noyau est prouvé sans QGIS par son propre job. Décision du propriétaire (2026-10-16) : option A, sans QGIS ni doublures de `qgis.core` ; tests unitaires en matrice ubuntu + windows. La preuve dynamique (zéro requête `QgsNetworkAccessManager`, chargement réel sur QGIS 3.44 et 4.x) est une dette connue, hors de cette story ; aucun faux test à doublures ne la simule.

## Boundaries & Constraints

**Always:** Python de recette en bibliothèque standard seule (`python -I`) ; garde par analyse `ast`, pas par expression régulière ; messages d'échec en français nommant fichier et ligne ; extension sans binaire, paquet < 25 Mo.

**Never:** aucun garde-fou posé à l'exécution dans QGIS (AD-14) ; aucune logique métier ajoutée à l'extension ; aucune dépendance Rust ou npm nouvelle ; ne pas modifier le noyau.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Extension propre | imports actuels | code 0 | — |
| Import interdit | `import urllib.request` ou `from http import client` | code ≠ 0, fichier:ligne | message nommant le module |
| PyQt direct | `import PyQt5.QtCore` / `PyQt6` | code ≠ 0 | message citant `qgis.PyQt` |
| Import tardif | import interdit dans une fonction | refusé aussi | idem |
| Paquet trop gros | dossier > 25 Mo | code ≠ 0 | taille affichée |
| Versions | `metadata.txt` ≠ 3.44 / 4.99 | code ≠ 0 | clé fautive nommée |

</frozen-after-approval>

## Code Map

- `.github/workflows/recette.yml` -- le commentaire « Point d'accroche réservé à l'epic 9 » : y ajouter le job `extension`.
- `qgis-plugin/cassini_qgis/*.py` -- seul `plugin.py` importe `qgis.*` (via `qgis.PyQt`, conforme) ; aucun import réseau aujourd'hui. À ne pas modifier.
- `qgis-plugin/cassini_qgis/metadata.txt` -- versions 3.44 / 4.99 déjà posées, `supportsQt6=True`.
- `qgis-plugin/tests/test_tracer.py` -- tests sans QGIS (`python -I -m unittest discover -s qgis-plugin/tests`) ; à lancer dans le job.
- `app/scripts/check-offline.mjs` -- modèle d'esprit de la garde existante (commentaires de motif, exceptions nommées) ; ne pas y toucher.
- `tests/epreuve-modele-local/check_*.py` -- précédent de scripts de contrôle Python du dépôt.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/tests/check_extension.py` -- garde `ast` : modules refusés `urllib`, `requests`, `socket`, `http`, `PyQt5`, `PyQt6` (racine du nom importé, imports tardifs compris), taille du paquet < 25 Mo, `metadata.txt` 3.44 / 4.99 -- AD-14, AD-18.
- [ ] `qgis-plugin/tests/test_check_extension.py` -- tests de la garde sur des sources temporaires : un cas propre, un par module refusé, import tardif, `from x import y`, taille, versions -- chaque cas de la matrice.
- [ ] `.github/workflows/recette.yml` -- job `extension` (matrice ubuntu-latest + windows-latest, `actions/setup-python`) : garde, puis `unittest discover` ; remplace le commentaire réservé -- critère « la recette échoue ».
- [ ] `qgis-plugin/README.md` -- section Tests : la commande de la garde -- documentation.

**Acceptance Criteria:**
- Given l'extension actuelle, when la garde tourne, then code 0.
- Given un `import urllib` ajouté dans l'extension, when la garde tourne, then code ≠ 0 avec fichier et ligne ; retiré, code 0.
- Given une machine sans QGIS, when `cargo test -p cassini-core -p cassini --locked` tourne, then les tests passent (job `noyau`, déjà sans QGIS).

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 0 high, 1 medium, 4 low, 0 false.

| Verdict | Route | Constat et preuve |
|---|---|---|
| medium | defer | La garde ne voit pas `qgis.PyQt.QtNetwork`, `ftplib`, `smtplib`, `ssl`, `asyncio` ni `importlib`/`__import__`. Réel, mais AD-14 et le plan figés ne listent que quatre modules ; l'étendre est une décision de périmètre. |
| low | défer | `ai/Cargo.toml` n'est pas couvert par `test_noyau_sans_qgis` (noyau, CLAUDE.md §10) ; le plan nomme `core/` et `cli/`. Réel, hors plan. |
| low | rejeté | `ValueError` d'`ast.parse` avant Python 3.12 sur octet nul : CI en 3.12, correctif = branche en plus. |
| low | rejeté | Analyse de `metadata.txt` naïve (commentaire en fin de ligne, doublon) : sans effet sur le fichier réel. |
| low | rejeté | `test_noyau_sans_qgis` cherche le mot dans tout le `Cargo.toml` : faux positif hypothétique, échec bruyant. |
| low | rejeté | La taille compte `__pycache__` et le dossier non zippé : ne fait que surestimer, ne laisse rien passer. |

## Design Notes

La garde est statique par construction : elle lit les sources, n'exécute rien de l'extension. Le noyau « sans QGIS » est déjà prouvé par le job `noyau` (ubuntu, aucune installation de QGIS) ; le plan n'ajoute qu'un test refusant toute mention de `qgis`/`PyQt` dans les `Cargo.toml` de `core/` et `cli/`.

## Verification

**Commands:**
- `python -I qgis-plugin/tests/check_extension.py` -- expected: code 0
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: tous les tests passent
- `cargo test -p cassini-core -p cassini --locked` -- expected: passent
