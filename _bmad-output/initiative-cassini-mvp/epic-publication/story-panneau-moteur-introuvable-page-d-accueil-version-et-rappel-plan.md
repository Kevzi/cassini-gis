---
title: "Panneau : moteur introuvable, page d'accueil, version et rappel de mise à jour"
type: 'feature'
ticket: '8'
created: '2026-11-13'
status: 'built'
baseline_revision: '3dc046082decc279a713999219162924fbe15823'
route: 'full'
route_source: 'auto'
risk: 'low'
review: 'quick'
review_source: 'pinned'
lenses_ran: [quick]
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-publication/epic-publication.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** sans noyau, le panneau dit seulement « introuvable » et laisse l'utilisateur sans chemin ; une fois le paquet installé, il faut redémarrer QGIS ; la version installée et l'âge de l'installation ne sont visibles nulle part.

**Approach:** le bandeau « introuvable » dit « Cassini a besoin de son moteur pour fonctionner » et gagne un bouton qui ouvre la page d'accueil d'AD-17 dans le navigateur ; l'extension resonde le noyau toutes les 3 s tant que le bandeau est affiché et se lance dès qu'elle le trouve ; les Réglages montrent la version lue dans `installation.json` ; passé trois mois depuis `installe_le`, une note invite à lancer « Mettre à jour Cassini ».

## Boundaries & Constraints

**Always:** aucune logique métier ni réseau dans l'extension : l'ouverture de la page passe par `QDesktopServices.openUrl` (jamais `webbrowser`, interdit par la garde), le rappel est un calcul de date locale ; Qt seulement par `qgis.PyQt` ; adresse fixe `https://kevzi.github.io/cassini-gis` (AD-17) ; `installation.json` est lu à côté du noyau trouvé, format de 7.1 : `{"version": "0.1.0", "installe_le": "2026-11-13T09:30:00Z"}` ; textes en français et en anglais ; « Choisir le noyau » reste proposé.

