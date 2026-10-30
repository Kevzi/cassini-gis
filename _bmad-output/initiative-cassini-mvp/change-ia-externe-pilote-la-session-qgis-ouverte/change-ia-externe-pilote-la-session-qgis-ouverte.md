---
type: change
title: "Une IA externe pilote la session QGIS ouverte, sous les yeux de l'utilisateur"
date: 2026-10-30
status: approved
scope: moderate
trigger: "Recette de 5.7, décision du propriétaire du 2026-10-30"
---

# Une IA externe pilote la session QGIS ouverte, sous les yeux de l'utilisateur

## 1. Résumé du problème

Le ticket 5.7 a livré `cassini mcp` : un serveur MCP sur stdio, lancé par l'application de l'IA externe,
**sans lien avec la session QGIS**. Il n'expose donc que les commandes du noyau (`system.about`,
`model.list`). La recette du 2026-10-30, faite avec Claude Code comme client, l'a montré : à la phrase de
démo, l'IA répond qu'elle ne peut rien faire.

Exigence du propriétaire (2026-10-30) : une IA externe doit travailler **pendant que QGIS est ouvert**, et
l'utilisateur doit **voir ce qui se fait** (carte, panneau Cassini) — « sinon ça ne sert à rien ». Elle doit
être **compatible dès le départ avec GPT et Gemini**, pas seulement avec Claude.

Fait vérifié le 2026-10-30 dans la documentation de chaque éditeur : les trois familles de clients savent lancer un serveur MCP local sur stdio —
Claude Desktop et Claude Code ; ChatGPT pour ordinateur et Codex CLI (configuration partagée,
`[mcp_servers.<nom>]`, `command` + `args`) ; Gemini CLI (`mcpServers` dans `settings.json`). L'application
web de ChatGPT, elle, ne lance pas de processus local. Réserve : la page d'OpenAI ne confirme pas
explicitement STDIO pour ChatGPT pour ordinateur **sous Windows** ; Codex CLI sert de repli (à vérifier en
5.27).

## 2. Analyse d'impact

### Pistes étudiées

| Piste | Principe | Verdict |
|---|---|---|
| A. Fournisseur choisi dans le panneau | Le panneau envoie la phrase à l'API d'OpenAI, de Google ou d'Anthropic | **Écartée.** Trois intégrations propres à chaque fournisseur, des clés d'API à garder, un client HTTP dans Cassini : contredit §2.8 (« IA externes via MCP »), AD-14 et `deny.toml`. |
| **B. Pont local MCP → session QGIS** | `cassini mcp` trouve la session QGIS ouverte et lui relaie les appels ; le noyau de la session les exécute, QGIS dessine, le panneau affiche chaque appel | **Retenue.** Un seul code pour tous les clients MCP (Claude, GPT, Gemini, et ceux à venir), aucune ligne propre à un fournisseur, aucun réseau dans Cassini : c'est l'application de l'IA, choisie par l'utilisateur, qui parle au réseau. |

### Ce que fait la piste B

1. Dans le panneau Cassini, un interrupteur **« Autoriser une IA externe »**, éteint par défaut.
2. Allumé, le noyau de la session (`cassini serve`) ouvre un **canal local** : tube nommé sous Windows
   (refus des clients distants, accès limité à l'utilisateur courant), socket de domaine Unix dans un
   dossier privé de l'utilisateur ailleurs. Ni TCP, ni port, ni pare-feu.
3. `cassini mcp`, lancé par l'application de l'IA, cherche ce canal :
   - **session trouvée** — il relaie ; le catalogue MCP devient celui de la session, commandes QGIS
     comprises (celles que AD-25 annonce) ;
   - **aucune session** — il garde le comportement de 5.7 (commandes du noyau seules) et le dit à l'IA.
4. Chaque appel de l'IA passe par le noyau de la session, qui valide les paramètres, dicte l'ordre à
   l'extension (AD-5), **journalise** (AD-11, un seul auteur) et rend le résultat.
5. Le panneau **montre chaque appel en direct** (commande, paramètres en clair, résultat), avec un bouton
   **Arrêter** (annulation, AD-12) ; la couche produite apparaît sur la carte.
6. Les appels deviennent non bloquants (report de 5.7) : `ping` et annulation restent servis pendant une
   commande longue.

### Épics et tickets

- **Epic agent et MCP (5)** : 5.7 reste valide (mode autonome) ; trois entrées s'ajoutent (§5). Le critère 4
  de « Done when » est élargi.
- **Epic façade QGIS (9)** : le panneau gagne l'interrupteur et le fil des appels externes — rempli par
  l'epic 5, comme pour le plan (Boundaries de l'epic 5).
