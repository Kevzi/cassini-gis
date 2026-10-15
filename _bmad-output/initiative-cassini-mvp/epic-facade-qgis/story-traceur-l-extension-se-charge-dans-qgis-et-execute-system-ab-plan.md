---
title: "Traceur : l'extension se charge dans QGIS et execute system.about par le noyau"
type: 'feature'
ticket: '1'
created: '2026-10-15'
status: 'built'
baseline_revision: '3d36aeb1236d9d3e287091695b6ee4e8996a4329'
route: 'full'
route_source: 'auto'
risk: 'high'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context: ['{project-root}/_bmad-output/initiative-cassini-mvp/architecture-noyau-et-facade-qgis/architecture-noyau-et-facade-qgis.md', '{project-root}/docs/canal-local.md']
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** Rien ne prouve encore que QGIS 3.44 charge une extension Cassini, lance le noyau en processus enfant et dialogue avec lui. Tout le reste de l'epic repose sur ce chemin.

**Approach:** Un traceur minimal, jetable côté interface : paquet `cassini_qgis` qui localise le noyau (AD-17), lance `cassini serve` par `QProcess`, envoie `session.hello` puis `system.about`, et affiche les deux versions dans un panneau ancré.

## Boundaries & Constraints

**Always:** Qt uniquement via `qgis.PyQt` (AD-18). `qgisMinimumVersion=3.44`, `qgisMaximumVersion=4.99`. JSON-RPC 2.0, un objet JSON par ligne sur stdin/stdout du processus enfant ; stderr lu et ignoré (débogage seulement). Aucune logique métier en Python (AD-22). Fermer QGIS ferme stdin du noyau, qui s'arrête seul. Paquet sans binaire, sous 25 Mo.

**Décisions :** (1) l'emplacement d'installation du noyau est `%LOCALAPPDATA%\Cassini\bin\cassini.exe` sous Windows et `~/.local/share/cassini/bin/cassini` sous Linux. (2) Sans projet enregistré ouvert, le noyau n'est pas lancé : le panneau demande d'ouvrir ou d'enregistrer un projet, et le lancement suit l'ouverture d'un projet.

**Never:** Aucun téléchargement, socket, port, `urllib`, `requests`, `socket`, `http`, `PyQt5`, `PyQt6`. Pas de boucle d'ordres, d'annulation ni de lecture non bloquante durcie (story 2), pas de garde hors ligne ni de recette Python (story 3), pas de commande `layer.add`. Le panneau n'est pas une interface définitive.

## I/O & Edge-Case Matrix

| Scenario | Input / State | Expected Output / Behavior | Error Handling |
|----------|--------------|---------------------------|----------------|
| Nominal | Noyau trouvé, projet ouvert | Panneau : version du noyau et du protocole | No error expected |
| Aucun projet enregistré | Projet vide ou non enregistré | Noyau non lancé, panneau demande d'ouvrir ou d'enregistrer un projet | Lancement à l'ouverture d'un projet |
| Noyau introuvable | Aucun des quatre emplacements | Panneau explique quoi installer, propose de choisir le fichier à la main | Rien n'est téléchargé |
| Protocole incompatible | `session.protocol_mismatch` | Message du noyau affiché tel quel | Noyau arrêté proprement |
| Noyau mort | Processus quitte avant réponse | Panneau indique l'arrêt du noyau | Pas de blocage de QGIS |

</frozen-after-approval>

## Code Map

