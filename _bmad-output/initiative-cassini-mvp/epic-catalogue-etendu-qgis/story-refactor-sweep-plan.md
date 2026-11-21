---
title: 'Refactor sweep'
type: 'chore'
ticket: '18'
created: '2026-11-21'
status: 'built'
baseline_revision: '623cb1d229d00c27e3f513b4c9cec5b509973bb4'
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

**Problem:** Nettoyage de fin d'epic 10. Recensement des entrées reportées de l'epic : une seule est un nettoyage pur et certain, l'avertissement clippy `manual_is_ascii_check` à `training/src/lib.rs:1580` (`matches!(c, '0'..='9')`). L'octet 0x08 du README est déjà corrigé (0 occurrence) ; `qgis_version` est lu (`plugin.py:232`), donc pas du code mort ; la ligne de licences de CLAUDE.md est un fichier de contexte d'agent, reportée. Le reste des entrées reportées change un comportement ou attend une décision humaine : hors balayage.

**Approach:** Remplacer le motif par `c.is_ascii_digit()`, puis vérifier fmt, clippy sans avertissement et tests, noyau et extension.

</frozen-after-approval>

## Implementation Notes

Plan oneshot : un seul changement mécanique certain. Vérification demandée par le ticket : tests du noyau et de l'extension verts, fmt et clippy propres, recette hors ligne verte.

## Review Triage Log

Relecture rapide : aucun constat.