- **Epic export et démo (6)** : la répétition de démo peut se jouer avec un client externe en plus du
  modèle local.
- Aucun ticket fait n'est défait.

### Artefacts en conflit

- **Architecture** : AD-2 (« aucune socket… »), AD-13 (mode de `mcp`), AD-14 (preuve étendue au canal
  local), AD-3 (inchangé dans sa règle, précisé pour le relais).
- **CLAUDE.md** §3 « Canal local » : « Aucune socket, aucun port, aucun fichier d'échange ».
- **Spec** CAP-8 : critère de succès élargi.
- **UX** `ux-panneau-cassini/EXPERIENCE.md` : nouvel état du panneau (IA externe branchée).

### Impact technique

- Noyau (`cli`) : serveur du canal local dans `serve`, relais dans `mcp`. Sous Windows, le tube nommé passe
  par l'API Win32 (`kernel32`) — la garde `hors_ligne.rs` (aucune DLL réseau) reste valable. Une
  dépendance d'accès Win32 sera nécessaire ; sa licence sera demandée au moment du ticket (§8).
- Extension : interrupteur et fil d'appels dans le panneau ; aucune logique métier ajoutée.
- Risque : `high` (canal, journal, nouvelle entrée sur la session de l'utilisateur).

## 3. Approche recommandée

**Ajustement direct** : ajout de trois entrées à l'epic 5 et amendement ciblé de l'architecture, sans
retour arrière. Effort estimé : trois tickets de taille moyenne. Calendrier : à placer après la fin de
l'epic 5 en cours et **avant** le scénario de démo (étape 7), pour que la démo puisse montrer GPT ou Gemini
piloter QGIS.

## 4. Propositions de modification

### Architecture — AD-2

OLD :
> **Rule:** l'extension lance le binaire du noyau en `cassini serve` et dialogue par son entrée et sa sortie
> standard. Aucune socket, aucun port TCP, aucun fichier d'échange, dans aucune des deux moitiés. […]

NEW :
> **Rule:** l'extension lance le binaire du noyau en `cassini serve` et dialogue par son entrée et sa sortie
> standard. Aucune socket réseau, aucun port TCP, aucun fichier d'échange, dans aucune des deux moitiés.
> **Seule exception, le canal d'IA externe (AD-29)** : quand l'utilisateur l'autorise dans le panneau, le
> noyau de la session ouvre un canal local réservé à `cassini mcp` — tube nommé Windows qui refuse les
> clients distants, socket de domaine Unix dans un dossier privé de l'utilisateur — fermé dès que
> l'interrupteur s'éteint ou que la session s'arrête. […]

Raison : ouvrir la session à une IA choisie par l'utilisateur sans ouvrir d'interface réseau.

### Architecture — nouvelle AD-29 « Une IA externe pilote la session ouverte par relais »

- **Binds:** CAP-8, CAP-13, CAP-1
- **Prevents:** une IA externe aveugle (sans QGIS), un second noyau qui écrirait le même journal, du code
  propre à un fournisseur d'IA.
- **Rule:** `cassini mcp` relaie, il n'exécute rien quand une session est ouverte : le noyau de la session
  reste seul à valider, dicter (AD-5) et journaliser (AD-11). Le catalogue MCP est celui de la session.
  Chaque appel externe est montré dans le panneau et peut être arrêté. Sans session, `mcp` sert les
  commandes du noyau seules (5.7). Aucun code propre à un fournisseur d'IA.

### Architecture — AD-13 et AD-14

- AD-13 : ajouter « `mcp` a deux modes : relais vers la session ouverte, ou autonome sur le registre du
  noyau ».
- AD-14 : ajouter une preuve : le canal local refuse un client distant, et le binaire n'importe toujours
  aucune DLL réseau.

### Spec — CAP-8, success

OLD : « un client MCP tiers découvre les commandes et en exécute une ; le produit garde toutes ses fonctions
quand aucune IA externe n'est branchée. »

NEW : « un client MCP tiers découvre les commandes et en exécute une ; **quand une session QGIS est ouverte
et que l'utilisateur l'autorise, il pilote cette session et chaque appel est visible dans le panneau ;
vérifié avec un client de chacune des familles Claude, GPT et Gemini** ; le produit garde toutes ses
fonctions quand aucune IA externe n'est branchée. »

### CLAUDE.md §3 « Canal local »

OLD : « Aucune socket, aucun port, aucun fichier d'échange. »

NEW : « Aucune socket réseau, aucun port, aucun fichier d'échange ; seule exception, le canal local
d'IA externe, ouvert à la demande de l'utilisateur (AD-29). »

### Epic 5 — Done when, point 4

OLD : « CAP-8 : un client MCP tiers découvre les commandes et en exécute une ; sans IA externe branchée,
l'application garde toutes ses fonctions. »

NEW : « CAP-8 : un client MCP tiers découvre les commandes et en exécute une ; avec QGIS ouvert, Claude,
GPT et Gemini pilotent la session sous les yeux de l'utilisateur ; sans IA externe branchée, l'application
garde toutes ses fonctions. »

### UX — EXPERIENCE.md

Ajouter l'état « IA externe autorisée » : interrupteur dans les réglages du panneau, bandeau « Une IA
externe peut agir sur ce projet », fil des appels (commande, paramètres en clair, résultat, durée), bouton
Arrêter.

## 5. Passation

Portée : **modérée** (ajouts au carnet de l'epic 5, amendement d'architecture).

Entrées à créer avec `bmad-ticket` dans l'epic 5 (à la création, le 2026-10-30, la vérification du lot a coupé la première en deux : 5.24 canal côté session, 5.25 relais `cassini mcp` ; le panneau devient 5.26 et la recette 5.27) :

1. **5.24 — Canal local : la session QGIS accepte une IA externe** (`risk: high`). Interrupteur, canal
   local ouvert et fermé par le noyau de la session, `cassini mcp` relaie le catalogue et les appels, appels
   non bloquants ; repli autonome sans session. *after* : 5.7. Prérequis : AD-2, AD-13, AD-14 amendées et
   AD-29 écrite par `bmad-architecture`.
2. **5.25 — Le panneau montre chaque appel de l'IA externe** (`risk: medium`). Bandeau, fil des appels,
   bouton Arrêter, couche sur la carte. *after* : 5.24.
3. **5.26 — Recette Claude, GPT et Gemini sur la phrase de démo** (`risk: medium`, hitl). Fiches de
   branchement (Claude Desktop, ChatGPT pour ordinateur ou Codex, Gemini CLI) dans le README, phrase de démo
   jouée avec chacun, QGIS ouvert. *after* : 5.25.

Ordre : `bmad-architecture` (AD-2, AD-13, AD-14, AD-29) → mise à jour de la spec, de CLAUDE.md et
d'EXPERIENCE.md → `bmad-ticket` (trois entrées) → `bmad-build` 5.24, 5.25, 5.26.

Critère de succès : QGIS ouvert sur le projet Villeneuve-le-Roi, l'utilisateur allume l'interrupteur, tape
la phrase de démo dans Claude, puis dans ChatGPT, puis dans Gemini ; chaque fois, les appels défilent dans
le panneau, la couche apparaît sur la carte, le journal se rejoue à l'identique, et le binaire n'importe
aucune DLL réseau.

## Décisions du propriétaire (2026-10-30)

Proposition approuvée telle quelle, recommandations comprises :

1. **Validation des appels externes** : chaque appel est exécuté et montré en direct, avec un bouton
   Arrêter ; une case « Me demander avant chaque appel » est proposée, éteinte par défaut.
2. **ChatGPT web** : hors périmètre ; GPT est servi par ChatGPT pour ordinateur et Codex.

## Sources

- ChatGPT et Codex, MCP : https://learn.chatgpt.com/docs/extend/mcp ; https://developers.openai.com/codex/mcp
- Gemini CLI, MCP : https://github.com/google-gemini/gemini-cli/blob/main/docs/tools/mcp-server.md