- `core/src/serve.rs` -- `read_lines` l.78, `fn hello` l.521 : contrat de `session.hello` ; le noyau s'arrête à la fermeture de stdin (code 0). Ne pas modifier.
- `core/src/registry.rs` l.16 -- `PROTOCOL_VERSION = 1`.
- `core/src/commands/system_about.rs` -- résultat `{core_version, protocol_version}`.
- `cli/src/main.rs` -- `cassini serve` sans argument.
- `docs/canal-local.md` -- cadrage du canal.
- `.github/workflows/recette.yml` -- placeholder du job `extension` : laissé à la story 3.
- `qgis-plugin/` -- n'existe pas, à créer.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/cassini_qgis/metadata.txt` -- versions 3.44 à 4.99, déclare le noyau comme dépendance externe dans About -- AD-17, AD-18
- [ ] `qgis-plugin/cassini_qgis/__init__.py` -- `classFactory` -- point d'entrée QGIS
- [ ] `qgis-plugin/cassini_qgis/locator.py` -- ordre fixe : réglage `cassini/core_path`, variable `CASSINI_CORE`, emplacement d'installation, `PATH` -- AD-17
- [ ] `qgis-plugin/cassini_qgis/client.py` -- `QProcess`, une ligne JSON par message, hello puis about, arrêt par fermeture de stdin -- canal local
- [ ] `qgis-plugin/cassini_qgis/plugin.py` -- dock minimal, cas d'erreur de la matrice, sélecteur de fichier manuel
- [ ] `qgis-plugin/tests/` -- tests unittest sans QGIS (Qt remplacé par un double) couvrant les 5 lignes de la matrice : localisation dans l'ordre AD-17, cadrage JSON par ligne, introuvable, sans projet, mismatch, noyau mort -- audit de la matrice
- [ ] `qgis-plugin/README.md` -- installation manuelle depuis le dépôt (hitl)

**Acceptance Criteria:**
- Given QGIS 3.44 avec l'extension chargée depuis le dépôt et un projet ouvert, when on l'active, then un panneau affiche la version du noyau et celle du protocole.
- Given l'extension active, when on ferme QGIS, then le processus `cassini` n'existe plus.
- Given aucun noyau installé, when on active l'extension, then rien n'est téléchargé et le panneau l'explique.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : 1 medium x3, 1 medium (tests), 1 low corrigé, 5 low/false rejetés.

| Verdict | Route | Constat et evidence |
|---------|-------|---------------------|
| medium | patch | `cleared` non connecté : File > New laisse l'ancien noyau et le panneau périmé (ligne "Aucun projet" non tenue). |
| medium | patch | Après DIED/FAILED, `client` et `project_path` restent fixés, donc `_refresh` ne relance jamais. |
| medium | patch | Lignes "introuvable" et "sans projet" de la matrice sans vrai test (test par positions de chaîne). |
| medium | patch | Fichiers `__pycache__`/`.pyc` dans le diff, aucune règle `.gitignore`. |
| low | patch | `aboutToQuit` jamais déconnecté dans `unload` (correction directe d'une ligne). |
| low | rejeté | `stop()` attend jusqu'à 3 s : le noyau sort en quelques ms à la fermeture de stdin, et un correctif ajouterait des branches. |
| low | rejeté | DIED possible deux fois après READY : même message, sans effet visible. |
| low | rejeté | `replace` de barres obliques inverses dans `start` : redondant, noms de fichiers Linux avec barre inverse négligeables. |
| false | rejeté | Hello écrit avant le démarrage : Qt met l'écriture en tampon ; l'échec de démarrage est déjà signalé par `errorOccurred`. |
| false | rejeté | Critères 1 et 2 non testés par les tests : ils relèvent du test manuel hitl prévu au plan. |

## Design Notes

`session.hello` : `{"jsonrpc":"2.0","id":1,"method":"session.hello","params":{"protocol_version":1,"commands":[],"project":"<chemin absolu, barres obliques>"}}`. Puis `command.run` avec `{"command":"system.about","params":{}}`. Le résultat est la valeur nue de la commande, sans enveloppe.

## Verification

**Commands:**
- `cargo test --workspace` -- expected: inchangé, noyau vert sans QGIS

**Manual checks (if no CLI):**
- Dans QGIS 3.44, charger l'extension, ouvrir un projet : le panneau affiche les deux versions ; fermer QGIS, vérifier dans le gestionnaire des tâches qu'aucun `cassini.exe` ne reste.
