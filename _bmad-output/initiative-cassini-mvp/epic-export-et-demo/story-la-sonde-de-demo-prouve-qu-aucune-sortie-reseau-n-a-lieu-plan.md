---
title: 'La sonde de démo prouve qu''aucune sortie réseau n''a lieu'
type: 'feature'
ticket: '5'
created: '2026-11-13'
status: 'built'
baseline_revision: '5464414953e55d868a872fcccd8024aa45a56934'
route: 'full'
route_source: 'auto'
risk: 'medium'
review: 'quick'
review_source: 'pinned'
lenses_ran: ['quick']
review_loop_iteration: 0
context:
  - '{project-root}/_bmad-output/initiative-cassini-mvp/epic-export-et-demo/story-la-sonde-de-demo-juge-la-phrase-rejoue-le-journal-et-verifie-plan.md'
---

<frozen-after-approval reason="human-owned intent — do not modify unless human renegotiates">

## Intent

**Problem:** la sonde de démo (`qgis-plugin/tests/probe_demo.py`) ne prouve pas le hors-ligne : rien ne vérifie que le noyau Cassini n'ouvre aucune connexion vers l'extérieur pendant la démo (exigence D1, principe 5 de `CLAUDE.md`).

**Approach:** pendant toute la sonde (plans, export, rejeu), un relevé périodique des connexions du processus du noyau (`netstat -ano` filtré par PID) est accumulé ; à la fin, `demo_verdict.py` juge : une connexion dont l'adresse distante n'est pas locale fait échouer le verdict en la nommant. Les connexions propres à QGIS ne sont pas surveillées.

## Boundaries & Constraints

**Always:** jugement pur dans `demo_verdict.py` (aucun import qgis/Qt), faits relevés par la sonde ; est locale l'adresse de bouclage (127.0.0.0/8, `::1`, IPv4 mappée) ou générique (`0.0.0.0`, `::`, `*`) ; surveiller le PID du noyau du plan puis celui du noyau du rejeu ; un relevé impossible (commande absente, sortie sans aucune ligne reconnue, plateforme sans `netstat`) rend ECHEC, jamais OK ; `VERDICT : ECHEC — réseau : <proto> <local> → <distant> (<état>) du noyau pid <n>`.

