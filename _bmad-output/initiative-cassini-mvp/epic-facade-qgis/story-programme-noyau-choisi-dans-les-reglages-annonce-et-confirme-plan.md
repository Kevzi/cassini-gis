---
title: 'Programme noyau choisi dans les reglages, annonce et confirme'
type: 'feature'
ticket: '15'
created: '2026-11-28'
status: 'built'
baseline_revision: '67062b55a3644a061d43672c69f6646bcaccf7e5'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/change-commandes-d-un-module-exterieur/change-commandes-d-un-module-exterieur.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** `cassini/core_path` (AD-17) désigne n'importe quel programme noyau, mais aucun champ ne le montre dans le panneau, le nom et la version du programme branché ne s'affichent pas, et rien ne prévient l'utilisateur qu'un programme dérivé est lancé.

**Approach:** Rendre le réglage visible dans les réglages du panneau (même ordre de recherche), afficher en permanence nom et version annoncés par `session.hello`, avec la mention « programme dérivé de Cassini » et la limite d'AD-14 quand le programme n'est pas officiel, et demander confirmation au premier branchement d'un programme non officiel en retenant son empreinte SHA-256 (hashlib, côté extension).

## Boundaries & Constraints

**Always:** L'extension ne connaît aucune commande particulière. L'ordre de recherche d'`locator.locate` ne change pas. Un refus sort de `_launch` avant `QProcess` : le programme n'est pas lancé. Qt via `qgis.PyQt`, aucun import réseau (garde `check_extension.py`). Textes fr/en dans `texts.py`.

**Décision (2026-11-28, propriétaire) :** avant lancement, « officiel » = le programme trouvé à l'emplacement d'installation de Cassini ; tout programme venu du réglage ou de `CASSINI_CORE` demande une confirmation une fois par empreinte (le programme officiel aussi, s'il est choisi à la main). La mention « programme dérivé » se décide ensuite sur `program_name` du hello (≠ `cassini`). Pas d'empreinte officielle dans `installation.json`.

**Never:** Aucun changement du noyau Rust ni du protocole. Pas de téléchargement. Pas de logique métier dans l'extension au-delà de la décision de confiance.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Programme officiel | hello `program_name` = `cassini` | Nom et version affichés, aucune mention dérivée | — |
| Premier branchement dérivé | programme non officiel, empreinte inconnue | Confirmation demandée avant lancement ; accord → empreinte retenue, lancement | — |
| Second branchement, même empreinte | empreinte déjà acceptée | Aucune question | — |
| Empreinte changée | binaire remplacé | Confirmation redemandée | — |
| Refus | l'utilisateur refuse | Programme non lancé, bannière explicative | Aucune empreinte retenue |
| Dérivé branché | hello `program_name` ≠ `cassini` | Nom, version, « programme dérivé de Cassini », phrase sur AD-14 | — |
| Fichier illisible | empreinte incalculable | Traité comme refus | Bannière d'erreur |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/locator.py` -- `SETTING_KEY`, `locate()` rend `(chemin, origine)` ; ne pas changer l'ordre.
- `qgis-plugin/cassini_qgis/plugin.py` -- `_launch` (≈204-241) : insérer la décision de confiance entre `locate` et `QProcess` ; `choose_core` (≈158) écrit le réglage ; `_on_event` (≈263).
- `qgis-plugin/cassini_qgis/client.py` -- ≈525, le dict `ready` du hello ne relaie pas encore `program_name` ni `program_version` (déjà envoyés par le noyau, `core/src/serve.rs:1029`).
- `qgis-plugin/cassini_qgis/panel.py` -- `settings_zone` (≈516) : ajouter le champ du programme noyau ; `set_ready` (≈967) et `install_info` (≈363) : afficher nom, version, mention ; libellés rafraîchis ≈1215.
- `qgis-plugin/cassini_qgis/texts.py` -- libellés fr/en.
- `qgis-plugin/tests/test_installation.py`, `test_panel_logic.py` -- modèles de tests unittest sans QGIS ; lancement : `python -I -m unittest discover -s qgis-plugin/tests`.
- `qgis-plugin/tests/check_extension.py` -- garde AST (hashlib permis).
- `tests/programme-exemple` -- programme dérivé de 2.14, pour la recette QGIS.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/cassini_qgis/trust.py` -- module pur : empreinte SHA-256 d'un fichier, décision `needs_confirmation(path, origine, accepted)`, mémorisation des empreintes acceptées -- logique testable hors QGIS
- [ ] `qgis-plugin/cassini_qgis/plugin.py` -- appeler `trust` avant `QProcess`, boîte de confirmation, refus = bannière sans lancement -- exigence du ticket
- [ ] `qgis-plugin/cassini_qgis/client.py` -- relayer `program_name` et `program_version` dans `ready`
- [ ] `qgis-plugin/cassini_qgis/panel.py`, `texts.py` -- champ du réglage avec explication, affichage permanent nom/version, mention dérivée et limite d'AD-14
- [ ] `qgis-plugin/tests/test_trust.py`, `test_panel_logic.py` -- cas de la matrice

**Acceptance Criteria:**
- Given le champ des réglages, when l'utilisateur choisit un programme, then `cassini/core_path` est écrit.
- Given un refus de confirmation, when `_launch` s'exécute, then aucun processus n'est créé.
- Given la recette QGIS avec le programme d'exemple de 2.14 puis le programme officiel, then l'affichage diffère comme décrit.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 3 medium, 3 low, 1 maybe-false, 2 false.

| Constat | Verdict | Route | Preuve / action |
|---|---|---|---|
| Aucun test de `_render_program` (lignes officiel / dérivé de la matrice) | medium | patch | `ProgramLineTest` ajouté dans `test_tracer.py` |
| Texte de la boîte en AutoText, chemin venu de l'extérieur | medium | patch | `setTextFormat(PlainText)` dans `_confirm_program` |
| `OFFICIAL_ORIGIN` dupliqué sans test de lien avec `locator` | medium | patch | test `test_official_origin_is_the_one_locator_produces` |
| Panneau garde l'info d'installation après un refus | low | patch | `_forget_install_info` sur les deux refus |
| `core_path` / `clear_core` non testés | low | rejeté | lecture correcte, `choose_core` inchangé ; correctif = tests seuls |
| Fenêtre entre empreinte et lancement | medium | defer | exige un lancement depuis une copie vérifiée ; hors ticket |
| Boîte modale rouverte par un second `_launch` | maybe-false | rejeté (low) | la sonde est arrêtée avant `_trusted` et la boîte est modale ; correctif ajoute un état |
| Aide ne cite pas PATH | low | rejeté | « un autre programme » le couvre |
| Casse de `program_name` | false | rejeté | le noyau compare sans casse (`registry.rs:587`) |

## Verification

**Commands:**
- `python -I qgis-plugin/tests/check_extension.py` -- expected: succès
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: tous verts

**Manual checks (if no CLI):**
- Recette QGIS : brancher le programme d'exemple (confirmation, mention dérivée) puis le programme officiel (rien de plus).