**Never:** rien n'est téléchargé ni lancé (« Mettre à jour Cassini » est lancé à la main par l'utilisateur) ; pas de bouton de mise à jour ; pas de rappel quand `installation.json` manque, est illisible ou sa date invalide (noyau venu du PATH ou d'une variable) ; le noyau n'est pas modifié ; le panneau ne fait pas d'accès disque : le plugin lit le fichier et le passe au panneau.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Noyau absent | locator renvoie None | bandeau « Cassini a besoin de son moteur pour fonctionner », boutons « Ouvrir la page d'accueil » et « Choisir le noyau » | — |
| Installation faite QGIS ouvert | noyau apparaît pendant le bandeau | au tick suivant (≤ 3 s) le noyau se lance, bandeau « Démarrage » | — |
| Version connue | `installation.json` valide à côté du noyau | Réglages : « Version installée : 0.1.0, le 2026-11-13 » | — |
| Installation ancienne | `installe_le` ≥ 3 mois avant aujourd'hui | note « … Lancez « Mettre à jour Cassini » » visible | — |
| Installation récente | moins de 3 mois | pas de note | — |
| Fichier absent/illisible/date invalide | pas de JSON utilisable | ni version ni rappel | aucune exception levée |

</frozen-after-approval>

## Code Map

- `qgis-plugin/cassini_qgis/locator.py` -- `locate()` rend (chemin, origine) ; modèle de module pur, sans Qt
- `qgis-plugin/cassini_qgis/plugin.py:171-185` -- `_launch()` : bandeau `not_found` ; `choose_core`/`relaunch` : modèle des actions du panneau ; `_refresh()` relance si `project_path` est None
- `qgis-plugin/cassini_qgis/panel.py:136-148` BANNERS ; `:511-522` zone du bandeau ; `:1155-1168` rendu du bandeau ; `:2014` `_banner_action` ; `:472-495` fenêtre Réglages ; `:2150` `_open_settings`
- `qgis-plugin/cassini_qgis/texts.py:45` / `:360` -- `not_found` fr/en, `choose_core`
- `qgis-plugin/tests/check_extension.py` -- garde hors ligne (webbrowser, QtNetwork interdits) ; `tests/test_panel_logic.py` -- modèle des tests sans QGIS
- `installer/cassini-en-ligne.iss:216` -- écrit `installation.json` dans `{app}\bin` (à côté de `cassini.exe`) ; ne pas modifier
- Épine : `installation.json` n'est lu que si le noyau trouvé est dans le dossier d'installation ; même lecture pour tout noyau (le fichier absent suffit à taire).

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/cassini_qgis/installation.py` -- NOUVEAU, pur : `HOME_URL`, `read(core_path)` -> `{version, installed_at}` ou None, `reminder_due(installed_at, now)` (3 mois calendaires, UTC) -- testable sans QGIS
- [ ] `qgis-plugin/cassini_qgis/texts.py` -- `not_found` réécrit ; `open_home`, `install_version`, `update_reminder` fr/en
- [ ] `qgis-plugin/cassini_qgis/panel.py` -- bouton « Ouvrir la page d'accueil » visible sous `not_found` ; ligne de version dans les Réglages ; note de rappel ; `set_install_info(info)` ; l'action `open_home` passe par `_actions`
- [ ] `qgis-plugin/cassini_qgis/plugin.py` -- `open_home()` (QDesktopServices.openUrl) ; `QTimer` 3 s actif tant que `not_found` ; `_launch` lit `installation.json` et le passe au panneau
- [ ] `qgis-plugin/tests/test_installation.py` -- NOUVEAU : cas de la matrice (valide, ancien, récent, absent, illisible, date invalide, bascule de mois), + texts fr/en présents
- [ ] `qgis-plugin/tests/test_panel_logic.py` (ou test existant) -- le bandeau `not_found` ne porte plus « introuvable » seul : clés fr/en alignées

**Acceptance Criteria:**
- Given un poste sans noyau, when QGIS s'ouvre avec un projet, then le bandeau dit « Cassini a besoin de son moteur pour fonctionner » et le bouton ouvre `https://kevzi.github.io/cassini-gis` dans le navigateur.
- Given ce bandeau affiché, when le noyau est posé dans l'emplacement d'installation, then le panneau le lance sans redémarrer QGIS.
- Given un `installation.json` daté de plus de trois mois, when le panneau est prêt, then la note de rappel paraît ; avec une date récente ou sans fichier, elle ne paraît pas.
- Given l'extension, when `check_extension.py` tourne, then il reste vert.

## Implementation Notes

## Plan Change Log

## Review Triage Log

## Design Notes

Le rappel se calcule à l'ouverture du panneau avec l'heure UTC du poste : c'est une aide, pas un contrôle. Trois mois = `installe_le` + 3 mois calendaires (jour ramené à la fin du mois si besoin). Le sondage de 3 s ne tourne que sous le bandeau `not_found` et s'arrête à la première trouvaille.

## Verification

**Commands:**
- `python -m unittest discover -s qgis-plugin/tests` -- expected: tous verts, dont `test_installation`
- `python qgis-plugin/tests/check_extension.py qgis-plugin/cassini_qgis` -- expected: garde verte

**Manual checks (if no CLI):**
- Dans QGIS (déployer extension et noyau soi-même) : sans noyau, message et bouton ; pose du noyau, le panneau le trouve ; `installe_le` vieilli à la main, la note paraît.

## Review Triage Log (ajout)

Revue quick, 5 remarques :
- medium, patch (corrigé) : la sonde de 3 s continuait sous le bandeau `no_project` ; `_refresh` l'arrête désormais.
- medium, patch (corrigé) : version et rappel restaient affichés quand le noyau disparaissait ; la branche `not_found` appelle `set_install_info(None)`. Test ajouté.
- false : le réglage lu par `_probe_tick` n'est pas couvert par le stub `QSettings` ; même appel que `_launch`, rien ne casse.
- false : tâche « test_panel_logic » non faite à la lettre ; les mêmes vérifications fr/en sont dans `test_installation.TextsTest`.
- false : `reminder_due` avec `now` aware : aucun appelant ne le fait, le plugin passe un UTC naïf.