**Never:** modifier le noyau ou `cassini_qgis/` ; surveiller QGIS (couvert par l'entrée 6) ; nouvelle dépendance (pas de psutil) ; ouvrir une connexion dans le mode nominal.

## I/O & Edge-Case Matrix

| Scénario | Entrée / état | Comportement attendu | Gestion d'erreur |
|----------|---------------|----------------------|------------------|
| Hors ligne propre | démo nominale | relevés du PID du noyau, aucune connexion distante non locale | `VERDICT : OK` |
| Connexion externe | `CASSINI_DEMO_CONNECT=192.0.2.1:9` | la sonde ouvre une connexion non bloquante vers cette adresse et surveille aussi son propre PID | `ECHEC — réseau : TCP … → 192.0.2.1:9 …`, code 1 |
| Relevé illisible | `netstat` absent ou sortie sans ligne | aucun fait exploitable | `ECHEC — réseau : relevé impossible`, code 1 |
| Adresse invalide | `CASSINI_DEMO_CONNECT=abc` | refus au démarrage | `ECHEC — CASSINI_DEMO_CONNECT : <hôte:port> attendu`, code 1 |

</frozen-after-approval>

## Code Map

- `qgis-plugin/tests/demo_verdict.py` -- ajouter `parse_netstat(text)` (lignes TCP à 5 colonnes, UDP à 4, IPv6 entre crochets avec `%zone`), `is_local_address(addr)`, `judge_network(connections, watched_pids, polls)`, `connect_refused_reason(value)` ; même convention (raison ou `None`). Modèles : `drop_item_refused_reason` (l.145), `fonts_reason` (l.131).
- `qgis-plugin/tests/test_demo_verdict.py` -- classes `NetstatParseTest`, `JudgeNetworkTest` (style `DropItemTest`, l.265).
- `qgis-plugin/tests/probe_demo.py` -- `state` (l.~236) reçoit `net`, `polls` ; `QTimer` de relevé toutes les 1 s lisant `process.processId()` (variable globale `process`, remplacée par `start_replay`) ; relevé aussi juste avant le verdict final dans `on_replay_event` (l.~468) ; `finish` n'est pas touché. Docstring en tête à compléter (`CASSINI_DEMO_CONNECT`).
- Ne pas modifier : `finish`, `start_replay`, les gestes `Timed`.

## Tasks & Acceptance

**Execution:**
- [ ] `qgis-plugin/tests/demo_verdict.py` -- analyse de `netstat -ano`, adresse locale, `judge_network`, validation de `CASSINI_DEMO_CONNECT` -- jugement pur
- [ ] `qgis-plugin/tests/test_demo_verdict.py` -- une raison d'ECHEC par test, un OK, lignes TCP/UDP/IPv6 réelles de Windows -- couvre la matrice
- [ ] `qgis-plugin/tests/probe_demo.py` -- relevé périodique par `netstat -ano`, connexion de test, verdict réseau avant le verdict final, docstring -- la preuve de démo

**Acceptance Criteria:**
- Given le projet de démo, when la sonde tourne sous QGIS avec le vrai noyau et le vrai modèle, then elle rend `VERDICT : OK` après au moins un relevé du PID du noyau, sans connexion distante non locale.
- Given `CASSINI_DEMO_CONNECT=192.0.2.1:9`, when la sonde juge le réseau, then elle rend ECHEC en nommant `192.0.2.1:9`.
- Given `netstat` introuvable, when la sonde tente un relevé, then elle rend ECHEC « relevé impossible », jamais OK.

## Implementation Notes

## Plan Change Log

## Review Triage Log

Passe 1 (quick) : medium 2, low 4, rejet 4 (dont 1 false). Pas de boucle.

- PID du noyau du rejeu relevé seulement par la minuterie (peut être manqué) -- medium, patch : relevé aussi à `READY` du plan et du rejeu.
- `CASSINI_DEMO_CONNECT` refusée d'emblée (hors ligne, nom inconnu) : aucune ligne `netstat`, verdict OK à tort -- medium, patch : ECHEC « connexion de test jamais relevée » si aucune ligne du PID de la sonde n'a été vue.
- Tests UDP/IPv6 externes et PID non surveillé avant un surveillé -- low, patch : deux tests ajoutés.
- `netstat` synchrone bloque la boucle Qt chaque seconde, durées faussées -- low, rejeté : gêne négligeable, un relevé asynchrone ajouterait un `QProcess` et des branches.
- `polls` compte tout relevé non vide, même sans PID du noyau valide -- low, rejeté : le relevé final a toujours un PID ou rend ECHEC ; correction = garde de plus pour un cas improbable.
- Hôte vide compté local dans `is_local_address` -- low, rejeté : `netstat` ne rend jamais d'adresse distante vide.
- Réseau non jugé si la sonde échoue avant le rejeu -- false : voulu, un autre ECHEC est déjà le verdict ; le réseau se juge sur le chemin du verdict OK.

## Design Notes

Ce que la sonde prouve : ce que `netstat` montre du PID du noyau à chaque seconde, plus un relevé final. Une connexion plus brève qu'une seconde peut passer entre deux relevés ; la démo machine débranchée (entrée 6) complète. `CASSINI_DEMO_CONNECT` prouve le chemin de jugement, pas le noyau, comme `CASSINI_DEMO_DROP_ITEM`. Les sockets en écoute sur une adresse non locale ne sont pas jugées ici (le ticket vise les connexions sortantes).

## Verification

**Commands:**
- `python -I -m unittest discover -s qgis-plugin/tests` -- expected: OK
- `python -I qgis-plugin/tests/check_extension.py` -- expected: propre

**Manual checks (if no CLI):**
- Sur le poste du propriétaire : `probe_demo.py` nominale `VERDICT : OK` ; avec `CASSINI_DEMO_CONNECT=192.0.2.1:9` ECHEC nommant l'adresse.
