---
title: 'Refactor sweep'
type: 'refactor'
ticket: '13'
created: '2026-10-16'
status: done
baseline_revision: '793c237f59c3a822cd043373bae2b93c8dc0272f'
route: 'oneshot'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Les entrées 9.1 à 9.12 ont reporté des constats dans `deferred-work.md` ; certains sont des garde-fous faciles à poser, d'autres sont périmés ou demandent QGIS.

**Approach:** Poser les trois garde-fous qui se testent sans QGIS (garde réseau étendue, test « noyau sans QGIS » couvrant `/ai`, masquage des chemins dans les messages) et écrire une décision pour chaque autre constat de l'epic.

Décisions de périmètre (2026-10-16, au mieux du projet) :
- Garde réseau : ajouter `ftplib`, `smtplib`, `ssl`, `asyncio`, `telnetlib`, `xmlrpc`, `QtNetwork`, `QtWebSockets`, et refuser `__import__(...)` et `importlib.import_module(...)`.
- `/ai` : le test couvre `ai/Cargo.toml` s'il existe (le crate n'est pas encore créé).
- `redact` : masque tout chemin absolu inclus dans une chaîne, pas seulement une chaîne entière.
- Shapefile sans `.prj` : constat périmé, `_resolve_crs` refuse déjà par `layer.unknown_crs` ou accepte le paramètre `crs`. Clos.
- Rejeu d'`analysis.buffer` : traité par 9.14, report maintenu.
- Bouton d'arrêt des gestes : report maintenu, relève de l'epic agent (5).
- `FieldCombo` en vrai panneau Qt et `NUMPOINTS` : demandent QGIS en interactif, reports maintenus avec leur décision écrite.

</frozen-after-approval>

## Implementation Notes

Oneshot : trois petites retouches de Python testables sans QGIS et cinq décisions écrites, environ 80 lignes.

## Plan Change Log

## Review Triage Log

- medium, patch : `redact` perdait le texte d'une phrase commençant par un chemin ; la branche « chemin seul » ne s'applique plus s'il y a « : » ou « , » suivi d'un blanc.
- medium, patch : chemin collé à `:`, `=`, `,`, `;` non masqué ; ajoutés au lookbehind. Chemin avec espace : masqué en entier quand il est seul dans la chaîne.
- low, patch : socketserver, selectors, webbrowser, QtWebKit, QtWebChannel ajoutés à la garde.
- low, defer écarté : contournements dynamiques (`getattr`, alias) et `from qgis.PyQt import *` hors de portée d'une garde statique ; limite notée dans la résolution.
- low, rejeté : saut silencieux de `ai`, accepté par le plan.

## Verification

**Commands:**
- `python -I qgis-plugin/tests/check_extension.py` -- expected: « Garde de l'extension : propre. »
- `python -m unittest discover -s qgis-plugin/tests` -- expected: tous les tests passent
